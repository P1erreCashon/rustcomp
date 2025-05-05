use arch::addr::PhysAddr;
use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Weak},
};
use system_result::SysResult;
use super::{TaskControlBlock,block_current_and_run_next,current_task,wakeup_task};
use lazy_static::*;
use sync::Mutex;

lazy_static!{
    pub static ref FUTEX_Q:Mutex<BTreeMap<FutexKey,FutexBucket>> = 
        Mutex::new(BTreeMap::new());
}

type FutexBucket = VecDeque<Weak<TaskControlBlock>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FutexKey{
    paddr:PhysAddr,
    pid:usize
}

impl FutexKey{
    pub fn new(paddr:PhysAddr,pid:usize)->Self{
        Self{
            paddr,pid
        }
    }
}

pub fn futex_wait(futexkey:FutexKey)->SysResult<isize>{
    let mut futex_q = FUTEX_Q.lock();
    let task = current_task().unwrap();
  //  println!("cur futex wait {} paddr:{:x} keypid:{}",task.getpid(),futexkey.paddr.addr(),futexkey.pid);
    if let Some(bucket) = futex_q.get_mut(&futexkey) {
        bucket.push_back(Arc::downgrade(&task));
    } else {
        futex_q.insert(futexkey, {
            let mut bucket = VecDeque::new();
            bucket.push_back(Arc::downgrade(&task));
            bucket
        });
    }
    drop(task);
    drop(futex_q);
    block_current_and_run_next();
   // let task = current_task().unwrap();
  //  let inner = task.inner_exclusive_access();
   /*// woke by signal
    if !inner
        .signals.
        .difference(task_inner.sig_mask)
        .is_empty()
    {cur futex wait 3 paddr:834d9368 keypid:3
    cur futex wait 4 paddr:834d9368 keypid:4
        return Err(SysErrNo::EINTR);
    } */ 
    Ok(0)

}

pub fn futex_wake(futexkey:FutexKey,max_size:usize)->usize{
    
    let mut futex_q = FUTEX_Q.lock();
    let mut num = 0;
    if let Some(queue) = futex_q.get_mut(&futexkey) {
        loop {
            if num >= max_size as usize {
                break;
            }
            if let Some(weak_task) = queue.pop_front() {
                if let Some(task) = weak_task.upgrade() {
                    //debug!("wake up task {}", task.pid());
                    wakeup_task(task);
                    num += 1;
                }
            } else {
                break;
            }
        }
    }//println!("futex wake paddr:{:x} keypid:{} waked:{}",futexkey.paddr.addr(),futexkey.pid,num); //futex wake paddr:8341e550 keypid:0
    num
}

pub fn futex_requeue(old_key: FutexKey, max_num: i32, new_key: FutexKey, max_num2: i32)->usize{
    let mut futex_q = FUTEX_Q.lock();
    let mut num = 0;
    let mut num2 = 0;
    let mut tmp = VecDeque::new();
    if let Some(queue) = futex_q.get_mut(&old_key) {
        while let Some(weak_task) = queue.pop_front() {
            if let Some(task) = weak_task.upgrade() {
                if num < max_num {
                    wakeup_task(task);
                    num += 1;
                } else if num2 < max_num2 {
                    tmp.push_back(Arc::downgrade(&task));
                    num2 += 1;
                }
            }
        }
    }
    if !tmp.is_empty() {
        futex_q
            .entry(new_key)
            .or_insert_with(VecDeque::new)
            .extend(tmp);
    }
    num as usize

}


