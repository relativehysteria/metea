// TODO:
// * Don't push strings around -- use place struct instead..
// * Figure out a better way to handle font other than writing heading()
//   everywhere.
// * Document all the fuckery; why contexts are needed in queries, etc.
//   Move queries and drains into logic.

#[macro_use] extern crate log;

pub mod weather;
pub mod geocoding;
mod internal_storage;
mod android_main;
mod app;

pub use app::App;
pub use internal_storage::InternalStorage;
