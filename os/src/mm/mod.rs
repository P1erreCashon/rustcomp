//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.
//mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;
mod vpn_range;

//pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use arch::addr::{PhysPage, VirtAddr, VirtPage,PhysAddr};
pub use frame_allocator::{frame_alloc,frame_alloc_more, frame_dealloc, FrameTracker,init_frame_allocator,frame_alloc_persist};
pub use heap_allocator::init_heap;
pub use memory_set::{MapPermission, MemorySet, MapType, MapArea, from_prot};
use page_table::PTEFlags;
pub use page_table::{translated_byte_buffer, translated_ref, translated_refmut, translated_str,safe_translated_byte_buffer,safe_translated_ref,safe_translated_refmut};
pub use vpn_range::VPNRange;