//! Constants used in rCore
#[allow(unused)]

pub const USER_STACK_SIZE: usize = 4096 * 5;
pub const KERNEL_STACK_SIZE: usize = 4096 *5;
pub const KERNEL_HEAP_SIZE: usize = 0x200_0000;

pub const PAGE_SIZE: usize = 0x1000;
//pub const PAGE_SIZE_BITS: usize = 0xc;


//pub use crate::board::CLOCK_FREQ;
