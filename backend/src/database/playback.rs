use std::backtrace::Backtrace;
use std::path::PathBuf;

use thiserror::Error;

use musium_core::api::PlaySourceKind;

use crate::database::spotify_track::SpotifyPlayError;

use super::{DatabaseConnection, DatabaseQueryError};

pub enum BackendPlaySource {
  AudioData(PathBuf),
  ExternallyPlayedOnSpotify,
}

#[derive(Debug, Error)]
pub enum PlayError {
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] DatabaseQueryError, Backtrace),
  #[error("Failed to play Spotify track")]
  SpotifyPlayFail(#[from] SpotifyPlayError, Backtrace),
}

impl DatabaseConnection {
  pub fn get_track_play_source_kind_by_id(&self, track_id: i32) -> Result<Option<PlaySourceKind>, PlayError> {
    Ok(if let Some(_) = self.get_local_track_path_by_track_id(track_id)? {
      Some(PlaySourceKind::AudioData)
    } else if let Some(_) = self.get_spotify_track_by_track_id(track_id)? {
      Some(PlaySourceKind::ExternalOnSpotify)
    } else {
      None
    })
  }

  pub async fn play_track_by_id(&self, track_id: i32, user_id: i32) -> Result<Option<BackendPlaySource>, PlayError> {
    Ok(if let Some(path) = self.get_local_track_path_by_track_id(track_id)? { // TODO: fix blocking code in async
      Some(BackendPlaySource::AudioData(path))
    } else if let true = self.play_spotify_track(track_id, user_id).await? {
      Some(BackendPlaySource::ExternallyPlayedOnSpotify)
    } else {
      None
    })
  }
}
