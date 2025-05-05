#![no_std]
#![no_main]

use log::*;
struct Logger;

#[crate_interface::def_interface]
pub trait LogIf: Send + Sync {
    fn print_log(record: &Record);
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Error// 在这里修改日志等级
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // 这里实现你的日志输出，例如通过串口或其他方式
            // 这里仅打印到控制台作为示例
            LogIf::print_log(record);
            //println!("{}: {}", record.level(), record.args());
                
        }
    }

    fn flush(&self) {}
}
///
pub fn init_logger() {
    log::set_logger(&Logger).map(|()| log::set_max_level(LevelFilter::Debug)).unwrap();
}
///模块日志等级名为 module_level
///将模块日志等级写进宏里
#[macro_export]
macro_rules! log_error {
    // debug!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // debug!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        if log::Level::Error >= MODULE_LEVEL{
            (log::log!(target: $target, log::Level::Error, $($arg)+))
        } 
    };

    // debug!("a {} event", "log")
    ($($arg:tt)+) => {
        if log::Level::Error >= MODULE_LEVEL{
            (log::log!(log::Level::Error, $($arg)+))
        }
    }
}
///
#[macro_export]
macro_rules! log_warn {
    // debug!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // debug!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        if log::Level::Warn >= MODULE_LEVEL{
            (log::log!(target: $target, log::Level::Warn, $($arg)+))
        } 
    };

    // debug!("a {} event", "log")
    ($($arg:tt)+) => {
        if log::Level::Warn >= MODULE_LEVEL{
            (log::log!(log::Level::Warn, $($arg)+))
        }
    }
}
///
#[macro_export]
macro_rules! log_info {
    // debug!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // debug!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        if log::Level::Info >= MODULE_LEVEL{
            (log::log!(target: $target, log::Level::Info, $($arg)+))
        } 
    };

    // debug!("a {} event", "log")
    ($($arg:tt)+) => {
        if log::Level::Info >= MODULE_LEVEL{
            (log::log!(log::Level::Info, $($arg)+))
        }
    }
}
///
#[macro_export]
macro_rules! log_debug {
    // debug!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // debug!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        if log::Level::Debug >= MODULE_LEVEL{
            (log::log!(target: $target, log::Level::Debug, $($arg)+))
        } 
    };

    // debug!("a {} event", "log")
    ($($arg:tt)+) => {
        if log::Level::Debug >= MODULE_LEVEL{
            (log::log!(log::Level::Debug, $($arg)+))
        }
    }
}
///
#[macro_export]
macro_rules! log_trace {
    // debug!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // debug!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => {
        if log::Level::Trace >= MODULE_LEVEL{
            (log::log!(target: $target, log::Level::Trace, $($arg)+))
        } 
    };

    // debug!("a {} event", "log")
    ($($arg:tt)+) => {
        if log::Level::Trace >= MODULE_LEVEL{
            (log::log!(log::Level::Trace, $($arg)+))
        }
    }
}
