use crate::fs::{open_file,path_to_dentry,path_to_father_dentry,create_file};
use crate::mm::{translated_ref, translated_refmut, translated_str, MapType};
use crate::task::{
    self, add_task, current_task, current_user_token, exit_current_and_run_next, suspend_current_and_run_next,SignalFlags,pid2task,remove_from_pid2task
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use arch::time::{self, Time};
use arch::{TrapFrameArgs,PAGE_SIZE};
use crate::drivers::BLOCK_DEVICE;
use vfs_defs::{DiskInodeType,OpenFlags,Dentry};
use crate::config::{UNAME, USER_STACK_SIZE,RLimit,Resource};
use arch::addr::{PhysPage, VirtAddr, VirtPage};
use crate::mm::{MapPermission, MapArea, from_prot};
use arch::pagetable::MappingSize;
use crate::task::{Tms, Utsname, TimeSpec, SysInfo};
use bitflags::*;
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

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(ts:*mut TimeSpec) -> isize {
    let token = current_user_token();
    if ts.is_null(){
        return -1;
    }
    let ts = translated_refmut(token, ts);
    let usec = Time::now().to_usec();
    *ts = TimeSpec{
        sec:usec/1000000,
        usec,
    };
    0
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_clone(flags:usize,stack_ptr:*const u8,ptid:*mut i32,_tls:*mut i32,ctid:*mut i32) -> isize {
    let flags = CloneFlags::from_bits(flags as u64 & !0xff);
    let token = current_user_token();
    if flags.is_none(){
        return -1;
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
    new_pid as isize
}

pub fn sys_exec(path: *const u8, mut args: *const usize) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    log_debug!("exec path={}",path);
    let mut args_vec: Vec<String> = Vec::new();
    loop {
        let arg_str_ptr = *translated_ref(token, args);
        if arg_str_ptr == 0 {
            break;
        }
        args_vec.push(translated_str(token, arg_str_ptr as *const u8));
        unsafe {
            args = args.add(1);
        }
    }
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        let argc = args_vec.len();
        task.exec(all_data.as_slice(),args_vec);
        argc as isize
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
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
        return -1;
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
            return found_pid as isize
        } else {
            drop(inner);
            suspend_current_and_run_next();
        }
    }

    // ---- release current PCB automatically
}

///
pub fn sys_chdir(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    println!("chdir_path:{}  path_len:{}",path,path.len());
    if let Some(dentry) = path_to_dentry(&path){
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        task_inner.cwd = dentry;
        0
    }
    else {
        -1
    }

}


pub fn sys_brk(new_brk:  usize) -> isize {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let cur_brk = task_inner.heap_top;
    //println!("sys_brk: heap_top = {}, stack_bottom = {} new_brk:{}",task_inner.heap_top,task_inner.stack_bottom,new_brk);
    if new_brk == 0 {
        return cur_brk as isize;
    }
    
    if task_inner.max_data_addr >= new_brk && new_brk < task_inner.stack_bottom { // 利用上一次分配的多余内存
        task_inner.heap_top = new_brk;
    //    return 0;
        return new_brk as isize;
    }

    if new_brk > cur_brk {
        let user_stack_bottom = task_inner.stack_bottom;
        
        if new_brk >= user_stack_bottom -PAGE_SIZE { 
            return cur_brk as isize;
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
        return new_brk as isize;
     //   0
    }
    else if new_brk == cur_brk {
          return cur_brk as isize;
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
        return new_brk as isize;
    //   0
    }
}

pub fn sys_times(tms_ptr: *mut Tms) -> isize {
    let binding = current_task().unwrap();
    let token = current_user_token();
    let taskinner = binding.inner_exclusive_access();
    if tms_ptr.is_null(){
        return -1;
    }
    // 安全地获取一个可变引用
    let tms = translated_refmut(token, tms_ptr);
    // 当前 - 起始
    tms.tms_utime = Time::now().to_msec() as usize - taskinner.tms.tms_utime;
    tms.tms_stime = Time::now().to_msec() as usize - taskinner.tms.tms_stime;
    tms.tms_cutime = Time::now().to_msec() as usize - taskinner.tms.tms_cutime;
    tms.tms_cstime = Time::now().to_msec() as usize - taskinner.tms.tms_cstime;
    tms.tms_cutime as isize
}

pub fn sys_uname(mes: *mut Utsname) -> isize {
    let uname = unsafe {
        if mes.is_null() {
            return -1;
        }
        &mut *mes
    };
    //
    uname.copy_from(&UNAME);
    0
}


/* for nanosleep:
struct timespec {
	time_t tv_sec;        /* 秒 */
	long   tv_nsec;       /* 纳秒, 范围在0~999999999 */
};
*/
pub fn sys_nanosleep(timespec:*const TimeSpec)->isize{
    if timespec.is_null(){
        return -1;
    }
    let token = current_user_token();
    let timespec = translated_ref(token, timespec);
    if timespec.usec > 99999999{
        return -1;
    }
    let total_time = timespec.sec * 1000000000 + timespec.usec;
    let start_time = Time::now().to_nsec();
    loop{
        let current_time = Time::now().to_nsec();
        if current_time - start_time < total_time{
            suspend_current_and_run_next();
        }
        else{
            return 0;
        }
    }
}

pub fn sys_getppid()->isize{
    let current = current_task().unwrap();
    let inner = current.inner_exclusive_access();
    if inner.parent.is_none(){
        return -1;
    }
    let parent = inner.parent.clone().unwrap().upgrade().unwrap();
    let ret = parent.pid.0 as isize;
    ret
}

pub fn sys_set_tid_address(tidptr:usize)->isize{
    let current = current_task().unwrap();
    let mut inner = current.inner_exclusive_access();
    inner.tidaddress.clear_child_tid = Some(tidptr);
    tidptr as isize
}


pub fn sys_prlimit64(pid: usize,resource: i32,new_limit: *const RLimit,old_limit: *mut RLimit) -> isize {
    let task;
    let token = current_user_token();
    if pid == 0{
        task = current_task().unwrap();
    }
    else {
        if let Some(t) = crate::task::get_task_from_pid(pid){
            task = t;
        }
        else {
            return -1;
        }
    }
    let mut inner = task.inner_exclusive_access();
    let resource = Resource::new(resource);
    if resource.is_none(){
        return -1;
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
                limit = inner.fd_table_rlimit;
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
                inner.set_fd_rlimit(limit);
            },
            _=>{}
        }
    }
    return 0;
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

pub fn sys_clock_gettime(clockid: usize, tp: *mut TimeSpec) -> isize {
    if tp.is_null() {
        return -1;
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
            panic!("unsupported clock_id!");
        }
    }
    0
}

pub fn sys_exit_group(exit_code: i32) -> isize { //退出线程组，但没有子线程
    exit_current_and_run_next((exit_code & 0xFF) << 8);//posix标准退出码
    panic!("Unreachable in sys_exit!");
}

pub fn sys_get_random(buf: *mut u8, len: usize, _flags: usize) -> isize {
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
    len as isize
} 

pub fn sys_info(info: *mut SysInfo) -> isize {
    let token = current_user_token();
    let info_ref = translated_refmut(token, info);
    *info_ref = SysInfo::default();
    0
}

pub fn sys_log(log_type: usize, _buf: *mut u8, _len: usize) -> isize {
    match log_type {
        2 |3 |4 => {
            0
        }
        _ => {
            0
        }
    }
}

pub fn sys_mprotect(addr: VirtAddr, _len: usize, prot: i32) -> isize {
    if addr.addr() / PAGE_SIZE *PAGE_SIZE != addr.addr() {
        return -1;
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
    0
}
pub fn sys_kill(pid: usize, signal: u32) -> isize {
    if let Some(process) = pid2task(pid) {
        if let Some(flag) = SignalFlags::from_bits(signal) {
            process.inner_exclusive_access().signals |= flag;
            0
        } else {
            -1
        }
    } else {
        -1
    }
}
