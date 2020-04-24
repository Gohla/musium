use reqwest::{Client, IntoUrl, Url, StatusCode};
use thiserror::Error;

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
  HttpClientCreateFail(#[from] reqwest::Error)
}

impl SpotifySync {
  pub fn new<U1: IntoUrl, U2: IntoUrl>(
    http_client: Client,
    accounts_api_base_url: U1,
    api_base_url: U2,
    client_id: String,
    client_secret: String,
  ) -> Self {
    let accounts_api_base_url = accounts_api_base_url.into();
    let api_base_url = api_base_url.into();
    Self {
      http_client,
      accounts_api_base_url,
      api_base_url,
      client_id,
      client_secret,
    }
  }

  pub fn new_from_client_id_secret(
    client_id: String,
    client_secret: String,
  ) -> Result<Self, SpotifySyncCreateError> {
    let http_client = Client::builder().build()?;
    let accounts_api_base_url = "https://accounts.spotify.com/api/";
    let api_base_url = "https://api.spotify.com/v1/";
    Ok(Self::new(http_client, accounts_api_base_url, api_base_url, client_id, client_secret))
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

impl SpotifySync {
  pub fn request_authorization(&self) -> Result<(), SpotifySyncRequestFail> {
    let url = self.accounts_api_base_url.join("authorize")?;
    let response = self.http_client.get(url)?
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