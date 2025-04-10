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
use core::ptr;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
extern "C" {
    static HEAP_START: u8;
    static HEAP_END: u8;
}

pub const USER_HEAP_SIZE: usize = 32768;

pub const SIGDEF: i32 = 0;
pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGTRAP: i32 = 5;
pub const SIGABRT: i32 = 6;
pub const SIGBUS: i32 = 7;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGSEGV: i32 = 11;
pub const SIGUSR2: i32 = 12;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;
pub const SIGSTKFLT: i32 = 16;
pub const SIGCHLD: i32 = 17;
pub const SIGCONT: i32 = 18;
pub const SIGSTOP: i32 = 19;
pub const SIGTSTP: i32 = 20;
pub const SIGTTIN: i32 = 21;
pub const SIGTTOU: i32 = 22;
pub const SIGURG: i32 = 23;
pub const SIGXCPU: i32 = 24;
pub const SIGXFSZ: i32 = 25;
pub const SIGVTALRM: i32 = 26;
pub const SIGPROF: i32 = 27;
pub const SIGWINCH: i32 = 28;
pub const SIGIO: i32 = 29;
pub const SIGPWR: i32 = 30;
pub const SIGSYS: i32 = 31;

// 新增：定义 SIG_BLOCK 和 SIG_UNBLOCK 常量
pub const SIG_BLOCK: i32 = 0;
pub const SIG_UNBLOCK: i32 = 1;
pub const SIG_SETMASK: i32 = 2;

// 新增：定义 TimeSpec 结构体
#[repr(C)]
#[derive(Debug)]
pub struct TimeSpec {
    pub tv_sec: isize,
    pub tv_nsec: isize,
}

pub type SignalHandler = unsafe extern "C" fn(i32);

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

/// Action for a signal
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    pub handler: usize,
    pub mask: SignalFlags,
}

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

bitflags! {
    pub struct SignalFlags: u32 {
        /// 默认信号处理，信号编号为 0
        const SIGDEF  = 1 << 0;  // 0
        /// 挂起信号（Hangup），信号编号为 1
        const SIGHUP  = 1 << 1;  // 1
        /// 中断信号（Interrupt），通常由 Ctrl+C 触发，信号编号为 2
        const SIGINT  = 1 << 2;  // 2
        /// 退出信号（Quit），信号编号为 3
        const SIGQUIT = 1 << 3;  // 3
        /// 非法指令信号（Illegal Instruction），信号编号为 4
        const SIGILL  = 1 << 4;  // 4
        /// 陷阱信号（Trap），信号编号为 5
        const SIGTRAP = 1 << 5;  // 5
        /// 终止信号（Abort），通常由程序错误触发，信号编号为 6
        const SIGABRT = 1 << 6;  // 6
        /// 总线错误信号（Bus Error），信号编号为 7
        const SIGBUS  = 1 << 7;  // 7
        /// 浮点异常信号（Floating-Point Exception），信号编号为 8
        const SIGFPE  = 1 << 8;  // 8
        /// 杀死信号（Kill），信号编号为 9
        const SIGKILL = 1 << 9;  // 9
        /// 用户定义信号 1，信号编号为 10
        const SIGUSR1 = 1 << 10; // 10
        /// 分段错误信号（Segmentation Violation），通常由非法内存访问触发，信号编号为 11
        const SIGSEGV = 1 << 11; // 11
        /// 用户定义信号 2，信号编号为 12
        const SIGUSR2 = 1 << 12; // 12
        /// 管道错误信号（Broken Pipe），信号编号为 13
        const SIGPIPE = 1 << 13; // 13
        /// 闹钟信号（Alarm Clock），信号编号为 14
        const SIGALRM = 1 << 14; // 14
        /// 终止信号（Terminate），信号编号为 15
        const SIGTERM = 1 << 15; // 15
        /// 堆栈错误信号（Stack Fault），信号编号为 16
        const SIGSTKFLT = 1 << 16; // 16
        /// 子进程状态改变信号（Child Status Changed），信号编号为 17
        const SIGCHLD = 1 << 17; // 17
        /// 继续信号（Continue），信号编号为 18
        const SIGCONT = 1 << 18; // 18
        /// 停止信号（Stop），信号编号为 19
        const SIGSTOP = 1 << 19; // 19
        /// 终端停止信号（Terminal Stop），信号编号为 20
        const SIGTSTP = 1 << 20; // 20
        /// 终端输入信号（Terminal Input），信号编号为 21
        const SIGTTIN = 1 << 21; // 21
        /// 终端输出信号（Terminal Output），信号编号为 22
        const SIGTTOU = 1 << 22; // 22
        /// 紧急信号（Urgent Condition），信号编号为 23
        const SIGURG  = 1 << 23; // 23
        /// CPU 时间限制信号（CPU Time Limit Exceeded），信号编号为 24
        const SIGXCPU = 1 << 24; // 24
        /// 文件大小限制信号（File Size Limit Exceeded），信号编号为 25
        const SIGXFSZ = 1 << 25; // 25
        /// 虚拟定时器信号（Virtual Timer Expired），信号编号为 26
        const SIGVTALRM = 1 << 26; // 26
        /// 性能分析定时器信号（Profiling Timer Expired），信号编号为 27
        const SIGPROF = 1 << 27; // 27
        /// 窗口大小改变信号（Window Size Change），信号编号为 28
        const SIGWINCH = 1 << 28; // 28
        /// I/O 信号（I/O Possible），信号编号为 29
        const SIGIO   = 1 << 29; // 29
        /// 电源故障信号（Power Failure），信号编号为 30
        const SIGPWR  = 1 << 30; // 30
        /// 系统调用错误信号（Bad System Call），信号编号为 31
        const SIGSYS  = 1 << 31; // 31
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
pub fn kill(pid: usize, signal: i32) -> isize {
    sys_kill(pid, signal)
}
pub fn sigaction(
    signum: i32,
    act: Option<&SigAction>,
    oldact: Option<&mut SigAction>,
) -> isize {
    sys_sigaction(
        signum,
        act.map_or(ptr::null(), |a| a as *const SigAction),
        oldact.map_or(ptr::null_mut(), |a| a as *mut SigAction),
    )
}
pub fn sigprocmask(how: i32, set: &u32, oldset: *mut u32) -> isize {
    sys_sigprocmask(how, set, oldset)
}
pub fn sigreturn() -> isize {
    sys_sigreturn()
}
pub fn tgkill(tgid: usize, tid: usize, sig: i32) -> isize {
    sys_tgkill(tgid, tid, sig)
}
pub fn get_time() -> isize {
    sys_get_time()
}
pub fn getpid() -> usize {
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

pub fn random(buf: *mut u8, len: usize, flags: usize) -> isize {
    sys_random(buf, len, flags)
}

pub fn signal(signum: i32, handler: SignalHandler) -> usize {
    let act = SigAction {
        handler: handler as usize,
        mask: SignalFlags::empty(),
    };
    let mut old_act = SigAction {
        handler: 0,
        mask: SignalFlags::empty(),
    };
    let ret = sigaction(signum, Some(&act), Some(&mut old_act));
    if ret < 0 {
        0 // 失败时返回 0
    } else {
        old_act.handler
    }
}

// 新增：实现 nanosleep 函数
pub fn nanosleep(req: &TimeSpec, rem: *mut TimeSpec) -> isize {
    sys_nanosleep(req, rem)
}