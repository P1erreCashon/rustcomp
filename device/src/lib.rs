#![no_std]
#![no_main]
extern crate alloc;
use alloc::sync::Arc;
use spin::Once;
pub mod block_dev;

pub use block_dev::BlockDevice;

pub static BLOCK_DEVICE: Once<Arc<dyn BlockDevice>> = Once::new();
