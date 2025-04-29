//! File system in os
mod inode;
mod stdio;

/*
/// File trait
pub trait File: Send + Sync {
    /// If readable
    fn readable(&self) -> bool;
    /// If writable
    fn writable(&self) -> bool;
    /// Read file to `UserBuffer`
    fn read(&self, buf: UserBuffer) -> usize;
    /// Write `UserBuffer` to file
    fn write(&self, buf: UserBuffer) -> usize;
} */

pub use inode::{list_apps, open_file,path_to_dentry,path_to_father_dentry,create_file};
pub use stdio::{Stdin, Stdout,StdioDentry,StdioInode};
/// pipe mod
pub mod pipe;
pub use pipe::{make_pipe,PipeDentry,PipeInode}; // 导出 make_pipe 函数
