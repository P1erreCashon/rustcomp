use super::{
    block_cache_sync_all, get_block_cache, Bitmap, BlockDevice, DiskInode, EfsInode,
    DiskSuperBlock,INODE_MANAGER
};
use crate::{dentry::EfsDentry, layout::EfsSuperBlock, BLOCK_SZ};
use alloc::{rc::Weak, string::{String, ToString}, sync::Arc};
use spin::Mutex;
use vfs_defs::{Dentry, DentryInner, DiskInodeType, FileSystemType, FileSystemTypeInner, Inode, MountFlags, SuperBlock, SuperBlockInner};
use system_result::{SysError, SysResult};
use vfs_defs::{File,FileInner};
///An easy file system on block
pub struct EasyFileSystem {
    ///Real device
    pub block_device: Arc<dyn BlockDevice>,
    ///Inode bitmap
    pub inode_bitmap: Bitmap,
    ///Data bitmap
    pub data_bitmap: Bitmap,
    inode_area_start_block: u32,
    data_area_start_block: u32,
}

type DataBlock = [u8; BLOCK_SZ];
/// An easy fs over a block device
impl EasyFileSystem {
    /// A data block of block size
    pub fn create(
        block_device: Arc<dyn BlockDevice>,
        total_blocks: u32,
        inode_bitmap_blocks: u32,
    ) -> Arc<Mutex<Self>> {
        // calculate block size of areas & create bitmaps
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);
        let inode_num = inode_bitmap.maximum();
        let inode_area_blocks =
            ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;
        let data_bitmap = Bitmap::new(
            (1 + inode_bitmap_blocks + inode_area_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };
        // clear all blocks
        for i in 0..total_blocks {
            get_block_cache(i as usize, Arc::clone(&block_device))
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() {
                        *byte = 0;
                    }
                });
        }
        // initialize SuperBlock
        get_block_cache(0, Arc::clone(&block_device)).lock().modify(
            0,
            |super_block: &mut DiskSuperBlock| {
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            },
        );
        // write back immediately
        // create a inode for root node "/"
        assert_eq!(efs.alloc_inode(), 0);
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
        block_cache_sync_all();
        Arc::new(Mutex::new(efs))
    }
    /// Open a block device as a filesystem
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        // read SuperBlock
        get_block_cache(0, Arc::clone(&block_device))
            .lock()
            .read(0, |super_block: &DiskSuperBlock| {
                assert!(super_block.is_valid(), "Error loading EFS!");
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;
                let efs = Self {
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,
                };
                Arc::new(Mutex::new(efs))
            })
    }
    /// Get inode by id
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {//输入inode的id返回inode的位置（磁盘块号+偏移量（单位为字节））
        let inode_size = core::mem::size_of::<DiskInode>();   //inode的id的含义为:这个inode为它是从0开始从前往后数第几个inode
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32;
        let block_id = self.inode_area_start_block + inode_id / inodes_per_block;
        (
            block_id,
            (inode_id % inodes_per_block) as usize * inode_size,
        )
    }
    /// Get inode id by pos
    pub fn get_disk_inode_id(&self, block_id: u32,block_offset:usize) ->u32{
        let inode_size = core::mem::size_of::<DiskInode>(); 
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32;
        inodes_per_block * (block_id - self.inode_area_start_block) +(block_offset / inode_size) as u32
    }
    /// Get data block by id
    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }
    /// Allocate a new inode
    pub fn alloc_inode(&mut self) -> u32 {//返回inode的id
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32//bitmap内编号直接就是inode id
    }

    /// Allocate a data block
    pub fn alloc_data(&mut self) -> u32 {//返回block的块号
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block//bitmap内编号加bitmap start盘块号为真正的盘块号
    }
    /// Deallocate a data block
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| {
                    *p = 0;
                })
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        )
    }
}

///
pub struct EfsFsType{
    inner:FileSystemTypeInner
}

impl  EfsFsType {
    ///
    pub fn new()->Self{
        Self{
            inner:FileSystemTypeInner::new(String::from("EasyFs")),
        }
    }
}

impl FileSystemType for EfsFsType{
    fn get_inner(&self)->&FileSystemTypeInner {
        &self.inner
    }
    fn mount(
        self:Arc<Self>,
        name:&str,
        parent:Option<Arc<dyn Dentry>>,
        _flags: MountFlags,
        device:Option<Arc<dyn BlockDevice>>)->SysResult<Arc<dyn Dentry>> {
        let inner = SuperBlockInner::new(device.unwrap(), self.clone());
        let superblock = Arc::new(EfsSuperBlock::new(inner));
        let root_inode = EfsSuperBlock::root_inode(superblock.clone());
        let root_dentry;
        if parent.is_none(){
            root_dentry = Arc::new(EfsDentry::new(DentryInner::new(name.to_string(), superblock.clone(), None)));
        }
        else{
            root_dentry = Arc::new(EfsDentry::new(DentryInner::new(name.to_string(), superblock.clone(), Some(Arc::downgrade(&parent.unwrap())))));
        }
        root_inode.set_type(DiskInodeType::Directory);
        root_dentry.set_inode(root_inode);
        superblock.set_root_dentry(root_dentry.clone());
        self.add_superblock("/", superblock);
        Ok(root_dentry)
        
    }
}

///
pub struct TestFile{
    readable: bool,
    writable: bool,
    inner:FileInner,
}

impl TestFile{
    ///
    pub fn new(readable:bool,writable:bool,inner:FileInner)->Self{
        Self { readable, writable, inner}
    }
}

impl File for TestFile{
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read_at(&self,offset:usize, buf: &mut [u8]) -> usize {
        let inode = self.get_dentry().get_inode().unwrap().downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR).unwrap();
        inode.read_at(offset, buf)
    }

    fn write_at(&self,offset:usize, buf: &[u8]) -> usize {
        let inode = self.get_dentry().get_inode().unwrap().downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR).unwrap();
        inode.write_at(offset, buf)
    }

    fn get_inner(&self)->&FileInner {
        &self.inner
    }
}