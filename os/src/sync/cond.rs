use spin::Mutex;
use alloc::{collections::VecDeque, sync::Arc};
use super::up::IntrCell;
use crate::task::{
    block_current_and_run_next, block_current_task, current_task, wakeup_task, TaskContext,
    TaskControlBlock,
};
///
pub struct Cond{
    ///
    pub wait_queue:IntrCell<VecDeque<Arc<TaskControlBlock>>>,
}

impl Cond{
    ///
    pub fn new()->Self{
        Self{
            wait_queue:IntrCell::new(VecDeque::new())
        }
    }
    ///
    pub fn signal(&self){
        let mut inner = self.wait_queue.lock();
        if let Some(task) = inner.pop_front(){
            wakeup_task(task);
        }
    }
    ///
    pub fn wait_no_sched(&self) -> *mut TaskContext {
        let mut inner = self.wait_queue.lock();
        inner.push_back(current_task().unwrap());
        drop(inner);
        block_current_task()
    }
}