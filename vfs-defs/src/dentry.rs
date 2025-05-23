use alloc::{
    collections::BTreeMap,
    string::String,
    sync::{Arc, Weak},
};
use crate::{inode::{DiskInodeType, Inode}, SuperBlock,intenal_to_leaf,dcache_lookup,dcache_drop};
use sync::{Mutex,MutexGuard};
use system_result::{SysError,SysResult};
use super::{File,OpenFlags,RenameFlags};
const MODULE_LEVEL:log::Level = log::Level::Debug;
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
    pub father: Option<Arc<dyn Dentry>>,
    ///
    pub inode: Mutex<Option<Arc<dyn Inode>>>,
    ///
    pub children: Mutex<BTreeMap<String, Weak<dyn Dentry>>>,
    ///
    pub state: Mutex<DentryState>,
}

impl DentryInner{
    ///
    pub fn new(name:String,superblock:Arc<dyn SuperBlock>,father: Option<Arc<dyn Dentry>>)->Self{
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
    ///    
    fn concrete_rename(self: Arc<Self>, new: Arc<dyn Dentry>, flags: RenameFlags) -> SysResult<()>;
    ///
    fn concrete_getchild(self:Arc<Self>, name: &str) -> Option<Arc<dyn Dentry>>;
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
    fn self_arc(self:Arc<Self>) -> Arc<dyn Dentry>;
    ///
    fn get_child(self:Arc<Self>, name: &str) -> Option<Arc<dyn Dentry>> {
        let mut children = self.get_inner().children.lock();
        if let Some(child) = children.get(name){
            return Some(child).as_ref().map(|p| p.upgrade().unwrap());
        }
        if let Some(child) = dcache_lookup(Some(&self.clone().self_arc()), name){
            children.insert(child.get_name_string(), Arc::downgrade(&child));
            drop(children);
            dcache_drop();
            return Some(child);
        }
        
        if let Some(child) = self.clone().concrete_getchild(name){
            children.insert(child.get_name_string(), Arc::downgrade(&child));            
            drop(children);
            dcache_drop();
            return Some(child);
        }
        drop(children);
        dcache_drop();
        return None;
    }
    /// Insert a child dentry to this dentry.
    fn add_child(&self, child: Arc<dyn Dentry>) -> Option<Weak<dyn Dentry>> {
        self.get_inner()
            .children
            .lock()
            .insert(child.get_name_string(), Arc::downgrade(&child))
    }
    ///
    fn get_state(&self)->MutexGuard<DentryState>{
        self.get_inner().state.lock()
    }
    ///
    fn get_father(&self) -> Option<Arc<dyn Dentry>> {
        self.get_inner().father.clone()
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
    /* 
    ///
    fn ls(self:Arc<Self>)->Vec<String>;*/
    ///
    fn load_dir(self:Arc<Self>)->SysResult<()>;
    ///
    fn on_drop(&self){
        if let Some(father) = self.get_father(){
            let mut children = father.get_inner().children.lock();
            children.remove(self.get_name_str());
            if children.len() == 0{
                intenal_to_leaf(&father);
            }
        }
    }
    
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
        if let Some(child) = self.clone().get_child(name){
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
            log_debug!("lookup:no child:{}",name);
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
        self.clone().get_child(name).unwrap_or_else(|| {
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
    ///
    pub fn vfs_rename(self: &Arc<Self>, new: &Arc<Self>,flags:RenameFlags)->SysResult<()>{
        if flags.contains(RenameFlags::RENAME_EXCHANGE) && (flags.contains(RenameFlags::RENAME_NOREPLACE) || flags.contains(RenameFlags::RENAME_WHITEOUT)){
            return Err(SysError::EINVAL);
        }
        if new.is_subdir(self) {
            return Err(SysError::EINVAL);
        }
        if new.has_no_inode() && flags.contains(RenameFlags::RENAME_EXCHANGE) {
            return Err(SysError::ENOENT);
        } else if flags.contains(RenameFlags::RENAME_NOREPLACE) {
            return Err(SysError::EEXIST);
        }
        self.clone().concrete_rename(new.clone(), flags)
    }
    ///
    pub fn is_subdir(self: &Arc<Self>, dir: &Arc<Self>) -> bool {
        let mut father = self.get_father();
        while let Some(parent) = father {
            if Arc::ptr_eq(self, dir) {
                return true;
            }
            father = parent.get_father();
        }
        false
    }
}