use ext4_rs::Ext4Error;
use vfs_defs::{Inode,InodeMeta, Kstat};
use super::Ext4Superblock;
use system_result::SysError;
use ext4_rs::Errno;
const MODULE_LEVEL:log::Level = log::Level::Trace;
pub struct Ext4Inode{
    meta:InodeMeta,
}

impl Ext4Inode{
    pub fn new(meta:InodeMeta)->Self{
        Self{
            meta,
        }
    }
}


impl Inode for Ext4Inode{
    fn get_meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn get_attr(&self)->system_result::SysResult<Kstat> {
        let sb = self.get_meta().superblock.upgrade().unwrap().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT)?;
        let r = sb.ext4fs.fuse_getattr(self.get_meta().ino as u64);
        if let Err(e) = r{
            let err = match e.error(){
                Errno::ENOENT=>SysError::ENOENT,
                _ => SysError::EINVAL,
            };
            Err(err)
        }
        else{
            let attr = r.unwrap();
            Ok(Kstat{
                st_dev: 0,
                st_ino: attr.ino,
                st_mode: 0,
                st_nlink: attr.nlink,
                st_uid: attr.uid,
                st_gid: attr.gid,
                st_rdev: attr.rdev as u64,
                __pad: 0,
                st_size: attr.size,
                st_blksize: attr.blksize,
                __pad2: 0,
                st_blocks: attr.blocks,
                st_atime_sec: attr.atime as u64,
                st_atime_nsec: attr.atime as u64,
                st_mtime_sec: attr.mtime as u64,
                st_mtime_nsec: attr.mtime as u64,
                st_ctime_sec: attr.ctime as u64,
                st_ctime_nsec: attr.ctime as u64,
                unused: 0,
            })
        }

    }
    fn load_from_disk(&self) {
        
    }
    fn clear(&self) {
        
    }
    fn get_size(&self) -> u32 {
        let sb = self.get_meta().superblock.upgrade().unwrap().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT).unwrap();
        let inoderef = sb.ext4fs.get_inode_ref(self.meta.ino as u32);
        inoderef.inode.size
    }
}