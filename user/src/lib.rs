#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

extern crate alloc;
#[macro_use]
extern crate bitflags;

use alloc::vec::Vec;
use buddy_system_allocator::LockedHeap;
use syscall::*;
use core::fmt;
use core::str;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
extern "C" {
    static HEAP_START: u8;
    static HEAP_END: u8;
}

pub const USER_HEAP_SIZE: usize = 32768;
/// 用于sys_times
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
    pub fn show(&self) {
        println!("sys:{}, user:{}", self.tms_stime, self.tms_cstime);
    }
}
/// 用于sys_uname
pub struct Utsname {
    ///
    pub sysname: [u8; 65],
    ///
    pub nodename: [u8; 65],
    ///
    pub release: [u8; 65],
    ///
    pub version: [u8; 65],
    ///
    pub machine: [u8; 65],
    ///
    pub domainname: [u8; 65],
}
impl Utsname {
    /// Prints the contents of the Utsname structure.
    pub fn show(&self) {
        println!("Sysname: {}", bytes_to_string(&self.sysname));
        println!("Nodename: {}", bytes_to_string(&self.nodename));
        println!("Release: {}", bytes_to_string(&self.release));
        println!("Version: {}", bytes_to_string(&self.version));
        println!("Machine: {}", bytes_to_string(&self.machine));
        println!("Domainname: {}", bytes_to_string(&self.domainname));
        
    }
}

/// Helper function to convert a byte array to a string.
fn bytes_to_string(bytes: &[u8; 65]) -> &str {
    // Find the first null byte (0) to determine the end of the string
    let null_pos = bytes.iter().position(|&x| x == 0).unwrap_or(bytes.len());
    // Convert the slice to a string
    str::from_utf8(&bytes[..null_pos]).unwrap_or("<invalid UTF-8>")
}
//pub static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

// 当前堆顶地址
static HEAP_TOP: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start(argc: usize, argv: usize) -> ! {
    // 初始化堆
    let heap_start = unsafe { &HEAP_START as *const u8 as usize };
    let heap_end = unsafe { &HEAP_END as *const u8 as usize };
    HEAP_TOP.store(heap_start, Ordering::SeqCst);

    unsafe {
        HEAP.lock()
            .init(heap_start, USER_HEAP_SIZE);
    }

    let mut v: Vec<&'static str> = Vec::new();
    for i in 0..argc {
        let str_start =
            unsafe { ((argv + i * core::mem::size_of::<usize>()) as *const usize).read_volatile() };
        let len = (0usize..)
            .find(|i| unsafe { ((str_start + *i) as *const u8).read_volatile() == 0 })
            .unwrap();
        v.push(
            core::str::from_utf8(unsafe {
                core::slice::from_raw_parts(str_start as *const u8, len)
            })
            .unwrap(),
        );
    }
    exit(main());
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

bitflags! {
    pub struct OpenFlags: u32 {
        const RDONLY = 0;
        const WRONLY = 1 << 0;
        const RDWR = 1 << 1;
        const CREATE = 1 << 6;
        const TRUNC = 1 << 10;
    }
}
pub fn chdir(path: &str) -> isize {
    sys_chdir(path)
}

pub fn unlink(path: &str) -> isize {
    sys_unlink(path)
}

pub fn link(old_path: &str,new_path: &str) -> isize {
    sys_link(old_path, new_path)
}
pub fn mkdir(path: &str) -> isize {
    sys_mkdir(path)
}
pub fn open(path: &str, flags: OpenFlags) -> isize {
    sys_open(path, flags.bits)
}
pub fn close(fd: usize) -> isize {
    sys_close(fd)
}
pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}
pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code);
}
pub fn yield_() -> isize {
    sys_yield()
}
pub fn get_time() -> isize {
    sys_get_time()
}
pub fn getpid() -> isize {
    sys_getpid()
}
pub fn fork() -> isize {
    sys_fork()
}
pub fn exec(path: &str, args: &[*const u8]) -> isize {
    sys_exec(path, args)
}
pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => {
                yield_();
            }
            // -1 or a real pid
            exit_pid => return exit_pid,
        }
    }
}
pub fn sleep(period_ms: usize) {
    let start = sys_get_time();
    while sys_get_time() < start + period_ms as isize {
        sys_yield();
    }
}

pub fn pipe(pipe_fd: &mut [usize]) -> isize { 
    sys_pipe(pipe_fd) 
}

pub fn brk(new_brk: usize) -> isize {
    sys_brk(new_brk)
}

pub fn getcwd(buf: *mut u8, size: usize) -> isize {
    sys_getcwd(buf, size)
}

pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}

pub fn dup3(old: usize, new: usize) -> isize {
    sys_dup3(old, new)
}

pub fn times(tms: *mut Tms) -> isize {
    sys_times(tms)
}

pub fn uname(mes: *mut Utsname) -> isize {
    sys_uname(mes)
}