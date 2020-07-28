use std::num::ParseIntError;
use std::str::FromStr;

use actix_files::NamedFile;
use actix_web::{http, HttpRequest, HttpResponse, ResponseError, web};
use actix_web::error::UrlGenerationError;
use actix_web::http::StatusCode;
use actix_web::web::Query;
use serde::Deserialize;
use thiserror::Error;
use tracing::{event, Level};

use musium_backend::database::{Database, DatabaseConnectError, DatabaseQueryError, sync::SyncError, user::UserAddVerifyError};
use musium_backend::database::source::spotify;
use musium_core::model::{NewLocalSource, NewUser};

use crate::auth::LoggedInUser;
use crate::sync::Sync;

// Local source

pub async fn list_local_sources(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_local_sources()?))
}

pub async fn show_local_source_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let local_source = database.connect()?.get_local_source_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(local_source))
}

pub async fn create_or_enable_local_source(
  new_local_source: web::Json<NewLocalSource>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.create_or_enable_local_source(&new_local_source)?))
}

pub async fn set_local_source_enabled(
  id: web::Path<i32>,
  enabled: web::Json<bool>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.set_local_source_enabled_by_id(*id, *enabled)?))
}

// Spotify source

pub async fn request_spotify_authorization(
  request: HttpRequest,
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let redirect_uri = request.url_for_static("spotify_authorization_callback").map_err(|e| UrlGenerationFail(e))?.to_string();
  // TODO: do not use user ID as state, since it is easily guessable.
  let url = database.connect()?.create_spotify_authorization_url(&logged_in_user.user, redirect_uri, Some(format!("{}", logged_in_user.user.id)))?;
  Ok(HttpResponse::TemporaryRedirect().header(http::header::LOCATION, url).finish().into_body())
}

#[derive(Deserialize)]
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
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  match query.into_inner() {
    SpotifyCallbackData { code: Some(code), error: None, state: Some(state) } => {
      let redirect_uri = request.url_for_static("spotify_authorization_callback").map_err(|e| UrlGenerationFail(e))?.to_string();
      let user_id = i32::from_str(&state)?; // TODO: do not abuse state to carry the user ID.
      let spotify_source = database.connect()?.create_spotify_source_from_authorization_callback(user_id, code, redirect_uri, Some(state)).await?;
      Ok(HttpResponse::Ok().json(spotify_source))
    }
    SpotifyCallbackData { error: Some(error), .. } => {
      event!(Level::ERROR, ?error, "Spotify authorization failed");
      Err(NotFoundFail) // TODO: better error
    }
    _ => {
      Err(NotFoundFail) // TODO: better error
    }
  }
}

pub(crate) async fn show_spotify_me(
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  let me_info = database.connect()?.show_spotify_me(&logged_in_user.user).await?;
  Ok(HttpResponse::Ok().json(me_info))
}

// Albums

pub async fn list_albums(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_albums()?))
}

pub async fn show_album_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let album = database.connect()?.get_album_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(album))
}

// Track

pub async fn list_tracks(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_tracks()?))
}

pub async fn show_track_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let track = database.connect()?.get_track_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(track))
}

pub async fn download_track_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<NamedFile, ApiError> {
  use ApiError::*;
  let path = database.connect()?.get_local_track_path_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(NamedFile::open(path)?)
}

// Artist

pub async fn list_artists(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_artists()?))
}

pub async fn show_artist_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let artist = database.connect()?.get_artist_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(artist))
}

// Users

pub async fn list_users(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_users()?))
}

pub async fn show_user_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let user = database.connect()?.get_user_by_id(*id)?.ok_or(NotFoundFail)?;
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
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.create_user(new_user.0)?))
}

pub async fn delete_user_by_name(
  name: web::Json<String>,
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  if *name == logged_in_user.user.name {
    return Err(CannotDeleteLoggedInUserFail);
  }
  if database.connect()?.delete_user_by_name(&*name)? {
    Ok(HttpResponse::Ok().finish())
  } else {
    Err(NotFoundFail)
  }
}

pub async fn delete_user_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  if *id == logged_in_user.user.id {
    return Err(CannotDeleteLoggedInUserFail);
  }
  if database.connect()?.delete_user_by_id(*id)? {
    Ok(HttpResponse::Ok().finish())
  } else {
    Err(NotFoundFail)
  }
}

// User data

pub async fn set_user_album_rating(
  logged_in_user: LoggedInUser,
  id: web::Path<i32>,
  rating: web::Path<i32>,
  database: web::Data<Database>,
) -> Result<HttpResponse, ApiError> {
  let rating = database.connect()?.set_user_album_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

pub async fn set_user_track_rating(
  logged_in_user: LoggedInUser,
  id: web::Path<i32>,
  rating: web::Path<i32>,
  database: web::Data<Database>,
) -> Result<HttpResponse, ApiError> {
  let rating = database.connect()?.set_user_track_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

pub async fn set_user_artist_rating(
  logged_in_user: LoggedInUser,
  id: web::Path<i32>,
  rating: web::Path<i32>,
  database: web::Data<Database>,
) -> Result<HttpResponse, ApiError> {
  let rating = database.connect()?.set_user_artist_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

// Scanning

pub async fn sync(
  database: web::Data<Database>,
  scanner: web::Data<Sync>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  let started_sync = scanner.sync(database.into_inner());
  Ok(if started_sync {
    HttpResponse::Accepted().finish()
  } else {
    HttpResponse::Ok().finish()
  })
}

// Error type

#[derive(Debug, Error)]
pub enum ApiError {
  #[error(transparent)]
  BackendConnectFail(#[from] DatabaseConnectError),
  #[error(transparent)]
  DatabaseQueryFail(#[from] DatabaseQueryError),
  #[error("Resource was not found")]
  NotFoundFail,
  #[error("Cannot delete logged-in user")]
  CannotDeleteLoggedInUserFail,
  #[error("URL generation failed: {0:?}")]
  UrlGenerationFail(UrlGenerationError),
  #[error(transparent)]
  SpotifySourceCreateAuthorizationUrlFail(#[from] spotify::CreateAuthorizationUrlError),
  #[error(transparent)]
  SpotifySourceCreateFail(#[from] spotify::CreateError),
  #[error(transparent)]
  SpotifyMeInfoError(#[from] spotify::MeInfoError),
  #[error(transparent)]
  ParseUserIdFail(#[from] ParseIntError),
  #[error(transparent)]
  UserAddFail(#[from] UserAddVerifyError),
  #[error(transparent)]
  IoFail(#[from] std::io::Error),
  #[error(transparent)]
  SyncFail(#[from] SyncError),
  #[error("Thread pool is gone")]
  ThreadPoolGoneFail,
}

impl ResponseError for ApiError {
  fn status_code(&self) -> StatusCode {
    use ApiError::*;
    match self {
      NotFoundFail => StatusCode::NOT_FOUND,
      _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }
}

