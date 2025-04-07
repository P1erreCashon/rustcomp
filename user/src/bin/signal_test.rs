// user/src/bin/signal_test.rs
#![no_std]
#![no_main]

use user_lib::{sigaction, kill, SigAction, SignalFlags, SIGUSR1};
use core::option::Option::Some;
use user_lib::println;

// 使用静态变量作为全局标志
static mut SIGNAL_RECEIVED: bool = false;

#[no_mangle]
fn main() -> i32 {
    println!("Signal Test Starting...");

    // 设置 SIGUSR1 的信号处理函数
    let handler = signal_handler as usize;
    let mut old_action = SigAction {
        handler: 0,
        mask: SignalFlags::empty(),
    };
    let new_action = SigAction {
        handler,
        mask: SignalFlags::empty(),
    };

    let ret = sigaction(SIGUSR1, Some(&new_action), Some(&mut old_action));
    if ret != 0 {
        println!("Failed to set signal handler for SIGUSR1");
        return 1;
    }
    println!("Set signal handler for SIGUSR1, old handler: {:#x}", old_action.handler);

    // 给自己发送 SIGUSR1 信号
    let pid = user_lib::getpid();
    let ret = kill(pid, SIGUSR1);
    if ret != 0 {
        println!("Failed to send SIGUSR1 to self");
        return 1;
    }
    println!("Sent SIGUSR1 to self");

    // 等待信号处理
    for _ in 0..1000 { // 增加到 1000 次
        user_lib::yield_();
        unsafe {
            if SIGNAL_RECEIVED {
                break;
            }
        }
    }

    unsafe {
        if !SIGNAL_RECEIVED {
            println!("Failed to receive SIGUSR1 signal");
            return 1;
        }
    }

    println!("Signal test completed successfully");
    0 // 成功退出
}

#[no_mangle]
fn signal_handler(sig: usize) {
    println!("Received signal: {}", sig);
    unsafe {
        SIGNAL_RECEIVED = true; // 设置信号已接收标志
    }
}