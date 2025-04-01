use alloc::sync::{Weak,Arc};
use device::BlockDevice;
use super::{FileSystemType,Dentry,Inode};
use downcast_rs::{impl_downcast, DowncastSync};
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
pub trait SuperBlock : Send + Sync + DowncastSync {
    ///
    fn get_inner(&self) -> &SuperBlockInner {
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

impl_downcast!(sync SuperBlock);