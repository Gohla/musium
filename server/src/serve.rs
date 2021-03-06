use std::net;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{App, HttpResponse, HttpServer, middleware, web};

use musium_backend::database::Database;
use musium_backend::sync::SyncClient;

use crate::api::*;
use crate::auth::*;

pub async fn serve<A: net::ToSocketAddrs, C: Into<Vec<u8>>>(database: Database, bind_address: A, cookie_identity_secret_key: C) -> std::io::Result<()> {
  let database_data = web::Data::new(database);
  let sync_client_data = web::Data::new(SyncClient::new());
  let cookie_identity_secret_key = cookie_identity_secret_key.into();
  HttpServer::new(move || {
    App::new()
      .wrap(middleware::Logger::default())
      .wrap(IdentityService::new(
        CookieIdentityPolicy::new(&cookie_identity_secret_key)
          .name("auth")
          .secure(false)
      ))
      .app_data(database_data.clone())
      .app_data(sync_client_data.clone())
      .route("/", web::get().to(index))
      // Auth
      .route("/login", web::post().to(login))
      .route("/logout", web::delete().to(logout))
      // API
      // Local source
      .route("/source/local", web::get().to(list_local_sources))
      .route("/source/local/{id}", web::get().to(show_local_source_by_id))
      .route("/source/local", web::post().to(create_or_enable_local_source))
      .route("/source/local/set_enabled/{id}", web::post().to(set_local_source_enabled))
      // Spotify source
      .route("/source/spotify", web::get().to(list_spotify_sources))
      .route("/source/spotify/{id}", web::get().to(show_spotify_source_by_id))
      .route("/source/spotify/request_authorization", web::get().to(request_spotify_authorization))
      .service(web::resource("/source/spotify/request_authorization/callback")
        .name("spotify_authorization_callback")
        .route(web::get().to(spotify_authorization_callback))
      )
      .route("/source/spotify/set_enabled/{id}", web::post().to(set_spotify_source_enabled))
      .route("/source/spotify/me", web::get().to(show_spotify_me))
      // Album
      .route("/album", web::get().to(list_albums))
      .route("/album/{id}", web::get().to(show_album_by_id))
      // Track
      .route("/track", web::get().to(list_tracks))
      .route("/track/{id}", web::get().to(show_track_by_id))
      .route("/track/play/{id}", web::get().to(play_track_by_id))
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
      .route("/sync", web::get().to(get_sync_status))
      .route("/sync", web::post().to(sync_all_sources))
      .route("/sync/local", web::post().to(sync_local_sources))
      .route("/sync/local/{id}", web::post().to(sync_local_source))
      .route("/sync/spotify", web::post().to(sync_spotify_sources))
      .route("/sync/spotify/{id}", web::post().to(sync_spotify_source))
  })
    .bind(bind_address)?
    .run()
    .await
}

async fn index() -> HttpResponse {
  HttpResponse::Ok().finish()
}
