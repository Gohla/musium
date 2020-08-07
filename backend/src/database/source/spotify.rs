use std::backtrace::Backtrace;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, Level};

use musium_core::api::SpotifyMeInfo;
use musium_core::model::{NewSpotifySource, SpotifySource, User};
use musium_core::schema;

use crate::database::{DatabaseConnection, DatabaseQueryError};
use crate::model::SpotifySourceEx;

impl DatabaseConnection<'_> {
  pub fn list_spotify_sources(&self) -> Result<Vec<SpotifySource>, DatabaseQueryError> {
    use schema::spotify_source::dsl::*;
    Ok(time!("list_spotify_sources.select", spotify_source.load::<SpotifySource>(&self.connection)?))
  }

  pub fn get_spotify_source_by_id(&self, local_source_id: i32) -> Result<Option<SpotifySource>, DatabaseQueryError> {
    let query = {
      use schema::spotify_source::dsl::*;
      spotify_source.find(local_source_id)
    };
    Ok(time!("get_spotify_source_by_id.select", query.first::<SpotifySource>(&self.connection).optional()?))
  }
}

// Create authorization URL

#[derive(Debug, Error)]
pub enum CreateAuthorizationUrlError {
  #[error(transparent)]
  SpotifyCreateAuthorizationUrlFail(#[from] musium_spotify_client::CreateAuthorizationUrlError),
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
  ) -> Result<String, CreateAuthorizationUrlError> {
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
  #[error("Failed to authorize with Spotify")]
  SpotifyAuthorizationFail(#[from] musium_spotify_client::AuthorizationHttpRequestError, Backtrace),
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
    let new_spotify_source = NewSpotifySource {
      user_id,
      enabled: true,
      refresh_token: authorization_info.refresh_token,
      access_token: authorization_info.access_token,
      expiry_date: authorization_info.expiry_date,
    };
    self.connection.transaction::<_, CreateError, _>(|| {
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
    })
  }
}

// Me info

#[derive(Debug, Error)]
pub enum MeInfoError {
  #[error("User {0:?} does not have a spotify source")]
  NoSpotifySource(User),
  #[error("Failed to execute Spotify API")]
  SpotifyApiFail(#[from] musium_spotify_client::HttpRequestError, Backtrace),
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
}

impl DatabaseConnection<'_> {
  pub async fn show_spotify_me(&self, user: &User) -> Result<SpotifyMeInfo, MeInfoError> {
    use MeInfoError::*;
    let spotify_source: Option<SpotifySource> = {
      use schema::spotify_source::dsl::*;
      let query = spotify_source.filter(user_id.eq(user.id));
      time!("get_spotify_me_info.select", query.first::<SpotifySource>(&self.connection).optional()?)
    };
    if let Some(mut spotify_source) = spotify_source {
      let mut authorization = spotify_source.to_spotify_authorization();
      let spotify_sync_me_info = self.database.spotify_sync.me(&mut authorization).await?;
      if spotify_source.update_from_spotify_authorization(authorization) {
        event!(Level::DEBUG, ?spotify_source, "Spotify source has changed, updating the database");
        spotify_source.save_changes::<SpotifySource>(&*self.connection)?;
      }
      Ok(SpotifyMeInfo {
        display_name: spotify_sync_me_info.display_name,
      })
    } else {
      Err(NoSpotifySource(user.clone()))
    }
  }
}
