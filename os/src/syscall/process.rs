use crate::fs::{open_file,path_to_dentry,path_to_father_dentry,create_file};
use crate::mm::{translated_ref, translated_refmut, translated_str, MapType};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next,
    suspend_current_and_run_next,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use arch::time::Time;
use arch::TrapFrameArgs;
use crate::drivers::BLOCK_DEVICE;
use vfs_defs::{DiskInodeType,OpenFlags,Dentry};
use crate::config::{PAGE_SIZE, UNAME};
use arch::addr::{PhysPage, VirtAddr, VirtPage};
use crate::mm::{MapPermission, MapArea};
use arch::pagetable::MappingSize;
use crate::task::{Tms, Utsname};

const MODULE_LEVEL:log::Level = log::Level::Trace;

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time() -> isize {
//    get_time_ms() as isize
    Time::now().to_msec() as isize
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();    
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    //trap_cx.x[10] = 0;
    trap_cx[TrapFrameArgs::RET] = 0;
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
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
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
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        if exit_code_ptr != core::ptr::null_mut(){
            *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        }
        found_pid as isize
    } else {
        -2
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

///
pub fn sys_link(old_path: *const u8,new_path:*const u8) -> isize {
    let token = current_user_token();
    let old_path = translated_str(token, old_path);
    let new_path = translated_str(token, new_path);
    if let Some(dentry) = path_to_dentry(&old_path){
        if dentry.is_dir(){
            drop(dentry);
            return -1
        }
        let mut name = String::new();
        if let Some(father_dentry) = path_to_father_dentry(&new_path,&mut name){
            let r = father_dentry.lookup(name.as_str());
            if r.is_ok(){//EEXIST
                return -1;
            }
            let new_dentry = father_dentry.find_or_create(name.as_str(), DiskInodeType::File);
            if let Err(e) = dentry.link(&new_dentry){
                println!("link err: {:?}",e);
                return -1;
            }
            return 0;
        }
        return -1;
    }
    else {
        -1
    }

}

///
pub fn sys_unlink(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    println!("unlink:{}",path);
    let mut name = String::new();
    if let Some(father) = path_to_father_dentry(&path,&mut name){
        if name.eq(".") || name.eq(".."){
            return -1
        }
        if let Some(old) = path_to_dentry(&path){
            if father.unlink(&old).is_err(){
                return -1;
            }
            return 0;
        }
        return -1;
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
        return 0;
    }

    if new_brk > cur_brk {
        let user_stack_bottom = task_inner.stack_bottom;
        
        if new_brk >= user_stack_bottom -PAGE_SIZE { 
            return -1;
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
        
        0
    }
    else if new_brk == cur_brk {
        -1
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
        
        0
    }
}

pub fn sys_times(tms_ptr: *mut Tms) -> isize {
    let binding = current_task().unwrap();
    let taskinner = binding.inner_exclusive_access();
    // 安全地获取一个可变引用
    let tms = unsafe {
        if tms_ptr.is_null() {
            return -1;
        }
        &mut *tms_ptr
    };
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