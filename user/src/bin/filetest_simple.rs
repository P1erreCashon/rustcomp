#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{chdir, close, link, mkdir, open, read, unlink, write, OpenFlags};

#[no_mangle]
pub fn main() -> i32 {
    let test_str = "Hello, world!";
    let filea = "/filea\0";
    let fd = open(filea, OpenFlags::CREATE | OpenFlags::WRONLY);
    assert!(fd > 0);
    let fd = fd as usize;
    write(fd, test_str.as_bytes());
    close(fd);

    let fd = open(filea, OpenFlags::RDONLY);
    assert!(fd > 0);
    let fd = fd as usize;
    let mut buffer = [0u8; 100];
    let read_len = read(fd, &mut buffer) as usize;
    close(fd);
    let mut chdir_path = "/\0";
    let dirname = "dira\0";
    mkdir(dirname);
    chdir(chdir_path);
    chdir_path = "/dira\0";
    chdir(chdir_path);
    mkdir(dirname);
    chdir(chdir_path);
    unlink(dirname);
    chdir(chdir_path);
    chdir_path = "/\0";
    chdir(chdir_path);
    if link("hello_world\0", "lwq\0") == -1{
        println!("link failed!");
    }
    unlink(dirname);
    chdir(chdir_path);
    assert_eq!(test_str, core::str::from_utf8(&buffer[..read_len]).unwrap(),);
    println!("file_test passed!");
    0
}
