// TODO:
// * Make writing to wrong files inexpressible.
// * Don't push strings around -- use place struct instead..
// * Figure out a better way to handle font other than writing heading()
//   everywhere.
// * Document all the fuckery; why contexts are needed in queries, etc.
// * Move queries and drains into logic.
// * Implement settings
//   - Global settings (e.g. what graphs to show as well as their order)
//   - Per-location settings (same as global settings but overwrite for a
//     specific location, as well as stuff like panel tilt and azimuth for GTI,
//     which doesn't make much sense to be made global)
//  Figure out graph interactions:
//   - Add tooltips or explanations. Should possibly be an option, or be at the
//      bottom of the page.
//   - Toggle viewing variables when graphs are clicked.
//   - Show specific value of variables at X coord when clicked.

#[macro_use] extern crate log;

pub mod weather;
pub mod geocoding;
mod storage;
mod app;

pub use app::{App, Platform};
pub use storage::Storage;

#[cfg(target_os = "android")]
mod android_main;
