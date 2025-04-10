// user/src/bin/test_tgkill.rs

#![no_std]
#![no_main]

use user_lib::{getpid, println, tgkill, signal, SIGTERM, SIGKILL, SIGUSR1, yield_};
use core::ffi::c_int;

// 信号处理函数
extern "C" fn handle_sigterm(_sig: c_int) {
    println!("Received SIGTERM signal!");
    // 移除 exit(0)，让信号处理程序完成
}

extern "C" fn handle_sigusr1(_sig: c_int) {
    println!("Received SIGUSR1 signal!");
    // 移除 exit(0)，让信号处理程序完成
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

    // 测试 1：向当前进程发送 SIGTERM
    let tgid = getpid();
    let tid = tgid;
    let sig = SIGTERM;
    test_tgkill(tgid, tid, sig);

    // 等待信号处理
    yield_(); // 使用 yield_ 让出 CPU

    // 测试 2：向当前进程发送 SIGUSR1
    test_tgkill(tgid, tid, SIGUSR1);

    // 等待信号处理
    yield_(); // 使用 yield_ 让出 CPU

    // 测试 3：无效信号编号
    let invalid_sig = 32; // MAX_SIG = 31
    test_tgkill(tgid, tid, invalid_sig);

    // 测试 4：无效 tgid
    let invalid_tgid = 9999; // 假设不存在的进程 ID
    test_tgkill(invalid_tgid, tid, sig);

    // 测试 5：无效 tid
    let invalid_tid = tgid + 1; // 假设 tid 不等于 tgid
    test_tgkill(tgid, invalid_tid, sig);

    // 测试 6：发送 SIGKILL（可能需要权限）
    test_tgkill(tgid, tid, SIGKILL);

    0 // 返回 0 表示成功
}