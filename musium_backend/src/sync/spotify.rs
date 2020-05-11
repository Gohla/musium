use std::collections::HashMap;

use reqwest::{Client, IntoUrl, StatusCode, Url};
use serde::Deserialize;
use thiserror::Error;

use musium_core::api::SpotifyMeInfo;

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
  ) -> Result<Url, CreateAuthorizationUrlError> {
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
    Ok(request.build()?.url().clone())
  }
}

// API errors

#[derive(Debug, Error)]
pub enum ApiError {
  #[error(transparent)]
  UrlJoinFail(#[from] url::ParseError),
  #[error(transparent)]
  HttpRequestFail(#[from] reqwest::Error),
  #[error("Invalid response {0:?} from the server")]
  InvalidResponse(StatusCode),
}

// Authorization callback

#[derive(Deserialize, Debug)]
pub struct AuthorizationInfo {
  pub access_token: String,
  pub token_type: String,
  pub scope: String,
  pub expires_in: i32,
  pub refresh_token: String,
}

impl SpotifySync {
  pub async fn authorization_callback(
    &self,
    code: impl Into<String>,
    redirect_uri: impl Into<String>,
    _state: Option<impl Into<String>>, // TODO: verify
  ) -> Result<AuthorizationInfo, ApiError> {
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
    Ok(request.send().await?.error_for_status()?.json().await?)
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
  pub async fn refresh_access_token(&self, refresh_token: impl Into<String>) -> Result<RefreshInfo, ApiError> {
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

// Me info

impl SpotifySync {
  pub async fn me(&self, access_token: impl Into<String>) -> Result<SpotifyMeInfo, ApiError> {
    let url = self.api_base_url.join("me")?;
    let request = self.http_client
      .get(url)
      .bearer_auth(access_token.into())
      ;
    Ok(request.send().await?.error_for_status()?.json().await?)
  }
}

// Sync

pub struct SpotifySyncTrack {}

impl SpotifySync {
  pub async fn sync(&self, access_token: impl Into<String>) -> Result<impl Iterator<Item=SpotifySyncTrack>, ApiError> {
    let access_token = access_token.into();
    let _followed_artist_ids = self.get_followed_artist_ids(&access_token).await?;
    Ok(std::iter::empty())
  }

  async fn get_followed_artist_ids(&self, access_token: impl Into<String>) -> Result<impl Iterator<Item=String>, ApiError> {
    let url = self.api_base_url.join("me/following")?;
    let request = self.http_client
      .get(url)
      .query(&[("type", "artist"), ("limit", "1")])
      .bearer_auth(access_token.into())
      ;
    let artists: CursorPageArtists = request.send().await?.error_for_status()?.json().await?;
    Ok(artists.artists.items.into_iter().map(|a| a.id))
  }
}

#[derive(Deserialize, Debug)]
struct CursorBasedPage<T> {
  items: Vec<T>,
}

#[derive(Deserialize, Debug)]
struct CursorPageArtists {
  artists: CursorBasedPage<Artist>,
}

#[derive(Deserialize, Debug)]
struct Artist {
  id: String
}