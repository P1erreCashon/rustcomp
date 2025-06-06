mod boot;
mod console;
mod consts;
mod context;
#[cfg(feature = "kcontext")]
mod kcontext;
mod page_table;
mod sigtrx;
mod timer;
mod trap;

pub use console::{console_getchar, console_putchar,console_init};
pub use consts::*;
pub use context::TrapFrame;
#[cfg(feature = "kcontext")]
pub use kcontext::{context_switch, context_switch_pt, read_current_tp, KContext};
use loongarch64::register::euen;
pub use page_table::*;
pub use trap::{disable_irq, enable_external_irq, enable_irq, init_interrupt, run_user_task};

use crate::api::ArchInterface;
use crate::clear_bss;

pub fn rust_tmp_main(hart_id: usize) {
    clear_bss();
    console_init();
    ArchInterface::init_logging();
    ArchInterface::init_allocator();
    trap::set_trap_vector_base();
    sigtrx::init();

    ArchInterface::add_memory_region(
        VIRT_ADDR_START | 0x9000_0000,
        VIRT_ADDR_START | (0x9000_0000 + 0x2000_0000),
    );
    info!("hart_id: {}", hart_id);

    ArchInterface::prepare_drivers();

    // Enable floating point
    euen::set_fpe(true);
    timer::init_timer();

    ArchInterface::main(0);

    shutdown();
}

pub fn shutdown() -> ! {
    error!("shutdown!");
    loop {
        unsafe { loongarch64::asm::idle() };
    }
}
