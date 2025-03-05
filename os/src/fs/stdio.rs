//!Stdin & Stdout
use crate::drivers::chardevice::{CharDevice, UART};
use crate::sbi::console_getchar;
use crate::task::suspend_current_and_run_next;
use vfs_defs::{File,UserBuffer,FileInner};
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
    fn read(&self, mut user_buf: UserBuffer) -> usize {
        assert_eq!(user_buf.len(), 1);
        // busy loop
        /* 
        let mut c: usize;
        loop {
            c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }*/
        let ch = UART.read();
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }
        1
    }
    fn write(&self, _user_buf: UserBuffer) -> usize {
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

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> usize {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
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
