//! `Arc<Inode>` -> `OSInodeInner`: In order to open files concurrently
//! we need to wrap `Inode` into `Arc`,but `Mutex` in `Inode` prevents
//! file systems from being accessed simultaneously
//!
//! `UPSafeCell<OSInodeInner>` -> `OSInode`: for static `ROOT_INODE`,we
//! need to wrap `OSInodeInner` into `UPSafeCell`
use core::str::ParseBoolError;

//use super::File;
use crate::{drivers::BLOCK_DEVICE, task::current_task};
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use easy_fs::{EasyFileSystem,DIRENT_SZ,INODE_MANAGER};
use lazy_static::*;
use spin::Mutex;
use alloc::string::String;
use vfs::get_root_dentry;
use vfs_defs::{Inode,DiskInodeType,File,OpenFlags,Dentry};
use system_result::{SysResult,SysError};


/* 
/// A wrapper around a filesystem inode
/// to implement File trait atop
pub struct OSInode {
    readable: bool,
    writable: bool,
    inner: Mutex<OSInodeInner>,
}
/// The OS inode inner in 'UPSafeCell'
pub struct OSInodeInner {
    offset: usize,
    inode: Arc<Inode>,
}

impl OSInode {
    /// Construct an OS inode from a inode
    pub fn new(readable: bool, writable: bool, inode: Arc<Inode>) -> Self {
        Self {
            readable,
            writable,
            inner: Mutex::new(OSInodeInner { offset: 0, inode }) ,
        }
    }
    /// Read all data inside a inode into vector
    pub fn read_all(&self) -> Vec<u8> {
        let mut inner = self.inner.lock();
        let mut buffer = [0u8; 512];
        let mut v: Vec<u8> = Vec::new();
        loop {
            let len = inner.inode.read_at(inner.offset, &mut buffer);
            if len == 0 {
                break;
            }
            inner.offset += len;
            v.extend_from_slice(&buffer[..len]);
        }
        v
    }
}*/
///List all files in the filesystems
pub fn list_apps() {
    println!("/**** APPS ****");
    let root_dentry = get_root_dentry();
    for app in root_dentry.ls(){
        println!("{}", app);
    }
   println!("**************/");
}
///
pub fn create_file(path:&str,type_:DiskInodeType)->SysResult<Arc<dyn Dentry>>{
    if let Ok(inode) = path_to_dentry(path){
        return Ok(inode);
    }
    let mut name = String::new();
    let parent = path_to_father_dentry(path,&mut name)?;
    let dentry = parent.create(name.as_str(),type_).unwrap();
    if type_ == DiskInodeType::Directory{
        let current = dentry.find_or_create(".", DiskInodeType::Directory);
        let mut res = dentry.link(&current);
        if let Err(e) = res{
            return Err(e);
        }
        let to_parent = dentry.find_or_create("..", DiskInodeType::Directory);
        res = parent.link(&to_parent); 
        if let Err(e) = res{
            return Err(e);
        }
    }
    return Ok(dentry);
}
///Open file with flags
pub fn open_file(path: &str, flags: OpenFlags) -> SysResult<Arc<dyn File>>{//还需增加对设备文件的支持
    let ret;
    if flags.contains(OpenFlags::CREATE) {// create file
        let dentry = create_file(path, DiskInodeType::File)?;
        dentry.get_inode().unwrap().clear();
        ret = dentry.open(flags);
    } else {
        let dentry = path_to_dentry(path)?;
        if dentry.is_dir() && ((flags.bits()&OpenFlags::RDONLY.bits()) != OpenFlags::RDONLY.bits()){
            return Err(SysError::EACCES);
        }
        if flags.contains(OpenFlags::TRUNC) && dentry.is_file(){
            dentry.get_inode().unwrap().clear();
        }
        ret = dentry.open(flags);
    }  
    Ok(ret)
}
/*
impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn read(&self, mut buf: UserBuffer) -> usize {
        let mut inner = self.inner.lock();
        let mut total_read_size = 0usize;
        for slice in buf.buffers.iter_mut() {
            let read_size = inner.inode.read_at(inner.offset, *slice);
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        total_read_size
    }
    fn write(&self, buf: UserBuffer) -> usize {
        let mut inner = self.inner.lock();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = inner.inode.write_at(inner.offset, *slice);
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        total_write_size
    }
}
 */
fn skipelem<'a>(path: &'a str, name: &mut String) -> Option<&'a str> {
    // 跳过开头的斜杠
    let mut path = path.trim_start_matches('/');
    
    if path.is_empty() {
        return None;
    }

    // 找到下一个斜杠或者结束
    let end_pos = path.find('/').unwrap_or(path.len());

    // 获取路径元素并存入 `name`
    let elem = &path[..end_pos];
    if elem.len() >= 32 {
        // 需要截断
        name.clear();
        name.push_str(&elem[..32]);
    } else {
        name.clear();
        name.push_str(elem);
    }

    // 返回剩余的路径部分，跳过一个斜杠
    path = &path[end_pos..].trim_start_matches('/');

    Some(path)
}
fn path_to_dirent_(path:&str,
                  to_father:bool,
                  name:&mut String,
                )->SysResult<Arc<dyn Dentry>>{
  //  let efs =  fs.lock();
    let mut dentry ;
    let mut current = path;
    if path.chars().next() == Some('/'){
        dentry = get_root_dentry();
    }
    else{
            //获得当前进程的cwd
        if let Some(current_task) = current_task(){
            dentry = current_task.inner_exclusive_access().cwd.clone();
        }
        else{
            dentry = get_root_dentry();
        }
    }
    while let Some(new_path) = skipelem(current,name){
        current = new_path;
        if name == "."{
            continue;
        }
        if name == ".."{
            if let Some(father) = dentry.get_father(){
                dentry = father;
                continue;
            }else{
                return Err(SysError::ENOENT);
            }
        }
        if to_father && current.len() == 0 {
            return Ok(dentry);
        }
        dentry = dentry.lookup(name)?;
    }
    return Ok(dentry);

}
/// get inode from path,get a clone of inode's Arc
pub fn path_to_dentry(path:&str)->SysResult<Arc<dyn Dentry>>{
    let mut name = String::new();
    path_to_dirent_(path, false,&mut name)
}
/// get father inode from path ,get a clone of inode's Arc
pub fn path_to_father_dentry(path:&str,name:&mut String)->SysResult<Arc<dyn Dentry>>{
    path_to_dirent_(path, true,name)

}
