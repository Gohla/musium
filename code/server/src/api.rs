use actix_files::NamedFile;
use actix_web::{HttpResponse, ResponseError, web};
use actix_web::http::StatusCode;
use thiserror::Error;

use backend::{Backend, BackendConnectError, DatabaseQueryError, ScanError, UserAddVerifyError};
use core::model::{NewScanDirectory, NewUser};

use crate::auth::LoggedInUser;
use crate::scanner::Scanner;

// Scan directory

pub async fn list_scan_directories(
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.list_scan_directories()?))
}

pub async fn show_scan_directory_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let scan_directory = backend.connect_to_database()?.get_scan_directory_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(scan_directory))
}

pub async fn create_scan_directory(
  new_scan_directory: web::Json<NewScanDirectory>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.create_scan_directory(new_scan_directory.0)?))
}

pub async fn delete_scan_directory_by_directory(
  directory: web::Json<String>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  if backend.connect_to_database()?.delete_scan_directory_by_directory(&*directory)? {
    Ok(HttpResponse::Ok().finish())
  } else {
    Err(NotFoundFail)
  }
}

pub async fn delete_scan_directory_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  if backend.connect_to_database()?.delete_scan_directory_by_id(*id)? {
    Ok(HttpResponse::Ok().finish())
  } else {
    Err(NotFoundFail)
  }
}

// Albums

pub async fn list_albums(
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.list_albums()?))
}

pub async fn show_album_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let album = backend.connect_to_database()?.get_album_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(album))
}

// Track

pub async fn list_tracks(
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.list_tracks()?))
}

pub async fn show_track_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let track = backend.connect_to_database()?.get_track_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(track))
}

pub async fn download_track_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<NamedFile, ApiError> {
  use ApiError::*;
  let path = backend.connect_to_database()?.get_track_path_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(NamedFile::open(path)?)
}

// Artist

pub async fn list_artists(
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.list_artists()?))
}

pub async fn show_artist_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let artist = backend.connect_to_database()?.get_artist_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(artist))
}

// Users

pub async fn list_users(
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.list_users()?))
}

pub async fn show_user_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  let user = backend.connect_to_database()?.get_user_by_id(*id)?.ok_or(NotFoundFail)?;
  Ok(HttpResponse::Ok().json(user))
}

pub async fn show_my_user(
  logged_in_user: LoggedInUser,
) -> HttpResponse {
  HttpResponse::Ok().json(logged_in_user)
}

pub async fn create_user(
  new_user: web::Json<NewUser>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.create_user(new_user.0)?))
}

pub async fn delete_user_by_name(
  name: web::Json<String>,
  backend: web::Data<Backend>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  if *name == logged_in_user.user.name {
    return Err(CannotDeleteLoggedInUserFail);
  }
  if backend.connect_to_database()?.delete_user_by_name(&*name)? {
    Ok(HttpResponse::Ok().finish())
  } else {
    Err(NotFoundFail)
  }
}

pub async fn delete_user_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  use ApiError::*;
  if *id == logged_in_user.user.id {
    return Err(CannotDeleteLoggedInUserFail);
  }
  if backend.connect_to_database()?.delete_user_by_id(*id)? {
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
  backend: web::Data<Backend>,
) -> Result<HttpResponse, ApiError> {
  let rating = backend.connect_to_database()?.set_user_album_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

pub async fn set_user_track_rating(
  logged_in_user: LoggedInUser,
  id: web::Path<i32>,
  rating: web::Path<i32>,
  backend: web::Data<Backend>,
) -> Result<HttpResponse, ApiError> {
  let rating = backend.connect_to_database()?.set_user_track_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

pub async fn set_user_artist_rating(
  logged_in_user: LoggedInUser,
  id: web::Path<i32>,
  rating: web::Path<i32>,
  backend: web::Data<Backend>,
) -> Result<HttpResponse, ApiError> {
  let rating = backend.connect_to_database()?.set_user_artist_rating(logged_in_user.user.id, *id, *rating)?;
  Ok(HttpResponse::Ok().json(rating))
}

// Scanning

pub async fn scan(
  backend: web::Data<Backend>,
  scanner: web::Data<Scanner>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  let started_scan = scanner.scan(backend.into_inner());
  Ok(if started_scan {
    HttpResponse::Accepted().finish()
  } else {
    HttpResponse::Ok().finish()
  })
}

// Error type

#[derive(Debug, Error)]
pub enum ApiError {
  #[error(transparent)]
  BackendConnectFail(#[from] BackendConnectError),
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
  ScanFail(#[from] ScanError),
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
