#![feature(backtrace)]

#[macro_use] // extern crate with #[macro_use] because diesel does not fully support Rust 2018 yet.
extern crate diesel;

pub mod database;
pub mod model;
pub mod password;
