//! File and filesystem-related syscalls
use crate::fs::open_file;
use crate::fs::make_pipe;
use crate::fs::path_to_dentry;
use crate::fs::path_to_father_dentry;
use crate::mm::{translated_refmut,translated_byte_buffer, translated_str};
use crate::task::{current_task, current_user_token};
use alloc::string::String;

use device::BLOCK_DEVICE;
use vfs_defs::Kstat;
use vfs_defs::MountFlags;

use vfs_defs::{OpenFlags,UserBuffer};
use vfs::FILE_SYSTEMS;
//
use crate::mm::frame_alloc_more;
use crate::mm::MapArea;
use crate::mm::MapPermission;
use crate::mm::frame_dealloc;
use crate::mm::MapType;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ptr;

//const HEAP_MAX: usize = 0;
pub const AT_FDCWD: isize = -100;

pub fn sys_write(fd: usize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(translated_byte_buffer(token, buf, len)) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(translated_byte_buffer(token, buf, len)) as isize
    } else {
        -1
    }
}

pub fn sys_openat(pfd:isize,path: *const u8, flags: u32,_mode:u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if path.chars().next() == Some('/') || pfd == AT_FDCWD{
        if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
            let mut inner = task.inner_exclusive_access();
            let fd = inner.alloc_fd();
            inner.fd_table[fd] = Some(inode);
            return fd as isize;
        } else {
            return -1;
        }
    }
    let mut inner = task.inner_exclusive_access();
    if let Some(file) = &inner.fd_table[pfd as usize]{
        let father_path = file.get_dentry().path();
        let child_path = father_path+&path;
        if let Some(inode) = open_file(child_path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
            let fd = inner.alloc_fd();
            inner.fd_table[fd] = Some(inode);
            return fd as isize;
        } else {
            return -1;
        }

    }
    return -1;

}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    //let mut inner = task.acquire_inner_lock();
    let mut inner = task.inner_exclusive_access();
    //自身目录项
    let self_dentry = inner.cwd.clone();
    let (pipe_read, pipe_write) = make_pipe(self_dentry); //创建一个管道并获取其读端和写端
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    // 文件描述符写回到应用地址空间
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    0
}

//char *buf, size_t size;
//long ret = syscall(SYS_getcwd, buf, size);
pub fn sys_getcwd(cwd: *mut u8, size: usize) -> isize {
    if size <= 0 {
        return -1;
    }
    let binding = current_task().unwrap();
    let task_inner = binding.inner_exclusive_access();
    let current_path = task_inner.cwd.path();
    if current_path.len() >= size {
        return -2;
    }
    let bytes = current_path.as_bytes();
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), cwd, bytes.len());
    }
    bytes.len() as isize
}

//int fd;
//int ret = syscall(SYS_dup, fd);
pub fn sys_dup(fd: usize) -> isize {
    let binding = current_task().unwrap();
    let mut task_inner = binding.inner_exclusive_access();
    let fd_table = &mut task_inner.fd_table;

    // 检查文件描述符的有效性
    if fd >= fd_table.len() {
        return -1; // EBADF: 无效的文件描述符
    }

    // 获取要复制的文件对象
    if let Some(file) = fd_table[fd].clone() { // 使用 clone 提前获取文件对象
        // 找到第一个空闲的文件描述符位置
        let mut new_fd = fd_table.len();
        for (i, entry) in fd_table.iter().enumerate() {
            if entry.is_none() {
                new_fd = i;
                break;
            }
        }

        // 如果没有找到空闲位置，扩展 fd_table
        if new_fd == fd_table.len() {
            fd_table.push(None);
        }

        // 复制文件对象的引用到新的位置
        fd_table[new_fd] = Some(file);

        // 返回新的文件描述符
        new_fd as isize
    } else {
        return -1;
    }
}

//int old, int new;
//int ret = syscall(SYS_dup3, old, new, 0);
pub fn sys_dup3(old: usize, new: usize, _flags: usize) -> isize {
    /*if old<0 || new<0 {
        return -1;
    }*/
    if old == new {
        return new as isize;
    }
    let binding = current_task().unwrap();
    let mut task_inner = binding.inner_exclusive_access();
    let fd_table = &mut task_inner.fd_table;

    // 检查文件描述符的有效性
    if old >= fd_table.len() {
        return -1; // EBADF: 无效的文件描述符
    }
    // 获取要复制的文件对象
    if let Some(file) = fd_table[old].clone() { // 使用 clone 提前获取文件对象
        
        if new >= fd_table.len() {
            let cnt = new - fd_table.len() + 1;
            for _ in 0..cnt {
                fd_table.push(None);
            }
            if new != fd_table.len()-1 {
                panic!("extend fd_table error!, len={}",fd_table.len());
            }
        }
        else if fd_table[new].is_some() {
            // new位置有效，需要关闭文件
            //sys_close(new); 被锁阻塞
            fd_table[new].take();
        }

        // 复制文件对象的引用到新的位置
        fd_table[new] = Some(file);

        // 返回新的文件描述符
        new as isize
    } else {
        return -1;
    }
}

pub fn sys_mount(_special:*const u8,dir:*const u8,fstype:*const u8,_flags:u32,_data:*const u8)->isize{
    let token = current_user_token();
    let dir = translated_str(token, dir);
    let fstype = translated_str(token, fstype);
    let ext4fstype = FILE_SYSTEMS.lock().find_fs(&String::from("Ext4")).unwrap();
    if fstype == "vfat"{
        let mut name = String::new();
        let parent = path_to_father_dentry(dir.as_str(), &mut name);
        let device = BLOCK_DEVICE.get().unwrap().clone();
        let r = ext4fstype.mount(name.as_str(), parent, MountFlags::empty(), Some(device));
        if r.is_err(){
            return -1;
        }
        return 0;
    }
    else{
        return -1;
    }
}

pub fn sys_umount(special:*const u8,_flags:u32)->isize{
    let token = current_user_token();
    let path = translated_str(token, special);
    let ext4fstype = FILE_SYSTEMS.lock().find_fs(&String::from("Ext4")).unwrap();
    if let Some(dentry) = path_to_dentry(&path){
        if let Err(_e) = ext4fstype.umount(dentry.path().as_str(), MountFlags::empty()){
            return -1;
        }
        return 0;
    }
    return -1;

}

pub fn sys_fstat(fd:usize,kst:*mut Kstat)->isize{
    let token = current_user_token();
    let task = current_task().unwrap();
    if let Some(file) = task.inner_exclusive_access().fd_table[fd].clone() {
        let r = file.get_dentry().get_inode().unwrap().get_attr();
        if r.is_err(){
            return -1;
        }    
        let kst = translated_refmut(token, kst);
        let attr = r.unwrap();
        *kst = attr;
        return 0;
    }
    return -1;
}