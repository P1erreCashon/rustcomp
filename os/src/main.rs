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
pub mod timer;

#[macro_use]
extern  crate logger;
use fs::open_file;
use logger::*;
use task::current_task;
use vfs_defs::OpenFlags;
use core::arch::global_asm;

//use drivers::{chardevice::{CharDevice, UART}, BLOCK_DEVICE};
use crate::{
    syscall::syscall,
    task::{
        exit_current_and_run_next,
        suspend_current_and_run_next, 
    },
};
use arch::{api::ArchInterface, PAGE_SIZE};
use arch::{TrapFrame, TrapFrameArgs, TrapType};
use arch::addr::PhysPage;
use crate_interface::impl_interface;
use fdt::node::FdtNode;
use lazy_static::*;
//use sync::IntrCell;
use arch::TrapType::*;
use log::Record;
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
                if id != 93 && id != 139{//exec sigreturn don't return
                    ctx[TrapFrameArgs::RET] = result as usize;//exec中这一句会干死glibc的动态链接器
                }
            }
            StorePageFault(_paddr) | LoadPageFault(_paddr) | InstructionPageFault(_paddr) => {
                let ctask = current_task().unwrap();
              //  println!("pgfault addr:{:x} tid:{}",_paddr,ctask.gettid());
              //  println!("trap_type @ {:x?} {:#x?}", trap_type, ctx);
                let inner = ctask.inner_exclusive_access();
                let mut memory_set = inner.memory_set.lock();
                if memory_set.handle_lazy_addr(_paddr, trap_type).is_err() {
                    match trap_type {
                        StorePageFault(_paddr)=>{
                            let r = memory_set.handle_cow_addr(_paddr);
                            if r.is_err(){
                         //       memory_set.debug_addr_info();                                
                                println!("err {:x?},sepc:{:x},sepcpage:{:x} id:{}", trap_type,ctx[TrapFrameArgs::SEPC],ctx[TrapFrameArgs::SEPC]/PAGE_SIZE,ctask.gettid());
                                //      ctx.syscall_ok();
                                memory_set.debug_addr_info();
                                drop(memory_set);
                                drop(inner);
                                drop(ctask);
                                exit_current_and_run_next(-1);
                            }
                        }
                        _ =>{
                            println!("err {:x?},sepc:{:x},sepcpage:{:x} id:{}", trap_type,ctx[TrapFrameArgs::SEPC],ctx[TrapFrameArgs::SEPC]/PAGE_SIZE,ctask.gettid());
                            //      ctx.syscall_ok();
                            memory_set.debug_addr_info();
                            drop(memory_set);
                            drop(inner);
                            drop(ctask);
                            exit_current_and_run_next(-1);
                        }
                    }

                }
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
        println!("intr init");
        device::BLOCK_DEVICE.call_once(||drivers::BLOCK_DEVICE.clone());
        println!("device added");
        vfs::init();
        let superblock = vfs::get_root_dentry().get_superblock();
        let dev = vfs::get_root_dentry().lookup("dev").unwrap();
        let ttyinner = vfs_defs::DentryInner::new(alloc::string::String::from("tty"), superblock.clone(),Some(dev));
        let ttydentry = fs::StdioDentry::new(ttyinner);
        let ttyinode = fs::StdioInode::new(vfs_defs::InodeMeta::new(vfs_defs::InodeMode::CHAR, vfs_defs::ino_alloc() as usize, superblock));
        vfs::add_tty(ttydentry,alloc::sync::Arc::new( ttyinode));
        println!("vfs init");
        fs::list_apps();
        create_testcase();
        task::add_initproc();
        println!("initproc add");
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

#[allow(unused)]
fn create_testcase(){
    let testfile = open_file("/testcase.sh", OpenFlags::RDWR | OpenFlags::CREATE).unwrap();
    let busyboxcmd = open_file("/glibc/busybox_cmd.txt", OpenFlags::RDWR).unwrap();
    busyboxcmd.write(BUSYBOX_SCRIPT.as_bytes());
    drop(busyboxcmd);
    let busyboxcmd = open_file("/musl/busybox_cmd.txt", OpenFlags::RDWR).unwrap();
    busyboxcmd.write(BUSYBOX_SCRIPT.as_bytes());
    drop(busyboxcmd);
    testfile.write("cd /glibc\n/glibc/busybox sh busybox_testcode.sh\n".as_bytes());
    testfile.write("cd /musl\n./busybox sh busybox_testcode.sh\n".as_bytes());
    testfile.write("cd /glibc\n./busybox sh libcbench_testcode.sh\n".as_bytes());
    testfile.write("cd /musl\n./busybox sh libcbench_testcode.sh\n".as_bytes());
    let luatestcode = open_file("/glibc/lua_testcode.sh", OpenFlags::RDWR).unwrap();
    luatestcode.write(LUA_SCRIPT_GLIBC.as_bytes());
    drop(luatestcode);
    let luatestcode = open_file("/musl/lua_testcode.sh", OpenFlags::RDWR).unwrap();
    luatestcode.write(LUA_SCRIPT_MUSL.as_bytes());
    drop(luatestcode);
   // testfile.write("cd /glibc\n./busybox sh lua_testcode.sh\n".as_bytes());
 //   testfile.write("cd /musl\n./busybox sh lua_testcode.sh\n".as_bytes());
    let runstatic = open_file("/glibc/run-static.sh", OpenFlags::RDWR).unwrap();
    runstatic.write(LIBCTEST_SCRIPT_GLIBC.as_bytes());
    drop(runstatic);
    let runstatic = open_file("/musl/run-static.sh", OpenFlags::RDWR).unwrap();
    runstatic.write(LIBCTEST_SCRIPT_MUSL.as_bytes());
    drop(runstatic);
    testfile.write("cd /glibc\n./busybox sh run-static.sh\n".as_bytes());
    testfile.write("cd /musl\n./busybox sh run-static.sh\n".as_bytes());
}
const BUSYBOX_SCRIPT: &str = r#####"echo "#### independent command test"
ash -c exit
sh -c exit
basename /aaa/bbb
cal
clear
date 
df 
dirname /aaa/bbb
dmesg 
du
expr 1 + 1
false
true
which ls
uname
uptime
printf "abc\n"
ps
pwd
free
hwclock
kill 10
ls
sleep 1
echo "#### file opration test"
touch test.txt
echo "hello world" > test.txt
cat test.txt
cut -c 3 test.txt
od test.txt
head test.txt
tail test.txt 
hexdump -C test.txt 
md5sum test.txt
echo "ccccccc" >> test.txt
echo "bbbbbbb" >> test.txt
echo "aaaaaaa" >> test.txt
echo "2222222" >> test.txt
echo "1111111" >> test.txt
echo "bbbbbbb" >> test.txt
sort test.txt | ./busybox uniq
stat test.txt
strings test.txt 
wc test.txt
[ -f test.txt ]
more test.txt
rm test.txt
mkdir test_dir
mv test_dir test
rmdir test
grep hello busybox_cmd.txt
cp busybox_cmd.txt busybox_cmd.bak
rm busybox_cmd.bak                                                                                       
"#####;


const LUA_SCRIPT_GLIBC: &str = r#####"./busybox echo "#### OS COMP TEST GROUP START lua-glibc ####"
./busybox sh ./test.sh date.lua
./busybox sh ./test.sh file_io.lua
./busybox sh ./test.sh max_min.lua
./busybox sh ./test.sh random.lua
./busybox sh ./test.sh remove.lua
./busybox sh ./test.sh round_num.lua
./busybox sh ./test.sh sin30.lua
./busybox sh ./test.sh sort.lua
./busybox sh ./test.sh strings.lua
./busybox echo "#### OS COMP TEST GROUP END lua-glibc ####"
"#####;

const LUA_SCRIPT_MUSL: &str = r#####"./busybox echo "#### OS COMP TEST GROUP START lua-musl ####"
./busybox sh ./test.sh date.lua
./busybox sh ./test.sh file_io.lua
./busybox sh ./test.sh max_min.lua
./busybox sh ./test.sh random.lua
./busybox sh ./test.sh remove.lua
./busybox sh ./test.sh round_num.lua
./busybox sh ./test.sh sin30.lua
./busybox sh ./test.sh sort.lua
./busybox sh ./test.sh strings.lua
./busybox echo "#### OS COMP TEST GROUP END lua-musl ####"
"#####;



#[cfg(any(target_arch = "riscv64"))]
const LIBCTEST_SCRIPT_GLIBC: &str = r#####"./busybox echo "#### OS COMP TEST GROUP START libctest-glibc ####"
./runtest.exe -w entry-static.exe argv
./runtest.exe -w entry-static.exe basename
./runtest.exe -w entry-static.exe clocale_mbfuncs
./runtest.exe -w entry-static.exe clock_gettime
./runtest.exe -w entry-static.exe dirname
./runtest.exe -w entry-static.exe env
./runtest.exe -w entry-static.exe fdopen
./runtest.exe -w entry-static.exe fnmatch
./runtest.exe -w entry-static.exe fscanf
./runtest.exe -w entry-static.exe fwscanf
./runtest.exe -w entry-static.exe iconv_open
./runtest.exe -w entry-static.exe inet_pton
./runtest.exe -w entry-static.exe mbc
./runtest.exe -w entry-static.exe memstream
./runtest.exe -w entry-static.exe pthread_cancel_points
./runtest.exe -w entry-static.exe pthread_cancel
./runtest.exe -w entry-static.exe pthread_cond
./runtest.exe -w entry-static.exe pthread_tsd
#./runtest.exe -w entry-static.exe qsort
./runtest.exe -w entry-static.exe random
./runtest.exe -w entry-static.exe search_hsearch
./runtest.exe -w entry-static.exe search_insque
./runtest.exe -w entry-static.exe search_lsearch
./runtest.exe -w entry-static.exe search_tsearch
./runtest.exe -w entry-static.exe setjmp
./runtest.exe -w entry-static.exe snprintf
#./runtest.exe -w entry-static.exe socket
./runtest.exe -w entry-static.exe sscanf
./runtest.exe -w entry-static.exe sscanf_long
./runtest.exe -w entry-static.exe stat
./runtest.exe -w entry-static.exe strftime
./runtest.exe -w entry-static.exe string
./runtest.exe -w entry-static.exe string_memcpy
./runtest.exe -w entry-static.exe string_memmem
./runtest.exe -w entry-static.exe string_memset
./runtest.exe -w entry-static.exe string_strchr
./runtest.exe -w entry-static.exe string_strcspn
./runtest.exe -w entry-static.exe string_strstr
./runtest.exe -w entry-static.exe strptime
./runtest.exe -w entry-static.exe strtod
./runtest.exe -w entry-static.exe strtod_simple
./runtest.exe -w entry-static.exe strtof
./runtest.exe -w entry-static.exe strtol
./runtest.exe -w entry-static.exe strtold
./runtest.exe -w entry-static.exe swprintf
./runtest.exe -w entry-static.exe tgmath
./runtest.exe -w entry-static.exe time
./runtest.exe -w entry-static.exe tls_align
./runtest.exe -w entry-static.exe udiv
./runtest.exe -w entry-static.exe ungetc
./runtest.exe -w entry-static.exe utime
./runtest.exe -w entry-static.exe wcsstr
./runtest.exe -w entry-static.exe wcstol
#./runtest.exe -w entry-static.exe daemon_failure
./runtest.exe -w entry-static.exe dn_expand_empty
./runtest.exe -w entry-static.exe dn_expand_ptr_0
#./runtest.exe -w entry-static.exe fflush_exit
./runtest.exe -w entry-static.exe fgets_eof
./runtest.exe -w entry-static.exe fgetwc_buffering
./runtest.exe -w entry-static.exe fpclassify_invalid_ld80
./runtest.exe -w entry-static.exe ftello_unflushed_append
./runtest.exe -w entry-static.exe getpwnam_r_crash
./runtest.exe -w entry-static.exe getpwnam_r_errno
./runtest.exe -w entry-static.exe iconv_roundtrips
./runtest.exe -w entry-static.exe inet_ntop_v4mapped
./runtest.exe -w entry-static.exe inet_pton_empty_last_field
./runtest.exe -w entry-static.exe iswspace_null
./runtest.exe -w entry-static.exe lrand48_signextend
./runtest.exe -w entry-static.exe lseek_large
./runtest.exe -w entry-static.exe malloc_0
./runtest.exe -w entry-static.exe mbsrtowcs_overflow
./runtest.exe -w entry-static.exe memmem_oob_read
./runtest.exe -w entry-static.exe memmem_oob
./runtest.exe -w entry-static.exe mkdtemp_failure
./runtest.exe -w entry-static.exe mkstemp_failure
./runtest.exe -w entry-static.exe printf_1e9_oob
./runtest.exe -w entry-static.exe printf_fmt_g_round
./runtest.exe -w entry-static.exe printf_fmt_g_zeros
./runtest.exe -w entry-static.exe printf_fmt_n
#./runtest.exe -w entry-static.exe pthread_robust_detach
./runtest.exe -w entry-static.exe pthread_cancel_sem_wait
./runtest.exe -w entry-static.exe pthread_cond_smasher
#./runtest.exe -w entry-static.exe pthread_condattr_setclock
./runtest.exe -w entry-static.exe pthread_exit_cancel
./runtest.exe -w entry-static.exe pthread_once_deadlock
./runtest.exe -w entry-static.exe pthread_rwlock_ebusy
./runtest.exe -w entry-static.exe putenv_doublefree
./runtest.exe -w entry-static.exe regex_backref_0
./runtest.exe -w entry-static.exe regex_bracket_icase
./runtest.exe -w entry-static.exe regex_ere_backref
./runtest.exe -w entry-static.exe regex_escaped_high_byte
./runtest.exe -w entry-static.exe regex_negated_range
./runtest.exe -w entry-static.exe regexec_nosub
./runtest.exe -w entry-static.exe rewind_clear_error
./runtest.exe -w entry-static.exe rlimit_open_files
./runtest.exe -w entry-static.exe scanf_bytes_consumed
./runtest.exe -w entry-static.exe scanf_match_literal_eof
./runtest.exe -w entry-static.exe scanf_nullbyte_char
#./runtest.exe -w entry-static.exe setvbuf_unget
./runtest.exe -w entry-static.exe sigprocmask_internal
./runtest.exe -w entry-static.exe sscanf_eof
./runtest.exe -w entry-static.exe statvfs
./runtest.exe -w entry-static.exe strverscmp
./runtest.exe -w entry-static.exe syscall_sign_extend
./runtest.exe -w entry-static.exe uselocale_0
./runtest.exe -w entry-static.exe wcsncpy_read_overflow
./runtest.exe -w entry-static.exe wcsstr_false_negative
./busybox echo "#### OS COMP TEST GROUP END libctest-glibc ####"
"#####;
#[cfg(any(target_arch = "loongarch64"))]
const LIBCTEST_SCRIPT_GLIBC: &str = r#####"./busybox echo "#### OS COMP TEST GROUP START libctest-glibc ####"
./runtest.exe -w entry-static.exe argv
./runtest.exe -w entry-static.exe basename
./runtest.exe -w entry-static.exe clocale_mbfuncs
./runtest.exe -w entry-static.exe clock_gettime
./runtest.exe -w entry-static.exe dirname
./runtest.exe -w entry-static.exe env
./runtest.exe -w entry-static.exe fdopen
./runtest.exe -w entry-static.exe fnmatch
./runtest.exe -w entry-static.exe fscanf
./runtest.exe -w entry-static.exe fwscanf
./runtest.exe -w entry-static.exe iconv_open
./runtest.exe -w entry-static.exe inet_pton
./runtest.exe -w entry-static.exe mbc
./runtest.exe -w entry-static.exe memstream
#./runtest.exe -w entry-static.exe pthread_cancel_points
#./runtest.exe -w entry-static.exe pthread_cancel
#./runtest.exe -w entry-static.exe pthread_cond
#./runtest.exe -w entry-static.exe pthread_tsd
#./runtest.exe -w entry-static.exe qsort
./runtest.exe -w entry-static.exe random
./runtest.exe -w entry-static.exe search_hsearch
./runtest.exe -w entry-static.exe search_insque
./runtest.exe -w entry-static.exe search_lsearch
./runtest.exe -w entry-static.exe search_tsearch
./runtest.exe -w entry-static.exe setjmp
./runtest.exe -w entry-static.exe snprintf
#./runtest.exe -w entry-static.exe socket
./runtest.exe -w entry-static.exe sscanf
./runtest.exe -w entry-static.exe sscanf_long
./runtest.exe -w entry-static.exe stat
./runtest.exe -w entry-static.exe strftime
./runtest.exe -w entry-static.exe string
./runtest.exe -w entry-static.exe string_memcpy
./runtest.exe -w entry-static.exe string_memmem
./runtest.exe -w entry-static.exe string_memset
./runtest.exe -w entry-static.exe string_strchr
./runtest.exe -w entry-static.exe string_strcspn
./runtest.exe -w entry-static.exe string_strstr
./runtest.exe -w entry-static.exe strptime
./runtest.exe -w entry-static.exe strtod
./runtest.exe -w entry-static.exe strtod_simple
./runtest.exe -w entry-static.exe strtof
./runtest.exe -w entry-static.exe strtol
./runtest.exe -w entry-static.exe strtold
./runtest.exe -w entry-static.exe swprintf
./runtest.exe -w entry-static.exe tgmath
./runtest.exe -w entry-static.exe time
./runtest.exe -w entry-static.exe tls_align
./runtest.exe -w entry-static.exe udiv
#./runtest.exe -w entry-static.exe ungetc
./runtest.exe -w entry-static.exe utime
./runtest.exe -w entry-static.exe wcsstr
./runtest.exe -w entry-static.exe wcstol
#./runtest.exe -w entry-static.exe daemon_failure
./runtest.exe -w entry-static.exe dn_expand_empty
./runtest.exe -w entry-static.exe dn_expand_ptr_0
#./runtest.exe -w entry-static.exe fflush_exit
./runtest.exe -w entry-static.exe fgets_eof
./runtest.exe -w entry-static.exe fgetwc_buffering
./runtest.exe -w entry-static.exe fpclassify_invalid_ld80
./runtest.exe -w entry-static.exe ftello_unflushed_append
./runtest.exe -w entry-static.exe getpwnam_r_crash
./runtest.exe -w entry-static.exe getpwnam_r_errno
./runtest.exe -w entry-static.exe iconv_roundtrips
./runtest.exe -w entry-static.exe inet_ntop_v4mapped
./runtest.exe -w entry-static.exe inet_pton_empty_last_field
./runtest.exe -w entry-static.exe iswspace_null
./runtest.exe -w entry-static.exe lrand48_signextend
./runtest.exe -w entry-static.exe lseek_large
./runtest.exe -w entry-static.exe malloc_0
./runtest.exe -w entry-static.exe mbsrtowcs_overflow
./runtest.exe -w entry-static.exe memmem_oob_read
./runtest.exe -w entry-static.exe memmem_oob
./runtest.exe -w entry-static.exe mkdtemp_failure
./runtest.exe -w entry-static.exe mkstemp_failure
./runtest.exe -w entry-static.exe printf_1e9_oob
./runtest.exe -w entry-static.exe printf_fmt_g_round
./runtest.exe -w entry-static.exe printf_fmt_g_zeros
./runtest.exe -w entry-static.exe printf_fmt_n
#./runtest.exe -w entry-static.exe pthread_robust_detach
#./runtest.exe -w entry-static.exe pthread_cancel_sem_wait
#./runtest.exe -w entry-static.exe pthread_cond_smasher
#./runtest.exe -w entry-static.exe pthread_condattr_setclock
#./runtest.exe -w entry-static.exe pthread_exit_cancel
#./runtest.exe -w entry-static.exe pthread_once_deadlock
#./runtest.exe -w entry-static.exe pthread_rwlock_ebusy
./runtest.exe -w entry-static.exe putenv_doublefree
./runtest.exe -w entry-static.exe regex_backref_0
./runtest.exe -w entry-static.exe regex_bracket_icase
./runtest.exe -w entry-static.exe regex_ere_backref
./runtest.exe -w entry-static.exe regex_escaped_high_byte
./runtest.exe -w entry-static.exe regex_negated_range
./runtest.exe -w entry-static.exe regexec_nosub
./runtest.exe -w entry-static.exe rewind_clear_error
./runtest.exe -w entry-static.exe rlimit_open_files
./runtest.exe -w entry-static.exe scanf_bytes_consumed
./runtest.exe -w entry-static.exe scanf_match_literal_eof
./runtest.exe -w entry-static.exe scanf_nullbyte_char
#./runtest.exe -w entry-static.exe setvbuf_unget
./runtest.exe -w entry-static.exe sigprocmask_internal
./runtest.exe -w entry-static.exe sscanf_eof
./runtest.exe -w entry-static.exe statvfs
./runtest.exe -w entry-static.exe strverscmp
./runtest.exe -w entry-static.exe syscall_sign_extend
./runtest.exe -w entry-static.exe uselocale_0
./runtest.exe -w entry-static.exe wcsncpy_read_overflow
./runtest.exe -w entry-static.exe wcsstr_false_negative
./busybox echo "#### OS COMP TEST GROUP END libctest-glibc ####"
"#####;

const LIBCTEST_SCRIPT_MUSL: &str = r#####"./busybox echo "#### OS COMP TEST GROUP START libctest-musl ####"
./runtest.exe -w entry-static.exe argv
./runtest.exe -w entry-static.exe basename
./runtest.exe -w entry-static.exe clocale_mbfuncs
./runtest.exe -w entry-static.exe clock_gettime
./runtest.exe -w entry-static.exe dirname
./runtest.exe -w entry-static.exe env
./runtest.exe -w entry-static.exe fdopen
./runtest.exe -w entry-static.exe fnmatch
./runtest.exe -w entry-static.exe fscanf
./runtest.exe -w entry-static.exe fwscanf
./runtest.exe -w entry-static.exe iconv_open
./runtest.exe -w entry-static.exe inet_pton
./runtest.exe -w entry-static.exe mbc
./runtest.exe -w entry-static.exe memstream
#./runtest.exe -w entry-static.exe pthread_cancel_points
#./runtest.exe -w entry-static.exe pthread_cancel
#./runtest.exe -w entry-static.exe pthread_cond
#./runtest.exe -w entry-static.exe pthread_tsd
#./runtest.exe -w entry-static.exe qsort
./runtest.exe -w entry-static.exe random
./runtest.exe -w entry-static.exe search_hsearch
./runtest.exe -w entry-static.exe search_insque
./runtest.exe -w entry-static.exe search_lsearch
./runtest.exe -w entry-static.exe search_tsearch
./runtest.exe -w entry-static.exe setjmp
./runtest.exe -w entry-static.exe snprintf
#./runtest.exe -w entry-static.exe socket
./runtest.exe -w entry-static.exe sscanf
./runtest.exe -w entry-static.exe sscanf_long
./runtest.exe -w entry-static.exe stat
./runtest.exe -w entry-static.exe strftime
./runtest.exe -w entry-static.exe string
./runtest.exe -w entry-static.exe string_memcpy
./runtest.exe -w entry-static.exe string_memmem
./runtest.exe -w entry-static.exe string_memset
./runtest.exe -w entry-static.exe string_strchr
./runtest.exe -w entry-static.exe string_strcspn
./runtest.exe -w entry-static.exe string_strstr
./runtest.exe -w entry-static.exe strptime
./runtest.exe -w entry-static.exe strtod
./runtest.exe -w entry-static.exe strtod_simple
./runtest.exe -w entry-static.exe strtof
./runtest.exe -w entry-static.exe strtol
./runtest.exe -w entry-static.exe strtold
./runtest.exe -w entry-static.exe swprintf
./runtest.exe -w entry-static.exe tgmath
./runtest.exe -w entry-static.exe time
./runtest.exe -w entry-static.exe tls_align
./runtest.exe -w entry-static.exe udiv
#./runtest.exe -w entry-static.exe ungetc
./runtest.exe -w entry-static.exe utime
./runtest.exe -w entry-static.exe wcsstr
./runtest.exe -w entry-static.exe wcstol
#./runtest.exe -w entry-static.exe daemon_failure
./runtest.exe -w entry-static.exe dn_expand_empty
./runtest.exe -w entry-static.exe dn_expand_ptr_0
#./runtest.exe -w entry-static.exe fflush_exit
./runtest.exe -w entry-static.exe fgets_eof
./runtest.exe -w entry-static.exe fgetwc_buffering
./runtest.exe -w entry-static.exe fpclassify_invalid_ld80
./runtest.exe -w entry-static.exe ftello_unflushed_append
./runtest.exe -w entry-static.exe getpwnam_r_crash
./runtest.exe -w entry-static.exe getpwnam_r_errno
./runtest.exe -w entry-static.exe iconv_roundtrips
./runtest.exe -w entry-static.exe inet_ntop_v4mapped
./runtest.exe -w entry-static.exe inet_pton_empty_last_field
./runtest.exe -w entry-static.exe iswspace_null
./runtest.exe -w entry-static.exe lrand48_signextend
./runtest.exe -w entry-static.exe lseek_large
./runtest.exe -w entry-static.exe malloc_0
./runtest.exe -w entry-static.exe mbsrtowcs_overflow
./runtest.exe -w entry-static.exe memmem_oob_read
./runtest.exe -w entry-static.exe memmem_oob
./runtest.exe -w entry-static.exe mkdtemp_failure
./runtest.exe -w entry-static.exe mkstemp_failure
./runtest.exe -w entry-static.exe printf_1e9_oob
./runtest.exe -w entry-static.exe printf_fmt_g_round
./runtest.exe -w entry-static.exe printf_fmt_g_zeros
./runtest.exe -w entry-static.exe printf_fmt_n
#./runtest.exe -w entry-static.exe pthread_robust_detach
#./runtest.exe -w entry-static.exe pthread_cancel_sem_wait
#./runtest.exe -w entry-static.exe pthread_cond_smasher
#./runtest.exe -w entry-static.exe pthread_condattr_setclock
#./runtest.exe -w entry-static.exe pthread_exit_cancel
#./runtest.exe -w entry-static.exe pthread_once_deadlock
#./runtest.exe -w entry-static.exe pthread_rwlock_ebusy
./runtest.exe -w entry-static.exe putenv_doublefree
./runtest.exe -w entry-static.exe regex_backref_0
./runtest.exe -w entry-static.exe regex_bracket_icase
./runtest.exe -w entry-static.exe regex_ere_backref
./runtest.exe -w entry-static.exe regex_escaped_high_byte
./runtest.exe -w entry-static.exe regex_negated_range
./runtest.exe -w entry-static.exe regexec_nosub
./runtest.exe -w entry-static.exe rewind_clear_error
./runtest.exe -w entry-static.exe rlimit_open_files
./runtest.exe -w entry-static.exe scanf_bytes_consumed
./runtest.exe -w entry-static.exe scanf_match_literal_eof
./runtest.exe -w entry-static.exe scanf_nullbyte_char
#./runtest.exe -w entry-static.exe setvbuf_unget
./runtest.exe -w entry-static.exe sigprocmask_internal
./runtest.exe -w entry-static.exe sscanf_eof
./runtest.exe -w entry-static.exe statvfs
./runtest.exe -w entry-static.exe strverscmp
./runtest.exe -w entry-static.exe syscall_sign_extend
./runtest.exe -w entry-static.exe uselocale_0
./runtest.exe -w entry-static.exe wcsncpy_read_overflow
./runtest.exe -w entry-static.exe wcsstr_false_negative
./busybox echo "#### OS COMP TEST GROUP END libctest-musl ####"
"#####;