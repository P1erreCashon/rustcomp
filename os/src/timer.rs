use crate::task::TaskControlBlock;
use crate::task::wakeup_task;
use crate::task::TimeSpec;
use alloc::{
    collections::BinaryHeap,
    sync::{Arc, Weak},
};
use core::cmp::Ordering;
use lazy_static::*;
use sync::Mutex;
use arch::time::{self, Time};


#[derive(Debug, PartialEq, Eq)]
pub enum TimerType {
    Futex,
    StoppedTask,
}

pub struct TimerCondVar {
    pub expire: TimeSpec,
    pub task: Weak<TaskControlBlock>,
    pub kind: TimerType,
}
impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire.sec == other.expire.sec && self.expire.usec == other.expire.usec
    }
}
impl Eq for TimerCondVar {}
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = -(self.expire.to_usec() as isize);
        let b = -(other.expire.to_usec() as isize);
        Some(a.cmp(&b))
    }
}
impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

lazy_static!{
    pub static ref TIMERS: Mutex<BinaryHeap<TimerCondVar>> =
        Mutex::new(BinaryHeap::<TimerCondVar>::new());
}


pub fn add_futex_timer(expire: TimeSpec, task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.lock();
    timers.push(TimerCondVar {
        expire,
        task: Arc::downgrade(&task),
        kind: TimerType::Futex,
    });
}

pub fn add_stopped_task_timer(expire: TimeSpec, task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.lock();
    timers.push(TimerCondVar {
        expire,
        task: Arc::downgrade(&task),
        kind: TimerType::StoppedTask,
    });
}

pub fn check_futex_timer() {
    let mut timers = TIMERS.lock();
    let current = Time::now().to_nsec();
    while let Some(timer) = timers.peek() {
        // debug!("expire={:?}, current={:?}", timer.expire, current);
        if timer.expire.to_usec() <= current {
            if let Some(task) = timer.task.upgrade() {
                if timer.kind == TimerType::Futex {
                    // 调用 wakeup_task 唤醒超时线程
                    wakeup_task(Arc::clone(&task));
                } else if timer.kind == TimerType::StoppedTask {
                    todo!()
                    // wakeup_stopped_task(Arc::clone(&task));
                }
            }
            timers.pop();
        } else {
            break;
        }
    }
}
