use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode,
    EasyFileSystem, DIRENT_SZ,INODE_DIRECT_COUNT,INDIRECT1_BOUND,INDIRECT2_BOUND,IndirectBlock,DataBlock,INODE_INDIRECT1_COUNT,INODE_INDIRECT2_COUNT,
    BLOCK_SZ,EfsSuperBlock
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;
use spin::{Mutex, MutexGuard};
use vfs_defs::{Inode, InodeMeta, InodeMetaInner, SuperBlock, SuperBlockInner,DiskInodeType,InodeState};
use device::BLOCK_DEVICE;
use system_result::{SysResult,SysError};

pub struct InodeInner{
    pub valid:bool,
 //   pub size: u32,//inode的大小（单位为字节）
    pub direct: [u32; INODE_DIRECT_COUNT],
    pub indirect1: u32,
    pub indirect2: u32,
 //   pub link_count: u32,//链接数
 //   type_: DiskInodeType,
}
/// Virtual filesystem layer over easy-fs
pub struct EfsInode {
    ///
    pub block_id: usize,//(这个inode所在的磁盘块id)
    ///
    pub block_offset: usize,//单位为字节，一定是sizeof(DiskInode)的整数倍
//    fs: Arc<Mutex<EasyFileSystem>>,
//    block_device: Arc<dyn BlockDevice>,
    inner:Mutex<InodeInner>,
    meta:InodeMeta,
}

impl Inode for EfsInode{
    fn get_meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn get_attr(&self)->SysResult<vfs_defs::Kstat> {
        Ok(vfs_defs::Kstat{
                st_dev: 0,
    st_ino: 0,
    st_mode: 0,
    st_nlink: 0,
    st_uid: 0,
    st_gid: 0,
    st_rdev: 0,
    __pad: 0,
    st_size: 0,
    st_blksize: 0,
    __pad2: 0,
    st_blocks: 0,
    st_atime_sec: 0,
    st_atime_nsec: 0,
    st_mtime_sec: 0,
    st_mtime_nsec: 0,
    st_ctime_sec: 0,
    st_ctime_nsec: 0,
    unused: 0,
        })
    }
    /// Clear the data in current inode
    fn clear(&self) {
        let (mut inner,mut meta) = self.lock_inner();
        let size = meta.size;
        let data_blocks_dealloc = self.clear_size(&mut inner,&mut meta);
        assert!(data_blocks_dealloc.len() == EfsInode::total_blocks(size) as usize);
        for data_block in data_blocks_dealloc.into_iter() {
            self.get_super().dealloc_data(data_block);
        }
        block_cache_sync_all();
    }
    fn load_from_disk(&self) {
        self.lock_inner();
    }
    fn get_size(&self) -> u32 {
        0
    }
}

impl InodeInner{
    pub fn new()->Self{
        Self { 
            valid: false, 
      //      size: 0, 
            direct:[0u32; INODE_DIRECT_COUNT], 
            indirect1: 0, 
            indirect2: 0, 
      //      link_count: 0, 
      //      type_:DiskInodeType::None
        }
    }
    // Whether this inode is a directory,must hold the lock
  //  pub fn is_dir(&self) -> bool {
  //      self.type_ == DiskInodeType::Directory
  //  }
    ///Whether this inode is valid,must hold the lock
    pub fn is_valid(&self) -> bool {
        self.valid
    }
    // Whether this inode is a file,must hold the lock
 //   #[allow(unused)]
 //   pub fn is_file(&self) -> bool {
  //      self.type_ == DiskInodeType::File
  //  }
}
impl EfsInode {
    /// Create a vfs inode
    pub fn new(
  //S      block_id: u32,
//        block_offset: usize,
//        fs: Arc<Mutex<EasyFileSystem>>,
//        block_device: Arc<dyn BlockDevice>,
        ino:usize,
        superblock:Arc<dyn SuperBlock>,
    ) -> Self {
        let (block_id,block_offset) = superblock.clone().downcast_arc::<EfsSuperBlock>().map_err(|_| SysError::ENOTDIR).unwrap().get_disk_inode_pos(ino as u32);
        Self {
            block_id: block_id as usize,
            block_offset,
//            fs,
//            block_device,
            inner:Mutex::new(InodeInner::new()),
            meta:InodeMeta::new(ino, superblock)
        }
    }
    fn get_super(&self)->Arc<EfsSuperBlock>{
        self.meta.superblock.upgrade().unwrap().downcast_arc::<EfsSuperBlock>().map_err(|_| SysError::ENOTDIR).unwrap()
    }
    fn get_dev(&self)->Arc<dyn BlockDevice>{
        self.meta.superblock.upgrade().unwrap().get_inner().dev.clone()
    }
    /// get locked &mut inner
    pub fn lock_inner(&self) -> (MutexGuard<InodeInner>,MutexGuard<InodeMetaInner> ){
        let mut inner = self.inner.lock();
        let mut meta_inner = self.meta.inner.lock();
        let mut state = self.get_state();
        if *state == InodeState::Invalid {
            self.read_disk_inode(|disk_inode| {
                meta_inner.size=disk_inode.size; 
                inner.direct=disk_inode.direct; 
                inner.indirect1=disk_inode.indirect1; 
                inner.indirect2=disk_inode.indirect2; 
                meta_inner.link=disk_inode.link_count;
                self.set_type(disk_inode.type_);

            });
            *state = InodeState::Valid;
            drop(state);
        }
        (inner,meta_inner)
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.get_dev()))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.get_dev()))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under in-memory inode by name
    fn find_inode_id(&self, name: &str,inner:&mut MutexGuard<InodeInner>,meta:&mut MutexGuard<InodeMetaInner>) -> Option<u32> {
        // assert it is a directory
        assert!(self.is_dir());
        let file_count = (meta.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                self.read_at_with_lock(DIRENT_SZ * i, dirent.as_bytes_mut(),inner,meta),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name ,get a clone of inode's Arc
    pub fn find(&self, name: &str) -> Option<Arc<dyn Inode>> {
        let (mut inner,mut meta) = self.lock_inner();
        self.find_inode_id(name,&mut inner,&mut meta).map(|inode_id| {
            INODE_MANAGER.lock().get_inode(inode_id as usize,self.get_super().clone())
        })
    }
    /// Increase the size of a disk inode
/*     fn increase_size(//这个函数要改
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        let inner = self.inner.lock();
        if new_size < inner.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    } */
    /// Create inode under current inode by name
    pub fn create(&self, name: &str,type_:DiskInodeType) -> Option<Arc<dyn Inode>> {
        let (mut inner,mut meta) = self.lock_inner();
        if self.find_inode_id(name,&mut inner,&mut meta).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = self.get_super().alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = self.get_super().get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.get_dev()))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(type_);
            });
        // append file in the dirent
        let file_count = (meta.size as usize) / DIRENT_SZ;
        // write dirent
        let dirent = DirEntry::new(name, new_inode_id);
        self.write_at_with_lock(
            file_count * DIRENT_SZ,
            dirent.as_bytes(),
            &mut inner,
            &mut meta
        );

        let (block_id, block_offset) = self.get_super().get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(
            INODE_MANAGER.lock().get_inode(
                new_inode_id as usize,
                self.get_super().clone()
            ))
/*         Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        ))) */
        // release efs lock automatically by compiler
    }
    /// link inode under current inode by name(just modify the dirent,link_count doesn't changed)
    pub fn link(&self, name: &str,ino:usize) -> isize {
        let (mut inner,mut meta) = self.lock_inner();
        if self.find_inode_id(name,&mut inner,&mut meta).is_some() {
            return -1;
        }
        // append file in the dirent
        let file_count = (meta.size as usize) / DIRENT_SZ;
 //       let inode_id = get_disk_inode_id(inode_block_id, inode_block_offset);
        // write dirent
        let dirent = DirEntry::new(name, ino as u32);
        self.write_at_with_lock(
            file_count * DIRENT_SZ,
            dirent.as_bytes(),
            &mut inner,
            &mut meta
        );
        block_cache_sync_all();
        // return inode
        0
/*         Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        ))) */
        // release efs lock automatically by compiler
    }
    /// Remove name's dirent
    pub fn unlink(&self, name: &str){
        let (mut inner,mut meta ) = self.lock_inner();
        let file_count = (meta.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                self.read_at_with_lock(DIRENT_SZ * i, dirent.as_bytes_mut(),&mut inner,&mut meta),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                dirent = DirEntry::empty();
                self.write_at_with_lock(DIRENT_SZ * i, dirent.as_bytes_mut(),&mut inner,&mut meta);
                return;
            }
        }
        panic!("unlink:name doesn't exist");
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let (mut inner,mut meta) = self.lock_inner();
        let file_count = (meta.size as usize) / DIRENT_SZ;
        let mut v: Vec<String> = Vec::new();
        for i in 0..file_count {
            let mut dirent = DirEntry::empty();
            assert_eq!(
                self.read_at_with_lock(i * DIRENT_SZ, dirent.as_bytes_mut(),&mut inner,&mut meta),
                DIRENT_SZ,
            );
            v.push(String::from(dirent.name()));
        }
        v
    }
        /// Get id of block given inner id
    pub fn get_block_id(&self, inner_id: u32,inner:&mut MutexGuard<InodeInner>) -> u32 {//给定inner_id(块的inode区内偏移量 （这个块是inode内从0开始从前往后数第几个块）)，返回实际盘块id
        let inner_id = inner_id as usize;                                          //会自动分配不在size范围内的盘块（inner.direct[inner_id] == 0等情况）
        if inner_id < INODE_DIRECT_COUNT {
            if inner.direct[inner_id] == 0{
                inner.direct[inner_id] = self.get_super().alloc_data();
            }
            inner.direct[inner_id]
        } else if inner_id < INDIRECT1_BOUND {
            if inner.indirect1 == 0{
                inner.indirect1 = self.get_super().alloc_data();
            }
            get_block_cache(inner.indirect1 as usize, Arc::clone(&self.get_dev()))
                .lock()
                .modify(0, |indirect_block: &mut IndirectBlock| {
                    if indirect_block[inner_id - INODE_DIRECT_COUNT] == 0{
                        indirect_block[inner_id - INODE_DIRECT_COUNT] = self.get_super().alloc_data();
                    }
                    indirect_block[inner_id - INODE_DIRECT_COUNT]
                })
        } else {
            let last = inner_id - INDIRECT1_BOUND;
            if inner.indirect2 == 0{
                inner.indirect2 = self.get_super().alloc_data();
            }
            let indirect1 = get_block_cache(inner.indirect2 as usize, Arc::clone(&self.get_dev()))
                .lock()
                .modify(0, |indirect2: &mut IndirectBlock| {
                    if indirect2[last / INODE_INDIRECT1_COUNT] == 0{
                        indirect2[last / INODE_INDIRECT1_COUNT] = self.get_super().alloc_data();
                    }
                    indirect2[last / INODE_INDIRECT1_COUNT]
                });
            get_block_cache(indirect1 as usize, Arc::clone(&self.get_dev()))
                .lock()
                .modify(0, |indirect1: &mut IndirectBlock| {
                    if indirect1[last % INODE_INDIRECT1_COUNT] == 0{
                        indirect1[last % INODE_INDIRECT1_COUNT] = self.get_super().alloc_data();
                    }
                    indirect1[last % INODE_INDIRECT1_COUNT]
                })
        }
    }

    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let(mut inner,mut meta) = self.lock_inner();
        let mut start = offset;
        let end = (offset + buf.len()).min(meta.size as usize);
        if start >= end {
            return 0;
        }
        let mut start_block = start / BLOCK_SZ;
        let mut read_size = 0usize;
        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            end_current_block = end_current_block.min(end);
            // read and update read size
            let block_read_size = end_current_block - start;
            let dst = &mut buf[read_size..read_size + block_read_size];
            get_block_cache(
                self.get_block_id(start_block as u32,&mut inner) as usize,
                self.get_dev().clone(),
            )
            .lock()
            .read(0, |data_block: &DataBlock| {
                let src = &data_block[start % BLOCK_SZ..start % BLOCK_SZ + block_read_size];
                dst.copy_from_slice(src);
            });
            read_size += block_read_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        read_size
    }
    /// Read data from current inode
    pub fn read_at_with_lock(&self, offset: usize, buf: &mut [u8],inner:&mut MutexGuard<InodeInner>,meta:&mut MutexGuard<InodeMetaInner>) -> usize {
        let mut start = offset;
        let end = (offset + buf.len()).min(meta.size as usize);
        if start >= end {
            return 0;
        }
        let mut start_block = start / BLOCK_SZ;
        let mut read_size = 0usize;
        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            end_current_block = end_current_block.min(end);
            // read and update read size
            let block_read_size = end_current_block - start;
            let dst = &mut buf[read_size..read_size + block_read_size];
            get_block_cache(
                self.get_block_id(start_block as u32,inner) as usize,
                self.get_dev().clone(),
            )
            .lock()
            .read(0, |data_block: &DataBlock| {
                let src = &data_block[start % BLOCK_SZ..start % BLOCK_SZ + block_read_size];
                dst.copy_from_slice(src);
            });
            read_size += block_read_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        read_size
    }
    /// Write data to current inode,without the lock
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let (mut inner,mut meta) = self.lock_inner();
        if offset > meta.size as usize{
            return 0;
        }
        if offset + buf.len() > INDIRECT2_BOUND*BLOCK_SZ{
            return 0;
        }
        let mut start = offset;
        let end = (offset + buf.len());
        assert!(start <= end);
        let mut start_block = start / BLOCK_SZ;
        let mut write_size = 0usize;
        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            end_current_block = end_current_block.min(end);
            // write and update write size
            let block_write_size = end_current_block - start;
            get_block_cache(
                self.get_block_id(start_block as u32,&mut inner) as usize,
                Arc::clone(&self.get_dev()),
            )
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                let src = &buf[write_size..write_size + block_write_size];
                let dst = &mut data_block[start % BLOCK_SZ..start % BLOCK_SZ + block_write_size];
                dst.copy_from_slice(src);
            });
            write_size += block_write_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        block_cache_sync_all();
        if offset + buf.len() > meta.size as usize{
            meta.size = (offset + buf.len() )as u32;
        }
        write_size
       // inner.size as usize
    }
    /// Write data to current inode,must hold the lock
    pub fn write_at_with_lock(&self, offset: usize, buf: &[u8],inner:&mut MutexGuard<InodeInner>,meta:&mut MutexGuard<InodeMetaInner>) -> usize {
        if offset > meta.size as usize{
            return 0;
        }
        if offset + buf.len() > INDIRECT2_BOUND*BLOCK_SZ{
            return 0;
        }
        let mut start = offset;
        let end = (offset + buf.len());
        assert!(start <= end);
        let mut start_block = start / BLOCK_SZ;
        let mut write_size = 0usize;
        loop {
            // calculate end of current block
            let mut end_current_block = (start / BLOCK_SZ + 1) * BLOCK_SZ;
            end_current_block = end_current_block.min(end);
            // write and update write size
            let block_write_size = end_current_block - start;
            get_block_cache(
                self.get_block_id(start_block as u32,inner) as usize,
                Arc::clone(&self.get_dev()),
            )
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                let src = &buf[write_size..write_size + block_write_size];
                let dst = &mut data_block[start % BLOCK_SZ..start % BLOCK_SZ + block_write_size];
                dst.copy_from_slice(src);
            });
            write_size += block_write_size;
            // move to next block
            if end_current_block == end {
                break;
            }
            start_block += 1;
            start = end_current_block;
        }
        block_cache_sync_all();
        if offset + buf.len() > meta.size as usize{
            meta.size = (offset + buf.len() )as u32;
        }
        write_size
    }    
    fn data_blocks(&self,meta:&mut MutexGuard<InodeMetaInner>) -> u32 {//返回文件占有多少用于存放数据的块
        Self::_data_blocks(meta.size)
    }
    fn _data_blocks(size: u32) -> u32 {
        (size + BLOCK_SZ as u32 - 1) / BLOCK_SZ as u32
    }
    /// Return number of blocks needed include indirect1/2.
    fn total_blocks(size: u32) -> u32 {//记录文件总共多少块（数据块+索引所需块）（不包括inode占的块）
        let data_blocks = Self::_data_blocks(size) as usize;
        let mut total = data_blocks as usize;
        // indirect1
        if data_blocks > INODE_DIRECT_COUNT {
            total += 1;
        }
        // indirect2
        if data_blocks > INDIRECT1_BOUND {
            total += 1;
            // sub indirect1
            total +=
                (data_blocks - INDIRECT1_BOUND + INODE_INDIRECT1_COUNT - 1) / INODE_INDIRECT1_COUNT;
        }
        total as u32
    } 
    /// Clear size to zero and return blocks that should be deallocated.
    /// We will clear the block contents to zero later.
    pub fn clear_size(&self,inner:&mut MutexGuard<InodeInner>,meta:&mut MutexGuard<InodeMetaInner>) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        let mut data_blocks = self.data_blocks(meta) as usize;
        meta.size = 0;
        let mut current_blocks = 0usize;
        // direct
        while current_blocks < data_blocks.min(INODE_DIRECT_COUNT) {
            v.push(inner.direct[current_blocks]);
            inner.direct[current_blocks] = 0;
            current_blocks += 1;
        }
        // indirect1 block
        if data_blocks > INODE_DIRECT_COUNT {
            v.push(inner.indirect1);
            data_blocks -= INODE_DIRECT_COUNT;
            current_blocks = 0;
        } else {
            return v;
        }
        // indirect1
        get_block_cache(inner.indirect1 as usize, Arc::clone(&self.get_dev()))
            .lock()
            .modify(0, |indirect1: &mut IndirectBlock| {
                while current_blocks < data_blocks.min(INODE_INDIRECT1_COUNT) {
                    v.push(indirect1[current_blocks]);
                    //indirect1[current_blocks] = 0;
                    current_blocks += 1;
                }
            });
        inner.indirect1 = 0;
        // indirect2 block
        if data_blocks > INODE_INDIRECT1_COUNT {
            v.push(inner.indirect2);
            data_blocks -= INODE_INDIRECT1_COUNT;
        } else {
            return v;
        }
        // indirect2
        assert!(data_blocks <= INODE_INDIRECT2_COUNT);
        let a1 = data_blocks / INODE_INDIRECT1_COUNT;
        let b1 = data_blocks % INODE_INDIRECT1_COUNT;
        get_block_cache(inner.indirect2 as usize, Arc::clone(&self.get_dev()))
            .lock()
            .modify(0, |indirect2: &mut IndirectBlock| {
                // full indirect1 blocks
                for entry in indirect2.iter_mut().take(a1) {
                    v.push(*entry);
                    get_block_cache(*entry as usize, Arc::clone(&self.get_dev()))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            for entry in indirect1.iter() {
                                v.push(*entry);
                            }
                        });
                }
                // last indirect1 block
                if b1 > 0 {
                    v.push(indirect2[a1]);
                    get_block_cache(indirect2[a1] as usize, Arc::clone(&self.get_dev()))
                        .lock()
                        .modify(0, |indirect1: &mut IndirectBlock| {
                            for entry in indirect1.iter().take(b1) {
                                v.push(*entry);
                            }
                        });
                    //indirect2[a1] = 0;
                }
            });
        inner.indirect2 = 0;
        v
    }

    ///
    pub fn is_dir_empty(&self,inner:&mut MutexGuard<InodeInner>,meta:&mut MutexGuard<InodeMetaInner>)->bool{
        let file_count = (meta.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 2..file_count {
            assert_eq!(
                self.read_at_with_lock(DIRENT_SZ * i, dirent.as_bytes_mut(),inner,meta),
                DIRENT_SZ,
            );
            if dirent.inode_number() != 0 {
                return false;
            }
        }
        true
    }
}
 /// Use a inode cache of 16 inodes
const INODE_CACHE_SIZE: usize = 32;
pub struct InodeManager{//InodeManager的实例会带一把锁，内部无需加锁
    queue:Vec<Arc<EfsInode>>
}
lazy_static! {
    /// The global block cache manager
    pub static ref INODE_MANAGER: Mutex<InodeManager> =
        Mutex::new(InodeManager::new());
}
impl InodeManager{
    pub fn new()->Self{
        Self { queue: Vec::new() }
    }
    pub fn get_inode(&mut self,//get a clone of inode's Arc
        ino: usize,
        superblock:Arc<dyn SuperBlock>,)->Arc<dyn Inode>{
        if let Some(inode) = self.queue.iter().find(|inode| inode.get_meta().ino == ino){
            return inode.clone();
        }
        else{
            if self.queue.len() >= INODE_CACHE_SIZE{
                if let Some(pos) = self.queue.iter().rposition(|inode| {Arc::strong_count(inode) <= 1}) {
                    self.queue.remove(pos);
                }
                else{
                    panic!("No more pos for new inode!");
                }
            }
            let inode = Arc::new(EfsInode::new(ino, superblock));
            let ret = inode.clone();
            self.queue.push(inode);
            return ret;
        }
    }
    fn sync_all(&mut self){
        while !self.queue.is_empty(){
            self.queue.pop();
        }
    }
}
 
/// Sync all inode cache to block device
pub fn inode_cache_sync_all() {
    INODE_MANAGER.lock().sync_all();
    block_cache_sync_all();
}
impl Drop for EfsInode{
    fn drop(&mut self) {
        let (inner,meta) = self.lock_inner();
        self.modify_disk_inode(|disk_inode|{
            disk_inode.size = meta.size;
            disk_inode.direct = inner.direct;
            disk_inode.indirect1 = inner.indirect1;
            disk_inode.indirect2 = inner.indirect2;
            disk_inode.link_count = meta.link;
        })
    }
}