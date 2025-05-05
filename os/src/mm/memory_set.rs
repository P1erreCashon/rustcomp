//! Implementation of [`MapArea`] and [`MemorySet`].
use core::fmt::Debug;

use super::{frame_alloc, vpn_range, FrameTracker};
//use super::{PTEFlags, PageTable, PageTableEntry};
//use super::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::vpn_range::VPNRange;
use alloc::alloc::dealloc;
use arch::pagetable::{MappingFlags, MappingSize, PageTable, PageTableWrapper};
use arch::addr::{PhysAddr, PhysPage, VirtAddr, VirtPage};
use vfs_defs::File;
use crate::fs::path_to_dentry;
use arch::{TrapType, PAGE_SIZE, USER_VADDR_END};
use config::{USER_HEAP_SIZE, USER_MMAP_TOP, USER_STACK_SIZE, USER_STACK_TOP,DL_INTERP_OFFSET};
use crate::sync::UPSafeCell;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::{String,ToString};
use spin::Mutex;
use system_result::{SysError,SysResult};

const MODULE_LEVEL:log::Level = log::Level::Debug;
/*
lazy_static! { 
/* a memory set instance through lazy_static! managing kernel space    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) }); */
        ///a memory set instance through lazy_static! and Mutex managing kernel space
        pub static ref KERNEL_SPACE:Mutex<MemorySet> = 
        Mutex::new(MemorySet::new_kernel());
}
///Get kernelspace root ppn
pub fn kernel_token() -> usize {
    KERNEL_SPACE.lock().token()
} */

bitflags! {
    // Defined in <bits/mman-linux.h>
    #[derive(Default)]
    pub struct MmapFlags: i32 {
        // Sharing types (must choose one and only one of these).
        /// Share changes.
        const MAP_SHARED = 0x01;
        /// Changes are private.
        const MAP_PRIVATE = 0x02;
        /// Share changes and validate
        const MAP_SHARED_VALIDATE = 0x03;
        const MAP_TYPE_MASK = 0x03;

        // Other flags
        /// Interpret addr exactly.
        const MAP_FIXED = 0x10;
        /// Don't use a file.
        const MAP_ANONYMOUS = 0x20;
        /// Don't check for reservations.
        const MAP_NORESERVE = 0x04000;
    }
}
/// memory set structure, controls virtual-memory space
#[derive(Clone)]
pub struct MemorySet {
    ///
    pub page_table: Arc<PageTableWrapper>,
    ///
    pub areas: Vec<MapArea>,
    ///
    pub mapareacontrol: MapAreaControl,
}

impl MemorySet {
    ///Create an empty `MemorySet`
    pub fn new_bare() -> Self {
        Self {
            page_table:Arc::new(PageTableWrapper::alloc()),
            areas: Vec::new(),
            mapareacontrol:MapAreaControl::new()
        }
    }
    ///Get pagetable `root_ppn`
    pub fn token(&self) -> PageTable {
        self.page_table.0
    }
    ///
    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }
    pub fn push_into_heaparea_lazy_while_clone(&mut self,mut map_area: MapArea) { 
        for vpn in map_area.vpn_range {
            //self.map_one(page_table, vpn);
            if self.page_table.translate(vpn.into()).is_some(){
                map_area.map_one(&self.page_table, vpn);
            }  
        }
        self.areas.push(map_area);
    }
    /// 
    pub fn push_into_area_lazy(&mut self, map_area: MapArea) { 
        self.areas.push(map_area);
    }
    pub fn handle_lazy_addr(&mut self,addr:usize,_type:TrapType)->SysResult<isize>{
        if let Some((ppn,_mp)) = self.translate(VirtPage::new(addr/PAGE_SIZE)){
            if ppn.to_addr() != 0{
                return Err(SysError::EADDRINUSE);
            }
        }
        for area in self.areas.iter_mut(){
            if area.area_type == MapAreaType::Heap && area.vpn_range.get_start().to_addr() <= addr && area.vpn_range.get_end().to_addr() > addr{
                area.map_one(&self.page_table, VirtPage::new(addr/PAGE_SIZE));
                return Ok(0);
            }
        }
        for area in self.areas.iter_mut(){
       //     if addr == 0x1108015000{
        //        println!("handle:{:x} {:x}",area.vpn_range.get_start().to_addr(),area.vpn_range.get_end().to_addr());
        //    }
            if area.area_type == MapAreaType::Mmap && area.vpn_range.get_start().to_addr() <= addr && area.vpn_range.get_end().to_addr() > addr{
                area.map_one(&self.page_table, VirtPage::new(addr/PAGE_SIZE));
                if area.map_file.is_some(){
                    let off = addr - (addr%PAGE_SIZE) - area.vpn_range.get_start().to_addr();
                    let mut buf = vec![0u8;PAGE_SIZE];
                    let file = area.map_file.clone().unwrap();
                    file.read_at(off, &mut buf);
                    let dst_ppn = area.data_frames.get(&VirtPage::new(addr/PAGE_SIZE)).unwrap().ppn;
                    dst_ppn.get_buffer().copy_from_slice(&buf); 
                }
                return Ok(0);
            }
        }
        return Err(SysError::EADDRNOTAVAIL);
    }
    pub fn handle_cow_addr(&mut self,addr:usize)->SysResult<isize>{
        for area in self.areas.iter_mut(){
            if area.vpn_range.get_start().to_addr() <= addr && area.vpn_range.get_end().to_addr() > addr{
                if let Some((_ppn,mut mp)) = self.page_table.translate(VirtAddr::from(addr)){
                    if mp.contains(MappingFlags::cow){
                        let vpn = VirtPage::new(addr/PAGE_SIZE);
                        let frame = area.data_frames.get(&vpn).unwrap();
                        if Arc::strong_count(frame) == 1{
                            mp |= MappingFlags::W;
                            mp &= !MappingFlags::cow;
                            self.page_table.map_page(vpn, frame.ppn, mp.into(), MappingSize::Page4KB);
                            return Ok(0);
                        }
                        let src_ppn = area.data_frames.get(&vpn).unwrap().ppn;
                        area.unmap_one(&self.page_table, vpn);
                        area.map_one(&self.page_table, vpn);
                        let dst_ppn = area.data_frames.get(&vpn).unwrap().ppn;
                        dst_ppn.get_buffer().copy_from_slice(src_ppn.get_buffer());
                        mp |= MappingFlags::W;
                        mp &= !MappingFlags::cow;
                        self.page_table.map_page(vpn, dst_ppn, mp.into(), MappingSize::Page4KB);
                        return Ok(0);
                    }
                }
            }
        }
        return Err(SysError::EADDRNOTAVAIL);
    }
    pub fn split_vpn_range(&mut self,start: VirtPage,end: VirtPage){
        let mut new_areas = Vec::new();
        for area in self.areas.iter_mut() {
            let area_start = area.vpn_range.get_start();
            let area_end = area.vpn_range.get_end();
            if area_start >= start && area_end <= end {
                continue;
            } else if area_start < start && area_end > start && area_end <= end {
                //修改area后半部分
                let mut new_area = MapArea::from_another(area);
                new_area.vpn_range = VPNRange::new(start, area_end,start.into(),area.vpn_range.end);
                area.vpn_range = VPNRange::new(area_start, start,area.vpn_range.start,start.into());
                while !area.data_frames.is_empty() {
                    let page = area.data_frames.pop_last().unwrap();
                    new_area.data_frames.insert(page.0, page.1);
                    if page.0 == start {
                        break;
                    }
                }
                new_areas.push(new_area);
                continue;
            } else if area_start >= start && area_start < end && area_end > end {
                //修改area前半部分
                let mut new_area = MapArea::from_another(area);
                new_area.vpn_range = VPNRange::new(area_start, end,area.vpn_range.start,end.into());
                area.vpn_range = VPNRange::new(end, area_end,end.into(),area.vpn_range.end);
                while !area.data_frames.is_empty() {
                    let page = area.data_frames.pop_first().unwrap();
                    if page.0 >= end {
                        area.data_frames.insert(page.0, page.1);
                        break;
                    }
                    new_area.data_frames.insert(page.0, page.1);
                }

                new_areas.push(new_area);
                continue;
            } else if area_start < start && area_end > end {
                //修改area中间部分
                let mut front_area = MapArea::from_another(area);
                let mut back_area = MapArea::from_another(area);
                front_area.vpn_range = VPNRange::new(area_start, start,area.vpn_range.start,start.into());
                back_area.vpn_range = VPNRange::new(end, area_end,end.into(),area.vpn_range.end);
                area.vpn_range = VPNRange::new(start, end,start.into(),end.into());
                while !area.data_frames.is_empty() {
                    let page = area.data_frames.pop_first().unwrap();
                    if page.0 >= start {
                        area.data_frames.insert(page.0, page.1);
                        break;
                    }
                    front_area.data_frames.insert(page.0, page.1);
                }
                while !area.data_frames.is_empty() {
                    let page = area.data_frames.pop_last().unwrap();
                    if page.0 < end {
                        area.data_frames.insert(page.0, page.1);
                        break;
                    }
                    back_area.data_frames.insert(page.0, page.1);
                }

                new_areas.push(front_area);
                new_areas.push(back_area);
            }
            //剩下的情况无相交部分，无需修改
        }
        for area in new_areas {
            self.areas.push(area);
        }
    }
    pub fn mprotect(&mut self,start: VirtPage,end: VirtPage,perm: MapPermission)->SysResult<isize>{
        self.split_vpn_range(start, end);
        for area in self.areas.iter_mut() {
            let area_start = area.vpn_range.get_start();
            let area_end = area.vpn_range.get_end();
            if area_start >= start && area_end <= end {
                //修改整个area
                if area.map_perm != perm{
                    area.map_perm = perm;
                    for (vpn, frame) in area.data_frames.iter() {
                        self.page_table.map_page(*vpn, frame.ppn, area.map_perm.into(), arch::pagetable::MappingSize::Page4KB);
                    }
                }
                
                continue;
            }
           
        }
        Ok(0)
    }    
    pub fn munmap(&mut self,_start:usize,len:usize)->SysResult<isize>{
   //     println!("unmap:{:x} {:x}",_start,_start+len);
        let start = VirtPage::from(VirtAddr::from(_start));
        let end = VirtPage::from(VirtAddr::from(_start + len));
        self.split_vpn_range(start, end);
         while let Some((idx, area)) = self.areas.iter_mut().enumerate()
        .filter(|(_, area)| area.area_type == MapAreaType::Mmap)
        .find(|(_, area)| {area.vpn_range.get_start() >= start && area.vpn_range.get_end() <= end}){
          
            if area.mmap_flag.contains(MmapFlags::MAP_SHARED) && area.map_perm.contains(MapPermission::W){
                let file = area.map_file.clone().unwrap();
                VPNRange::new(start, end, start.into(), end.into()).into_iter().for_each(
                    |vpn|{
                        if area.data_frames.contains_key(&vpn){
                            let frame = area.data_frames.get_mut(&vpn).unwrap();
                            let off = vpn.to_addr() - area.vpn_range.get_start().to_addr();
                            file.write_at(off, frame.ppn.get_buffer());    
                        }
                        area.unmap_one(&self.page_table, vpn);
                    }
                );
                
            }
       //     println!("remove:{:x} {:x}",area.vpn_range.get_start_addr().addr(),area.vpn_range.get_end_addr().addr());
            self.areas.remove(idx);
        }
        Ok(0)
    }
    pub fn load_interp(&mut self,elf_data: &[u8]) -> Option<usize>{
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let ph_count = elf_header.pt2.ph_count();
        let mut is_dl = false;
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Interp{
                    is_dl = true;
                    break;
            }
        }
        if is_dl{
            let section = elf.find_section_by_name(".interp").unwrap();
            let mut interp = String::from_utf8(section.raw_data(&elf).to_vec()).unwrap();
            interp = interp.strip_suffix("\0").unwrap_or(&interp).to_string();

            let interps: Vec<String> = vec![interp.clone()];

            let mut interp_dentry: system_result::SysResult<Arc<dyn vfs_defs::Dentry>> = Err(system_result::SysError::ENOENT);
            for interp in interps.into_iter() {
              //  if interp == String::from("/lib/ld-linux-riscv64-lp64.so.1"){
               //     interp = String::from("/lib/libc.so");
               // }
                
                if let Ok(dentry) = path_to_dentry(interp.as_str()) {
                    interp_dentry = Ok(dentry);
                    break;
                }
            }
            if interp_dentry.is_err(){
                return None;
            }
            let interp_dentry: Arc<dyn vfs_defs::Dentry> = interp_dentry.unwrap();
            let interp_file = interp_dentry.open(vfs_defs::OpenFlags::RDONLY);
            let interp_elf_data = interp_file.read_all();
            let interp_elf = xmas_elf::ElfFile::new(&interp_elf_data).unwrap();

            self.map_elf( &interp_elf, DL_INTERP_OFFSET.into());

            Some(interp_elf.header.pt2.entry_point() as usize + DL_INTERP_OFFSET)
        }
        else{
            None
        }
    }
    fn map_elf(&mut self,elf:& xmas_elf::ElfFile,offset:usize)->(u64,usize,u64,u16,usize){//(max_virt_mem,header_va,tls_addr,ph_count)
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut tls_addr:u64 = 0;
  //      let mut max_end_vpn = VirtPage::new(0);
        let mut max_virt_mem = 0;
        let mut header_va = 0;
        let mut found_header_va = false;
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Tls{
                tls_addr = ph.virtual_addr() + offset as u64;
            }
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize + offset).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize + offset).into();                
                if !found_header_va {
                    header_va = start_va.addr();
                    found_header_va = true;
                }
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm,MapAreaType::Elf);
              //  max_end_vpn = map_area.vpn_range.get_end();
                self.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
                // 最大段地址
                //let section_end = align_up(ph.virtual_addr() + ph.mem_size(), PAGE_SIZE);
                let mut section_end = ph.virtual_addr() + ph.mem_size() + offset as u64;
                let pn: u64 = section_end / PAGE_SIZE as u64;
                if pn * PAGE_SIZE as u64 != section_end {
                    section_end = (pn + 1) * PAGE_SIZE as u64;
                }
                max_virt_mem = max_virt_mem.max(section_end);
            }
        }
        (max_virt_mem,header_va,tls_addr,ph_count,header_va + elf_header.pt2.ph_offset() as usize)
    }
    /// Include sections in elf and trampoline and TrapContext and user stack,
    /// also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize, usize,u16,u16,u64,usize) {
        let mut memory_set = Self::new_bare();
        // map trampoline
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let (max_virt_mem,_header_va,tls_addr,ph_count,phdr) = memory_set.map_elf(&elf, 0);

        // 为程序映像转储 elf 程序头

        let heap_start:usize =  max_virt_mem.try_into().unwrap();
        let heap_top: usize = heap_start + USER_HEAP_SIZE;
        memory_set.push_into_area_lazy(
            MapArea::new(
                heap_start.into(),
                heap_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
                MapAreaType::Heap
            ),
        );
        // map user stack with U flags
        // let max_end_va: VirtAddr = max_end_vpn.into();
        
        // guard page
        let user_stack_top = USER_STACK_TOP; //8G
        let user_stack_bottom = user_stack_top - USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
                MapAreaType::Stack
            ),
            None,
        );
        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
            heap_top,
            elf.header.pt2.ph_entry_size(),
            ph_count,
            tls_addr,
            phdr
        )
    }
    /// 打印memset
    pub fn show(&self) {
        println!("\nareas");
        for area in &self.areas {
            println!("range {}-{}",area.vpn_range.get_start(),area.vpn_range.get_end());
            for (vpn,frame) in &area.data_frames {
                println!("{:x} {} {} arc:{}",vpn.value(),frame.ppn,self.page_table.get_pte_flags(*vpn).bits(),Arc::strong_count(&frame));
            }
        }
    }
    ///Clone a same `MemorySet`
    pub fn from_existed_user(user_space: &MemorySet) -> MemorySet {
        let mut memory_set = Self::new_bare();
        // map trampoline
        // copy data sections/trap_context/user_stack
        memory_set.mapareacontrol = user_space.mapareacontrol.clone();
        let pagetable = memory_set.page_table.clone();
        for area in user_space.areas.iter() {
            if area.area_type == MapAreaType::Heap || area.area_type == MapAreaType::Mmap{
                let mut new_area = MapArea::from_another(area);
                new_area.data_frames = area.data_frames.clone();
                for vpn in area.vpn_range {
                    //self.map_one(page_table, vpn);
                    if let Some((ppn,_mp)) = user_space.translate(vpn.into()){
                        if ppn.to_addr() != 0 {
                            let mut pte = user_space.page_table.get_pte_flags(vpn);
                            if pte.contains(MappingFlags::W) || pte.contains(MappingFlags::cow){
                                pte |= MappingFlags::cow;
                                pte &= !MappingFlags::W;
                            }
                            user_space.page_table.map_page(vpn, ppn, pte.into(), MappingSize::Page4KB);
                            pagetable.map_page(vpn, ppn, pte.into(), MappingSize::Page4KB);
                        }
                }     
                }
                memory_set.areas.push(new_area);
                continue;
            }
            let mut new_area = MapArea::from_another(area);
            new_area.data_frames = area.data_frames.clone();
            for (vpn,frame) in new_area.data_frames.iter(){
                let mut pte = user_space.page_table.get_pte_flags(*vpn);
                if pte.contains(MappingFlags::W) || pte.contains(MappingFlags::cow){
                    pte |= MappingFlags::cow;
                    pte &= !MappingFlags::W;
                }
                user_space.page_table.map_page(*vpn, frame.ppn, pte.into(), MappingSize::Page4KB);
                pagetable.map_page(*vpn, frame.ppn, pte.into(), MappingSize::Page4KB);
            }
            /* 
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().0;
                let dst_ppn = memory_set.translate(vpn).unwrap().0;
              //  dst_ppn
                //    .get_bytes_array()
                  //  .copy_from_slice(src_ppn.get_bytes_array());
                  dst_ppn.get_buffer().copy_from_slice(src_ppn.get_buffer())
            }*/
            memory_set.areas.push(new_area);
        }
        memory_set
    }
    ///Refresh TLB with `sfence.vma`
    pub fn activate(&self) {
        self.page_table.change();
    }
    ///Translate throuth pagetable
    pub fn translate(&self, vpn: VirtPage) -> Option<(PhysPage, MappingFlags)> {
        self.page_table
            .translate(vpn.into())
            .map(|(pa, flags)| (pa.into(), flags))
    }
    ///Remove all `MapArea`
    pub fn recycle_data_pages(&mut self) {
        //*self = Self::new_bare();
        self.areas.clear();
    }
    /// 用于munmap
    pub fn remove_map_area_by_vpn_start(&mut self, num: VirtPage) -> i32 {
        if let Some(pos) = self.areas.iter().position(|map_area| map_area.area_type == MapAreaType::Mmap&& map_area.vpn_range.get_start() == num) {
        //    println!("remove vpn: {}~{}",self.mmap_area[pos].vpn_range.get_start(),self.mmap_area[pos].vpn_range.get_end());
            self.areas.remove(pos);
            0 // 成功找到并移除，返回 0 或其他表示成功的值
        } else {
            -1 // 未找到，返回 -1 或其他表示失败的值
        }
    }
    ///
    pub fn debug_addr_info(&self) {
        log_debug!("normal:");
    
        for ele in &self.areas {
            print!("{:x} {:x} {:x} {:x} perm:{:x} ",ele.vpn_range.get_start_addr().addr(),ele.vpn_range.get_end_addr().addr(),ele.vpn_range.get_start().to_addr(),ele.vpn_range.get_end().to_addr(),ele.map_perm.bits());
            match ele.area_type {
                MapAreaType::Elf=>{println!("elf");},
                MapAreaType::Heap=>{println!("heap");},
                MapAreaType::Mmap=>{println!("mmap");},
                MapAreaType::Stack=>{println!("Stack");},
            }
        }
    }
}
impl Drop for MemorySet{
    fn drop(&mut self) {
        self.recycle_data_pages();
    }
}
/*
// 动态堆
impl MemorySet {
    /// 映射虚拟页号到物理页帧
    pub fn map_page(&mut self, vpn: VirtPage, flags: MapPermission, size: MappingSize) -> PhysPage{
        let frame = frame_alloc().unwrap();
        let ppn: PhysPage = frame.ppn;
        if self.heap_area.vpn_range.get_start() == VirtPage::new(0) { //未初始化
            self.heap_area.vpn_range.l = vpn;
            self.heap_area.vpn_range.r = vpn;
        } 
        else {
            self.heap_area.vpn_range.r = vpn;
        }
        self.heap_area.data_frames.insert(ppn, frame);
        //let mut map_area = MapArea::new(startva, endva, MapType::Framed, flags);
        //map_area.data_frames.insert(ppn, frame);
        //self.push(map_area, None);
        let page_table = Arc::get_mut(&mut self.page_table).unwrap();
        page_table.map_page(vpn, ppn, flags.into(), size);
        //println!("after alloc: {},{}",self.heap_area.vpn_range.l,self.heap_area.vpn_range.r);
        ppn
        //println!("vpn={}, ppn={}",vpn,ppn);
    }

    /// 解除映射虚拟页号 还需要改，目前不使用
    pub fn unmap_page(&mut self, vpn: VirtPage) {
        //self.areas
        /*if self.map_type == MapType::Framed {
            self.data_frames.remove(&ppn);
        }*/
        if vpn.value() == 0 {
            //此时未拓展堆
            panic!("heap has not been extended!");
        }
        if vpn.value() > self.heap_area.vpn_range.r.value() || self.heap_area.vpn_range.l.value() > vpn.value() {
            panic!("vpn-range:({},{}) err-vpn:{}",self.heap_area.vpn_range.l, self.heap_area.vpn_range.r, vpn);
        }

        let page_table = Arc::get_mut(&mut self.page_table).unwrap();
        page_table.unmap_page(vpn);
        
        if let Some((pa,_flags)) = page_table.translate(VirtAddr::new(vpn.value() * PAGE_SIZE)) {
            self.heap_area.data_frames.remove(&pa.into());
            self.heap_area.vpn_range.r = VirtPage::new(vpn.value()-1);
            // frame_dealloc(ppn); // 如果需要释放物理帧
        } else {
            panic!("vpn:{} not exists!",vpn);
        }
        //println!("after dealloc: {},{}",self.heap_area.vpn_range.l,self.heap_area.vpn_range.r);
    }
        //frame_dealloc(ppn);
        //还需要对area做data_frames.remove(&ppn);
}*/

/// map area structure, controls a contiguous piece of virtual memory
#[derive(Clone)]
pub struct MapArea {
    ///
    pub vpn_range: VPNRange,
    ///
    pub data_frames: BTreeMap<VirtPage, Arc<FrameTracker>>,
    ///
    pub map_type: MapType,
    ///
    pub map_perm: MapPermission,
    ///
    pub area_type:MapAreaType,
    ///
    pub map_file:Option<Arc<dyn File>>,
    ///
    pub mmap_flag:MmapFlags
}

impl MapArea {
    ///
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
        area_type:MapAreaType,
    ) -> Self {
        let start_vpn: VirtPage = start_va.floor().into();
        let end_vpn: VirtPage = end_va.ceil().into();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn,start_va,end_va),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
            area_type,
            map_file:None,
            mmap_flag:MmapFlags::empty(),
        }
    }
    ///
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.get_start(), another.vpn_range.get_end(),another.vpn_range.get_start_addr(),another.vpn_range.get_end_addr()),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            area_type:another.area_type,
            map_file:another.map_file.clone(),
            mmap_flag:another.mmap_flag.clone()
            //data_frames: another.data_frames.clone(), // 使用 clone 方法来复制 BTreeMap
            //map_type: another.map_type.clone(),
            //map_perm: another.map_perm.clone(),
        }
    }
    pub fn map_one(&mut self, page_table: &Arc<PageTableWrapper>, vpn: VirtPage) {
        let frame = frame_alloc().unwrap();
        let ppn: PhysPage = frame.ppn;
        self.data_frames.insert(vpn, frame.into());
        /*match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }*/
        //let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map_page(vpn, ppn, self.map_perm.into(),MappingSize::Page4KB);
    }
    pub fn map_one_with_flags(&mut self, page_table: &Arc<PageTableWrapper>, vpn: VirtPage, flags: MappingFlags) {
        let frame = frame_alloc().unwrap();
        let ppn: PhysPage = frame.ppn;
        self.data_frames.insert(vpn, frame.into());
        
        page_table.map_page(vpn, ppn, flags,MappingSize::Page4KB);
    }
    
    pub fn unmap_one(&mut self, page_table: &Arc<PageTableWrapper>, vpn: VirtPage) {
        self.data_frames.remove(&vpn);
        page_table.unmap_page(vpn);
    }
    ///
    pub fn map(&mut self, page_table: &Arc<PageTableWrapper>) {
        for vpn in self.vpn_range {
            //self.map_one(page_table, vpn);
            let p_tracker = frame_alloc().expect("can't allocate frame");
            //println!("vpn={},ppn={}",vpn,p_tracker.ppn);
            page_table.map_page(vpn, p_tracker.ppn, self.map_perm.into(), MappingSize::Page4KB);
            self.data_frames.insert(vpn, p_tracker.into());   
        }
    } 
    /* 
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }*/
    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &Arc<PageTableWrapper>, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        let start_addr = self.vpn_range.get_start_addr().addr();
        if start_addr % PAGE_SIZE != 0{
            let copy_size = PAGE_SIZE - start_addr % PAGE_SIZE;
            
            let src = &data[start..len.min(start + copy_size)];
            let dst = &mut PhysPage::from(page_table.translate(current_vpn.into()).unwrap().0)
                .get_buffer()[start_addr % PAGE_SIZE..src.len() + start_addr % PAGE_SIZE];
            dst.copy_from_slice(src);
            start += copy_size;
            if start >= len {
                return;
            }
            // current_vpn.step();
            current_vpn = current_vpn + 1;
        }
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut PhysPage::from(page_table.translate(current_vpn.into()).unwrap().0)
                .get_buffer()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            // current_vpn.step();
            current_vpn = current_vpn + 1;
        }
    }

    pub fn transfer_frame(&mut self, new_area: &mut MapArea) {
        // frame_tracker转移
        for vpn_num in new_area.vpn_range.l.value()..new_area.vpn_range.r.value() {
            let vpn = VirtPage::new(vpn_num);
            // 可能因懒分配失败
            if let Some(frame) = self.data_frames.remove(&vpn) {
                // 转移frame到new_area.data_frames
                new_area.data_frames.insert(vpn, frame);
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
/// map type for memory set: identical or framed
pub enum MapType {
    ///
    Identical,
    ///
    Framed,
}

#[derive(Copy, Clone, PartialEq, Debug)]
/// map type for memory set: identical or framed
pub enum MapAreaType {
    ///
    Elf,
    ///
    Heap,
    ///
    Mmap,
    ///
    Stack
}

bitflags! {
    /// map permission corresponding to that in pte: `R W X U`
    pub struct MapPermission: u8 {
        ///Readable
        const R = 1 << 1;
        ///Writable
        const W = 1 << 2;
        ///Executable
        const X = 1 << 3;
        ///Accessible in U mode
        const U = 1 << 4;
    }
}

impl Into<MappingFlags> for MapPermission {
    fn into(self) -> MappingFlags {
        let mut flags = MappingFlags::empty();
        if self.contains(MapPermission::R) {
            flags |= MappingFlags::R;
        }
        if self.contains(MapPermission::W) {
            flags |= MappingFlags::W;
        }
        if self.contains(MapPermission::X) {
            flags |= MappingFlags::X;
        }
        if self.contains(MapPermission::U) {
            flags |= MappingFlags::U;
        }
        flags
    }
}
///
pub fn from_prot(prot: i32) -> MapPermission {
    let mut perm = MapPermission{bits: 0u8};
    if prot & 2 != 0 {
        perm |= MapPermission::R; // PROT_READ
    }
    if prot & 4 != 0 {
        perm |= MapPermission::W; // PROT_WRITE
    }
    if prot & 8 != 0 {
        perm |= MapPermission::X; // PROT_EXEC
    }
    if prot & 16 !=0 {
        perm |= MapPermission::U;
    }
    
    perm
}
// 对齐到PAGE_SIZE
#[allow(unused)]
fn align_up(x: u64, align: usize) -> u64 {
    let align_64 = align as u64;
    (x + align_64 - 1) & !(align_64 - 1)
}
/* 
#[allow(unused)]
///Check PageTable running correctly
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.lock();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);
    println!("remap_test passed!");
}*/
#[derive(Clone)]
pub struct MapAreaControl {
    pub mmap_top: usize,
    mapfreeblock: Vec<MapFreeControl>,
}
impl MapAreaControl {
    pub fn new() -> Self {
        Self { 
            mmap_top: USER_MMAP_TOP, 
            mapfreeblock: Vec::new() 
        }
    }
    // 找到第一个合适的块
    pub fn find_block(&mut self, num: usize) -> usize {
        for (i, block) in self.mapfreeblock.iter_mut().enumerate() {
            if block.num >= num {
                block.num -= num;
                if block.num == 0 {
                    // 移除当前块并返回起始dizhi
                    return self.mapfreeblock.swap_remove(i).start_va;
                } else {
                    return block.start_va;
                }
            }
        }
        0
    }
}
#[derive(Clone)]
pub struct MapFreeControl {
    pub start_va: usize,
    pub num: usize,
}