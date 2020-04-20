use actix_files::NamedFile;
use actix_web::{HttpResponse, ResponseError, web};
use actix_web::http::StatusCode;
use thiserror::Error;

use musium_backend::database::{Database, DatabaseConnectError, DatabaseQueryError, sync::SyncError, user::UserAddVerifyError};
use musium_core::model::{NewSource, NewUser};
use musium_core::model::collection::{Albums, Tracks};

use crate::auth::LoggedInUser;
use crate::scanner::Sync;

// Source

pub async fn list_sources(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.list_sources()?))
}

pub async fn show_source_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let source = database.connect()?.get_source_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(source))
}

pub async fn create_scan_directory(
  new_source: web::Json<NewSource>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(database.connect()?.create_source(new_source.0)?))
}

pub async fn delete_source_by_id(
  id: web::Path<i32>,
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  if database.connect()?.delete_source_by_id(*id)? {
    Ok(HttpResponse::Ok().finish())
  } else {
    Err(NotFoundFail)
  }
}

// Albums

pub async fn list_albums(
  database: web::Data<Database>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  let albums: Albums = database.connect()?.list_albums()?.into();
  Ok(HttpResponse::Ok().json(albums))
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
  let tracks: Tracks = database.connect()?.list_tracks()?.into();
  Ok(HttpResponse::Ok().json(tracks))
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
  let path = database.connect()?.get_track_path_by_id(*id)?.ok_or(NotFoundFail)?;
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
