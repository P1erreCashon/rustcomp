// os/src/task/action.rs

use crate::task::signal::{SigAction, SignalFlags,SigActionFlags};
use super::MAX_SIG; // 从父模块导入 MAX_SIG

/// 信号处理函数表，包含每个信号的处理函数
#[derive(Clone)]
pub struct SignalActions {
    pub table: [SigAction; MAX_SIG + 1], // 信号编号从 0 到 MAX_SIG
}

impl SignalActions {
    /// 创建一个新的 SignalActions，初始化所有信号的处理函数为空
    pub fn new() -> Self {
        // 初始化所有信号的处理函数为默认值（handler = 0，mask 为空）
        let default_action = SigAction {
            handler: 0, // 默认无处理函数
            flags:SigActionFlags::empty(),
            restore: 0,
            mask: SignalFlags::empty(),
        };
        SignalActions {
            table: [default_action; MAX_SIG + 1],
        }
    }
}

impl Default for SignalActions {
    fn default() -> Self {
        Self::new()
    }
}