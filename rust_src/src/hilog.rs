pub mod hilog_sys {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    use std::os::raw::c_int;

    pub const LOG_APP: c_int = 0;

    pub const LOG_DEBUG: c_int = 3;
    pub const LOG_INFO: c_int = 4;
    pub const LOG_WARN: c_int = 5;
    pub const LOG_ERROR: c_int = 6;
    pub const LOG_FATAL: c_int = 7;

    pub type LogType = c_int;
    pub type LogLevel = c_int;

    extern "C" {
        pub fn OH_LOG_Print(
            type_: LogType,
            level: LogLevel,
            domain: u32,
            tag: *const i8,
            fmt: *const i8,
            ...
        ) -> c_int;

        pub fn OH_LOG_IsLoggable(
            domain: u32,
            tag: *const i8,
            level: LogLevel,
        ) -> bool;
    }
}

use std::ffi::CString;

const LOG_DOMAIN: u32 = 0x3200;
const LOG_TAG: &str = "VFS_APIS";

#[allow(dead_code)]
pub enum LogLevel {
    Debug = 3,
    Info = 4,
    Warn = 5,
    Error = 6,
    Fatal = 7,
}

#[allow(dead_code)]
pub fn log_debug(message: &str) {
    log_internal(LogLevel::Debug, message);
}

pub fn log_info(message: &str) {
    log_internal(LogLevel::Info, message);
}

pub fn log_warn(message: &str) {
    log_internal(LogLevel::Warn, message);
}

pub fn log_error(message: &str) {
    log_internal(LogLevel::Error, message);
}

#[allow(dead_code)]
pub fn log_fatal(message: &str) {
    log_internal(LogLevel::Fatal, message);
}

#[allow(dead_code)]
pub fn log(level: LogLevel, message: &str) {
    log_internal(level, message);
}

fn log_internal(level: LogLevel, message: &str) {
    let tag_c = match CString::new(LOG_TAG) {
        Ok(c) => c,
        Err(_) => return,
    };
    
    let msg_c = match CString::new(message) {
        Ok(c) => c,
        Err(_) => return,
    };
    
    let fmt_c = match CString::new("%{public}s") {
        Ok(c) => c,
        Err(_) => return,
    };
    
    unsafe {
        hilog_sys::OH_LOG_Print(
            hilog_sys::LOG_APP,
            level as i32,
            LOG_DOMAIN,
            tag_c.as_ptr() as *const i8,
            fmt_c.as_ptr() as *const i8,
            msg_c.as_ptr(),
        );
    }
}

#[macro_export]
macro_rules! vfs_log_debug {
    ($($arg:tt)*) => {
        $crate::hilog::log_debug(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! vfs_log_info {
    ($($arg:tt)*) => {
        $crate::hilog::log_info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! vfs_log_warn {
    ($($arg:tt)*) => {
        $crate::hilog::log_warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! vfs_log_error {
    ($($arg:tt)*) => {
        $crate::hilog::log_error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! vfs_log_fatal {
    ($($arg:tt)*) => {
        $crate::hilog::log_fatal(&format!($($arg)*))
    };
}
