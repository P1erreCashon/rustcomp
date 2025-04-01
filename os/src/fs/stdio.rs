//!Stdin & Stdout
//use crate::drivers::chardevice::{CharDevice, UART};
use arch::console_getchar;
use spin::{Mutex, MutexGuard};
use crate::task::suspend_current_and_run_next;
use vfs_defs::{File,UserBuffer,FileInner};
use lazy_static::*;
///Standard input
pub struct Stdin;
///Standard output
pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self,  user_buf: &mut [u8]) -> usize {
        assert_eq!(user_buf.len(), 1);
        // busy loop
        let c: u8;
        loop {
            if let Some(ch) = console_getchar() {
                c = ch;
                break;
            }
            suspend_current_and_run_next();
        }
        user_buf[0] = c as u8;
        /* 
        let ch = UART.read();
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }*/
        1
    }
    fn write(&self, _user_buf: &[u8]) -> usize {
        panic!("Cannot write to stdin!");
    }
    fn get_inner(&self)->&FileInner {
        unimplemented!()
    }
    fn read_at(&self, _offset: usize, _buf: &mut [u8])->usize {
        unimplemented!()
    }
    fn write_at(&self, _offset: usize, _buf: &[u8])->usize {
        unimplemented!()
    }
}

lazy_static! {
            pub static ref STDOUTOFF: Mutex<usize> =
            Mutex::new(0);
    }

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: &mut[u8]) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: &[u8]) -> usize {
    //    for buffer in user_buf.iter() {
            print!("{}", core::str::from_utf8(user_buf).unwrap());
    //    }
        user_buf.len()
    }
    fn get_inner(&self)->&FileInner {
        unimplemented!()
    }
    fn read_at(&self, _offset: usize, _buf: &mut [u8])->usize {
        unimplemented!()
    }
    fn write_at(&self, _offset: usize, _buf: &[u8])->usize {
        print!("{}", core::str::from_utf8(_buf).unwrap());
        _buf.len()
    }
    fn get_offset(&self)->MutexGuard<usize> {
        STDOUTOFF.lock()
    }
}
