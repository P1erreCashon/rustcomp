//!A virtual file system isolated from the kernel
#![no_std]
#![deny(missing_docs)]
extern crate alloc;
mod inode;
mod dentry;
mod superblock;
mod filesystemtype;
mod file;
#[macro_use]
extern crate logger;
pub use filesystemtype::{FileSystemType,FileSystemTypeInner,MountFlags};
pub use dentry::{Dentry,DentryInner,DentryState};
pub use superblock::{SuperBlock,SuperBlockInner};
pub use inode::{Inode,InodeMeta,InodeMetaInner,DiskInodeType,InodeState};
pub use file::{File,FileInner,OpenFlags,UserBuffer,UserBufferIterator};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
#[repr(C)]
///
pub struct Kstat {
    ///
    pub st_dev: u64,
    ///
    pub st_ino: u64,
    ///
    pub st_mode: u32,
    ///
    pub st_nlink: u32,
    ///
    pub st_uid: u32,
    ///
    pub st_gid: u32,
    ///
    pub st_rdev: u64,
    ///
    pub __pad: u64,
    ///
    pub st_size: u64,
    ///
    pub st_blksize: u32,
    ///
    pub __pad2: u32,
    ///
    pub st_blocks: u64,
    ///
    pub st_atime_sec: u64,
    ///
    pub st_atime_nsec: u64,
    ///
    pub st_mtime_sec: u64,
    ///
    pub st_mtime_nsec: u64,
    ///
    pub st_ctime_sec: u64,
    ///
    pub st_ctime_nsec: u64,
    ///
    pub unused: u64,
}