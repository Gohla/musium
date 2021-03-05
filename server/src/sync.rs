use std::backtrace::Backtrace;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{JoinHandle, spawn};

use scopeguard::defer;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tracing::{event, instrument, Level};

use musium_backend::database::{Database, DatabaseConnectError, sync::SyncError as DatabaseSyncError};
use musium_core::api::SyncStatus;
use musium_core::format_error::FormatError;

// pub struct Sync {
//   thread_handle: Mutex<Option<JoinHandle<Result<(), SyncError>>>>,
//   is_working: Arc<AtomicBool>,
// }
//
// #[derive(Debug, Error)]
// pub enum SyncError {
//   #[error("Database connection failure")]
//   DatabaseConnectFail(#[from] DatabaseConnectError, Backtrace),
//   #[error("Synchronization failure")]
//   SyncFail(#[from] DatabaseSyncError, Backtrace),
// }
//
// impl Sync {
//   pub fn new() -> Self {
//     Self {
//       thread_handle: Mutex::new(None),
//       is_working: Arc::new(AtomicBool::new(false)),
//     }
//   }
// }
//
// impl Sync {
//   #[instrument(skip(self, database), level = "trace")]
//   pub fn sync(&self, database: Arc<Database>) -> bool {
//     let is_working = self.is_working.swap(true, Ordering::SeqCst);
//     if is_working {
//       false
//     } else {
//       let is_working_clone = self.is_working.clone();
//       let mut thread_handle_guard = self.thread_handle.lock().unwrap();
//       *thread_handle_guard = Some(spawn(move || {
//         // Set is_working to false when this scope ends (normally, erroneously, or when panicking)
//         defer!(is_working_clone.store(false, Ordering::SeqCst));
//         if let Err(e) = (|| -> Result<(), SyncError> { Ok(database.connect()?.sync()?) })() {
//           let format_error = FormatError::new(&e);
//           event!(Level::ERROR, "{:?}", format_error);
//         }
//         Ok(())
//       }));
//       true
//     }
//   }
// }

#[derive(Debug, Error)]
pub enum SyncError {
  #[error("Database connection failure")]
  DatabaseConnectFail(#[from] DatabaseConnectError, Backtrace),
  #[error("Synchronization failure")]
  SyncFail(#[from] DatabaseSyncError, Backtrace),
}

#[derive(Clone)]
pub struct SyncClient {
  tx: mpsc::Sender<Request>,
  server_task: tokio::task::JoinHandle<()>,
}

impl SyncClient {
  pub fn new() -> Self {
    let (tx, mut rx) = mpsc::channel(32);
    let server_task = tokio::spawn(async move {
      SyncServer { rx, sync_task: None }.run().await;
    });
    Self { tx, server_task: server_task }
  }
}


struct SyncServer {
  rx: mpsc::Receiver<Request>,
  sync_task: Option<SyncTask>,
}

struct Request {
  command: Command,
  database: Arc<Database>,
  tx: oneshot::Sender<SyncStatus>,
}

enum Command {
  GetStatus,
  SyncAll,
  SyncLocalSources,
  SyncLocalSource(i32),
  SyncSpotifySources,
  SyncSpotifySource(i32),
}

struct SyncTask {
  handle: tokio::task::JoinHandle<Result<(), SyncError>>,
  rx: watch::Receiver<SyncStatus>,
}

impl SyncServer {
  async fn run(mut self) {
    while let Some(request) = self.rx.recv().await {
      let tx = request.tx;
      let db = request.database;
      match request.command {
        Command::GetStatus => {
          match &self.sync_task {
            None => tx.send(SyncStatus::Idle),
            Some(sync_task) => {
              tx.send(sync_task.rx.borrow().clone());
            }
          }
        }
        Command::SyncAll => {
          if let Some(sync_task) = &self.sync_task {
            tx.send(sync_task.rx.borrow().clone());
          } else {
            let (progress_tx, rx) = watch::channel(SyncStatus::Busy(None));
            let handle = tokio::spawn(async move {
              match (|| -> Result<(), SyncError> { Ok(db.connect()?.sync_all_sources().await?) })() {
                Ok(_) => progress_tx.send(SyncStatus::Completed),
                Err(e) => {
                  let format_error = FormatError::new(&e);
                  event!(Level::ERROR, "{:?}", format_error);
                  progress_tx.send(SyncStatus::Failed)
                }
              };
            });
            self.sync_task = Some(SyncTask { handle, rx });
          }
        }
        Command::SyncLocalSources => {}
        Command::SyncLocalSource(_) => {}
        Command::SyncSpotifySources => {}
        Command::SyncSpotifySource(_) => {}
      }
    }
  }
}
