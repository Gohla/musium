use std::backtrace::Backtrace;
use std::fmt::{Debug, Formatter};

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool, PooledConnection};
use thiserror::Error;

use crate::password::PasswordHasher;
use crate::sync::local::LocalSync;
use crate::sync::spotify::SpotifySync;

macro_rules! time {
  ($s:expr, $e:expr) => {{
    let start = std::time::Instant::now();
    let result = $e;
    metrics::timing!($s, start.elapsed());
    result
  }}
}

pub mod source;
pub mod album;
pub mod track;
pub mod artist;
pub mod user;
pub mod sync;


#[derive(Clone)]
pub struct Database {
  connection_pool: Pool<ConnectionManager<SqliteConnection>>,
  local_sync: LocalSync,
  spotify_sync: SpotifySync,
  password_hasher: PasswordHasher,
}


// Creation

#[derive(Debug, Error)]
pub enum DatabaseCreateError {
  #[error("Failed to create database connection pool")]
  ConnectionPoolCreateFail(#[from] r2d2::PoolError, Backtrace),
}

impl Database {
  pub fn new<D: AsRef<str>>(
    database_url: D,
    local_sync: LocalSync,
    spotify_sync: SpotifySync,
    password_hasher: PasswordHasher,
  ) -> Result<Database, DatabaseCreateError> {
    let connection_pool = Pool::builder()
      .max_size(16)
      .build(ConnectionManager::<SqliteConnection>::new(database_url.as_ref()))?;
    Ok(Database { connection_pool, local_sync, spotify_sync, password_hasher })
  }
}


// Connecting to the database

pub struct DatabaseConnection<'a> {
  database: &'a Database,
  connection: PooledConnection<ConnectionManager<SqliteConnection>>,
}

#[derive(Debug, Error)]
pub enum DatabaseConnectError {
  #[error("Failed to get database connection from database connection pool")]
  ConnectionGetFail(#[from] r2d2::PoolError, Backtrace),
}

impl Database {
  pub fn connect(&self) -> Result<DatabaseConnection, DatabaseConnectError> {
    let connection = self.connection_pool.get()?;
    Ok(DatabaseConnection { database: self, connection })
  }
}


// Generic database query error.

#[derive(Debug, Error)]
pub enum DatabaseQueryError {
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
}


// Debug implementations

impl Debug for Database {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Ok(write!(f, "Backend")?)
  }
}

impl Debug for DatabaseConnection<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Ok(write!(f, "BackendConnected")?)
  }
}
