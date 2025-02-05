//!An easy file system isolated from the kernel
#![no_std]
#![deny(missing_docs)]
extern crate alloc;
mod bitmap;
mod block_cache;
mod block_dev;
mod efs;
mod layout;
mod vfs;
/// Use a block size of 512 bytes
pub const BLOCK_SZ: usize = 512;
use bitmap::Bitmap;
use block_cache::{block_cache_sync_all, get_block_cache};
pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
use layout::*;
pub use layout::DiskInode;//debug
pub use vfs::{Inode,INODE_MANAGER,inode_cache_sync_all};
pub use layout::{DIRENT_SZ,INODE_DIRECT_COUNT,INDIRECT1_BOUND,INDIRECT2_BOUND,IndirectBlock,DataBlock,INODE_INDIRECT1_COUNT,INODE_INDIRECT2_COUNT,DiskInodeType};
