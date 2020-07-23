use std::backtrace::Backtrace;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, instrument, Level};

use musium_core::model::{Album, NewSpotifyAlbum, NewSpotifyTrack, NewTrack, SpotifyAlbum, SpotifySource, SpotifyTrack, Track};
use musium_core::schema;

use crate::database::{DatabaseConnection, DatabaseQueryError};
use crate::database::sync::{SelectOrInsertAlbumError, SelectOrInsertTrackError};
use crate::model::{SpotifySourceEx, UpdateFrom};

#[derive(Debug, Error)]
pub enum SpotifySyncError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Call to Spotify API failed")]
  SpotifyApiFail(#[from] musium_spotify_sync::ApiError, Backtrace),
  #[error(transparent)]
  GetOrCreateAlbumFail(#[from] SelectOrInsertAlbumError),
  #[error(transparent)]
  GetOrCreateTrackFail(#[from] SelectOrInsertTrackError),
}

impl From<DatabaseQueryError> for SpotifySyncError {
  fn from(e: DatabaseQueryError) -> Self {
    match e {
      DatabaseQueryError::DatabaseQueryFail(e, bt) => Self::DatabaseQueryFail(e, bt)
    }
  }
}

impl DatabaseConnection<'_> {
  #[instrument(skip(self, spotify_sources), err)]
  pub(crate) async fn spotify_sync(&self, spotify_sources: Vec<SpotifySource>) -> Result<(), SpotifySyncError> {
    for mut spotify_source in spotify_sources {
      let mut authorization = spotify_source.to_spotify_authorization();
      let albums = self.database.spotify_sync.get_albums_of_followed_artists(&mut authorization).await?;
      for album in albums {
        self.sync_spotify_album(&album)?;
      }
      if spotify_source.update_from_spotify_authorization(authorization) {
        event!(Level::DEBUG, ?spotify_source, "Spotify source has changed, updating the database");
        spotify_source.save_changes::<SpotifySource>(&*self.connection)?;
      }
    }
    Ok(())
  }

  fn sync_spotify_album(&self, spotify_album: &musium_spotify_sync::Album) -> Result<Album, SpotifySyncError> {
    event!(Level::TRACE, ?spotify_album, "Synchronizing Spotify album");
    let spotify_album_id = &spotify_album.id;
    let select_query = {
      use schema::spotify_album::dsl::*;
      spotify_album.filter(spotify_id.eq(spotify_album_id))
    };
    let db_spotify_album: Option<SpotifyAlbum> = time!("sync_spotify_album.select_spotify_album", select_query.first(&self.connection).optional()?);
    let db_album = if let Some(db_spotify_album) = db_spotify_album {
      // A Spotify album was found for the Spotify album ID: update it.
      // TODO: update Spotify album columns when they are added.
      // TODO: select album with a join on the previous query?
      let db_album = self.get_album_by_id(db_spotify_album.album_id)?.unwrap();
      // TODO: update album columns when they are added.
      db_album
    } else {
      // No Spotify album was found for the Spotify album ID: get or create an album, then create a Spotify album.
      let db_album = self.select_or_insert_album(&spotify_album.name)?;
      let new_spotify_album = NewSpotifyAlbum { album_id: db_album.id, spotify_id: spotify_album_id.clone() };
      event!(Level::DEBUG, ?new_spotify_album, "Inserting Spotify album");
      let insert_spotify_album_query = {
        use schema::spotify_album::dsl::*;
        diesel::insert_into(spotify_album).values(new_spotify_album)
      };
      time!("sync_spotify_album.insert_spotify_album", insert_spotify_album_query.execute(&self.connection)?);
      db_album
    };

    for spotify_track in &spotify_album.tracks.items {
      self.sync_spotify_track(spotify_track, &db_album)?;
    }

    Ok(db_album)
  }

  fn sync_spotify_track(&self, spotify_track: &musium_spotify_sync::TrackSimple, album: &Album) -> Result<Track, SpotifySyncError> {
    event!(Level::TRACE, ?spotify_track, "Synchronizing Spotify track");
    let spotify_track_id = &spotify_track.id;
    let select_query = {
      use schema::spotify_track::dsl::*;
      spotify_track.filter(spotify_id.eq(spotify_track_id))
    };
    let db_spotify_track: Option<SpotifyTrack> = time!("sync_spotify_track.select_spotify_track", select_query.first(&self.connection).optional()?);
    if let Some(db_spotify_track) = db_spotify_track {
      // A Spotify track was found for the Spotify track ID: update it.
      // TODO: update Spotify album columns when they are added.
      // TODO: select track with a join on the previous query?
      let mut db_track = self.get_track_by_id(db_spotify_track.track_id)?.unwrap();
      if db_track.update_from(album, spotify_track) {
        db_track.save_changes::<Track>(&*self.connection)?;
      }
      Ok(db_track)
    } else {
      // No Spotify track was found for the Spotify track ID: get or create an track, then create a Spotify track.
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
      let new_spotify_track = NewSpotifyTrack { track_id: db_track.id, spotify_id: spotify_track_id.clone() };
      event!(Level::DEBUG, ?new_spotify_track, "Inserting Spotify track");
      let insert_spotify_track_query = {
        use schema::spotify_track::dsl::*;
        diesel::insert_into(spotify_track).values(new_spotify_track)
      };
      time!("sync_spotify_track.insert_spotify_track", insert_spotify_track_query.execute(&self.connection)?);
      Ok(db_track)
    }
  }
}
