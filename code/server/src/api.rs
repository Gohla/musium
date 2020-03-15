use actix_web::{HttpResponse, ResponseError, web};
use thiserror::Error;

use backend::{Backend, BackendConnectError, DatabaseQueryError, UserAddVerifyError};
use core::model::{NewScanDirectory, NewUser};

use crate::auth::LoggedInUser;

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
  Ok(if let Some(scan_directory) = backend.connect_to_database()?.get_scan_directory_by_id(*id)? {
    HttpResponse::Ok().json(scan_directory)
  } else {
    HttpResponse::NotFound().finish()
  })
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
  Ok(if backend.connect_to_database()?.delete_scan_directory_by_directory(&*directory)? {
    HttpResponse::Ok().finish()
  } else {
    HttpResponse::NotFound().finish()
  })
}

pub async fn delete_scan_directory_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  Ok(if backend.connect_to_database()?.delete_scan_directory_by_id(*id)? {
    HttpResponse::Ok().finish()
  } else {
    HttpResponse::NotFound().finish()
  })
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
  Ok(if let Some(album) = backend.connect_to_database()?.get_album_by_id(*id)? {
    HttpResponse::Ok().json(album)
  } else {
    HttpResponse::NotFound().finish()
  })
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
  Ok(if let Some(track) = backend.connect_to_database()?.get_track_by_id(*id)? {
    HttpResponse::Ok().json(track)
  } else {
    HttpResponse::NotFound().finish()
  })
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
  Ok(if let Some(artist) = backend.connect_to_database()?.get_artist_by_id(*id)? {
    HttpResponse::Ok().json(artist)
  } else {
    HttpResponse::NotFound().finish()
  })
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
  Ok(if let Some(user) = backend.connect_to_database()?.get_user_by_id(*id)? {
    HttpResponse::Ok().json(user)
  } else {
    HttpResponse::NotFound().finish()
  })
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
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  // TODO: disallow logged in user to delete their self.
  Ok(if backend.connect_to_database()?.delete_user_by_name(&*name)? {
    HttpResponse::Ok().finish()
  } else {
    HttpResponse::NotFound().finish()
  })
}

pub async fn delete_user_by_id(
  id: web::Path<i32>,
  backend: web::Data<Backend>,
  _logged_in_user: LoggedInUser,
) -> Result<HttpResponse, ApiError> {
  // TODO: disallow logged in user to delete their self.
  Ok(if backend.connect_to_database()?.delete_user_by_id(*id)? {
    HttpResponse::Ok().finish()
  } else {
    HttpResponse::NotFound().finish()
  })
}

// Error type

#[derive(Debug, Error)]
pub enum ApiError {
  #[error(transparent)]
  BackendConnectFail(#[from] BackendConnectError),
  #[error(transparent)]
  DatabaseQueryFail(#[from] DatabaseQueryError),
  #[error(transparent)]
  UserAddFail(#[from] UserAddVerifyError),
}

impl ResponseError for ApiError {}
