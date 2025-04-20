use vfs_defs::{FileInner,File,PollEvents};
use super::MemInode;
use system_result::SysError;

pub struct MemFile{
    inner:FileInner
}

impl MemFile{
    pub fn new(inner:FileInner)->Self{
        Self{
            inner
        }
    }
}

impl File for MemFile{
    fn get_inner(&self)->&FileInner {
        &self.inner
    }
    fn read_at(&self, offset: usize, buf: &mut [u8])->usize {
        let inode = self.get_dentry().get_inode().unwrap().downcast_arc::<MemInode>().map_err(|_| SysError::ENOENT).unwrap();
        inode.read(offset, buf)

    }
    fn write_at(&self, offset: usize, buf: &[u8])->usize {
        let inode = self.get_dentry().get_inode().unwrap().downcast_arc::<MemInode>().map_err(|_| SysError::ENOENT).unwrap();
        inode.write(offset, buf)
    }
    fn readable(&self) -> bool {
        let (readable,_writable) = self.get_inner().flags.lock().read_write();
        readable
    }
    fn writable(&self) -> bool {
        let (_readable,writable) = self.get_inner().flags.lock().read_write();
        writable
    }
    fn poll(&self, _events: PollEvents) -> PollEvents {
        return PollEvents::POLLIN | PollEvents::POLLOUT;
    }
}