use vfs_defs::{FileSystemType,FileSystemTypeInner,SuperBlock,SuperBlockInner,Dentry,MountFlags,InodeMode,DentryState};
use alloc::{string::String, sync::Arc};
use device::BlockDevice;
use super::{MemDentry,MemInode,add_vfs_dentry};

pub struct TmpFsType {
    inner: FileSystemTypeInner,
}

impl TmpFsType {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: FileSystemTypeInner::new(String::from("tmpfs")),
        })
    }
}

impl FileSystemType for TmpFsType {
    fn get_inner(&self)->&FileSystemTypeInner{
        &self.inner
    }

    fn mount(self:Arc<Self>,
            name:&str,
            parent:Option<Arc<dyn Dentry>>,
            _flags: MountFlags,
            device:Option<Arc<dyn BlockDevice>>)->system_result::SysResult<Arc<dyn Dentry>> {
        let superblock = TmpSuperBlock::new(device, self.clone());
        let root_dentry = MemDentry::new(name, superblock.clone(), parent.clone());
        let root_inode = MemInode::new(InodeMode::DIR,  superblock.clone());
        root_dentry.set_inode(root_inode);
        *root_dentry.get_state() = DentryState::Valid;
        if let Some(parent) = parent{
            parent.add_child(root_dentry.clone());
            add_vfs_dentry(root_dentry.clone());
        }
        self.add_superblock(root_dentry.path().as_str(), superblock);
        Ok(root_dentry)
    }
    fn umount(self:Arc<Self>,
            _path:&str,
            _flags:MountFlags
        )->system_result::SysResult<()> {
        unimplemented!()
    }

}

pub struct TmpSuperBlock {
    inner: SuperBlockInner,
}

impl TmpSuperBlock {
    pub fn new(
        device: Option<Arc<dyn BlockDevice>>,
        fs_type: Arc<dyn FileSystemType>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: SuperBlockInner::new(device, fs_type),
        })
    }
}

impl SuperBlock for TmpSuperBlock {
    fn get_inner(&self) -> &SuperBlockInner{
        &self.inner
    }
}
