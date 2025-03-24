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
const SYSCALL_CHDIR: usize = 9;
const SYSCALL_GETCWD: usize =17;
const SYSCALL_UNLINK: usize = 18;
const SYSCALL_LINK: usize = 19;
const SYSCALL_MKDIR: usize = 20;
const SYSCALL_DUP: usize = 23;
const SYSCALL_DUP3: usize =  24;//?
//const SYSCALL_DUP2: usize =  ???
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_TIMES: usize = 153;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_BRK: usize = 214;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;

mod fs;
mod process;

use fs::*;
use process::*;
use crate::task::Tms;
const MODULE_LEVEL:log::Level = log::Level::Trace;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
   // println!("syscallid:{}",syscall_id);
    let mut result:isize = 0;
    match syscall_id {
        SYSCALL_CHDIR => {
            result = sys_chdir(args[0] as *const u8);
            log_debug!("syscall_chdir result:{}",result);
        }
        SYSCALL_LINK => {
            result = sys_link(args[0] as *const u8,args[1] as *const u8);
            log_debug!("syscall_link result:{}",result);
        }
        SYSCALL_UNLINK => {
            result = sys_unlink(args[0] as *const u8);
            log_debug!("syscall_unlink result:{}",result);
        }
        SYSCALL_MKDIR => {
            result = sys_mkdir(args[0] as *const u8);
            log_debug!("syscall_mkdir result:{}",result);
        }
        SYSCALL_OPEN => {
            result = sys_open(args[0] as *const u8, args[1] as u32);
            log_debug!("syscall_open result:{}",result);
        },
        SYSCALL_CLOSE => {
            result = sys_close(args[0]);
            log_debug!("syscall_close result:{}",result);
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
            log_debug!("syscall_yield");
            sys_yield();
        },
        SYSCALL_GET_TIME => {
            result = sys_get_time();
            log_debug!("syscall_get_time result:{}",result);
        },
        SYSCALL_GETPID => {
            result = sys_getpid();
            log_debug!("syscall_getpid result:{}",result);
        },
        SYSCALL_FORK => {
            result = sys_fork();
            log_debug!("syscall_fork result:{}",result);
        },
        SYSCALL_EXEC => {
            result = sys_exec(args[0] as *const u8, args[1] as *const usize);
            log_debug!("syscall_exec result:{}",result);
        },
        SYSCALL_WAITPID => {
            result = sys_waitpid(args[0] as isize, args[1] as *mut i32);
            log_debug!("syscall_waitpid result:{}",result);
        },
        SYSCALL_PIPE => {
            result = sys_pipe(args[0] as *mut usize);
            log_debug!("syscall_pipe result:{}",result);
        },
        SYSCALL_BRK => {
            result = sys_brk(args[0]);
            log_debug!("syscall_pipe result:{}",result);
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
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
    result
}
