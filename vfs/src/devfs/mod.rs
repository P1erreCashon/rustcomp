use vfs_defs::{FileSystemType,FileSystemTypeInner,SuperBlock,SuperBlockInner,Dentry,MountFlags,InodeMode,DentryState};
use alloc::{string::String, sync::Arc};
use device::BlockDevice;
use super::{MemDentry,MemInode,add_vfs_dentry};
use system_result::SysResult;
mod tty;
mod cpu_dma_latency;
mod null;
mod rtc;
mod urandom;
mod zero;

use cpu_dma_latency::{CpuDmaLatencyDentry,CpuDmaLatencyInode};
use null::{NullDentry,NullInode};
use rtc::{RtcDentry,RtcInode};
use urandom::{UrandomDentry,UrandomInode};
use zero::{ZeroDentry,ZeroInode};

pub fn init_devfs(root_dentry: Arc<dyn Dentry>) -> SysResult<()> {
    let sb = root_dentry.get_superblock();

    let zero_dentry = ZeroDentry::new("zero", sb.clone(), Some(root_dentry.clone()));
    let zero_inode = ZeroInode::new(sb.clone(),0);
    zero_dentry.set_inode(zero_inode);
    *zero_dentry.get_state() = DentryState::Valid;
    root_dentry.add_child(zero_dentry.clone());
    add_vfs_dentry(zero_dentry);

    let null_dentry = NullDentry::new("null", sb.clone(), Some(root_dentry.clone()));
    let null_inode = NullInode::new(sb.clone(),0);
    null_dentry.set_inode(null_inode);
    *null_dentry.get_state() = DentryState::Valid;
    root_dentry.add_child(null_dentry.clone());
    add_vfs_dentry(null_dentry);

    let rtc_dentry = RtcDentry::new("rtc", sb.clone(), Some(root_dentry.clone()));
    let rtc_inode = RtcInode::new(sb.clone(),0);
    rtc_dentry.set_inode(rtc_inode);
    *rtc_dentry.get_state() = DentryState::Valid;
    root_dentry.add_child(rtc_dentry.clone());
    add_vfs_dentry(rtc_dentry);

    let cpu_dma_latency_dentry =
        CpuDmaLatencyDentry::new("cpu_dma_latency", sb.clone(), Some(root_dentry.clone()));
    let cpu_dma_latency_inode = CpuDmaLatencyInode::new(sb.clone(),0);
    cpu_dma_latency_dentry.set_inode(cpu_dma_latency_inode);
    *cpu_dma_latency_dentry.get_state() = DentryState::Valid;
    root_dentry.add_child(cpu_dma_latency_dentry.clone());
    add_vfs_dentry(cpu_dma_latency_dentry);

    let urandom_dentry = UrandomDentry::new("urandom", sb.clone(), Some(root_dentry.clone()));
    let urandom_inode = UrandomInode::new(sb.clone(),0);
    urandom_dentry.set_inode(urandom_inode);
    *urandom_dentry.get_state() = DentryState::Valid;
    root_dentry.add_child(urandom_dentry.clone());
    add_vfs_dentry(urandom_dentry);
/* 
    let tty_dentry = TtyDentry::new("tty", sb.clone(), Some(root_dentry.clone()));
    root_dentry.insert(tty_dentry.clone());
    let tty_inode = TtyInode::new(sb.clone());
    tty_dentry.set_inode(tty_inode);
    let tty_file = TtyFile::new(tty_dentry.clone(), tty_dentry.inode()?);
    TTY.call_once(|| tty_file);*/


    let shm_dentry = MemDentry::new("shm", sb.clone(), Some(root_dentry.clone()));
    let shm_inode = MemInode::new(InodeMode::DIR, sb.clone());
    shm_dentry.set_inode(shm_inode);
    *shm_dentry.get_state() = DentryState::Valid;
    root_dentry.add_child(shm_dentry.clone());
    add_vfs_dentry(shm_dentry);

    Ok(())
}

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
            name:&str,
            parent:Option<Arc<dyn Dentry>>,
            _flags: MountFlags,
            device:Option<Arc<dyn BlockDevice>>)->system_result::SysResult<Arc<dyn Dentry>> {
        let superblock = DevSuperBlock::new(device, self.clone());
        let root_dentry = MemDentry::new(name, superblock.clone(), parent.clone());
        let root_inode = MemInode::new(InodeMode::DIR, superblock.clone());
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
