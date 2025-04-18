use alloc::{string::{String,ToString}, vec::Vec,sync::Arc};
use alloc::collections::BTreeMap;
use device::BlockDevice;
use system_result::{SysError, SysResult};
use super::Dentry;
use super::SuperBlock;
use sync::Mutex;
///
bitflags::bitflags! {
    ///
    #[derive(Debug)]
    pub struct MountFlags:u32 {
}
}

///
pub struct FileSystemTypeInner{
    ///
    pub name:String,
    ///
    pub superblocks:Mutex<BTreeMap<String, Arc<dyn SuperBlock>>>,
}

impl  FileSystemTypeInner{

    ///
    pub fn new(name:String)->Self{
        Self{
            name,
            superblocks:Mutex::new(BTreeMap::new())
        }
    }

}

///
pub trait FileSystemType: Send + Sync  {    
    ///
    fn get_inner(&self)->&FileSystemTypeInner;
    ///
    fn mount(self:Arc<Self>,
        name:&str,
        parent:Option<Arc<dyn Dentry>>,
        _flags: MountFlags,
        device:Option<Arc<dyn BlockDevice>>)->SysResult<Arc<dyn Dentry>>{
            unimplemented!()
    }
    ///
    fn umount(self:Arc<Self>,
        path:&str,
        _flags:MountFlags
    )->SysResult<()>;
    ///
    fn add_superblock(&self, abs_mount_path: &str, superblock: Arc<dyn SuperBlock>) {
        self.get_inner()
            .superblocks
            .lock()
            .insert(abs_mount_path.to_string(), superblock);
    }
    ///
    fn remove_superblock(&self,abs_mount_path: &str)->SysResult<()>{
        if let Some(_sb) = self.get_inner()
            .superblocks
            .lock()
            .remove(abs_mount_path){
            Ok(())
        }
        else{
            Err(SysError::ENOENT)
        }
        
    }
}
