#[macro_use] extern crate log;

pub mod weather;
pub mod geocoding;
mod storage;
mod app;

pub use app::{App, Platform};
pub use storage::Storage;

#[cfg(target_os = "android")]
mod android_main;
