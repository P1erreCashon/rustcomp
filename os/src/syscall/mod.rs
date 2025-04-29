//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.
const SYSCALL_CHDIR: usize = 49;
const SYSCALL_GETCWD: usize =17;
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_LINKAT: usize = 37;
const SYSCALL_MKDIRAT: usize = 34;
const SYSCALL_DUP: usize = 23;
const SYSCALL_DUP3: usize =  24;//?
const SYSCALL_FCNTL:usize = 25;
//const SYSCALL_DUP2: usize =  ???
const SYSCALL_IOCTL:usize = 29;
const SYSCALL_UMOUNT: usize = 39;
const SYSCALL_MOUNT: usize = 40;
const SYSCALL_STATFS: usize = 43;
const SYSCALL_FACCESSAT:usize = 48;
const SYSCALL_OPENAT: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_GETDENTS64:usize = 61;
const SYSCALL_LSEEK:usize = 62;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_WRITEV: usize = 66;
const SYSCALL_SENDFILE:usize = 71;
const SYSCALL_PPOLL:usize = 73;
const SYSCALL_READLINKAT:usize = 78;
const SYSCALL_FSTATAT: usize = 79;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_UTIMENSAT:usize = 88;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_EXIT_GROUP: usize =94;
const SYSCALL_SET_TID_ADDRESS: usize = 96;
const SYSCALL_SET_ROBUST_LIST:usize = 99;
const SYSCALL_GET_ROBUST_LIST:usize = 100;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_CLOCK_GETTIME: usize =113;
const SYSCALL_CLOCK_NANOSLEEP: usize =115;
const SYSCALL_SYSLOG: usize = 116;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_SETGID: usize = 144;
const SYSCALL_SETUID: usize =146;
const SYSCALL_KILL: usize = 129;
const SYSCALL_TGKILL: usize = 131;
const SYSCALL_SIGACTION: usize = 134;
const SYSCALL_SIGPROCMASK: usize = 135;
const SYSCALL_RT_SIGTIMEDWAIT:usize = 137;
const SYSCALL_SIGRETURN: usize = 139;
const SYSCALL_TIMES: usize = 153;
const SYSCALL_SETPGID:usize = 154;
const SYSCALL_GETPGID:usize = 155;
const SYSCALL_UNAME: usize = 160;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETPPID: usize = 173;
const SYSCALL_GETUID: usize = 174;
const SYSCALL_GETEUID: usize = 175;
const SYSCALL_GETGID: usize = 176;
const SYSCALL_GETEGID: usize = 177;
const SYSCALL_GETTID: usize = 178;
const SYSCALL_SYSINFO: usize = 179;
const SYSCALL_BRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_CLONE: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MPROTECT: usize = 226;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_PRLIMIT64: usize = 261;
const SYSCALL_RENAMEAT2: usize = 276;
const SYSCALL_GET_RANDOM: usize = 278;
const SYSCALL_STATX: usize = 291;

mod fs;
mod process;

use alloc::string::String;
use arch::addr::VirtAddr;
use fs::*;
use process::*;
use crate::task::{check_signals_error_of_current, current_task, exit_current_and_run_next, pid2task, suspend_current_and_run_next, SignalFlags};
use crate::task::{TimeSpec, Tms, Utsname, SysInfo};
use config::RLimit;
use system_result::{SysResult,SysError};
const MODULE_LEVEL:log::Level = log::Level::Debug;
use crate::task::check_pending_signals;
pub use process::CloneFlags;
/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
   // println!("syscallid:{}",syscall_id);
    let result:SysResult<isize>;
    match syscall_id {
        SYSCALL_IOCTL => {
            result = Ok(0);
        }
        SYSCALL_CHDIR => {
            result = sys_chdir(args[0] as *const u8);
        }
        SYSCALL_LINKAT => {
            result = sys_link(args[0] as isize,args[1] as *const u8,args[2] as isize,args[3] as *const u8,args[4] as u32);
        }
        SYSCALL_UNLINKAT => {
            result = sys_unlink(args[0] as isize,args[1] as *const u8,args[2] as u32);
        }
        SYSCALL_MKDIRAT => {
            result = sys_mkdirat(args[0] as isize,args[1] as *const u8, args[2] as u32);
        }
        SYSCALL_OPENAT => {
            result = sys_openat(args[0] as isize,args[1] as *const u8, args[2] as u32,args[3] as u32);
        },
        SYSCALL_CLOSE => {
            result = sys_close(args[0]);
            log_debug!("syscall_close  closed:{}",args[0]);
        },
        SYSCALL_READ => {
            result = sys_read(args[0], args[1] as *mut u8, args[2]);
      //      log_debug!("syscall_read result:{}",result);
        },
        SYSCALL_WRITE =>{
            result = sys_write(args[0], args[1] as *mut u8, args[2]);
      //      log_debug!("syscall_write result:{}",result);
        },
        SYSCALL_WRITEV =>{
            result = sys_writev(args[0] as isize, args[1] as *const IoVec, args[2]);
        },
        SYSCALL_EXIT => {
            let pid = current_task().unwrap().pid.0;
            log_debug!("syscall_exit exit code:{} pid:{}",args[0],pid);
            sys_exit(args[0] as i32);
            
        },
        SYSCALL_YIELD => {
        //    log_debug!("syscall_yield");
            result = sys_yield();
        },
        SYSCALL_KILL => {
            log_debug!("syscall_kill pid={} signal={}", args[0], args[1]);
            result = sys_kill(args[0], args[1] as u32);
        },
        SYSCALL_SIGACTION => {
            result = sys_sigaction(args[0] as i32, args[1] as *const _, args[2] as *mut _);
        }
        SYSCALL_SIGPROCMASK => {
            result = sys_sigprocmask(args[0] as i32, args[1] as *const _, args[2] as *mut _);
        }
        SYSCALL_SIGRETURN => {
            result = sys_sigreturn();
        }
        SYSCALL_GET_TIME => {
            result = sys_get_time(args[0] as *mut TimeSpec);
        },
        SYSCALL_GETPID => {
            result = sys_getpid();
        },
        SYSCALL_CLONE => {
            result = sys_clone(args[0],args[1] as *const u8,args[2] as *mut i32,args[3] as *mut i32,args[4] as *mut i32);
        },
        SYSCALL_EXEC => {
            result = sys_exec(args[0] as *const u8, args[1] as *const usize);
        },
        SYSCALL_WAITPID => {
            result = sys_waitpid(args[0] as isize, args[1] as *mut i32);
        //    log_debug!("syscall_waitpid result:{}",result);
        },
        SYSCALL_PIPE => {
            result = sys_pipe(args[0] as *mut i32);
        },
        SYSCALL_BRK => {
         //   log_debug!("syscall_brk arg:{:x}",args[0]);
            result = sys_brk(args[0]);
        },
        SYSCALL_MOUNT => {
            result = sys_mount(args[0] as *const u8,args[1] as *const u8,args[2] as *const u8,args[3] as u32,args[4] as *const u8,);
        },
        SYSCALL_UMOUNT => {
            result = sys_umount(args[0] as *const u8,args[1] as u32);
        },
        SYSCALL_STATFS => {
            result = sys_statfs(args[0] as *const u8,args[1] as *mut vfs_defs::StatFs);
        },
        SYSCALL_FACCESSAT => {
            result = sys_faccessat(args[0] as isize,args[1] as *const u8,args[2],args[3] as i32);
        },
        SYSCALL_LSEEK => {
            result = sys_lseek(args[0] as isize,args[1] as isize,args[2]);
        },
        SYSCALL_FSTATAT => {
            result = sys_fstatat(args[0],args[1] as *const u8,args[2] as *mut vfs_defs::Kstat,args[3] as i32);
        },
        SYSCALL_FSTAT => {
            result = sys_fstat(args[0],args[1] as *mut vfs_defs::Kstat);
        },
        SYSCALL_UTIMENSAT => {
            result = sys_utimensat(args[0] as isize,args[1] as *const u8,args[2] as *const TimeSpec,args[3] as i32);
        },
        SYSCALL_GETCWD => {
            result = sys_getcwd(args[0] as *mut u8, args[1] as usize);
        }
        SYSCALL_DUP => {
            result = sys_dup(args[0] as usize);
        }
        SYSCALL_DUP3 => {
            result = sys_dup3(args[0] as usize, args[1] as usize, 0);
        }
        SYSCALL_FCNTL => {
            result = sys_fcntl(args[0] as isize, args[1] as isize, args[2]);
        }
        SYSCALL_TIMES => {
            result = sys_times(args[0] as *mut Tms);
        }
        SYSCALL_GETPGID => {
            result = Ok(0);
        }
        SYSCALL_SETPGID => {
            result = Ok(0);
        }
        SYSCALL_UNAME => {
            result = sys_uname(args[0] as *mut Utsname);
        }
        SYSCALL_MMAP => {
            result = sys_mmap(args[0] as *mut usize, args[1], args[2] as i32, args[3] as i32, args[4],args[5] as i32);
        }
        SYSCALL_MUNMAP => {
            result = sys_munmap(args[0] as *mut usize, args[1]);
        }
        SYSCALL_GETDENTS64 => {
            result = sys_getdents(args[0] ,args[1] as *mut u8,args[2]);
        }
        SYSCALL_NANOSLEEP => {
            result = sys_nanosleep(args[0] as *const TimeSpec);
        }
        SYSCALL_GETPPID => {
            result = sys_getppid();
        },
        SYSCALL_GETUID=>{//没有用户，返回代表root的0
            result = Ok(1);
        }
        SYSCALL_GETEUID=>{//没有用户，返回代表root的0
            result = Ok(1);
        }
        SYSCALL_GETGID=>{//没有用户，返回代表root的0
            result = Ok(1);
        }
        SYSCALL_GETEGID=>{//没有用户，返回代表root的0
            result = Ok(1);
        }
        SYSCALL_GETTID=>{//没有用户，返回代表root的0
            result = sys_gettid();
        }
        SYSCALL_SET_ROBUST_LIST=>{//没有影响
            result = Ok(0);
        }
        SYSCALL_GET_ROBUST_LIST=>{//没有影响
            result = Ok(0);
        }
        SYSCALL_SET_TID_ADDRESS=>{//
            result = sys_set_tid_address(args[0]);
        }
        SYSCALL_PRLIMIT64=>{//
            result = sys_prlimit64(args[0], args[1] as i32, args[2] as *const RLimit, args[3] as *mut RLimit);
        }
        SYSCALL_SENDFILE=>{//
            result = sys_sendfile(args[0] as isize, args[1] as isize, args[2] as *mut usize, args[3]);
        }
        SYSCALL_PPOLL=>{//
            result = sys_poll(args[0] as *mut PollFd, args[1], args[2] as *const TimeSpec);
        }
        SYSCALL_READLINKAT=>{//
            result = Ok(-1);
        }
        SYSCALL_SETGID => {// 无
            result = Ok(0);
        }
        SYSCALL_SETUID => {// 无
            result = Ok(0);
        }
        SYSCALL_EXIT_GROUP => {// 无返回值
            let pid = current_task().unwrap().pid.0;
            log_debug!("syscall_exit exit code:{} pid:{}", args[0],pid);
            sys_exit_group(args[0] as i32);
        }
        SYSCALL_CLOCK_GETTIME => {
            result = sys_clock_gettime(args[0], args[1] as *mut TimeSpec);
        }
        SYSCALL_CLOCK_NANOSLEEP => {
            result = sys_clock_nanosleep(args[0],args[1], args[2] as *const TimeSpec,args[3] as *mut TimeSpec);
        }
        SYSCALL_RENAMEAT2 => {
            result = sys_renameat2(args[0] as isize, args[1] as *const u8, args[2] as isize,args[3] as *const u8,args[4]);
        }
        SYSCALL_GET_RANDOM => {
            result = sys_get_random(args[0] as *mut u8, args[1] as usize, args[2] as usize);
        }
        SYSCALL_SYSINFO => {
            result = sys_info(args[0] as *mut SysInfo);
        }
        SYSCALL_SYSLOG => {
            result = sys_log(args[0] as usize, args[1] as *mut u8, args[2] as usize);
        }
        SYSCALL_TGKILL => {
            result = sys_tgkill(args[0] as isize, args[1] as isize, args[2] as i32);
        },
        SYSCALL_MPROTECT => {
            result = sys_mprotect(VirtAddr::new(args[0]), args[1], args[2] as i32);
        },
        SYSCALL_RT_SIGTIMEDWAIT=>{//TODO
            result = Ok(0);
        }
        SYSCALL_STATX=>{
            result = sys_statx(args[0] as isize,args[1] as *const u8,args[2] as i32,args[3] as u32,args[4] as *mut Statx);
        }
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
    // 在系统调用返回前检查信号
    check_pending_signals();
    
    if let Some((code, msg)) = check_signals_error_of_current() {
        println!("Process terminated due to signal: {}", msg);
        exit_current_and_run_next(code as i32);
        unreachable!("Should have exited");
    }
    if let Err(e) = result{
        log_debug!("{} err:{}",sysid_to_string(syscall_id),e.as_str());
        return -(e as isize);
    }   
    else{
        let pid = current_task().unwrap().pid.0;
        if syscall_id != 63 && syscall_id != 64{
            log_debug!("pid:{} {} result:{}",pid,sysid_to_string(syscall_id),result.clone().unwrap());
        }
        return result.unwrap();
    }
}

fn sysid_to_string(syscall_id: usize)->String{
    let mut ret = String::new();
    match syscall_id {
        SYSCALL_IOCTL => {
            ret.push_str("sys_ioctl");
        }
        SYSCALL_CHDIR => {
            ret.push_str("sys_chdir");
        }
        SYSCALL_LINKAT => {
            ret.push_str("sys_linkat");
        }
        SYSCALL_UNLINKAT => {
            ret.push_str("sys_unlinkat");
        }
        SYSCALL_MKDIRAT => {
            ret.push_str("sys_mkdirat");
        }
        SYSCALL_OPENAT => {
            ret.push_str("sys_openat");
        },
        SYSCALL_CLOSE => {
            ret.push_str("sys_close");
        },
        SYSCALL_READ => {
            ret.push_str("sys_read");
      //      log_debug!("syscall_read result:{}",result);
        },
        SYSCALL_WRITE =>{
            ret.push_str("sys_write");
      //      log_debug!("syscall_write result:{}",result);
        },
        SYSCALL_WRITEV =>{
            ret.push_str("sys_writev");
        },
        SYSCALL_EXIT => {
            ret.push_str("sys_exit");
            
        },
        SYSCALL_YIELD => {
        //    log_debug!("syscall_yield");
        ret.push_str("sys_yield");
        },
        SYSCALL_KILL => {
            ret.push_str("sys_kill");
        },
        SYSCALL_SIGACTION => {
            ret.push_str("sys_sigaction");
        },
        SYSCALL_SIGPROCMASK => {
            ret.push_str("sys_sigprocmask");
        },
        SYSCALL_SIGRETURN => {
            ret.push_str("sys_sigreturn");
        }
        SYSCALL_GET_TIME => {
            ret.push_str("sys_gettime");
        },
        SYSCALL_GETPID => {
            ret.push_str("sys_getpid");
        },
        SYSCALL_CLONE => {
            ret.push_str("sys_clone");
        },
        SYSCALL_EXEC => {
            ret.push_str("sys_exec");
        },
        SYSCALL_WAITPID => {
            ret.push_str("sys_waitpid");
        //    log_debug!("syscall_waitpid result:{}",result);
        },
        SYSCALL_PIPE => {
            ret.push_str("sys_pipe");
        },
        SYSCALL_BRK => {
            ret.push_str("sys_brk");
        },
        SYSCALL_MOUNT => {
            ret.push_str("sys_mount");
        },
        SYSCALL_UMOUNT => {
            ret.push_str("sys_umount");
        },
        SYSCALL_STATFS => {
            ret.push_str("sys_statfs");
        },
        SYSCALL_FACCESSAT => {
            ret.push_str("sys_faccrssat");
        },
        SYSCALL_LSEEK => {
            ret.push_str("sys_lseek");
        },
        SYSCALL_FSTATAT => {
            ret.push_str("sys_fstatat");
        },
        SYSCALL_FSTAT => {
            ret.push_str("sys_fstat");
        },
        SYSCALL_UTIMENSAT => {
            ret.push_str("sys_utimensat");
        },
        SYSCALL_GETCWD => {
            ret.push_str("sys_getcwd");
        }
        SYSCALL_DUP => {
            ret.push_str("sys_dup");
        }
        SYSCALL_DUP3 => {
            ret.push_str("sys_dup3");
        }
        SYSCALL_FCNTL => {
            ret.push_str("sys_fcntl");
        }
        SYSCALL_TIMES => {
            ret.push_str("times");
        }
        SYSCALL_GETPGID => {
            ret.push_str("sys_getpgid");
        }
        SYSCALL_SETPGID => {
            ret.push_str("sys_setpgid");
        }
        SYSCALL_UNAME => {
            ret.push_str("sys_uname");
        }
        SYSCALL_MMAP => {
            ret.push_str("sys_mmap");
        }
        SYSCALL_MUNMAP => {
            ret.push_str("sys_munmap");
        }
        SYSCALL_GETDENTS64 => {
            ret.push_str("sys_getdents64");
        }
        SYSCALL_NANOSLEEP => {
            ret.push_str("sys_nanosleep");
        }
        SYSCALL_GETPPID => {
            ret.push_str("sys_getppid");
        },
        SYSCALL_GETUID=>{//没有用户，返回代表root的0
            ret.push_str("sys_gettuid");
        }
        SYSCALL_GETEUID=>{//没有用户，返回代表root的0
            ret.push_str("sys_geteuid");
        }
        SYSCALL_GETGID=>{//没有用户，返回代表root的0
            ret.push_str("sys_getgid");
        }
        SYSCALL_GETEGID=>{//没有用户，返回代表root的0
            ret.push_str("sys_getegid");
        }
        SYSCALL_SET_ROBUST_LIST=>{//没有影响
            ret.push_str("sys_set_rubust_list");
        }
        SYSCALL_GET_ROBUST_LIST=>{//没有影响
            ret.push_str("sys_get_robust_list");
        }
        SYSCALL_SET_TID_ADDRESS=>{//
            ret.push_str("sys_settidaddress");
        }
        SYSCALL_PRLIMIT64=>{//
            ret.push_str("sys_prlimit64");
        }
        SYSCALL_SENDFILE=>{//
            ret.push_str("sys_sendfile");
        }
        SYSCALL_READLINKAT=>{//
            ret.push_str("sys_readlinkat");
        }
        SYSCALL_SETGID => {// 无
            ret.push_str("sys_setgid");
        }
        SYSCALL_SETUID => {// 无
            ret.push_str("sys_setuid");
        }
        SYSCALL_EXIT_GROUP => {// 无返回值
            ret.push_str("sys_exitgroup");
        }
        SYSCALL_CLOCK_GETTIME => {
            ret.push_str("sys_gettime");
        }
        SYSCALL_GET_RANDOM => {
            ret.push_str("sys_getrandom");
        }
        SYSCALL_SYSINFO => {
            ret.push_str("sys_sysinfo");
        }
        SYSCALL_SYSLOG => {
            ret.push_str("sys_log");
        }
        SYSCALL_MPROTECT => {
            ret.push_str("sys_mprotect");
        }
        SYSCALL_TGKILL => {
            ret.push_str("sys_tgkill");
        },
        SYSCALL_PPOLL => {
            ret.push_str("sys_ppoll");
        }
        SYSCALL_GETTID => {
            ret.push_str("sys_gettid");
        }
        SYSCALL_RENAMEAT2 => {
            ret.push_str("sys_renameat2");
        }
        SYSCALL_CLOCK_NANOSLEEP => {
            ret.push_str("sys_clock_nanosleep");
        }
        SYSCALL_RT_SIGTIMEDWAIT=>{
            ret.push_str("sys_rt_sigtimedwait");
        }
        SYSCALL_STATX=>{
            ret.push_str("sys_statx");
        }
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
    ret
}
