// user/src/bin/sigaction.rs
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{fork, kill, waitpid, sys_getpid, sigaction, SignalFlags, SigAction};

#[no_mangle]
pub fn main() -> i32 {
    let pid = fork();
    if pid == 0 {
        // 子进程：设置 SIGINT 的处理函数
        let handler = sig_handler as usize; // 信号处理函数地址
        let mut act = SigAction {
            sa_handler: handler,
            sa_mask: SignalFlags::empty(),
        };
        let result = sigaction(2, &act, 0 as *mut SigAction); // SIGINT = 2
        if result != 0 {
            println!("sigaction failed: {}", result);
            return -1;
        }
        println!("Child process (pid: {}) set SIGINT handler, waiting...", sys_getpid());
        loop {}
    } else {
        // 父进程：向子进程发送 SIGINT
        println!("Parent process sending SIGINT to child (pid: {})", pid);
        let signal = 1 << 2; // SIGINT
        let result = kill(pid as usize, signal as i32);
        if result != 0 {
            println!("Failed to send SIGINT to child process");
            return -1;
        }
        let mut exit_code: i32 = 0;
        let wait_result = waitpid(pid as usize, &mut exit_code);
        if wait_result != pid {
            println!("waitpid failed");
            return -1;
        }
        if exit_code == 0 {
            println!("Child process handled SIGINT successfully");
            0
        } else {
            println!("Child process did not handle SIGINT, exit code: {}", exit_code);
            -1
        }
    }
}

#[no_mangle]
pub fn sig_handler(_sig: i32) {
    println!("Child process received SIGINT, exiting...");
    // 退出
    user_lib::exit(0);
}