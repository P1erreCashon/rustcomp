use crate::board::CharDeviceImpl;
use alloc::sync::Arc;
use lazy_static::*;
pub use uart::NS16550a;
mod uart;
pub trait CharDevice {
    #[allow(unused)]
    fn init(&self);
    #[allow(unused)]
    fn read(&self) -> u8;
    #[allow(unused)]
    fn write(&self, ch: u8);
    #[allow(unused)]
    fn handle_irq(&self);
}

lazy_static! {
    pub static ref UART: Arc<CharDeviceImpl> = Arc::new(CharDeviceImpl::new());
}
