use vfs_defs::{File,FileInner, Inode};
use system_result::SysError;
use crate::superblock::Ext4Superblock;
use crate::inode::Ext4Inode;
pub struct Ext4ImplFile{
    inner:FileInner
}

impl Ext4ImplFile{
    pub fn new(inner:FileInner)->Self{
        Self{
            inner
        }
    }
}

impl File for Ext4ImplFile{
    fn get_inner(&self)->&FileInner {
        &self.inner
    }
    fn read_at(&self, _offset: usize, _buf: &mut [u8])->usize {
        let dentry = self.get_dentry();
        let sb = dentry.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT).unwrap();
        let inode = dentry.get_inode().unwrap().downcast_arc::<Ext4Inode>().map_err(|_| SysError::ENOENT).unwrap();
        let read_len = _buf.len();
        let r = sb.ext4fs.ext4_file_read(inode.get_meta().ino as u64,read_len as u32, _offset as i64).unwrap();
        let read_len = r.len();
        if r.len() < _buf.len(){
            for (i, &v) in r.iter().enumerate() {
                _buf[i] = v;
            }
        }
        else{
            _buf.copy_from_slice(&r);
        }
        read_len
    }
    fn write_at(&self, _offset: usize, _buf: &[u8])->usize {
        let dentry = self.get_dentry();
        let sb = dentry.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT).unwrap();
        let inode = dentry.get_inode().unwrap().downcast_arc::<Ext4Inode>().map_err(|_| SysError::ENOENT).unwrap();
        let write_len = _buf.len();
        let _ = sb.ext4fs.ext4_file_write(inode.get_meta().ino as u64, _offset as i64,_buf);
        write_len
    }
    fn readable(&self) -> bool {
        let (readable,_writable) = self.get_inner().flags.lock().read_write();
        readable
    }
    fn writable(&self) -> bool {
        let (_readable,writable) = self.get_inner().flags.lock().read_write();
        writable
    }
}