//!An easy file system isolated from the kernel
#![no_std]
#![deny(missing_docs)]
extern crate alloc;
mod bitmap;
//mod block_cache;
//mod block_dev;
mod efs;
mod layout;
mod vfs;
mod dentry;
mod file;
/// Use a block size of 512 bytes
//pub const BLOCK_SZ: usize = 512;
use bitmap::Bitmap;
use buffer::{block_cache_sync_all, get_block_cache,DataBlock};
use config::BLOCK_SZ;
pub use device::block_dev::BlockDevice;
pub use efs::{EasyFileSystem,EfsFsType};
use layout::*;
pub use layout::DiskInode;//debug
pub use layout::EfsSuperBlock;
pub use vfs::{EfsInode,INODE_MANAGER,inode_cache_sync_all};
pub use layout::{DIRENT_SZ,INODE_DIRECT_COUNT,INDIRECT1_BOUND,INDIRECT2_BOUND,IndirectBlock,INODE_INDIRECT1_COUNT,INODE_INDIRECT2_COUNT};
pub use file::EfsFile;