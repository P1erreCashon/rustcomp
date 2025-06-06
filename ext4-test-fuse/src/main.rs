use clap::{App, Arg};
use ext4::{Ext4ImplFsType,Ext4Superblock,Ext4Inode,Ext4ImplFile};
use vfs::get_root_dentry;
use std::fs::{read_dir, File, OpenOptions};
use std::mem;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Once};
use std::sync::Mutex;
use vfs_defs::{dcache_drop, dcache_lookup, Dentry, DentryState, DiskInodeType, File as OtherFile, FileInner, FileSystemType, OpenFlags, SuperBlock, SuperBlockInner,dcache_sync_call};
use system_result::{SysError,SysResult};
use device::BlockDevice;
use buffer::block_cache_sync_all;
const BLOCK_SZ: usize = 512;
use crate_interface::impl_interface;
#[macro_use]
extern crate logger;
use logger::*;
use log::Record;
struct LogIfImpl;

#[impl_interface]
impl LogIf for LogIfImpl{
    fn print_log(record: &Record){
        println!("{}: {}", record.level(), record.args());
    }
}
#[derive(Debug)]
struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        let len = file.read(buf).unwrap();
        if len != BLOCK_SZ{
            println!("blockid:{}",block_id);
        }
        assert_eq!(len, BLOCK_SZ, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }
 //   fn handle_irq(&self) {
 //       unimplemented!();
 //   }
}

fn main() {
   easy_fs_pack().expect("Error when packing ext4!");      
//rv_pack().expect("Error when packing ext4!");                                                   
}

fn rv_pack() -> std::io::Result<()> {
    let matches = App::new("Ext4 packer")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .help("Executable target dir(with backslash)"),
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
         //   .open("ext4.img")?;
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(8192 * 1024 * 1024).unwrap();
        f
    })));
    
    
    device::BLOCK_DEVICE.call_once(||block_file);
    logger::init_logger();
    vfs::init();
    let root_dentry = get_root_dentry();
    let sb = root_dentry.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT).unwrap();
    let apps: Vec<_> = read_dir(src_path)
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
   //         name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
     

    for app in apps {
        // load app data from host file system
        println!("{}",app);
        if app != String::from("user_shell"){
            continue;
        }
        let mut host_file = File::open(format!("{}{}", src_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in ext4
        let den =root_dentry.create(app.as_str(),DiskInodeType::File).unwrap();
        let inode = den.get_inode().unwrap().get_meta().ino;
        // write data to ext4
        sb.ext4fs.ext4_file_write(inode as u64, 0, all_data.as_slice());
    }
    {
    let mut host_file = File::open(format!("{}{}", target_path, "initproc")).unwrap();
    let mut all_data: Vec<u8> = Vec::new();
    host_file.read_to_end(&mut all_data).unwrap();
    // create a file in ext4
    let den =root_dentry.create("initproc",DiskInodeType::File).unwrap();
    let inode = den.get_inode().unwrap().get_meta().ino;
    // write data to ext4
    sb.ext4fs.ext4_file_write(inode as u64, 0, all_data.as_slice());
    }
    println!("app:----");
    drop(root_dentry);
    dcache_sync_call();
    dcache_drop();
    block_cache_sync_all();    
    Ok(())
}


fn easy_fs_pack() -> std::io::Result<()> {
    let matches = App::new("Ext4 packer")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .help("Executable target dir(with backslash)"),
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
         //   .open("ext4.img")?;
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(8192 * 1024 * 1024).unwrap();
        f
    })));
    
    
    device::BLOCK_DEVICE.call_once(||block_file);
    logger::init_logger();
    vfs::init();
    let root_dentry = get_root_dentry();
    let sb = root_dentry.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT).unwrap();
    let apps: Vec<_> = read_dir(src_path)
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
   //         name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    let mnt = root_dentry.create("mnt",DiskInodeType::Directory).unwrap();
     let lib = root_dentry.create("lib",DiskInodeType::Directory).unwrap();
     
     let mntapps: Vec<_> = read_dir(format!("{}{}", src_path, "mnt/"))
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
        //         name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
         .collect();
    let libapps: Vec<_> = read_dir(format!("{}{}", src_path, "lib/"))
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
        //         name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
         .collect();
    for app in mntapps {
            // load app data from host file system
            println!("{}",app);
            let mut host_file = File::open(format!("{}{}{}", src_path,"mnt/", app)).unwrap();
            let mut all_data: Vec<u8> = Vec::new();
            host_file.read_to_end(&mut all_data).unwrap();
            // create a file in ext4
            let den =mnt.create(app.as_str(),DiskInodeType::File).unwrap();
            let inode = den.get_inode().unwrap().get_meta().ino;
            // write data to ext4
            sb.ext4fs.ext4_file_write(inode as u64, 0, all_data.as_slice());
        }
    for app in libapps {
            // load app data from host file system
            println!("{}",app);
            let mut host_file = File::open(format!("{}{}{}", src_path,"lib/", app)).unwrap();
            let mut all_data: Vec<u8> = Vec::new();
            host_file.read_to_end(&mut all_data).unwrap();
            // create a file in ext4
            let den =lib.create(app.as_str(),DiskInodeType::File).unwrap();
            let inode = den.get_inode().unwrap().get_meta().ino;
            // write data to ext4
            sb.ext4fs.ext4_file_write(inode as u64, 0, all_data.as_slice());
        }
    for app in apps {
        // load app data from host file system
        println!("{}",app);
        if app == String::from("mnt") || app == String::from("lib"){
            continue;
        }
        let mut host_file = File::open(format!("{}{}", src_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in ext4
        let den =root_dentry.create(app.as_str(),DiskInodeType::File).unwrap();
        let inode = den.get_inode().unwrap().get_meta().ino;
        // write data to ext4
        sb.ext4fs.ext4_file_write(inode as u64, 0, all_data.as_slice());
    }
    println!("app:----");
    drop(mnt);
    drop(lib);
    drop(root_dentry);
    dcache_sync_call();
    dcache_drop();
    block_cache_sync_all();    
    Ok(())
}