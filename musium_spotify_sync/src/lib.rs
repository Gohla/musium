#![feature(backtrace)]

use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use chrono::{Duration, NaiveDateTime, Utc};
use itertools::Itertools;
use reqwest::{Client, header, IntoUrl, RequestBuilder, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{event, instrument, Level};

#[derive(Clone)]
pub struct SpotifyClient {
  http_client: Client,
  accounts_api_base_url: Url,
  api_base_url: Url,
  client_id: String,
  client_secret: String,
  max_retries: u8,
}

// Creation

#[derive(Debug, Error)]
pub enum CreateError {
  #[error(transparent)]
  UrlCreateFail(#[from] url::ParseError),
  #[error(transparent)]
  HttpClientCreateFail(#[from] reqwest::Error),
}

impl SpotifyClient {
  pub fn new<U1: IntoUrl, U2: IntoUrl>(
    http_client: Client,
    accounts_api_base_url: U1,
    api_base_url: U2,
    client_id: String,
    client_secret: String,
    max_retries: u8,
  ) -> Result<Self, CreateError> {
    let accounts_api_base_url = accounts_api_base_url.into_url()?;
    let api_base_url = api_base_url.into_url()?;
    Ok(Self {
      http_client,
      accounts_api_base_url,
      api_base_url,
      client_id,
      client_secret,
      max_retries,
    })
  }

  pub fn new_from_client_id_secret(
    client_id: String,
    client_secret: String,
  ) -> Result<Self, CreateError> {
    let http_client = Client::builder().build()?;
    let accounts_api_base_url = "https://accounts.spotify.com/";
    let api_base_url = "https://api.spotify.com/v1/";
    let max_retries = 2;
    Self::new(http_client, accounts_api_base_url, api_base_url, client_id, client_secret, max_retries)
  }
}

// Create authorization URL

#[derive(Debug, Error)]
pub enum CreateAuthorizationUrlError {
  #[error(transparent)]
  UrlJoinFail(#[from] url::ParseError),
  #[error(transparent)]
  HttpRequestBuildFail(#[from] reqwest::Error),
}

impl SpotifyClient {
  pub fn create_authorization_url(
    &self,
    redirect_uri: impl Into<String>,
    state: Option<impl Into<String>>,
  ) -> Result<String, CreateAuthorizationUrlError> {
    let url = self.accounts_api_base_url.join("authorize")?;
    let query_map = {
      let mut map = HashMap::new();
      map.insert("client_id", self.client_id.clone());
      map.insert("response_type", "code".to_owned());
      map.insert("redirect_uri", redirect_uri.into());
      if let Some(state) = state {
        map.insert("state", state.into());
      }
      map.insert("scope", "user-read-playback-state user-modify-playback-state user-read-currently-playing user-follow-read".to_owned());
      map
    };
    let request = self.http_client
      .get(url)
      .query(&query_map)
      ;
    Ok(request.build()?.url().to_string())
  }
}

// Authorization callback

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct Authorization {
  pub access_token: String,
  pub expiry_date: NaiveDateTime,
  pub refresh_token: String,
}

impl SpotifyClient {
  pub async fn authorization_callback(
    &self,
    code: impl Into<String>,
    redirect_uri: impl Into<String>,
    _state: Option<impl Into<String>>, // TODO: verify
  ) -> Result<Authorization, HttpRequestError> {
    let url = self.accounts_api_base_url.join("api/token")?;
    let request = self.http_client
      .post(url)
      .form(&{
        let mut map = HashMap::new();
        map.insert("grant_type", "authorization_code".to_owned());
        map.insert("code", code.into());
        map.insert("redirect_uri", redirect_uri.into());
        map
      })
      .basic_auth(&self.client_id, Some(&self.client_secret))
      ;
    #[derive(Deserialize)]
    struct AuthorizationInfo {
      pub access_token: String,
      pub token_type: String,
      pub scope: String,
      pub expires_in: i32,
      pub refresh_token: String,
    }
    let authorization_info: AuthorizationInfo = request.send().await?.error_for_status()?.json().await?;
    Ok(Authorization {
      access_token: authorization_info.access_token,
      expiry_date: (Utc::now() + Duration::seconds(authorization_info.expires_in as i64)).naive_utc(),
      refresh_token: authorization_info.refresh_token,
    })
  }
}

// Refresh access token

#[derive(Deserialize, Debug)]
pub struct RefreshInfo {
  pub access_token: String,
  pub token_type: String,
  pub scope: String,
  pub expires_in: i32,
}

impl SpotifyClient {
  #[instrument(level = "trace", skip(self, refresh_token))]
  async fn refresh_access_token(&self, refresh_token: impl Into<String>) -> Result<RefreshInfo, HttpRequestError> {
    let url = self.accounts_api_base_url.join("api/token")?;
    let request = self.http_client
      .post(url)
      .form(&{
        let mut map = HashMap::new();
        map.insert("grant_type", "refresh_token".to_owned());
        map.insert("refresh_token", refresh_token.into());
        map
      })
      .basic_auth(&self.client_id, Some(&self.client_secret))
      ;
    Ok(request.send().await?.error_for_status()?.json().await?)
  }
}

// Keeping authorization info up-to-date

impl SpotifyClient {
  #[instrument(level = "trace", skip(self, authorization))]
  async fn update_authorization_info(&self, authorization: &mut Authorization) -> Result<String, HttpRequestError> {
    let refresh_info = self.refresh_access_token(authorization.refresh_token.clone()).await?;
    event!(Level::DEBUG, ?refresh_info, "Updating Spotify authorization with new access token");
    authorization.access_token = refresh_info.access_token.clone();
    authorization.expiry_date = (Utc::now() + Duration::seconds(refresh_info.expires_in as i64)).naive_utc();
    Ok(authorization.access_token.clone())
  }

  #[instrument(level = "trace", skip(self, authorization))]
  async fn update_authorization_info_if_needed(&self, authorization: &mut Authorization) -> Result<String, HttpRequestError> {
    if Utc::now().naive_utc() >= authorization.expiry_date {
      self.update_authorization_info(authorization).await
    } else {
      Ok(authorization.access_token.clone())
    }
  }
}

// Sending a request, taking care of authorization, 401 Unauthorized errors, 429 Too Many Requests errors, and retries.

#[derive(Debug, Error)]
pub enum SpotifyError {
  #[error("status code '{0}' and error message '{1}'")]
  Error(StatusCode, String),
  #[error("status code '{0}'")]
  ErrorWithoutMessage(StatusCode),
}

#[derive(Debug, Error)]
pub enum HttpRequestError {
  #[error("Failed to join URLs")]
  UrlJoinFail(#[from] url::ParseError, Backtrace),
  #[error("HTTP request failed")]
  HttpRequestFail(#[from] reqwest::Error, Backtrace),
  #[error("Server responded with {0}")]
  UnexpectedStatusCodeFail(SpotifyError),
  #[error("Server responded with {0}, even after {1} retries")]
  RetryFail(SpotifyError, u8),
  #[error("Server responded with {0}, but a retry was not possible due to the request builder not being cloneable")]
  CannotRetryFail(SpotifyError),
}

impl SpotifyClient {
  async fn send_request(&self, request_builder: RequestBuilder, authorization: &mut Authorization) -> Result<Response, HttpRequestError> {
    self.send_request_with_retry(request_builder, authorization, 0).await
  }

  #[instrument(level = "trace", skip(self, request_builder, authorization))]
  fn send_request_with_retry<'a>(&'a self, request_builder: RequestBuilder, authorization: &'a mut Authorization, retry: u8) -> Pin<Box<dyn 'a + Future<Output=Result<Response, HttpRequestError>>>> {
    use HttpRequestError::*;
    Box::pin(async move { // Pin box future because this is a recursive async method.
      let access_token = self.update_authorization_info_if_needed(authorization).await?;
      let request_builder = request_builder.bearer_auth(access_token);
      let request_builder_clone = request_builder.try_clone();
      let response = request_builder.send().await?;
      match response.status() {
        StatusCode::UNAUTHORIZED => {
          let error = Self::response_to_error(response).await;
          if retry >= self.max_retries {
            return Err(RetryFail(error, retry));
          }

          // When the request was unauthorized, request a new access token and then retry.
          event!(Level::TRACE, ?request_builder_clone, "Server responded with {}; retrying with new access token", error);
          let access_token = self.update_authorization_info_if_needed(authorization).await?;
          let request_builder = request_builder_clone.ok_or(CannotRetryFail(error))?.bearer_auth(access_token);
          Ok(self.send_request_with_retry(request_builder, authorization, retry + 1).await?)
        }
        StatusCode::TOO_MANY_REQUESTS => {
          let default_duration = tokio::time::Duration::from_secs(5);
          let retry_after = if let Some(retry_after) = response.headers().get(header::RETRY_AFTER) {
            if let Ok(retry_after) = retry_after.to_str() {
              if let Ok(retry_after_seconds) = retry_after.parse::<u32>() {
                tokio::time::Duration::from_secs((retry_after_seconds + 1 + retry as u32) as u64)
              } else {
                default_duration
              }
            } else {
              default_duration
            }
          } else {
            default_duration
          };

          let error = Self::response_to_error(response).await;
          if retry >= self.max_retries {
            return Err(RetryFail(error, retry));
          }

          // When the request was rate limited, delay for some time and then retry.
          event!(Level::TRACE, ?request_builder_clone, "Server responded with {}; retrying after {:?}", error, retry_after);
          tokio::time::delay_for(retry_after).await;
          let request_builder = request_builder_clone.ok_or(CannotRetryFail(error))?;
          Ok(self.send_request_with_retry(request_builder, authorization, retry + 1).await?)
        }
        StatusCode::BAD_REQUEST |
        StatusCode::FORBIDDEN |
        StatusCode::NOT_FOUND |
        StatusCode::INTERNAL_SERVER_ERROR |
        StatusCode::BAD_GATEWAY |
        StatusCode::SERVICE_UNAVAILABLE => {
          let error = Self::response_to_error(response).await;
          Err(UnexpectedStatusCodeFail(error))
        }
        _ => Ok(response)
      }
    })
  }

  async fn response_to_error(response: Response) -> SpotifyError {
    #[derive(Deserialize)]
    struct RegularError {
      error: Error
    }
    #[derive(Deserialize)]
    struct Error {
      message: String
    }
    let status_code = response.status();
    let regular_error: Option<RegularError> = response.json().await.ok();
    if let Some(regular_error) = regular_error {
      SpotifyError::Error(status_code, regular_error.error.message)
    } else {
      SpotifyError::ErrorWithoutMessage(status_code)
    }
  }
}

// Me info

#[derive(Serialize, Deserialize, Debug)]
pub struct MeInfo {
  pub display_name: String,
}

impl SpotifyClient {
  pub async fn me(&self, authorization: &mut Authorization) -> Result<MeInfo, HttpRequestError> {
    let url = self.api_base_url.join("me")?;
    let request = self.http_client.get(url);
    Ok(self.send_request(request, authorization).await?.error_for_status()?.json().await?)
  }
}

// Paging

#[derive(Deserialize, Debug)]
pub struct Paging<T> {
  pub items: Vec<T>,
  pub offset: usize,
  pub total: usize,
}

// Artist

#[derive(Deserialize, Debug)]
pub struct Artist {
  pub id: String,
  pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ArtistSimple {
  pub id: String,
  pub name: String,
}

impl SpotifyClient {
  #[instrument(level = "trace", skip(self, authorization))]
  pub async fn get_followed_artists(&self, authorization: &mut Authorization) -> Result<Vec<Artist>, HttpRequestError> {
    let mut all_artists = Vec::new();
    let mut after = None;
    loop {
      let artists = self.get_followed_artist_raw(after, authorization).await?;
      all_artists.extend(artists.items);
      after = artists.cursors.after;
      if after.is_none() { break; }
    }
    Ok(all_artists)
  }

  #[instrument(level = "trace", skip(self, authorization))]
  async fn get_followed_artist_raw(&self, after: Option<String>, authorization: &mut Authorization) -> Result<CursorBasedPaging<Artist>, HttpRequestError> {
    let url = self.api_base_url.join("me/following")?;
    let mut request = self.http_client
      .get(url)
      .query(&[("type", "artist"), ("limit", "50")])
      ;
    if let Some(after) = after {
      request = request.query(&[("after", after)]);
    }
    #[derive(Deserialize, Debug)]
    struct CursorBasedPagingArtists {
      pub artists: CursorBasedPaging<Artist>,
    }
    let artists: CursorBasedPagingArtists = self
      .send_request(request, authorization).await?
      .error_for_status()?
      .json().await?;
    Ok(artists.artists)
  }
}

// Album

#[derive(Deserialize, Debug)]
pub struct Album {
  pub id: String,
  pub name: String,
  pub artists: Vec<ArtistSimple>,
  pub tracks: Paging<TrackSimple>,
}

#[derive(Deserialize, Debug)]
pub struct AlbumSimple {
  pub id: String,
  pub name: String,
  pub artists: Vec<ArtistSimple>,
}

impl SpotifyClient {
  #[instrument(level = "trace", skip(self, authorization))]
  pub async fn get_albums_of_followed_artists(&self, authorization: &mut Authorization) -> Result<impl Iterator<Item=Album>, HttpRequestError> {
    let mut all_albums = Vec::new();
    let followed_artist = self.get_followed_artists(authorization).await?;
    for artist in followed_artist {
      let artist_albums_simple = self.get_artist_albums_simple(artist.id, authorization).await?;
      let albums = self.get_albums(artist_albums_simple.into_iter().map(|a| a.id), authorization).await?;
      all_albums.extend(albums)
    }
    Ok(all_albums.into_iter())
  }

  #[instrument(level = "trace", skip(self, authorization))]
  pub async fn get_artist_albums_simple(&self, artist_id: String, authorization: &mut Authorization) -> Result<Vec<AlbumSimple>, HttpRequestError> {
    let mut all_albums = Vec::new();
    let mut offset = 0;
    loop {
      let albums = self.get_artist_albums_simple_raw(&artist_id, offset, authorization).await?;
      let len = albums.items.len();
      all_albums.extend(albums.items);
      offset += len;
      if offset >= albums.total { break; }
    }
    Ok(all_albums)
  }

  #[instrument(level = "trace", skip(self, authorization))]
  async fn get_artist_albums_simple_raw(&self, artist_id: &String, offset: usize, authorization: &mut Authorization) -> Result<Paging<AlbumSimple>, HttpRequestError> {
    let url = self.api_base_url.join(&format!("artists/{}/albums", artist_id))?;
    let request = self.http_client
      .get(url)
      .query(&[("include_groups", "album,single"), ("country", "from_token"), ("limit", "50"), ("offset", &offset.to_string())])
      ;
    let albums: Paging<AlbumSimple> = self
      .send_request(request, authorization).await?
      .error_for_status()?
      .json().await?;
    Ok(albums)
  }

  #[instrument(level = "trace", skip(self, album_ids, authorization))]
  pub async fn get_albums(&self, album_ids: impl IntoIterator<Item=String>, authorization: &mut Authorization) -> Result<Vec<Album>, HttpRequestError> {
    let url = self.api_base_url.join("albums")?;
    let mut all_albums = Vec::new();
    for mut album_ids_per_20 in &album_ids.into_iter().chunks(20) {
      let request = self.http_client
        .get(url.clone())
        .query(&[("ids", album_ids_per_20.join(","))])
        ;
      #[derive(Deserialize, Debug)]
      struct Albums {
        pub albums: Vec<Album>
      }
      let albums: Albums = self
        .send_request(request, authorization).await?
        .error_for_status()?
        .json().await?;
      all_albums.extend(albums.albums)
    }
    Ok(all_albums)
  }
}

// Track

#[derive(Deserialize, Debug)]
pub struct TrackSimple {
  pub id: String,
  pub name: String,
  pub artists: Vec<ArtistSimple>,
  pub track_number: i32,
  pub disc_number: i32,
}

// Player

impl SpotifyClient {
  #[instrument(level = "trace", skip(self, authorization))]
  pub async fn play_track(&self, track_id: &String, authorization: &mut Authorization) -> Result<(), HttpRequestError> {
    let url = self.api_base_url.join("me/player/play")?;
    #[derive(Serialize, Debug)]
    struct Body {
      uris: Vec<String>,
    }
    let body = Body { uris: vec![format!("spotify:track:{}", track_id)] };
    let request = self.http_client
      .put(url)
      .json(&body)
      ;
    self.send_request(request, authorization).await?.error_for_status()?;
    Ok(())
  }
}

// Cursor-based paging

#[derive(Deserialize, Debug)]
struct Cursor {
  pub after: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CursorBasedPaging<T> {
  pub items: Vec<T>,
  pub cursors: Cursor,
}
