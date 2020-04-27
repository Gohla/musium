use diesel::prelude::*;
use url::Url;

use musium_core::model::{NewSpotifySource, SpotifySource, User};
use musium_core::schema;

use crate::database::{DatabaseConnection, DatabaseQueryError};
use crate::sync::spotify::SpotifySyncRequestFail;

impl DatabaseConnection<'_> {
  pub fn create_spotify_authorization_url<S1: Into<String>, S2: Into<String>>(
    &self,
    redirect_uri: S1,
    state: Option<S2>,
  ) -> Result<Option<Url>, SpotifySyncRequestFail> {
    if let Some(spotify_sync) = &self.database.spotify_sync {
      Ok(Some(spotify_sync.create_authorization_url(redirect_uri, state)?))
    } else {
      Ok(None)
    }
  }

  pub fn spotify_authorization_callback(&self, code: String, state: Option<String>) {

  }
}
