#![allow(dead_code)]

use std::backtrace::Backtrace;
use std::collections::HashSet;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, instrument, Level};

use musium_core::model::{Album, AlbumArtist, Artist, LocalSource, NewAlbum, NewAlbumArtist, NewArtist, NewTrack, NewTrackArtist, SpotifySource, Track, TrackArtist};
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

    event!(Level::INFO, "Starting synchronization...");

    // Local source sync
    let local_sync_errors = self.connection.transaction::<_, SyncError, _>(|| {
      event!(Level::INFO, "Starting local filesystem synchronization...");
      let local_sources: Vec<LocalSource> = time!("sync.list_local_sources", self.list_local_sources()?);
      let local_sync_errors = self.local_sync(local_sources)?;
      Ok(local_sync_errors)
    })?;
    event!(Level::INFO, "... successfully completed local filesystem synchronization");

    // Spotify source sync
    self.connection.transaction::<_, SyncError, _>(|| {
      event!(Level::INFO, "Starting Spotify synchronization...");
      let spotify_sources: Vec<SpotifySource> = time!("sync.list_spotify_sources", self.list_spotify_sources()?);
      let mut rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(self.spotify_sync(spotify_sources))?;
      Ok(())
    })?;
    event!(Level::INFO, "... successfully completed Spotify synchronization");

    event!(Level::INFO, "... successfully completed synchronization");

    if !local_sync_errors.is_empty() {
      return Err(LocalSyncNonFatalFail(local_sync_errors));
    }
    Ok(())
  }
}

// Shared sync API for specific sync implementations

// Album

#[derive(Debug, Error)]
pub enum SelectAlbumError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple albums with the same name, which is currently not supported: {0:#?}")]
  MultipleAlbumsSameName(Vec<Album>),
}

impl DatabaseConnection<'_> {
  pub(crate) fn select_album_by_id(&self, input_id: i32) -> Result<Album, diesel::result::Error> {
    use schema::album::dsl::*;
    Ok(album.find(input_id).first::<Album>(&self.connection)?)
  }

  pub(crate) fn select_one_album_by_name(&self, input_name: &String) -> Result<Option<Album>, SelectAlbumError> {
    use schema::album::dsl::*;
    let db_albums: Vec<Album> = album.filter(name.eq(input_name)).load::<Album>(&self.connection)?;
    match db_albums.len() {
      0 => Ok(None),
      1 => Ok(Some(db_albums.into_iter().next().unwrap())),
      _ => Err(SelectAlbumError::MultipleAlbumsSameName(db_albums)),
    }
  }

  pub(crate) fn insert_album(&self, input_name: &String) -> Result<Album, diesel::result::Error> {
    use schema::album::dsl::*;
    let new_album = NewAlbum { name: input_name.clone() };
    event!(Level::DEBUG, ?new_album, "Inserting album");
    time!("insert_album.insert", diesel::insert_into(album).values(new_album).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_album.select_inserted", album.order(id.desc()).first::<Album>(&self.connection)?))
  }

  pub(crate) fn select_or_insert_album(&self, input_name: &String) -> Result<Album, SelectAlbumError> {
    let db_album = match self.select_one_album_by_name(input_name)? {
      Some(db_album) => db_album,
      None => self.insert_album(input_name)?,
    };
    Ok(db_album)
  }
}

// Track

#[derive(Debug, Error)]
pub enum SelectTrackError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple tracks with the same album and title, which is currently not supported: {0:#?}")]
  MultipleTracksSameAlbumAndTitle(Vec<Track>),
}

impl DatabaseConnection<'_> {
  pub(crate) fn select_track_by_id(&self, input_id: i32) -> Result<Track, diesel::result::Error> {
    use schema::track::dsl::*;
    Ok(track.find(input_id).first::<Track>(&self.connection)?)
  }

  pub(crate) fn select_one_track_by_album_and_title(&self, input_album_id: i32, input_title: &String) -> Result<Option<Track>, SelectTrackError> {
    use schema::track::dsl::*;
    let db_tracks: Vec<Track> = track.filter(album_id.eq(input_album_id)).filter(title.eq(input_title)).load::<Track>(&self.connection)?;
    match db_tracks.len() {
      0 => Ok(None),
      1 => Ok(Some(db_tracks.into_iter().next().unwrap())),
      _ => Err(SelectTrackError::MultipleTracksSameAlbumAndTitle(db_tracks)),
    }
  }

  pub(crate) fn insert_track(&self, new_track: NewTrack) -> Result<Track, diesel::result::Error> {
    use schema::track::dsl::*;
    event!(Level::DEBUG, ?new_track, "Inserting track");
    time!("insert_track.insert", diesel::insert_into(track).values(new_track).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_track.select_inserted", track.order(id.desc()).first::<Track>(&self.connection)?))
  }

  pub(crate) fn select_or_insert_track<N, U>(
    &self,
    input_album_id: i32,
    input_title: &String,
    new_track_fn: N,
    update_track_fn: U,
  ) -> Result<Track, SelectTrackError> where
    N: FnOnce(i32, String) -> NewTrack,
    U: FnOnce(&mut Track) -> bool,
  {
    let db_track = match self.select_one_track_by_album_and_title(input_album_id, input_title)? {
      Some(mut db_track) => {
        if update_track_fn(&mut db_track) {
          event!(Level::DEBUG, ?db_track, "Track has changed, updating the database");
          db_track.save_changes::<Track>(&*self.connection)?
        } else {
          db_track
        }
      }
      None => self.insert_track(new_track_fn(input_album_id, input_title.clone()))?,
    };
    Ok(db_track)
  }
}

// Artist

#[derive(Debug, Error)]
pub enum SelectArtistError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple artists with the same name, which is currently not supported: {0:#?}")]
  MultipleArtistsSameName(Vec<Artist>),
}

impl DatabaseConnection<'_> {
  pub(crate) fn select_artist_by_id(&self, input_id: i32) -> Result<Artist, diesel::result::Error> {
    use schema::artist::dsl::*;
    Ok(artist.find(input_id).first::<Artist>(&self.connection)?)
  }

  pub(crate) fn select_one_artist_by_name(&self, input_name: &String) -> Result<Option<Artist>, SelectArtistError> {
    use schema::artist::dsl::*;
    let db_artists: Vec<Artist> = artist.filter(name.eq(input_name)).load::<Artist>(&self.connection)?;
    match db_artists.len() {
      0 => Ok(None),
      1 => Ok(Some(db_artists.into_iter().next().unwrap())),
      _ => Err(SelectArtistError::MultipleArtistsSameName(db_artists)),
    }
  }

  pub(crate) fn insert_artist(&self, input_name: &String) -> Result<Artist, diesel::result::Error> {
    use schema::artist::dsl::*;
    let new_artist = NewArtist { name: input_name.clone() };
    event!(Level::DEBUG, ?new_artist, "Inserting artist");
    time!("insert_artist.insert", diesel::insert_into(artist).values(new_artist).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_artist.select_inserted", artist.order(id.desc()).first::<Artist>(&self.connection)?))
  }

  pub(crate) fn select_or_insert_artist(&self, input_name: &String) -> Result<Artist, SelectArtistError> {
    let db_artist = match self.select_one_artist_by_name(input_name)? {
      Some(db_artist) => db_artist,
      None => self.insert_artist(input_name)?,
    };
    Ok(db_artist)
  }
}

// Album and track artist associations.

impl DatabaseConnection<'_> {
  fn sync_album_artists(&self, album: &Album, mut db_artists: HashSet<Artist>) -> Result<(), diesel::result::Error> {
    let select_query = {
      use schema::album_artist::dsl::*;
      album_artist
        .filter(album_id.eq(album.id))
        .inner_join(schema::artist::table)
    };
    let db_album_artists: Vec<(AlbumArtist, Artist)> = time!("sync_album_artists.select", select_query.load(&self.connection)?);
    for (db_album_artist, db_artist) in db_album_artists {
      if db_artists.contains(&db_artist) {
        // TODO: update album artist columns if they are added.
      } else {
        event!(Level::DEBUG, ?db_album_artist, "Deleting album artist");
        time!("sync_album_artists.delete", diesel::delete(&db_album_artist).execute(&self.connection)?);
      }
      db_artists.remove(&db_artist); // Remove from set, so we know what to insert afterwards.
    }
    for artist in db_artists {
      let new_album_artist = NewAlbumArtist { album_id: album.id, artist_id: artist.id };
      event!(Level::DEBUG, ?new_album_artist, "Inserting album artist");
      let insert_query = {
        use schema::album_artist::dsl::*;
        diesel::insert_into(album_artist).values(new_album_artist)
      };
      time!("sync_album_artists.insert", insert_query.execute(&self.connection)?);
    }
    Ok(())
  }

  fn sync_track_artists(&self, track: &Track, mut db_artists: HashSet<Artist>) -> Result<(), diesel::result::Error> {
    let select_query = {
      use schema::track_artist::dsl::*;
      track_artist
        .filter(track_id.eq(track.id))
        .inner_join(schema::artist::table)
    };
    let db_track_artists: Vec<(TrackArtist, Artist)> = time!("sync_track_artists.select", select_query.load(&self.connection)?);
    for (db_track_artist, db_artist) in db_track_artists {
      if db_artists.contains(&db_artist) {
        // TODO: update track artist columns if they are added.
      } else {
        event!(Level::DEBUG, ?db_track_artist, "Deleting track artist");
        time!("sync_track_artists.delete", diesel::delete(&db_track_artist).execute(&self.connection)?);
      }
      db_artists.remove(&db_artist); // Remove from set, so we know what to insert afterwards.
    }
    for artist in db_artists {
      let new_track_artist = NewTrackArtist { track_id: track.id, artist_id: artist.id };
      event!(Level::DEBUG, ?new_track_artist, "Inserting track artist");
      let insert_query = {
        use schema::track_artist::dsl::*;
        diesel::insert_into(track_artist).values(new_track_artist)
      };
      time!("sync_track_artists.insert", insert_query.execute(&self.connection)?);
    }
    Ok(())
  }
}
