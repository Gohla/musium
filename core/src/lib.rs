#![feature(backtrace)]
#![feature(try_trait_v2)]

#[cfg(feature = "diesel")]
#[cfg_attr(feature = "diesel", macro_use)] // extern crate with #[macro_use] because diesel does not fully support Rust 2018 yet.
extern crate diesel;

#[cfg(feature = "diesel")]
pub mod schema;
pub mod model;
pub mod api;
pub mod error;
pub mod format_error;
pub mod untagged_result;
pub mod panic;
