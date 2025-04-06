use vfs_defs::{SuperBlock,SuperBlockInner,Dentry};
use alloc::sync::Arc;
use ext4_rs::Ext4;
use super::Ext4Disk;

pub struct Ext4Superblock{
    inner:SuperBlockInner,
    pub ext4fs:Ext4
}

impl Ext4Superblock{
    pub fn new(inner:SuperBlockInner)->Self{
        let dev = inner.dev.as_ref().cloned().unwrap();
        let ext4fs = Ext4::open(Arc::new(Ext4Disk::new(dev)));
        Self { inner, ext4fs }
    }

}

impl SuperBlock for Ext4Superblock {
    fn get_inner(&self) -> &SuperBlockInner {
        &self.inner
    }
}