#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::dup;


#[no_mangle]
pub fn main() -> i32 {
    let fd = dup(1);
    assert!(fd >= 0);
    println!("fd = {}",fd);
    0
}