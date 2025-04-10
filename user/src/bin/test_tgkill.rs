// user/src/bin/test_tgkill.rs

#![no_std]
#![no_main]

use user_lib::{getpid, println, tgkill, signal, sigprocmask, nanosleep, sigreturn, SIGTERM, SIGKILL, SIGUSR1, SIGUSR2, SIG_BLOCK, SIG_UNBLOCK};
use core::ffi::c_int;

// 信号处理函数
extern "C" fn handle_sigterm(_sig: c_int) {
    println!("Received SIGTERM signal!");
    sigreturn(); // 显式调用 sigreturn
}

extern "C" fn handle_sigusr1(_sig: c_int) {
    println!("Received SIGUSR1 signal!");
    let ret = sigreturn();
    println!("sigreturn returned: {}", ret); // 调试用，实际不会执行到这里
}

extern "C" fn handle_sigusr2(_sig: c_int) {
    println!("Received SIGUSR2 signal!");
    sigreturn(); // 显式调用 sigreturn
}

// 使用 nanosleep 等待一段时间
fn sleep_ms(ms: u64) {
    let timespec = user_lib::TimeSpec {
        tv_sec: (ms / 1000) as isize,
        tv_nsec: ((ms % 1000) * 1_000_000) as isize,
    };
    nanosleep(&timespec, core::ptr::null_mut()); // 修正 ×pec 为 &timespec
}

fn test_tgkill(tgid: usize, tid: usize, sig: c_int) -> isize {
    println!("Calling tgkill(tgid={}, tid={}, sig={})", tgid, tid, sig);
    let ret = tgkill(tgid, tid, sig);
    if ret < 0 {
        println!("tgkill failed: {}", ret);
    } else {
        println!("tgkill succeeded: {}", ret);
    }
    ret
}

#[no_mangle]
pub fn main() -> i32 {
    // 设置 SIGTERM 的信号处理程序
    unsafe {
        signal(SIGTERM, handle_sigterm);
    }

    // 设置 SIGUSR1 的信号处理程序
    unsafe {
        signal(SIGUSR1, handle_sigusr1);
    }

    // 设置 SIGUSR2 的信号处理程序
    unsafe {
        signal(SIGUSR2, handle_sigusr2);
    }

    let tgid = getpid();
    let tid = tgid;

    // 测试 1：向当前进程发送 SIGUSR1
    println!("\nTest 1: Send SIGUSR1");
    test_tgkill(tgid, tid, SIGUSR1);
    sleep_ms(100);

    // 测试 2：测试信号掩码（屏蔽 SIGUSR1，发送 SIGUSR1 和 SIGUSR2）
    println!("\nTest 2: Signal mask test - Block SIGUSR1");
    let mut mask: u32 = 0;
    unsafe {
        mask |= 1 << SIGUSR1;
        sigprocmask(SIG_BLOCK, &mask, core::ptr::null_mut());
    }
    test_tgkill(tgid, tid, SIGUSR1); // 应该被屏蔽
    test_tgkill(tgid, tid, SIGUSR2); // 不被屏蔽
    sleep_ms(100);
    unsafe {
        sigprocmask(SIG_UNBLOCK, &mask, core::ptr::null_mut());
    }
    sleep_ms(100); // 等待屏蔽的 SIGUSR1 被处理

    // 测试 3：多信号测试（发送 SIGUSR1 和 SIGUSR2）
    println!("\nTest 3: Multiple signals - Send SIGUSR1 and SIGUSR2");
    test_tgkill(tgid, tid, SIGUSR1);
    test_tgkill(tgid, tid, SIGUSR2);
    sleep_ms(100);

    // 测试 4：无效信号编号
    println!("\nTest 4: Invalid signal number");
    let invalid_sig = 32;
    test_tgkill(tgid, tid, invalid_sig);

    // 测试 5：无效 tgid
    println!("\nTest 5: Invalid tgid");
    let invalid_tgid = 9999;
    test_tgkill(invalid_tgid, tid, SIGTERM);

    // 测试 6：无效 tid
    println!("\nTest 6: Invalid tid");
    let invalid_tid = tgid + 1;
    test_tgkill(tgid, invalid_tid, SIGTERM);

    // 测试 7：发送 SIGKILL（可能需要权限）
    println!("\nTest 7: Send SIGKILL");
    test_tgkill(tgid, tid, SIGKILL);

    // 测试 8：向当前进程发送 SIGTERM
    println!("\nTest 8: Send SIGTERM");
    test_tgkill(tgid, tid, SIGTERM);
    sleep_ms(100);

    println!("\nAll tests completed!");
    0
}