use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, spawn};

use scopeguard::defer;
use thiserror::Error;
use tracing::{event, instrument, Level};
use std::error::Error;

use musium_backend::database::{Database, DatabaseConnectError, sync::SyncError as DatabaseSyncError};
use std::backtrace::Backtrace;

pub struct Sync {
  thread_handle: Mutex<Option<JoinHandle<Result<(), SyncError>>>>,
  is_working: Arc<AtomicBool>,
}

#[derive(Debug, Error)]
pub enum SyncError {
  #[error("Database connection failure")]
  DatabaseConnectFail(#[from] DatabaseConnectError, Backtrace),
  #[error("Synchronization failure")]
  SyncFail(#[from] DatabaseSyncError, Backtrace),
}

impl Sync {
  pub fn new() -> Self {
    Self {
      thread_handle: Mutex::new(None),
      is_working: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl Sync {
  #[instrument(skip(self, database), level = "trace")]
  pub fn sync(&self, database: Arc<Database>) -> bool {
    let is_working = self.is_working.swap(true, Ordering::Relaxed);
    if is_working {
      false
    } else {
      let is_working_clone = self.is_working.clone();
      let mut thread_handle_guard = self.thread_handle.lock().unwrap();
      *thread_handle_guard = Some(spawn(move || {
        // Set is_working to false when this scope ends (normally, erroneously, or when panicking)
        defer!(is_working_clone.store(false, Ordering::Relaxed));
        if let Err(e) = (|| -> Result<(), SyncError> { Ok(database.connect()?.sync()?) })() {
          event!(Level::ERROR, "Synchronization failed: {}.\nError:\n\t{:?}\nBacktrace:\n{}", e, e, e.backtrace().unwrap());
        }
        Ok(())
      }));
      true
    }
  }
}
