use core::f32::consts::E;
use core::ops::Add;

use crate::fs::{open_file,path_to_dentry,path_to_father_dentry,create_file};
use crate::mm::{translated_ref, translated_refmut, translated_str, MapType};
use crate::task::{
    self, UNAME,add_task, current_task, current_user_token, 
    exit_current_and_run_next, suspend_current_and_run_next,SignalFlags,pid2task,remove_from_pid2task,
    MAX_SIG,SigAction
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use arch::time::{self, Time};
use arch::{TrapFrameArgs,PAGE_SIZE};
use crate::drivers::BLOCK_DEVICE;
use vfs_defs::{DiskInodeType,OpenFlags,Dentry};
use config::{ USER_STACK_SIZE,RLimit,Resource};
use arch::addr::{PhysPage, VirtAddr, VirtPage};
use crate::mm::{MapPermission, MapArea, from_prot};
use arch::pagetable::MappingSize;
use crate::task::{Tms, Utsname, TimeSpec, SysInfo};
use bitflags::*;
use system_result::{SysError,SysResult};
const MODULE_LEVEL:log::Level = log::Level::Trace;

bitflags! {
    /// Defined in <bits/sched.h>
    pub struct CloneFlags: u64 {
        /// Set if VM shared between processes.
        const VM = 0x0000100;
        /// Set if fs info shared between processes.
        const FS = 0x0000200;
        /// Set if open files shared between processes.
        const FILES = 0x0000400;
        /// Set if signal handlers shared.
        const SIGHAND = 0x00000800;
        /// Set if a pidfd should be placed in parent.
        const PIDFD = 0x00001000;
        /// Set if we want to have the same parent as the cloner.
        const PARENT = 0x00008000;
        /// Set to add to same thread group.
        const THREAD = 0x00010000;
        /// Set to shared SVID SEM_UNDO semantics.
        const SYSVSEM = 0x00040000;
        /// Set TLS info.
        const SETTLS = 0x00080000;
        /// Store TID in userlevel buffer before MM copy.
        const PARENT_SETTID = 0x00100000;
        /// Register exit futex and memory location to clear.
        const CHILD_CLEARTID = 0x00200000;
        /// Store TID in userlevel buffer in the child.
        const CHILD_SETTID = 0x01000000;
        /// Create clone detached.
        const DETACHED = 0x00400000;
        /// Set if the tracing process can't
        const UNTRACED = 0x00800000;
        /// New cgroup namespace.
        const NEWCGROUP = 0x02000000;
        /// New utsname group.
        const NEWUTS = 0x04000000;
        /// New ipcs.
        const NEWIPC = 0x08000000;
        /// New user namespace.
        const NEWUSER = 0x10000000;
        /// New pid namespace.
        const NEWPID = 0x20000000;
        /// New network namespace.
        const NEWNET = 0x40000000;
        /// Clone I/O context.
        const IO = 0x80000000 ;
    }
}

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next((exit_code & 0xFF) << 8);//posix标准退出码
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> SysResult<isize> {
    suspend_current_and_run_next();
    Ok(0)
}

pub fn sys_get_time(ts:*mut TimeSpec) -> SysResult<isize> {
    let token = current_user_token();
    if ts.is_null(){
        return Err(SysError::EINVAL);
    }
    let ts = translated_refmut(token, ts);
    let usec = Time::now().to_usec();
    *ts = TimeSpec{
        sec:usec/1000000,
        usec,
    };
    Ok(0)
}

pub fn sys_getpid() -> SysResult<isize> {
    Ok(current_task().unwrap().pid.0 as isize)
}

pub fn sys_clone(flags:usize,stack_ptr:*const u8,ptid:*mut i32,_tls:*mut i32,ctid:*mut i32) -> SysResult<isize> {
    let flags = CloneFlags::from_bits(flags as u64 & !0xff);
    let token = current_user_token();
    if flags.is_none(){
        return Err(SysError::EINVAL);
    }
    let flags = flags.unwrap();
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();    
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    //trap_cx.x[10] = 0;
    trap_cx[TrapFrameArgs::RET] = 0;
    if !stack_ptr.is_null(){
        let new_sp = translated_ref(token, stack_ptr);
        trap_cx[TrapFrameArgs::SP] = new_sp as *const u8 as usize;
    }
    if flags.contains(CloneFlags::PARENT_SETTID) {
        *translated_refmut(token, ptid) = new_pid as i32;
    }
    if flags.contains(CloneFlags::CHILD_SETTID) {
        *translated_refmut(token, ctid) = new_pid as i32;
    }
    // add new task to scheduler
    add_task(new_task);
    Ok(new_pid as isize)
}

pub fn sys_exec(path: *const u8, mut args: *const usize) -> SysResult<isize> {
    let token = current_user_token();
    let path = translated_str(token, path);
    log_debug!("exec path={}",path);
    let mut args_vec: Vec<String> = Vec::new();
    let mut first_arg = true;
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        if first_arg{
            let mut name = translated_str(token, arg_str_ptr as *const u8);
            name.insert_str(0, "./");
            args_vec.push(name);
            first_arg = false;
        }
        else{
            let arg_str = translated_str(token, arg_str_ptr as *const u8);
            if arg_str == "rm"{
                args_vec.push(arg_str);
                args_vec.push(String::from("-f"));
            }
            else{
                args_vec.push(arg_str);
            }
        }
        
        unsafe {
            args = args.add(1);
        }
    }
    let app_inode = open_file(path.as_str(), OpenFlags::RDONLY)?;
    let all_data = app_inode.read_all();
    let task = current_task().unwrap();
    task.exec(all_data.as_slice(),args_vec);
    Ok(0)
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> SysResult<isize> {
    let task = current_task().unwrap();
    // find a child process
    log_debug!("waitpid pid={}",pid);
    // ---- access current PCB exclusively
    let inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return Err(SysError::ESRCH);
        // ---- release current PCB
    }
    drop(inner);
    loop{
        let mut inner = task.inner_exclusive_access();
        let pair = inner.children.iter().enumerate().find(|(_, p)| {
            // ++++ temporarily access child PCB exclusively
            p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
            // ++++ release child PCB
        });
        if let Some((idx, _)) = pair {
            let child = inner.children.remove(idx);
            // confirm that child will be deallocated after being removed from children list
            assert_eq!(Arc::strong_count(&child), 1);
            let found_pid = child.getpid();
            // 移除 PID2TCB 中的引用（以防万一）
            remove_from_pid2task(found_pid);
            // 确认引用计数（仅用于调试，可选）
            log::debug!("Child strong count after removal: {}", Arc::strong_count(&child));
            // ++++ temporarily access child PCB exclusively
            let exit_code = child.inner_exclusive_access().exit_code;
            // ++++ release child PCB
            if exit_code_ptr != core::ptr::null_mut(){
                *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
            }
            return Ok(found_pid as isize);
        } else {
            drop(inner);
            suspend_current_and_run_next();
        }
    }

    // ---- release current PCB automatically
}

///
pub fn sys_chdir(path: *const u8) -> SysResult<isize> {
    let token = current_user_token();
    let path = translated_str(token, path);
    let dentry = path_to_dentry(&path)?;
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.cwd = dentry;
    Ok(0)
}


pub fn sys_brk(new_brk:  usize) -> SysResult<isize> {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let cur_brk = task_inner.heap_top;
    //println!("sys_brk: heap_top = {}, stack_bottom = {} new_brk:{}",task_inner.heap_top,task_inner.stack_bottom,new_brk);
    if new_brk == 0 {
        return Ok(cur_brk as isize);
    }
    
    if task_inner.max_data_addr >= new_brk && new_brk < task_inner.stack_bottom { // 利用上一次分配的多余内存
        task_inner.heap_top = new_brk;
    //    return 0;
        return Ok(new_brk as isize);
    }

    if new_brk > cur_brk {
        let user_stack_bottom = task_inner.stack_bottom;
        
        if new_brk >= user_stack_bottom -PAGE_SIZE { 
            return Ok(cur_brk as isize);
//            return -1;
        }
        let cur_addr = VirtAddr::new(task_inner.max_data_addr).floor();
        let new_addr = VirtAddr::new(new_brk).ceil();
        
        let page_count = (new_addr.addr() - cur_addr.addr()) / PAGE_SIZE;
        let alloc_start_addr = task_inner.max_data_addr;
        task_inner.memory_set.push_into_heaparea(
            MapArea::new(
                VirtAddr::new(alloc_start_addr), //向下
                VirtAddr::new(new_brk), //向上
                MapType::Framed,
                MapPermission::R|MapPermission::U|MapPermission::W|MapPermission::X
            ),
            None
        );
        task_inner.max_data_addr += PAGE_SIZE*page_count;
        //println!("max_data_addr = {}", task_inner.max_data_addr);
        task_inner.heap_top = new_brk;
        return Ok(new_brk as isize);
     //   0
    }
    else if new_brk == cur_brk {
          return Ok(cur_brk as isize);
    //    -1
    }
    else {// 不考虑不合理的减小
        // 确认需要释放的虚页号
        /*let cur_page = cur_brk / PAGE_SIZE;
        
        let new_page = new_brk / PAGE_SIZE ; 
        let page_count = cur_page - new_page;
        
        if page_count > 0 {
            // 解除映射并释放物理页帧
            for i in 1..(page_count + 1) {
                let vpn = VirtPage::from(cur_page - i);
                task_inner.memory_set.unmap_page(vpn);
                    //frame_dealloc(ppn);
            }
        }*/
        // 不释放
        // new_brk 应当大于数据段起始 未做判断
        task_inner.heap_top = new_brk;
        return Ok(new_brk as isize);
    //   0
    }
}

pub fn sys_times(tms_ptr: *mut Tms) -> SysResult<isize> {
    let binding = current_task().unwrap();
    let token = current_user_token();
    let taskinner = binding.inner_exclusive_access();
    if tms_ptr.is_null(){
        return Err(SysError::EINVAL);
    }
    // 安全地获取一个可变引用
    let tms = translated_refmut(token, tms_ptr);
    // 当前 - 起始
    tms.tms_utime = Time::now().to_msec() as usize - taskinner.tms.tms_utime;
    tms.tms_stime = Time::now().to_msec() as usize - taskinner.tms.tms_stime;
    tms.tms_cutime = Time::now().to_msec() as usize - taskinner.tms.tms_cutime;
    tms.tms_cstime = Time::now().to_msec() as usize - taskinner.tms.tms_cstime;
    Ok(tms.tms_cutime as isize)
}

pub fn sys_uname(mes: *mut Utsname) -> SysResult<isize> {
    let uname = unsafe {
        if mes.is_null() {
            return Err(SysError::EINVAL);
        }
        &mut *mes
    };
    //
    uname.copy_from(&UNAME);
    Ok(0)
}


/* for nanosleep:
struct timespec {
	time_t tv_sec;        /* 秒 */
	long   tv_nsec;       /* 纳秒, 范围在0~999999999 */
};
*/
pub fn sys_nanosleep(timespec:*const TimeSpec)->SysResult<isize>{
    if timespec.is_null(){
        return Err(SysError::EINVAL);
    }
    let token = current_user_token();
    let timespec = translated_ref(token, timespec);
    if timespec.usec > 99999999{
        return Err(SysError::EINVAL);
    }
    let total_time = timespec.sec * 1000000000 + timespec.usec;
    let start_time = Time::now().to_nsec();
    loop{
        let current_time = Time::now().to_nsec();
        if current_time - start_time < total_time{
            suspend_current_and_run_next();
        }
        else{
            return Ok(0);
        }
    }
}

pub fn sys_getppid()->SysResult<isize>{
    let current = current_task().unwrap();
    let inner = current.inner_exclusive_access();
    if inner.parent.is_none(){
        return Err(SysError::ESRCH);
    }
    let parent = inner.parent.clone().unwrap().upgrade().unwrap();
    let ret = parent.pid.0 as isize;
    Ok(ret)
}

pub fn sys_set_tid_address(tidptr:usize)->SysResult<isize>{
    let current = current_task().unwrap();
    let mut inner = current.inner_exclusive_access();
    inner.tidaddress.clear_child_tid = Some(tidptr);
    Ok(tidptr as isize)
}


pub fn sys_prlimit64(pid: usize,resource: i32,new_limit: *const RLimit,old_limit: *mut RLimit) -> SysResult<isize> {
    let task;
    let token = current_user_token();
    if pid == 0{
        task = current_task().unwrap();
    }
    else {
        if let Some(t) = pid2task(pid){
            task = t;
        }
        else {
            return Err(SysError::ESRCH);
        }
    }
    let mut inner = task.inner_exclusive_access();
    let resource = Resource::new(resource);
    if resource.is_none(){
        return Err(SysError::EINVAL);
    }
    let resource = resource.unwrap();
    if !old_limit.is_null(){
        let limit;
        match resource{
            Resource::STACK=>{
                limit = RLimit{
                    rlimit_cur:USER_STACK_SIZE,
                    rlimit_max:USER_STACK_SIZE,
                }
            },
            Resource::NOFILE=>{
                limit = inner.fd_table.rlimit();
            }
            _=>{
                limit = RLimit{
                    rlimit_cur:0,
                    rlimit_max:0
                }
            }
        };
        *translated_refmut(token,old_limit) = limit;
    }
    if !new_limit.is_null(){
        let limit = *translated_ref(token, new_limit);
        match resource {
            Resource::NOFILE=>{
                inner.fd_table.set_rlimit(limit);
            },
            _=>{}
        }
    }
    return Ok(0);
} 

/*
struct timespec {
	time_t tv_sec;        /* 秒 */
	long   tv_nsec;       /* 纳秒, 范围在0~999999999 */
};
*/
pub const CLOCK_REALTIME: usize = 0; //标准POSIX实时时钟
pub const CLOCK_MONOTONIC: usize = 1; //POSIX时钟,以恒定速率运行;不会复位和调整,它的取值和CLOCK_REALTIME是一样的.
pub const CLOCK_PROCESS_CPUTIME_ID: usize = 2;
pub const CLOCK_THREAD_CPUTIME_ID: usize = 3; //是CPU中的硬件计时器中实现的.

pub fn sys_clock_gettime(clockid: usize, tp: *mut TimeSpec) -> SysResult<isize> {
    if tp.is_null() {
        return Err(SysError::EINVAL);
    }
    match clockid {
        CLOCK_REALTIME | CLOCK_MONOTONIC => {
            let token = current_user_token();
            let tp_ref = translated_refmut(token, tp);
            tp_ref.sec = Time::now().to_sec();
            tp_ref.usec = Time::now().to_usec();
        }
        CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => {
            panic!("CLOCK_PROCESS_CPUTIME_ID/CLOCK_THREAD_CPUTIME_ID unsupported!");
        }
        _ => {
            //panic!("unsupported clock_id!");
            return Ok(0);
        }
    }
    Ok(0)
}

pub fn sys_exit_group(exit_code: i32) -> ! { //退出线程组，但没有子线程
    exit_current_and_run_next((exit_code & 0xFF) << 8);//posix标准退出码
    panic!("Unreachable in sys_exit!");
}

pub fn sys_get_random(buf: *mut u8, len: usize, _flags: usize) -> SysResult<isize> {
    let token = current_user_token();
    for index in 0..len {
        let mut byte: u8 = 0;
        for _ in 0..8 {
            let t = Time::now().to_usec();
            //println!("time={}",t);
            byte <<= 1;
            if t/2*2 == t { //0
                byte |= 0;
            }
            else { //1
                byte |= 1;
            }
        }
        let buf_ref = translated_refmut(token, buf.wrapping_add(index * 1));
        *buf_ref = byte;
    }
    return Ok(0);
} 
pub fn sys_info(info: *mut SysInfo) -> SysResult<isize> {
    let token = current_user_token();
    let info_ref = translated_refmut(token, info);
    *info_ref = SysInfo::default();
    Ok(0)
}

pub fn sys_log(log_type: usize, _buf: *mut u8, _len: usize) -> SysResult<isize> {
    match log_type {
        2 |3 |4 => {
            Ok(0)
        }
        _ => {
            Ok(0)
        }
    }
}

pub fn sys_mprotect(addr: VirtAddr, _len: usize, prot: i32) -> SysResult<isize> {
    if addr.addr() / PAGE_SIZE *PAGE_SIZE != addr.addr() {
        return Err(SysError::EINVAL);
    }
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    //addr-addr+len-1
    let perm = from_prot(prot); //提取权限
    // 找到包含addr的区域
    let mut is_find = false;
    for ele in &mut inner.memory_set.mmap_area {
        if ele.vpn_range.get_start_addr() >= addr && ele.vpn_range.get_end_addr() < addr {
            ele.map_perm = perm;
            is_find = true;
            break;
        }
    }
    if !is_find {
        for ele in &mut inner.memory_set.heap_area {
            if ele.vpn_range.get_start_addr() >= addr && ele.vpn_range.get_end_addr() < addr {
                ele.map_perm = perm;
                is_find = true;
                break;
            }
        }
    }
    if !is_find {
        for ele in &mut inner.memory_set.areas {
            if ele.vpn_range.get_start_addr() >= addr && ele.vpn_range.get_end_addr() < addr {
                ele.map_perm = perm;
                break;
            }
        }
    } 
    Ok(0)
}
pub fn sys_kill(pid: usize, signal: u32) -> SysResult<isize> {
    if let Some(process) = pid2task(pid) {
        if let Some(flag) = SignalFlags::from_bits(1 << signal) {
            println!("[kernel] sys_kill: Adding signal {} to pid {}", signal, pid);
            let mut inner = process.inner_exclusive_access();
            inner.signals |= flag;
            drop(inner);
            Ok(0)
        } else {
            println!("[kernel] sys_kill: Invalid signal {}", signal);
            Err(SysError::EINVAL)
        }
    } else {
        println!("[kernel] sys_kill: Process {} not found", pid);
        Err(SysError::ESRCH)
    }
}



pub fn sys_sigaction(
    signum: i32,
    action: *const SigAction,
    old_action: *mut SigAction,
) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    // 检查信号编号是否合法
    if signum <= 0 || signum as usize > MAX_SIG {
        
        return -1;
    }

    // 将 signum 转换为 SignalFlags
    let flag = match SignalFlags::from_bits(1 << signum) {
        Some(flag) => flag,
        None => {
            
            SignalFlags::empty()
        }
    };

    // 检查参数是否合法
    if check_sigaction_error(flag) {
        
        return -1;
    }

    // 保存旧的信号处理函数
    let prev_action = inner.signal_actions.table[signum as usize];
    if !old_action.is_null() {
        *translated_refmut(token, old_action) = prev_action;
    }

    // 设置新的信号处理函数
    if !action.is_null() {
        inner.signal_actions.table[signum as usize] = *translated_ref(token, action);
    }

    0
}

fn check_sigaction_error(signal: SignalFlags) -> bool {
    // 只限制 SIGKILL 和 SIGSTOP
    signal == SignalFlags::SIGKILL || signal == SignalFlags::SIGSTOP
}

// 定义 how 参数的可能值（参考 POSIX）
const SIG_BLOCK: i32 = 0;   // 将 set 中的信号添加到掩码
const SIG_UNBLOCK: i32 = 1; // 从掩码中移除 set 中的信号
const SIG_SETMASK: i32 = 2; // 将掩码设置为 set

pub fn sys_sigprocmask(how: i32, set: *const SignalFlags, oldset: *mut SignalFlags) -> isize {
    let token = current_user_token();
    if let Some(task) = current_task() {
        let mut inner = task.inner_exclusive_access();
        let old_mask = inner.signal_mask;

        // 保存旧的信号掩码
        if !oldset.is_null() {
            *translated_refmut(token, oldset) = old_mask;
        }

        // 如果 set 不为空，更新信号掩码
        if !set.is_null() {
            let new_set = *translated_ref(token, set);
            match how {
                SIG_BLOCK => {
                    inner.signal_mask |= new_set;
                    
                }
                SIG_UNBLOCK => {
                    inner.signal_mask &= !new_set;
                    
                }
                SIG_SETMASK => {
                    inner.signal_mask = new_set;
                    
                }
                _ => {
                    
                    return -1;
                }
            }
        }

        0
    } else {
        
        -1
    }
}
pub fn sys_gettid()->SysResult<isize>{
    Ok(current_task().unwrap().pid.0 as isize)
}
pub fn sys_sigreturn() -> isize {
    if let Some(task) = current_task() {
        let mut task_inner = task.inner_exclusive_access();
        if let Some(backup) = task_inner.trap_ctx_backup.take() {
            task_inner.handling_sig = -1;
            task_inner.trap_cx = backup;
            // 返回信号处理程序的返回值（通常为 0）
            0
        } else {

            -1
        }
    } else {
        
        -1
    }
}

pub fn sys_clock_nanosleep(clockid: usize,flags:usize,request:*const TimeSpec,remain:*mut TimeSpec)->SysResult<isize>{
    pub const TIMER_ABSTIME: usize = 1;
    match clockid {
        CLOCK_REALTIME | CLOCK_MONOTONIC => {
            let token = current_user_token();
            let request = translated_ref(token, request);
            let req= request.sec * 1000000000 + request.usec; 
            let total_time;
            let mut rem:TimeSpec = TimeSpec{sec:0,usec:0};
            if flags == TIMER_ABSTIME {
                let current_time = Time::now().to_nsec();
                // request time is absolutely
                if req.le(&current_time) {
                    return Ok(0);
                }
                total_time = req - current_time;
            } else {
                total_time = req;
            };
            let start_time = Time::now().to_nsec();
            loop{
                let current_time = Time::now().to_nsec();
                if current_time - start_time < total_time{
                    rem.sec = (total_time - current_time + start_time) / 1000000000;
                    rem.usec = (total_time - current_time + start_time) % 1000000000;
                    suspend_current_and_run_next();
                }
                else{
                    rem.sec = 0;
                    rem.usec = 0;
                    break;
                }
            }
            if rem.sec == 0 &&rem.usec == 0 {
                Ok(0)
            } else {
                if !remain.is_null() {
                    *translated_refmut(token, remain) = rem;
                }
                Err(SysError::EINTR)
            }
        }
        _ => {
            return Err(SysError::EINVAL);
        }
    }
}
