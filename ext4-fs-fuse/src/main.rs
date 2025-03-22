use clap::{App, Arg};
use ext4::{Ext4ImplFsType,Ext4Superblock,Ext4Inode,Ext4ImplFile};
use vfs::get_root_dentry;
use std::fs::{read_dir, File, OpenOptions};
use std::mem;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Once};
use std::sync::Mutex;
use vfs_defs::{Dentry, DentryState, DiskInodeType, File as OtherFile, FileInner, FileSystemType, SuperBlock, SuperBlockInner,OpenFlags};
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
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    for app in apps {
        // load app data from host file system
        println!("{}",app);
        if app == String::from("mnt"){
            continue;
        }
        let mut host_file = File::open(format!("{}{}", target_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in ext4
        let den =root_dentry.create(app.as_str(),DiskInodeType::File).unwrap();
        let inode = den.get_inode().unwrap().get_meta().ino;
        // write data to ext4
        sb.ext4fs.ext4_file_write(inode as u64, 0, all_data.as_slice());
    }
    println!("app:----");
    // list apps
     for app in root_dentry.clone().ls() {
         println!("{}", app);
     }
    drop(root_dentry);
    block_cache_sync_all();
    Ok(())
}

#[test]
fn efs_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/ext4.img")?;
        f.set_len(8192 * 1024 * 1024).unwrap();
        f
    })));
    device::BLOCK_DEVICE.call_once(||block_file);
    vfs::init();
    let root_dentry = get_root_dentry();
    let sb = root_dentry.get_superblock().downcast_arc::<Ext4Superblock>().map_err(|_| SysError::ENOENT).unwrap();
    root_dentry.create("filea", DiskInodeType::File);
    root_dentry.create("fileb", DiskInodeType::File);
    for name in root_dentry.clone().ls(){
        println!("{}",name);
    }
    let filea = root_dentry.get_child("filea").unwrap().get_inode().unwrap().get_meta().ino;
    let greet_str = "Hello, world!";
    sb.ext4fs.ext4_file_write(filea as u64, 0, greet_str.as_bytes());
    //let mut buffer = [0u8; 512];



    let fileb = root_dentry.get_child("fileb").unwrap().get_inode().unwrap().get_meta().ino;
    let greet_str = "Hello, world1!";
    sb.ext4fs.ext4_file_write(fileb as u64, 0, greet_str.as_bytes());
    //let mut buffer = [0u8; 512];

    let filea = root_dentry.get_child("filea").unwrap().get_inode().unwrap().get_meta().ino;
    let greet_str = "Hello, world!";
    let mut buffer = [0u8;233];
    let len = sb.ext4fs.read_at(filea as u32, 0,&mut buffer).unwrap();
    let len = greet_str.len();
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap(),);
    drop(filea);

    let fileb = root_dentry.get_child("fileb").unwrap().get_inode().unwrap().get_meta().ino;
    let mut buffer = [0u8; 233];
    let greet_str = "Hello, world1!";
    let len = sb.ext4fs.read_at(fileb as u32, 0,&mut buffer).unwrap();
    let len = greet_str.len();
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap(),);
    drop(fileb);
   // return Ok(());
    let filea = sb.ext4fs.ext4_file_open("/filea", "w+");
    let mut random_str_test = |len: usize| {
        sb.ext4fs.file_remove("/filea");
        let filea = sb.ext4fs.ext4_file_open("/filea", "w+").unwrap();

        let mut str = String::new();
        use rand;
        // random digit
        for _ in 0..len {
            str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
        }
        let l = sb.ext4fs.write_at(filea as u32,0, str.as_bytes());
        if l.unwrap() != str.len(){
            panic!("l.unwrap() != str.len() {} {}",l.unwrap(),str.len());
        }
        let mut read_buffer = vec![0u8; str.len()];
        let mut offset = 0usize;
        let mut read_str = String::new();
        sb.ext4fs.read_at(filea as u32, 0, &mut read_buffer);
            //let v = sb.ext4fs.ext4_file_read(filea as u64, str.len() as u32,0).unwrap();
            read_str.push_str(core::str::from_utf8(&read_buffer[..str.len()]).unwrap());
        assert_eq!(str, read_str);
    };

    random_str_test(4 * BLOCK_SZ);
    random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
    random_str_test(100 * BLOCK_SZ);
    random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
    random_str_test((12 + 128) * BLOCK_SZ);
    random_str_test(400 * BLOCK_SZ);
    random_str_test(1000 * BLOCK_SZ);
    random_str_test(2000 * BLOCK_SZ);
    Ok(())
}
