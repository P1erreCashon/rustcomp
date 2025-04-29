use vfs_defs::{Dentry, DentryInner, DentryState, DiskInodeType, File, FileInner, Inode, InodeMeta, OpenFlags,RenameFlags,alloc_dentry,InodeMode};
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::{String,ToString};
use system_result::{SysResult,SysError};
use crate::superblock::Ext4Superblock;
use crate::file::Ext4ImplFile;
use ext4_rs::*;

use super::Ext4Inode;
const MODULE_LEVEL:log::Level = log::Level::Debug;
pub const EXT_MAX_BLOCKS: u32 = u32::MAX;
pub struct Ext4Dentry{
    inner:DentryInner
}

impl Ext4Dentry{
    pub fn new(inner:DentryInner)->Self{
        Self{inner}
    }
}

impl Dentry for Ext4Dentry{
    fn get_inner(&self) -> &DentryInner {
        &self.inner
    }
    fn self_arc(self:Arc<Self>) -> Arc<dyn Dentry> {
        self.clone()
    }
    fn concrete_create(self: Arc<Self>, name: &str, _type:DiskInodeType) -> SysResult<Arc<dyn Dentry>> {
        let sblock = self.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT)?;
        let child_dir = self.get_child(name).unwrap();
        let path = child_dir.path();
        let child_ino;
        match _type{
            DiskInodeType::File=>{
                //child_ino = sblock.ext4fs.ext4_file_open( path.as_str(), "w+");
                child_ino = sblock.ext4fs.generic_open( path.as_str(),&mut 2,true,InodeFileType::S_IFREG.bits(),&mut 0 );
            }
            DiskInodeType::Directory=>{
                child_ino = sblock.ext4fs.ext4_dir_mk(path.as_str());
            }
            DiskInodeType::None=>{
                return Err(SysError::EISDIR);
            }
        }
        let child_inode = Ext4Inode::new(InodeMeta::new(InodeMode::from_type(_type),child_ino.unwrap() as usize, sblock),);
        child_inode.set_type(_type);
        child_dir.set_inode(Arc::new(child_inode));
        *child_dir.get_state() = DentryState::Valid;
        Ok(child_dir)
    }
    fn concrete_link(self: Arc<Self>, new: &Arc<dyn Dentry>) -> SysResult<()> {
        let sblock = self.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT)?;
        let ino = self.get_inode().unwrap().get_meta().ino as u64;
        let newparent = new.get_father().unwrap().get_inode()?.get_meta().ino as u64;
    //    log_debug!("link path:{} parent ino:{}",new.path(),newparent);
        let mut parent_inode_ref = sblock.ext4fs.get_inode_ref(newparent as u32);
        let mut child_inode_ref = sblock.ext4fs.get_inode_ref(ino as u32);
      //  sblock.ext4fs.fuse_link(ino, newparent, newname)
        let r = sblock.ext4fs.link(&mut parent_inode_ref, &mut child_inode_ref, new.get_name_str());
        if let Err(e) = r {
            log_debug!("link err:{:?}",r);
            return match e.error() {   
                Errno::ENOENT => Err(SysError::ENOENT),
                Errno::EEXIST => Err(SysError::EEXIST),
                Errno::EINVAL => Err(SysError::EINVAL),
                _ => Err(SysError::EINVAL),
            }
        } 
        Ok(())
    }
    fn load_dir(self:Arc<Self>)->SysResult<()> {
        let sblock = self.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT)?;
        let entries = sblock.ext4fs.dir_get_entries(self.get_inode().unwrap().get_meta().ino as u32);
        for entry in entries{ 
            let name_bytes = &entry.name[..entry.name_len as usize]; // 取前 name_len 个字节
            let child_dir_name = String::from_utf8_lossy(name_bytes).to_string();
            if child_dir_name == "." || child_dir_name == ".."{
                continue;
            }
            if let Some(child) = self.clone().get_child(child_dir_name.as_str()){
                let mut state = child.get_state();
                if *state == DentryState::Invalid {
                    self.clone().concrete_lookup(child_dir_name.as_str())?;
                    *state = DentryState::Valid;
                    drop(state);
                }
                else{drop(state);}
            }
        }
        Ok(())
    }
    fn concrete_lookup(self: Arc<Self>, name: &str) -> SysResult<Arc<dyn Dentry>> {
        let sblock = self.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT)?;
        let mut ino = self.get_inode().unwrap().get_meta().ino as u32;
        let child = self.get_child(name).unwrap();
        let path = child.path();
        let mut r;
        r = sblock.ext4fs.ext4_dir_open(path.as_str());
        if let Err(_e) = r {
            r = sblock.ext4fs.generic_open(path.as_str(), &mut ino, false, 0, &mut 0);
            if let Err(e) = r {
                return match e.error() {
                    Errno::ENOENT => Err(SysError::ENOENT),
                    Errno::EINVAL => Err(SysError::EINVAL),
                    _ => Err(SysError::EINVAL),
                }
            }
        }            
        let inode_ref = sblock.ext4fs.get_inode_ref(r.unwrap());
        if inode_ref.inode.is_file(){             
            let child_inode = Arc::new(Ext4Inode::new(InodeMeta::new(InodeMode::FILE,r.unwrap() as usize, sblock)));
            child_inode.set_type(DiskInodeType::File);
            child.set_inode(child_inode);
        } 
        else{
            let child_inode = Arc::new(Ext4Inode::new(InodeMeta::new(InodeMode::DIR,r.unwrap() as usize, sblock)));
            child_inode.set_type(DiskInodeType::Directory);
            child.set_inode(child_inode);            
        }
        Ok(child)

    }
    fn concrete_new_child(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        let dyn_dentry:Arc<dyn Dentry> = self.clone();
        let child_dir = Arc::new(Ext4Dentry::new(DentryInner::new(String::from(_name), self.get_superblock(),Some(dyn_dentry.clone()))));
        alloc_dentry(Some(&dyn_dentry), _name, child_dir.clone());
        return child_dir;
    }
    fn concrete_unlink(self: Arc<Self>, old: &Arc<dyn Dentry>) -> SysResult<()> {
        let sblock = self.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT)?;
        let inode_num = self.get_inode()?.downcast_arc::<Ext4Inode>().map_err(|_| SysError::ENOENT)?.get_meta().ino as u32;
        let child_ino = old.get_inode()?.downcast_arc::<Ext4Inode>().map_err(|_| SysError::ENOENT)?.get_meta().ino as u32;
        let mut inode_ref = sblock.ext4fs.get_inode_ref(child_ino);
        if inode_ref.inode.is_file(){ 
            let child_link_cnt = inode_ref.inode.links_count();
            if child_link_cnt == 1 {
                let old_size = inode_ref.inode.size();
        
                if old_size != 0 {
                    let block_size = BLOCK_SIZE as u64;
                    let new_blocks_cnt = ((0 + block_size - 1) / block_size) as u32;
                    let old_blocks_cnt = ((old_size + block_size - 1) / block_size) as u32;
                    let diff_blocks_cnt = old_blocks_cnt - new_blocks_cnt;
        
                    if diff_blocks_cnt > 0{
                        let _ = sblock.ext4fs.extent_remove_space(&mut inode_ref, new_blocks_cnt, EXT_MAX_BLOCKS);
                    }
        
                    inode_ref.inode.set_size(0);
                    sblock.ext4fs.write_back_inode(&mut inode_ref); 
                }
            }

            // load parent
            let mut parent_inode_ref = sblock.ext4fs.get_inode_ref(inode_num);

            let r = sblock.ext4fs.unlink(
                &mut parent_inode_ref,
                &mut inode_ref,
                old.get_name_str(),
            );

            if r.is_err(){
                return Err(SysError::ENOENT);
            }
        }
        else{
            let _ = sblock.ext4fs.dir_remove(inode_num, old.get_name_str());
        }
        self.get_inner().children.lock().remove(&old.get_name_string());
        Ok(())
    }
    fn concrete_rename(self: Arc<Self>, new: Arc<dyn Dentry>, flags: RenameFlags) -> SysResult<()> {
        let sblock = self.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT)?;
        let old_type = *self.get_inode()?.get_meta()._type.lock();
        if !new.has_no_inode() {
            let new_type = *new.get_inode()?.get_meta()._type.lock();
            if new_type != old_type {
                return match (old_type, new_type) {
                    (DiskInodeType::File, DiskInodeType::Directory) => Err(SysError::EISDIR),
                    (DiskInodeType::Directory, DiskInodeType::File) => Err(SysError::ENOTDIR),
                    _ => unimplemented!(),
                };
            }
            match new_type {
                DiskInodeType::Directory => {
                    let parent = new.get_father().unwrap().get_inode()?.get_meta().ino;
                    let _ = sblock.ext4fs.dir_remove(parent as u32, new.path().as_str());
                },
                DiskInodeType::File => {let _ = sblock.ext4fs.file_remove(new.path().as_str());},
                _ => todo!(),
            };
        }
        self.clone().concrete_link(&new)?;
        new.set_inode(self.get_inode()?);
        if flags.contains(RenameFlags::RENAME_EXCHANGE) {
            self.set_inode(new.get_inode()?);
        } else {
            *self.inner.inode.lock() = None;
        }
        Ok(())
    }
    fn concrete_getchild(self:Arc<Self>, name: &str) -> Option<Arc<dyn Dentry>> {
        let sblock = self.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT).unwrap();
        let entries = sblock.ext4fs.dir_get_entries(self.get_inode().unwrap().get_meta().ino as u32);
        for entry in entries{ 
            let name_bytes = &entry.name[..entry.name_len as usize]; // 取前 name_len 个字节
            let child_dir_name = String::from_utf8_lossy(name_bytes).to_string();
            if child_dir_name == name{
                let child = self.clone().concrete_new_child(name);
                return Some(child);
            }
        }
        None
    }
    fn open(self:Arc<Self>,flags:OpenFlags)->Arc<dyn File> {
        let len = self.get_inode().unwrap().get_size();
        let file = Arc::new(Ext4ImplFile::new(FileInner::new(self)));        
        if flags.contains(OpenFlags::APPEND){
            *file.get_offset() = len as usize;
        }
        *file.get_inner().flags.lock() = flags;
        file
    }
    /*
    fn ls(self:Arc<Self>)->Vec<String> {
        let sblock = self.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT).unwrap();
        let entries = sblock.ext4fs.dir_get_entries(self.get_inode().unwrap().get_meta().ino as u32);
        let mut names = Vec::new();
        for entry in entries{ 
            let name_bytes = &entry.name[..entry.name_len as usize]; // 取前 name_len 个字节
            let child_dir_name = String::from_utf8_lossy(name_bytes).to_string();
            let sub_child_dir = self.clone().concrete_new_child(child_dir_name.as_str());
            self.add_child(sub_child_dir);
            names.push(child_dir_name);
        }
        names
    } */
}

impl Drop for Ext4Dentry{
    fn drop(&mut self) {
        self.on_drop();
    }
}