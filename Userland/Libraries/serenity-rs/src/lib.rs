#![feature(rustc_private)]

pub mod core;
pub mod ipc;

extern "C" {
    pub fn dbgputstr(characters: *const u8, length: usize) -> i32;
}

#[macro_export]
macro_rules! dbgln {
    () => {
        unsafe { serenity::dbgputstr("\n", 1) }
    };
    ($($arg:tt)*) => {
        let mut s = format!($($arg)*);
        s.push('\n');
        unsafe { serenity::dbgputstr(s.as_ptr(), s.len()) }
    };
}
