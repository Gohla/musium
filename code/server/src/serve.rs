use std::net;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{App, HttpResponse, HttpServer, middleware, web};

use backend::Backend;

use crate::api::*;
use crate::auth::*;

pub async fn serve<A: net::ToSocketAddrs, C: Into<Vec<u8>>>(backend: Backend, bind_address: A, cookie_identity_secret_key: C) -> std::io::Result<()> {
  let backend_data = web::Data::new(backend);
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
      .route("/", web::get().to(index))
      // Auth
      .route("/login", web::post().to(login))
      .route("/logout", web::delete().to(logout))
      // API
      .route("/scan_directory", web::get().to(list_scan_directories))
      .route("/scan_directory/{id}", web::get().to(show_scan_directory))
      .route("/scan_directory", web::put().to(create_scan_directory))
      .route("/scan_directory", web::delete().to(delete_scan_directory_by_directory))
      .route("/scan_directory/{id}", web::delete().to(delete_scan_directory_by_id))
      .route("/tracks", web::get().to(list_tracks))
  })
    .bind(bind_address)?
    .run()
    .await
}

async fn index() -> HttpResponse {
  HttpResponse::Ok().finish()
}
