use std::backtrace::Backtrace;

use musium_core::model::SpotifySource;

use crate::database::DatabaseConnection;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpotifySyncDatabaseError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error(transparent)]
  ApiFail(#[from] musium_spotify_sync::ApiError),
}

impl DatabaseConnection<'_> {
  pub(crate) async fn _spotify_sync(&self, spotify_sources: Vec<SpotifySource>) -> Result<(), SpotifySyncDatabaseError> {
    for spotify_source in spotify_sources {
      self._do_spotify_sync(spotify_source).await?;
    }
    Ok(())
  }

  async fn _do_spotify_sync(&self, _spotify_source: SpotifySource) -> Result<(), SpotifySyncDatabaseError> {
    //let _albums = self.database.spotify_sync.sync(spotify_source.access_token).await?;
    Ok(())
  }
}
