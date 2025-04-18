use vfs_defs::{InodeMeta,Kstat,Inode,InodeMode,ino_alloc,SuperBlock};
use alloc::vec::Vec;
use alloc::sync::Arc;
use sync::Mutex;

pub struct MemInode{
    meta:InodeMeta,
    data:Mutex<Vec<u8>>
}

impl MemInode{
    pub fn new(mode:InodeMode,superblock:Arc<dyn SuperBlock>)->Arc<Self>{
        let ret = Arc::new(Self{
            meta:InodeMeta::new(mode, ino_alloc(), superblock),
            data:Mutex::new(Vec::new())
        });
        let _type = mode.into();
        *ret.meta._type.lock() = _type;
        ret
    }
    pub fn read(self:Arc<Self>,offset:usize,buf:&mut [u8])->usize{
        let data = self.data.lock();
        let available = data.len().saturating_sub(offset);
        let copy_len = core::cmp::min(available, buf.len());
        buf[..copy_len].copy_from_slice(&data[offset..offset + copy_len]);
        copy_len
    }
    pub fn write(self:Arc<Self>,offset:usize,buf:&[u8])->usize{
        let required_len = offset + buf.len();
        let mut data = self.data.lock();
        if data.len() < required_len {
            let extra = required_len - data.len();
            data.reserve(extra); 
            for _ in 0..extra {
                data.push(0);
            }
        }
        for (i, &byte) in buf.iter().enumerate() {
            data[offset + i] = byte;
        }
        return buf.len();
    }
}

impl Inode for MemInode{
    fn get_meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn load_from_disk(&self) {
        
    }
    fn clear(&self) {
        
    }
    fn get_attr(&self)->system_result::SysResult<Kstat> {
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
    fn get_size(&self) -> u32 {
        self.data.lock().len() as u32
    }
}