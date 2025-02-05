//! `Arc<Inode>` -> `OSInodeInner`: In order to open files concurrently
//! we need to wrap `Inode` into `Arc`,but `Mutex` in `Inode` prevents
//! file systems from being accessed simultaneously
//!
//! `UPSafeCell<OSInodeInner>` -> `OSInode`: for static `ROOT_INODE`,we
//! need to wrap `OSInodeInner` into `UPSafeCell`
use super::File;
use crate::{drivers::BLOCK_DEVICE, task::current_task};
use crate::mm::UserBuffer;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use easy_fs::{EasyFileSystem, Inode,DIRENT_SZ,DiskInodeType,INODE_MANAGER};
use lazy_static::*;
use spin::Mutex;
use alloc::string::String;
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
}
lazy_static! {
    ///
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        EasyFileSystem::root_inode(&efs)
    };
}
///List all files in the filesystems
pub fn list_apps() {
    println!("/**** APPS ****");
    for app in ROOT_INODE.ls() {
        println!("{}", app);
    }
    println!("**************/");
}

bitflags! {
    ///Open file flags
    pub struct OpenFlags: u32 {
        ///Read only
        const RDONLY = 0;
        ///Write only
        const WRONLY = 1 << 0;
        ///Read & Write
        const RDWR = 1 << 1;
        ///Allow create
        const CREATE = 1 << 9;
        ///Clear file and return an empty one
        const TRUNC = 1 << 10;
    }
}

impl OpenFlags {
    /// Do not check validity for simplicity
    /// Return (readable, writable)
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }
}
///
pub fn create_file(path:&str,type_:DiskInodeType,fs: Arc<Mutex<EasyFileSystem>>,)->Option<Arc<Inode>>{
    if let Some(inode) = path_to_inode(path, fs.clone()){
        return Some(inode);
    }
    let mut name = String::new();
    if let Some(parent) = path_to_father_inode(path,fs.clone(),&mut name){
        if let Some(inode) = parent.create(name.as_str(),type_){
            if type_ == DiskInodeType::Directory{
                if inode.link(".", inode.block_id as u32, inode.block_offset) < 0 
                || inode.link("..", parent.block_id as u32, parent.block_offset) < 0{
                    inode.lock_inner().link_count = 0;
                    return None;
                }
                parent.lock_inner().link_count += 1;
            }
            return Some(inode);
        }
        else{
            return None;
        }
    }
    else{
        return None;
    }
}
///Open file with flags
pub fn open_file(path: &str, flags: OpenFlags) -> Option<Arc<OSInode>> {//还需增加对设备文件的支持
    let (readable, writable) = flags.read_write();
    let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
    let ret;
    if flags.contains(OpenFlags::CREATE) {// create file
        if let Some(inode) = create_file(path, DiskInodeType::File,efs.clone()){
            inode.clear();
            ret = Some(Arc::new(OSInode::new(readable, writable, inode)));
        } else {
            return None;
                //.map(|inode| Arc::new(OSInode::new(readable, writable, inode)))
        }
    } else {
        if let Some(inode) = path_to_inode(path, efs.clone()){
            if inode.lock_inner().is_dir() && flags != OpenFlags::RDONLY{
                return None;
            }
            if flags.contains(OpenFlags::TRUNC) && inode.lock_inner().is_file(){
                inode.clear();
            }
            ret = Some(Arc::new(OSInode::new(readable, writable, inode)));
        }
        else{
            return None;
        }
    }  
    ret
}

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
fn path_to_inode_(path:&str,
                  to_father:bool,
                  fs: Arc<Mutex<EasyFileSystem>>,
                  name:&mut String,
                )->Option<Arc<Inode>>{
  //  let efs =  fs.lock();
    let mut inode ;
    let mut current = path;
    if path.chars().next() == Some('/'){
        inode = EasyFileSystem::root_inode(&fs);
    }
    else{
            //获得当前进程的cwd
        if let Some(current_task) = current_task(){
            inode = current_task.inner_exclusive_access().cwd.clone();
        }
        else{
            inode = EasyFileSystem::root_inode(&fs);
        }
    }
    while let Some(new_path) = skipelem(current,name){
        current = new_path;
        if to_father && current.len() == 0 {
            return Some(inode);
        }
        if let Some(new_inode) = inode.find(name){
            inode = new_inode;
        }
        else{
            return None;
        }
        
    }
    return Some(inode);

}
/// get inode from path,get a clone of inode's Arc
pub fn path_to_inode(path:&str,fs: Arc<Mutex<EasyFileSystem>>)->Option<Arc<Inode>>{
    let mut name = String::new();
    path_to_inode_(path, false, fs,&mut name)
}
/// get father inode from path ,get a clone of inode's Arc
pub fn path_to_father_inode(path:&str,fs: Arc<Mutex<EasyFileSystem>>,name:&mut String)->Option<Arc<Inode>>{
    path_to_inode_(path, true, fs,name)

}
