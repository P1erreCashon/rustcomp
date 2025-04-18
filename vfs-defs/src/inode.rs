use sync::{Mutex, MutexGuard};
use alloc::sync::{Weak,Arc};
use downcast_rs::{impl_downcast, DowncastSync};
use system_result::SysResult;
use time::*;
use crate::Kstat;

use super::SuperBlock;
/// Type of a disk inode
#[derive(Clone, Copy)]
#[derive(PartialEq)]
pub enum DiskInodeType {
    ///
    File = 0o10,
    ///
    Directory= 0o4,
    ///
    None=0,
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
bitflags::bitflags! {
    ///
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct InodeMode: u32 {
        /// Type.
        const TYPE_MASK = 0o170000;
        /// FIFO.
        const FIFO  = 0o010000;
        /// Character device.
        const CHAR  = 0o020000;
        /// Directory
        const DIR   = 0o040000;
        /// Block device
        const BLOCK = 0o060000;
        /// Regular file.
        const FILE  = 0o100000;
        /// Symbolic link.
        const LINK  = 0o120000;
        /// Socket
        const SOCKET = 0o140000;

        /// Set-user-ID on execution.
        const SET_UID = 0o4000;
        /// Set-group-ID on execution.
        const SET_GID = 0o2000;
        /// sticky bit
        const STICKY = 0o1000;
        /// Read, write, execute/search by owner.
        const OWNER_MASK = 0o700;
        /// Read permission, owner.
        const OWNER_READ = 0o400;
        /// Write permission, owner.
        const OWNER_WRITE = 0o200;
        /// Execute/search permission, owner.
        const OWNER_EXEC = 0o100;

        /// Read, write, execute/search by group.
        const GROUP_MASK = 0o70;
        /// Read permission, group.
        const GROUP_READ = 0o40;
        /// Write permission, group.
        const GROUP_WRITE = 0o20;
        /// Execute/search permission, group.
        const GROUP_EXEC = 0o10;

        /// Read, write, execute/search by others.
        const OTHER_MASK = 0o7;
        /// Read permission, others.
        const OTHER_READ = 0o4;
        /// Write permission, others.
        const OTHER_WRITE = 0o2;
        /// Execute/search permission, others.
        const OTHER_EXEC = 0o1;
    }
}

impl InodeMode {
    ///
    pub fn to_type(&self) -> DiskInodeType {
        (*self).into()
    }
    ///
    pub fn from_type(_type: DiskInodeType) -> Self {
        let perm_mode = InodeMode::OWNER_MASK | InodeMode::GROUP_MASK | InodeMode::OTHER_MASK;
        let file_mode = match _type {
            DiskInodeType::Directory => InodeMode::DIR,
            DiskInodeType::File => InodeMode::FILE,
            DiskInodeType::None => InodeMode::TYPE_MASK,
        };
        file_mode | perm_mode
    }
}
impl From<InodeMode> for DiskInodeType {
    fn from(mode: InodeMode) -> Self {
        match mode.intersection(InodeMode::TYPE_MASK) {
            InodeMode::DIR => DiskInodeType::Directory,
            InodeMode::FILE => DiskInodeType::File,
            _ => DiskInodeType::None,
        }
    }
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
    pub _type:Mutex<DiskInodeType>,
    ///
    pub mode:InodeMode,
}
///
pub struct InodeMetaInner {
    /// 
    pub size: u32,
    ///
    pub link: u32,   
    /// Last access time.
    pub atime: TimeSpec,
    /// Last modification time.
    pub mtime: TimeSpec,
    /// Last status change time.
    pub ctime: TimeSpec, 

}
impl InodeMetaInner{
    ///
    pub fn new()->Self{
        Self{
            size:0,
            link:0,
            atime:TimeSpec::default(),
            mtime:TimeSpec::default(),
            ctime:TimeSpec::default()
        }
    }
}

impl InodeMeta {
    ///
    pub fn new(mode:InodeMode,ino:usize,superblock:Arc<dyn SuperBlock>)->Self{
        Self{
            ino,
            superblock:Arc::downgrade(&superblock),
            inner:Mutex::new(InodeMetaInner::new()),
            state:Mutex::new(InodeState::Invalid),
            _type:Mutex::new(DiskInodeType::None),
            mode
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