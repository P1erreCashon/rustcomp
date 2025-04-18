#![no_std]
#![no_main]
extern crate alloc;
pub mod block_cache;

use config::BLOCK_SZ;
use config::DISK_BLOCK_SZ;
/// A data block
pub type DataBlock = [u8; DISK_BLOCK_SZ];

//pub use block_cache::BLOCK_CACHE_MANAGER;
pub use block_cache::{block_cache_sync_all, get_block_cache};
