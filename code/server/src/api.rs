use actix_web::{HttpResponse, ResponseError, web};
use thiserror::Error;

use backend::{Backend, BackendConnectError, DatabaseQueryError};
use core::model::NewScanDirectory;

// Scan directory

pub async fn list_scan_directories(backend: web::Data<Backend>) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.list_scan_directories()?))
}

pub async fn show_scan_directory(backend: web::Data<Backend>, id: web::Path<i32>) -> Result<HttpResponse, ApiError> {
  Ok(if let Some(scan_directory) = backend.connect_to_database()?.get_scan_directory(*id)? {
    HttpResponse::Ok().json(scan_directory)
  } else {
    HttpResponse::NotFound().finish()
  })
}

pub async fn create_scan_directory(backend: web::Data<Backend>, new_scan_directory: web::Json<NewScanDirectory>) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.create_scan_directory(new_scan_directory.clone())?))
}

pub async fn delete_scan_directory_by_directory(backend: web::Data<Backend>, directory: web::Json<String>) -> Result<HttpResponse, ApiError> {
  Ok(if backend.connect_to_database()?.delete_scan_directory_by_directory(directory.clone())? {
    HttpResponse::Ok().finish()
  } else {
    HttpResponse::NotFound().finish()
  })
}

pub async fn delete_scan_directory_by_id(backend: web::Data<Backend>, id: web::Path<i32>) -> Result<HttpResponse, ApiError> {
  Ok(if backend.connect_to_database()?.delete_scan_directory_by_id(*id)? {
    HttpResponse::Ok().finish()
  } else {
    HttpResponse::NotFound().finish()
  })
}

// Track

pub async fn list_tracks(backend: web::Data<Backend>) -> Result<HttpResponse, ApiError> {
  Ok(HttpResponse::Ok().json(backend.connect_to_database()?.list_tracks()?))
}

// Error type

#[derive(Debug, Error)]
pub enum ApiError {
  #[error(transparent)]
  BackendConnectFail(#[from] BackendConnectError),
  #[error(transparent)]
  DatabaseQueryFail(#[from] DatabaseQueryError),
}

impl ResponseError for ApiError {}
