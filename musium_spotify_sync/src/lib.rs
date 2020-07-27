#![feature(backtrace)]

use std::backtrace::Backtrace;
use std::collections::HashMap;

use chrono::{Duration, NaiveDateTime, Utc};
use itertools::Itertools;
use reqwest::{Client, IntoUrl, RequestBuilder, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{event, instrument, Level};

#[derive(Clone)]
pub struct SpotifySync {
  http_client: Client,
  accounts_api_base_url: Url,
  api_base_url: Url,
  client_id: String,
  client_secret: String,
}

// Creation

#[derive(Debug, Error)]
pub enum CreateError {
  #[error(transparent)]
  UrlCreateFail(#[from] url::ParseError),
  #[error(transparent)]
  HttpClientCreateFail(#[from] reqwest::Error),
}

impl SpotifySync {
  pub fn new<U1: IntoUrl, U2: IntoUrl>(
    http_client: Client,
    accounts_api_base_url: U1,
    api_base_url: U2,
    client_id: String,
    client_secret: String,
  ) -> Result<Self, CreateError> {
    let accounts_api_base_url = accounts_api_base_url.into_url()?;
    let api_base_url = api_base_url.into_url()?;
    Ok(Self {
      http_client,
      accounts_api_base_url,
      api_base_url,
      client_id,
      client_secret,
    })
  }

  pub fn new_from_client_id_secret(
    client_id: String,
    client_secret: String,
  ) -> Result<Self, CreateError> {
    let http_client = Client::builder().build()?;
    let accounts_api_base_url = "https://accounts.spotify.com/";
    let api_base_url = "https://api.spotify.com/v1/";
    Self::new(http_client, accounts_api_base_url, api_base_url, client_id, client_secret)
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

impl SpotifySync {
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

// API errors

#[derive(Debug, Error)]
pub enum ApiError {
  #[error("Failed to join URLs")]
  UrlJoinFail(#[from] url::ParseError, Backtrace),
  #[error("HTTP request failed")]
  HttpRequestFail(#[from] reqwest::Error, Backtrace),
  #[error("Invalid response {0:?} from the server")]
  InvalidResponseFail(StatusCode, Backtrace),
  #[error("Request was met with a 401 Unauthorized response, but a retry with a new access token was not possible due to the request builder not being cloneable")]
  UnauthorizedAndCannotCloneRequestBuilderFail,
}

// Authorization callback

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct Authorization {
  pub access_token: String,
  pub expiry_date: NaiveDateTime,
  pub refresh_token: String,
}

impl SpotifySync {
  pub async fn authorization_callback(
    &self,
    code: impl Into<String>,
    redirect_uri: impl Into<String>,
    _state: Option<impl Into<String>>, // TODO: verify
  ) -> Result<Authorization, ApiError> {
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

impl SpotifySync {
  #[instrument(level = "trace", skip(self, refresh_token), err)]
  async fn refresh_access_token(&self, refresh_token: impl Into<String>) -> Result<RefreshInfo, ApiError> {
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

impl SpotifySync {
  #[instrument(level = "trace", skip(self, authorization), err)]
  async fn update_authorization_info(&self, authorization: &mut Authorization) -> Result<String, ApiError> {
    let refresh_info = self.refresh_access_token(authorization.refresh_token.clone()).await?;
    event!(Level::DEBUG, ?refresh_info, "Updating Spotify authorization with new access token");
    authorization.access_token = refresh_info.access_token.clone();
    authorization.expiry_date = (Utc::now() + Duration::seconds(refresh_info.expires_in as i64)).naive_utc();
    Ok(authorization.access_token.clone())
  }

  #[instrument(level = "trace", skip(self, authorization), err)]
  async fn update_authorization_info_if_needed(&self, authorization: &mut Authorization) -> Result<String, ApiError> {
    if Utc::now().naive_utc() >= authorization.expiry_date {
      self.update_authorization_info(authorization).await
    } else {
      Ok(authorization.access_token.clone())
    }
  }

  #[instrument(level = "trace", skip(self, request_builder, authorization), err)]
  async fn send_request_with_access_token(&self, request_builder: RequestBuilder, authorization: &mut Authorization) -> Result<Response, ApiError> {
    let access_token = self.update_authorization_info_if_needed(authorization).await?;
    let request_builder = request_builder.bearer_auth(access_token);
    let request_builder_clone = request_builder.try_clone();
    let response = request_builder.send().await?;
    if response.status() == StatusCode::UNAUTHORIZED {
      match request_builder_clone {
        Some(request_builder_clone) => {
          // When the request was unauthorized, request a new access token and retry once.
          event!(Level::TRACE, ?request_builder_clone, "Request was met with 401 Unauthorized response; retrying with new access token");
          let access_token = self.update_authorization_info_if_needed(authorization).await?;
          Ok(request_builder_clone.bearer_auth(access_token).send().await?)
        }
        None => Err(ApiError::UnauthorizedAndCannotCloneRequestBuilderFail) // If request cannot be cloned, we cannot retry when unauthorized.
      }
    } else {
      Ok(response)
    }
  }
}

// Me info

#[derive(Serialize, Deserialize, Debug)]
pub struct MeInfo {
  pub display_name: String,
}

impl SpotifySync {
  pub async fn me(&self, authorization: &mut Authorization) -> Result<MeInfo, ApiError> {
    let url = self.api_base_url.join("me")?;
    let request = self.http_client.get(url);
    Ok(self.send_request_with_access_token(request, authorization).await?.error_for_status()?.json().await?)
  }
}

// Sync

impl SpotifySync {
  #[instrument(level = "trace", skip(self, authorization), err)]
  pub async fn get_albums_of_followed_artists(&self, authorization: &mut Authorization) -> Result<impl Iterator<Item=Album>, ApiError> {
    let mut all_albums = Vec::new();
    let followed_artist = self.get_followed_artist(authorization).await?;
    for artist in followed_artist {
      let artist_albums_simple = self.get_artist_albums_simple(artist.id, authorization).await?;
      let albums = self.get_albums(artist_albums_simple.into_iter().map(|a| a.id), authorization).await?;
      all_albums.extend(albums)
    }
    Ok(all_albums.into_iter())
  }

  #[instrument(level = "trace", skip(self, authorization), err)]
  pub async fn get_followed_artist(&self, authorization: &mut Authorization) -> Result<Vec<Artist>, ApiError> {
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

  #[instrument(level = "trace", skip(self, authorization), err)]
  async fn get_followed_artist_raw(&self, after: Option<String>, authorization: &mut Authorization) -> Result<CursorBasedPaging<Artist>, ApiError> {
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
      .send_request_with_access_token(request, authorization).await?
      .error_for_status()?
      .json().await?;
    Ok(artists.artists)
  }

  #[instrument(level = "trace", skip(self, authorization), err)]
  pub async fn get_artist_albums_simple(&self, artist_id: String, authorization: &mut Authorization) -> Result<Vec<AlbumSimple>, ApiError> {
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

  #[instrument(level = "trace", skip(self, authorization), err)]
  pub async fn get_artist_albums_simple_raw(&self, artist_id: &String, offset: usize, authorization: &mut Authorization) -> Result<Paging<AlbumSimple>, ApiError> {
    let url = self.api_base_url.join(&format!("artists/{}/albums", artist_id))?;
    let request = self.http_client
      .get(url)
      .query(&[("include_groups", "album,single"), ("country", "from_token"), ("limit", "50"), ("offset", &offset.to_string())])
      ;
    let albums: Paging<AlbumSimple> = self
      .send_request_with_access_token(request, authorization).await?
      .error_for_status()?
      .json().await?;
    Ok(albums)
  }

  #[instrument(level = "trace", skip(self, album_ids, authorization), err)]
  pub async fn get_albums(&self, album_ids: impl IntoIterator<Item=String>, authorization: &mut Authorization) -> Result<impl Iterator<Item=Album>, ApiError> {
    let url = self.api_base_url.join("albums")?;
    let mut all_albums = Vec::new();
    for mut album_ids_per_20 in &album_ids.into_iter().chunks(20) {
      let request = self.http_client
        .get(url.clone())
        .query(&[("ids", album_ids_per_20.join(","))])
        ;
      let albums: Albums = self
        .send_request_with_access_token(request, authorization).await?
        .error_for_status()?
        .json().await?;
      all_albums.extend(albums.albums)
    }
    Ok(all_albums.into_iter())
  }
}

// Paging

#[derive(Deserialize, Debug)]
pub struct Paging<T> {
  pub items: Vec<T>,
  pub offset: usize,
  pub total: usize,
}

// Cursor-based paging

#[derive(Deserialize, Debug)]
pub struct Cursor {
  pub after: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct CursorBasedPaging<T> {
  pub items: Vec<T>,
  pub cursors: Cursor,
}

// Artists

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

// Albums

#[derive(Deserialize, Debug)]
pub struct Albums {
  pub albums: Vec<Album>
}

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

// Tracks

#[derive(Deserialize, Debug)]
pub struct TrackSimple {
  pub id: String,
  pub name: String,
  pub artists: Vec<ArtistSimple>,
  pub track_number: i32,
  pub disc_number: i32,
}
