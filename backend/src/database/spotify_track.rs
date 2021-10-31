use std::backtrace::Backtrace;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, Level};

use musium_core::model::{SpotifySource, SpotifyTrack};
use musium_core::schema;

use crate::database::DatabaseQueryError;
use crate::model::SpotifySourceEx;

use super::DatabaseConnection;

impl DatabaseConnection {
  pub fn get_spotify_track_by_track_id(&self, input_track_id: i32) -> Result<Option<SpotifyTrack>, DatabaseQueryError> {
    use schema::spotify_track::dsl::*;
    Ok(spotify_track
      .filter(track_id.eq(input_track_id))
      .first::<SpotifyTrack>(&self.connection)
      .optional()?)
  }
}

#[derive(Debug, Error)]
pub enum SpotifyPlayError {
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Failed to execute Spotify play API")]
  SpotifyApiFail(#[from] musium_spotify_client::PlaybackError, Backtrace),
}

impl DatabaseConnection {
  pub async fn play_spotify_track(&self, input_track_id: i32, input_user_id: i32) -> Result<bool, SpotifyPlayError> {
    let spotify_track: Option<SpotifyTrack> = {
      use schema::spotify_track::dsl::*;
      spotify_track
        .filter(track_id.eq(input_track_id))
        .first::<SpotifyTrack>(&self.connection)
        .optional()?
    };
    let spotify_source: Option<SpotifySource> = {
      use schema::spotify_source::dsl::*;
      spotify_source
        .filter(user_id.eq(input_user_id))
        .first::<SpotifySource>(&self.connection)
        .optional()?
    };
    if let (Some(spotify_track), Some(mut spotify_source)) = (spotify_track, spotify_source) {
      let mut authorization = spotify_source.to_spotify_authorization();
      self.inner.spotify_sync.start_or_resume_playback(&spotify_track.spotify_id, None, &mut authorization).await?;
      if spotify_source.update_from_spotify_authorization(authorization) {
        event!(Level::DEBUG, ?spotify_source, "Spotify source has changed, updating the database");
        spotify_source.save_changes::<SpotifySource>(&*self.connection)?;
      }
      return Ok(true);
    }
    Ok(false)
  }
}
