use vfs_defs::{Dentry, DentryInner, DentryState, DiskInodeType, File, FileInner, Inode, InodeMode, OpenFlags, RenameFlags, SuperBlock};
use super::{MemFile,MemInode,add_vfs_dentry};
use alloc::sync::Arc;
use system_result::{SysError,SysResult};
use alloc::string::String;

pub struct MemDentry{
    inner:DentryInner
}

impl MemDentry{
    pub fn new(name:&str,superblock:Arc<dyn SuperBlock>,father:Option<Arc<dyn Dentry>>)->Arc<Self>{
        Arc::new(Self{
            inner:DentryInner::new(String::from(name), superblock, father)
        })
    }
}

impl Dentry for MemDentry{
    fn get_inner(&self) -> &DentryInner {
        &self.inner
    }
    fn open(self:Arc<Self>,flags:OpenFlags)->Arc<dyn File> {
        let ret = Arc::new(MemFile::new(FileInner::new(self)));
        *ret.get_inner().flags.lock() = flags;
        ret
    }
    fn concrete_lookup(self: Arc<Self>, _name: &str) -> SysResult<Arc<dyn Dentry>> {
        Err(SysError::ENOENT)
    }
    fn concrete_create(self: Arc<Self>, name: &str, _type:DiskInodeType) -> SysResult<Arc<dyn Dentry>> {
        let child = self.clone().get_child(name);
        if child.is_none(){
            return Err(SysError::ENOENT);
        }
        let child = child.unwrap();
        let inode = MemInode::new(InodeMode::from_type(_type), self.get_superblock().clone());
        inode.set_type(_type);
        child.set_inode(inode);
        *child.get_state() = DentryState::Valid;
        add_vfs_dentry(child.clone());
        Ok(child)
    }
    fn concrete_unlink(self: Arc<Self>, old: &Arc<dyn Dentry>) -> SysResult<()> {
        self.get_inner().children.lock().remove(old.get_name_str()).ok_or(SysError::ENOENT).map(|_| ())
    }
    fn concrete_new_child(self: Arc<Self>, name: &str) -> Arc<dyn Dentry> {
        Self::new(name, self.get_superblock(), Some(self))
    }
    fn concrete_link(self: Arc<Self>, _new: &Arc<dyn Dentry>) -> SysResult<()> {
        unimplemented!()
    }
    fn concrete_rename(self: Arc<Self>, _new: Arc<dyn Dentry>, _flags: RenameFlags) -> SysResult<()> {
        unimplemented!()
    }
    fn concrete_getchild(self:Arc<Self>, _name: &str) -> Option<Arc<dyn Dentry>> {
        None
    }
    fn load_dir(self:Arc<Self>)->SysResult<()> {
        Ok(())
    }
    fn self_arc(self:Arc<Self>) -> Arc<dyn Dentry> {
        self.clone()
    }
}