mod meminfo;
mod mounts;
mod exe;
use vfs_defs::{FileSystemType,FileSystemTypeInner,SuperBlock,SuperBlockInner,Dentry,MountFlags,InodeMode,DiskInodeType,OpenFlags,DentryState};
use alloc::{string::String, sync::Arc};
use device::BlockDevice;
use super::{MemDentry,MemInode,add_vfs_dentry,FILE_SYSTEMS};
use meminfo::{MemInfoDentry,MemInfoInode};
use mounts::{MountsInode,MountsDentry};
use exe::{ExeInode,ExeDentry};
use system_result::SysResult;

pub fn init_procfs(root_dentry: Arc<dyn Dentry>) -> SysResult<()> {
    let mem_info_dentry = MemInfoDentry::new(
        "meminfo",
        root_dentry.get_superblock(),
        Some(root_dentry.clone()),
    );
    let mem_info_inode = MemInfoInode::new(root_dentry.get_superblock(), 0);
    mem_info_dentry.set_inode(mem_info_inode);
    *mem_info_dentry.get_state() = DentryState::Valid;
    root_dentry.add_child(mem_info_dentry.clone());
    add_vfs_dentry(mem_info_dentry);

    let mounts_dentry = MountsDentry::new(
        "mounts",
        root_dentry.get_superblock(),
        Some(root_dentry.clone()),
    );
    let mounts_inode = MountsInode::new(root_dentry.get_superblock(), 0);
    mounts_dentry.set_inode(mounts_inode);
    *mounts_dentry.get_state() = DentryState::Valid;
    root_dentry.add_child(mounts_dentry.clone());
    add_vfs_dentry(mounts_dentry);

    let sys_dentry: Arc<dyn Dentry> =
        MemDentry::new("sys", root_dentry.get_superblock(), Some(root_dentry.clone()));
    let sys_inode = MemInode::new(InodeMode::DIR, root_dentry.get_superblock());
    sys_dentry.set_inode(sys_inode);
    *sys_dentry.get_state() = DentryState::Valid;

    let kernel_dentry = sys_dentry.create("kernel", DiskInodeType::Directory)?;
    let pid_max_dentry = kernel_dentry.create("pid_max", DiskInodeType::File)?;
    let pid_max_file = pid_max_dentry.open(OpenFlags::RDWR);
    pid_max_file.write("32768\0".as_bytes());
    root_dentry.add_child(sys_dentry.clone());
    add_vfs_dentry(sys_dentry);

    let self_dentry: Arc<dyn Dentry> =
        MemDentry::new("self", root_dentry.get_superblock(), Some(root_dentry.clone()));
    let self_inode = MemInode::new(InodeMode::DIR, root_dentry.get_superblock());
    self_dentry.set_inode(self_inode);
    *self_dentry.get_state() = DentryState::Valid;


    let exe_dentry: Arc<dyn Dentry> =
        ExeDentry::new(root_dentry.get_superblock(), Some(root_dentry.clone()));
    let exe_inode = ExeInode::new(root_dentry.get_superblock(), 0);
    exe_dentry.set_inode(exe_inode);
    *exe_dentry.get_state() = DentryState::Valid;
    self_dentry.add_child(exe_dentry.clone());
    add_vfs_dentry(exe_dentry);
    root_dentry.add_child(self_dentry.clone());
    add_vfs_dentry(self_dentry);

    Ok(())
}

pub struct ProcFsType {
    inner: FileSystemTypeInner,
}

impl ProcFsType {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: FileSystemTypeInner::new(String::from("procfs")),
        })
    }
}

impl FileSystemType for ProcFsType {
    fn get_inner(&self)->&FileSystemTypeInner{
        &self.inner
    }

    fn mount(self:Arc<Self>,
            name:&str,
            parent:Option<Arc<dyn Dentry>>,
            _flags: MountFlags,
            device:Option<Arc<dyn BlockDevice>>)->system_result::SysResult<Arc<dyn Dentry>> {
        let superblock = ProcSuperBlock::new(device, self.clone());
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

pub struct ProcSuperBlock {
    inner: SuperBlockInner,
}

impl ProcSuperBlock {
    pub fn new(
        device: Option<Arc<dyn BlockDevice>>,
        fs_type: Arc<dyn FileSystemType>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: SuperBlockInner::new(device, fs_type),
        })
    }
}

impl SuperBlock for ProcSuperBlock {
    fn get_inner(&self) -> &SuperBlockInner{
        &self.inner
    }
}
