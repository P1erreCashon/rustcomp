// os/src/task/signal.rs
use bitflags::*;

bitflags! {
    pub struct SignalFlags: u32 {
        /// 中断信号（Interrupt），通常由 Ctrl+C 触发，信号编号为 2
        const SIGINT = 1 << 2;
        
        /// 非法指令信号（Illegal Instruction），信号编号为 4
        const SIGILL = 1 << 4;
        
        /// 终止信号（Abort），通常由程序错误触发，信号编号为 6
        const SIGABRT = 1 << 6;
        
        /// 浮点异常信号（Floating-Point Exception），信号编号为 8
        const SIGFPE = 1 << 8;
        
        /// 分段错误信号（Segmentation Violation），通常由非法内存访问触发，信号编号为 11
        const SIGSEGV = 1 << 11;
    }
}

impl SignalFlags {
    /// 检查是否设置了错误信号，并返回相应的错误码和消息
    ///
    /// 如果设置了错误信号，返回 `Some((error_code, error_message))`，
    /// 否则返回 `None`。
    pub fn check_error(&self) -> Option<(i32, &'static str)> {
        if self.contains(Self::SIGINT) {
            Some((-2, "Killed, SIGINT=2"))
        } else if self.contains(Self::SIGILL) {
            Some((-4, "Illegal Instruction, SIGILL=4"))
        } else if self.contains(Self::SIGABRT) {
            Some((-6, "Aborted, SIGABRT=6"))
        } else if self.contains(Self::SIGFPE) {
            Some((-8, "Erroneous Arithmetic Operation, SIGFPE=8"))
        } else if self.contains(Self::SIGSEGV) {
            Some((-11, "Segmentation Fault, SIGSEGV=11"))
        } else {
            None
        }
    }
}

// 实现 Default trait
impl Default for SignalFlags {
    fn default() -> Self {
        SignalFlags::empty()
    }
}