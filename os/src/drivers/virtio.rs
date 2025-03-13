use crate::mm::{
    frame_alloc_more, frame_dealloc,FrameTracker,
};
use crate::sync::IntrCell;
use alloc::vec::Vec;
use lazy_static::*;
use virtio_drivers::Hal;
use arch::addr::{PhysAddr, PhysPage, VirtAddr};
use arch::kernel_page_table;

lazy_static! {
    static ref QUEUE_FRAMES: IntrCell<Vec<FrameTracker>> =
       IntrCell::new(Vec::new());
}

pub struct VirtioHal;

impl Hal for VirtioHal {
    fn dma_alloc(pages: usize) -> usize {
        let trakcers = frame_alloc_more(pages);
        let ppn_base = trakcers.as_ref().unwrap().last().unwrap().ppn;
        QUEUE_FRAMES
            .lock()
            .append(&mut trakcers.unwrap());
        let pa: PhysAddr = ppn_base.into();
        pa.addr()
    }

    fn dma_dealloc(pa: usize, pages: usize) -> i32 {
        let mut pa = PhysAddr::new(pa);
        let mut ppn_base: PhysPage = pa.into();
        for _ in 0..pages {
            frame_dealloc(ppn_base );
            let paddr = ppn_base.as_num()+1;
            pa = PhysAddr::new(paddr);
            ppn_base = pa.into();
        }
        0
    }

    fn phys_to_virt(addr: usize) -> usize {
        addr
    }

    fn virt_to_phys(vaddr: usize) -> usize {
        kernel_page_table()
            .translate(VirtAddr::new(vaddr))
            .unwrap()
            .0.addr()
 //     PageTable::from_token(kernel_token())
 //           .translate_va(VirtAddr::from(vaddr))
 //           .unwrap()
  //          .0
    }
}
