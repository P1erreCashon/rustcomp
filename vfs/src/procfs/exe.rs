use alloc::{
    string::String,
    sync::Arc,
};

use system_result::{SysError, SysResult};
use vfs_defs::{
    Dentry, DentryInner, File, FileInner, Inode, InodeMeta, InodeMode, SuperBlock,Kstat,OpenFlags,DiskInodeType,RenameFlags,ino_alloc,
};
use config::DISK_BLOCK_SZ;

pub struct ExeDentry {
    inner: DentryInner,
}

impl ExeDentry {
    pub fn new(
        super_block: Arc<dyn SuperBlock>,
        parent: Option<Arc<dyn Dentry>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: DentryInner::new(String::from("exe"), super_block, parent),
        })
    }
}

impl Dentry for ExeDentry {
    fn get_inner(&self) -> &DentryInner {
        &self.inner
    }

    fn open(self: Arc<Self>,flags:OpenFlags) -> Arc<dyn File> {
        let ret = Arc::new(ExeFile {
            inner: FileInner::new(self),
        });
        *ret.get_inner().flags.lock() = flags;
        ret
    }

    fn concrete_lookup(self: Arc<Self>, _name: &str) -> SysResult<Arc<dyn Dentry>> {
        Err(SysError::ENOTDIR)
    }

    fn concrete_create(self: Arc<Self>, _name: &str, _type:DiskInodeType) -> SysResult<Arc<dyn Dentry>> {
        Err(SysError::ENOTDIR)
    }

    fn concrete_unlink(self: Arc<Self>, _old: &Arc<dyn Dentry>) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_new_child(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unimplemented!()
    }
    fn concrete_link(self: Arc<Self>, _new: &Arc<dyn Dentry>) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_rename(self: Arc<Self>, _new: Arc<dyn Dentry>, _flags: RenameFlags) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_getchild(self:Arc<Self>, _name: &str) -> Option<Arc<dyn Dentry>> {
        None
    }
    fn self_arc(self:Arc<Self>) -> Arc<dyn Dentry> {
        self.clone()
    }
    fn load_dir(self:Arc<Self>)->SysResult<()> {
        Err(SysError::ENOTDIR)
    }
}

pub struct ExeInode {
    meta: InodeMeta,
}

impl ExeInode {
    pub fn new(super_block: Arc<dyn SuperBlock>, _size: usize) -> Arc<Self> {
        let size = DISK_BLOCK_SZ;
        let ret = Arc::new(Self {
            meta: InodeMeta::new(InodeMode::FILE,ino_alloc(), super_block),
        });
        *ret.meta._type.lock() = DiskInodeType::File;
        ret.get_meta().inner.lock().size = size as u32;
        ret
    }
}

impl Inode for ExeInode {
    fn get_meta(&self) -> &InodeMeta {
        &self.meta
    }

    fn get_attr(&self) -> SysResult<Kstat> {
        let inner = self.meta.inner.lock();
        let mode = self.meta.mode.bits();
        let len = inner.size;
        Ok(Kstat {
            st_dev: 0,
            st_ino: self.meta.ino as u64,
            st_mode: mode,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: len as u64,
            st_blksize: 512,
            __pad2: 0,
            st_blocks: (len / 512) as u64,
            st_atime_sec:inner.atime.sec as u64,
            st_atime_nsec:inner.atime.usec as u64,
            st_mtime_sec:inner.mtime.sec as u64,
            st_mtime_nsec:inner.mtime.usec as u64,
            st_ctime_sec:inner.ctime.sec as u64,
            st_ctime_nsec:inner.ctime.usec as u64,
            unused: 0,
        })
    }
    fn load_from_disk(&self) {
        
    }
    fn get_size(&self) -> u32 {
        self.meta.inner.lock().size
    }
    fn clear(&self) {
        
    }
}

pub struct ExeFile {
    inner: FileInner,
}

impl File for ExeFile {
    fn get_inner(&self) -> &FileInner {
        &self.inner
    }

    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> usize {
        0
    }

    fn write_at(&self, _offset: usize, _buf: &[u8]) -> usize {
        0
    }
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn poll(&self, _events: vfs_defs::PollEvents) -> vfs_defs::PollEvents {
        vfs_defs::PollEvents::empty()
    }
}
