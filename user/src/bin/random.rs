#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern  crate alloc;
use alloc::vec;
use core::{convert::TryInto, str};
use user_lib::{random};


#[no_mangle]
pub fn main() -> i32 {
    let len =5;
    let mut buf = vec![0u8;5];
    let res = random(buf.as_mut_ptr(), len, 0);
    assert_eq!(res, 0);
    println!("{:?}", buf);
    0
}