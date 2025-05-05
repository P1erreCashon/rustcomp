//!Implementation of [`PidAllocator`]
use crate::sync::UPSafeCell;
use alloc::vec::Vec;
use lazy_static::*;
use spin::Mutex;
///Tid Allocator struct
pub struct TidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl TidAllocator {
    ///Create an empty `PidAllocator`
    pub fn new() -> Self {
        TidAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }
    ///Allocate a tid
    pub fn alloc(&mut self) -> TidHandle {
        if let Some(tid) = self.recycled.pop() {
            TidHandle(pid)
        } else {
            self.current += 1;
            TidHandle(self.current - 1)
        }
    }
    ///Recycle a pid
    pub fn dealloc(&mut self, tid: usize) {
        assert!(tid < self.current);
        assert!(
            !self.recycled.iter().any(|ppid| *ppid == tid),
            "pid {} has been deallocated!",
            tid
        );
        self.recycled.push(tid);
    }
}

lazy_static! {
/*     pub static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> =
        unsafe { UPSafeCell::new(PidAllocator::new()) }; */
        pub static ref PID_ALLOCATOR: Mutex<PidAllocator> =
        Mutex::new(PidAllocator::new());
}
///Bind pid lifetime to `PidHandle`
pub struct PidHandle(pub usize);

impl Drop for PidHandle {
    fn drop(&mut self) {
        //println!("drop pid {}", self.0);
        PID_ALLOCATOR.lock().dealloc(self.0);
    }
}
///Allocate a pid from PID_ALLOCATOR
pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.lock().alloc()
}
