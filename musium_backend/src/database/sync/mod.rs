use std::backtrace::Backtrace;
use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, instrument, Level};

use musium_core::model::{LocalSource, SpotifySource};

use crate::database::{DatabaseConnection, DatabaseQueryError};
use crate::database::sync::local::LocalSyncDatabaseError;
use crate::sync::local::LocalSyncError;

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
  #[error("One or more errors occurred during local synchronization, but the database has already received a partial update")]
  LocalSyncNonFatalFail(Vec<LocalSyncError>),
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
      let (local_sync_tracks, local_sync_errors) = self.local_sync(&local_sources)?;
      let mut synced_file_paths = HashMap::<i32, HashSet<String>>::new();
      // Insert tracks and related entities.
      for local_sync_track in local_sync_tracks {
        event!(Level::TRACE, ?local_sync_track, "Processing local sync track");
        synced_file_paths.entry(local_sync_track.local_source_id)
          .or_default()
          .insert(local_sync_track.file_path.clone());
        let album = self.sync_local_album(&local_sync_track)?;
        let track = self.sync_local_track(&album, &local_sync_track)?;
        self.sync_local_track_artists(&track, &local_sync_track)?;
        self.sync_local_album_artists(&album, &local_sync_track)?;
      }
      self.sync_local_removed_tracks(synced_file_paths)?;
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

