#![no_std]
#![no_main]
pub mod devfs;
mod procfs;
mod memfs;
mod tmpfs;
//mod fdtable;
extern crate alloc;
use alloc::{collections::BTreeMap, string::{String, ToString}, sync::Arc,vec::Vec};
use system_result::{SysResult,SysError};
use easy_fs::EfsFsType;
use ext4::Ext4ImplFsType;
use devfs::{DevFsType,init_devfs};
use procfs::{ProcFsType,init_procfs};
use tmpfs::TmpFsType;
use lazy_static::lazy_static;
use sync::{Mutex,Once};
use vfs_defs::{FileSystemType, MountFlags,Dentry};
use device::BLOCK_DEVICE;
pub use ext4::BLOCK_SIZE;
use memfs::{MemFile,MemInode,MemDentry};
pub use devfs::add_tty;

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
    let _ = file_systems.register_fs("tmpfs".to_string(), TmpFsType::new());
    let _ = file_systems.register_fs("procfs".to_string(), ProcFsType::new());
    let _ = file_systems.register_fs("devfs".to_string(), DevFsType::new());
}

pub fn init(){
    register_all_fs();
    let root_fs = FILE_SYSTEMS.lock().file_systems.get(ROOT_FS).unwrap().clone();
    let root_dentry = root_fs.mount("/", None,MountFlags::empty(),Some(BLOCK_DEVICE.get().unwrap().clone())).unwrap();
    
    let dev_fs = FILE_SYSTEMS.lock().file_systems.get("devfs").unwrap().clone();
    let dev_dentry = dev_fs.mount("dev", Some(root_dentry.clone()), MountFlags::empty(), None).unwrap();
    init_devfs(dev_dentry);
    
    let proc_fs = FILE_SYSTEMS.lock().file_systems.get("procfs").unwrap().clone();
    let proc_dentry = proc_fs.mount("proc", Some(root_dentry.clone()), MountFlags::empty(), None).unwrap();
    init_procfs(proc_dentry);

    let tmp_fs = FILE_SYSTEMS.lock().file_systems.get("tmpfs").unwrap().clone();
    let _dev_dentry = tmp_fs.mount("tmp", Some(root_dentry.clone()), MountFlags::empty(), None).unwrap();

    ROOT_DENTRY.call_once(|| root_dentry);

}

pub fn get_root_dentry() -> Arc<dyn Dentry> {
    ROOT_DENTRY.get().unwrap().clone()
}

pub struct VfsDentryHolder{
    dentry:Vec<Arc<dyn Dentry>>
}

lazy_static!{
    pub static ref VFS_DENTRY:Mutex<VfsDentryHolder> =
    Mutex::new(VfsDentryHolder{dentry:Vec::new()});
}

pub fn add_vfs_dentry(dent:Arc<dyn Dentry>){
    VFS_DENTRY.lock().dentry.push(dent);
}