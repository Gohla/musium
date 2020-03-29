use std::net;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{App, HttpResponse, HttpServer, middleware, web};

use musium_backend::Backend;

use crate::api::*;
use crate::auth::*;
use crate::scanner::Scanner;

pub async fn serve<A: net::ToSocketAddrs, C: Into<Vec<u8>>>(backend: Backend, bind_address: A, cookie_identity_secret_key: C) -> std::io::Result<()> {
  let backend_data = web::Data::new(backend);
  let scanner_data = web::Data::new(Scanner::new());
  let cookie_identity_secret_key = cookie_identity_secret_key.into();
  HttpServer::new(move || {
    App::new()
      .wrap(middleware::Logger::default())
      .wrap(IdentityService::new(
        CookieIdentityPolicy::new(&cookie_identity_secret_key)
          .name("auth")
          .secure(false)
      ))
      .app_data(backend_data.clone())
      .app_data(scanner_data.clone())
      .route("/", web::get().to(index))
      // Auth
      .route("/login", web::post().to(login))
      .route("/logout", web::delete().to(logout))
      // API
      // Scan directory
      .route("/scan_directory", web::get().to(list_scan_directories))
      .route("/scan_directory/{id}", web::get().to(show_scan_directory_by_id))
      .route("/scan_directory", web::post().to(create_scan_directory))
      .route("/scan_directory", web::delete().to(delete_scan_directory_by_directory))
      .route("/scan_directory/{id}", web::delete().to(delete_scan_directory_by_id))
      // Album
      .route("/album", web::get().to(list_albums))
      .route("/album/{id}", web::get().to(show_album_by_id))
      // Track
      .route("/track", web::get().to(list_tracks))
      .route("/track/{id}", web::get().to(show_track_by_id))
      .route("/track/download/{id}", web::get().to(download_track_by_id))
      // Artist
      .route("/artist", web::get().to(list_artists))
      .route("/artist/{id}", web::get().to(show_artist_by_id))
      // User
      .route("/user", web::get().to(list_users))
      .route("/user/me", web::get().to(show_my_user))
      .route("/user/{id}", web::get().to(show_user_by_id))
      .route("/user", web::post().to(create_user))
      .route("/user", web::delete().to(delete_user_by_name))
      .route("/user/{id}", web::delete().to(delete_user_by_id))
      // User data
      .route("/user/data/album/{id}/rating/{rating}", web::put().to(set_user_album_rating))
      .route("/user/data/track/{id}/rating/{rating}", web::put().to(set_user_track_rating))
      .route("/user/data/artist/{id}/rating/{rating}", web::put().to(set_user_artist_rating))
      // Scan
      .route("/scan", web::get().to(scan))
  })
    .bind(bind_address)?
    .run()
    .await
}

async fn index() -> HttpResponse {
  HttpResponse::Ok().finish()
}