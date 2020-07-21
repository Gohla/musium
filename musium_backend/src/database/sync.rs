use std::backtrace::Backtrace;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, Level};
use tracing::instrument;

use musium_core::model::{Album, LocalSource, NewAlbum, SpotifySource};
use musium_core::schema;
use musium_filesystem_sync::FilesystemSyncError;

use crate::database::{DatabaseConnection, DatabaseQueryError};
use crate::database::sync::local::LocalSyncError;
use crate::database::sync::spotify::SpotifySyncError;

pub mod local;
pub mod spotify;

// Sync

#[derive(Debug, Error)]
pub enum SyncError {
  #[error("Failed to list sources")]
  ListSourcesFail(#[from] DatabaseQueryError, Backtrace),
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error(transparent)]
  LocalSyncFail(#[from] LocalSyncError),
  #[error("One or more errors occurred during local filesystem synchronization, but the database has already received a partial update")]
  LocalSyncNonFatalFail(Vec<FilesystemSyncError>),
  #[error(transparent)]
  SpotifySyncFail(#[from] SpotifySyncError),
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
      let local_sync_errors = self.local_sync(local_sources)?;
      Ok(local_sync_errors)
    })?;

    // Spotify source sync
    self.connection.transaction::<_, SyncError, _>(|| {
      let spotify_sources: Vec<SpotifySource> = time!("sync.list_spotify_sources", self.list_spotify_sources()?);
      tokio::runtime::Runtime::new().unwrap().block_on(self.spotify_sync(spotify_sources))?;
      Ok(())
    })?;

    if !local_sync_errors.is_empty() {
      return Err(LocalSyncNonFatalFail(local_sync_errors));
    }
    Ok(())
  }
}

// Shared sync API for specific sync implementations

#[derive(Debug, Error)]
pub enum GetOrCreateAlbumError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple albums with the same name, which is currently not supported: {0:#?}")]
  MultipleAlbumsSameName(Vec<Album>),
}

impl DatabaseConnection<'_> {
  pub(crate) fn get_or_create_album_by_name(&self, album_name: &String) -> Result<Album, GetOrCreateAlbumError> {
    use GetOrCreateAlbumError::*;
    let select_query = {
      use schema::album::dsl::*;
      album.filter(name.eq(album_name))
    };
    let db_albums: Vec<Album> = time!("get_or_create_album_by_name.select_album", select_query.load::<Album>(&self.connection)?);
    let db_albums_len = db_albums.len();
    if db_albums_len == 0 {
      // No album with the same name was found: insert it.
      let new_album = NewAlbum { name: album_name.clone() };
      event!(Level::DEBUG, ?new_album, "Inserting album");
      let insert_album_query = {
        use schema::album::dsl::*;
        diesel::insert_into(album).values(new_album)
      };
      time!("get_or_create_album_by_name.insert_album", insert_album_query.execute(&self.connection)?);
      let album = time!("get_or_create_album_by_name.select_inserted_album", select_query.first::<Album>(&self.connection)?);
      Ok(album)
    } else if db_albums_len == 1 {
      // One album with the same name was found: return it.
      let album = db_albums.into_iter().next().unwrap();
      Ok(album)
    } else {
      // Multiple albums with the same name were found: for now, we error out.
      Err(MultipleAlbumsSameName(db_albums))
    }
  }
}
