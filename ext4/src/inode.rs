use vfs_defs::{Inode,InodeMeta};
use super::Ext4Superblock;
use system_result::SysError;
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
    fn load_from_disk(&self) {
        
    }
    fn clear(&self) {
        
    }
}