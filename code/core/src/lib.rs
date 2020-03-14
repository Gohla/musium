#[macro_use] // extern crate with #[macro_use] because diesel does not fully support Rust 2018 yet.
extern crate diesel;

pub mod schema;
pub mod model;