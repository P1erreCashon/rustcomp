use alloc::{
    string::String,
    sync::Arc,
};
use config::DISK_BLOCK_SZ;
use system_result::{SysError, SysResult};
use vfs_defs::{
    Dentry, DentryInner, File, FileInner, Inode, InodeMeta, InodeMode, SuperBlock,Kstat,OpenFlags,DiskInodeType,RenameFlags,ino_alloc,
};

/// Linear congruence generator (LCG)
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    // 使用时间初始化种子
    pub const fn new() -> Self {
        // let seed = get_time_duration();
        let seed = 42;
        Self { state: seed }
    }

    // 生成下一个随机数
    pub fn next_u32(&mut self) -> u32 {
        const A: u64 = 6364136223846793005;
        const C: u64 = 1;
        self.state = self.state.wrapping_mul(A).wrapping_add(C);
        (self.state >> 32) as u32
    }

    #[allow(dead_code)]
    pub fn next_u8(&mut self) -> u8 {
        // LCG 参数：乘数、增量和模数
        const A: u64 = 1664525;
        const C: u64 = 1013904223;

        // 更新内部状态
        self.state = self.state.wrapping_mul(A).wrapping_add(C);

        // 返回最低 8 位
        (self.state >> 24) as u8
    }

    /// Generate a random number of u32 (4 bytes) at a time, and then split it
    /// into bytes to fill in the buf
    pub fn fill_buf(&mut self, buf: &mut [u8]) {
        let mut remaining = buf.len();
        let mut offset = 0;

        while remaining > 0 {
            // 生成一个随机的 u32 值
            let rand = self.next_u32();
            let rand_bytes = rand.to_le_bytes();

            // 计算要复制的字节数
            let chunk_size = remaining.min(4);

            // 将 rand_bytes 中的字节填充到 buf 中
            buf[offset..offset + chunk_size].copy_from_slice(&rand_bytes[..chunk_size]);

            // 更新剩余字节数和偏移量
            remaining -= chunk_size;
            offset += chunk_size;
        }
    }
}

pub struct UrandomDentry {
    inner: DentryInner,
}

impl UrandomDentry {
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

impl Dentry for UrandomDentry {
    fn get_inner(&self) -> &DentryInner {
        &self.inner
    }

    fn open(self: Arc<Self>,flags:OpenFlags) -> Arc<dyn File> {
        let ret = Arc::new(UrandomFile {
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

pub struct UrandomInode {
    meta: InodeMeta,
}

impl UrandomInode {
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

impl Inode for UrandomInode {
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

pub struct UrandomFile {
    inner: FileInner,
}
pub static mut RNG: SimpleRng = SimpleRng::new();

impl File for UrandomFile {
    fn get_inner(&self) -> &FileInner {
        &self.inner
    }

    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> usize {
        unsafe {
            RNG.fill_buf(buf);
        }
        buf.len()
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> usize {
        0
    }
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        true
    }
    fn poll(&self, _events: vfs_defs::PollEvents) -> vfs_defs::PollEvents {
        vfs_defs::PollEvents::POLLOUT
    }
}
