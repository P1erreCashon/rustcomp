#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use user_lib::{exec, fork, wait, yield_,chdir,waitpid,exit};

fn run_basic_test_glibc(name:&str){
    let pid = fork();
    if pid == 0 {
        println!("Testing {} :",name);
        exec(format!("{}{}{}","/glibc/basic/",name,"\0").as_str(), &[core::ptr::null::<u8>()]);
        exit(0);
    // exec("user_shell\0", &[core::ptr::null::<u8>()]);
    } else {
        let mut exit_code:i32 = 0;
        waitpid(pid as usize,&mut exit_code);
    }
}
fn run_basic_test_musl(name:&str){
    let pid = fork();
    if pid == 0 {
        println!("Testing {} :",name);
        exec(format!("{}{}{}","/musl/basic/",name,"\0").as_str(), &[core::ptr::null::<u8>()]);
        exit(0);
    // exec("user_shell\0", &[core::ptr::null::<u8>()]);
    } else {
        let mut exit_code:i32 = 0;
        waitpid(pid as usize,&mut exit_code);
    }
}
fn run_basic_tests(){
    chdir("/glibc/basic\0");
    println!("#### OS COMP TEST GROUP START basic-musl-glibc ####");
    run_basic_test_glibc("brk");
    run_basic_test_glibc("chdir");
    run_basic_test_glibc("clone");
    run_basic_test_glibc("close");
    run_basic_test_glibc("dup2");
    run_basic_test_glibc("dup");
    run_basic_test_glibc("execve");
    run_basic_test_glibc("exit");
    run_basic_test_glibc("fork");
    run_basic_test_glibc("fstat");
    run_basic_test_glibc("getcwd");
    run_basic_test_glibc("getdents");
    run_basic_test_glibc("getpid");
    run_basic_test_glibc("getppid");
    run_basic_test_glibc("gettimeofday");
    run_basic_test_glibc("mkdir_");
    run_basic_test_glibc("mmap");
    run_basic_test_glibc("mount");
    run_basic_test_glibc("munmap");
    run_basic_test_glibc("openat");
    run_basic_test_glibc("open");
    run_basic_test_glibc("pipe");
    run_basic_test_glibc("read");
    run_basic_test_glibc("times");
    run_basic_test_glibc("umount");
    run_basic_test_glibc("uname");
    run_basic_test_glibc("unlink");
    run_basic_test_glibc("wait");
    run_basic_test_glibc("waitpid");
    run_basic_test_glibc("write");
    run_basic_test_glibc("yield");
    println!("#### OS COMP TEST GROUP END basic-musl-glibc ####");
    chdir("/glibc/musl\0");
    println!("#### OS COMP TEST GROUP START basic-musl-musl ####");
    run_basic_test_musl("brk");
    run_basic_test_musl("chdir");
    run_basic_test_musl("clone");
    run_basic_test_musl("close");
    run_basic_test_musl("dup2");
    run_basic_test_musl("dup");
    run_basic_test_musl("execve");
    run_basic_test_musl("exit");
    run_basic_test_musl("fork");
    run_basic_test_musl("fstat");
    run_basic_test_musl("getcwd");
    run_basic_test_musl("getdents");
    run_basic_test_musl("getpid");
    run_basic_test_musl("getppid");
    run_basic_test_musl("gettimeofday");
    run_basic_test_musl("mkdir_");
    run_basic_test_musl("mmap");
    run_basic_test_musl("mount");
    run_basic_test_musl("munmap");
    run_basic_test_musl("openat");
    run_basic_test_musl("open");
    run_basic_test_musl("pipe");
    run_basic_test_musl("read");
    run_basic_test_musl("times");
    run_basic_test_musl("umount");
    run_basic_test_musl("uname");
    run_basic_test_musl("unlink");
    run_basic_test_musl("wait");
    run_basic_test_musl("waitpid");
    run_basic_test_musl("write");
    run_basic_test_musl("yield");
    println!("#### OS COMP TEST GROUP END basic-musl-musl ####");
}
#[no_mangle]
fn main() -> i32 {
    println!("initproc");
    let mut args_copy: Vec<String> =Vec::new();
    args_copy.push(String::from("busybox"));
    args_copy.push(String::from("sh"));
    args_copy.push(String::from("/testcase.sh"));
    let mut args_addr: Vec<*const u8> = args_copy.iter().map(|arg| arg.as_ptr()).collect();
    args_addr.push(core::ptr::null::<u8>());
    run_basic_tests();
    if fork() == 0 {
        exec("/glibc/busybox\0", &args_addr);
   //  exec("user_shell\0", &[core::ptr::null::<u8>()]);
    } else {
        loop {
            let mut exit_code: i32 = 0;
           let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
       //     println!(
        //        "[initproc] Released a zombie process, pid={}, exit_code={}",
        //        pid, exit_code,
        //    );
        }
    }
    0
}
