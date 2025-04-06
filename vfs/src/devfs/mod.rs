use vfs_defs::{FileSystemType,FileSystemTypeInner,SuperBlock,SuperBlockInner,Dentry,MountFlags};
use alloc::{string::String, sync::Arc};
use device::BlockDevice;
mod tty;


pub struct DevFsType {
    inner: FileSystemTypeInner,
}

impl DevFsType {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: FileSystemTypeInner::new(String::from("devfs")),
        })
    }
}

impl FileSystemType for DevFsType {
    fn get_inner(&self)->&FileSystemTypeInner{
        &self.inner
    }

    fn mount(self:Arc<Self>,
            _name:&str,
            _parent:Option<Arc<dyn Dentry>>,
            _flags: MountFlags,
            _device:Option<Arc<dyn BlockDevice>>)->system_result::SysResult<Arc<dyn Dentry>> {
        unimplemented!()
    }
    fn umount(self:Arc<Self>,
            _path:&str,
            _flags:MountFlags
        )->system_result::SysResult<()> {
        unimplemented!()
    }

}

pub struct DevSuperBlock {
    inner: SuperBlockInner,
}

impl DevSuperBlock {
    pub fn new(
        device: Option<Arc<dyn BlockDevice>>,
        fs_type: Arc<dyn FileSystemType>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: SuperBlockInner::new(device, fs_type),
        })
    }
}

impl SuperBlock for DevSuperBlock {
    fn get_inner(&self) -> &SuperBlockInner{
        &self.inner
    }
}
