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
  #[error("Local filesystem synchronization failed")]
  LocalSyncFail(#[from] LocalSyncError, Backtrace),
  #[error("One or more errors occurred during local filesystem synchronization, but the database has already received a partial update")]
  LocalSyncNonFatalFail(Vec<FilesystemSyncError>),
  #[error("Spotify synchronization failed")]
  SpotifySyncFail(#[from] SpotifySyncError, Backtrace),
}

impl DatabaseConnection<'_> {
  #[instrument(skip(self))]
  /// Synchronize with all sources, adding/removing/changing tracks/albums/artists in the database. When a LocalSyncFail
  /// error is returned, the database has already received a partial update.
  pub fn sync(&self) -> Result<(), SyncError> {
    use SyncError::*;

    event!(Level::INFO, "Starting synchronization...");

    // Local source sync
    let local_sync_errors = self.connection.transaction::<_, SyncError, _>(|| {
      event!(Level::INFO, "Starting local filesystem synchronization...");
      let local_sources: Vec<LocalSource> = self.list_local_sources()?;
      let local_sync_errors = self.local_sync(local_sources)?;
      Ok(local_sync_errors)
    })?;
    event!(Level::INFO, "... successfully completed local filesystem synchronization");

    // Spotify source sync
    self.connection.transaction::<_, SyncError, _>(|| {
      event!(Level::INFO, "Starting Spotify synchronization...");
      let spotify_sources: Vec<SpotifySource> = self.list_spotify_sources()?;
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

pub enum SelectOrInsert<T> {
  Selected(Vec<T>),
  Inserted(T),
}

pub enum SelectOrInsertOne<T> {
  Selected(T),
  Inserted(T),
}

impl<T> SelectOrInsertOne<T> {
  fn into(self) -> T {
    use SelectOrInsertOne::*;
    match self {
      Selected(t) => t,
      Inserted(t) => t,
    }
  }
}

// Album

#[derive(Debug, Error)]
pub enum SelectAlbumError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple albums with the same name, which is currently not supported: {0:#?}")]
  MultipleAlbumsSameName(Vec<Album>, Backtrace),
}

impl DatabaseConnection<'_> {
  pub(crate) fn select_album_by_id(&self, input_id: i32) -> Result<Album, diesel::result::Error> {
    use schema::album::dsl::*;
    Ok(album.find(input_id).first(&self.connection)?)
  }

  pub(crate) fn select_albums_by_name(&self, input_name: &String) -> Result<Vec<Album>, diesel::result::Error> {
    use schema::album::dsl::*;
    let db_albums: Vec<Album> = album
      .filter(name.eq(input_name))
      .order(id.desc())
      .load(&self.connection)?;
    Ok(db_albums)
  }

  pub(crate) fn select_one_album_by_name(&self, input_name: &String) -> Result<Option<Album>, SelectAlbumError> {
    let db_albums = self.select_albums_by_name(input_name)?;
    match db_albums.len() {
      0 => Ok(None),
      1 => Ok(Some(db_albums.into_iter().next().unwrap())),
      _ => Err(SelectAlbumError::MultipleAlbumsSameName(db_albums, Backtrace::capture())),
    }
  }

  pub(crate) fn insert_album(&self, input_name: &String) -> Result<Album, diesel::result::Error> {
    use schema::album::dsl::*;
    let new_album = NewAlbum { name: input_name.clone() };
    event!(Level::DEBUG, ?new_album, "Inserting album");
    time!("insert_album.insert", diesel::insert_into(album).values(new_album).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_album.select_inserted", album.order(id.desc()).first(&self.connection)?))
  }

  pub(crate) fn select_or_insert_album(&self, input_name: &String) -> Result<SelectOrInsert<Album>, diesel::result::Error> {
    let db_albums = self.select_albums_by_name(input_name)?;
    let result = match db_albums.len() {
      0 => SelectOrInsert::Inserted(self.insert_album(input_name)?),
      _ => SelectOrInsert::Selected(db_albums),
    };
    Ok(result)
  }

  pub(crate) fn select_one_or_insert_album(&self, input_name: &String) -> Result<SelectOrInsertOne<Album>, SelectAlbumError> {
    let result = match self.select_or_insert_album(input_name)? {
      SelectOrInsert::Inserted(album) => SelectOrInsertOne::Inserted(album),
      SelectOrInsert::Selected(db_albums) if db_albums.len() == 1 => SelectOrInsertOne::Inserted(db_albums.into_iter().next().unwrap()),
      SelectOrInsert::Selected(db_albums) => return Err(SelectAlbumError::MultipleAlbumsSameName(db_albums, Backtrace::capture())),
    };
    Ok(result)
  }
}

// Track

#[derive(Debug, Error)]
pub enum SelectTrackError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple tracks with the same album and title, which is currently not supported: {0:#?}")]
  MultipleTracksSameAlbumAndTitle(Vec<Track>, Backtrace),
}

impl DatabaseConnection<'_> {
  pub(crate) fn select_track_by_id(&self, input_id: i32) -> Result<Track, diesel::result::Error> {
    use schema::track::dsl::*;
    Ok(track.find(input_id).first(&self.connection)?)
  }

  pub(crate) fn select_one_track(
    &self,
    input_album_id: i32,
    input_title: &String,
    input_disc_number: Option<i32>,
    input_track_number: Option<i32>,
  ) -> Result<Option<Track>, SelectTrackError> {
    let select_query = {
      use schema::track::dsl::*;
      let mut query = track
        .filter(album_id.eq(input_album_id))
        .filter(title.eq(input_title))
        .into_boxed();
      if let Some(input_disc_number) = input_disc_number {
        query = query.filter(disc_number.eq(input_disc_number))
      }
      if let Some(input_track_number) = input_track_number {
        query = query.filter(track_number.eq(input_track_number))
      }
      query
    };
    let db_tracks: Vec<Track> = select_query.load(&self.connection)?;
    match db_tracks.len() {
      0 => Ok(None),
      1 => Ok(Some(db_tracks.into_iter().next().unwrap())),
      _ => Err(SelectTrackError::MultipleTracksSameAlbumAndTitle(db_tracks, Backtrace::capture())),
    }
  }

  pub(crate) fn insert_track(&self, new_track: NewTrack) -> Result<Track, diesel::result::Error> {
    use schema::track::dsl::*;
    event!(Level::DEBUG, ?new_track, "Inserting track");
    time!("insert_track.insert", diesel::insert_into(track).values(new_track).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_track.select_inserted", track.order(id.desc()).first(&self.connection)?))
  }

  pub(crate) fn select_or_insert_track<N, U>(
    &self,
    album_id: i32,
    title: &String,
    disc_number: Option<i32>,
    track_number: Option<i32>,
    new_track_fn: N,
    update_track_fn: U,
  ) -> Result<SelectOrInsertOne<Track>, SelectTrackError> where
    N: FnOnce(NewTrack) -> NewTrack,
    U: FnOnce(&mut Track) -> bool,
  {
    let result = match self.select_one_track(album_id, title, track_number, disc_number)? {
      Some(mut db_track) => {
        let db_track = if update_track_fn(&mut db_track) {
          event!(Level::DEBUG, ?db_track, "Track has changed, updating the database");
          db_track.save_changes::<Track>(&*self.connection)?
        } else {
          db_track
        };
        SelectOrInsertOne::Selected(db_track)
      }
      None => SelectOrInsertOne::Inserted(self.insert_track(new_track_fn(NewTrack {
        album_id,
        title: title.clone(),
        disc_number,
        track_number,
        ..NewTrack::default()
      }))?),
    };
    Ok(result)
  }
}

// Artist

#[derive(Debug, Error)]
pub enum SelectArtistError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Found multiple artists with the same name, which is currently not supported: {0:#?}")]
  MultipleArtistsSameName(Vec<Artist>, Backtrace),
}

impl DatabaseConnection<'_> {
  pub(crate) fn select_artist_by_id(&self, input_id: i32) -> Result<Artist, diesel::result::Error> {
    use schema::artist::dsl::*;
    Ok(artist.find(input_id).first(&self.connection)?)
  }

  pub(crate) fn select_artists_by_name(&self, input_name: &String) -> Result<Vec<Artist>, diesel::result::Error> {
    use schema::artist::dsl::*;
    let db_artists: Vec<Artist> = artist
      .filter(name.eq(input_name))
      .order(id.desc())
      .load(&self.connection)?;
    Ok(db_artists)
  }

  pub(crate) fn select_one_artist_by_name(&self, input_name: &String) -> Result<Option<Artist>, SelectArtistError> {
    let db_artists = self.select_artists_by_name(input_name)?;
    match db_artists.len() {
      0 => Ok(None),
      1 => Ok(Some(db_artists.into_iter().next().unwrap())),
      _ => Err(SelectArtistError::MultipleArtistsSameName(db_artists, Backtrace::capture())),
    }
  }

  pub(crate) fn insert_artist(&self, input_name: &String) -> Result<Artist, diesel::result::Error> {
    use schema::artist::dsl::*;
    let new_artist = NewArtist { name: input_name.clone() };
    event!(Level::DEBUG, ?new_artist, "Inserting artist");
    time!("insert_artist.insert", diesel::insert_into(artist).values(new_artist).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_artist.select_inserted", artist.order(id.desc()).first(&self.connection)?))
  }

  pub(crate) fn select_or_insert_artist(&self, input_name: &String) -> Result<SelectOrInsert<Artist>, diesel::result::Error> {
    let db_artists = self.select_artists_by_name(input_name)?;
    let result = match db_artists.len() {
      0 => SelectOrInsert::Inserted(self.insert_artist(input_name)?),
      _ => SelectOrInsert::Selected(db_artists),
    };
    Ok(result)
  }

  pub(crate) fn select_one_or_insert_artist(&self, input_name: &String) -> Result<SelectOrInsertOne<Artist>, SelectArtistError> {
    let result = match self.select_or_insert_artist(input_name)? {
      SelectOrInsert::Inserted(artist) => SelectOrInsertOne::Inserted(artist),
      SelectOrInsert::Selected(db_artists) if db_artists.len() == 1 => SelectOrInsertOne::Inserted(db_artists.into_iter().next().unwrap()),
      SelectOrInsert::Selected(db_artists) => return Err(SelectArtistError::MultipleArtistsSameName(db_artists, Backtrace::capture())),
    };
    Ok(result)
  }
}

// Album and track artist associations.

impl DatabaseConnection<'_> {
  fn sync_album_artists(&self, album: &Album, mut artist_ids: HashSet<i32>) -> Result<(), diesel::result::Error> {
    let select_query = {
      use schema::album_artist::dsl::*;
      album_artist
        .filter(album_id.eq(album.id))
        .order(artist_id.desc())
    };
    let db_album_artists: Vec<AlbumArtist> = time!("sync_album_artists.select", select_query.load(&self.connection)?);
    for db_album_artist in db_album_artists {
      // Album-artist already database.
      let artist_id = db_album_artist.artist_id;
      if artist_ids.contains(&artist_id) {
        // Album-artist was found in the set of artists to set as album-artists (artist_ids): keep it by doing nothing.
        // TODO: update album artist columns if they are added.
      } else {
        // Album-artist was not found in the set of artists to set as album-artists (artist_ids): delete it.
        event!(Level::DEBUG, ?db_album_artist, "Deleting album-artist");
        time!("sync_album_artists.delete", diesel::delete(&db_album_artist).execute(&self.connection)?);
      }
      // Remove from artist_ids set, so db_artists contains exactly what we want to insert after this loop.
      artist_ids.remove(&artist_id);
    }
    for artist_id in artist_ids {
      let new_album_artist = NewAlbumArtist { album_id: album.id, artist_id };
      event!(Level::DEBUG, ?new_album_artist, "Inserting album-artist");
      let insert_query = {
        use schema::album_artist::dsl::*;
        diesel::insert_into(album_artist).values(new_album_artist)
      };
      time!("sync_album_artists.insert", insert_query.execute(&self.connection)?);
    }
    Ok(())
  }

  fn sync_track_artists(&self, track: &Track, mut artist_ids: HashSet<i32>) -> Result<(), diesel::result::Error> {
    let select_query = {
      use schema::track_artist::dsl::*;
      track_artist
        .filter(track_id.eq(track.id))
        .order(artist_id.desc())
    };
    let db_track_artists: Vec<TrackArtist> = time!("sync_track_artists.select", select_query.load(&self.connection)?);
    for db_track_artist in db_track_artists {
      // Track-artist already database.
      let artist_id = db_track_artist.artist_id;
      if artist_ids.contains(&artist_id) {
        // Track-artist was found in the set of artists to set as track-artists (artist_ids): keep it by doing nothing.
        // TODO: update track artist columns if they are added.
      } else {
        // Track-artist was not found in the set of artists to set as track-artists (artist_ids): delete it.
        event!(Level::DEBUG, ?db_track_artist, "Deleting track-artist");
        time!("sync_track_artists.delete", diesel::delete(&db_track_artist).execute(&self.connection)?);
      }
      // Remove from artist_ids set, so db_artists contains exactly what we want to insert after this loop.
      artist_ids.remove(&artist_id);
    }
    for artist_id in artist_ids {
      let new_track_artist = NewTrackArtist { track_id: track.id, artist_id };
      event!(Level::DEBUG, ?new_track_artist, "Inserting track-artist");
      let insert_query = {
        use schema::track_artist::dsl::*;
        diesel::insert_into(track_artist).values(new_track_artist)
      };
      time!("sync_track_artists.insert", insert_query.execute(&self.connection)?);
    }
    Ok(())
  }
}
