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
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_TIMES: usize = 153;
const SYSCALL_UNAME: usize = 160;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETPPID: usize = 173;
const SYSCALL_BRK: usize = 214;
const SYSCALL_CLONE: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;

mod fs;
mod process;

use fs::*;
use process::*;
use crate::task::{Tms, Utsname,TimeSpec};
const MODULE_LEVEL:log::Level = log::Level::Trace;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 5]) -> isize {
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
            log_debug!("syscall_read result:{}",result);
        },
        SYSCALL_WRITE =>{
            result = sys_write(args[0], args[1] as *mut u8, args[2]);
            log_debug!("syscall_write result:{}",result);
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
            result = sys_brk(args[0]);
            log_debug!("syscall_brk result:{}",result);
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
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
    result
}
