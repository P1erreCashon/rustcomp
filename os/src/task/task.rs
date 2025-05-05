//!Implementation of [`TaskControlBlock`]
use super::{tid_alloc, TidAddress, TidHandle,current_task,Tms,TimeSpec,FdTable};
use super::aux::*;
use config::{KERNEL_STACK_SIZE, USER_MMAP_TOP, USER_STACK_SIZE,RLimit,MAX_FD};
use crate::fs::{Stdin, Stdout};
use crate::mm::{translated_ref, translated_refmut, MapArea, MapAreaType, MapPermission, MapType, MemorySet};
use arch::addr::VirtAddr;
use riscv::register::mvendorid;
use spin::{Mutex, MutexGuard};
//use crate::trap::{trap_handler, TrapContext};
use arch::pagetable::PageTable;
use arch::{
    read_current_tp, run_user_task, KContext, KContextArgs, TrapFrame, TrapFrameArgs, PAGE_SIZE,
};
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use alloc::string::String;
use core::cell::RefMut;
use vfs_defs::{Dentry,File};
use vfs::get_root_dentry;
use system_result::{SysResult,SysError};
use core::mem::size_of;
use crate::task::SignalFlags;
use crate::task::signal::SigAction;
use crate::task::action::SignalActions;
use crate::syscall::CloneFlags;
//use user_lib::{USER_HEAP_SIZE};

const MODULE_LEVEL:log::Level = log::Level::Trace;

const _F_SIZE: usize = 20 - 2 * size_of::<u64>() - size_of::<u32>();

pub struct KernelStack {
//    inner: Arc<[u128; KERNEL_STACK_SIZE / size_of::<u128>()]>,
    inner: Arc<Vec<u128>>,
}
impl KernelStack {
    pub fn new() -> Self {  
        Self {
            inner: Arc::new(vec![0u128; KERNEL_STACK_SIZE / size_of::<u128>()]),
        }
    }

    pub fn get_position(&self) -> (usize, usize) {
        let bottom = self.inner.as_ptr() as usize;
        (bottom, bottom + KERNEL_STACK_SIZE)
    }

    pub fn release(&mut self){
        self.inner = Arc::new(vec![0u128;1]);
    }
}

///
pub struct TaskControlBlock {
    // immutable
    ///
    pub tid: TidHandle,
    ///
    pub pid:usize,
    // mutable
    inner: Mutex<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub trap_cx: TrapFrame,
    #[allow(unused)]
    pub base_size: usize,
    pub task_cx:KContext,
    pub task_status: TaskStatus,
    pub memory_set: Arc<Mutex<MemorySet>>,
    pub kernel_stack: KernelStack,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,//why use Arc:TaskManager->TCB & TCB.children->TCB & TaskManager creates Arc<TCB>
    pub exit_code: i32,
    pub fd_table: Arc<Mutex<FdTable>>,//Vec<Option<Arc<dyn File + Send + Sync>>>,
   // pub fd_table_rlimit:RLimit,
    pub signals: SignalFlags, // 新增：未处理的信号
    pub signal_queue: Vec<usize>, // 新增：信号队列，按发送顺序存储
    pub killed: bool,         // 新增：是否被信号终止
    pub frozen: bool,
    pub signal_mask: SignalFlags,      // 信号掩码
    pub signal_mask_backup: SignalFlags, // 保存原始信号掩码
    pub signal_actions: Arc<Mutex<SignalActions>>, // 信号处理函数表
    pub handling_sig: isize,           // 当前正在处理的信号
    pub trap_ctx_backup: Option<TrapFrame>, // 添加 trap_ctx_backup 字段
    pub cwd:Arc<dyn Dentry>,//工作目录
    pub heap_top: usize,
    pub heap_bottom: usize, //brk收缩判断
    pub stack_bottom: usize,
    pub max_data_addr: usize,
    pub tms: Tms,

    pub tidaddress:TidAddress,
    //pub mmap_top: usize,
}
fn task_entry() {
    let task = current_task()
        .unwrap()
        .inner
        .lock()
        .get_trap_cx() as *mut TrapFrame;
    // run_user_task_forever(unsafe { task.as_mut().unwrap() })
    let ctx_mut = unsafe { task.as_mut().unwrap() };
    loop {
        run_user_task(ctx_mut);
    }
}

fn blank_kcontext(ksp: usize) -> KContext {
    let mut kcx = KContext::blank();
    kcx[KContextArgs::KPC] = task_entry as usize;
    kcx[KContextArgs::KSP] = ksp;
    kcx[KContextArgs::KTP] = read_current_tp();
    kcx
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapFrame  {
    //    self.trap_cx_ppn.get_mut()
        let paddr = &self.trap_cx as *const TrapFrame as usize as *mut TrapFrame;
        unsafe { paddr.as_mut().unwrap() }
    }
    pub fn get_user_token(&self) -> PageTable  {
        self.memory_set.lock().token()
    }
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

impl TaskControlBlock {
    ///
    pub fn inner_exclusive_access(&self) -> MutexGuard<TaskControlBlockInner> {
        self.inner.lock()
    }
    ///
    pub fn new(elf_data: &[u8]) -> Self {//这个函数似乎只用来创建initproc,所以它的cwd是确定的
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point, heap_top,_entry_size,_ph_count,_tls_addr,_phdr) = MemorySet::from_elf(elf_data);

        // alloc a pid and a kernel stack in kernel space
        let tid_handle = tid_alloc();
        let pid = tid_handle.0;
        let kstack = KernelStack::new();
        let task_control_block = Self {
            tid: tid_handle,
            pid,
            inner: 
                Mutex::new(TaskControlBlockInner {
                    trap_cx:TrapFrame::new(),
                    base_size: user_sp,
                    task_cx: blank_kcontext(kstack.get_position().1),
                    task_status: TaskStatus::Ready,
                    memory_set:Arc::new(Mutex::new(memory_set)),
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: Arc::new(Mutex::new(FdTable::new())),
                    cwd:get_root_dentry(),
                    kernel_stack: kstack,
                    signals: Default::default(),  // 使用 Default::default() 初始化 signals
                    killed: false,
                    frozen: false,
                    signal_mask: SignalFlags::empty(),
                    signal_mask_backup: SignalFlags::empty(),
                    signal_actions: Arc::new(Mutex::new(SignalActions::new())),
                    handling_sig: -1,
                    heap_top: heap_top,
                    heap_bottom: heap_top,
                    stack_bottom: user_sp - USER_STACK_SIZE,
                    max_data_addr: heap_top,
                    tms: Tms::new(),
                    //mmap_top: USER_MMAP_TOP,
                    tidaddress:TidAddress::new(),
                    trap_ctx_backup: None, // 初始化 trap_ctx_backup
                    signal_queue: Vec::new(),
                    }
                ),
        };
            
        
        log_info!("proc {} created",task_control_block.getpid());
        // prepare TrapContext in user space
        let trap_cx = task_control_block.inner_exclusive_access().get_trap_cx();
  //     *trap_cx = TrapContext::app_init_context(
  //          entry_point,
  //          user_sp,
  //          KERNEL_SPACE.lock().token(),
   //         kernel_stack_top,
   //         trap_handler as usize,
   //     );
        trap_cx[TrapFrameArgs::SEPC] = entry_point;
        trap_cx[TrapFrameArgs::SP] = user_sp;
        task_control_block
    }
    ///
    fn push_into_user_stack<T: 'static>(&self,token:PageTable,user_sp:&mut usize,data:T){
        *user_sp -= core::mem::size_of::<T>();
        *translated_refmut(token, *user_sp as *mut T) = data;
    }
    ///
    pub fn exec(&self, elf_data: &[u8], args: Vec<String>) {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (mut memory_set, mut user_sp, entry_point, heap_top,entry_size,ph_count,tls_addr,phdr) = MemorySet::from_elf(elf_data);
        self.inner_exclusive_access().heap_top = heap_top;
        self.inner_exclusive_access().stack_bottom =user_sp - USER_STACK_SIZE;
        self.inner_exclusive_access().max_data_addr = heap_top;
        self.inner_exclusive_access().tms= Tms::new();
        memory_set.activate();
        //1. 使用0标记栈底，压入一个用于glibc的伪随机数，并以16字节对齐
        let token = memory_set.token();
        let mut data:u64 = 0;
        self.push_into_user_stack(token,&mut user_sp,data);

        data = 0x114514FF114514;
        self.push_into_user_stack(token,&mut user_sp,data);

        data = 0x2 << 60;
        self.push_into_user_stack(token,&mut user_sp,data);

        data = 0x3 << 60;
        self.push_into_user_stack(token,&mut user_sp,data);

        let rd_pos = user_sp;

        user_sp = (user_sp - 1) & !0xf;
        // 2. 压入 env string

        data = 0;
        self.push_into_user_stack(token,&mut user_sp,data);

        user_sp -= user_sp % 16;

        // 3. 压入 arg string 

        let mut argv_addr:Vec<usize> = vec![0;args.len()];
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            argv_addr[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(memory_set.token(), p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(memory_set.token(), p as *mut u8) = 0;
        }

        user_sp -= user_sp % 16;

        // 4. 压入 auxv
        let mut aux = AuxvT::new(AT_NULL, 0);
        self.push_into_user_stack(token,&mut user_sp,aux);

        aux.a_type = AT_PAGESZ;
        aux.a_val = PAGE_SIZE;
        self.push_into_user_stack(token,&mut user_sp,aux);

        aux.a_type = AT_PHNUM;
        aux.a_val = ph_count as usize;
        self.push_into_user_stack(token,&mut user_sp,aux);
    
        aux.a_type = AT_PHENT;
        aux.a_val = entry_size as usize;
        self.push_into_user_stack(token,&mut user_sp,aux);

        aux.a_type = AT_PHDR;
        aux.a_val = phdr;
        self.push_into_user_stack(token,&mut user_sp,aux);

        aux.a_type = AT_RANDOM;
        aux.a_val = rd_pos;
        self.push_into_user_stack(token,&mut user_sp,aux);

        if let Some(dl_entry) = memory_set.load_interp(elf_data){
            aux.a_type = AT_BASE;
            aux.a_val = dl_entry;
            self.push_into_user_stack(token,&mut user_sp,aux);
        }
        else{
            aux.a_type = AT_BASE;
            aux.a_val = 0;
            self.push_into_user_stack(token,&mut user_sp,aux);
        }

        // 5. 压入 envp
        data = 0;
        self.push_into_user_stack(token,&mut user_sp,data);

        //push *argv
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    memory_set.token(),
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;
        for i in 0..args.len(){
            *argv[i] = argv_addr[i];
        }
        //push argc on stack
        data = args.len() as u64;        
        self.push_into_user_stack(token,&mut user_sp,data);
        // make the user_sp aligned to 8B for k210 platform
        user_sp -= user_sp % core::mem::size_of::<usize>();

        memory_set.activate();
        // **** access current TCB exclusively
        let mut inner = self.inner_exclusive_access();
        // substitute memory_set
        inner.memory_set = Arc::new(Mutex::new(memory_set));
        // update trap_cx ppn
        // FIXME: This is a temporary solution
        inner.trap_cx = TrapFrame::new();
        // initialize trap_cx
        let mut trap_cx = TrapFrame::new();
        trap_cx[TrapFrameArgs::SEPC] = entry_point;
        trap_cx[TrapFrameArgs::SP] = user_sp;
    //    trap_cx[TrapFrameArgs::ARG0] = args.len();  //这一句会干死glibc的动态链接器 ..?
    //    trap_cx[TrapFrameArgs::ARG1] = argv_base;   
        trap_cx[TrapFrameArgs::TLS] = tls_addr as usize;
        // TODO: Set Kernel Stack Top
        *inner.get_trap_cx() = trap_cx;
        // **** release current PCB
    }
    ///
    pub fn fork(self: &Arc<TaskControlBlock>, flags: CloneFlags,stack:usize,ctid:*mut i32) -> Arc<TaskControlBlock> {
        //crate::mm::show_mem_alloced();
        // ---- hold parent PCB lock
        let mut parent_inner = self.inner_exclusive_access();
        // copy user space(include trap context)
        let memory_set;
        let fd_table;
        let signal_actions;
        let signal_mask;
        let mut tidaddress = TidAddress::new();
        let mut trap_cx = parent_inner.trap_cx.clone();
        let tid_handle = tid_alloc();
        let pid;
        if flags.contains(CloneFlags::FILES) {
            fd_table = Arc::clone(&parent_inner.fd_table);
        }
        else{
            fd_table = Arc::new(Mutex::new(FdTable::from_existed_table(&parent_inner.fd_table.lock())));
        }        
        if flags.contains(CloneFlags::VM) {
            memory_set = Arc::clone(&parent_inner.memory_set);
        }
        else {
            memory_set = Arc::new(Mutex::new(MemorySet::from_existed_user(&parent_inner.memory_set.lock())));
        }
        if flags.contains(CloneFlags::SIGHAND){
            signal_actions = Arc::clone(&parent_inner.signal_actions);
        }
        else{
            signal_actions = Arc::new(Mutex::new(SignalActions::new()));
        }
        if flags.contains(CloneFlags::THREAD) {
            signal_mask = SignalFlags::empty();
            pid = self.pid;
        } else {
            signal_mask = parent_inner.signal_mask.clone();
            trap_cx[TrapFrameArgs::RET] = 0;
            pid = tid_handle.0;
        }
        if stack != 0{
            let token = parent_inner.get_user_token();
            let entry_point = translated_ref(token, stack as *const usize);
            let arg = translated_ref(token, (stack + 8) as *const usize);
        //    println!("entrypoint:{:x} arg:{:x}",*entry_point,*arg);
            trap_cx[TrapFrameArgs::SEPC] = *entry_point;
            trap_cx[TrapFrameArgs::RET] = *arg;
            trap_cx[TrapFrameArgs::SP] = stack;
        }
     //   println!("fork ctid:{:x}",ctid as usize);
        if flags.contains(CloneFlags::CHILD_SETTID) {
            tidaddress.set_child_tid = Some(ctid as usize);
        }
        if flags.contains(CloneFlags::CHILD_CLEARTID) {
            tidaddress.clear_child_tid = Some(ctid as usize);
        } 
        // alloc a pid and a kernel stack in kernel space
        
        let kstack = KernelStack::new();
        log::debug!("fork curproc={} new proc={}",self.getpid(),tid_handle.0);
        let task_control_block = Arc::new(TaskControlBlock {
            tid: tid_handle,
            pid,
            inner: 
                Mutex::new(TaskControlBlockInner {
                    trap_cx,
                    base_size: parent_inner.base_size,
                    task_cx: blank_kcontext(kstack.get_position().1),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table,
                    cwd:parent_inner.cwd.clone(),
                    kernel_stack: kstack,
                    signals: Default::default(),  // 使用 Default::default() 初始化 signals
                    killed: false,
                    frozen: false,
                    signal_mask,
                    signal_mask_backup: SignalFlags::empty(),
                    signal_actions,
                    handling_sig: -1,
                    heap_top: parent_inner.heap_top,
                    heap_bottom: parent_inner.heap_bottom,
                    stack_bottom: parent_inner.stack_bottom,
                    max_data_addr: parent_inner.max_data_addr,
                    tms: Tms::from_other_task(&parent_inner.tms),
                    //mmap_top: parent_inner.mmap_top,
                    tidaddress,
                    trap_ctx_backup: None, // 初始化 trap_ctx_backup
                    signal_queue: Vec::new(),
                })
            ,
        });

        parent_inner.children.push(task_control_block.clone());
        // modify kernel_sp in trap_cx
        // **** access child PCB exclusively
        // return
        task_control_block
        // **** release child PCB
        // ---- release parent PCB
    }
    ///
    pub fn getpid(&self) -> usize {
        self.pid
    }
    ///
    pub fn gettid(&self) ->usize {
        self.tid.0
    }
}

///
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    ///
    Ready,
    ///
    Running,
    ///
    Zombie,
    ///
    Blocked,
}
