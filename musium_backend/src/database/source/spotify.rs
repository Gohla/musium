use std::backtrace::Backtrace;

use chrono::{Duration, Utc};
use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, Level};
use url::Url;

use musium_core::model::{NewSpotifySource, SpotifySource, User};
use musium_core::schema;

use crate::database::DatabaseConnection;
use crate::sync::spotify;
use musium_core::api::SpotifyMeInfo;

// Create authorization URL

#[derive(Debug, Error)]
pub enum CreateAuthorizationUrlError {
  #[error(transparent)]
  SpotifyCreateAuthorizationUrlFail(#[from] spotify::CreateAuthorizationUrlError),
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
  ) -> Result<Url, CreateAuthorizationUrlError> {
    use CreateAuthorizationUrlError::*;
    // First check if user already has a spotify source.
    let select_by_user_id_query = {
      use schema::spotify_source::dsl::*;
      spotify_source.filter(user_id.eq(user.id))
    };
    let db_spotify_source: Option<SpotifySource> = time!("create_spotify_authorization_url.select", select_by_user_id_query.first::<SpotifySource>(&self.connection).optional()?);
    if db_spotify_source.is_some() {
      return Err(AlreadyExistsFail(user.clone()));
    }
    Ok(self.database.spotify_sync.create_authorization_url(redirect_uri, state)?)
  }
}

// Source creation

#[derive(Debug, Error)]
pub enum CreateError {
  #[error(transparent)]
  SpotifyAuthorizationFail(#[from] spotify::ApiError),
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
  ) -> Result<SpotifySource, CreateError> {
    let authorization_info = self.database.spotify_sync.authorization_callback(code, redirect_uri, state).await?;
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
    Ok(time!("create_spotify_authorization_url.select_inserted", select_query.first::<SpotifySource>(&self.connection)?))
  }
}

// Refresh access token

#[derive(Debug, Error)]
pub enum RefreshAccessTokenError {
  #[error(transparent)]
  SpotifyApiFail(#[from] spotify::ApiError),
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
}

impl DatabaseConnection<'_> {
  async fn refresh_access_token_if_needed(&self, spotify_source: SpotifySource) -> Result<SpotifySource, RefreshAccessTokenError> {
    if Utc::now().naive_utc() >= spotify_source.expiry_date {
      self.refresh_access_token(spotify_source).await
    } else {
      Ok(spotify_source)
    }
  }

  async fn refresh_access_token(&self, mut spotify_source: SpotifySource) -> Result<SpotifySource, RefreshAccessTokenError> {
    let refresh_info = self.database.spotify_sync.refresh_access_token(&spotify_source.refresh_token).await?;
    event!(Level::DEBUG, ?spotify_source, ?refresh_info, "Updating Spotify source with new access token");
    spotify_source.access_token = refresh_info.access_token.clone();
    spotify_source.expiry_date = (Utc::now() + Duration::seconds(refresh_info.expires_in as i64)).naive_utc();
    Ok(time!("refresh_access_token.update", spotify_source.save_changes::<SpotifySource>(&*self.connection)?))
  }
}


// Me info

#[derive(Debug, Error)]
pub enum MeInfoError {
  #[error("User {0:?} does not have a spotify source")]
  NoSpotifySource(User),
  #[error(transparent)]
  RefreshAccessTokenFail(#[from] RefreshAccessTokenError),
  #[error(transparent)]
  SpotifyApiFail(#[from] spotify::ApiError),
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
}

impl DatabaseConnection<'_> {
  pub async fn show_spotify_me(&self, user: &User) -> Result<SpotifyMeInfo, MeInfoError> {
    use MeInfoError::*;
    let spotify_source = {
      use schema::spotify_source::dsl::*;
      let query = spotify_source.filter(user_id.eq(user.id));
      time!("get_spotify_me_info.select", query.first::<SpotifySource>(&self.connection).optional()?)
    };
    if let Some(spotify_source) = spotify_source {
      let spotify_source = self.refresh_access_token_if_needed(spotify_source).await?;
      Ok(self.database.spotify_sync.me(spotify_source.access_token).await?)
    } else {
      Err(NoSpotifySource(user.clone()))
    }
  }
}
