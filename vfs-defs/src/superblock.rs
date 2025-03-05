use alloc::sync::{Weak,Arc};
use device::BlockDevice;
use super::{FileSystemType,Dentry,Inode};
use spin::Once;
///
pub struct SuperBlockInner{
    ///
    pub dev:Arc<dyn BlockDevice>,
    ///
    pub _type:Weak<dyn FileSystemType>,
    ///
    pub root:Once<Arc<dyn Dentry>>
}
///
pub trait SuperBlock : Send + Sync {
    ///
    fn get_inner(&self) -> &SuperBlockInner {
        unimplemented!()
    }
    /// Allocate a new inode
    fn alloc_inode(&self) -> u32 {//返回inode的id
        unimplemented!()
    }

    /// Allocate a data block
    fn alloc_data(&self) -> u32 {//返回block的块号
        unimplemented!()
    }
    /// Deallocate a data block
    fn dealloc_data(&self, block_id: u32) {
        unimplemented!()
    }
    /// Get inode by id
    fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {//输入inode的id返回inode的位置（磁盘块号+偏移量（单位为字节））
        unimplemented!()
    }
    ///
    fn set_root_dentry(&self, root_dentry: Arc<dyn Dentry>) {
        self.get_inner().root.call_once(|| root_dentry);
    }
}

impl SuperBlockInner{
    ///
    pub fn new(dev:Arc<dyn BlockDevice>,_type:Arc<dyn FileSystemType>)->Self{
        Self{
            dev,
            _type:Arc::downgrade(&_type),
            root:Once::new()
        }
    }
}