//! Implementation of [`MapArea`] and [`MemorySet`].
use super::{frame_alloc, FrameTracker};
//use super::{PTEFlags, PageTable, PageTableEntry};
//use super::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::vpn_range::VPNRange;
use alloc::alloc::dealloc;
use arch::pagetable::{MappingFlags, MappingSize, PageTable, PageTableWrapper};
use arch::addr::{PhysAddr, PhysPage, VirtAddr, VirtPage};
use arch::{USER_VADDR_END,PAGE_SIZE};
use crate::config::{USER_HEAP_SIZE, USER_MMAP_TOP, USER_STACK_SIZE, USER_STACK_TOP};
use crate::sync::UPSafeCell;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

//const MODULE_LEVEL:log::Level = log::Level::Info;

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
/// memory set structure, controls virtual-memory space
pub struct MemorySet {
    ///
    pub page_table: Arc<PageTableWrapper>,
    ///
    pub areas: Vec<MapArea>,
    ///
    pub heap_area: Vec<MapArea>,
    ///
    pub mmap_area: Vec<MapArea>,
}

impl MemorySet {
    ///Create an empty `MemorySet`
    pub fn new_bare() -> Self {
        Self {
            page_table:Arc::new(PageTableWrapper::alloc()),
            areas: Vec::new(),
            heap_area: Vec::new(),
            mmap_area: Vec::new(),
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
    /// 分配+映射->heap_area
    pub fn push_into_heaparea(&mut self, mut map_area: MapArea, data: Option<&[u8]>) { 
        map_area.map(&self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.heap_area.push(map_area);
    }
    /// 分配+映射->mmap_area
    pub fn push_into_mmaparea(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.mmap_area.push(map_area);
    }
    /// Include sections in elf and trampoline and TrapContext and user stack,
    /// also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize, usize,u16,u16,u64,usize) {
        let mut memory_set = Self::new_bare();
        // map trampoline
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
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
                tls_addr = ph.virtual_addr();
            }
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();                
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
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
              //  max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
                // 最大段地址
                //let section_end = align_up(ph.virtual_addr() + ph.mem_size(), PAGE_SIZE);
                let mut section_end = ph.virtual_addr() + ph.mem_size();
                let pn: u64 = section_end / PAGE_SIZE as u64;
                if pn * PAGE_SIZE as u64 != section_end {
                    section_end = (pn + 1) * PAGE_SIZE as u64;
                }
                max_virt_mem = max_virt_mem.max(section_end);
            }
        }

        // 为程序映像转储 elf 程序头

        let heap_start:usize =  max_virt_mem.try_into().unwrap();
        let heap_top: usize = heap_start + USER_HEAP_SIZE;
        memory_set.push(
            MapArea::new(
                heap_start.into(),
                heap_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );
        // map user stack with U flags
        // let max_end_va: VirtAddr = max_end_vpn.into();
        
        // guard page
        let user_stack_top = USER_STACK_TOP; //8G
        let user_stack_bottom = user_stack_top - USER_STACK_SIZE;
        //println!("heaptop:{:x} user_stack_bottom:{:x}",heap_top,user_stack_top);
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
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
            header_va + elf_header.pt2.ph_offset() as usize
        )
    }
    ///Clone a same `MemorySet`
    pub fn from_existed_user(user_space: &MemorySet) -> MemorySet {
        let mut memory_set = Self::new_bare();
        // map trampoline
        // copy data sections/trap_context/user_stack
        for area in user_space.areas.iter() {
            let new_area = MapArea::from_another(area);
            memory_set.push(new_area, None);
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().0;
                let dst_ppn = memory_set.translate(vpn).unwrap().0;
              //  dst_ppn
                //    .get_bytes_array()
                  //  .copy_from_slice(src_ppn.get_bytes_array());
                  dst_ppn.get_buffer().copy_from_slice(src_ppn.get_buffer())
            }
        }
        // copy heap_area (可能出错)
        for area in user_space.heap_area.iter() {
            let new_area = MapArea::from_another(area);
            memory_set.push_into_heaparea(new_area, None);
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().0;
                let dst_ppn = memory_set.translate(vpn).unwrap().0;
              //  dst_ppn
                //    .get_bytes_array()
                  //  .copy_from_slice(src_ppn.get_bytes_array());
                  dst_ppn.get_buffer().copy_from_slice(src_ppn.get_buffer())
            }
        }
        //copy mmap_area (可能出错)
        for area in user_space.mmap_area.iter() {
            let new_area = MapArea::from_another(area);
            memory_set.push_into_mmaparea(new_area, None);
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().0;
                let dst_ppn = memory_set.translate(vpn).unwrap().0;
              //  dst_ppn
                //    .get_bytes_array()
                  //  .copy_from_slice(src_ppn.get_bytes_array());
                  dst_ppn.get_buffer().copy_from_slice(src_ppn.get_buffer())
            }
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
        if let Some(pos) = self.mmap_area.iter().position(|map_area| map_area.vpn_range.get_start() == num) {
        //    println!("remove vpn: {}~{}",self.mmap_area[pos].vpn_range.get_start(),self.mmap_area[pos].vpn_range.get_end());
            self.mmap_area.remove(pos);
            0 // 成功找到并移除，返回 0 或其他表示成功的值
        } else {
            -1 // 未找到，返回 -1 或其他表示失败的值
        }
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

pub struct MapArea {
    ///
    pub vpn_range: VPNRange,
    data_frames: BTreeMap<PhysPage, FrameTracker>,
    ///
    pub map_type: MapType,
    ///
    pub map_perm: MapPermission,
}

impl MapArea {
    ///
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPage = start_va.floor().into();
        let end_vpn: VirtPage = end_va.ceil().into();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn,start_va,end_va),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }
    ///
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.get_start(), another.vpn_range.get_end(),another.vpn_range.get_start_addr(),another.vpn_range.get_end_addr()),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            //data_frames: another.data_frames.clone(), // 使用 clone 方法来复制 BTreeMap
            //map_type: another.map_type.clone(),
            //map_perm: another.map_perm.clone(),
        }
    }
    ///
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPage) {
        let frame = frame_alloc().unwrap();
        let ppn: PhysPage = frame.ppn;
        self.data_frames.insert(ppn, frame);
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
    ///
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPage, ppn: PhysPage) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&ppn);
        }
        page_table.unmap_page(vpn);
    }
    ///
    pub fn map(&mut self, page_table: &Arc<PageTableWrapper>) {
        for vpn in self.vpn_range {
            //self.map_one(page_table, vpn);
            let p_tracker = frame_alloc().expect("can't allocate frame");
            //println!("vpn={},ppn={}",vpn,p_tracker.ppn);
            page_table.map_page(vpn, p_tracker.ppn, self.map_perm.into(), MappingSize::Page4KB);
            self.data_frames.insert(p_tracker.ppn, p_tracker);   
        }
    } /* 
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
}

#[derive(Copy, Clone, PartialEq, Debug)]
/// map type for memory set: identical or framed
pub enum MapType {
    ///
    Identical,
    ///
    Framed,
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
    if prot & 1 != 0 {
        perm |= MapPermission::R; // PROT_READ
    }
    if prot & 2 != 0 {
        perm |= MapPermission::W; // PROT_WRITE
    }
    if prot & 4 != 0 {
        perm |= MapPermission::X; // PROT_EXEC
    }
    if prot & 8 !=0 {
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
