//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of [`PidAllocator`] called `PID_ALLOCATOR` allocates
//! pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
//mod context;
mod manager;
mod pid;
mod processor;
mod signal;
//mod switch;
#[allow(clippy::module_inception)]
#[allow(rustdoc::private_intra_doc_links)]
mod task;
mod aux;
mod tid;

use crate::fs::open_file;
use alloc::sync::Arc;
use arch::shutdown;
use arch::KContext;
use arch::TrapFrameArgs;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager,wakeup_task,get_task_from_pid, pid2task, insert_into_pid2task, remove_from_pid2task}};
pub use task::{TaskControlBlock, TaskStatus, Tms, Utsname,TimeSpec, MapFdControl};
use vfs_defs::OpenFlags;
pub use manager::add_task;
pub use pid::{pid_alloc,  PidAllocator, PidHandle};
pub use tid::{tid_alloc , TidAllocator, TidHandle, TidAddress};
pub use processor::{
    current_task,  current_user_token, run_tasks, schedule, take_current_task,
    Processor, PROCESSOR
};
pub use signal::SignalFlags; // 显式导出 SignalFlags
pub use aux::*;

const MODULE_LEVEL:log::Level = log::Level::Trace;
pub const MAX_SIG: usize = 31;

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();
    log_info!("task:{} suspend",task.getpid());//最好不要显示这个
    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut KContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}
/* 
/// This function must be followed by a schedule
pub fn block_current_task() -> *mut KContext{
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.task_status = TaskStatus::Blocked;
    &mut task_inner.task_cx as *mut KContext
}
///
pub fn block_current_and_run_next() {
    let task_cx_ptr = block_current_task();
    schedule(task_cx_ptr);
}*/

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// 终止当前任务并切换到下一个任务
pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let pid = task.getpid();
    // 移除 PID2TCB 中的引用
    remove_from_pid2task(pid);
    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    inner.children.clear();
    inner.memory_set.recycle_data_pages();
    drop(inner);
    drop(task);
    let mut _unused = KContext::blank();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    ///Globle process that init user shell
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new({
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        TaskControlBlock::new(v.as_slice())
    });
}
///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// 检查并处理当前任务的信号
/// 检查并处理当前任务的信号
pub fn handle_signals() {
    loop {
        check_pending_signals();
        let killed = match current_task() {
            Some(task) => {
                let task_inner = task.inner_exclusive_access();
                task_inner.killed
            }
            None => false, // 如果没有当前任务，假设未被杀死
        };
        if killed {
            break;
        }
        break;
    }
}

/// 检查当前任务是否有未处理的信号，并调用信号处理函数
fn check_pending_signals() {
    // 如果没有当前任务，直接返回
    let task = match current_task() {
        Some(task) => task,
        None => return,
    };
    for sig in 0..=MAX_SIG {
        let task_inner = task.inner_exclusive_access();
        let signal = match SignalFlags::from_bits(1 << sig) {
            Some(signal) => signal,
            None => continue, // 跳过无效信号
        };
        if task_inner.signals.contains(signal) {
            drop(task_inner);
            drop(task);
            call_kernel_signal_handler(sig, signal);
            return;
        }
    }
}

/// 处理内核态信号
fn call_kernel_signal_handler(_sig: usize, _signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    // 默认动作：终止进程
    task_inner.killed = true;
    // 不要移除信号，保留以便 check_error 使用
    // task_inner.signals.remove(signal);
}

/// 检查当前任务是否因信号被终止，并返回错误码和消息
///
/// 如果任务被标记为 `killed`，返回 `Some((error_code, error_message))`，否则返回 `None`。
pub fn check_signals_error_of_current() -> Option<(isize, &'static str)> {
    let task = match current_task() {
        Some(task) => task,
        None => return None,
    };
    let inner = task.inner_exclusive_access();
    if inner.killed {
        if let Some((code, msg)) = inner.signals.check_error() {
            return Some((code as isize, msg));
        }
        Some((-1, "Process killed by signal"))
    } else {
        None
    }
}