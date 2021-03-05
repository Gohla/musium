use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::sync::Arc;

use diesel::connection::TransactionManager;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager, Pool, PooledConnection};
use thiserror::Error;

use musium_spotify_client::SpotifyClient;

use crate::password::PasswordHasher;

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
pub mod local_track;
pub mod spotify_track;
pub mod artist;
pub mod user;
pub mod sync;


#[derive(Clone)]
pub struct Database {
  connection_pool: Pool<ConnectionManager<SqliteConnection>>,
  inner: Arc<Inner>,
}

struct Inner {
  spotify_sync: SpotifyClient,
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
    spotify_sync: SpotifyClient,
    password_hasher: PasswordHasher,
  ) -> Result<Database, DatabaseCreateError> {
    let connection_pool = Pool::builder()
      .max_size(16)
      .build(ConnectionManager::<SqliteConnection>::new(database_url.as_ref()))?;
    let inner = Arc::new(Inner { spotify_sync, password_hasher });
    Ok(Database { connection_pool, inner })
  }
}


// Connecting to the database

pub struct DatabaseConnection {
  connection: PooledConnection<ConnectionManager<SqliteConnection>>,
  inner: Arc<Inner>,
}

#[derive(Debug, Error)]
pub enum DatabaseConnectError {
  #[error("Failed to get database connection from database connection pool")]
  ConnectionGetFail(#[from] r2d2::PoolError, Backtrace),
}

impl Database {
  pub fn connect(&self) -> Result<DatabaseConnection, DatabaseConnectError> {
    let connection = self.connection_pool.get()?;
    let inner = self.inner.clone();
    Ok(DatabaseConnection { connection, inner })
  }
}


// Generic database query error.

#[derive(Debug, Error)]
pub enum DatabaseQueryError {
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
}


// Async transaction

#[derive(Debug, Error)]
pub enum TransactionError<E: 'static + StdError + Send> {
  #[error("Failed to begin, commit, or roll back a transaction")]
  TransactionFail(#[source] diesel::result::Error),
  #[error("Failure during transaction")]
  InsideTransactionFail(#[source] E),
}

impl DatabaseConnection {
  pub async fn run_in_transaction<T, E: 'static + StdError + Send>(&self, future: impl Future<Output=Result<T, E>> + 'static + Send) -> Result<T, TransactionError<E>> {
    use TransactionError::*;
    self.connection.transaction_manager().begin_transaction(&self.connection).map_err(TransactionFail)?;
    let result = future.await.map_err(InsideTransactionFail);
    match &result {
      Ok(_) => self.connection.transaction_manager().commit_transaction(&self.connection).map_err(TransactionFail)?,
      Err(_) => self.connection.transaction_manager().rollback_transaction(&self.connection).map_err(TransactionFail)?,
    }
    result
  }
}


// Debug implementations

impl Debug for Database {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Ok(write!(f, "Backend")?)
  }
}

impl Debug for DatabaseConnection {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Ok(write!(f, "BackendConnected")?)
  }
}
