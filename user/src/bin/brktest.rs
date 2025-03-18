#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::brk;
use core::convert::TryInto;
#[no_mangle]
pub fn main() -> i32 {
    let cur_brk = brk(0);
    println!("cur_brk = {}",cur_brk);
    let result = brk((cur_brk + 4096).try_into().unwrap());
    assert_eq!(result, 0, "brk return error");
    let new_brk = brk(0);
    println!("new_brk = {}",new_brk);
    assert_eq!(cur_brk + 4096, new_brk, "Failed to set new break point");
    0
}