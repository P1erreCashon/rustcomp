use alloc::{
    vec::Vec,
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
};
use crate::{inode::{DiskInodeType, Inode}, superblock, SuperBlock};
use spin::{Mutex,MutexGuard};
use system_result::{SysError,SysResult};
use super::{File,OpenFlags};

///
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub enum DentryState {
    ///
    #[default]
    Invalid,
    ///
    Valid,
    ///
    Dirty,
}
///
pub struct DentryInner {
    ///
    pub name: String,
    ///
    pub superblock: Weak<dyn SuperBlock>,
    ///
    pub father: Option<Weak<dyn Dentry>>,
    ///
    pub inode: Mutex<Option<Arc<dyn Inode>>>,
    ///
    pub children: Mutex<BTreeMap<String, Arc<dyn Dentry>>>,
    ///
    pub state: Mutex<DentryState>,
}

impl DentryInner{
    ///
    pub fn new(name:String,superblock:Arc<dyn SuperBlock>,father: Option<Weak<dyn Dentry>>)->Self{
        Self{
            name,
            superblock:Arc::downgrade(&superblock),
            father,
            inode:Mutex::new(None),
            children:Mutex::new(BTreeMap::new()),
            state:Mutex::new(DentryState::Invalid)
        }
    }
    ///
    pub fn set_inode(&mut self,inode:Arc<dyn Inode>){
        *self.inode.lock() = Some(inode);
    }
}
///
pub trait Dentry: Send + Sync {
    ///
    fn get_inner(&self) -> &DentryInner;
    /// If the dentry itself has a negative child with `name`, it will create an
    /// inode for the negative child and return the child.
    fn concrete_create(self: Arc<Self>, name: &str, _type:DiskInodeType) -> SysResult<Arc<dyn Dentry>>;
    /// Look up in a directory inode and find file with `name` from disk.
    fn concrete_lookup(self: Arc<Self>, name: &str) -> SysResult<Arc<dyn Dentry>>;
    /// Create a negetive child dentry with `name`.
    fn concrete_new_child(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry>;
    ///
    fn concrete_link(self: Arc<Self>, new: &Arc<dyn Dentry>) -> SysResult<()>;
    ///
    fn concrete_unlink(self: Arc<Self>, old: &Arc<dyn Dentry>) -> SysResult<()>;
    /// get a clone of self inode
    fn get_inode(&self) -> SysResult<Arc<dyn Inode>> {
        self.get_inner()
            .inode
            .lock()
            .as_ref()
            .ok_or(SysError::ENOENT)
            .cloned()
    }
    ///
    fn set_inode(&self, inode: Arc<dyn Inode>) {
        if self.get_inner().inode.lock().is_some() {
        //    log::warn!("[Dentry::set_inode] replace inode in {:?}", self.name());
        }
        *self.get_inner().inode.lock() = Some(inode);
    }
    ///
    fn get_superblock(&self) -> Arc<dyn SuperBlock> {
        self.get_inner().superblock.upgrade().unwrap()
    }
    ///
    fn get_name_string(&self) -> String {
        self.get_inner().name.clone()
    }
    ///
    fn get_name_str(&self) -> &str {
        &self.get_inner().name
    }
    ///
    fn get_child(&self, name: &str) -> Option<Arc<dyn Dentry>> {
        self.get_inner().children.lock().get(name).cloned()

    }
    /// Insert a child dentry to this dentry.
    fn add_child(&self, child: Arc<dyn Dentry>) -> Option<Arc<dyn Dentry>> {
        self.get_inner()
            .children
            .lock()
            .insert(child.get_name_string(), child)
    }
    ///
    fn get_state(&self)->MutexGuard<DentryState>{
        self.get_inner().state.lock()
    }
    ///
    fn get_father(&self) -> Option<Arc<dyn Dentry>> {
        self.get_inner().father.as_ref().map(|p| p.upgrade().unwrap())
    }
    /// Get the path of this dentry.
    fn path(&self) -> String {
        if let Some(p) = self.get_father() {
            let p_path = p.path();
            if p_path == "/" {
                p_path + self.get_name_str()
            } else {
                p_path + "/" + self.get_name_str()
            }
        } else {
            String::from("/")
        }
    }
    ///
    fn open(self:Arc<Self>,flags:OpenFlags)->Arc<dyn File>;    
    ///
    fn is_dir(&self)->bool{
        self.get_inode().unwrap().is_dir()
    }
    ///
    fn is_file(&self)->bool{
        self.get_inode().unwrap().is_file()
    } 
    ///
    fn ls(self:Arc<Self>)->Vec<String>;
    
}

impl dyn Dentry{    
    ///
    pub fn has_no_inode(&self) -> bool {
        self.get_inner().inode.lock().is_none()
    }                                     
    /// Find exist dentry(with inode) under current dentry by name from disk ,load it to memory,get a clone of dentry's Arc
    pub fn lookup(self: &Arc<Self>, name: &str) -> SysResult<Arc<dyn Dentry>> {
        if !self.get_inode()?.is_dir() {
            return Err(SysError::ENOTDIR);
        }
        if let Some(child) = self.get_child(name){
            let mut state = child.get_state();
            if *state == DentryState::Invalid {
                self.clone().concrete_lookup(name)?;
                *state = DentryState::Valid;
                drop(state);
                return Ok(child);
            }
            drop(state);
            return Ok(child);
        }
        else{
            return Err(SysError::ENOENT);
        }

    }
    /// Create a negetive child dentry with `name`.
    pub fn new_child(self: &Arc<Self>, name: &str) -> Arc<dyn Dentry> {
        let child = self.clone().concrete_new_child(name);
        child
    }
    /// Find dentry under current dentry by name,if not found,create it (without inode) ,get a clone of dentry's Arc
    pub fn find_or_create(self: &Arc<Self>, name: &str,_type:DiskInodeType) -> Arc<dyn Dentry> {
        self.get_child(name).unwrap_or_else(|| {
            let new_dentry = self.new_child(name);
            self.add_child(new_dentry.clone());
            let mut state = self.get_state();
            *state = DentryState::Dirty;
            drop(state);
            new_dentry
        })
    }
    /// Create a dirent(with inode) under current dirent by name
    pub fn create(self: &Arc<Self>, name: &str, _type:DiskInodeType) -> SysResult<Arc<dyn Dentry>> {
        if !self.get_inode()?.is_dir() {
            return Err(SysError::ENOTDIR);
        }
        let child = self.find_or_create(name,_type);
        if child.has_no_inode() {
            self.clone().concrete_create(name, _type)?;
        }
        let mut state = self.get_state();
        *state = DentryState::Dirty;
        drop(state);
        Ok(child)
    }
    ///link self inode to new dentry
    pub fn link(self: &Arc<Self>, new: &Arc<dyn Dentry>) -> SysResult<()> {
        if self.has_no_inode() {
            Err(SysError::ENOENT)
        } else if !new.has_no_inode() {
            Err(SysError::EEXIST)
        } else {
            let ret = self.clone().concrete_link(new);
            self.get_inode()?.get_meta().inner.lock().link += 1;
            ret
        }
    }
    ///
    pub fn unlink(self: &Arc<Self>, old: &Arc<dyn Dentry>) -> SysResult<()> {
        if self.has_no_inode() {
            Err(SysError::ENOENT)
        } else if old.has_no_inode() {
            Err(SysError::ENOENT)
        } else {
            old.get_inode()?.get_meta().inner.lock().link -= 1;
            let ret = self.clone().concrete_unlink(old);
            ret
        }
    }
}