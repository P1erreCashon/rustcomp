//! Implementation of [`PageTableEntry`] and [`PageTable`].
use arch::addr::VirtPage;
//use super::{frame_alloc, FrameTracker, PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use arch::pagetable::{MappingFlags, MappingSize, PageTable};
use arch::{TrapType, PAGE_SIZE};
use alloc::string::{String,ToString};
use _core::str::from_utf8_unchecked;
use _core::slice;
use bitflags::*;
use super::{MemorySet,VirtAddr};
use alloc::sync::Arc;
use sync::Mutex;

//const MODULE_LEVEL:log::Level = log::Level::Info;

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}
///
pub fn translated_byte_buffer(_token: PageTable, ptr: *mut u8, len: usize) -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(ptr, len) }
}
///
#[allow(unused)]
pub fn safe_translated_byte_buffer(
    memory_set: Arc<Mutex<MemorySet>>,
    ptr: *mut u8,
    len: usize,
) -> &'static mut [u8] {
    let mut memory_set = memory_set.lock();
    let page_table = memory_set.page_table.clone();
    let mut start = ptr as usize;
    let end = start + len;
    while start < end {
        let start_va = VirtAddr::from(start);
        let vpn:VirtPage = start_va.into();
        match page_table.translate(vpn.into()) {
            None => {
                let r = memory_set.handle_lazy_addr(start_va.addr(),TrapType::StorePageFault(start_va.addr()) );
                if r.is_err(){
                    if let Err(e) = memory_set.handle_cow_addr(start_va.addr()){
                        panic!("err when translating refmut:{:?}",e);
                    }
                }
            }
            Some((pa,_mp)) => {
                if pa.addr() == 0 || !_mp.contains(MappingFlags::P){
                    let r = memory_set.handle_lazy_addr(start_va.addr(),TrapType::StorePageFault(start_va.addr()) );
                    if r.is_err(){
                        if let Err(e) = memory_set.handle_cow_addr(start_va.addr()){
                            panic!("err when translating refmut:{:?}",e);
                        }
                    }
                }
                if _mp.contains(MappingFlags::cow) && pa.addr() != 0{
                    if let Err(e) = memory_set.handle_cow_addr(start_va.addr()){
                        panic!("err when translating refmut:{:?}",e);
                    }
                }
            }
        }
        let mut end_va: VirtAddr = (vpn.to_addr() + PAGE_SIZE).into() ;
        end_va = end_va.min(VirtAddr::from(end));
        start = end_va.into();
    }
    unsafe { core::slice::from_raw_parts_mut(ptr, len) }
}


unsafe fn str_len(ptr: *const u8) -> usize {
    let mut i = 0;
    loop {
        if *ptr.add(i) == 0 {
            break i;
        }
        i += 1;
    }
}

/// Load a string from other address spaces into kernel space without an end `\0`.
pub fn translated_str(_token: PageTable, ptr: *const u8) -> String {
    unsafe {
        let len = str_len(ptr);
        from_utf8_unchecked(slice::from_raw_parts(ptr, len)).to_string()
    }
}
///
pub fn translated_ref<T>(_token: PageTable, ptr: *const T) -> &'static T {
    unsafe { ptr.as_ref().unwrap() }
}
///
#[allow(unused)]
pub fn safe_translated_ref<T>(memory_set: Arc<Mutex<MemorySet>>, ptr: *const T) -> &'static T {
    let mut memory_set = memory_set.lock();
    let page_table = memory_set.page_table.clone();
    let va = VirtAddr::from(ptr as usize);
    match page_table.translate(va) {
        None => {
            let _ = memory_set.handle_lazy_addr(va.addr(),TrapType::StorePageFault(va.addr()) );
        }
        Some((pa,_mp)) => {
            if pa.addr() == 0 {
                let _ = memory_set.handle_lazy_addr(va.addr(),TrapType::StorePageFault(va.addr()) );
            }
        }
    }
    unsafe { ptr.as_ref().unwrap() }
}

///
pub fn translated_refmut<T>(_token: PageTable, ptr: *mut T) -> &'static mut T {
    unsafe { ptr.as_mut().unwrap() }
}
///
#[allow(unused)]
pub fn safe_translated_refmut<T>(memory_set: Arc<Mutex<MemorySet>>, ptr: *mut T) -> &'static mut T {
    let mut memory_set = memory_set.lock();
    let page_table = memory_set.page_table.clone();
    let va = VirtAddr::from(ptr as usize);
    match page_table.translate(va) {
        None => {
            let r = memory_set.handle_lazy_addr(va.addr(),TrapType::StorePageFault(va.addr()) );
            if r.is_err(){
                if let Err(e) = memory_set.handle_cow_addr(va.addr()){
                    panic!("err when translating refmut:{:?}",e);
                }
            }
        }
        Some((pa,_mp)) => {
            if pa.addr() == 0 || !_mp.contains(MappingFlags::P){
                let r = memory_set.handle_lazy_addr(va.addr(),TrapType::StorePageFault(va.addr()) );
                if r.is_err(){
                    if let Err(e) = memory_set.handle_cow_addr(va.addr()){
                        panic!("err when translating refmut:{:?}",e);
                    }
                }
            }
            if _mp.contains(MappingFlags::cow) && pa.addr() != 0{
                if let Err(e) = memory_set.handle_cow_addr(va.addr()){
                    panic!("err when translating refmut:{:?}",e);
                }
            }
        }
    }
    unsafe { ptr.as_mut().unwrap() }
}
/*
#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    ///PTE
    pub bits: usize,
}

impl PageTableEntry {
    ///Create a PTE from ppn
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    ///Return an empty PTE
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    ///Return 44bit ppn
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    ///Return 10bit flag
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    ///Check PTE valid
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    ///Check PTE readable
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    ///Check PTE writable
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    ///Check PTE executable
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}
///Record root ppn and has the same lifetime as 1 and 2 level `PageTableEntry`
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Create an empty `PageTable`
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        log_info!("create pagetable:{:x}",frame.ppn.0);
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find phsical address by virtual address, create a frame if not exist
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                log_debug!("create pte:{:x}",frame.ppn.0);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find phsical address by virtual address
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    #[allow(unused)]
    /// Create a mapping form `vpn` to `ppn`
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);        
        log_debug!("mapping table:{:x} vpn:{:x} ppn:{:x}",self.root_ppn.0,vpn.0,ppn.0);
    }
    #[allow(unused)]
    /// Delete a mapping form `vpn`
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
        log_debug!("unmapping table:{:x} vpn:{:x}",self.root_ppn.0,vpn.0);
    }
    /// Translate `VirtPageNum` to `PageTableEntry`
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// Translate `VirtAddr` to `PhysAddr`
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
    /// Get root ppn
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}
/// Translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// Translate a pointer to a mutable u8 Vec end with `\0` through page table to a `String`
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .get_mut());
        if ch == 0 {
            break;
        }
        string.push(ch as char);
        va += 1;
    }
    string
}

#[allow(unused)]
///Translate a generic through page table and return a reference
pub fn translated_ref<T>(token: usize, ptr: *const T) -> &'static T {
    let page_table = PageTable::from_token(token);
    page_table
        .translate_va(VirtAddr::from(ptr as usize))
        .unwrap()
        .get_ref()
}
///Translate a generic through page table and return a mutable reference
pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .get_mut()
}
        */