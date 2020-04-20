use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, spawn};

use scopeguard::defer;
use thiserror::Error;

use musium_backend::database::{Database, DatabaseConnectError, sync::SyncError as DatabaseSyncError};

pub struct Sync {
  thread_handle: Mutex<Option<JoinHandle<Result<(), SyncError>>>>,
  is_working: Arc<AtomicBool>,
}

#[derive(Debug, Error)]
pub enum SyncError {
  #[error(transparent)]
  DatabaseConnectFail(#[from] DatabaseConnectError),
  #[error(transparent)]
  SyncFail(#[from] DatabaseSyncError),
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
        database.connect()?.sync()?;
        Ok(())
      }));
      true
    }
  }
}
