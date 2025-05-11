use alloc::{string::String, vec::Vec};
use alloc::vec;
use alloc::sync::Arc;
use crate::fs::{Stdin, Stdout,StdioDentry,StdioInode,Stderr};
use vfs_defs::{Dentry, DentryInner, DentryState, File, FileInner, OpenFlags};
use vfs::devfs::{DevFsType,DevSuperBlock};
use config::{RLimit,MAX_FD};
use system_result::{SysError,SysResult};
use bitflags::bitflags;


bitflags::bitflags! {
    // Defined in <bits/fcntl-linux.h>.
    pub struct FdFlags: u8 {
        const CLOEXEC = 1;
    }
}

impl From<OpenFlags> for FdFlags {
    fn from(value: OpenFlags) -> Self {
        if value.contains(OpenFlags::CLOEXEC) {
            FdFlags::CLOEXEC
        } else {
            FdFlags::empty()
        }
    }
}
#[derive(Clone)]
pub struct Fd {
    flags: FdFlags,
    file: Arc<dyn File + Send + Sync>,
}
impl Fd{
    pub fn new(file:Arc<dyn File + Send + Sync>,flags:FdFlags)->Self{
        Self{
            flags,file
        }
    }
    pub fn file(&self)->Arc<dyn File>{
        self.file.clone()
    }
    pub fn get_flags(&self)->FdFlags{
        self.flags
    }
    pub fn set_flags(&mut self,flags:FdFlags){
        self.flags = flags;
    }
}

pub struct FdTable{
    pub fd_table: Vec<Option<Fd>>,
    pub fd_table_rlimit:RLimit,
}

fn new_devfssuper()->Arc<DevSuperBlock>{
    DevSuperBlock::new(None, DevFsType::new())
}

impl FdTable{
    pub fn new()->Self{
        let superblock = new_devfssuper();
        let stdininner = DentryInner::new(String::from("stdin"), superblock.clone(),None);
        let stdoutinner = DentryInner::new(String::from("stdout"), superblock.clone(),None);
        let stdioinode = Arc::new(StdioInode::new(vfs_defs::InodeMeta::new(vfs_defs::InodeMode::CHAR, vfs_defs::ino_alloc() as usize, superblock)));
        let stdindentry = StdioDentry::new(stdininner);
        let stdoutdentry = StdioDentry::new(stdoutinner);
        stdindentry.set_inode(stdioinode.clone());
        stdoutdentry.set_inode(stdioinode);
        *stdindentry.get_state() = DentryState::Valid;
        *stdoutdentry.get_state() = DentryState::Valid;
        Self{
            fd_table: vec![
                // 0 -> stdin
                Some(Fd::new(Arc::new(Stdin::new(FileInner::new(stdindentry))), FdFlags::empty())),
                // 1 -> stdout
                Some(Fd::new(Arc::new(Stdout::new(FileInner::new(stdoutdentry.clone()))), FdFlags::empty())),
                // 2 -> stderr
                Some(Fd::new(Arc::new(Stderr::new(FileInner::new(stdoutdentry))), FdFlags::empty())),
            ],
            fd_table_rlimit:RLimit{rlimit_cur:MAX_FD,rlimit_max:MAX_FD},
        }
    }
    pub fn from_existed_table(table:&FdTable)->Self{
        let mut new_fd_table = FdTable{
            fd_table:Vec::new(),
            fd_table_rlimit:RLimit{rlimit_cur:MAX_FD,rlimit_max:MAX_FD},
        };
        for fd in table.fd_table.iter() {
            if let Some(fd) = fd {
                let _ = new_fd_table.insert(Some(Fd::new(fd.file(), fd.flags)));
            } else {
                let _ = new_fd_table.insert(None);
            }
        }
        new_fd_table
    }
    pub fn get_file(&self,fd:usize)->SysResult<Arc<dyn File>>{
        if fd >= self.fd_table.len() {
            Err(SysError::EBADF)
        } else {
            self.fd_table[fd].as_ref().map(|fd|{fd.file()}).ok_or(SysError::EBADF)
        }
    }
    pub fn alloc_fd(&mut self) -> SysResult<usize> {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            Ok(fd)
        } else if self.fd_table.len() < self.fd_table_rlimit.rlimit_max{
            self.fd_table.push(None);
            Ok(self.fd_table.len() - 1)
        }
        else {
            Err(SysError::EMFILE)
        }
    }
    pub fn alloc_fd_from(&mut self,start:usize)->Option<usize>{
        let fd = self
            .fd_table
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_i, e)| e.is_none())
            .map(|(i, _)| i);
        if fd.is_some() {
            return fd;
        } else if fd.is_none() && self.fd_table.len() < self.fd_table_rlimit.rlimit_max {
            self.fd_table.push(None);
            return Some(self.fd_table.len() - 1);
        } else {
            return None;
        }
    }
    pub fn dup(&mut self,old_fd:usize)->SysResult<isize>{
        if old_fd >= self.fd_table.len() {
            return Err(SysError::EBADF); // EBADF: 无效的文件描述符
        }
        // 获取要复制的文件对象
        let file = self.get_file(old_fd)?;// 使用 clone 提前获取文件对象
        // 找到一个空闲的文件描述符位置
        let new_fd = self.alloc_fd()?;
        // 复制文件对象的引用到新的位置
        self.fd_table[new_fd] = Some(Fd::new(file, FdFlags::empty()));

        // 返回新的文件描述符
        return Ok(new_fd as isize);
    }
    pub fn dup_with_arg(&mut self,old_fd:usize,arg:usize,flags:vfs_defs::OpenFlags)->SysResult<isize>{
        if old_fd >= self.fd_table.len() {
            return Err(SysError::EBADF); // EBADF: 无效的文件描述符
        }
        // 获取要复制的文件对象
        let file= self.get_file(old_fd)?; // 使用 clone 提前获取文件对象
        // 找到一个空闲的文件描述符位置
        if let Some(new_fd) = self.alloc_fd_from(arg){
            // 复制文件对象的引用到新的位置
            self.fd_table[new_fd] = Some(Fd::new(file, flags.into()));
            // 返回新的文件描述符
            return Ok(new_fd as isize);
        }
        return Err(SysError::ENFILE);
    }
    pub fn dup3(&mut self,old: usize, new: usize, _flags: usize)->SysResult<isize>{
        if old == new {
            return Ok(new as isize);
        }
    
        // 检查文件描述符的有效性
        if old >= self.fd_table.len() {
            return Err(SysError::EBADF); // EBADF: 无效的文件描述符
        }
        // 获取要复制的文件对象
        if let Some(file) = self.fd_table[old].clone() { // 使用 clone 提前获取文件对象
            
            if new >= self.fd_table.len() {
                let cnt = new - self.fd_table.len() + 1;
                for _ in 0..cnt {
                    self.fd_table.push(None);
                }
                if new != self.fd_table.len()-1 {
                    panic!("extend fd_table error!, len={}",self.fd_table.len());
                }
            }
            else if self.fd_table[new].is_some() {
                // new位置有效，需要关闭文件
                //sys_close(new); 被锁阻塞
                self.fd_table[new].take();
            }
    
            // 复制文件对象的引用到新的位置
            self.fd_table[new] = Some(file);
    
            // 返回新的文件描述符
            Ok(new as isize)
        } else {
            return Err(SysError::EBADF);
        }
    }
    pub fn insert(&mut self,file:Option<Fd>)->SysResult<usize>{
        let fd = self.alloc_fd()?;
        self.fd_table[fd] = file;
        Ok(fd)
    }
    pub fn get(&self,fd:usize)->SysResult<&Fd>{
        if fd >= self.fd_table.len() {
            Err(SysError::EBADF)
        } else {
            self.fd_table[fd].as_ref().ok_or(SysError::EBADF)
        }
    }
    pub fn get_mut(&mut self,fd:usize)->SysResult<&mut Fd>{
        if fd >= self.fd_table.len() {
            Err(SysError::EBADF)
        } else {
            self.fd_table[fd].as_mut().ok_or(SysError::EBADF)
        }
    }
    pub fn remove(&mut self, fd: usize) -> SysResult<()> {
        if fd >= self.fd_table.len() {
            Err(SysError::EBADF)
        } else if self.fd_table[fd].is_none() {
            Err(SysError::EBADF)
        } else {
            self.fd_table[fd] = None;
            Ok(())
        }
    }
    pub fn rlimit(&self) -> RLimit {
        self.fd_table_rlimit
    }

    pub fn set_rlimit(&mut self, rlimit: RLimit) {
        self.fd_table_rlimit = rlimit;
        if rlimit.rlimit_max <= self.fd_table.len() {
            self.fd_table.truncate(rlimit.rlimit_max);
        }
    }
}