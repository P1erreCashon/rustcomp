#![no_std]
#![no_main]
extern crate alloc;
pub mod block_cache;

/// Use a fs block size of 512 bytes
pub const BLOCK_SZ: usize = 4096;

// The io block size of the disk layer
pub const DISK_BLOCK_SZ: usize = 512;
/// A data block
pub type DataBlock = [u8; DISK_BLOCK_SZ];

//pub use block_cache::BLOCK_CACHE_MANAGER;
pub use block_cache::{block_cache_sync_all, get_block_cache};
