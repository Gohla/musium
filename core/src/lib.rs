#![feature(backtrace)]
#![feature(try_trait)]

#[macro_use] // extern crate with #[macro_use] because diesel does not fully support Rust 2018 yet.
extern crate diesel;

pub mod schema;
pub mod model;
pub mod api;
pub mod format_error;
pub mod untagged_result;
