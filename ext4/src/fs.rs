use vfs_defs::{Dentry, DentryInner, FileSystemType, FileSystemTypeInner, Inode, InodeMeta, SuperBlock, SuperBlockInner};
use alloc::sync::Arc;
use system_result::SysResult;
use device::BlockDevice;
use crate::{dentry::Ext4Dentry, superblock::Ext4Superblock, Ext4Inode};
use alloc::string::{String,ToString};
const MODULE_LEVEL:log::Level = log::Level::Trace;
pub struct Ext4ImplFsType{
    inner:FileSystemTypeInner
}


impl Ext4ImplFsType{
    pub fn new()->Self{
        Self{
            inner:FileSystemTypeInner::new(String::from("Ext4")),
        }
    }
}

impl FileSystemType for Ext4ImplFsType{
    fn get_inner(&self)->&FileSystemTypeInner {
        &self.inner
    }
    fn mount(self:Arc<Self>,
            name:&str,
            parent:Option<Arc<dyn Dentry>>,
            _flags: vfs_defs::MountFlags,
            device:Option<Arc<dyn BlockDevice>>)->SysResult<Arc<dyn Dentry>> {
        let inner = SuperBlockInner::new(device.unwrap(), self.clone());
        let superblock = Arc::new(Ext4Superblock::new(inner));
        let root_ino= 2;
        let root_inode = Arc::new(Ext4Inode::new(InodeMeta::new(root_ino, superblock.clone())));
        root_inode.set_type(vfs_defs::DiskInodeType::Directory);
        let root_dentry;
        let abs_mount_path;
        let mut path = String::new();
        if parent.is_none(){
            root_dentry = Arc::new(Ext4Dentry::new(DentryInner::new(name.to_string(), superblock.clone(),None)));
            abs_mount_path = "/";
        }
        else{
            path = parent.as_ref().unwrap().path()+name;
            abs_mount_path = path.as_str();
            root_dentry = Arc::new(Ext4Dentry::new(DentryInner::new(name.to_string(), superblock.clone(),Some(Arc::downgrade(&parent.unwrap())))));
        }
        log_debug!("abs_m_path:{}",abs_mount_path);
        root_dentry.set_inode(root_inode);
        superblock.set_root_dentry(root_dentry.clone());
        self.add_superblock(&abs_mount_path, superblock);
        Ok(root_dentry)
    }
    fn umount(self:Arc<Self>,
            path:&str,
            _flags:vfs_defs::MountFlags
        )->SysResult<()> {
        let r = self.remove_superblock(path);
        log_debug!("umount_path:{}",path);
        if let Err(e) = r{
            return Err(e);
        }
        Ok(())
    }
}