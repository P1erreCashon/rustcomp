use core::num::NonZero;
use lazy_static::*;
use sync::Mutex;
use alloc::{
    vec::Vec,
    collections::BTreeMap,
    string::String,
    sync::Arc,
};
use lru::LruCache;
use super::Dentry;
const DENTRY_LRU_SIZE:usize = 50;//must be bigger than max dir size,or getdents will be wrong
pub struct DentryCache{
    intenal_list:BTreeMap<(usize,String),Arc<dyn Dentry>>,
    leaf_list:LruCache<(usize,String),Arc<dyn Dentry>>,
}
pub struct DropList{
    drop_list:Vec<Arc<dyn Dentry>>,
}
impl DropList{
    pub fn new()->Self{
        Self{
            drop_list:Vec::new()
        }
    }
    pub fn empty_droplist(&mut self){
        while !self.drop_list.is_empty(){
            self.drop_list.pop();
        }
    }
}
impl DentryCache {
    pub fn new()->Self{
        Self { 
            intenal_list:BTreeMap::new() , 
            leaf_list: LruCache::new(NonZero::new(DENTRY_LRU_SIZE).unwrap()) 
        }
    }
    fn leaf_to_intenal(&mut self,dentry:&Arc<dyn Dentry>){
        let parent_pt = DentryCache::get_parent_pt(dentry.get_father().as_ref());
        let name = dentry.get_name_string();
        if let Some(r) = self.leaf_list.pop(&(parent_pt as usize,name.clone())){
            self.intenal_list.insert((parent_pt as usize,name), r);
        }
    }
    fn intenal_to_leaf(&mut self,dentry:&Arc<dyn Dentry>){
        let parent_pt = DentryCache::get_parent_pt(dentry.get_father().as_ref());
        let name = dentry.get_name_string();
        if let Some(r) = self.intenal_list.remove(&(parent_pt as usize,name.clone())){
            self.leaf_list.push((parent_pt as usize,name), r);
        }
    }
    fn get_parent_pt(parent:Option<&Arc<dyn Dentry>>)->*const Arc<dyn Dentry>{
        let parent_pt:*const Arc<dyn Dentry>;
        if parent.is_none(){
            parent_pt = core::ptr::null();
        }
        else{
            let parent = parent.unwrap();
            parent_pt = &*parent;
        }
        return parent_pt;
    }
    pub fn lookup(&mut self,parent:Option<&Arc<dyn Dentry>>,name:&str)->Option<Arc<dyn Dentry>>{
        let parent_pt = DentryCache::get_parent_pt(parent);
        let name = String::from(name);
        if let Some(child) = self.intenal_list.get(&(parent_pt as usize,name.clone())){
            return Some(child.clone());
        }
        if let Some(child) = self.leaf_list.get(&(parent_pt as usize,name)){
            return Some(child.clone());
        }
        return None;
    }
    pub fn add(&mut self,parent:Option<&Arc<dyn Dentry>>,name:&str,child:Arc<dyn Dentry>){
        if parent.is_some(){
            self.leaf_to_intenal(parent.clone().unwrap());
        }
        if let Some(_r) = self.lookup(parent, name){
            return;
        }
        let parent_pt = DentryCache::get_parent_pt(parent);
        let name = String::from(name);
        if let Some(((_f,_n),old)) = self.leaf_list.push((parent_pt as usize,name),child){
            DROPLIST.lock().drop_list.push(old);
        }
    }
}

lazy_static! {
    /// The global block cache manager
    pub static ref DENTRY_CACHE_MANAGER: Mutex<DentryCache> =
        Mutex::new(DentryCache::new());
}

lazy_static! {
    /// The global block cache manager
    pub static ref DROPLIST: Mutex<DropList> =
        Mutex::new(DropList::new());
}

/// Get the block cache corresponding to the given block id and block device
pub fn alloc_dentry(
    parent:Option<&Arc<dyn Dentry>>,name:&str,child:Arc<dyn Dentry>
){
    DENTRY_CACHE_MANAGER
        .lock()
        .add(parent, name, child);
}
///
pub fn intenal_to_leaf(dentry:&Arc<dyn Dentry>){
    DENTRY_CACHE_MANAGER
        .lock()
        .intenal_to_leaf(dentry);
}
///
pub fn dcache_lookup(parent:Option<&Arc<dyn Dentry>>,name:&str)->Option<Arc<dyn Dentry>>{
    DENTRY_CACHE_MANAGER
    .lock()
    .lookup(parent, name)
}
///
pub fn dcache_drop(){
    DROPLIST
    .lock()
    .empty_droplist();
}