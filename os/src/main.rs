//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`task`]: Task management
//! - [`syscall`]: System call handling and implementation
//! - [`mm`]: Address map using SV39
//! - [`sync`]: Wrap a static data structure inside it so that we are able to access it without any `unsafe`.
//! - [`fs`]: Separate user from file system with some structures
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`task::run_tasks()`] and for the first time go to
//! userspace.

#![allow(missing_docs)]
#![deny(warnings)]
#![allow(unused_imports)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
extern crate bitflags;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
//#[macro_use]
//mod logging;
//mod config;
mod drivers;
pub mod fs;
pub mod lang_items;
pub mod mm;
pub mod sync;
pub mod syscall;
pub mod task;

#[macro_use]
extern  crate logger;
use logger::*;
use task::current_task;
use core::arch::global_asm;

//use drivers::{chardevice::{CharDevice, UART}, BLOCK_DEVICE};
use crate::{
    syscall::syscall,
    task::{
        exit_current_and_run_next,
        suspend_current_and_run_next, 
    },
};
use arch::api::ArchInterface;
use arch::{TrapFrame, TrapFrameArgs, TrapType};
use arch::addr::PhysPage;
use crate_interface::impl_interface;
use fdt::node::FdtNode;
use lazy_static::*;
//use sync::IntrCell;
use arch::TrapType::*;
use log::Record;
use syscall::lazy_brk;
//lazy_static! {
    //
  //  pub static ref DEV_NON_BLOCKING_ACCESS: IntrCell<bool> =
  //      IntrCell::new(false);
//}

//extern  "C"{
//     fn _skernel();
//     fn stext();
//     fn etext();
//     fn srodata();
//     fn erodata();
//     fn _sdata();
//     fn _edata();
//     fn _load_end();
//     fn _sbss();
//     fn _ebss();
//     fn end();
//}
struct LogIfImpl;

#[impl_interface]
impl LogIf for LogIfImpl{
    fn print_log(record: &Record){
        println!("{}: {}", record.level(), record.args());
    }
}
///
pub struct ArchInterfaceImpl;

#[impl_interface]
impl ArchInterface for ArchInterfaceImpl {
    /// Init allocator
    fn init_allocator(){
        mm::init_heap();
    }
    /// kernel interrupt
    fn kernel_interrupt(ctx: &mut TrapFrame, trap_type: TrapType){
        // println!("trap_type @ {:x?} {:#x?}", trap_type, ctx);
        match trap_type {
            Breakpoint => return,
            UserEnvCall => {
                // jump to next instruction anyway
                ctx.syscall_ok();
                let args = ctx.args();
                // get system call return value
                // info!("syscall: {}", ctx[TrapFrameArgs::SYSCALL]);
                let id = ctx[TrapFrameArgs::SYSCALL];
                let result = syscall(ctx[TrapFrameArgs::SYSCALL], [args[0], args[1], args[2],args[3],args[4],args[5]]);
                // cx is changed during sys_exec, so we have to call it again
                if id != 93{//exec don't return
                    ctx[TrapFrameArgs::RET] = result as usize;//exec中这一句会干死glibc的动态链接器
                }
            }
            StorePageFault(_paddr) | LoadPageFault(_paddr) | InstructionPageFault(_paddr) => {
                /* 
                println!(
                    "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                    scause.cause(),
                    stval,
                    current_trap_cx().sepc,
                );
                */
                
                /*let task=current_task().unwrap();
                let inner=task.inner_exclusive_access();
                println!("\nheaptop={} data={}",inner.heap_top,inner.max_data_addr);
                drop(inner);
                */
                match lazy_brk(_paddr) {
                    Ok(0) => {
                        /*println!("lazy-brk: {}",_paddr);
                        let task=current_task().unwrap();
                        let inner=task.inner_exclusive_access();
                        println!("heaptop={} data={}",inner.heap_top,inner.max_data_addr);
                        */
                    }
                    _ => {
                        println!("err {:x?},sepc:{:x}", trap_type,ctx.sepc);
                    //      ctx.syscall_ok();
                        exit_current_and_run_next(-1);
                    }
                }
                //println!("err {:x?},sepc:{:x}", trap_type,ctx.sepc);
          //      ctx.syscall_ok();
                //exit_current_and_run_next(-1);
            }
            IllegalInstruction(_) => {
                println!("IllegalInstruction!");
                exit_current_and_run_next(-1);
            }
            Time => {   
                suspend_current_and_run_next();
            }
            _ => {
           //     println!("unsuspended trap type: {:?}", trap_type);
            }
        }
    }
    /// init log
    fn init_logging(){
        logger::init_logger();
    }
    /// add a memory region
    fn add_memory_region(start: usize, end: usize){
        mm::init_frame_allocator(start, end);
    }
    /// kernel main function, entry point.
    fn main(hartid: usize){
        if hartid != 0 {
            return;
        }
        //  UART.init();    
        // println!("[kernel] Hello, world! id:{}",hartid);
        // println!("_skernel:{:x}",_skernel as usize);
        // println!("stext:{:x}",stext as usize);
        // println!("etext:{:x}",etext as usize);
        // println!("srodata:{:x}",srodata as usize);
        // println!("erodata:{:x}",erodata as usize);
        // println!("_sdata:{:x}",_sdata as usize);
        // println!("_edata:{:x}",_edata as usize);
        // println!("_load_end:{:x}",_load_end as usize);
        // println!("_sbss:{:x}",_sbss as usize);
        // println!("_ebss:{:x}",_ebss as usize);
        // println!("_end:{:x}",end as usize);
        arch::init_interrupt();
        //timer::set_next_trigger();
    //    board::device_init();
        device::BLOCK_DEVICE.call_once(||drivers::BLOCK_DEVICE.clone());
        vfs::init();
        fs::list_apps();
        task::add_initproc();
    //    *DEV_NON_BLOCKING_ACCESS.lock() = true;
        task::run_tasks();
        panic!("Unreachable in rust_main!");
    }
    /// Alloc a persistent memory page.
    fn frame_alloc_persist() -> PhysPage{
        mm::frame_alloc_persist().expect("can't find memory page")
    }
    /// Unalloc a persistent memory page
    fn frame_unalloc(ppn: PhysPage){
        mm::frame_dealloc(ppn);
    }
    /// Preprare drivers.
    fn prepare_drivers(){

    }
    /// Try to add device through FdtNode
    fn try_to_add_device(_fdt_node: &FdtNode){

    }
}
