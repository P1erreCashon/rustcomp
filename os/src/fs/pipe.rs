extern crate alloc; 
use core::task::Poll;

use crate::suspend_current_and_run_next;
use alloc::sync::{Arc,Weak}; 
use sync::Mutex;
use vfs_defs::DentryState;
use vfs_defs::File;
use vfs_defs::UserBuffer;
use vfs_defs::FileInner;
use vfs_defs::{Dentry,PollEvents,Inode,InodeMeta,DentryInner,OpenFlags,DiskInodeType,RenameFlags,Kstat,ino_alloc,InodeMode,SuperBlock};
use alloc::string::String;
use system_result::{SysError,SysResult};
use crate::sync::UPSafeCell;
const RING_BUFFER_SIZE: usize = 2048;

/// pipe
pub struct Pipe {
    readable: bool,
    writable: bool,
    buffer: Arc<Mutex<PipeRingBuffer>>,
    inner: FileInner,
}

impl Pipe {
    /// from an existed pipe creates read-pipe
    pub fn read_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>, dentry: Arc<dyn Dentry>) -> Self {
        Self {
            readable: true,
            writable: false,
            buffer,
            inner: FileInner::new(dentry),
        }
    }
    /// from an existed pipe creates write-pipe
    pub fn write_end_with_buffer(buffer: Arc<Mutex<PipeRingBuffer>>, dentry: Arc<dyn Dentry>) -> Self {
        Self {
            readable: false,
            writable: true,
            buffer,
            inner: FileInner::new(dentry),
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum RingBufferStatus {
    //FULL,
    EMPTY,
    NORMAL,
}
///带有一定大小缓冲区的字节队列，我们抽象为 PipeRingBuffer 类型
pub struct PipeRingBuffer {
    arr: [u8; RING_BUFFER_SIZE],
    head: usize,
    tail: usize,
    // a ring queue
    status: RingBufferStatus,
    write_end: Option<Weak<Pipe>>,
}

impl PipeRingBuffer {
    /// new method
    pub fn new() -> Self {
        Self {
            arr: [0; RING_BUFFER_SIZE],
            head: 0,
            tail: 0,
            status: RingBufferStatus::EMPTY,
            write_end: None,
        }
    }
    ///keep weak-ptr in write-end
    pub fn set_write_end(&mut self, write_end: &Arc<Pipe>) {
        self.write_end = Some(Arc::downgrade(write_end));
    }
}

/// Return (read_end, write_end)
pub fn make_pipe(superblock:Arc<dyn SuperBlock>) -> (Arc<Pipe>, Arc<Pipe>) {
    let pipe_dentry = PipeDentry::new(superblock.clone());
    let pipe_inode = PipeInode::new(superblock);
    pipe_dentry.set_inode(pipe_inode);
    *pipe_dentry.get_state() = DentryState::Valid;
    let buffer = Arc::new(Mutex::new(PipeRingBuffer::new()));
    let read_end = Arc::new(
        Pipe::read_end_with_buffer(buffer.clone(), pipe_dentry.clone())
    );
    let write_end = Arc::new(
        Pipe::write_end_with_buffer(buffer.clone(), pipe_dentry)
    );
    buffer.lock().set_write_end(&write_end);// 调用 PipeRingBuffer::set_write_end 在管道中保留它的写端的弱引用计数
    (read_end, write_end)
}

impl PipeRingBuffer {
    /// read first byte(head++)
    pub fn read_byte(&mut self) -> u8 {
        self.status = RingBufferStatus::NORMAL;
        let c = self.arr[self.head];
        self.head = (self.head + 1) % RING_BUFFER_SIZE;
        if self.head == self.tail {
            self.status = RingBufferStatus::EMPTY;
        }
        c
    }
    /// return available number of bytes
    pub fn available_read(&self) -> usize {
        if self.status == RingBufferStatus::EMPTY {
            0
        } else {
            if self.tail > self.head {
                self.tail - self.head
            } else {
                self.tail + RING_BUFFER_SIZE - self.head
            }
        }
    }
    /// try to destroy pipe
    pub fn all_write_ends_closed(&self) -> bool {
        self.write_end.as_ref().unwrap().upgrade().is_none()
    }
}

impl File for Pipe {
    //fn read(&self, buf: UserBuffer) -> usize {
    fn read(&self, buf: &mut [u8]) -> usize {
        assert!(self.readable());
        let want_to_read = buf.len();
        let mut buf_iter = buf.into_iter(); // iterator for reading bytes 
        let mut already_read = 0usize;
        // change task when data inadequate

            //let mut ring_buffer = self.buffer.exclusive_access();
        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_read = ring_buffer.available_read();
            if loop_read == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return already_read;
                }
                drop(ring_buffer);
                suspend_current_and_run_next(); //change task: empty pipe
                continue;
            }    
            let want_to_read = want_to_read.min(loop_read);
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    *byte_ref = ring_buffer.read_byte();
                    already_read += 1;
                    if already_read == want_to_read {
                        return want_to_read;
                    }
                } else {
                    return already_read;
                }
            }
            return already_read;
        }
    }
    //需要返回什么？
    fn get_inner(&self) -> &FileInner { 
        &self.inner
    }

    fn readable(&self) -> bool { 
        self.readable 
    }

    fn writable(&self) -> bool { 
        self.writable 
    }

    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> usize {
        assert!(self.readable());

        let want_to_read = buf.len();
        let mut buf_iter = buf.into_iter();
        let mut already_read = 0usize;

        loop {
            let mut ring_buffer = self.buffer.lock();
            let loop_read = ring_buffer.available_read();

            if loop_read == 0 {
                if ring_buffer.all_write_ends_closed() {
                    return already_read; // 如果写端已关闭，返回已读取的字节数
                }
                drop(ring_buffer);
                suspend_current_and_run_next(); // 切换任务：管道为空
                continue;
            }
            let want_to_read = want_to_read.min(loop_read);
            for _ in 0..loop_read {
                if let Some(byte_ref) = buf_iter.next() {
                    *byte_ref = ring_buffer.read_byte(); // 从缓冲区读取一个字节
                    already_read += 1;

                    if already_read == want_to_read {
                        return want_to_read; // 读取完成
                    }
                } else {
                    return already_read; // 缓冲区已读完
                }
            }
        }
    }
    
    fn write_at(&self, _offset: usize, buf: &[u8]) -> usize {
        assert!(self.writable());

        let want_to_write = buf.len();
        let mut buf_iter = buf.iter(); // 迭代器用于写入字节
        let mut already_written = 0usize;

        // change task while buffer is full
        loop {
            let mut ring_buffer = self.buffer.lock();

            // 计算可写入的空间
            let available_write = RING_BUFFER_SIZE - ring_buffer.available_read();

            if available_write == 0 {
                drop(ring_buffer);
                suspend_current_and_run_next();
                continue;
            }
            // 完成一轮写重新计算可写量
            for _ in 0..available_write {
                if let Some(&byte) = buf_iter.next() {
                    let tail=ring_buffer.tail;
                    ring_buffer.arr[tail] = byte; // 写入一个字节
                    ring_buffer.tail = (ring_buffer.tail + 1) % RING_BUFFER_SIZE;
                    ring_buffer.status = RingBufferStatus::NORMAL;
                    already_written += 1;

                    if already_written == want_to_write {
                        return want_to_write; // 写入完成
                    }
                } else {
                    return already_written; // 数据已写完
                }
            }
        }
    }
    fn poll(&self, _events: PollEvents) -> PollEvents {
        if self.readable{
            return PollEvents::POLLIN;
        }
        return PollEvents::POLLOUT;
    }
}

pub struct PipeDentry {
    inner: DentryInner,
}

impl PipeDentry {
    pub fn new(
        superblock:Arc<dyn SuperBlock>
    ) -> Arc<Self> {
        Arc::new(Self {
            inner:DentryInner::new(String::from("pipe"), superblock, None),
        })
    }
}
impl Dentry for PipeDentry{
    fn get_inner(&self) -> &DentryInner {
        &self.inner
    }
    fn open(self:Arc<Self>,_flags:OpenFlags)->Arc<dyn File> {
        unreachable!()
    }
    fn concrete_create(self: Arc<Self>, _name: &str, _type:DiskInodeType) -> SysResult<Arc<dyn Dentry>> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_lookup(self: Arc<Self>, _name: &str) -> SysResult<Arc<dyn Dentry>> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_link(self: Arc<Self>, _new: &Arc<dyn Dentry>) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_unlink(self: Arc<Self>, _old: &Arc<dyn Dentry>) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn load_dir(self:Arc<Self>)->SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    /* 
    fn ls(self:Arc<Self>)->Vec<String> {
        Vec::new()
    }*/
    fn concrete_new_child(self: Arc<Self>, _name: &str) -> Arc<dyn Dentry> {
        unimplemented!()
    }
    fn concrete_rename(self: Arc<Self>, _new: Arc<dyn Dentry>, _flags: RenameFlags) -> SysResult<()> {
        Err(SysError::ENOTDIR)
    }
    fn concrete_getchild(self:Arc<Self>, _name: &str) -> Option<Arc<dyn Dentry>> {
        unimplemented!()
    }
    fn self_arc(self:Arc<Self>) -> Arc<dyn Dentry> {
        unimplemented!()
    }
}

pub struct PipeInode{
    meta:InodeMeta
}
impl PipeInode{
    pub fn new(superblock:Arc<dyn SuperBlock>)->Arc<Self>{
        Arc::new(Self{
            meta:InodeMeta::new(InodeMode::FIFO, ino_alloc(), superblock),
        })
    }
}
impl Inode for PipeInode{
    fn get_meta(&self) -> &InodeMeta {
        &self.meta
    }
    fn get_attr(&self)->system_result::SysResult<Kstat> {
            Ok(Kstat{
                st_dev: 0,
                st_ino: self.meta.ino as u64,
                st_mode: self.meta.mode.bits(),
                st_nlink: 0,
                st_uid: 0,
                st_gid: 0,
                st_rdev: 0,
                __pad: 0,
                st_size: self.get_size() as u64,
                st_blksize: 0,
                __pad2: 0,
                st_blocks:0,
                st_atime_sec: 0,
                st_atime_nsec: 0,
                st_mtime_sec: 0,
                st_mtime_nsec: 0,
                st_ctime_sec: 0,
                st_ctime_nsec: 0,
                unused: 0,
            })

    }
    fn load_from_disk(&self) {
        
    }
    fn clear(&self) {
        
    }
    fn get_size(&self) -> u32 {
        let size = self.meta.inner.lock().size as u32;
        return size;
    }
}