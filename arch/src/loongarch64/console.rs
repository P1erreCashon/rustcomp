use core::fmt::Write;

use spin::Mutex;

use crate::VIRT_ADDR_START;

const UART_ADDR: usize =  0x1fe001e0 | VIRT_ADDR_START;

static COM1: Mutex<Uart> = Mutex::new(Uart::new(UART_ADDR));
use bitflags::*;
use volatile::{ReadOnly, Volatile, WriteOnly};

bitflags! {
    #[derive(Copy, Clone)]
    /// InterruptEnableRegister
    pub struct IER: u8 {
        const RX_AVAILABLE = 1 << 0;
        const TX_EMPTY = 1 << 1;
    }
    #[derive(Copy, Clone)]
    /// LineStatusRegister
    pub struct LSR: u8 {
        const DATA_AVAILABLE = 1 << 0;
        const THR_EMPTY = 1 << 5;
    }
    #[derive(Copy, Clone)]
    /// Model Control Register
    pub struct MCR: u8 {
        const DATA_TERMINAL_READY = 1 << 0;
        const REQUEST_TO_SEND = 1 << 1;
        const AUX_OUTPUT1 = 1 << 2;
        const AUX_OUTPUT2 = 1 << 3;
    }
}

#[repr(C)]
#[allow(dead_code)]
struct ReadWithoutDLAB {
    /// receiver buffer register
    pub rbr: ReadOnly<u8>,
    /// interrupt enable register
    pub ier: Volatile<IER>,
    /// interrupt identification register
    pub iir: ReadOnly<u8>,
    /// line control register
    pub lcr: Volatile<u8>,
    /// model control register
    pub mcr: Volatile<MCR>,
    /// line status register
    pub lsr: ReadOnly<LSR>,
    /// ignore MSR
    _padding1: ReadOnly<u8>,
    /// ignore SCR
    _padding2: ReadOnly<u8>,
}

#[repr(C)]
#[allow(dead_code)]
struct WriteWithoutDLAB {
    /// transmitter holding register
    pub thr: WriteOnly<u8>,
    /// interrupt enable register
    pub ier: Volatile<IER>,
    /// ignore FCR
    _padding0: ReadOnly<u8>,
    /// line control register
    pub lcr: Volatile<u8>,
    /// modem control register
    pub mcr: Volatile<MCR>,
    /// line status register
    pub lsr: ReadOnly<LSR>,
    /// ignore other registers
    _padding1: ReadOnly<u16>,
}

pub struct Uart {
    base_address: usize,
}

impl Uart {
    pub const fn new(base_address: usize) -> Self {
        Uart { base_address }
    }
    fn read_end(&mut self) -> &mut ReadWithoutDLAB {
        unsafe { &mut *(self.base_address as *mut ReadWithoutDLAB) }
    }

    fn write_end(&mut self) -> &mut WriteWithoutDLAB {
        unsafe { &mut *(self.base_address as *mut WriteWithoutDLAB) }
    }

    pub fn init(&mut self) {
        let read_end = self.read_end();
        let mut mcr = MCR::empty();
        mcr |= MCR::DATA_TERMINAL_READY;
        mcr |= MCR::REQUEST_TO_SEND;
        mcr |= MCR::AUX_OUTPUT2;
        read_end.mcr.write(mcr);
        let ier = IER::RX_AVAILABLE;
        read_end.ier.write(ier);
    }

    pub fn putchar(&mut self, c: u8) {
        let write_end = self.write_end();
        loop {
            if write_end.lsr.read().contains(LSR::THR_EMPTY) {
                write_end.thr.write(c);
                break;
            }
        }
    }

    pub fn getchar(&mut self) -> Option<u8> {
        let read_end = self.read_end();
        let lsr = read_end.lsr.read();
        unsafe {
            if lsr.contains(LSR::DATA_AVAILABLE)  {// The DR bit is 1, meaning data!
                 Some(read_end.rbr.read())
            } else { // The DR bit is 0, meaning no data
                 None
            }
        }
    }
}
impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.bytes() {
            self.putchar(c);
        }
        Ok(())
    }
}

/// Writes a byte to the console.
pub fn console_putchar(c: u8) {
    COM1.lock().putchar(c)
}

/// read a byte, return -1 if nothing exists.
pub fn console_getchar() -> Option<u8> {
    COM1.lock().getchar()
}
///
pub fn console_init(){
    COM1.lock().init();
}
