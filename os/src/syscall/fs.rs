//! File and filesystem-related syscalls
use crate::fs::open_file;
use crate::fs::make_pipe;
use crate::mm::{translated_refmut,translated_byte_buffer, translated_str};
use crate::task::{current_task, current_user_token};
use vfs_defs::{OpenFlags,UserBuffer};
//
use crate::config::PAGE_SIZE;
use crate::mm::frame_alloc_more;
use crate::mm::MapArea;
//use arch::addr::VirtPage;
use crate::mm::MapPermission;
use arch::pagetable::MappingSize;
use crate::mm::frame_dealloc;
use crate::mm::MapType;
use arch::addr::{PhysPage, VirtAddr, VirtPage};
use alloc::sync::Arc;
use alloc::vec::Vec;
//
//const HEAP_MAX: usize = 0;

pub fn sys_write(fd: usize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(translated_byte_buffer(token, buf, len)) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(translated_byte_buffer(token, buf, len)) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

pub fn sys_pipe(pipe: *mut usize) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    //let mut inner = task.acquire_inner_lock();
    let mut inner = task.inner_exclusive_access();
    //自身目录项
    let self_dentry = inner.cwd.clone();
    let (pipe_read, pipe_write) = make_pipe(self_dentry); //创建一个管道并获取其读端和写端
    let read_fd = inner.alloc_fd();
    inner.fd_table[read_fd] = Some(pipe_read);
    let write_fd = inner.alloc_fd();
    inner.fd_table[write_fd] = Some(pipe_write);
    // 文件描述符写回到应用地址空间
    *translated_refmut(token, pipe) = read_fd;
    *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
    0
}

pub fn sys_brk(mut new_brk:  usize) -> isize {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let cur_brk = task_inner.heap_top;
    //println!("sys_brk: heap_top = {}, stack_bottom = {} new_brk:{}",task_inner.heap_top,task_inner.stack_bottom,new_brk);
    if new_brk == 0 {
        return cur_brk as isize;
    }
    let mut is_align:bool = true;
    // new_brk->align to 4K
    let num = new_brk / PAGE_SIZE;
    if num*PAGE_SIZE < new_brk {
        new_brk = (num + 1)*PAGE_SIZE;
        is_align = false;
    }

    if new_brk > cur_brk {
        let user_stack_bottom = task_inner.stack_bottom;
        
        if new_brk >= user_stack_bottom { 
            return user_stack_bottom as isize;
        }
        // 确认新增虚拟页号范围
        //let cur_page = (cur_brk + PAGE_SIZE - 1) / PAGE_SIZE;
        //let new_page = (new_brk + PAGE_SIZE - 1) / PAGE_SIZE;
        let cur_page = cur_brk / PAGE_SIZE;
        let new_page = new_brk / PAGE_SIZE;
        let page_count = new_page - cur_page;

        let mut all_vpn = Vec::<VirtPage>::new();
        let mut all_ppn = Vec::<PhysPage>::new();
        if page_count > 0 {
            // 申请等量的物理页帧
            /*let frames = frame_alloc_more(page_count);
            if frames.is_none() {
                return -1; // 物理内存不足
            }
            let frames = frames.unwrap();*/

            // 在 memory_set 中映射新增的虚拟页号到物理页帧,如(31,32)->(31,30.9->31) 实际申请一页
            let _start_va: VirtAddr = (cur_brk).into();
            let _end_va: VirtAddr = (new_brk - PAGE_SIZE -1).into();
            
            for i in 0..page_count {
                let vpn = VirtPage::from(cur_page + i);
                //let ppn = frames[i].ppn;
                let mp = MapPermission::R | MapPermission::W | MapPermission::U;
                let ppn = task_inner.memory_set.map_page(vpn, mp, MappingSize::Page4KB);
                all_vpn.push(vpn);
                all_ppn.push(ppn);
                //println!("vpn:{} ppn:{}",vpn,ppn);
                /*task_inner.memory_set.map_page(
                    vpn,
                    ppn,
                    MapPermission::R | MapPermission::W | MapPermission::U,
                    MappingSize::Page4KB,
                );*/
            }
        }

        task_inner.heap_top = new_brk;
        for (vpn, ppn) in all_vpn.iter().zip(all_ppn.iter()) {
            println!("VPN: {:?}, PPN: {:?}", vpn, ppn);
        }
        0
    }
    else if new_brk == cur_brk {
        -3
    }
    else {// 不考虑不合理的减小
        // 确认需要释放的虚页号
        let cur_page = cur_brk / PAGE_SIZE;
        if !is_align { //newbrk已向上取整
            new_brk -= PAGE_SIZE;
        }
        let new_page = new_brk / PAGE_SIZE ; 
        let page_count = cur_page - new_page;
        
        if page_count > 0 {
            // 解除映射并释放物理页帧
            for i in 1..(page_count + 1) {
                let vpn = VirtPage::from(cur_page - i);
                task_inner.memory_set.unmap_page(vpn);
                    //frame_dealloc(ppn);
            }
        }

        task_inner.heap_top = new_brk ;

        0
    }
}