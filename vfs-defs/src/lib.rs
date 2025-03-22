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