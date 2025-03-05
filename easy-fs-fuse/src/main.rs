use clap::{App, Arg};
use easy_fs::{inode_cache_sync_all, BlockDevice, EasyFileSystem, EfsSuperBlock,EfsFsType,EfsInode};
use vfs::get_root_dentry;
use std::fs::{read_dir, File, OpenOptions};
use std::mem;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Once};
use std::sync::Mutex;
use vfs_defs::{Dentry, DentryState, DiskInodeType, File as OtherFile, FileInner, FileSystemType, SuperBlock, SuperBlockInner};
use system_result::{SysError,SysResult};
use easy_fs::EfsFile;
const BLOCK_SZ: usize = 512;

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
    fn handle_irq(&self) {
        unimplemented!();
    }
}


fn main() {
   easy_fs_pack().expect("Error when packing easy-fs!");                                                       
}

fn easy_fs_pack() -> std::io::Result<()> {
    let matches = App::new("EasyFileSystem packer")
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
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(16 * 2048 * 512).unwrap();
        f
    })));
    
    // 16MiB, at most 4095 files
    let efs = EasyFileSystem::create(block_file.clone(), 16 * 2048, 1);
    
    device::BLOCK_DEVICE.call_once(||block_file);
    vfs::init();
    let root_inode = get_root_dentry();
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
        let mut host_file = File::open(format!("{}{}", target_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in easy-fs
        let den = root_inode.create(app.as_str(),DiskInodeType::File).unwrap();
        let inode = den.get_inode().unwrap().downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR).unwrap();
        // write data to easy-fs
        inode.write_at(0, all_data.as_slice());
    }
    // list apps
     for app in root_inode.clone().ls() {
         println!("{}", app);
     }
    root_inode.get_inner().children.lock().clear();
    *root_inode.get_inner().inode.lock() = None;
    drop(root_inode);
    inode_cache_sync_all();
    Ok(())
}

#[test]
fn efs_test() -> std::io::Result<()> {
    let size = mem::size_of::<easy_fs::DiskInode>();
    assert_eq!(size,128);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(8192 * 512).unwrap();
        f
    })));
    EasyFileSystem::create(block_file.clone(), 4096, 1);
  //  let efs = EasyFileSystem::open(block_file.clone());
    device::BLOCK_DEVICE.call_once(||block_file);
    vfs::init();
    let root_dentry = get_root_dentry();
    root_dentry.create("filea", DiskInodeType::File);
    root_dentry.create("fileb", DiskInodeType::File);
    for (name,child) in (*root_dentry.get_inner().children.lock()).iter(){
        println!("{}",name);
    }
    
 //   Ok(())
    
    let filea = EfsFile::new(true, true, FileInner::new(root_dentry.lookup("filea").unwrap()));
    let greet_str = "Hello, world!";
    filea.write(greet_str.as_bytes());
    //let mut buffer = [0u8; 512];
    drop(filea);

    let fileb = EfsFile::new(true, true, FileInner::new(root_dentry.lookup("fileb").unwrap()));
    let greet_str = "Hello, world1!";
    fileb.write(greet_str.as_bytes());
    //let mut buffer = [0u8; 512];
    drop(fileb);

    let filea = EfsFile::new(true, true, FileInner::new(root_dentry.lookup("filea").unwrap()));
    let greet_str = "Hello, world!";
    let mut buffer = [0u8; 233];
    let len = filea.read(&mut buffer);
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap(),);
    drop(filea);

    let fileb = EfsFile::new(true, true, FileInner::new(root_dentry.lookup("fileb").unwrap()));
    buffer = [0u8; 233];
    let greet_str = "Hello, world1!";
    let len = fileb.read(&mut buffer);
    assert_eq!(greet_str, core::str::from_utf8(&buffer[..len]).unwrap(),);
    drop(fileb);

    let filea = EfsFile::new(true, true, FileInner::new(root_dentry.lookup("filea").unwrap()));
    let mut random_str_test = |len: usize| {
        filea.get_dentry().get_inode().unwrap().downcast_arc::<EfsInode>().map_err(|_| SysError::ENOTDIR).unwrap().clear();
        assert_eq!(filea.read_at(0, &mut buffer), 0,);
        let mut str = String::new();
        use rand;
        // random digit
        for _ in 0..len {
            str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
        }
        filea.write_at(0, str.as_bytes());
        let mut read_buffer = [0u8; 127];
        let mut offset = 0usize;
        let mut read_str = String::new();
        loop {
            let len = filea.read_at(offset, &mut read_buffer);
            if len == 0 {
                break;
            }
            offset += len;
            read_str.push_str(core::str::from_utf8(&read_buffer[..len]).unwrap());
        }
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
