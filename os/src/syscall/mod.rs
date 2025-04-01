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
//const SYSCALL_DUP2: usize =  ???
const SYSCALL_UMOUNT: usize = 39;
const SYSCALL_MOUNT: usize = 40;
const SYSCALL_OPENAT: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_GETDENTS64:usize = 61;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_WRITEV: usize = 66;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_EXIT_GROUP: usize =94;
const SYSCALL_SET_TID_ADDRESS: usize = 96;
const SYSCALL_SET_ROBUST_LIST:usize = 99;
const SYSCALL_GET_ROBUST_LIST:usize = 100;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_CLOCK_GETTIME: usize =113;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_SETGID: usize = 144;
const SYSCALL_SETUID: usize =146;
const SYSCALL_TIMES: usize = 153;
const SYSCALL_UNAME: usize = 160;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETPPID: usize = 173;
const SYSCALL_GETUID: usize = 174;
const SYSCALL_GETEUID: usize = 175;
const SYSCALL_GETGID: usize = 176;
const SYSCALL_GETEGID: usize = 177;
const SYSCALL_BRK: usize = 214;
const SYSCALL_MUNMAP: usize = 215;
const SYSCALL_CLONE: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_WAITPID: usize = 260;
const SYSCALL_PRLIMIT64: usize = 261;
const SYSCALL_GET_RANDOM: usize = 278;

mod fs;
mod process;

use fs::*;
use process::*;
use crate::{config::RLimit, task::{TimeSpec, Tms, Utsname}};
const MODULE_LEVEL:log::Level = log::Level::Trace;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
   // println!("syscallid:{}",syscall_id);
    let mut result:isize = 0;
    match syscall_id {
        SYSCALL_CHDIR => {
            result = sys_chdir(args[0] as *const u8);
            log_debug!("syscall_chdir result:{}",result);
        }
        SYSCALL_LINKAT => {
            result = sys_link(args[0] as isize,args[1] as *const u8,args[2] as isize,args[3] as *const u8,args[4] as u32);
            log_debug!("syscall_link result:{}",result);
        }
        SYSCALL_UNLINKAT => {
            result = sys_unlink(args[0] as isize,args[1] as *const u8,args[2] as u32);
            log_debug!("syscall_unlink result:{}",result);
        }
        SYSCALL_MKDIRAT => {
            result = sys_mkdirat(args[0] as isize,args[1] as *const u8, args[2] as u32);
            log_debug!("syscall_mkdir result:{}",result);
        }
        SYSCALL_OPENAT => {
            result = sys_openat(args[0] as isize,args[1] as *const u8, args[2] as u32,args[3] as u32);
            log_debug!("syscall_open result:{}",result);
        },
        SYSCALL_CLOSE => {
            result = sys_close(args[0]);
            log_debug!("syscall_close result:{} closed:{}",result,args[0]);
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
            log_debug!("syscall_exit exit code:{}",args[0]);
            sys_exit(args[0] as i32);
            
        },
        SYSCALL_YIELD => {
        //    log_debug!("syscall_yield");
            sys_yield();
        },
        SYSCALL_GET_TIME => {
            result = sys_get_time(args[0] as *mut TimeSpec);
            log_debug!("syscall_get_time result:{}",result);
        },
        SYSCALL_GETPID => {
            result = sys_getpid();
            log_debug!("syscall_getpid result:{}",result);
        },
        SYSCALL_CLONE => {
            result = sys_clone(args[0],args[1] as *const u8,args[2] as *mut i32,args[3] as *mut i32,args[4] as *mut i32);
            log_debug!("syscall_fork result:{}",result);
        },
        SYSCALL_EXEC => {
            result = sys_exec(args[0] as *const u8, args[1] as *const usize);
            log_debug!("syscall_exec result:{}",result);
        },
        SYSCALL_WAITPID => {
            result = sys_waitpid(args[0] as isize, args[1] as *mut i32);
        //    log_debug!("syscall_waitpid result:{}",result);
        },
        SYSCALL_PIPE => {
            result = sys_pipe(args[0] as *mut i32);
            log_debug!("syscall_pipe result:{}",result);
        },
        SYSCALL_BRK => {
            log_debug!("syscall_brk arg:{:x}",args[0]);
            result = sys_brk(args[0]);
            log_debug!("syscall_brk result:{:x}",result);
        },
        SYSCALL_MOUNT => {
            result = sys_mount(args[0] as *const u8,args[1] as *const u8,args[2] as *const u8,args[3] as u32,args[4] as *const u8,);
            log_debug!("syscall_mount result:{}",result);
        },
        SYSCALL_UMOUNT => {
            result = sys_umount(args[0] as *const u8,args[1] as u32);
            log_debug!("syscall_umount result:{}",result);
        },
        SYSCALL_FSTAT => {
            result = sys_fstat(args[0],args[1] as *mut vfs_defs::Kstat);
            log_debug!("syscall_umount result:{}",result);
        },
        SYSCALL_GETCWD => {
            result = sys_getcwd(args[0] as *mut u8, args[1] as usize);
            log_debug!("syscall_getcwd result:{}",result);
        }
        SYSCALL_DUP => {
            result = sys_dup(args[0] as usize);
            log_debug!("syscall_dup result:{}",result);
        }
        SYSCALL_DUP3 => {
            result = sys_dup3(args[0] as usize, args[1] as usize, 0);
            log_debug!("syscall_dup3 result:{}",result);
        }
        SYSCALL_TIMES => {
            result = sys_times(args[0] as *mut Tms);
            log_debug!("syscall_times result:{}",result);
        }
        SYSCALL_UNAME => {
            result = sys_uname(args[0] as *mut Utsname);
            log_debug!("syscall_uname result:{}",result);
        }
        SYSCALL_MMAP => {
            result = sys_mmap(args[0] as *mut usize, args[1], args[2] as i32, args[3] as i32, args[4],args[5] as i32);
            log_debug!("syscall_mmap result:{}",result);
        }
        SYSCALL_MUNMAP => {
            result = sys_munmap(args[0] as *mut usize, args[1]);
            log_debug!("syscall_munmap result:{}",result);
        }
        SYSCALL_GETDENTS64 => {
            result = sys_getdents(args[0] ,args[1] as *mut u8,args[2]);
            log_debug!("syscall_getdents result:{}",result);
        }
        SYSCALL_NANOSLEEP => {
            result = sys_nanosleep(args[0] as *const TimeSpec);
            log_debug!("syscall_nanosleep result:{}",result);
        }
        SYSCALL_GETPPID => {
            result = sys_getppid();
            log_debug!("syscall_getppid result:{}",result);
        },
        SYSCALL_GETUID=>{//没有用户，返回代表root的0
            result = 1;
            log_debug!("syscall_getuid result:{}",result);
        }
        SYSCALL_GETEUID=>{//没有用户，返回代表root的0
            result = 1;
            log_debug!("syscall_geteuid result:{}",result);
        }
        SYSCALL_GETGID=>{//没有用户，返回代表root的0
            result = 1;
            log_debug!("syscall_getgid result:{}",result);
        }
        SYSCALL_GETEGID=>{//没有用户，返回代表root的0
            result = 1;
            log_debug!("syscall_getteuid result:{}",result);
        }
        SYSCALL_SET_ROBUST_LIST=>{//没有影响
            result = 0;
            log_debug!("syscall_set_robust_list result:{}",result);
        }
        SYSCALL_GET_ROBUST_LIST=>{//没有影响
            result = 0;
            log_debug!("syscall_get_robust_list result:{}",result);
        }
        SYSCALL_SET_TID_ADDRESS=>{//
            result = sys_set_tid_address(args[0]);
            log_debug!("syscall_settidaddr result:{:x}",result);
        }
        SYSCALL_PRLIMIT64=>{//
            result = sys_prlimit64(args[0], args[1] as i32, args[2] as *const RLimit, args[3] as *mut RLimit);
            log_debug!("syscall_prlimit64 result:{:x}",result);
        }
        SYSCALL_SETGID => {// 无
            result = 0;
            log_debug!("syscall_setgid result:{}",result);
        }
        SYSCALL_SETUID => {// 无
            result = 0;
            log_debug!("syscall_setuid result:{}",result);
        }
        SYSCALL_EXIT_GROUP => {// 无返回值
            log_debug!("syscall_exit exit code:{}", args[0]);
            result = sys_exit_group(args[0] as i32);
        }
        SYSCALL_CLOCK_GETTIME => {
            result = sys_clock_gettime(args[0], args[1] as *mut TimeSpec);

        }
        SYSCALL_GET_RANDOM => {
            result = sys_get_random(args[0] as *mut u8, args[1] as usize, args[2] as usize);
            log_debug!("syscall_get_random result:{}",result);
        }

        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
    result
}
