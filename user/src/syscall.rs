use core::arch::asm;
use crate::{Tms, Utsname};
use crate::SigAction;
use crate::SignalFlags;
use crate::TimeSpec;


const SYSCALL_CHDIR: usize = 49;
const SYSCALL_GETCWD: usize =17;
const SYSCALL_LINK: usize = 37;
const SYSCALL_UNLINK: usize = 18;
const SYSCALL_MKDIR: usize = 34;
const SYSCALL_DUP: usize = 23;
const SYSCALLDUP3: usize = 24;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_KILL: usize = 129;
const SYSCALL_TGKILL: usize = 131;
const SYSCALL_SIGACTION: usize = 134;
const SYSCALL_SIGPROCMASK: usize = 135;
const SYSCALL_SIGRETURN: usize = 139;
const SYSCALL_TIMES: usize =153;
const SYSCALL_UNAME: usize = 160;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_BRK: usize = 214;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_GET_RANDOM: usize = 278;

#[cfg(target_arch = "riscv64")]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}

#[cfg(target_arch = "loongarch64")]
fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "syscall 0",
            inlateout("$r4") args[0] => ret,
            in("$r5") args[1],
            in("$r6") args[2],
            in("$r11") id
        );
    }
    ret
}

pub fn sys_chdir(path: &str) -> isize {
    syscall(SYSCALL_CHDIR, [path.as_ptr() as usize, 0, 0])
}

pub fn sys_unlink(path: &str) -> isize {
    syscall(SYSCALL_UNLINK, [path.as_ptr() as usize, 0, 0])
}

pub fn sys_link(old_path: &str,new_path: &str) -> isize {
    syscall(SYSCALL_LINK, [old_path.as_ptr() as usize, new_path.as_ptr() as usize, 0])
}

pub fn sys_mkdir(path: &str) -> isize {
    syscall(SYSCALL_MKDIR, [path.as_ptr() as usize, 0, 0])
}

pub fn sys_open(path: &str, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, [path.as_ptr() as usize, flags as usize, 0])
}

pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(
        SYSCALL_READ,
        [fd, buffer.as_mut_ptr() as usize, buffer.len()],
    )
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0]);
    panic!("sys_exit never returns!");
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

pub fn sys_kill(pid: usize, signal: i32) -> isize {
    syscall(SYSCALL_KILL, [pid, signal as usize, 0])
}

pub fn sys_sigaction(signum: i32, act: *const SigAction, oldact: *mut SigAction) -> isize {
    syscall(SYSCALL_SIGACTION, [signum as usize, act as usize, oldact as usize])
}

pub fn sys_sigprocmask(how: i32, set: *const u32, oldset: *mut u32) -> isize {
    syscall(SYSCALL_SIGPROCMASK, [how as usize, set as usize, oldset as usize])
}

pub fn sys_sigreturn() -> isize {
    syscall(SYSCALL_SIGRETURN, [0, 0, 0])
}

pub fn sys_tgkill(tgid: usize, tid: usize, sig: i32) -> isize {
    syscall(SYSCALL_TGKILL, [tgid, tid, sig as usize])
}

pub fn sys_get_time() -> isize {
    syscall(SYSCALL_GET_TIME, [0, 0, 0])
}

pub fn sys_getpid() -> usize {
    let result = syscall(SYSCALL_GETPID, [0, 0, 0]);
    assert!(result >= 0, "sys_getpid failed unexpectedly");
    result as usize
}

pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

pub fn sys_exec(path: &str, args: &[*const u8]) -> isize {
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, args.as_ptr() as usize, 0])
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}

pub fn sys_pipe(pipe: &mut [usize]) -> isize {
    syscall(SYSCALL_PIPE, [pipe.as_mut_ptr() as usize, 0, 0])
}

pub fn sys_brk(new_brk: usize) -> isize {
    syscall(SYSCALL_BRK, [new_brk as usize, 0, 0])
}

pub fn sys_getcwd(buf: *mut u8, size: usize) -> isize {
    syscall(SYSCALL_GETCWD, [buf as usize, size, 0])
}

pub fn sys_dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd, 0, 0])
}

pub fn sys_dup3(old: usize, new: usize) -> isize {
    syscall(SYSCALLDUP3, [old, new, 0])
}

pub fn sys_times(tms: *mut Tms) -> isize {
    syscall(SYSCALL_TIMES, [tms as usize, 0, 0])
}

pub fn sys_uname(mes: *mut Utsname) -> isize {
    syscall(SYSCALL_UNAME, [mes as usize, 0, 0])
}

pub fn sys_random(buf: *mut u8, len: usize, flags: usize) -> isize {
    syscall(SYSCALL_GET_RANDOM, [buf as usize, len, flags])
}

pub fn sys_nanosleep(req: *const TimeSpec, rem: *mut TimeSpec) -> isize {
    syscall(SYSCALL_NANOSLEEP, [req as usize, rem as usize, 0])
}