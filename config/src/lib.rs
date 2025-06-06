#![no_std]
#![no_main]

//! Constants used in rCore

#[allow(unused)]
pub const USER_STACK_SIZE: usize = 4096 * 85;
pub const KERNEL_STACK_SIZE: usize = 4096 * 10;
pub const KERNEL_HEAP_SIZE: usize = 0x500_0000;
pub const USER_STACK_TOP: usize = 0x13_0000_0000;
pub const USER_MMAP_TOP: usize = 0x11_0000_0000;

pub const DL_INTERP_OFFSET: usize = 0x15_0000_0000;

pub const MAX_FD:usize = 1024;
//pub const PAGE_SIZE: usize = 0x1000;
#[allow(unused)]
pub const USER_HEAP_SIZE: usize = 0x8000;
//pub const PAGE_SIZE_BITS: usize = 0xc;

/// Use a fs block size of 512 bytes
pub const BLOCK_SZ: usize = 4096;

// The io block size of the disk layer
pub const DISK_BLOCK_SZ: usize = 512;
//pub use crate::board::CLOCK_FREQ;


#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Resource {
    // 
    CPU = 0,
    // 
    FSIZE = 1,
    // 
    DATA = 2,
    // 
    STACK = 3,
    // 
    CORE = 4,
    // 
    RSS = 5,
    // 
    NPROC = 6,
    // 
    NOFILE = 7,
    // 
    MEMLOCK = 8,
    // 
    AS = 9,
    // 
    LOCKS = 10,
    // 
    SIGPENDING = 11,
    //
    MSGQUEUE = 12,
    ///
    NICE = 13,
    ///
    RTPRIO = 14,
    ///
    RTTIME = 15,
}
impl Resource{
    pub fn new(r:i32)->Option<Self>{
        match r{
            0=>Some(Resource::CPU),
            1=>Some(Resource::FSIZE),
            2=>Some(Resource::DATA),
            3=>Some(Resource::STACK),
            4=>Some(Resource::CORE),
            5=>Some(Resource::RSS),
            6=>Some(Resource::NPROC),
            7=>Some(Resource::NOFILE),
            8=>Some(Resource::MEMLOCK),
            9=>Some(Resource::AS),
            10=>Some(Resource::LOCKS),
            11=>Some(Resource::SIGPENDING),
            12=>Some(Resource::MSGQUEUE),
            13=>Some(Resource::NICE),
            14=>Some(Resource::RTPRIO),
            15=>Some(Resource::RTTIME),
            _=>None
        }
    }
}
/// 
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RLimit {
    ///
    pub rlimit_cur: usize,
    ///
    pub rlimit_max: usize,
}
