use vfs_defs::{File,FileInner};
use system_result::SysError;
use super::EfsInode;

///
pub struct EfsFile{
    readable: bool,
    writable: bool,
    inner:FileInner,
}

impl EfsFile{
    ///
    pub fn new(readable:bool,writable:bool,inner:FileInner)->Self{
        Self { readable, writable, inner}
    }
    ///
    pub fn set_readable(&mut self,readable:bool){
        self.readable = readable;
    }
    ///
    pub fn set_writable(&mut self,writable:bool){
        self.writable = writable;
    }
}

impl File for EfsFile{
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read_at(&self,offset:usize, buf: &mut [u8]) -> usize {
        let inode = self.get_dentry().get_inode().unwrap().downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR).unwrap();
        inode.read_at(offset, buf)
    }

    fn write_at(&self,offset:usize, buf: &[u8]) -> usize {
        let inode = self.get_dentry().get_inode().unwrap().downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR).unwrap();
        inode.write_at(offset, buf)
    }

    fn get_inner(&self)->&FileInner {
        &self.inner
    }
}