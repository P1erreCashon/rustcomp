use lazy_static::*;
use spin::Mutex;
use alloc::vec::Vec;
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
            TidHandle(tid)
        } else {
            self.current += 1;
            TidHandle(self.current - 1)
        }
    }
    ///Recycle a pid
    pub fn dealloc(&mut self, tid: usize) {
        assert!(tid < self.current);
        assert!(
            !self.recycled.iter().any(|ptid| *ptid == tid),
            "tid {} has been deallocated!",
            tid
        );
        self.recycled.push(tid);
    }
}

lazy_static! {
        pub static ref TID_ALLOCATOR: Mutex<TidAllocator> =
        Mutex::new(TidAllocator::new());
}
///Bind pid lifetime to `TidHandle`
pub struct TidHandle(pub usize);

impl Drop for TidHandle {
    fn drop(&mut self) {
        TID_ALLOCATOR.lock().dealloc(self.0);
    }
}
///Allocate a pid from PID_ALLOCATOR
pub fn tid_alloc() -> TidHandle {
    TID_ALLOCATOR.lock().alloc()
}

///
pub struct TidAddress {
    ///
    pub set_child_tid: Option<usize>,
    ///
    pub clear_child_tid: Option<usize>,
}

impl TidAddress {
    ///
    pub fn new() -> Self {
        Self {
            set_child_tid: None,
            clear_child_tid: None,
        }
    }
}