use spin::{Mutex, MutexGuard};
use alloc::sync::{Weak,Arc};
use downcast_rs::{impl_downcast, DowncastSync};
use system_result::SysResult;
use crate::Kstat;

use super::SuperBlock;
/// Type of a disk inode
#[derive(Clone, Copy)]
#[derive(PartialEq)]
pub enum DiskInodeType {
    ///
    File,
    ///
    Directory,
    ///
    None,
}
///
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum InodeState {
    ///
    #[default]
    Invalid,
    ///
    Valid,
    ///
    Dirty,
}
///
pub struct InodeMeta {
    /// Inode number.
    pub ino: usize,
    ///
    pub superblock: Weak<dyn SuperBlock>,
    ///
    pub inner: Mutex<InodeMetaInner>,
    ///
    pub state:Mutex<InodeState>,
    ///
    pub _type:Mutex<DiskInodeType>
}
///
pub struct InodeMetaInner {
    /// 
    pub size: u32,
    ///
    pub link: u32,    

}
impl InodeMetaInner{
    ///
    pub fn new()->Self{
        Self{
            size:0,
            link:0,
        }
    }
}

impl InodeMeta {
    ///
    pub fn new(ino:usize,superblock:Arc<dyn SuperBlock>)->Self{
        Self{
            ino,
            superblock:Arc::downgrade(&superblock),
            inner:Mutex::new(InodeMetaInner::new()),
            state:Mutex::new(InodeState::Invalid),
            _type:Mutex::new(DiskInodeType::None),
        }
    }
}
///
pub trait Inode: Send + Sync+ DowncastSync {
    ///inode state must be invalid and hold the state lock
    fn load_from_disk(&self);
    ///
    fn get_meta(&self) -> &InodeMeta;
    ///
    fn get_attr(&self)->SysResult<Kstat>;
    ///
    fn get_size(&self) -> u32; //{//这要改
  //      self.get_meta().inner.lock().size
  //  }
    ///
    fn set_size(&self, size: u32) {//这要改
        self.get_meta().inner.lock().size = size;
    }
    ///
    fn get_state(&self)->MutexGuard<InodeState>{
        self.get_meta().state.lock()
    }    
    ///
    fn set_type(&self,_type:DiskInodeType){
        *self.get_meta()._type.lock() = _type;
    }       
    ///
    fn is_dir(&self) -> bool{
        let ty = *self.get_meta()._type.lock();
        if ty == DiskInodeType::None{
            panic!("is_dir:a None file");
        }
        ty == DiskInodeType::Directory
    }    
    /// Whether this inode is a file,must hold the lock
    #[allow(unused)]
    fn is_file(&self) -> bool {
        let ty = *self.get_meta()._type.lock();
        if ty == DiskInodeType::None{
            panic!("is_dir:a None file");
        }
        ty == DiskInodeType::File
    }

    ///
    fn clear(&self);
}
impl dyn Inode{


}

impl_downcast!(sync Inode);