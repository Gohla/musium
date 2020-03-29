use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, spawn};

use scopeguard::defer;
use thiserror::Error;

use backend::{Backend, BackendConnectError};

pub struct Scanner {
  thread_handle: Mutex<Option<JoinHandle<Result<(), ScanError>>>>,
  is_working: Arc<AtomicBool>,
}

#[derive(Debug, Error)]
pub enum ScanError {
  #[error(transparent)]
  BackendConnectFail(#[from] BackendConnectError),
  #[error(transparent)]
  ScanFail(#[from] backend::ScanError),
}

impl Scanner {
  pub fn new() -> Self {
    Self {
      thread_handle: Mutex::new(None),
      is_working: Arc::new(AtomicBool::new(false)),
    }
  }
}

impl Scanner {
  pub fn scan(&self, backend: Arc<Backend>) -> bool {
    let is_working = self.is_working.swap(true, Ordering::Relaxed);
    if is_working {
      false
    } else {
      let is_working_clone = self.is_working.clone();
      let mut thread_handle_guard = self.thread_handle.lock().unwrap();
      *thread_handle_guard = Some(spawn(move || {
        // Set is_working to false when this scope ends (normally, erroneously, or when panicking)
        defer!(is_working_clone.store(false, Ordering::Relaxed));
        backend.connect_to_database()?.scan()?;
        Ok(())
      }));
      true
    }
  }
}