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
mod info;
mod fdtable;
mod action;

use crate::fs::open_file;
use alloc::sync::Arc;
use arch::shutdown;
use arch::KContext;
use arch::TrapFrameArgs;
use lazy_static::*;
pub use manager::{fetch_task, TaskManager,wakeup_task, pid2task, insert_into_pid2task, remove_from_pid2task};
pub use task::{TaskControlBlock, TaskStatus, MapFdControl};
pub use info::{Utsname,SysInfo,UNAME};
pub use time::{Tms,TimeSpec};
pub use fdtable::{FdTable,Fd,FdFlags};
use vfs_defs::OpenFlags;
pub use manager::add_task;
pub use pid::{pid_alloc,  PidAllocator, PidHandle};
pub use tid::{tid_alloc , TidAllocator, TidHandle, TidAddress};
pub use processor::{
    current_task,  current_user_token, run_tasks, schedule, take_current_task,
    Processor, PROCESSOR
};
pub use signal::{SignalFlags, SigAction};
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
pub fn check_pending_signals() {
    let task = match current_task() {
        Some(task) => task,
        None => return,
    };

    let mut task_inner = task.inner_exclusive_access();

    // 如果当前正在处理一个信号，延迟处理其他信号
    if task_inner.handling_sig != -1 {
        println!(
            "[kernel] Signal handling in progress (sig={}), deferring other signals",
            task_inner.handling_sig
        );
        return;
    }

    if task_inner.signal_queue.is_empty() {
        return;
    }

    let sig = task_inner.signal_queue[0];
    let signal = match SignalFlags::from_bits(1 << sig) {
        Some(signal) => signal,
        None => {
            println!("[kernel] check_pending_signals: Signal {} not in SignalFlags, removing", sig);
            task_inner.signal_queue.remove(0);
            return;
        }
    };

    if !task_inner.signals.contains(signal) {
        println!("[kernel] Signal {} not in task.signals, removing from queue", sig);
        task_inner.signal_queue.remove(0);
        return;
    }

    println!(
        "[kernel] Signal {} found in task.signals, mask: {:?}, handling_sig: {}",
        sig, task_inner.signal_mask, task_inner.handling_sig
    );

    if task_inner.signal_mask.contains(signal) {
        // 移除日志打印
        // println!("[kernel] Signal {} is masked by signal_mask", sig);
        return;
    }

    let mut masked = true;
    let handling_sig = task_inner.handling_sig;
    if handling_sig == -1 {
        masked = false;
    } else {
        let handling_sig = handling_sig as usize;
        if !task_inner.signal_actions.table[handling_sig]
            .mask
            .contains(signal)
        {
            masked = false;
        }
    }
    if masked {
        println!("[kernel] Signal {} is masked by handling_sig {}", sig, handling_sig);
        return;
    }

    println!("[kernel] Signal {} is not masked, proceeding to handle", sig);

    task_inner.signal_queue.remove(0);

    drop(task_inner);
    drop(task);

    if signal == SignalFlags::SIGKILL
        || signal == SignalFlags::SIGSTOP
        || signal == SignalFlags::SIGCONT
    {
        call_kernel_signal_handler(sig, signal);
    } else {
        call_user_signal_handler(sig, signal);
    }
}

/// 处理内核态信号
pub fn call_kernel_signal_handler(sig: usize, signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    match signal {
        SignalFlags::SIGSTOP => {
            task_inner.frozen = true;
            task_inner.signals.remove(SignalFlags::SIGSTOP);
            println!("[kernel] Task {} stopped by SIGSTOP", task.getpid());
        }
        SignalFlags::SIGCONT => {
            task_inner.frozen = false;
            task_inner.signals.remove(SignalFlags::SIGCONT);
            println!("[kernel] Task {} continued by SIGCONT", task.getpid());
        }
        SignalFlags::SIGKILL => {
            task_inner.killed = true;
            println!("[kernel] Task {} killed by SIGKILL", task.getpid());
        }
        _ => {
            task_inner.killed = true;
            println!("[kernel] Task {} terminated by signal {}", task.getpid(), sig);
        }
    }
}

/// 处理用户态信号
pub fn call_user_signal_handler(sig: usize, _signal: SignalFlags) {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();

    let handler = task_inner.signal_actions.table[sig].handler;
    if handler == 0 {
        println!("[kernel] No handler for signal {}, ignoring", sig);
        return;
    }

    // 保存当前的 trap 上下文
    task_inner.trap_ctx_backup = Some(task_inner.get_trap_cx().clone());
    task_inner.signal_mask_backup = task_inner.signal_mask;

    // 设置信号掩码
    task_inner.signal_mask = task_inner.signal_actions.table[sig].mask;

    task_inner.handling_sig = sig as isize;

    let trap_ctx = task_inner.get_trap_cx();
    trap_ctx.sepc = handler;
    trap_ctx.x[10] = (sig as i32) as usize; // 修正：将 sig 转换为 i32 后再转换为 usize

    println!("[kernel] Calling user signal handler for signal {} at {:#x}", sig, handler);
}

pub fn check_signals_error_of_current() -> Option<(isize, &'static str)> {
    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();
    if task_inner.killed {
        return Some((-1, "Killed"));
    }
    None
}