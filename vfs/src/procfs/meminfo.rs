use alloc::{
    string::{String, ToString},
    sync::Arc,
};
use core::cmp;

use system_result::{SysError, SysResult};
use vfs_defs::{
    Dentry, DentryInner, File, FileInner, Inode, InodeMeta, InodeMode, SuperBlock,Kstat,OpenFlags,DiskInodeType,RenameFlags,ino_alloc,
};

use sync::Mutex;

pub static MEM_INFO: Mutex<MemInfo> = Mutex::new(MemInfo::new());

const TOTAL_MEM: usize = 16251136;
const FREE_MEM: usize = 327680;
const BUFFER: usize = 373336;
const CACHED: usize = 10391984;
const TOTAL_SWAP: usize = 4194300;

/// Mapping to free output: https://access.redhat.com/solutions/406773.
pub struct MemInfo {
    /// General memory
    pub total_mem: usize,
    pub free_mem: usize,
    pub avail_mem: usize,
    /// Buffer and cache
    pub buffers: usize,
    pub cached: usize,
    /// Swap space
    pub total_swap: usize,
    pub free_swap: usize,
    /// Share memory
    pub shmem: usize,
    pub slab: usize,
}

impl MemInfo {
    pub const fn new() -> Self {
        Self {
            total_mem: TOTAL_MEM,
            free_mem: FREE_MEM,
            avail_mem: TOTAL_MEM - FREE_MEM,
            buffers: BUFFER,
            cached: CACHED,
            total_swap: TOTAL_SWAP,
            free_swap: TOTAL_SWAP,
            shmem: 0,
            slab: 0,
        }
    }
    pub fn serialize(&self) -> String {
        let mut res = "".to_string();
        let end = " KB\n";
        let total_mem = "MemTotal:\t".to_string() + self.total_mem.to_string().as_str() + end;
        let free_mem = "MemFree:\t".to_string() + self.free_mem.to_string().as_str() + end;
        let avail_mem = "MemAvailable:\t".to_string() + self.avail_mem.to_string().as_str() + end;
        let buffers = "Buffers:\t".to_string() + self.buffers.to_string().as_str() + end;
        let cached = "Cached:\t".to_string() + self.cached.to_string().as_str() + end;
        let cached_swap = "SwapCached:\t".to_string() + 0.to_string().as_str() + end;
        let total_swap = "SwapTotal:\t".to_string() + self.total_swap.to_string().as_str() + end;
        let free_swap = "SwapFree:\t".to_string() + self.free_swap.to_string().as_str() + end;
        let shmem = "Shmem:\t".to_string() + self.shmem.to_string().as_str() + end;
        let slab = "Slab:\t".to_string() + self.slab.to_string().as_str() + end;
        res += total_mem.as_str();
        res += free_mem.as_str();
        res += avail_mem.as_str();
        res += buffers.as_str();
        res += cached.as_str();
        res += cached_swap.as_str();
        res += total_swap.as_str();
        res += free_swap.as_str();
        res += shmem.as_str();
        res += slab.as_str();
        res
    }
}

pub struct MemInfoDentry {
    inner: DentryInner,
}

impl MemInfoDentry {
    pub fn new(
        name: &str,
        super_block: Arc<dyn SuperBlock>,
        parent: Option<Arc<dyn Dentry>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            inner: DentryInner::new(String::from(name), super_block, parent),
        })
    }
}

impl Dentry for MemInfoDentry {
    fn get_inner(&self) -> &DentryInner {
        &self.inner
    }

    fn open(self: Arc<Self>,flags:OpenFlags) -> Arc<dyn File> {
        let ret = Arc::new(MemInfoFile {
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

pub struct MemInfoInode {
    meta: InodeMeta,
}

impl MemInfoInode {
    pub fn new(super_block: Arc<dyn SuperBlock>, _size: usize) -> Arc<Self> {
        let size = MEM_INFO.lock().serialize().len();
        let ret = Arc::new(Self {
            meta: InodeMeta::new(InodeMode::FILE,ino_alloc(), super_block),
        });
        *ret.meta._type.lock() = DiskInodeType::File;
        ret.get_meta().inner.lock().size = size as u32;
        ret
    }
}

impl Inode for MemInfoInode {
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

pub struct MemInfoFile {
    inner: FileInner,
}

impl File for MemInfoFile {
    fn get_inner(&self) -> &FileInner {
        &self.inner
    }

    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let meminfo = MEM_INFO.lock();
        let info = meminfo.serialize();
        let len = cmp::min(info.len() - offset, buf.len());
        buf[..len].copy_from_slice(&info.as_bytes()[offset..offset + len]);
        len
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
        vfs_defs::PollEvents::POLLOUT
    }
}
