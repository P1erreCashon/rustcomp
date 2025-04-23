#![no_std]
#![no_main]

//mod kspinbase;
//mod kspinlib;
pub use spin::{Mutex,MutexGuard,Once};
pub use spin::{Mutex as MutexNoIrq, MutexGuard as MutexGuardNoIrq};
//pub use kspinlib::{SpinNoIrq as MutexNoIrq, SpinNoIrqGuard as MutexGuardNoIrq};