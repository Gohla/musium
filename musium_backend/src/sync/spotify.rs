use std::collections::HashMap;

use reqwest::{Client, IntoUrl, StatusCode, Url};
use serde::Deserialize;
use thiserror::Error;

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

// Authorization callback

#[derive(Deserialize, Debug)]
pub struct AuthorizationInfo {
  pub access_token: String,
  pub token_type: String,
  pub scope: String,
  pub expires_in: i32,
  pub refresh_token: String,
}

#[derive(Debug, Error)]
pub enum AuthorizationError {
  #[error(transparent)]
  UrlJoinFail(#[from] url::ParseError),
  #[error(transparent)]
  HttpRequestFail(#[from] reqwest::Error),
  #[error("Invalid response {0:?} from the server")]
  InvalidResponse(StatusCode),
}

impl SpotifySync {
  pub async fn authorization_callback(
    &self,
    code: impl Into<String>,
    redirect_uri: impl Into<String>,
    _state: Option<impl Into<String>>, // TODO: verify
  ) -> Result<AuthorizationInfo, AuthorizationError> {
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
    Ok(request
      .send()
      .await?
      .error_for_status()?
      .json().await?
    )
  }
}
