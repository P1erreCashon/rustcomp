// os/src/task/signal.rs

use bitflags::*;

bitflags! {
    pub struct SignalFlags: u32 {
        /// 默认信号处理，信号编号为 0
        const SIGDEF  = 1 << 0;  // 0
        /// 挂起信号（Hangup），信号编号为 1
        const SIGHUP  = 1 << 1;  // 1
        /// 中断信号（Interrupt），通常由 Ctrl+C 触发，信号编号为 2
        const SIGINT  = 1 << 2;  // 2
        /// 退出信号（Quit），信号编号为 3
        const SIGQUIT = 1 << 3;  // 3
        /// 非法指令信号（Illegal Instruction），信号编号为 4
        const SIGILL  = 1 << 4;  // 4
        /// 陷阱信号（Trap），信号编号为 5
        const SIGTRAP = 1 << 5;  // 5
        /// 终止信号（Abort），通常由程序错误触发，信号编号为 6
        const SIGABRT = 1 << 6;  // 6
        /// 总线错误信号（Bus Error），信号编号为 7
        const SIGBUS  = 1 << 7;  // 7
        /// 浮点异常信号（Floating-Point Exception），信号编号为 8
        const SIGFPE  = 1 << 8;  // 8
        /// 杀死信号（Kill），信号编号为 9
        const SIGKILL = 1 << 9;  // 9
        /// 用户定义信号 1，信号编号为 10
        const SIGUSR1 = 1 << 10; // 10
        /// 分段错误信号（Segmentation Violation），通常由非法内存访问触发，信号编号为 11
        const SIGSEGV = 1 << 11; // 11
        /// 用户定义信号 2，信号编号为 12
        const SIGUSR2 = 1 << 12; // 12
        /// 管道错误信号（Broken Pipe），信号编号为 13
        const SIGPIPE = 1 << 13; // 13
        /// 闹钟信号（Alarm Clock），信号编号为 14
        const SIGALRM = 1 << 14; // 14
        /// 终止信号（Terminate），信号编号为 15
        const SIGTERM = 1 << 15; // 15
        /// 堆栈错误信号（Stack Fault），信号编号为 16
        const SIGSTKFLT = 1 << 16; // 16
        /// 子进程状态改变信号（Child Status Changed），信号编号为 17
        const SIGCHLD = 1 << 17; // 17
        /// 继续信号（Continue），信号编号为 18
        const SIGCONT = 1 << 18; // 18
        /// 停止信号（Stop），信号编号为 19
        const SIGSTOP = 1 << 19; // 19
        /// 终端停止信号（Terminal Stop），信号编号为 20
        const SIGTSTP = 1 << 20; // 20
        /// 终端输入信号（Terminal Input），信号编号为 21
        const SIGTTIN = 1 << 21; // 21
        /// 终端输出信号（Terminal Output），信号编号为 22
        const SIGTTOU = 1 << 22; // 22
        /// 紧急信号（Urgent Condition），信号编号为 23
        const SIGURG  = 1 << 23; // 23
        /// CPU 时间限制信号（CPU Time Limit Exceeded），信号编号为 24
        const SIGXCPU = 1 << 24; // 24
        /// 文件大小限制信号（File Size Limit Exceeded），信号编号为 25
        const SIGXFSZ = 1 << 25; // 25
        /// 虚拟定时器信号（Virtual Timer Expired），信号编号为 26
        const SIGVTALRM = 1 << 26; // 26
        /// 性能分析定时器信号（Profiling Timer Expired），信号编号为 27
        const SIGPROF = 1 << 27; // 27
        /// 窗口大小改变信号（Window Size Change），信号编号为 28
        const SIGWINCH = 1 << 28; // 28
        /// I/O 信号（I/O Possible），信号编号为 29
        const SIGIO   = 1 << 29; // 29
        /// 电源故障信号（Power Failure），信号编号为 30
        const SIGPWR  = 1 << 30; // 30
        /// 系统调用错误信号（Bad System Call），信号编号为 31
        const SIGSYS  = 1 << 31; // 31
    }
}

/// 信号处理动作结构体
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    pub handler: usize,      // 信号处理函数地址
    pub mask: SignalFlags,   // 信号掩码
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