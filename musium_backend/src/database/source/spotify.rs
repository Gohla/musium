use std::backtrace::Backtrace;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, Level};
use url::Url;

use musium_core::model::{NewSpotifySource, SpotifySource, User};
use musium_core::schema;

use crate::database::DatabaseConnection;
use crate::sync::spotify::{AuthorizationError, CreateAuthorizationUrlError};

#[derive(Debug, Error)]
pub enum SpotifySourceCreateAuthorizationUrlError {
  #[error(transparent)]
  CreateAuthorizationUrlFail(#[from] CreateAuthorizationUrlError),
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("User {0:?} already has a spotify source")]
  AlreadyExistsFail(User),
}

impl DatabaseConnection<'_> {
  pub fn create_spotify_authorization_url<S1: Into<String>>(
    &self,
    user: &User,
    redirect_uri: S1,
    state: Option<String>,
  ) -> Result<Option<Url>, SpotifySourceCreateAuthorizationUrlError> {
    use SpotifySourceCreateAuthorizationUrlError::*;
    Ok(if let Some(spotify_sync) = &self.database.spotify_sync {
      // First check if user already has a spotify source.
      let select_by_user_id_query = {
        use schema::spotify_source::dsl::*;
        spotify_source.filter(user_id.eq(user.id))
      };
      let db_spotify_source: Option<SpotifySource> = time!("create_spotify_authorization_url.select", select_by_user_id_query.first::<SpotifySource>(&self.connection).optional()?);
      if db_spotify_source.is_some() {
        return Err(AlreadyExistsFail(user.clone()));
      }
      Some(spotify_sync.create_authorization_url(redirect_uri, state)?)
    } else {
      None
    })
  }
}

#[derive(Debug, Error)]
pub enum SpotifySourceCreateError {
  #[error(transparent)]
  CreateAuthorizationUrlFail(#[from] AuthorizationError),
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
}

impl DatabaseConnection<'_> {
  pub async fn create_spotify_source_from_authorization_callback<S1: Into<String>, S2: Into<String>>(
    &self,
    user_id: i32, // TODO: should be an &User.
    code: S1,
    redirect_uri: S2,
    state: Option<String>,
  ) -> Result<Option<SpotifySource>, SpotifySourceCreateError> {
    Ok(if let Some(spotify_sync) = &self.database.spotify_sync {
      let authorization_info = spotify_sync.authorization_callback(code, redirect_uri, state).await?;
      event!(Level::DEBUG, ?authorization_info, "Callback from Spotify with authorization info");
      let expiry_date = Utc::now() + Duration::seconds(authorization_info.expires_in as i64);
      let new_spotify_source = NewSpotifySource {
        user_id,
        enabled: true,
        refresh_token: authorization_info.refresh_token,
        access_token: authorization_info.access_token,
        expiry_date: expiry_date.naive_utc(),
      };
      // TODO: must be done in transaction for consistency.
      event!(Level::DEBUG, ?new_spotify_source, "Inserting Spotify source");
      let insert_query = {
        use schema::spotify_source::dsl::*;
        diesel::insert_into(spotify_source).values(new_spotify_source)
      };
      time!("create_spotify_authorization_url.insert", insert_query.execute(&self.connection)?);
      let select_query = {
        use schema::spotify_source::dsl::*;
        spotify_source.order(id.desc()).limit(1)
      };
      Some(time!("create_spotify_authorization_url.select_inserted", select_query.first::<SpotifySource>(&self.connection)?))
    } else {
      None
    })
  }
}
