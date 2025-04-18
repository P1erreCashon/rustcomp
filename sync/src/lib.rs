#![no_std]
#![no_main]

mod kspinbase;
mod kspinlib;
pub use spin::{Mutex,MutexGuard,Once};
pub use kspinlib::{SpinNoIrq as MutexNoIrq, SpinNoIrqGuard as MutexGuardNoIrq};