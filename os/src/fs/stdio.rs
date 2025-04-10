//!Stdin & Stdout
//use crate::drivers::chardevice::{CharDevice, UART};
use arch::console_getchar;
use alloc::{sync::Arc, vec::Vec};
use alloc::string::String;
use spin::{Mutex, MutexGuard};
use crate::task::suspend_current_and_run_next;
use vfs_defs::{Dentry, DentryInner, DiskInodeType, File, FileInner, Inode, InodeMeta, InodeMetaInner, Kstat, OpenFlags, PollEvents, SuperBlock, UserBuffer,RenameFlags};
use lazy_static::*;
use system_result::{SysResult,SysError};
///Standard input
pub struct Stdin{
    buf:Mutex<Option<u8>>,
    inner:FileInner
}
///Standard output
pub struct Stdout{
    inner:FileInner
}

impl Stdin{
    pub fn new(inner:FileInner)->Self{
        Self{
            buf:Mutex::new(None),
            inner
        }
    }
}
impl Stdout{
    pub fn new(inner:FileInner)->Self{
        Self{
            inner
        }
    }
}

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
        let mut buf = self.buf.lock();
        if buf.is_some(){
            user_buf[0] = buf.unwrap();
            *buf = None;
            return 1;
        }
        drop(buf);
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
        &self.inner
    }
    fn read_at(&self, _offset: usize, _buf: &mut [u8])->usize {
        unimplemented!()
    }
    fn write_at(&self, _offset: usize, _buf: &[u8])->usize {
        unimplemented!()
    }
    fn get_offset(&self)->MutexGuard<usize> {
        self.get_inner().offset.lock()
    }
    fn poll(&self, events: PollEvents) -> PollEvents {
        if  ! events.contains(PollEvents::POLLIN) { 
            return PollEvents::empty();
        }
        loop {
            if let Some(ch) = console_getchar() {
                let mut buf = self.buf.lock();
                *buf = Some(ch);
                drop(buf);
                break;
            }
            suspend_current_and_run_next();
        }
        return PollEvents::POLLIN;
    }
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
     //       print!("{}", core::str::from_utf8(user_buf).unwrap());
            unsafe {print!("{}", core::str::from_utf8_unchecked(user_buf));}
    //    }
        user_buf.len()
    }
    fn get_inner(&self)->&FileInner {
        &self.inner
    }
    fn read_at(&self, _offset: usize, _buf: &mut [u8])->usize {
        unimplemented!()
    }
    fn write_at(&self, _offset: usize, _buf: &[u8])->usize {
        print!("{}", core::str::from_utf8(_buf).unwrap());
        _buf.len()
    }
    fn get_offset(&self)->MutexGuard<usize> {
        self.get_inner().offset.lock()
    }
    fn poll(&self, _events: PollEvents) -> PollEvents {
        return PollEvents::POLLOUT;
    }
}

pub struct StdioDentry {
    inner: DentryInner,
    is_stdin:bool,
}

impl StdioDentry {
    pub fn new(
        inner:DentryInner,
        is_stdin:bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner,
            is_stdin,
        })
    }
}
impl Dentry for StdioDentry{
    fn get_inner(&self) -> &DentryInner {
        &self.inner
    }
    fn open(self:Arc<Self>,flags:OpenFlags)->Arc<dyn File> {
        let file;
        if self.is_stdin{
            file = Arc::new(Stdin::new(FileInner::new(self)));
        }
        else{
            file = Arc::new(Stdin::new(FileInner::new(self)));
        }
        *file.get_inner().flags.lock() = flags;
        return file;
    }
    fn concrete_create(self: Arc<Self>, _name: &str, _type:DiskInodeType) -> SysResult<Arc<dyn Dentry>> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_lookup(self: Arc<Self>, _name: &str) -> SysResult<Arc<dyn Dentry>> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_link(self: Arc<Self>, _new: &Arc<dyn Dentry>) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_unlink(self: Arc<Self>, _old: &Arc<dyn Dentry>) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn load_dir(self:Arc<Self>)->SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn ls(self:Arc<Self>)->Vec<String> {
        Vec::new()
    }
    fn concrete_new_child(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unimplemented!()
    }
    fn concrete_rename(self: Arc<Self>, _new: Arc<dyn Dentry>, _flags: RenameFlags) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
}

pub struct StdioInode{
    meta:InodeMeta
}
impl StdioInode{
    pub fn new(meta:InodeMeta)->Self{
        Self{
            meta,
        }
    }
}
impl Inode for StdioInode{
    fn get_meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn get_attr(&self)->system_result::SysResult<Kstat> {
            Ok(Kstat{
                st_dev: 0,
                st_ino: self.meta.ino as u64,
                st_mode: 0,
                st_nlink: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                __pad: 0,
                st_size: self.get_size() as u64,
                st_blksize: 0,
                __pad2: 0,
                st_blocks:0,
                st_atime_sec: 0,
                st_atime_nsec: 0,
                st_mtime_sec: 0,
                st_mtime_nsec: 0,
                st_ctime_sec: 0,
                st_ctime_nsec: 0,
                unused: 0,
            })

    }
    fn load_from_disk(&self) {
        
    }
    fn clear(&self) {
        
    }
    fn get_size(&self) -> u32 {
        let size = self.meta.inner.lock().size as u32;
        return size;
    }
}