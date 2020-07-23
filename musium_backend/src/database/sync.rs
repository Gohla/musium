use std::backtrace::Backtrace;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, instrument, Level};

use musium_core::model::{Album, LocalSource, NewAlbum, NewTrack, SpotifySource, Track};
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
  #[error("Spotify sync failed")]
  SpotifySyncFail(#[from] SpotifySyncError, Backtrace),
}

impl DatabaseConnection<'_> {
  #[instrument(skip(self), err)]
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
      let mut rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(self.spotify_sync(spotify_sources))?;
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
pub enum SelectOrInsertAlbumError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple albums with the same name, which is currently not supported: {0:#?}")]
  MultipleAlbumsSameName(Vec<Album>),
}

impl DatabaseConnection<'_> {
  pub(crate) fn select_or_insert_album(&self, input_name: &String) -> Result<Album, SelectOrInsertAlbumError> {
    use SelectOrInsertAlbumError::*;
    let select_query = {
      use schema::album::dsl::*;
      album.filter(name.eq(input_name))
    };
    let db_albums: Vec<Album> = time!("get_or_create_album.select_album", select_query.load::<Album>(&self.connection)?);
    let db_albums_len = db_albums.len();
    if db_albums_len == 0 {
      // No album with the same name was found: insert it.
      let new_album = NewAlbum { name: input_name.clone() };
      event!(Level::DEBUG, ?new_album, "Inserting album");
      let insert_album_query = {
        use schema::album::dsl::*;
        diesel::insert_into(album).values(new_album)
      };
      time!("get_or_create_album.insert_album", insert_album_query.execute(&self.connection)?);
      let album = time!("get_or_create_album.select_inserted_album", select_query.first::<Album>(&self.connection)?);
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

#[derive(Debug, Error)]
pub enum SelectOrInsertTrackError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple tracks with the same album and name, which is currently not supported: {0:#?}")]
  MultipleTracksSameName(Vec<Track>),
}

impl DatabaseConnection<'_> {
  pub(crate) fn insert_track(&self, new_track: NewTrack) -> Result<Track, diesel::result::Error> {
    event!(Level::DEBUG, ?new_track, "Inserting track");
    let (select_query, insert_query) = {
      use schema::track::dsl::*;
      (
        track.filter(album_id.eq(new_track.album_id)).filter(title.eq(new_track.title.clone())),
        diesel::insert_into(track).values(new_track),
      )
    };
    time!("get_or_create_track.insert_track", insert_query.execute(&self.connection)?);
    let db_track = time!("get_or_create_track.select_inserted_track", select_query.first::<Track>(&self.connection)?);
    Ok(db_track)
  }

  pub(crate) fn select_or_insert_track<N, U>(
    &self,
    input_album_id: i32,
    input_title: &String,
    new_track_fn: N,
    update_track_fn: U,
  ) -> Result<Track, SelectOrInsertTrackError> where
    N: FnOnce(i32, String) -> NewTrack,
    U: FnOnce(&mut Track) -> bool,
  {
    use SelectOrInsertTrackError::*;
    let select_query = {
      use schema::track::dsl::*;
      track
        .filter(title.eq(input_title))
        .filter(album_id.eq(input_album_id))
    };
    let db_tracks: Vec<Track> = time!("get_or_create_track.select_track", select_query.load::<Track>(&self.connection)?);
    let db_tracks_len = db_tracks.len();
    if db_tracks_len == 0 {
      // No track with the same name was found: insert it.
      let new_track = new_track_fn(input_album_id, input_title.clone());
      Ok(self.insert_track(new_track)?)
    } else if db_tracks_len == 1 {
      // One track with the same name was found: return it.
      let mut db_track = db_tracks.into_iter().next().unwrap();
      if update_track_fn(&mut db_track) {
        event!(Level::DEBUG, ?db_track, "Track has changed, updating the database");
        db_track.save_changes::<Track>(&*self.connection)?;
      }
      Ok(db_track)
    } else {
      // Multiple tracks with the same name were found: for now, we error out.
      Err(MultipleTracksSameName(db_tracks))
    }
  }
}

