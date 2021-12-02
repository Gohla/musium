#![feature(backtrace)]

use std::backtrace::Backtrace;
use std::fmt::{Debug, Formatter};

use async_trait::async_trait;
use reqwest::{Client as ReqwestHttpClient, header::CONTENT_TYPE, header::ToStrError, Method, redirect, RequestBuilder, Response, StatusCode};
pub use reqwest::Url;
use serde::Serialize;
use thiserror::Error;

pub use musium_client::Client;
use musium_core::{
  api::{InternalServerError, SpotifyMeInfo},
  model::{
    *,
    collection::{AlbumsRaw, TracksRaw},
  },
};
use musium_core::api::{AudioCodec, PlaySource, PlaySourceKind, SyncStatus};

#[derive(Clone)]
pub struct HttpClient {
  client: ReqwestHttpClient,
  url: Url,
}

// Creation

#[derive(Debug, Error)]
pub enum HttpClientCreateError {
  #[error(transparent)]
  HttpClientCreateFail(#[from] reqwest::Error)
}

impl HttpClient {
  pub fn new(url: Url) -> Result<Self, HttpClientCreateError> {
    let client: ReqwestHttpClient = ReqwestHttpClient::builder()
      .cookie_store(true)
      .redirect(redirect::Policy::none())
      .build()?;
    Ok(Self { client, url })
  }

  pub fn set_url(&mut self, url: Url) {
    self.url = url;
  }
}

// Error types

#[derive(Debug, Error)]
pub enum HttpRequestError {
  #[error(transparent)]
  UrlJoinFail(#[from] url::ParseError),
  #[error(transparent)]
  RequestFail(#[from] reqwest::Error),
  #[error("Server responded with an internal error")]
  InternalServerFail(#[from] InternalServerError, Backtrace),
  #[error("Server responded with unexpected status code: {0}")]
  UnexpectedStatusCode(StatusCode, Backtrace),
}

#[derive(Debug, Error)]
pub enum SpotifySourceError {
  #[error(transparent)]
  CreateSpotifySourceAuthorizationUrlFail(#[from] CreateSpotifySourceAuthorizationUrlError),
  #[error(transparent)]
  HttpRequestFail(#[from] HttpRequestError),
  #[error(transparent)]
  UrlJoinFail(#[from] url::ParseError),
  #[error(transparent)]
  RequestFail(#[from] reqwest::Error),
  #[error("Server responded with an internal error")]
  InternalServerFail(#[from] InternalServerError, Backtrace),
  #[error("Server responded with unexpected status code: {0}")]
  UnexpectedStatusCode(StatusCode, Backtrace),
}

#[derive(Debug, Error)]
pub enum CreateSpotifySourceAuthorizationUrlError {
  #[error("The LOCATION header is missing from the HTTP response")]
  LocationHeaderMissingFail,
  #[error("Failed to convert the LOCATION header in the HTTP response to a string")]
  LocationHeaderToStringFail(#[from] ToStrError),
}

#[async_trait]
impl Client for HttpClient {
  // Login

  type LoginError = HttpRequestError;

  async fn login(&self, user_login: &UserLogin) -> Result<User, Self::LoginError> {
    let response = self.post_simple_with_json("login", user_login).await?;
    Ok(response.json().await?)
  }

  // Local source

  type LocalSourceError = HttpRequestError;

  async fn list_local_sources(&self) -> Result<Vec<LocalSource>, Self::LocalSourceError> {
    let response = self.get_simple("source/local").await?;
    Ok(response.json().await?)
  }

  async fn get_local_source_by_id(&self, id: i32) -> Result<Option<LocalSource>, Self::LocalSourceError> {
    let response = self.get_simple(format!("source/local/{}", id)).await?;
    Ok(response.json().await?)
  }

  async fn create_or_enable_local_source(&self, new_local_source: &NewLocalSource) -> Result<LocalSource, Self::LocalSourceError> {
    let response = self.post_simple_with_json("source/local", new_local_source).await?;
    Ok(response.json().await?)
  }

  async fn set_local_source_enabled_by_id(&self, id: i32, enabled: bool) -> Result<Option<LocalSource>, Self::LocalSourceError> {
    let response = self.post_simple_with_json(format!("source/local/set_enabled/{}", id), &enabled).await?;
    Ok(response.json().await?)
  }

  // Spotify source

  type SpotifySourceError = SpotifySourceError;

  async fn list_spotify_sources(&self) -> Result<Vec<SpotifySource>, Self::SpotifySourceError> {
    let response = self.get_simple("source/spotify").await?;
    Ok(response.json().await?)
  }

  async fn get_spotify_source_by_id(&self, id: i32) -> Result<Option<SpotifySource>, Self::SpotifySourceError> {
    let response = self.get_simple(format!("source/spotify/{}", id)).await?;
    Ok(response.json().await?)
  }

  async fn create_spotify_source_authorization_url(&self) -> Result<String, Self::SpotifySourceError> {
    use CreateSpotifySourceAuthorizationUrlError::*;
    use SpotifySourceError::*;
    let response = self.get("source/spotify/request_authorization", |r| r, &[StatusCode::TEMPORARY_REDIRECT]).await?;
    if let Some(url) = response.headers().get(reqwest::header::LOCATION) {
      Ok(url.to_str().map_err(|e| LocationHeaderToStringFail(e))?.to_owned())
    } else {
      Err(CreateSpotifySourceAuthorizationUrlFail(LocationHeaderMissingFail))
    }
  }

  async fn set_spotify_source_enabled_by_id(&self, id: i32, enabled: bool) -> Result<Option<SpotifySource>, Self::SpotifySourceError> {
    let response = self.post_simple_with_json(format!("source/spotify/set_enabled/{}", id), &enabled).await?;
    Ok(response.json().await?)
  }

  async fn show_spotify_me(&self) -> Result<SpotifyMeInfo, Self::SpotifySourceError> {
    let response = self.get_simple("source/spotify/me").await?;
    Ok(response.json().await.map_err(|e| HttpRequestError::RequestFail(e))?)
  }

  // Album

  type AlbumError = HttpRequestError;

  async fn list_albums(&self) -> Result<AlbumsRaw, Self::AlbumError> {
    let response = self.get_simple("album").await?;
    let albums_raw: AlbumsRaw = response.json().await?;
    Ok(albums_raw)
  }

  async fn get_album_by_id(&self, id: i32) -> Result<Option<LocalAlbum>, Self::AlbumError> {
    let response = self.get_simple(format!("album/{}", id)).await?;
    Ok(response.json().await?)
  }

  // Track

  type TrackError = HttpRequestError;

  async fn list_tracks(&self) -> Result<TracksRaw, Self::TrackError> {
    let response = self.get_simple("track").await?;
    let tracks_raw: TracksRaw = response.json().await?;
    Ok(tracks_raw)
  }

  async fn get_track_by_id(&self, id: i32) -> Result<Option<LocalTrack>, Self::TrackError> {
    let response = self.get_simple(format!("track/{}", id)).await?;
    Ok(response.json().await?)
  }

  // Artist

  type ArtistError = HttpRequestError;

  async fn list_artists(&self) -> Result<Vec<Artist>, Self::ArtistError> {
    let response = self.get_simple("artist").await?;
    Ok(response.json().await?)
  }

  async fn get_artist_by_id(&self, id: i32) -> Result<Option<Artist>, Self::ArtistError> {
    let response = self.get_simple(format!("artist/{}", id)).await?;
    Ok(response.json().await?)
  }

  // Playback

  type PlaybackError = HttpRequestError;


  async fn get_track_play_source_kind_by_id(&self, id: i32) -> Result<Option<PlaySourceKind>, Self::PlaybackError> {
    let response = self.get_simple(format!("track/play_source_kind/{}", id)).await?;
    Ok(response.json().await?)
  }

  async fn play_track_by_id(&self, id: i32) -> Result<Option<PlaySource>, Self::PlaybackError> {
    let response = self.get(
      format!("track/play/{}", id),
      |r| r,
      &[StatusCode::OK, StatusCode::ACCEPTED, StatusCode::NOT_FOUND],
    ).await?;
    let play_source = match response.status() {
      StatusCode::OK => {
        let codec = response.headers().get(CONTENT_TYPE).and_then(|mime| mime.to_str().map_or(None, |str| AudioCodec::from_mime(str)));
        let data = response.bytes().await?.to_vec();
        Some(PlaySource::AudioData { codec, data })
      }
      StatusCode::ACCEPTED => Some(PlaySource::ExternallyPlayedOnSpotify),
      StatusCode::NOT_FOUND => None,
      _ => unreachable!()
    };
    Ok(play_source)
  }

  // User

  type UserError = HttpRequestError;

  async fn list_users(&self) -> Result<Vec<User>, Self::UserError> {
    let response = self.get_simple("user").await?;
    Ok(response.json().await?)
  }

  async fn get_my_user(&self) -> Result<User, Self::UserError> {
    let response = self.get_simple("user/me").await?;
    Ok(response.json().await?)
  }

  async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, Self::UserError> {
    let response = self.get_simple(format!("user/{}", id)).await?;
    Ok(response.json().await?)
  }

  async fn create_user(&self, new_user: &NewUser) -> Result<User, Self::UserError> {
    let response = self.post_simple_with_json("user", new_user).await?;
    Ok(response.json().await?)
  }

  async fn delete_user_by_name(&self, name: &String) -> Result<(), Self::UserError> {
    self.delete_simple_with_json("user", name).await?;
    Ok(())
  }

  async fn delete_user_by_id(&self, id: i32) -> Result<(), Self::UserError> {
    self.delete_simple(format!("user/{}", id)).await?;
    Ok(())
  }

  // User data

  type UserDataError = HttpRequestError;

  async fn set_user_album_rating(&self, album_id: i32, rating: i32) -> Result<UserAlbumRating, Self::UserDataError> {
    let response = self.put_simple(format!("user/data/album/{}/rating/{}", album_id, rating)).await?;
    Ok(response.json().await?)
  }

  async fn set_user_track_rating(&self, track_id: i32, rating: i32) -> Result<UserTrackRating, Self::UserDataError> {
    let response = self.put_simple(format!("user/data/track/{}/rating/{}", track_id, rating)).await?;
    Ok(response.json().await?)
  }

  async fn set_user_artist_rating(&self, artist_id: i32, rating: i32) -> Result<UserArtistRating, Self::UserDataError> {
    let response = self.put_simple(format!("user/data/artist/{}/rating/{}", artist_id, rating)).await?;
    Ok(response.json().await?)
  }

  // Sync

  type SyncError = HttpRequestError;

  async fn get_sync_status(&self) -> Result<SyncStatus, Self::SyncError> {
    let response = self.get_simple("sync").await?;
    Ok(response.json().await?)
  }

  async fn sync_all_sources(&self) -> Result<SyncStatus, Self::SyncError> {
    let response = self.post_simple("sync").await?;
    Ok(response.json().await?)
  }

  async fn sync_local_sources(&self) -> Result<SyncStatus, Self::SyncError> {
    let response = self.post_simple("sync/local").await?;
    Ok(response.json().await?)
  }

  async fn sync_local_source(&self, local_source_id: i32) -> Result<SyncStatus, Self::SyncError> {
    let response = self.post_simple(format!("sync/local/{}", local_source_id)).await?;
    Ok(response.json().await?)
  }

  async fn sync_spotify_sources(&self) -> Result<SyncStatus, Self::SyncError> {
    let response = self.post_simple("sync/spotify").await?;
    Ok(response.json().await?)
  }

  async fn sync_spotify_source(&self, spotify_source_id: i32) -> Result<SyncStatus, Self::SyncError> {
    let response = self.post_simple(format!("sync/spotify/{}", spotify_source_id)).await?;
    Ok(response.json().await?)
  }
}

// Internals

#[allow(dead_code)]
impl HttpClient {
  async fn request(
    &self,
    method: Method,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    use HttpRequestError::*;
    let url = self.url.join(url_suffix.as_ref())?;
    let response = f_request(self.client.request(method, url)).send().await?;
    match response.status() {
      c @ StatusCode::INTERNAL_SERVER_ERROR => {
        let json: Result<InternalServerError, _> = response.json().await;
        return Err(if let Ok(internal_server_error) = json {
          InternalServerFail(internal_server_error, Backtrace::capture())
        } else {
          UnexpectedStatusCode(c, Backtrace::capture())
        });
      }
      c if !expected_status_codes.as_ref().contains(&c) => {
        return Err(UnexpectedStatusCode(c, Backtrace::capture()));
      }
      _ => {}
    }
    Ok(response)
  }

  async fn request_simple(
    &self,
    method: Method,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.request(method, url_suffix, |r| r, &[StatusCode::OK]).await
  }

  async fn request_simple_with_json(
    &self,
    method: Method,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.request(method, url_suffix, |r| r.json(json), &[StatusCode::OK]).await
  }

  async fn get(
    &self,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    self.request(Method::GET, url_suffix, f_request, expected_status_codes).await
  }

  async fn get_simple(
    &self,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.request_simple(Method::GET, url_suffix).await
  }

  async fn get_simple_with_json(
    &self,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.request_simple_with_json(Method::GET, url_suffix, json).await
  }


  async fn post(
    &self,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    self.request(Method::POST, url_suffix, f_request, expected_status_codes).await
  }

  async fn post_simple(
    &self,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.request_simple(Method::POST, url_suffix).await
  }

  async fn post_simple_with_json(
    &self,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.request_simple_with_json(Method::POST, url_suffix, json).await
  }


  async fn put(
    &self,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    self.request(Method::PUT, url_suffix, f_request, expected_status_codes).await
  }

  async fn put_simple(
    &self,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.request_simple(Method::PUT, url_suffix).await
  }

  async fn put_simple_with_json(
    &self,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.request_simple_with_json(Method::PUT, url_suffix, json).await
  }


  async fn delete(
    &self,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    self.request(Method::DELETE, url_suffix, f_request, expected_status_codes).await
  }

  async fn delete_simple(
    &self,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.request_simple(Method::DELETE, url_suffix).await
  }

  async fn delete_simple_with_json(
    &self,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.request_simple_with_json(Method::DELETE, url_suffix, json).await
  }
}

impl Debug for HttpClient {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("HttpClient")
      .field("url", &self.url)
      .finish()
  }
}
