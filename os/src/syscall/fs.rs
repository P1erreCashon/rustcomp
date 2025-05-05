//! File and filesystem-related syscalls
use config::USER_STACK_TOP;
use crate::fs::{open_file,create_file};
use crate::fs::make_pipe;
use crate::fs::path_to_dentry;
use crate::fs::path_to_father_dentry;
use crate::mm::{safe_translated_refmut, translated_byte_buffer, translated_ref, translated_refmut, translated_str,safe_translated_byte_buffer,MmapFlags,MapAreaType};
use crate::task::{current_task, current_user_token, Fd, FdFlags, TimeSpec};
use alloc::string::String;

use arch::addr::{VirtAddr, VirtPage};
use arch::PAGE_SIZE;
use arch::time::Time;
use device::BLOCK_DEVICE;
use vfs_defs::{Kstat, PollEvents};
use vfs_defs::MountFlags;

use vfs_defs::{OpenFlags,UserBuffer,StatFs,SeekFlags,RenameFlags};
use vfs::FILE_SYSTEMS;
//
use crate::mm::frame_alloc_more;
use crate::mm::MapArea;
use crate::mm::MapPermission;
use crate::mm::frame_dealloc;
use crate::mm::MapType;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ptr::{self, addr_of_mut, null_mut};
use core::{mem, slice};
use alloc::{task, vec};
use system_result::{SysError,SysResult};


//const MODULE_LEVEL:log::Level = log::Level::Debug;
//const HEAP_MAX: usize = 0;
pub const AT_FDCWD: isize = -100;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IoVec {
    pub base: usize,
    pub len: usize,
}

pub fn sys_write(fd: usize, buf: *mut u8, len: usize) -> SysResult<isize> {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(fd)?;
    let file = file.file();
    drop(fdtable);
    if !file.writable() {
        return Err(SysError::EPERM);
    }
    let file = file.clone();
    // release current task TCB manually to avoid multi-borrow
    drop(inner);
    Ok(file.write(translated_byte_buffer(token, buf, len)) as isize)
}


pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> SysResult<isize> {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(fd)?;
    let file = file.file();
    drop(fdtable);
    if !file.readable() {
        return Err(SysError::EPERM);
    }
    // release current task TCB manually to avoid multi-borrow
    drop(inner);
    Ok(file.read(translated_byte_buffer(token, buf, len)) as isize)
}

pub fn sys_openat(pfd:isize,path: *const u8, flags: u32,_mode:u32) -> SysResult<isize> {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if path.chars().next() == Some('/') || pfd == AT_FDCWD{
        let inode = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap())?;
        let inner = task.inner_exclusive_access();
        let fd = inner.fd_table.lock().insert(Some(Fd::new(inode, FdFlags::empty())))?;
        return Ok(fd as isize);
    }
    let inner = task.inner_exclusive_access();
    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(pfd as usize)?;
    let file = file.file();
    drop(fdtable);
    let father_path = file.get_dentry().path();
    let child_path = father_path+&path;
    let inode = open_file(child_path.as_str(), OpenFlags::from_bits(flags).unwrap())?;
    let fd = inner.fd_table.lock().insert(Some(Fd::new(inode, FdFlags::empty())))?;
    return Ok(fd as isize);
}

pub fn sys_close(fd: usize) -> SysResult<isize> {
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let _ = inner.fd_table.lock().remove(fd)?;
    return Ok(0);
}

///
pub fn sys_mkdirat(pfd:isize,path: *const u8,_mode:u32) -> SysResult<isize> {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if path.chars().next() == Some('/') || pfd == AT_FDCWD{
        let _inode = create_file(path.as_str(), vfs_defs::DiskInodeType::Directory)?;
        return Ok(0);
    }
    let inner = task.inner_exclusive_access();
    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(pfd as usize)?;
    let file = file.file();
    drop(fdtable);
    let father_path = file.get_dentry().path();
    let child_path = father_path+&path;
    let _inode = create_file(child_path.as_str(), vfs_defs::DiskInodeType::Directory)?;
    return Ok(0);
}

pub fn sys_pipe(pipe: *mut i32) -> SysResult<isize> {
    let task = current_task().unwrap();
    let token = current_user_token();
    //let mut inner = task.acquire_inner_lock();
    let inner = task.inner_exclusive_access();
    //自身目录项
    let self_super = inner.cwd.get_superblock();
    let (pipe_read, pipe_write) = make_pipe(self_super); //创建一个管道并获取其读端和写端
    let read_fd = inner.fd_table.lock().insert(Some(Fd::new(pipe_read, FdFlags::empty())))?;
    let write_fd = inner.fd_table.lock().insert(Some(Fd::new(pipe_write, FdFlags::empty())));
    if let Err(e) = write_fd{
        let _ = inner.fd_table.lock().remove(read_fd);
        return Err(e);
    }
    let write_fd = write_fd.unwrap();
    // 文件描述符写回到应用地址空间
    *translated_refmut(token, pipe) = read_fd as i32;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd as i32;
    Ok(0)
}

//char *buf, size_t size;
//long ret = syscall(SYS_getcwd, buf, size);
pub fn sys_getcwd(cwd: *mut u8, size: usize) -> SysResult<isize> {
    if size <= 0 {
        return Err(SysError::EINVAL);
    }
    let binding = current_task().unwrap();
    let token = current_user_token();
    let task_inner = binding.inner_exclusive_access();
    let current_path = task_inner.cwd.path();
    if current_path.len() >= size {
        return Err(SysError::ENOENT);
    }
    let bytes = current_path.as_bytes();
    let cwd = translated_byte_buffer(token, cwd, bytes.len());
    cwd.copy_from_slice(bytes);
    Ok(bytes.len() as isize)
}

//int fd;
//int ret = syscall(SYS_dup, fd);
pub fn sys_dup(fd: usize) -> SysResult<isize> {
    let binding = current_task().unwrap();
    let task_inner = binding.inner_exclusive_access();
    return task_inner.fd_table.lock().dup(fd);
}

//int old, int new;
//int ret = syscall(SYS_dup3, old, new, 0);
pub fn sys_dup3(old: usize, new: usize, _flags: usize) -> SysResult<isize> {
    /*if old<0 || new<0 {
        return -1;
    }*/
    if old == new {
        return Ok(new as isize);
    }
    let binding = current_task().unwrap();
    let task_inner = binding.inner_exclusive_access();
    let fd_table = &task_inner.fd_table;
    return  fd_table.lock().dup3(old, new, _flags);
}

pub fn sys_mount(_special:*const u8,dir:*const u8,fstype:*const u8,_flags:u32,_data:*const u8)->SysResult<isize>{
    let token = current_user_token();
    let dir = translated_str(token, dir);
    let fstype = translated_str(token, fstype);
    let ext4fstype = FILE_SYSTEMS.lock().find_fs(&String::from("Ext4")).unwrap();
    if fstype == "vfat"{
        let mut name = String::new();
        let parent = path_to_father_dentry(dir.as_str(), &mut name);
        let mount_parent;
        let device = BLOCK_DEVICE.get().unwrap().clone();
        if let Ok(p) = parent{
            mount_parent = Some(p);
        }
        else {
            mount_parent = None;
        }
        let r = ext4fstype.mount(name.as_str(), mount_parent, MountFlags::empty(), Some(device));
        if let Err(e) = r{
            return Err(e);
        }
        return Ok(0);
    }
    else{
        return Err(SysError::EINVAL);
    }
}

pub fn sys_umount(special:*const u8,_flags:u32)->SysResult<isize>{
    let token = current_user_token();
    let path = translated_str(token, special);
    let ext4fstype = FILE_SYSTEMS.lock().find_fs(&String::from("Ext4")).unwrap();
    let dentry = path_to_dentry(&path)?;
    if let Err(_e) = ext4fstype.umount(dentry.path().as_str(), MountFlags::empty()){
        return Ok(0);
       // return Err(e);
    }
    return Ok(0);
}

pub fn sys_fstat(fd:usize,kst:*mut Kstat)->SysResult<isize>{
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(fd)?;
    let file = file.file();
    drop(fdtable);
    let attr = file.get_attr()?;
    let kst = translated_refmut(token, kst);
    *kst = attr;
    return Ok(0);
}

pub fn sys_fstatat(dirfd:usize,path:*const u8,kst:*mut Kstat,_flags:i32)->SysResult<isize>{
    let path = parse_fd_path(dirfd as isize, path)?;
    let old = path_to_dentry(&path)?;
    let attr = old.get_inode().unwrap().get_attr()?;
    let token = current_user_token();
    let kst = translated_refmut(token, kst);
    *kst = attr;
    return Ok(0);
}

/*
addr:
指定映射被放置的虚拟地址，如果将addr指定为NULL，那么内核会为映射分配一个合适的地址。如果addr为一个非NULL值，
则内核在选择地址映射时会将该参数值作为一个提示信息来处理。不管采用何种方式，内核会选择一个不与任何既有映射冲突的地
址。在处理过程中， 内核会将指定的地址舍入到最近的一个分页边界处。
length:
参数指定了映射的字节数。尽管length 无需是一个系统分页大小的倍数，但内核会以分页大小为单位来创建映射，
因此实际上length会被向上提升为分页大小的下一个倍数。
prot: 映射的内存保护方式，可取：PROT_EXEC, PROT_READ, PROT_WRITE, PROT_NONE
flags: 映射是否与其他进程共享的标志，
fd: 文件句柄，
off: 文件偏移量 =? (off,off+len)
返回值：成功返回已映射区域的指针，失败返回-1;
void *start, size_t len, int prot, int flags, int fd, off_t off
long ret = syscall(SYS_mmap, start, len, prot, flags, fd, off);
不需要实际分配物理地址
*/
pub fn sys_mmap(
    _start: *mut usize,
    len: usize,
    mprot: i32,
    flags: i32,
    fd: usize,
    off: i32,
) -> SysResult<isize> {
    if len==0 || off!=0 {
        return Err(SysError::EINVAL);
    }
    // 长度对齐
    let mut num = len / PAGE_SIZE;//num:页数量
    if num*PAGE_SIZE != len {
        num = num + 1;
    }
    
    let mut prot = MapPermission::U;
    if mprot & 0x1 != 0{
        prot |= MapPermission::R;
    }
    if mprot & 0x2 != 0{
        prot |= MapPermission::W;
    }    
    if mprot & 0x4 != 0{
        prot |= MapPermission::X;
    }
    let flags = MmapFlags::from_bits_truncate(flags);
    let mut map_file = None;
    let task = current_task().unwrap();
    if !flags.contains(MmapFlags::MAP_ANONYMOUS){
        let inner = task.inner_exclusive_access();
        let fdtable = inner.fd_table.lock();
        let file = fdtable.get(fd)?;
        let file = file.file();
        drop(fdtable);
        if !file.readable() {
            return Err(SysError::EACCES);
        }
        map_file = Some(file);
            // release current task TCB manually to avoid multi-borrow
        drop(inner);
    }
    if flags.contains(MmapFlags::MAP_FIXED){
        let inner = task.inner_exclusive_access();
        let mut area =MapArea::new(VirtAddr::new(_start as usize),VirtAddr::new(_start as usize + len),MapType::Framed,prot,MapAreaType::Mmap,);
        area.map_file = map_file;
        area.mmap_flag = flags;
     //   println!("mmap fixed start:{:x} end:{:x} prot:{:x} {:x}",_start as usize,_start as usize + len,mprot,prot);
        inner.memory_set.lock().debug_addr_info();
        inner.memory_set.lock().push_into_area_lazy(area);
        inner.memory_set.lock().debug_addr_info();
        return Ok(_start as isize);
    }
    let inner = task.inner_exclusive_access();
    let mut memory_set = inner.memory_set.lock();
    let start = memory_set.mapareacontrol.mmap_top;
    if len + start >= USER_STACK_TOP {
        // 与栈重叠
        return Err(SysError::ENOMEM);
    }
    let start_va = VirtAddr::new(start);
    let end_va = VirtAddr::new(start + len);
    let mut area =MapArea::new(start_va,end_va,MapType::Framed,prot,MapAreaType::Mmap,);
    area.map_file = map_file;
    area.mmap_flag = flags;
  //  println!("mmap start:{:x} end:{:x} prot:{:x} {:x}",start as usize,start as usize + len,mprot,prot);
    memory_set.push_into_area_lazy(area);
    memory_set.mapareacontrol.mmap_top += num * PAGE_SIZE;
    return Ok(start as isize);
    
}
//void *start, size_t len
//int ret = syscall(SYS_munmap, start, len);
pub fn sys_munmap(start: *mut usize, len: usize) -> SysResult<isize> {
    if start as usize/ PAGE_SIZE *PAGE_SIZE != start as usize {
        // 未对齐，地址错误
        return Err(SysError::EINVAL);
    };
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let mut memory_set = inner.memory_set.lock();
    memory_set.munmap( start as usize, len)
}
pub fn sys_getdents(fd:usize,buf:*mut u8,len:usize)->SysResult<isize>{
    #[derive(Debug, Clone, Copy)]
    #[repr(C)]
    struct SyscallDirent {
        d_ino: u64,
        d_off: u64,
        d_reclen: u16,
        d_type: u8,
    }
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let buf = safe_translated_byte_buffer(inner.memory_set.clone(), buf, len);
    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(fd)?;
    let file = file.file();
    drop(fdtable);
    let _ = file.load_dir()?;
    let mut write_size = 0;
    let mut buf_slice = buf;
    let mut offset = file.get_offset();
    let file_dentry = file.get_dentry();
    let children = file_dentry.get_inner().children.lock();
    for dentry in children.values().skip(*offset){
        let dentry = dentry.upgrade().unwrap();
        if dentry.has_no_inode(){
            *offset+=1;
            continue;
        }
        let name_size = dentry.get_name_str().len() + 1;
        let d_reclen = ((19 + name_size + 7)  & !0x7) as u16;
        let d_off = *offset as u64;
        let inode = dentry.get_inode().unwrap();
        let d_ino = inode.get_meta().ino as u64;
        let mut d_type:u8 = 0;
        if inode.is_dir(){
            d_type = 4;
        }
        else if inode.is_file(){
            d_type = 8;
        }
        let syscall_dirent = SyscallDirent{
            d_ino,
            d_off,
            d_reclen,
            d_type,
        };
        if write_size + d_reclen > len as u16{
            break;
        }
        *offset +=1;
        let ptr = buf_slice.as_mut_ptr() as *mut SyscallDirent;
        *safe_translated_refmut(inner.memory_set.clone(), ptr) = syscall_dirent;
        buf_slice[19..19 + name_size - 1].copy_from_slice(dentry.get_name_str().as_bytes());
        buf_slice[19 + name_size - 1] = b'\0';
        buf_slice = &mut buf_slice[d_reclen as usize..];
        write_size += d_reclen;
    }
    return Ok(write_size as isize);
}

///
pub fn sys_link(old_dirfd:isize,old_path: *const u8,new_dirfd:isize,new_path:*const u8,_flags:u32) -> SysResult<isize> {
    let old_path = parse_fd_path(old_dirfd, old_path)?;
    let new_path = parse_fd_path(new_dirfd, new_path)?;
    let dentry = path_to_dentry(&old_path)?;
    if dentry.is_dir(){
        drop(dentry);
        return Err(SysError::EISDIR);
    }
    let mut name = String::new();
    let father_dentry = path_to_father_dentry(&new_path,&mut name)?;
    let r = father_dentry.lookup(name.as_str());
    if r.is_ok(){//EEXIST
        return Err(SysError::EEXIST);
    }
    let new_dentry = father_dentry.find_or_create(name.as_str(), vfs_defs::DiskInodeType::File);
    let _ = dentry.link(&new_dentry)?;
    return Ok(0);
}

///
pub fn sys_unlink(dirfd:isize,path: *const u8,_flags:u32) -> SysResult<isize> {
    let path = parse_fd_path(dirfd, path)?;
    let mut name = String::new();
    let father = path_to_father_dentry(&path,&mut name)?;
    if name.eq(".") || name.eq(".."){
        return Err(SysError::EINVAL);
    }
    let old = path_to_dentry(&path)?;
    father.unlink(&old)?;
    return Ok(0);
}

fn parse_fd_path(fd: isize,path:*const u8)->SysResult<String>{
   // if fd == 0 || fd == 1{
  //      return None;
  //  }
    let token = current_user_token();
    let path = translated_str(token, path);
    if path.chars().next() == Some('/') || fd == AT_FDCWD{
        return Ok(path);
    }
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(fd as usize)?;
    let file = file.file();
    drop(fdtable);
    let father_path = file.get_dentry().path();
    let child_path = father_path+&path;
    drop(inner);
    return Ok(child_path);
}

pub fn sys_writev(fd:isize,iov:*const IoVec,iovcnt:usize)->SysResult<isize>{
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(fd as usize)?;
    let file = file.file();
    drop(fdtable);
    let mut offset = file.get_offset();
    let mut total_write_size = 0;
    let mut iov_iter = iov;
   
    for _i in 0..iovcnt{ 
    //    println!("iov:{:?}",iov_iter);
        let iovs = translated_ref(token, iov_iter);
        if iovs.len == 0{
            unsafe {
                let _ = iov_iter.add(1);
            }
            continue;
        }
    //    println!("writev:write len:{}",iovs.len);
        let ptr = iovs.base;
        let buf = translated_byte_buffer(token, ptr as *mut u8, iovs.len);
        let write_size = file.write_at(*offset, buf);
        total_write_size += write_size;
        *offset += write_size;
        unsafe {
            iov_iter = iov_iter.add(1);
        }
    }
    drop(offset);
    file.seek(total_write_size as i64, vfs_defs::SeekFlags::SEEK_CUR)?;
    return Ok(total_write_size as isize);
}

pub fn sys_statfs(_path:*const u8,buf:*mut StatFs)->SysResult<isize>{
    let token = current_user_token();
    let buf = translated_refmut(token, buf);
    *buf =  StatFs {
        f_type: 0x2011BAB0 as i64,
        f_bsize: vfs::BLOCK_SIZE as i64,
        f_blocks: 1 << 27,
        f_bfree: 1 << 26,
        f_bavail: 1 << 20,
        f_files: 1 << 10,
        f_ffree: 1 << 9,
        f_fsid: [0; 2],
        f_namelen: 1 << 8,
        f_frsize: 1 << 9,
        f_flags: 1 << 1 as i64,
        f_spare: [0; 4],
    };
    Ok(0)
}

pub fn sys_faccessat(dirfd:isize,path:*const u8,_mode:usize,flags:i32)->SysResult<isize>{
    let path = parse_fd_path(dirfd, path)?;
    if flags == 0x100 as i32 {//no symlink now
        return Err(SysError::EINVAL);
    }
    let _file = open_file(path.as_str(), OpenFlags::empty())?;
    return Ok(0);
}

pub fn sys_lseek(fd:isize,offset:isize,whence:usize)->SysResult<isize>{
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let fdtable = inner.fd_table.lock();
    let file = fdtable.get(fd as usize)?;
    let _file = file.file();
    drop(fdtable);
    match whence{
        0=>{
            return _file.seek(offset as i64, SeekFlags::SEEK_SET);
        },
        1=>{
            return _file.seek(offset as i64, SeekFlags::SEEK_CUR);
        },
        2=>{
            return _file.seek(offset as i64, SeekFlags::SEEK_END);
        },
        _=>{}
    }
    return Err(SysError::EINVAL);
}

pub fn sys_utimensat(dirfd:isize,path:*const u8,times:*const crate::task::TimeSpec,flags:i32)->SysResult<isize>{
    let inode;
    if path.is_null(){
        match  dirfd {
            AT_FDCWD=>{
                return Err(SysError::EINVAL);
            }
            _=>{
                let task = current_task().unwrap();
                let inner = task.inner_exclusive_access();
                inode = inner.fd_table.lock().get_file(dirfd as usize)?.get_dentry().get_inode()?;
            }
        }
    }
    else{
        let path = parse_fd_path(dirfd, path)?;
        let flags = OpenFlags::from_bits(flags as u32);
        if flags.is_none(){
            return Err(SysError::EINVAL);
        }
        inode = open_file(path.as_str(), flags.unwrap())?.get_dentry().get_inode()?
    }
    let current = Time::now();
    let mut inner = inode.get_meta().inner.lock();
    let timespec_now = TimeSpec{
        sec:current.to_sec(),
        usec:current.to_usec()
    };
    if times.is_null(){
        inner.atime = timespec_now;
        inner.mtime = timespec_now;
        inner.ctime = timespec_now;
    }
    else{
        const UTIME_NOW: usize = 0x3fffffff;
        const UTIME_OMIT: usize = 0x3ffffffe;
        let token = current_user_token();
        let times1 = translated_ref(token, times);
        let times2;
        unsafe {
            times2 = translated_ref(token, times.add(1));
        }
        match times1.usec{
            UTIME_NOW=>{
                inner.atime = timespec_now;
            }
            UTIME_OMIT=>{}
            _=>{
                inner.atime = *times1;
            }
        }
        match times2.usec{
            UTIME_NOW=>{
                inner.mtime = timespec_now;
            }
            UTIME_OMIT=>{}
            _=>{
                inner.mtime = *times2;
            }
        }
        inner.ctime = timespec_now;
    }
    Ok(0)
}
const F_DUPFD:isize = 0;
const F_DUPFD_CLOEXEC:isize = 1030;
const F_GETFD:isize = 1;
const F_SETFD:isize = 2;
const F_GETFL:isize = 3;
const F_SETFL:isize = 4;
//F_UNIMPL,
pub fn sys_fcntl(fd:isize,op:isize,arg:usize)->SysResult<isize>{
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let mut fdtable = inner.fd_table.lock();
    match op{
        F_DUPFD=>{
            inner.fd_table.lock().dup_with_arg(fd as usize, arg,OpenFlags::empty())
        }
        F_DUPFD_CLOEXEC=>{
            inner.fd_table.lock().dup_with_arg(fd as usize, arg,OpenFlags::CLOEXEC)
        }
        F_GETFD=>{
            let fd = fdtable.get(fd as usize)?;
            return Ok(fd.get_flags().bits() as isize);
        }
        F_SETFD=>{
            let fd = fdtable.get_mut(fd as usize)?;
            fd.set_flags(FdFlags::from(OpenFlags::from_bits_retain(arg as u32)));
            return Ok(0);
        }
        F_GETFL=>{
            let file = fdtable.get(fd as usize)?;
            let file = file.file();
            let flag = file.get_inner().flags.lock();
            return Ok(flag.bits() as isize);
        }
        F_SETFL=>{
            let file = fdtable.get(fd as usize)?;
            let file = file.file();
            *file.get_inner().flags.lock() = OpenFlags::from_bits_retain(arg as u32);
            return Ok(0);
        }
        _ =>{
            return Ok(0);
        }
    }
}

pub fn sys_sendfile(outfd:isize,infd:isize,offset:*mut usize,count:usize)->SysResult<isize>{
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let (outfile,infile) = (inner.fd_table.lock().get_file(outfd as usize)?,inner.fd_table.lock().get_file(infd as usize)?);
    if !infile.readable() || !outfile.writable() {
        return Err(SysError::EBADF);
    }
    let mut buf = vec![0 as u8; count];
    let len;
    if offset.is_null(){
        len = infile.read(&mut buf);
    }
    else{
        let offset = translated_refmut(token, offset);
        len = infile.read_at(*offset, &mut buf);
        *offset = *offset + len;
    }
    let ret = outfile.write(&buf[..len]);
    return Ok(ret as isize);
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct PollFd {
    /// file descriptor
    fd: i32,
    /// requested events    
    events: PollEvents,
    /// returned events
    revents: PollEvents,
}

pub fn sys_poll(fds:*mut PollFd,nfds:usize,_timeout:*const TimeSpec)->SysResult<isize>{
    let token = current_user_token();
    let mut poll_fds:Vec<&mut PollFd> = Vec::new();
    let mut ret = 0;
    for _i in 0..nfds{
        let fd = translated_refmut(token, fds);
        poll_fds.push(fd);
        unsafe {
            let _ = fds.add(1);
        }
    }
    loop{
        for fd in poll_fds.iter_mut(){
            fd.revents = PollEvents::empty();
            if fd.fd < 0 {continue;}
            let mut reti = 0;
            let task = current_task().unwrap();
            let inner = task.inner_exclusive_access();
            let file = inner.fd_table.lock().get_file(fd.fd as usize);
            if file.is_err(){
                fd.revents = fd.revents | PollEvents::POLLINVAL;
                reti = 1;
            }
            else{
                let file = file.unwrap();
                if fd.events.contains(PollEvents::POLLIN){ 
                    drop(inner);
                    if file.poll(PollEvents::POLLIN).contains(PollEvents::POLLIN){
                        fd.revents = fd.revents | PollEvents::POLLIN;
                        reti = 1;
                    }
                }
                if fd.events.contains(PollEvents::POLLOUT){
                    if file.poll(PollEvents::empty()).contains(PollEvents::POLLOUT){
                        fd.revents = fd.revents | PollEvents::POLLOUT;
                        reti = 1;
                    }
                }
            }
            ret += reti;
        }
        if ret != 0{
            break;
        }
    }
    return Ok(ret);
}

pub fn sys_renameat2(olddirfd:isize,oldpath:*const u8,newdirfd:isize,newpath:*const u8,flags:usize)->SysResult<isize>{
    let flags = RenameFlags::from_bits_retain(flags as i32);
    let oldpath = parse_fd_path(olddirfd, oldpath)?;
    let newpath = parse_fd_path(newdirfd, newpath)?;
    let old_dentry = path_to_dentry(&oldpath)?;
    let new_dentry;
    let r = path_to_dentry(&newpath);
    if r.is_ok(){
        new_dentry = r.unwrap();
    }
    else{
        let mut name = String::new();
        let father = path_to_father_dentry(&newpath, &mut name)?;
        new_dentry = father.find_or_create(name.as_str(), *old_dentry.get_inode()?.get_meta()._type.lock());
    }
    if let Err(e)= old_dentry.vfs_rename(&new_dentry, flags){
        return Err(e);
    }
    return Ok(0);
    
}

#[repr(C)]
pub struct Statx {
    stx_mask:u32,        /* 统计信息位标识 */
    stx_blksize:u32,     /* 推荐的 I/O 大小 */
    stx_attributes:u64,  /* 文件属性标志 */
    stx_nlink:u32,       /* 链接数 */
    stx_uid:u32,        /* 所有者用户 ID */
    stx_gid:u32,         /* 所有者组 ID */
    stx_mode:u16,        /* 文件类型和访问模式 */
    __spare0:u16,
    stx_ino:u64,         /* inode 号 */
    stx_size:u64,        /* 文件大小（字节） */
    stx_blocks:u64,      /* 分配的块数 */
    stx_attributes_mask:u64,
    stx_atime:TimeSpec, /* 最后访问时间 */
    stx_btime:TimeSpec, /* 文件创建时间 */
    stx_ctime:TimeSpec, /* 元数据改变时间 */
    stx_mtime:TimeSpec, /* 最后修改时间 */
    stx_rdev_major:u32,  /* 设备号（主） */
    stx_rdev_minor:u32,  /* 设备号（次） */
    stx_dev_major:u32,   /* 文件所在设备号（主） */
    stx_dev_minor:u32,   /* 文件所在设备号（次） */
    __spare2:[u64;14],
}
///
pub fn sys_statx(dirfd:isize,path:*const u8,_flags:i32,_mask:u32,statx:*mut Statx)->SysResult<isize>{
    let token = current_user_token();
    let path = parse_fd_path(dirfd, path)?;
    let dentry = path_to_dentry(path.as_str())?;
    let attr = dentry.get_inode()?.get_attr()?;
    let statx = translated_refmut(token, statx);
    statx.stx_mask = 0;
    statx.stx_blksize = attr.st_blksize;
    statx.stx_attributes = 0;
    statx.stx_nlink = attr.st_nlink;
    statx.stx_uid = 1;
    statx.stx_gid = 1;
    statx.stx_mode = attr.st_mode as u16;
    statx.__spare0 = 0;
    statx.stx_ino = attr.st_ino;
    statx.stx_atime.sec = attr.st_atime_sec as usize;
    statx.stx_atime.usec = attr.st_atime_nsec as usize;
    statx.stx_mtime.sec = attr.st_mtime_sec as usize;
    statx.stx_mtime.usec = attr.st_mtime_nsec as usize;
    statx.stx_ctime.sec = attr.st_ctime_sec as usize;
    statx.stx_ctime.usec = attr.st_ctime_nsec as usize;
    Ok(0)
}