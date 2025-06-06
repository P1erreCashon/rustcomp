use bitflags::bitflags;

use x86::tlb;

use crate::addr::{PhysAddr, PhysPage, VirtAddr, VirtPage};
use crate::{
    pagetable::{MappingFlags, PageTable, PTE, TLB},
    VIRT_ADDR_START,
};

bitflags! {
    pub struct PTEFlags: u64 {
        /// Page is present in the page table
        const P         = bit!(0);
        /// Read/Write; if 0, Only read
        const RW        = bit!(1);
        /// User/Supervisor; if 0, Only supervisor
        const US        = bit!(2);
        /// Page-level wright-through
        const PWT       = bit!(3);
        /// Page-level cache disable.
        const PCD       = bit!(4);
        /// Accessed; indicates whether software has accessed the 4-KByte page
        const A         = bit!(5);
        /// Dirty; indicates whether software has written to the 4-KByte page referenced by this entry.
        const D         = bit!(6);
        /// Page size; if set this entry maps a 2-MByte page; otherwise, this entry references a page directory.
        const PS      = bit!(7);
        /// Global; if CR4.PGE = 1, determines whether the translation is global (see Section 4.10); ignored otherwise
        const G         = bit!(8);
        /// User defined flag -- ignored by hardware (bit 9)
        const USER_9    = bit!(9);
        /// User defined flag -- ignored by hardware (bit 10)
        const USER_10   = bit!(10);
        /// User defined flag -- ignored by hardware (bit 11)
        const USER_11   = bit!(11);
        ///  If IA32_EFER.NXE = 1, execute-disable
        ///  If 1, instruction fetches are not allowed from the 512-GByte region.
        const XD        = bit!(63);
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(flags: MappingFlags) -> Self {
        let mut res = Self::P;
        if flags.contains(MappingFlags::W) {
            res |= Self::RW;
        }
        if flags.contains(MappingFlags::U) {
            res |= Self::US;
        }
        if flags.contains(MappingFlags::A) {
            res |= Self::A;
        }
        if flags.contains(MappingFlags::D) {
            res |= Self::D;
        }
        if flags.contains(MappingFlags::X) {
            res.remove(Self::XD);
        }
        res
    }
}

impl Into<MappingFlags> for PTEFlags {
    fn into(self) -> MappingFlags {
        let mut res = MappingFlags::empty();
        if self.contains(Self::RW) {
            res |= MappingFlags::W
        };
        if self.contains(Self::US) {
            res |= MappingFlags::U
        };
        if self.contains(Self::A) {
            res |= MappingFlags::A;
        }
        if self.contains(Self::D) {
            res |= MappingFlags::D;
        }
        if !self.contains(Self::XD) {
            res |= MappingFlags::X
        }
        res
    }
}

impl PTE {
    #[inline]
    pub(crate) fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::P)
    }

    #[inline]
    pub(crate) fn is_table(&self) -> bool {
        self.flags().contains(PTEFlags::P)
    }

    #[inline]
    pub(crate) fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate(self.0 as _)
    }

    #[inline]
    pub(crate) fn new_table(ppn: PhysPage) -> Self {
        Self(ppn.to_addr() | (PTEFlags::P | PTEFlags::US | PTEFlags::RW).bits() as usize)
    }

    #[inline]
    pub(crate) fn new_page(ppn: PhysPage, flags: PTEFlags) -> Self {
        Self(ppn.to_addr() | flags.bits() as usize)
    }

    #[inline]
    pub(crate) fn address(&self) -> PhysAddr {
        PhysAddr(self.0 & 0xFFFF_FFFF_F000)
    }
}

impl PageTable {
    /// The size of the page for this platform.
    pub(crate) const PAGE_SIZE: usize = 0x1000;
    pub(crate) const PAGE_LEVEL: usize = 4;
    pub(crate) const PTE_NUM_IN_PAGE: usize = 0x200;
    pub(crate) const GLOBAL_ROOT_PTE_RANGE: usize = 0x100;
    pub(crate) const VADDR_BITS: usize = 48;
    pub(crate) const USER_VADDR_END: usize = (1 << Self::VADDR_BITS) - 1;
    pub(crate) const KERNEL_VADDR_START: usize = !Self::USER_VADDR_END;

    #[inline]
    pub fn restore(&self) {
        self.release();

        extern "C" {
            fn _kernel_mapping_pdpt();
        }
        let pml4 = self.0.slice_mut_with_len::<PTE>(Self::PTE_NUM_IN_PAGE);
        pml4[0x1ff] = PTE((_kernel_mapping_pdpt as usize - VIRT_ADDR_START as usize) | 0x3);
        TLB::flush_all();
    }

    #[inline]
    pub fn change(&self) {
        unsafe {
            core::arch::asm!("mov     cr3, {}", in(reg) self.0.0);
        }
    }
}

/// TLB operations
impl TLB {
    /// flush the TLB entry by VirtualAddress
    /// just use it directly
    ///
    /// TLB::flush_vaddr(arg0); // arg0 is the virtual address(VirtAddr)
    #[inline]
    pub fn flush_vaddr(vaddr: VirtAddr) {
        unsafe { tlb::flush(vaddr.into()) }
    }

    /// flush all tlb entry
    ///
    /// how to use ?
    /// just
    /// TLB::flush_all();
    #[inline]
    pub fn flush_all() {
        unsafe { tlb::flush_all() }
    }
}

impl VirtPage {
    /// Get n level page table index of the given virtual address
    #[inline]
    pub fn pn_index(&self, n: usize) -> usize {
        (self.0 >> 9 * n) & 0x1ff
    }
}

impl VirtAddr {
    /// Get n level page table offset of the given virtual address
    #[inline]
    pub fn pn_offest(&self, n: usize) -> usize {
        self.0 % (1 << (12 + 9 * n))
    }
}
