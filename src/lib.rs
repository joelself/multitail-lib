extern crate notify;
extern crate libc;
#[cfg(target_os = "macos")]
extern crate fsevent;
pub mod mtail;
#[macro_use]
mod macros;