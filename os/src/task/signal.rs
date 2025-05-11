// os/src/task/signal.rs
use super::exit_current_and_run_next;
use bitflags::*;

bitflags! {
    pub struct SignalFlags: usize {
        /// 挂起信号（Hangup），信号编号为 1
        const SIGHUP  = 1 << 0;  // 1
        /// 中断信号（Interrupt），通常由 Ctrl+C 触发，信号编号为 2
        const SIGINT  = 1 << 1;  // 2
        /// 退出信号（Quit），信号编号为 3
        const SIGQUIT = 1 << 2;  // 3
        /// 非法指令信号（Illegal Instruction），信号编号为 4
        const SIGILL  = 1 << 3;  // 4
        /// 陷阱信号（Trap），信号编号为 5
        const SIGTRAP = 1 << 4;  // 5
        /// 终止信号（Abort），通常由程序错误触发，信号编号为 6
        const SIGABRT = 1 <<5;  // 6
        /// 总线错误信号（Bus Error），信号编号为 7
        const SIGBUS  = 1 << 6;  // 7
        /// 浮点异常信号（Floating-Point Exception），信号编号为 8
        const SIGFPE  = 1 << 7;  // 8
        /// 杀死信号（Kill），信号编号为 9
        const SIGKILL = 1 << 8;  // 9
        /// 用户定义信号 1，信号编号为 10
        const SIGUSR1 = 1 << 9; // 10
        /// 分段错误信号（Segmentation Violation），通常由非法内存访问触发，信号编号为 11
        const SIGSEGV = 1 << 10; // 11
        /// 用户定义信号 2，信号编号为 12
        const SIGUSR2 = 1 << 11; // 12
        /// 管道错误信号（Broken Pipe），信号编号为 13
        const SIGPIPE = 1 << 12; // 13
        /// 闹钟信号（Alarm Clock），信号编号为 14
        const SIGALRM = 1 << 13; // 14
        /// 终止信号（Terminate），信号编号为 15
        const SIGTERM = 1 << 14; // 15
        /// 堆栈错误信号（Stack Fault），信号编号为 16
        const SIGSTKFLT = 1 << 15; // 16
        /// 子进程状态改变信号（Child Status Changed），信号编号为 17
        const SIGCHLD = 1 << 16; // 17
        /// 继续信号（Continue），信号编号为 18
        const SIGCONT = 1 << 17; // 18
        /// 停止信号（Stop），信号编号为 19
        const SIGSTOP = 1 << 18; // 19
        /// 终端停止信号（Terminal Stop），信号编号为 20
        const SIGTSTP = 1 << 19; // 20
        /// 终端输入信号（Terminal Input），信号编号为 21
        const SIGTTIN = 1 << 20; // 21
        /// 终端输出信号（Terminal Output），信号编号为 22
        const SIGTTOU = 1 << 21; // 22
        /// 紧急信号（Urgent Condition），信号编号为 23
        const SIGURG  = 1 << 22; // 23
        /// CPU 时间限制信号（CPU Time Limit Exceeded），信号编号为 24
        const SIGXCPU = 1 << 23; // 24
        /// 文件大小限制信号（File Size Limit Exceeded），信号编号为 25
        const SIGXFSZ = 1 << 24; // 25
        /// 虚拟定时器信号（Virtual Timer Expired），信号编号为 26
        const SIGVTALRM = 1 << 25; // 26
        /// 性能分析定时器信号（Profiling Timer Expired），信号编号为 27
        const SIGPROF = 1 << 26; // 27
        /// 窗口大小改变信号（Window Size Change），信号编号为 28
        const SIGWINCH = 1 << 27; // 28
        /// I/O 信号（I/O Possible），信号编号为 29
        const SIGIO   = 1 << 28; // 29
        /// 电源故障信号（Power Failure），信号编号为 30
        const SIGPWR  = 1 << 29; // 30
        /// 系统调用错误信号（Bad System Call），信号编号为 31
        const SIGSYS  = 1 << 30; // 31
        /// 
        const SIGRTMIN  = 1 << 31; // 31
        /// 
        const SIGRT1  = 1 << 32; // 31
    }
}

bitflags! {
    /// Bits in `sa_flags' used to denote the default signal action.
    pub struct SigActionFlags: usize{
    /// Don't send SIGCHLD when children stop.
        const NOCLDSTOP = 1		   ;
    /// Don't create zombie on child death.
        const NOCLDWAIT = 2		   ;
    /// Invoke signal-catching function with three arguments instead of one.
        const SIGINFO   = 4		   ;
    /// Use signal stack by using `sa_restorer'.
        const ONSTACK   = 0x08000000;
    /// Restart syscall on signal return.
        const RESTART   = 0x10000000;
    /// Don't automatically block the signal when its handler is being executed.
        const NODEFER   = 0x40000000;
    /// Reset to SIG_DFL on entry to handler.
        const RESETHAND = 0x80000000;
    /// Historical no-op.
        const INTERRUPT = 0x20000000;
    /// Use signal trampoline provided by C library's wrapper function.
        const RESTORER  = 0x04000000;
    }
}


/// 信号处理动作结构体
#[cfg(any(target_arch = "riscv64"))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    pub handler: usize,      // 信号处理函数地址
    pub flags: SigActionFlags,
    pub restore: usize,
    pub mask: SignalFlags,   // 信号掩码
}

/// 信号处理动作结构体
#[cfg(any(target_arch = "loongarch64"))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SigAction {
    pub handler: usize,      // 信号处理函数地址
    pub flags: SigActionFlags,
    pub mask: SignalFlags,   // 信号掩码
    pub restore: usize,
}

impl SigAction{
    pub fn new(signo: usize) -> Self {
        let handler: usize;
        if signo == 0 {
            handler = 1;
        } else {
            handler = match SignalFlags::from_bits(1 << (signo - 1)).unwrap() {
                SignalFlags::SIGCONT |  //continue_signals
                SignalFlags::SIGCHLD | //ignore_signals
                SignalFlags::SIGURG |
                SignalFlags::SIGWINCH => 1,
                SignalFlags::SIGSTOP | //stop_signals
                SignalFlags::SIGTSTP |
                SignalFlags::SIGTTIN |
                SignalFlags::SIGTTOU => 1,  
                SignalFlags::SIGHUP | //terminate_signals
                SignalFlags::SIGINT |
                SignalFlags::SIGKILL|
                SignalFlags::SIGUSR1|
                SignalFlags::SIGUSR2|
                SignalFlags::SIGPIPE|
                SignalFlags::SIGALRM|
                SignalFlags::SIGTERM|
                SignalFlags::SIGSTKFLT|
                SignalFlags::SIGVTALRM|
                SignalFlags::SIGPROF|
                SignalFlags::SIGIO|
                SignalFlags::SIGPWR|
                SignalFlags::SIGILL| //dump_signals
                SignalFlags::SIGQUIT |
                SignalFlags::SIGTRAP|
                SignalFlags::SIGABRT|
                SignalFlags::SIGBUS |
                SignalFlags::SIGFPE |
                SignalFlags::SIGSEGV|
                SignalFlags::SIGXCPU|
                SignalFlags::SIGXFSZ|
                SignalFlags::SIGSYS => exit_current_and_run_next as usize,
                _ =>{
                    panic!("unimplemented! {}",signo);
                }
            }
        };
        Self {
            handler: handler,
            flags: SigActionFlags::empty(),
            restore: 0,
            mask: SignalFlags::empty(),
        }
    }
    pub fn ignore()->Self{
        Self {
            handler: 1,
            flags: SigActionFlags::empty(),
            restore: 0,
            mask: SignalFlags::empty(),
        }
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


#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SigInfo {
    pub signum: i32,
    pub code: i32,
    pub details: SigDetails,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum SigDetails {
    None,
    Kill {
        /// sender's pid
        pid: usize,
    },
}

#[allow(unused)]
impl SigInfo {
    /// sent by kill, sigsend, raise
    pub const USER: i32 = 0;
    /// sent by the kernel from somewhere
    pub const KERNEL: i32 = 0x80;
    /// sent by sigqueue
    pub const QUEUE: i32 = -1;
    /// sent by timer expiration
    pub const TIMER: i32 = -2;
    /// sent by real time mesq state change
    pub const MESGQ: i32 = -3;
    /// sent by AIO completion
    pub const ASYNCIO: i32 = -4;
    /// sent by queued SIGIO
    pub const SIGIO: i32 = -5;
    /// sent by tkill system call
    pub const TKILL: i32 = -6;
    /// sent by execve() killing subsidiary threads
    pub const DETHREAD: i32 = -7;
    /// sent by glibc async name lookup completion
    pub const ASYNCNL: i32 = -60;

    // SIGCHLD si_codes
    /// child has exited
    pub const CLD_EXITED: i32 = 1;
    /// child was killed
    pub const CLD_KILLED: i32 = 2;
    /// child terminated abnormally
    pub const CLD_DUMPED: i32 = 3;
    /// traced child has trapped
    pub const CLD_TRAPPED: i32 = 4;
    /// child has stopped
    pub const CLD_STOPPED: i32 = 5;
    /// stopped child has continued
    pub const CLD_CONTINUED: i32 = 6;
    pub const NSIGCHLD: i32 = 6;
}

bitflags! {
    pub struct SignalStackFlags : u32 {
        const ONSTACK = 1;
        const DISABLE = 2;
        const AUTODISARM = 0x80000000;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SignalStack {
    pub sp: usize,
    pub flags: u32,
    pub size: usize,
}

impl SignalStack {
    pub fn new(sp: usize, size: usize) -> Self {
        SignalStack {
            sp,
            flags: SignalStackFlags::DISABLE.bits,
            size,
        }
    }
}


#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct GeneralRegs {
    pub x: [usize; 32],
}
#[cfg(any(target_arch = "riscv64"))]
/// FP registers
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct FloatRegs {
    pub f: [usize; 32],
    pub fcsr: u32,
}

#[cfg(any(target_arch = "riscv64"))]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct MachineContext {
    gp: GeneralRegs,
    fp: FloatRegs,
}
#[cfg(any(target_arch = "riscv64"))]
pub fn into_mcontext(trap_cx:&arch::TrapFrame)->MachineContext{
    MachineContext{
        gp:GeneralRegs{x:trap_cx.x,},            
        fp:FloatRegs{
                f:[0usize;32],
                fcsr:0
            }
    }
}

#[cfg(any(target_arch = "riscv64"))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UserContext {
    pub flags: usize,
    pub link: usize,
    pub stack: SignalStack,
    pub sigmask: SignalFlags,
    pub __pad: [u8; 128],
    pub mcontext: MachineContext,
}


#[cfg(any(target_arch = "loongarch64"))]
pub fn into_mcontext(trap_cx:&arch::TrapFrame)->MachineContext{
    MachineContext{
        gp:GeneralRegs{x:trap_cx.regs,},            
        fp:FloatRegs{
                f:[0usize;32],
                fcsr:0,
                fcc:0
            }
    }
}
#[cfg(any(target_arch = "loongarch64"))]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct FloatRegs {
    pub f: [usize; 32],
    pub fcsr: u32,
    pub fcc: u8,
}


#[cfg(any(target_arch = "loongarch64"))]
#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct MachineContext {
    gp:GeneralRegs,
    fp: FloatRegs,
}

#[cfg(any(target_arch = "loongarch64"))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UserContext {
    pub flags: usize,
    pub link: usize,
    pub stack: SignalStack,
    pub sigmask: SignalFlags,
    pub __pad: [u8; 128],
    pub mcontext: MachineContext,
}
