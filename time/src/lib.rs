#![no_std]
#![no_main]

use arch::time::Time;

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
/// Describes times in seconds and microseconds.
pub struct TimeSpec {
    /// second
    pub sec: usize,
    /// microsecond
    pub usec: usize,
}

impl TimeSpec{
    pub fn to_usec(&self)->usize{
        self.sec*1000_000_000+self.usec
    }
}

#[repr(C)]
///
pub struct Tms { //记录起始时间
    /// 用户时间
    pub tms_utime: usize,
    /// 系统时间
    pub tms_stime: usize,
    /// 子进程用户时间
    pub tms_cutime: usize, 
    /// 子进程系统时间
    pub tms_cstime: usize, 
}
impl Tms {
    ///
    pub fn new() -> Self {
        Self {
            tms_utime: 0,
            tms_stime: 0,
            tms_cutime: Time::now().to_msec() as usize,
            tms_cstime: Time::now().to_msec() as usize,
        }
    }
    ///
    pub fn from_other_task(o_tms: &Tms) -> Self {
        Self {
            tms_utime: o_tms.tms_utime,
            tms_stime: o_tms.tms_stime,
            tms_cutime: Time::now().to_msec() as usize,
            tms_cstime: Time::now().to_msec() as usize,
        }
    }
}