use std::error::Error as StdError;
use std::sync::{Arc, RwLock};

use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tracing::{event, instrument, Level};

use musium_core::api::SyncStatus;
use musium_core::format_error::FormatError;

use crate::database::{Database, DatabaseConnectError, DatabaseConnection, sync::SyncAllSourcesError};
use std::ops::Deref;

#[derive(Clone)]
pub struct SyncClient {
  tx: mpsc::Sender<Request>,
  server_task: tokio::task::JoinHandle<()>,
}

impl SyncClient {
  pub fn new() -> Self {
    let (tx, mut rx) = mpsc::channel(32);
    let server_task = tokio::spawn(async move {
      SyncServer { rx, sync_task: RwLock::new(None) }.run().await;
    });
    Self { tx, server_task }
  }
}


struct SyncServer {
  rx: mpsc::Receiver<Request>,
  sync_task: RwLock<Option<SyncTask>>,
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
  handle: tokio::task::JoinHandle<()>,
  rx: watch::Receiver<SyncStatus>,
}

impl SyncServer {
  async fn run(mut self) {
    while let Some(request) = self.rx.recv().await {
      let tx = request.tx;
      let db = request.database;
      match request.command {
        Command::GetStatus => {
          let lock = self.sync_task.read().unwrap();
          match lock.deref() {
            None => tx.send(SyncStatus::Idle).ok(),
            Some(sync_task) => tx.send(sync_task.rx.borrow().clone()).ok(),
          };
        }
        Command::SyncAll => {
          let lock = self.sync_task.read().unwrap();
          if let Some(sync_task) = lock.deref() {
            tx.send(sync_task.rx.borrow().clone());
          } else {
            drop(lock);
            self.do_sync(db, |connection| connection.sync_all_sources());
          }
        }
        Command::SyncLocalSources => {}
        Command::SyncLocalSource(_) => {}
        Command::SyncSpotifySources => {}
        Command::SyncSpotifySource(_) => {}
      };
    };
  }

  async fn do_sync<E: StdError + From<DatabaseConnectError>>(&self, db: Arc<Database>, sync: impl FnOnce(DatabaseConnection) -> Result<(), E>) {
    let (progress_tx, rx) = watch::channel(SyncStatus::Busy(None));
    let handle = tokio::task::spawn_blocking(move || {
      match (|| -> Result<(), E> { Ok(sync(db.connect()?)?) })() {
        Ok(_) => progress_tx.send(SyncStatus::Completed),
        Err(e) => {
          let format_error = FormatError::new(&e);
          event!(Level::ERROR, "{:?}", format_error);
          progress_tx.send(SyncStatus::Failed)
        }
      };
      {
        let lock = self.sync_task.write().unwrap();
        *lock = None;
      }
    });
    {
      let lock = self.sync_task.write().unwrap();
      *lock = Some(SyncTask { handle, rx });
    }
  }
}
