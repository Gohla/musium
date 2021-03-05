use std::backtrace::Backtrace;
use std::path::PathBuf;

use diesel::prelude::*;
use thiserror::Error;

use musium_core::model::{Album, AlbumArtist, Artist, Track, TrackArtist};
use musium_core::model::collection::TracksRaw;
use musium_core::schema;

use crate::database::spotify_track::SpotifyPlayError;

use super::{DatabaseConnection, DatabaseQueryError};

impl DatabaseConnection {
  pub fn list_tracks(&self) -> Result<TracksRaw, DatabaseQueryError> {
    let tracks = schema::track::table.load::<Track>(&self.connection)?;
    let albums = schema::album::table.load::<Album>(&self.connection)?;
    let artists = schema::artist::table.load::<Artist>(&self.connection)?;
    let track_artists = schema::track_artist::table.load::<TrackArtist>(&self.connection)?;
    let album_artists = schema::album_artist::table.load::<AlbumArtist>(&self.connection)?;
    Ok(TracksRaw { albums, tracks, artists, album_artists, track_artists })
  }

  pub fn get_track_by_id(&self, input_id: i32) -> Result<Option<Track>, DatabaseQueryError> {
    use schema::track::dsl::*;
    Ok(track.find(input_id).first::<Track>(&self.connection).optional()?)
  }
}

pub enum PlaySource {
  AudioData(PathBuf),
  ExternallyPlayed,
}

#[derive(Debug, Error)]
pub enum PlayError {
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] DatabaseQueryError, Backtrace),
  #[error("Failed to play Spotify track")]
  SpotifyPlayFail(#[from] SpotifyPlayError, Backtrace),
}

impl DatabaseConnection {
  pub async fn play_track(&self, input_id: i32, user_id: i32) -> Result<Option<PlaySource>, PlayError> {
    let source = if let Some(path) = self.get_local_track_path_by_track_id(input_id)? { // TODO: fix blocking code in async
      Some(PlaySource::AudioData(path))
    } else if let true = self.play_spotify_track(input_id, user_id).await? {
      Some(PlaySource::ExternallyPlayed)
    } else {
      None
    };
    Ok(source)
  }
}
