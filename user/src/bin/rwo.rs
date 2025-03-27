#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern  crate alloc;
use alloc::vec;
use core::{convert::TryInto, str};
use user_lib::{open,write,read,OpenFlags,close};


#[no_mangle]
pub fn main() -> i32 {// 测试 open, write, read 功能
    const FILE_NAME: &str = "./testfile.txt";
    const CONTENT: &str = "Hello, Rust userland!";
 
    // 打开或创建文件（O_CREATE | O_WRONLY | O_TRUNC）
    let flags = OpenFlags::CREATE | OpenFlags::WRONLY | OpenFlags::TRUNC;
    let fd = open(FILE_NAME, flags);
    if fd < 0 {
        println!("Failed to open file: {}", fd);
        return -1;
    }
    println!("File opened successfully, fd: {}", fd);
 
    // 写入文件
    let content_bytes = CONTENT.as_bytes();
    let bytes_written = write(fd.try_into().unwrap(), content_bytes);
    if bytes_written < 0 {
        println!("Failed to write to file: {}", bytes_written);
        close(fd.try_into().unwrap());
        return -1;
    }
    println!("Written {} bytes to file", bytes_written);
 
    // 读取文件内容
    let mut buffer = vec![0u8; 128]; // 假设缓冲区足够大
    let bytes_read = read(fd.try_into().unwrap(), &mut buffer);
    if bytes_read < 0 {
        println!("Failed to read from file: {}", bytes_read);
        close(fd.try_into().unwrap());
        return -1;
    }
    println!("Read {} bytes from file", bytes_read);
 
    // 将读取的内容转换为字符串并打印
    let content = str::from_utf8(&buffer[..bytes_read as usize]).unwrap_or("<invalid UTF-8>");
    println!("File content: {}", content);
 
    // 关闭文件
    if close(fd.try_into().unwrap()) < 0 {
        println!("Failed to close file");
        return -1;
    }
    println!("File closed successfully after reading");
 
    0 // 正常退出
}