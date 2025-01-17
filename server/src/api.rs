use std::backtrace::Backtrace;
use std::num::ParseIntError;
use std::str::FromStr;

use actix_files::NamedFile;
use actix_web::{Either, http, HttpRequest, HttpResponse, ResponseError, web};
use actix_web::error::UrlGenerationError;
use actix_web::http::StatusCode;
use actix_web::web::Query;
use serde::Deserialize;
use thiserror::Error;
use tracing::{event, Level};

use musium_backend::database::{Database, DatabaseConnectError, DatabaseQueryError, user::UserAddVerifyError};
use musium_backend::database::playback::{BackendPlaySource, PlayError};
use musium_backend::database::source::spotify;
use musium_backend::sync::{SyncClient, SyncClientError};
use musium_core::api::InternalServerError;
use musium_core::model::{NewLocalSource, NewUser};

use crate::auth::LoggedInUser;

// TODO: all async functions that touch the database are blocking! this should not be the case!

// Local source

pub(crate) async fn list_local_sources(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let local_sources = database.connect()?.list_local_sources()?;
  Ok(HttpResponse::Ok().json(local_sources))
}

pub(crate) async fn show_local_source_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let local_source = database.connect()?.get_local_source_by_id(*id)?;
  Ok(HttpResponse::Ok().json(local_source))
}

pub(crate) async fn create_or_enable_local_source(
  new_local_source: web::Json<NewLocalSource>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  Ok(HttpResponse::Ok().json(database.connect()?.create_or_enable_local_source(&new_local_source)?))
}

pub(crate) async fn set_local_source_enabled(
  id: web::Path<i32>,
  enabled: web::Json<bool>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  Ok(HttpResponse::Ok().json(database.connect()?.set_local_source_enabled_by_id(*id, *enabled)?))
}

// Spotify source

pub(crate) async fn list_spotify_sources(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let local_sources = database.connect()?.list_spotify_sources()?;
  Ok(HttpResponse::Ok().json(local_sources))
}

pub(crate) async fn show_spotify_source_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let local_source = database.connect()?.get_spotify_source_by_id(*id)?;
  Ok(HttpResponse::Ok().json(local_source))
}

pub(crate) async fn request_spotify_authorization(
  request: HttpRequest,
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  use InternalError::*;
  let redirect_uri = request.url_for_static("spotify_authorization_callback").map_err(|e| UrlGenerationFail(e))?.to_string();
  // TODO: do not use user ID as state, since it is easily guessable.
  let url = database.connect()?.create_spotify_authorization_url(&logged_in_user.user, redirect_uri, Some(format!("{}", logged_in_user.user.id)))?;
  Ok(HttpResponse::TemporaryRedirect().append_header((http::header::LOCATION, url)).finish())
}

#[derive(Deserialize, Debug)]
pub(crate) struct SpotifyCallbackData {
  code: Option<String>,
  error: Option<String>,
  #[allow(dead_code)] state: Option<String>,
}

pub(crate) async fn spotify_authorization_callback(
  request: HttpRequest,
  query: Query<SpotifyCallbackData>,
  database: web::Data<Database>,
  //logged_in_user: LoggedInUser, // TODO: require a logged-in user.
) -> Result<HttpResponse, InternalError> {
  use InternalError::*;
  match query.into_inner() {
    SpotifyCallbackData { code: Some(code), error: None, state: Some(state) } => {
      let redirect_uri = request.url_for_static("spotify_authorization_callback").map_err(|e| UrlGenerationFail(e))?.to_string();
      let user_id = i32::from_str(&state)?; // TODO: do not abuse state to carry the user ID.
      let spotify_source = database.connect()?.create_spotify_source_from_authorization_callback(user_id, code, redirect_uri, Some(state)).await?;
      Ok(HttpResponse::Ok().json(spotify_source))
    }
    SpotifyCallbackData { error: Some(error), .. } => {
      Err(SpotifyAuthorizationCallbackFail(error))
    }
    _ => {
      Err(SpotifyAuthorizationCallbackUnexpectedFail)
    }
  }
}

pub(crate) async fn set_spotify_source_enabled(
  id: web::Path<i32>,
  enabled: web::Json<bool>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  Ok(HttpResponse::Ok().json(database.connect()?.set_spotify_source_enabled_by_id(*id, *enabled)?))
}

pub(crate) async fn show_spotify_me(
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let me_info = database.connect()?.show_spotify_me(&logged_in_user.user).await?;
  Ok(HttpResponse::Ok().json(me_info))
}

// Albums

pub async fn list_albums(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_albums()?))
}

pub async fn show_album_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let album = database.connect()?.get_album_by_id(*id)?;
  Ok(HttpResponse::Ok().json(album))
}

// Track

pub async fn list_tracks(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_tracks()?))
}

pub async fn show_track_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let track = database.connect()?.get_track_by_id(*id)?;
  Ok(HttpResponse::Ok().json(track))
}

// Artist

pub async fn list_artists(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_artists()?))
}

pub async fn show_artist_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let artist = database.connect()?.get_artist_by_id(*id)?;
  Ok(HttpResponse::Ok().json(artist))
}

// Playback

pub async fn show_track_play_source_kind(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let play_source_kind = database.connect()?.get_track_play_source_kind_by_id(*id)?;
  Ok(HttpResponse::Ok().json(play_source_kind))
}

pub async fn play_track_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<Either<NamedFile, HttpResponse>, InternalError> {
  if let Some(play_source) = database.connect()?.play_track_by_id(*id, logged_in_user.user.id).await? {
    let response = match play_source {
      BackendPlaySource::AudioData(path) => Either::Left(NamedFile::open_async(path).await?),
      BackendPlaySource::ExternallyPlayedOnSpotify => Either::Right(HttpResponse::Accepted().finish()),
    };
    Ok(response)
  } else {
    Ok(Either::Right(HttpResponse::NotFound().finish()))
  }
}

// Users

pub async fn list_users(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_users()?))
}

pub async fn show_user_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let user = database.connect()?.get_user_by_id(*id)?;
  Ok(HttpResponse::Ok().json(user))
}

pub async fn show_my_user(
  logged_in_user: LoggedInUser,
) -> HttpResponse {
  HttpResponse::Ok().json(logged_in_user)
}

pub async fn create_user(
  new_user: web::Json<NewUser>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  Ok(HttpResponse::Ok().json(database.connect()?.create_user(new_user.0)?))
}

pub async fn delete_user_by_name(
  name: web::Json<String>,
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  use InternalError::*;
  if *name == logged_in_user.user.name {
    return Err(CannotDeleteLoggedInUserFail);
  }
  if database.connect()?.delete_user_by_name(&*name)? {
    Ok(HttpResponse::Ok().finish())
  } else {
    Ok(HttpResponse::NotFound().finish())
  }
}

pub async fn delete_user_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  use InternalError::*;
  if *id == logged_in_user.user.id {
    return Err(CannotDeleteLoggedInUserFail);
  }
  if database.connect()?.delete_user_by_id(*id)? {
    Ok(HttpResponse::Ok().finish())
  } else {
    Ok(HttpResponse::NotFound().finish())
  }
}

// User data

pub async fn set_user_album_rating(
  logged_in_user: LoggedInUser,
  id: web::Path<i32>,
  rating: web::Path<i32>,
  database: web::Data<Database>,
) -> Result<HttpResponse, InternalError> {
  let rating = database.connect()?.set_user_album_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

pub async fn set_user_track_rating(
  logged_in_user: LoggedInUser,
  id: web::Path<i32>,
  rating: web::Path<i32>,
  database: web::Data<Database>,
) -> Result<HttpResponse, InternalError> {
  let rating = database.connect()?.set_user_track_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

pub async fn set_user_artist_rating(
  logged_in_user: LoggedInUser,
  id: web::Path<i32>,
  rating: web::Path<i32>,
  database: web::Data<Database>,
) -> Result<HttpResponse, InternalError> {
  let rating = database.connect()?.set_user_artist_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

// Sync

pub async fn get_sync_status(
  database: web::Data<Database>,
  sync_client: web::Data<SyncClient>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let sync_status = sync_client.get_status(database.into_inner()).await?;
  Ok(HttpResponse::Ok().json(sync_status))
}

pub async fn sync_all_sources(
  database: web::Data<Database>,
  sync_client: web::Data<SyncClient>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let sync_status = sync_client.sync_all_sources(database.into_inner()).await?;
  Ok(HttpResponse::Ok().json(sync_status))
}

pub async fn sync_local_sources(
  database: web::Data<Database>,
  sync_client: web::Data<SyncClient>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let sync_status = sync_client.sync_local_sources(database.into_inner()).await?;
  Ok(HttpResponse::Ok().json(sync_status))
}

pub async fn sync_local_source(
  id: web::Path<i32>,
  database: web::Data<Database>,
  sync_client: web::Data<SyncClient>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let sync_status = sync_client.sync_local_source(id.into_inner(), database.into_inner()).await?;
  Ok(HttpResponse::Ok().json(sync_status))
}

pub async fn sync_spotify_sources(
  database: web::Data<Database>,
  sync_client: web::Data<SyncClient>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let sync_status = sync_client.sync_spotify_sources(database.into_inner()).await?;
  Ok(HttpResponse::Ok().json(sync_status))
}

pub async fn sync_spotify_source(
  id: web::Path<i32>,
  database: web::Data<Database>,
  sync_client: web::Data<SyncClient>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, InternalError> {
  let sync_status = sync_client.sync_spotify_source(id.into_inner(), database.into_inner()).await?;
  Ok(HttpResponse::Ok().json(sync_status))
}

// Error type

#[derive(Debug, Error)]
pub enum InternalError {
  #[error("Failed to connect to the database")]
  BackendConnectFail(#[from] DatabaseConnectError, Backtrace),
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] DatabaseQueryError, Backtrace),
  #[error("Cannot delete logged-in user")]
  CannotDeleteLoggedInUserFail,
  #[error("URL generation failed: {0:?}")]
  UrlGenerationFail(UrlGenerationError),
  #[error("Failed to create a Spotify authorization URL")]
  SpotifySourceCreateAuthorizationUrlFail(#[from] spotify::CreateAuthorizationUrlError, Backtrace),
  #[error("Spotify authorization callback resulted in an error: {0}")]
  SpotifyAuthorizationCallbackFail(String),
  #[error("Spotify authorization callback returned an unexpected result")]
  SpotifyAuthorizationCallbackUnexpectedFail,
  #[error("Failed to create a Spotify source")]
  SpotifySourceCreateFail(#[from] spotify::CreateError, Backtrace),
  #[error("Failed to request Spotify user info")]
  SpotifyMeInfoError(#[from] spotify::MeInfoError, Backtrace),
  #[error(transparent)]
  ParseUserIdFail(#[from] ParseIntError),
  #[error("Failed to add a user")]
  UserAddFail(#[from] UserAddVerifyError, Backtrace),
  #[error("I/O failure")]
  IoFail(#[from] std::io::Error, Backtrace),
  #[error("Failed to play track")]
  PlayFail(#[from] PlayError, Backtrace),
  #[error("Failed to start sync or get sync status")]
  SyncFail(#[from] SyncClientError, Backtrace),
}

impl ResponseError for InternalError {
  fn status_code(&self) -> StatusCode { StatusCode::INTERNAL_SERVER_ERROR }

  fn error_response(&self) -> HttpResponse {
    let format_error = musium_core::format_error::FormatError::new(self);
    event!(Level::ERROR, "{:?}", format_error);
    HttpResponse::build(self.status_code()).json(InternalServerError {
      message: self.to_string()
    })
  }
}
