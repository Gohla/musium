#[macro_use] // macro_use because diesel developers refuse to make create compatible with Rust 2018.
extern crate diesel;

use diesel::prelude::*;
use thiserror::Error;

use core::model::Track;

pub mod schema;

pub struct Server {
  connection: SqliteConnection,
}

// Creation

#[derive(Debug, Error)]
pub enum ServerCreateError {
  #[error(transparent)]
  ConnectionCreateFail(#[from] ConnectionError),
}

impl Server {
  pub fn new<S: AsRef<str>>(database_url: S) -> Result<Server, ServerCreateError> {
    let connection = SqliteConnection::establish(database_url.as_ref())?;
    Ok(Server { connection })
  }
}

// Queries

#[derive(Debug, Error)]
pub enum QueryError {
  #[error(transparent)]
  QueryFail(#[from] diesel::result::Error),
}

impl Server {
  pub fn get_all_tracks(&self) -> Result<Vec<Track>, QueryError> {
    use schema::tracks::dsl::*;

    Ok(tracks.load::<Track>(&self.connection)?)
  }
}
