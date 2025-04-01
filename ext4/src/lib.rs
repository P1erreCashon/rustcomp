#![no_std]
#![no_main]
extern crate alloc;
#[macro_use]
extern crate logger;

mod dentry;
mod fs;
mod block;
mod superblock;
mod inode;
mod file;

pub use block::Ext4Disk;
pub use inode::Ext4Inode;
pub use fs::Ext4ImplFsType;
pub use dentry::Ext4Dentry;
pub use superblock::Ext4Superblock;
pub use file::Ext4ImplFile;
pub use ext4_rs::BLOCK_SIZE;