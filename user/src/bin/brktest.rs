#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::brk;
use core::convert::TryInto;

const PAGE_SIZE: isize= 4096;
#[no_mangle]
pub fn main() -> i32 {
    let init_brk = brk(0);
    let size: isize = 6000;
    for i in 1..5 {
        
        println!("第{}轮扩充({}):", i, size/PAGE_SIZE + 1);
        let cur_brk = brk(0);
        println!("cur_brk = {}({:x})", cur_brk, cur_brk/PAGE_SIZE -1);
        let result = brk((cur_brk + size).try_into().unwrap());
        assert_eq!(result, 0, "return error!");
        let new_brk = brk(0);
        println!("new_brk = {}({:x})", new_brk, new_brk/PAGE_SIZE -1);
    }

    for i in 1..5 {
        println!("第{}轮缩小({}):", i, size/PAGE_SIZE + 1);
        let cur_brk = brk(0);
        println!("cur_brk = {}({:x})",cur_brk, cur_brk/PAGE_SIZE -1);
        let result = brk((cur_brk - size).try_into().unwrap());
        assert_eq!(result, 0, "return error!");
        let new_brk = brk(0);
        println!("new_brk = {}({:x})",new_brk, new_brk/PAGE_SIZE -1);
    }
    let latest_brk = brk(0);
    assert_eq!(init_brk, latest_brk);
    
    println!("brktest passed!");
    0
}