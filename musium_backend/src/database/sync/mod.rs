use std::backtrace::Backtrace;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{instrument};

use musium_core::model::{LocalSource, SpotifySource};
use musium_filesystem_sync::FilesystemSyncError;

use crate::database::{DatabaseConnection, DatabaseQueryError};
use crate::database::sync::local::LocalSyncDatabaseError;

pub mod local;
pub mod spotify;

#[derive(Debug, Error)]
pub enum SyncError {
  #[error("Failed to list sources")]
  ListSourcesFail(#[from] DatabaseQueryError, Backtrace),
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error(transparent)]
  LocalSyncFail(#[from] LocalSyncDatabaseError),
  #[error("One or more errors occurred during local filesystem synchronization, but the database has already received a partial update")]
  LocalSyncNonFatalFail(Vec<FilesystemSyncError>),
}

impl DatabaseConnection<'_> {
  #[instrument]
  /// Synchronize with all sources, adding/removing/changing tracks/albums/artists in the database. When a LocalSyncFail
  /// error is returned, the database has already received a partial update.
  pub fn sync(&self) -> Result<(), SyncError> {
    use SyncError::*;

    // Local source sync
    let local_sync_errors = self.connection.transaction::<_, SyncError, _>(|| {
      let local_sources: Vec<LocalSource> = time!("sync.list_local_sources", self.list_local_sources()?);
      let local_sync_errors = self.local_sync(&local_sources)?;
      Ok(local_sync_errors)
    })?;

    // Spotify source sync
    self.connection.transaction::<_, SyncError, _>(|| {
      let _spotify_sources: Vec<SpotifySource> = time!("sync.list_spotify_sources", self.list_spotify_sources()?);
      Ok(())
    })?;

    if !local_sync_errors.is_empty() {
      return Err(LocalSyncNonFatalFail(local_sync_errors));
    }
    Ok(())
  }
}

