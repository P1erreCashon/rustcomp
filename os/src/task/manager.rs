//!Implementation of [`TaskManager`]
use super::{TaskControlBlock,TaskStatus};
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use lazy_static::*;
use spin::Mutex;

const MODULE_LEVEL:log::Level = log::Level::Trace;

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,//why use Arc:TaskManager->TCB & TCB.children->TCB & TaskManager creates Arc<TCB>
    block_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            block_queue:VecDeque::new(),
        }
    }
    ///Add a task to `TaskManager`
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    ///Add a blocked task to `TaskManager`
    pub fn add_block(&mut self, task: Arc<TaskControlBlock>) {
        self.block_queue.push_back(task);
    }
    ///Remove the first task and return it,or `None` if `TaskManager` is empty
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    /// 根据 PID 查找任务（仅在就绪队列中查找）
    pub fn find_task_by_tid(&self, tid: usize) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.iter().find(|task| task.gettid() == tid).cloned()
    }
    pub fn remove_blocked_task_by_tid(&mut self, tid: usize){
        if let Some(pos) = self.block_queue.iter().position(|task| task.gettid() == tid){
            self.block_queue.remove(pos);
        }
    }
}

lazy_static! {
/*     pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) }; */
        pub static ref TASK_MANAGER: Mutex<TaskManager> =
        Mutex::new(TaskManager::new());
        // 新增全局 PID 到 TaskControlBlock 的映射
        pub static ref PID2TCB: Mutex<BTreeMap<usize, Arc<TaskControlBlock>>> = 
        Mutex::new(BTreeMap::new());
}
///Interface offered to add task
pub fn add_task(task: Arc<TaskControlBlock>) {
    log_debug!("add task:{} to ready queue",task.gettid());
    TASK_MANAGER.lock().add(task.clone());
    // 同时添加到 PID2TCB
    PID2TCB.lock().insert(task.gettid(), task);
}
//
pub fn add_blocked_task(task: Arc<TaskControlBlock>) {
    log_debug!("add task:{} to block queue",task.gettid());
    TASK_MANAGER.lock().add_block(task.clone());
}
///
pub fn wakeup_task(task: Arc<TaskControlBlock>) {
    let mut task_inner = task.inner_exclusive_access();
    task_inner.task_status = TaskStatus::Ready;
    TASK_MANAGER.lock().remove_blocked_task_by_tid(task.gettid());
    drop(task_inner);
    add_task(task);
}
///Interface offered to pop the first task
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.lock().fetch()
}

/// 根据 PID 查找任务控制块
pub fn tid2task(tid: usize) -> Option<Arc<TaskControlBlock>> {
    let map = PID2TCB.lock();
    map.get(&tid).map(Arc::clone)
}

/// 将任务插入 PID2TCB 映射
pub fn insert_into_tid2task(tid: usize, task: Arc<TaskControlBlock>) {
    PID2TCB.lock().insert(tid, task);
}

/// 从 PID2TCB 映射中移除任务
pub fn remove_from_tid2task(tid: usize) {
    let mut map = PID2TCB.lock();
    if map.remove(&tid).is_none() {
        // log::warn!("cannot find pid {} in pid2task, already removed?", pid);
        // 不再 panic，而是记录警告
    }
}

#[allow(unused)]
pub fn deb(){
    let manager = TASK_MANAGER.lock();
    for task in manager.block_queue.iter(){
        println!("block q task:{} ",task.gettid());
        for child in task.inner_exclusive_access().children.iter(){
            println!("task {} has child {}",task.gettid(),child.gettid());
        }
    }
    for task in manager.ready_queue.iter(){
        println!("ready q task:{} ",task.gettid());
        for child in task.inner_exclusive_access().children.iter(){
            println!("task {} has child {}",task.gettid(),child.gettid());
        }
    }
    drop(manager);
    let t2t = PID2TCB.lock();
    for (id,_) in t2t.iter(){
        println!("t2t has {}",id);
    }
}
