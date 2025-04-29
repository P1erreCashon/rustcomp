#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use user_lib::{exec, fork, wait, yield_,chdir};

#[no_mangle]
fn main() -> i32 {
    println!("initproc");
 //   chdir("/glibc\0");
 //   let mut args_copy: Vec<String> =Vec::new();
 //   args_copy.push(String::from("sh"));
  //  let mut args_addr: Vec<*const u8> = args_copy.iter().map(|arg| arg.as_ptr()).collect();
 //   args_addr.push(core::ptr::null::<u8>());
    if fork() == 0 {
     //   exec("/glibc/busybox\0", &args_addr);
     exec("user_shell\0", &[core::ptr::null::<u8>()]);
    } else {
        loop {
            let mut exit_code: i32 = 0;
           let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
            println!(
                "[initproc] Released a zombie process, pid={}, exit_code={}",
                pid, exit_code,
            );
        }
    }
    0
}
