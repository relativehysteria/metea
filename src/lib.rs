#[macro_use] extern crate log;

pub mod geocoding;
mod internal_storage;
mod android_main;
mod app;

pub use app::App;
pub use internal_storage::InternalStorage;
