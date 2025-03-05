use vfs_defs::{Dentry, DentryInner, DiskInodeType, File, FileInner, Inode, OpenFlags,DentryState};
use system_result::{SysResult,SysError};
use alloc::sync::Arc;
use crate::EfsFile;

use super::EfsInode;
use alloc::string::String;
pub struct EfsDentry{
    inner:DentryInner,
}

impl EfsDentry{
    pub fn new(inner:DentryInner)->Self{
        Self { inner }
    }
}

impl Dentry for EfsDentry{    
    fn ls(self:Arc<Self>)->alloc::vec::Vec<String> {
        let inode = self.get_inode().unwrap().downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR).unwrap();
        let child_dir_names = inode.ls();
        self.get_inner().children.lock().clear();
        for  child_dir_name in child_dir_names.iter(){
            let sub_child_dir = self.clone().concrete_new_child(child_dir_name.as_str());
            self.add_child(sub_child_dir);
        }
        return child_dir_names;
    }
    fn get_inner(&self) -> &DentryInner {
        &self.inner
    }
    fn concrete_create(self:Arc<Self>, name: &str, _type:DiskInodeType) -> SysResult<Arc<dyn Dentry>> {
        let child_dir = self.get_child(name).unwrap();
        let inode = self.get_inode()?.downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR)?;
        if inode.find(name).is_some(){
            return Err(SysError::EEXIST);
        }
        if let Some(child_inode) = inode.create(name, _type) {
            child_dir.set_inode(child_inode);
            return Ok(child_dir);
        }
        else{
            return Err(SysError::ENOSPC);
        }
    }

    fn concrete_lookup(self: Arc<Self>, name: &str) -> SysResult<Arc<dyn Dentry>> {
        let inode = self.get_inode()?.downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR)?;
        if let Some(child_inode) = inode.find(name){
            let child_dir = self.get_child(name).unwrap();
            let efs_child_inode = child_inode.downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR)?;
            efs_child_inode.load_from_disk();
            if(efs_child_inode.is_dir()){
                let sub_child_dir_names = efs_child_inode.ls();
                for  sub_child_dir_name in sub_child_dir_names{
                    let sub_child_dir = child_dir.clone().concrete_new_child(sub_child_dir_name.as_str());
                    child_dir.add_child(sub_child_dir);
                }
            }
            child_dir.set_inode(efs_child_inode);
            Ok(child_dir)
        }
        else{
            return Err(SysError::ENOTDIR);
        }
    }

    fn concrete_new_child(self:Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        let dyn_dentry:Arc<dyn Dentry> = self.clone();
        let child_dir = Arc::new(EfsDentry::new(DentryInner::new(String::from(_name), self.get_superblock(),Some(Arc::downgrade(&dyn_dentry)))));
        return child_dir;
    }
    fn concrete_link(self: Arc<Self>, new: &Arc<dyn Dentry>) -> SysResult<()> {
        let inode = new.get_father().unwrap().get_inode()?.downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR)?;
        if inode.find(new.get_name_str()).is_some(){
            return Err(SysError::EEXIST);
        }
        if inode.link(new.get_name_str(), self.get_inode().unwrap().get_meta().ino) == 0 {
            new.set_inode(self.get_inode()?);
            return Ok(());
        }
        return Err(SysError::EEXIST);
    }
    //self is old's father dentry
    fn concrete_unlink(self: Arc<Self>, old: &Arc<dyn Dentry>) -> SysResult<()> {
        let inode = self.get_inode()?.downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR)?;
        if inode .find(old.get_name_str()).is_none(){
            return Err(SysError::ENOENT);
        }
        inode.unlink(old.get_name_str());
        self.get_inner().children.lock().remove(&old.get_name_string());
        Ok(())
    }
    fn open(self:Arc<Self>,flags:OpenFlags)->Arc<dyn File> {
        let (readable,writable) = flags.read_write();
        Arc::new(EfsFile::new(readable, writable, FileInner::new(self)))
    }

}

impl Drop for EfsDentry{
    fn drop(&mut self) {
        self.get_inner().children.lock().clear();
    }
}
