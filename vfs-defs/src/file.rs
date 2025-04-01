use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize,Ordering};
use spin::{Mutex, MutexGuard};
use super::{Dentry,Inode};
use bitflags::*;
use alloc::vec::Vec;


bitflags! {
    #[derive(PartialEq)]
    ///
    pub struct SeekFlags: i32 {
        ///
        const SEEK_SET = 0;
        ///
        const SEEK_CUR = 1;
        ///
        const SEEK_END = 2;
    }
}

bitflags! {
    ///Open file flags
    pub struct OpenFlags: u32 {
        ///Read only
        const RDONLY = 0;
        ///Write only
        const WRONLY = 1 << 0;
        ///Read & Write
        const RDWR = 1 << 1;
        ///Allow create
        const CREATE = 1 << 6;
        ///Clear file and return an empty one
        const TRUNC = 1 << 10;
        ///
        const DIRECTORY = 1 << 16;
    }
}
impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}
///
pub struct FileInner {
    /// Dentry which pointes to this file.
    pub dentry: Arc<dyn Dentry>,
    ///
//    pub inode: Arc<dyn Inode>,

    /// Offset position of this file.
    /// WARN: may cause trouble if this is not locked with other things.
    pub offset: Mutex<usize>,
    ///
    pub flags: Mutex<OpenFlags>,
}

impl FileInner{
    ///
    pub fn new(dentry: Arc<dyn Dentry>) -> Self {
        Self {
            dentry,
//            inode,
            offset: 0.into(),
            flags: Mutex::new(OpenFlags::empty()),
        }
    }
}

///
pub trait File: Send + Sync{
    ///
    fn get_inner(&self)->&FileInner;

    /// If readable
    fn readable(&self) -> bool;
    /// If writable
    fn writable(&self) -> bool;
    ///
    fn read_at(&self, _offset: usize, _buf: &mut [u8])->usize;
    ///
    fn write_at(&self, _offset: usize, _buf: &[u8])->usize;
    ///
    fn get_offset(&self)->MutexGuard<usize>{
        self.get_inner().offset.lock()
    }
    ///
    fn get_dentry(&self)->Arc<dyn Dentry>{
        self.get_inner().dentry.clone()
    }
    /// Read file to `buf`
    fn read(&self, buf: &mut [u8]) -> usize{
        let mut offset = self.get_offset();
        let read_size = self.read_at(*offset, buf);
        *offset += read_size;
        read_size
    }    
    /// Write `buf` to file
    fn write(&self, buf: &[u8]) -> usize{
        let mut offset = self.get_offset();
        let write_size = self.write_at(*offset, buf);
        assert_eq!(write_size, buf.len());
        *offset += write_size;
        write_size
    }
    /// Read all data inside a inode into vector
    fn read_all(&self) -> Vec<u8> {
        let mut offset = self.get_offset();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = self.read_at(*offset, &mut buffer);
            if len == 0 {
                break;
            }
            *offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
    ///
    fn seek(&self,pos:i64,flags:SeekFlags)->isize{
        let mut cur_pos = self.get_offset();
        match flags {
            SeekFlags::SEEK_CUR=>{
                if pos < 0{
                    if *cur_pos as i64 - pos.abs() < 0 {
                        return -1;
                    }
                    *cur_pos -= pos.abs() as usize;
                } else {
                    *cur_pos += pos as usize;
                }
            }
            SeekFlags::SEEK_SET=>{
                *cur_pos = pos as usize;
            }
            SeekFlags::SEEK_END=>{
                let size = self.get_dentry().get_inode().unwrap().get_size() as usize;
                if pos < 0 {
                    *cur_pos = size - pos.abs() as usize;
                } else {
                    *cur_pos = size + pos as usize;
                }
            }
            _ =>{
                return -1;
            }
        }
        return *cur_pos as isize;
    }
}

impl dyn File{

    
}

///Array of u8 slice that user communicate with os
pub struct UserBuffer {
    ///U8 vec
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    ///Create a `UserBuffer` by parameter
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    ///Length of `UserBuffer`
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}
/// Iterator of `UserBuffer`
pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
