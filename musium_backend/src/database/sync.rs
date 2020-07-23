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

#[derive(Debug, Error)]
pub enum SelectOrInsertAlbumError {
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

  pub(crate) fn select_album_by_name(&self, input_name: &String) -> Result<Album, diesel::result::Error> {
    use schema::album::dsl::*;
    Ok(album.filter(name.eq(input_name)).first::<Album>(&self.connection)?)
  }

  pub(crate) fn insert_album(&self, input_name: &String) -> Result<Album, diesel::result::Error> {
    use schema::album::dsl::*;
    let new_album = NewAlbum { name: input_name.clone() };
    event!(Level::DEBUG, ?new_album, "Inserting album");
    time!("insert_album.insert_album", diesel::insert_into(album).values(new_album).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_album.select_inserted_album", album.order(id.desc()).first::<Album>(&self.connection)?))
  }

  pub(crate) fn select_or_insert_album(&self, input_name: &String) -> Result<Album, SelectOrInsertAlbumError> {
    use SelectOrInsertAlbumError::*;
    let select_query = {
      use schema::album::dsl::*;
      album.filter(name.eq(input_name))
    };
    let db_albums: Vec<Album> = time!("get_or_create_album.select_album", select_query.load::<Album>(&self.connection)?);
    let db_albums_len = db_albums.len();
    Ok(if db_albums_len == 0 {
      // No album with the same name was found: insert it.
      self.insert_album(input_name)?
    } else if db_albums_len == 1 {
      // One album with the same name was found: return it.
      let db_album = db_albums.into_iter().next().unwrap();
      // TODO: update album columns when they are added.
      db_album
    } else {
      // Multiple albums with the same name were found: for now, we error out.
      return Err(MultipleAlbumsSameName(db_albums));
    })
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
  pub(crate) fn select_track_by_id(&self, input_id: i32) -> Result<Track, diesel::result::Error> {
    use schema::track::dsl::*;
    Ok(track.find(input_id).first::<Track>(&self.connection)?)
  }

  pub(crate) fn select_track_by_album_title(&self, input_album_id: i32, input_title: &String) -> Result<Track, diesel::result::Error> {
    use schema::track::dsl::*;
    Ok(track.filter(album_id.eq(input_album_id)).filter(title.eq(input_title)).first::<Track>(&self.connection)?)
  }

  pub(crate) fn insert_track(&self, new_track: NewTrack) -> Result<Track, diesel::result::Error> {
    use schema::track::dsl::*;
    event!(Level::DEBUG, ?new_track, "Inserting track");
    time!("insert_track.insert_track", diesel::insert_into(track).values(new_track).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_track.select_inserted_track", track.order(id.desc()).first::<Track>(&self.connection)?))
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
    Ok(if db_tracks_len == 0 {
      // No track with the same name was found: insert it.
      let new_track = new_track_fn(input_album_id, input_title.clone());
      self.insert_track(new_track)?
    } else if db_tracks_len == 1 {
      // One track with the same name was found: return it.
      let mut db_track = db_tracks.into_iter().next().unwrap();
      if update_track_fn(&mut db_track) {
        event!(Level::DEBUG, ?db_track, "Track has changed, updating the database");
        db_track.save_changes::<Track>(&*self.connection)?
      } else {
        db_track
      }
    } else {
      // Multiple tracks with the same name were found: for now, we error out.
      return Err(MultipleTracksSameName(db_tracks));
    })
  }
}

#[derive(Debug, Error)]
pub enum SelectOrInsertArtistError {
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

  pub(crate) fn select_artist_by_name(&self, input_name: &String) -> Result<Artist, diesel::result::Error> {
    use schema::artist::dsl::*;
    Ok(artist.filter(name.eq(input_name)).first::<Artist>(&self.connection)?)
  }

  pub(crate) fn insert_artist(&self, input_name: &String) -> Result<Artist, diesel::result::Error> {
    use schema::artist::dsl::*;
    let new_artist = NewArtist { name: input_name.clone() };
    event!(Level::DEBUG, ?new_artist, "Inserting artist");
    time!("insert_artist.insert_artist", diesel::insert_into(artist).values(new_artist).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_artist.select_inserted_artist", artist.order(id.desc()).first::<Artist>(&self.connection)?))
  }

  pub(crate) fn select_or_insert_artist(&self, input_name: &String) -> Result<Artist, SelectOrInsertArtistError> {
    use SelectOrInsertArtistError::*;
    let select_query = {
      use schema::artist::dsl::*;
      artist.filter(name.eq(input_name))
    };
    let db_artists: Vec<Artist> = time!("get_or_create_artist.select_artist", select_query.load::<Artist>(&self.connection)?);
    let db_artists_len = db_artists.len();
    Ok(if db_artists_len == 0 {
      // No artist with the same name was found: insert it.
      self.insert_artist(input_name)?
    } else if db_artists_len == 1 {
      // One artist with the same name was found: return it.
      let db_artist = db_artists.into_iter().next().unwrap();
      // TODO: update artist columns when they are added.
      db_artist
    } else {
      // Multiple artists with the same name were found: for now, we error out.
      return Err(MultipleArtistsSameName(db_artists));
    })
  }
}

impl DatabaseConnection<'_> {
  fn sync_album_artists(&self, album: &Album, mut db_artists: HashSet<Artist>) -> Result<(), diesel::result::Error> {
    let select_query = {
      use schema::album_artist::dsl::*;
      album_artist
        .filter(album_id.eq(album.id))
        .inner_join(schema::artist::table)
    };
    let db_album_artists: Vec<(AlbumArtist, Artist)> = time!("sync_album_artists.select_album_artists", select_query.load(&self.connection)?);
    for (db_album_artist, db_artist) in db_album_artists {
      if db_artists.contains(&db_artist) {
        // TODO: update album artist columns if they are added.
      } else {
        event!(Level::DEBUG, ?db_album_artist, "Deleting album artist");
        time!("sync.delete_album_artist", diesel::delete(&db_album_artist).execute(&self.connection)?);
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
      time!("sync_album_artists.insert_album_artist", insert_query.execute(&self.connection)?);
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
    let db_track_artists: Vec<(TrackArtist, Artist)> = time!("sync_track_artists.select_track_artists", select_query.load(&self.connection)?);
    for (db_track_artist, db_artist) in db_track_artists {
      if db_artists.contains(&db_artist) {
        // TODO: update track artist columns if they are added.
      } else {
        event!(Level::DEBUG, ?db_track_artist, "Deleting track artist");
        time!("sync.delete_track_artist", diesel::delete(&db_track_artist).execute(&self.connection)?);
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
      time!("sync_track_artists.insert_track_artist", insert_query.execute(&self.connection)?);
    }
    Ok(())
  }
}
