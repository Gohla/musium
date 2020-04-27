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
pub enum SpotifySyncCreateError {
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
  ) -> Result<Self, SpotifySyncCreateError> {
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
  ) -> Result<Self, SpotifySyncCreateError> {
    let http_client = Client::builder().build()?;
    let accounts_api_base_url = "https://accounts.spotify.com/api/";
    let api_base_url = "https://api.spotify.com/v1/";
    Self::new(http_client, accounts_api_base_url, api_base_url, client_id, client_secret)
  }
}

#[derive(Debug, Error)]
pub enum SpotifySyncRequestFail {
  #[error(transparent)]
  UrlJoinFail(#[from] url::ParseError),
  #[error(transparent)]
  HttpRequestFail(#[from] reqwest::Error),
  #[error("Invalid response {0:?} from the server")]
  InvalidResponse(StatusCode),
}

// Authorization

#[derive(Deserialize, Debug)]
pub struct SpotifyAuthorizationInfo {
  pub access_token: String,
  pub token_type: String,
  pub scope: String,
  pub expires_in: i32,
  pub refresh_token: String,
}

impl SpotifySync {
  pub fn create_authorization_url<S1: Into<String>, S2: Into<String>>(
    &self,
    redirect_uri: S1,
    state: Option<S2>,
  ) -> Result<Url, SpotifySyncRequestFail> {
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
    };
    let request = self.http_client
      .get(url)
      .query(&query_map)
      ;
    Ok(request.build()?.url().clone())
  }

  pub async fn authorization_callback<S1: Into<String>>(
    &self,
    code: String,
    state: Option<String>, // TODO: verify
    redirect_uri: S1,
  ) -> Result<SpotifyAuthorizationInfo, SpotifySyncRequestFail> {
    let url = self.accounts_api_base_url.join("token")?;
    let request = self.http_client
      .post(url)
      .form(&{
        let mut map = HashMap::new();
        map.insert("grant_type", "authorization_code".to_owned());
        map.insert("code", code);
        map.insert("redirect_uri", redirect_uri.into())
      })
      .basic_auth(&self.client_id, Some(&self.client_secret))
      ;
    let response = request.send().await?;
    Ok(response.json().await?)
  }
}

// impl SpotifySync {
//   pub fn request_authorization(&self, )
// }

// #[derive(Clone, Debug)]
// pub struct SpotifySyncTrack {
//   pub source_id: i32,
//   pub title: String,
// }
//
// impl SpotifySync {
//   pub fn sync(source_id: i32, data: &SpotifySourceData) -> impl Iterator<Item=SpotifySyncTrack> {
//
//   }
// }
