use std::backtrace::Backtrace;
use std::collections::HashSet;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, instrument, Level};

use musium_core::model::{Album, Artist, NewSpotifyAlbum, NewSpotifyAlbumSource, NewSpotifyArtist, NewSpotifyArtistSource, NewSpotifyTrack, NewSpotifyTrackSource, NewTrack, SpotifyAlbum, SpotifyAlbumSource, SpotifyArtist, SpotifyArtistSource, SpotifySource, SpotifyTrack, SpotifyTrackSource, Track};
use musium_core::schema;

use crate::database::{DatabaseConnection, DatabaseQueryError};
use crate::database::sync::{SelectAlbumError, SelectArtistError, SelectTrackError};
use crate::model::{SpotifySourceEx, UpdateFrom};

#[derive(Debug, Error)]
pub enum SpotifySyncError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Call to Spotify API failed")]
  SpotifyApiFail(#[from] musium_spotify_sync::HttpRequestError, Backtrace),
  #[error(transparent)]
  SelectAlbumFail(#[from] SelectAlbumError),
  #[error(transparent)]
  SelectTrackFail(#[from] SelectTrackError),
  #[error(transparent)]
  SelectArtistFail(#[from] SelectArtistError),
}

impl From<DatabaseQueryError> for SpotifySyncError {
  fn from(e: DatabaseQueryError) -> Self {
    match e {
      DatabaseQueryError::DatabaseQueryFail(e, bt) => Self::DatabaseQueryFail(e, bt)
    }
  }
}

impl DatabaseConnection<'_> {
  #[instrument(skip(self, spotify_sources))]
  pub(crate) async fn spotify_sync(&self, spotify_sources: Vec<SpotifySource>) -> Result<(), SpotifySyncError> {
    for mut spotify_source in spotify_sources {
      let mut authorization = spotify_source.to_spotify_authorization();

      let spotify_albums = self.database.spotify_sync.get_albums_of_followed_artists(&mut authorization).await?;
      let mut synced_album_ids = HashSet::<i32>::new();
      let mut synced_track_ids = HashSet::<i32>::new();
      let mut synced_artist_ids = HashSet::<i32>::new();
      for spotify_album in spotify_albums {
        let db_album = self.sync_spotify_album(&spotify_album, spotify_source.id)?;
        synced_album_ids.insert(db_album.id);
        let db_album_artists: Result<HashSet<_>, _> = spotify_album.artists.iter()
          .map(|spotify_artist| self.sync_spotify_artist(spotify_artist, spotify_source.id))
          .collect();
        let db_album_artists = db_album_artists?;
        synced_artist_ids.extend(db_album_artists.iter().map(|a| a.id));
        self.sync_album_artists(&db_album, db_album_artists)?;

        for spotify_track in &spotify_album.tracks.items {
          let db_track = self.sync_spotify_track(spotify_track, &db_album, spotify_source.id)?;
          synced_track_ids.insert(db_track.id);
          let db_track_artists: Result<HashSet<_>, _> = spotify_track.artists.iter()
            .map(|spotify_artist| self.sync_spotify_artist(spotify_artist, spotify_source.id))
            .collect();
          let db_track_artists = db_track_artists?;
          synced_artist_ids.extend(db_track_artists.iter().map(|a| a.id));
          self.sync_track_artists(&db_track, db_track_artists)?;
        }
      }
      self.cleanup_spotify_album_sources(synced_album_ids, spotify_source.id)?;
      self.cleanup_spotify_track_sources(synced_track_ids, spotify_source.id)?;
      self.cleanup_spotify_artist_sources(synced_artist_ids, spotify_source.id)?;

      if spotify_source.update_from_spotify_authorization(authorization) {
        event!(Level::DEBUG, ?spotify_source, "Spotify source has changed, updating the database");
        spotify_source.save_changes::<SpotifySource>(&*self.connection)?;
      }
    }
    Ok(())
  }

  fn sync_spotify_album(&self, spotify_album: &musium_spotify_sync::Album, spotify_source_id: i32) -> Result<Album, SpotifySyncError> {
    event!(Level::TRACE, ?spotify_album, "Synchronizing Spotify album");
    let db_album = match self.select_spotify_album(&spotify_album.id)? {
      Some(db_spotify_album) => {
        self.ensure_spotify_album_source_exists(db_spotify_album.album_id, spotify_source_id)?;
        self.select_album_by_id(db_spotify_album.album_id)?
      }
      None => {
        let db_album = self.select_or_insert_album(&spotify_album.name)?;
        self.insert_spotify_album(db_album.id, &spotify_album.id)?;
        self.ensure_spotify_album_source_exists(db_album.id, spotify_source_id)?;
        db_album
      }
    };
    Ok(db_album)
  }

  fn sync_spotify_track(&self, spotify_track: &musium_spotify_sync::TrackSimple, album: &Album, spotify_source_id: i32) -> Result<Track, SpotifySyncError> {
    event!(Level::TRACE, ?spotify_track, "Synchronizing Spotify track");
    let db_track = match self.select_spotify_track(&spotify_track.id)? {
      Some(db_spotify_track) => {
        self.ensure_spotify_track_source_exists(db_spotify_track.track_id, spotify_source_id)?;
        let mut db_track = self.select_track_by_id(db_spotify_track.track_id)?;
        if db_track.update_from(album, spotify_track) {
          db_track.save_changes::<Track>(&*self.connection)?
        } else {
          db_track
        }
      }
      None => {
        let db_track = self.select_or_insert_track(
          album.id,
          &spotify_track.name,
          |album_id, title| {
            NewTrack {
              album_id,
              disc_number: Some(spotify_track.disc_number),
              disc_total: None,
              track_number: Some(spotify_track.track_number),
              track_total: None,
              title,
            }
          },
          |track| track.update_from(album, spotify_track),
        )?;
        self.insert_spotify_track(db_track.id, &spotify_track.id)?;
        self.ensure_spotify_track_source_exists(db_track.id, spotify_source_id)?;
        db_track
      }
    };
    Ok(db_track)
  }

  fn sync_spotify_artist(&self, spotify_artist: &musium_spotify_sync::ArtistSimple, spotify_source_id: i32) -> Result<Artist, SpotifySyncError> {
    event!(Level::TRACE, ?spotify_artist, "Synchronizing Spotify artist");
    let db_artist = match self.select_spotify_artist(&spotify_artist.id)? {
      Some(db_spotify_artist) => {
        self.ensure_spotify_artist_source_exists(db_spotify_artist.artist_id, spotify_source_id)?;
        self.select_artist_by_id(db_spotify_artist.artist_id)?
      }
      None => {
        let db_artist = self.select_or_insert_artist(&spotify_artist.name)?;
        self.insert_spotify_artist(db_artist.id, &spotify_artist.id)?;
        self.ensure_spotify_artist_source_exists(db_artist.id, spotify_source_id)?;
        db_artist
      }
    };
    Ok(db_artist)
  }

  fn cleanup_spotify_album_sources(&self, synced_album_ids: HashSet::<i32>, input_spotify_source_id: i32) -> Result<(), SpotifySyncError> {
    let db_spotify_album_data: Vec<i32> = {
      use schema::spotify_album_source::dsl::*;
      spotify_album_source
        .select(album_id)
        .filter(spotify_source_id.eq(input_spotify_source_id))
        .load::<i32>(&self.connection)?
    };
    for db_album_id in db_spotify_album_data {
      if !synced_album_ids.contains(&db_album_id) {
        event!(Level::DEBUG, "Spotify album with ID '{}' from Spotify source with ID '{}' was not seen during synchronization: removing it from the database", db_album_id, input_spotify_source_id);
        let delete_query = {
          use schema::spotify_album_source::dsl::*;
          diesel::delete(spotify_album_source)
            .filter(album_id.eq(db_album_id))
            .filter(spotify_source_id.eq(input_spotify_source_id))
        };
        time!("cleanup_spotify_album_sources.delete", delete_query.execute(&self.connection)?);
      }
    }
    Ok(())
  }

  fn cleanup_spotify_track_sources(&self, synced_track_ids: HashSet::<i32>, input_spotify_source_id: i32) -> Result<(), SpotifySyncError> {
    let db_spotify_track_data: Vec<i32> = {
      use schema::spotify_track_source::dsl::*;
      spotify_track_source
        .select(track_id)
        .filter(spotify_source_id.eq(input_spotify_source_id))
        .load::<i32>(&self.connection)?
    };
    for db_track_id in db_spotify_track_data {
      if !synced_track_ids.contains(&db_track_id) {
        event!(Level::DEBUG, "Spotify track with ID '{}' from Spotify source with ID '{}' was not seen during synchronization: removing it from the database", db_track_id, input_spotify_source_id);
        let delete_query = {
          use schema::spotify_track_source::dsl::*;
          diesel::delete(spotify_track_source)
            .filter(track_id.eq(db_track_id))
            .filter(spotify_source_id.eq(input_spotify_source_id))
        };
        time!("cleanup_spotify_track_sources.delete", delete_query.execute(&self.connection)?);
      }
    }
    Ok(())
  }

  fn cleanup_spotify_artist_sources(&self, synced_artist_ids: HashSet::<i32>, input_spotify_source_id: i32) -> Result<(), SpotifySyncError> {
    let db_spotify_artist_data: Vec<i32> = {
      use schema::spotify_artist_source::dsl::*;
      spotify_artist_source
        .select(artist_id)
        .filter(spotify_source_id.eq(input_spotify_source_id))
        .load::<i32>(&self.connection)?
    };
    for db_artist_id in db_spotify_artist_data {
      if !synced_artist_ids.contains(&db_artist_id) {
        event!(Level::DEBUG, "Spotify artist with ID '{}' from Spotify source with ID '{}' was not seen during synchronization: removing it from the database", db_artist_id, input_spotify_source_id);
        let delete_query = {
          use schema::spotify_artist_source::dsl::*;
          diesel::delete(spotify_artist_source)
            .filter(artist_id.eq(db_artist_id))
            .filter(spotify_source_id.eq(input_spotify_source_id))
        };
        time!("cleanup_spotify_artist_sources.delete", delete_query.execute(&self.connection)?);
      }
    }
    Ok(())
  }
}

// Helpers for selecting/inserting.

// Spotify Album (source)

impl DatabaseConnection<'_> {
  fn select_spotify_album(&self, input_spotify_id: &String) -> Result<Option<SpotifyAlbum>, diesel::result::Error> {
    use schema::spotify_album::dsl::*;
    Ok(spotify_album.filter(spotify_id.eq(input_spotify_id)).first::<SpotifyAlbum>(&self.connection).optional()?)
  }

  fn insert_spotify_album(&self, input_album_id: i32, input_spotify_id: &String) -> Result<SpotifyAlbum, diesel::result::Error> {
    use schema::spotify_album::dsl::*;
    let new_spotify_album = NewSpotifyAlbum { album_id: input_album_id, spotify_id: input_spotify_id.clone() };
    event!(Level::DEBUG, ?new_spotify_album, "Inserting Spotify album");
    time!("insert_spotify_album.insert", diesel::insert_into(spotify_album).values(new_spotify_album).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_spotify_album.select_inserted", spotify_album.filter(album_id.eq(input_album_id)).filter(spotify_id.eq(input_spotify_id)).first::<SpotifyAlbum>(&self.connection)?))
  }


  fn select_spotify_album_source(&self, input_album_id: i32, input_spotify_source_id: i32) -> Result<Option<SpotifyAlbumSource>, diesel::result::Error> {
    use schema::spotify_album_source::dsl::*;
    Ok(spotify_album_source
      .filter(album_id.eq(input_album_id))
      .filter(spotify_source_id.eq(input_spotify_source_id))
      .first::<SpotifyAlbumSource>(&self.connection)
      .optional()?)
  }

  fn insert_spotify_album_source(&self, input_album_id: i32, input_spotify_source_id: i32) -> Result<SpotifyAlbumSource, diesel::result::Error> {
    use schema::spotify_album_source::dsl::*;
    let new_spotify_album_source = NewSpotifyAlbumSource { album_id: input_album_id, spotify_source_id: input_spotify_source_id };
    event!(Level::DEBUG, ?new_spotify_album_source, "Inserting Spotify album source");
    time!("insert_spotify_album_source.insert", diesel::insert_into(spotify_album_source).values(new_spotify_album_source).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_spotify_album_source.select_inserted", spotify_album_source.filter(album_id.eq(input_album_id)).filter(spotify_source_id.eq(input_spotify_source_id)).first::<SpotifyAlbumSource>(&self.connection)?))
  }

  fn ensure_spotify_album_source_exists(&self, input_album_id: i32, input_spotify_source_id: i32) -> Result<SpotifyAlbumSource, diesel::result::Error> {
    let db_spotify_album_source = match self.select_spotify_album_source(input_album_id, input_spotify_source_id)? {
      Some(db_spotify_album_source) => db_spotify_album_source,
      None => self.insert_spotify_album_source(input_album_id, input_spotify_source_id)?,
    };
    Ok(db_spotify_album_source)
  }
}

// Spotify Track (source)

impl DatabaseConnection<'_> {
  fn select_spotify_track(&self, input_spotify_id: &String) -> Result<Option<SpotifyTrack>, diesel::result::Error> {
    use schema::spotify_track::dsl::*;
    Ok(spotify_track.filter(spotify_id.eq(input_spotify_id)).first::<SpotifyTrack>(&self.connection).optional()?)
  }

  fn insert_spotify_track(&self, input_track_id: i32, input_spotify_id: &String) -> Result<SpotifyTrack, diesel::result::Error> {
    use schema::spotify_track::dsl::*;
    let new_spotify_track = NewSpotifyTrack { track_id: input_track_id, spotify_id: input_spotify_id.clone() };
    event!(Level::DEBUG, ?new_spotify_track, "Inserting Spotify track");
    time!("insert_spotify_track.insert", diesel::insert_into(spotify_track).values(new_spotify_track).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_spotify_track.select_inserted", spotify_track.filter(track_id.eq(input_track_id)).filter(spotify_id.eq(input_spotify_id)).first::<SpotifyTrack>(&self.connection)?))
  }


  fn select_spotify_track_source(&self, input_track_id: i32, input_spotify_source_id: i32) -> Result<Option<SpotifyTrackSource>, diesel::result::Error> {
    use schema::spotify_track_source::dsl::*;
    Ok(spotify_track_source
      .filter(track_id.eq(input_track_id))
      .filter(spotify_source_id.eq(input_spotify_source_id))
      .first::<SpotifyTrackSource>(&self.connection)
      .optional()?)
  }

  fn insert_spotify_track_source(&self, input_track_id: i32, input_spotify_source_id: i32) -> Result<SpotifyTrackSource, diesel::result::Error> {
    use schema::spotify_track_source::dsl::*;
    let new_spotify_track_source = NewSpotifyTrackSource { track_id: input_track_id, spotify_source_id: input_spotify_source_id };
    event!(Level::DEBUG, ?new_spotify_track_source, "Inserting Spotify track source");
    time!("insert_spotify_track_source.insert", diesel::insert_into(spotify_track_source).values(new_spotify_track_source).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_spotify_track_source.select_inserted", spotify_track_source.filter(track_id.eq(input_track_id)).filter(spotify_source_id.eq(input_spotify_source_id)).first::<SpotifyTrackSource>(&self.connection)?))
  }

  fn ensure_spotify_track_source_exists(&self, input_track_id: i32, input_spotify_source_id: i32) -> Result<SpotifyTrackSource, diesel::result::Error> {
    let db_spotify_track_source = match self.select_spotify_track_source(input_track_id, input_spotify_source_id)? {
      Some(db_spotify_track_source) => db_spotify_track_source,
      None => self.insert_spotify_track_source(input_track_id, input_spotify_source_id)?,
    };
    Ok(db_spotify_track_source)
  }
}

// Spotify Artist (source)

impl DatabaseConnection<'_> {
  fn select_spotify_artist(&self, input_spotify_id: &String) -> Result<Option<SpotifyArtist>, diesel::result::Error> {
    use schema::spotify_artist::dsl::*;
    Ok(spotify_artist.filter(spotify_id.eq(input_spotify_id)).first::<SpotifyArtist>(&self.connection).optional()?)
  }

  fn insert_spotify_artist(&self, input_artist_id: i32, input_spotify_id: &String) -> Result<SpotifyArtist, diesel::result::Error> {
    use schema::spotify_artist::dsl::*;
    let new_spotify_artist = NewSpotifyArtist { artist_id: input_artist_id, spotify_id: input_spotify_id.clone() };
    event!(Level::DEBUG, ?new_spotify_artist, "Inserting Spotify artist");
    time!("insert_spotify_artist.insert", diesel::insert_into(spotify_artist).values(new_spotify_artist).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_spotify_artist.select_inserted", spotify_artist.filter(artist_id.eq(input_artist_id)).filter(spotify_id.eq(input_spotify_id)).first::<SpotifyArtist>(&self.connection)?))
  }


  fn select_spotify_artist_source(&self, input_artist_id: i32, input_spotify_source_id: i32) -> Result<Option<SpotifyArtistSource>, diesel::result::Error> {
    use schema::spotify_artist_source::dsl::*;
    Ok(spotify_artist_source
      .filter(artist_id.eq(input_artist_id))
      .filter(spotify_source_id.eq(input_spotify_source_id))
      .first::<SpotifyArtistSource>(&self.connection)
      .optional()?)
  }

  fn insert_spotify_artist_source(&self, input_artist_id: i32, input_spotify_source_id: i32) -> Result<SpotifyArtistSource, diesel::result::Error> {
    use schema::spotify_artist_source::dsl::*;
    let new_spotify_artist_source = NewSpotifyArtistSource { artist_id: input_artist_id, spotify_source_id: input_spotify_source_id };
    event!(Level::DEBUG, ?new_spotify_artist_source, "Inserting Spotify artist source");
    time!("insert_spotify_artist_source.insert", diesel::insert_into(spotify_artist_source).values(new_spotify_artist_source).execute(&self.connection)?);
    // NOTE: must be executed in a transaction for consistency
    Ok(time!("insert_spotify_artist_source.select_inserted", spotify_artist_source.filter(artist_id.eq(input_artist_id)).filter(spotify_source_id.eq(input_spotify_source_id)).first::<SpotifyArtistSource>(&self.connection)?))
  }

  fn ensure_spotify_artist_source_exists(&self, input_artist_id: i32, input_spotify_source_id: i32) -> Result<SpotifyArtistSource, diesel::result::Error> {
    let db_spotify_artist_source = match self.select_spotify_artist_source(input_artist_id, input_spotify_source_id)? {
      Some(db_spotify_artist_source) => db_spotify_artist_source,
      None => self.insert_spotify_artist_source(input_artist_id, input_spotify_source_id)?,
    };
    Ok(db_spotify_artist_source)
  }
}
