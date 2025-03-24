#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::brk;
use user_lib::dup;
use user_lib::getcwd;
use user_lib::dup3;
use user_lib::uname;
use user_lib::times;
use user_lib::Tms;
use user_lib::Utsname;
use core::convert::TryInto;
use core::ffi::CStr;
use core::ptr::addr_of_mut;

static mut tms: Tms = Tms {
    tms_utime: 0,
    tms_stime: 0,
    tms_cutime: 0,
    tms_cstime: 0,
};
static mut mes: Utsname = Utsname {
    sysname: [0; 65],
    nodename: [0; 65],
    release: [0; 65],
    version: [0; 65],
    machine: [0; 65],
    domainname: [0; 65],
};
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

    //测试getcwd
    {
        const s:usize=10;
        let mut buf: [u8;s]=[0;s];
        unsafe {
            // 将数组转换为可变指针
            let result = getcwd(buf.as_mut_ptr(), s);
 
            if result !=-1 {
            // 从缓冲区指针创建一个 CStr
            let c_str = CStr::from_ptr(buf.as_ptr() as *const i8);
 
            // 尝试将 CStr 转换为 Rust 的字符串切片
            match c_str.to_str() {
                Ok(path) => println!("Current directory: {}", path),
                Err(e) => println!("Error decoding path: {}", e),
            }
            } else {
            println!("Error getting current working directory");
            }
        }
    }
    //测试dup
    {
        let fd = dup(1);
        assert!(fd >= 0);
        println!("fd = {}",fd);
        let fd = dup(1);
        assert!(fd >= 0);
        println!("fd = {}",fd);
    }
    //测试dup2
    {
        // 0 1 2 3 4 已存在
        let fd = dup3(1,1);
        assert!(fd == 1);
        println!("fd = {}",fd);
        let fd = dup3(1, 3);
        assert!(fd == 3);
        println!("fd = {}",fd);
        let fd = dup3(1, 6);
        assert!(fd == 6);
        println!("fd = {}",fd);
    }
    //测试times 153
    {
        let tms_ptr = unsafe {
            //&mut tms as *mut Tms
            addr_of_mut!(tms)
        };
        for _ in 0..5 {
            //println!("proc_times={}, sys_times={}", times(tms_ptr), get_time());
            times(tms_ptr);
            unsafe {
                tms.show();
            }
        }
    }
    //测试uname 160
    {
        println!("before check");
        unsafe {
            mes.show();
        }
        let uname_ptr = unsafe {
            addr_of_mut!(mes)
        };
        assert_eq!(uname(uname_ptr), 0);
        println!("after check");
        unsafe {
            mes.show();
        }
    }
    0
}