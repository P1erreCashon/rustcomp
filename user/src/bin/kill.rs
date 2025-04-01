// user/src/bin/kill.rs
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{fork, kill, waitpid, getpid};

#[no_mangle]
pub fn main() -> i32 {
    // 1. 创建一个子进程
    let pid = fork();
    if pid == 0 {
        // 子进程：进入一个无限循环，等待被杀死
        println!("Child process (pid: {}) started, waiting to be killed...", getpid());
        loop {
            // 无限循环，模拟一个运行中的进程
        }
    } else {
        // 父进程：向子进程发送 SIGINT 信号
        println!("Parent process sending SIGINT to child (pid: {})", pid);
        let signal = 1 << 2; // SIGINT 信号（根据 SignalFlags 定义）
        let result = kill(pid as usize, signal as i32);
        if result != 0 {
            println!("Failed to send SIGINT to child process");
            return -1;
        }

        // 等待子进程退出
        let mut exit_code: i32 = 0;
        let wait_result = waitpid(pid as usize, &mut exit_code);
        if wait_result != pid {
            println!("waitpid failed");
            return -1;
        }

        // 验证子进程是否因 SIGINT 退出
        // 根据 SignalFlags::check_error，SIGINT 对应的退出码是 -2
        if exit_code == -2 {
            println!("Child process exited with SIGINT (exit code: {})", exit_code);
            0 // 测试成功
        } else {
            println!("Child process did not exit with SIGINT, exit code: {}", exit_code);
            -1 // 测试失败
        }
    }
}