use bitflags::bitflags;
use riscv::register::satp;

use crate::addr::{PhysAddr, PhysPage, VirtAddr, VirtPage};
use crate::pagetable::{PageTable, PTE, TLB};
use crate::{pagetable::MappingFlags, sigtrx::get_trx_mapping};

impl PTE {
    #[inline]
    pub const fn from_ppn(ppn: usize, flags: PTEFlags) -> Self {
        // let flags = flags.union(PTEFlags::D);
        let mut flags = flags;
        if flags.contains(PTEFlags::R) | flags.contains(PTEFlags::X) {
            flags = flags.union(PTEFlags::A)
        }
        if flags.contains(PTEFlags::W) {
            flags = flags.union(PTEFlags::D)
        }
        // TIPS: This is prepare for the extend bits of T-HEAD C906
        #[cfg(c906)]
        if flags.contains(PTEFlags::G) && ppn == 0x8_0000 {
            Self(
                ppn << 10
                    | flags
                        .union(PTEFlags::C)
                        .union(PTEFlags::B)
                        .union(PTEFlags::K)
                        .bits() as usize,
            )
        } else if flags.contains(PTEFlags::G) && ppn == 0 {
            Self(ppn << 10 | flags.union(PTEFlags::SE).union(PTEFlags::SO).bits() as usize)
        } else {
            Self(ppn << 10 | flags.union(PTEFlags::C).bits() as usize)
        }

        #[cfg(not(c906))]
        Self(ppn << 10 | flags.bits() as usize)
    }

    #[inline]
    pub const fn from_addr(addr: usize, flags: PTEFlags) -> Self {
        Self::from_ppn(addr >> 12, flags)
    }

    #[inline]
    pub const fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits_truncate((self.0 & 0x1ff) as u64)
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.flags().contains(PTEFlags::V) && self.0 > u8::MAX as usize
    }

    /// 判断是否是大页
    ///
    /// 大页判断条件 V 位为 1, R/W/X 位至少有一个不为 0
    /// PTE 页表范围 1G(0x4000_0000) 2M(0x20_0000) 4K(0x1000)
    #[inline]
    pub fn is_huge(&self) -> bool {
        return self.flags().contains(PTEFlags::V)
            && (self.flags().contains(PTEFlags::R)
                || self.flags().contains(PTEFlags::W)
                || self.flags().contains(PTEFlags::X));
    }

    #[inline]
    pub(crate) fn is_table(&self) -> bool {
        return self.flags().contains(PTEFlags::V)
            && !(self.flags().contains(PTEFlags::R)
                || self.flags().contains(PTEFlags::W)
                || self.flags().contains(PTEFlags::X));
    }

    #[inline]
    pub(crate) fn new_table(ppn: PhysPage) -> Self {
        Self((ppn.0 << 10) | (PTEFlags::V).bits() as usize)
    }

    #[inline]
    pub(crate) fn new_page(ppn: PhysPage, flags: PTEFlags) -> Self {
        Self((ppn.0 << 10) | flags.bits() as usize)
    }

    #[inline]
    pub(crate) fn address(&self) -> PhysAddr {
        PhysAddr((self.0 << 2) & 0xFFFF_FFFF_F000)
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PTEFlags: u64 {
        const V = bit!(0);
        const R = bit!(1);
        const W = bit!(2);
        const X = bit!(3);
        const U = bit!(4);
        const G = bit!(5);
        const A = bit!(6);
        const D = bit!(7);
        const cow = bit!(8);

        #[cfg(c906)]
        const SO = bit!(63);
        #[cfg(c906)]
        const C = bit!(62);
        #[cfg(c906)]
        const B = bit!(61);
        #[cfg(c906)]
        const K = bit!(60);
        #[cfg(c906)]
        const SE = bit!(59);

        const VRWX  = Self::V.bits() | Self::R.bits() | Self::W.bits() | Self::X.bits();
        const ADUVRX = Self::A.bits() | Self::D.bits() | Self::U.bits() | Self::V.bits() | Self::R.bits() | Self::X.bits();
        const ADVRWX = Self::A.bits() | Self::D.bits() | Self::VRWX.bits();
        const ADGVRWX = Self::G.bits() | Self::ADVRWX.bits();
    }
}

impl From<MappingFlags> for PTEFlags {
    fn from(flags: MappingFlags) -> Self {
        if flags.is_empty() {
            Self::empty()
        } else {
            let mut res = Self::V;
            if flags.contains(MappingFlags::R) {
                res |= PTEFlags::R;
            }
            if flags.contains(MappingFlags::W) {
                res |= PTEFlags::W;
            }
            if flags.contains(MappingFlags::X) {
                res |= PTEFlags::X;
            }
            if flags.contains(MappingFlags::U) {
                res |= PTEFlags::U;
            }
            if flags.contains(MappingFlags::A) {
                res |= PTEFlags::A;
            }
            if flags.contains(MappingFlags::D) {
                res |= PTEFlags::D;
            }
            if flags.contains(MappingFlags::cow) {
                res |= PTEFlags::cow;
            }
            res
        }
    }
}

impl From<PTEFlags> for MappingFlags {
    fn from(value: PTEFlags) -> Self {
        let mut mapping_flags = MappingFlags::empty();
        if value.contains(PTEFlags::V) {
            mapping_flags |= MappingFlags::P;
        }
        if value.contains(PTEFlags::R) {
            mapping_flags |= MappingFlags::R;
        }
        if value.contains(PTEFlags::W) {
            mapping_flags |= MappingFlags::W;
        }
        if value.contains(PTEFlags::X) {
            mapping_flags |= MappingFlags::X;
        }
        if value.contains(PTEFlags::U) {
            mapping_flags |= MappingFlags::U;
        }
        if value.contains(PTEFlags::A) {
            mapping_flags |= MappingFlags::A;
        }
        if value.contains(PTEFlags::D) {
            mapping_flags |= MappingFlags::D;
        }
        if value.contains(PTEFlags::cow) {
            mapping_flags |= MappingFlags::cow;
        }
        mapping_flags
    }
}

impl PageTable {
    /// The size of the page for this platform.
    pub(crate) const PAGE_SIZE: usize = 0x1000;
    pub(crate) const PAGE_LEVEL: usize = 3;
    pub(crate) const PTE_NUM_IN_PAGE: usize = 0x200;
    pub(crate) const GLOBAL_ROOT_PTE_RANGE: usize = 0x100;
    pub(crate) const VADDR_BITS: usize = 39;
    pub(crate) const USER_VADDR_END: usize = (1 << Self::VADDR_BITS) - 1;
    pub(crate) const KERNEL_VADDR_START: usize = !Self::USER_VADDR_END;

    pub fn current() -> Self {
        Self(PhysAddr(satp::read().ppn() << 12))
    }

    #[inline]
    pub fn restore(&self) {
        self.release();
        let arr = Self::get_pte_list(self.0);
        arr[0x100] = PTE::from_addr(0x0000_0000, PTEFlags::ADGVRWX);
        arr[0x101] = PTE::from_addr(0x4000_0000, PTEFlags::ADGVRWX);
        arr[0x102] = PTE::from_addr(0x8000_0000, PTEFlags::ADGVRWX);
        arr[0x104] = PTE::from_addr(get_trx_mapping(), PTEFlags::V);
        arr[0x106] = PTE::from_addr(0x8000_0000, PTEFlags::ADGVRWX);
        // arr[0..0x100].fill(PTE::from_addr(0, PTEFlags::empty()));
        arr[0..0x100].fill(PTE(0));
    }

    #[inline]
    pub fn change(&self) {
        // Write page table entry for
        satp::write((8 << 60) | (self.0 .0 >> 12));
        TLB::flush_all();
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
