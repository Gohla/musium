use std::error::Error as StdError;
use std::sync::{Arc, RwLock};

use thiserror::Error;
use tokio::{self, sync::{mpsc, oneshot, watch}, task};
use tracing::{event, instrument, Level};

use musium_core::api::SyncStatus;
use musium_core::format_error::FormatError;
use musium_core::panic::try_panic_into_string;

use crate::database::{Database, DatabaseConnection};

// Creation

#[derive(Clone)]
pub struct SyncClient {
  tx: mpsc::Sender<Request>,
  worker_task: Arc<task::JoinHandle<()>>,
}

impl SyncClient {
  pub fn new() -> Self {
    let (tx, rx) = mpsc::channel(32);
    let worker_task = Arc::new(tokio::spawn(async move {
      SyncWorker { rx, sync_task: Arc::new(RwLock::new(None)) }.run().await;
    }));
    Self { tx, worker_task }
  }
}

// Destruction

#[derive(Debug, Error)]
pub enum SyncClientDestroyError {
  #[error("Cannot destroy the sync client because one or more clones still exist. All clones must be dropped before stopping so that the worker task can gracefully stop")]
  ClonesStillExist,
  #[error("Rodio audio output was destroyed, but the worker thread panicked before stopping with message: {0}")]
  TaskPanicked(String),
  #[error("Rodio audio output was destroyed, but the worker thread panicked before stopping without a message")]
  TaskPanickedSilently,
}

impl SyncClient {
  /// Destroys this sync client. Returns an error if it cannot be destroyed because there are still clones around, or
  /// if the worker task was stopped but panicked.
  ///
  /// Dropping this sync client and all its clones will also properly destroy it but ignores the panic produced by the
  /// worker task (if any), and does not wait for the worker task to complete first.
  pub async fn destroy(self) -> Result<(), SyncClientDestroyError> {
    use SyncClientDestroyError::*;
    let SyncClient { tx, worker_task } = self;
    drop(tx); // Dropping sender will cause the worker task to break out of the loop and complete.
    let worker_task = Arc::try_unwrap(worker_task).map_err(|_| ClonesStillExist)?;
    worker_task.abort(); // Also aborting the worker task just in case.
    if let Err(e) = worker_task.await {
      if let Ok(panic) = e.try_into_panic() {
        return if let Some(msg) = try_panic_into_string(panic) {
          Err(TaskPanicked(msg))
        } else {
          Err(TaskPanickedSilently)
        };
      }
    }
    Ok(())
  }
}

// Syncing

#[derive(Debug, Error)]
pub enum SyncClientError {
  #[error("Failed to send command because worker task was stopped")]
  SendCommandFail,
  #[error("Failed to receive sync status because worker task was stopped")]
  ReceiveSyncStatusFail,
}

impl SyncClient {
  #[instrument(skip(self, database))]
  pub async fn get_status(&self, database: Arc<Database>) -> Result<SyncStatus, SyncClientError> {
    self.send_receive(Command::GetStatus, database).await
  }

  #[instrument(skip(self, database))]
  pub async fn sync_all_sources(&self, database: Arc<Database>) -> Result<SyncStatus, SyncClientError> {
    self.send_receive(Command::SyncAll, database).await
  }

  #[instrument(skip(self, database))]
  pub async fn sync_local_sources(&self, database: Arc<Database>) -> Result<SyncStatus, SyncClientError> {
    self.send_receive(Command::SyncLocalSources, database).await
  }

  #[instrument(skip(self, database))]
  pub async fn sync_local_source(&self, local_source_id: i32, database: Arc<Database>) -> Result<SyncStatus, SyncClientError> {
    self.send_receive(Command::SyncLocalSource(local_source_id), database).await
  }

  #[instrument(skip(self, database))]
  pub async fn sync_spotify_sources(&self, database: Arc<Database>) -> Result<SyncStatus, SyncClientError> {
    self.send_receive(Command::SyncSpotifySources, database).await
  }

  #[instrument(skip(self, database))]
  pub async fn sync_spotify_source(&self,  spotify_source_id: i32, database: Arc<Database>) -> Result<SyncStatus, SyncClientError> {
    self.send_receive(Command::SyncSpotifySource(spotify_source_id), database).await
  }
}

// Internals

impl SyncClient {
  async fn send_receive(&self, command: Command, database: Arc<Database>) -> Result<SyncStatus, SyncClientError> {
    use SyncClientError::*;
    let (request, rx) = Request::new(command, database);
    self.tx.send(request).await.map_err(|_| SendCommandFail)?;
    Ok(rx.await.map_err(|_| ReceiveSyncStatusFail)?)
  }
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

impl Request {
  fn new(command: Command, database: Arc<Database>) -> (Self, oneshot::Receiver<SyncStatus>) {
    let (tx, rx) = oneshot::channel();
    (Self { command, database, tx }, rx)
  }
}


struct SyncWorker {
  rx: mpsc::Receiver<Request>,
  sync_task: Arc<RwLock<Option<SyncTask>>>,
}

struct SyncTask {
  // TODO: do we need to keep on to this handle?
  _handle: task::JoinHandle<()>,
  rx: watch::Receiver<SyncStatus>,
}

impl SyncWorker {
  #[instrument(skip(self))]
  async fn run(mut self) {
    while let Some(request) = self.rx.recv().await {
      let tx = request.tx;
      let db = request.database;
      match request.command {
        Command::GetStatus => {
          // OK: receiver hung up -> we don't care.
          tx.send(Self::get_sync_status(&self.sync_task).unwrap_or(SyncStatus::Idle)).ok();
        }
        Command::SyncAll => {
          tx.send(Self::get_sync_status(&self.sync_task).unwrap_or_else(
            || Self::do_sync(self.sync_task.clone(), db, move |c| c.sync_all_sources())
          )).ok(); // OK: receiver hung up -> we don't care.
        }
        Command::SyncLocalSources => {
          tx.send(Self::get_sync_status(&self.sync_task).unwrap_or_else(
            || Self::do_sync(self.sync_task.clone(), db, move |c| c.sync_local_sources())
          )).ok(); // OK: receiver hung up -> we don't care.
        }
        Command::SyncLocalSource(local_source_id) => {
          tx.send(Self::get_sync_status(&self.sync_task).unwrap_or_else(
            || Self::do_sync(self.sync_task.clone(), db, move |c| c.sync_local_source(local_source_id))
          )).ok(); // OK: receiver hung up -> we don't care.
        }
        Command::SyncSpotifySources => {
          tx.send(Self::get_sync_status(&self.sync_task).unwrap_or_else(
            || Self::do_sync(self.sync_task.clone(), db, move |c| c.sync_spotify_sources())
          )).ok(); // OK: receiver hung up -> we don't care.
        }
        Command::SyncSpotifySource(spotify_source_id) => {
          tx.send(Self::get_sync_status(&self.sync_task).unwrap_or_else(
            || Self::do_sync(self.sync_task.clone(), db, move |c| c.sync_spotify_source(spotify_source_id))
          )).ok(); // OK: receiver hung up -> we don't care.
        }
      };
    };
  }

  #[instrument(skip(sync_task))]
  fn get_sync_status(sync_task: &Arc<RwLock<Option<SyncTask>>>) -> Option<SyncStatus> {
    // UNWRAP: errors if writer has panicked -> we panic as well.
    sync_task.clone().read().unwrap().as_ref().map(|st| *st.rx.borrow())
  }

  #[instrument(skip(sync_task, db, sync))]
  fn do_sync<E: StdError>(
    sync_task: Arc<RwLock<Option<SyncTask>>>,
    db: Arc<Database>,
    sync: impl 'static + Send + FnOnce(DatabaseConnection) -> Result<(), E>,
  ) -> SyncStatus {
    let sync_status = SyncStatus::Started(None);
    let (progress_tx, rx) = watch::channel(sync_status);
    let set_sync_task = sync_task.clone();
    // Lock before spawning to ensure we have the write lock, so we can set the sync task to Some value before the task
    // sets it to None again when it is finished.
    // UNWRAP: errors if writer has panicked -> we panic as well.
    let mut set_sync_task_lock = set_sync_task.write().unwrap();
    let handle = task::spawn_blocking(move || {
      progress_tx.send(SyncStatus::Busy(None)).ok(); // OK: receiver hung up -> we don't care.
      match db.connect() {
        Ok(c) => match sync(c) {
          Ok(_) => { progress_tx.send(SyncStatus::Completed).ok(); } // OK: receiver hung up -> we don't care.
          Err(e) => {
            event!(Level::ERROR, "{:?}", FormatError::new(&e));
            progress_tx.send(SyncStatus::Failed).ok(); // OK: receiver hung up -> we don't care.
          }
        }
        Err(e) => {
          event!(Level::ERROR, "{:?}", FormatError::new(&e));
          progress_tx.send(SyncStatus::Failed).ok(); // OK: receiver hung up -> we don't care.
        }
      };
      // UNWRAP: errors if writer has panicked -> we panic as well.
      *sync_task.write().unwrap() = None;
    });
    *set_sync_task_lock = Some(SyncTask { _handle: handle, rx });
    sync_status
  }
}
