#![no_std]
#![no_main]
extern crate alloc;
use alloc::{collections::BTreeMap, string::{String, ToString}, sync::Arc};
use system_result::{SysResult,SysError};
use easy_fs::EfsFsType;
use ext4::Ext4ImplFsType;
use lazy_static::lazy_static;
use spin::{Mutex,Once};
use vfs_defs::{FileSystemType, MountFlags,Dentry};
use device::BLOCK_DEVICE;

lazy_static!{
    pub static ref FILE_SYSTEMS:Mutex<FileSystemManager> =
    Mutex::new(FileSystemManager::new());
}
pub const ROOT_FS: &str = "Ext4";
pub static ROOT_DENTRY: Once<Arc<dyn Dentry>> = Once::new();
pub struct FileSystemManager{
    file_systems:BTreeMap<String,Arc<dyn FileSystemType>>
}

impl FileSystemManager{
    pub fn new()->Self{
        Self{
            file_systems:BTreeMap::new()
        }
    }
    //return a clone of Arc<dyn FileSystemType>
    pub fn find_fs(&self,name:&String)->Option<Arc<dyn FileSystemType>>{
        if let Some(fs_type) = self.file_systems.get(name){
            return Some(fs_type.clone());
        }
        return None;
    }

    pub fn register_fs(&mut self,name:String,fs_type:Arc<dyn FileSystemType>)->SysResult<()>{
        if let Some(fs) = self.find_fs(&name){
            return Err(SysError::EBUSY);
        }
        self.file_systems.insert(name, fs_type);
        Ok(())
    }
}



pub fn register_all_fs(){
    let mut file_systems = FILE_SYSTEMS.lock();
    let _ = file_systems.register_fs("EasyFs".to_string(), Arc::new(EfsFsType::new()));
    let _ = file_systems.register_fs("Ext4".to_string(), Arc::new(Ext4ImplFsType::new()));
}

pub fn init(){
    register_all_fs();
    let root_fs = FILE_SYSTEMS.lock().file_systems.get(ROOT_FS).unwrap().clone();
    let root_dentry = root_fs.mount("/", None,MountFlags::empty(),Some(BLOCK_DEVICE.get().unwrap().clone()));
    ROOT_DENTRY.call_once(|| root_dentry.unwrap());
}

pub fn get_root_dentry() -> Arc<dyn Dentry> {
    ROOT_DENTRY.get().unwrap().clone()
}