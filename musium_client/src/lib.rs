use std::io::Read;

use reqwest::blocking::{Client as HttpClient, Response};
use reqwest::header::ToStrError;
use reqwest::StatusCode;
use reqwest::redirect;
pub use reqwest::Url;
use thiserror::Error;

use musium_core::model::*;
use musium_core::model::collection::{Albums, AlbumsRaw, Tracks, TracksRaw};

pub struct Client {
  client: HttpClient,
  url: Url,
}

// Creation

#[derive(Debug, Error)]
pub enum ClientCreateError {
  #[error(transparent)]
  HttpClientCreateFail(#[from] reqwest::Error)
}

impl Client {
  pub fn new(url: Url) -> Result<Self, ClientCreateError> {
    let client: HttpClient = HttpClient::builder()
      .cookie_store(true)
      .redirect(redirect::Policy::none())
      .build()?;
    Ok(Self { client, url })
  }
}

// Generic error type for client failures

#[derive(Debug, Error)]
pub enum ClientError {
  #[error(transparent)]
  UrlJoinFail(#[from] url::ParseError),
  #[error(transparent)]
  HttpRequestFail(#[from] reqwest::Error),
  #[error(transparent)]
  HeaderValueToStringFail(#[from] ToStrError),
  #[error("Invalid response {0:?} from the server")]
  InvalidResponse(StatusCode),
}

// Login

impl Client {
  pub fn login(&self, user_login: &UserLogin) -> Result<User, ClientError> {
    let url = self.url.join("login")?;
    let user = self.client.post(url)
      .json(user_login)
      .send()?
      .json()?;
    Ok(user)
  }
}

// Local source

impl Client {
  pub fn list_local_sources(&self) -> Result<Vec<LocalSource>, ClientError> {
    let local_sources = self.client.get(self.url.join("source/local")?)
      .send()?
      .json()?;
    Ok(local_sources)
  }

  pub fn get_local_source_by_id(&self, id: i32) -> Result<Option<LocalSource>, ClientError> {
    let response = self.client.get(self.url.join(&format!("source/local/{}", id))?)
      .send()?;
    match response.status() {
      StatusCode::OK => Ok(Some(response.json()?)),
      _ => Ok(None)
    }
  }

  pub fn create_or_enable_local_source(&self, new_local_source: &NewLocalSource) -> Result<LocalSource, ClientError> {
    let local_source = self.client.post(self.url.join("source/local")?)
      .json(new_local_source)
      .send()?
      .json()?;
    Ok(local_source)
  }

  pub fn set_local_source_enabled_by_id(&self, id: i32, enabled: bool) -> Result<Option<LocalSource>, ClientError> {
    let local_source = self.client.post(self.url.join(&format!("source/local/set_enabled/{}", id))?)
      .json(&enabled)
      .send()?
      .json()?;
    Ok(local_source)
  }
}

// Spotify source

impl Client {
  pub fn create_spotify_source_authorization_url(&self) -> Result<String, ClientError> {
    use ClientError::*;
    let response = self.client.get(self.url.join("source/spotify/request_authorization")?)
      .send()?
      .error_for_status()?;
    if let Some(url) = response.headers().get(reqwest::header::LOCATION) {
      Ok(url.to_str()?.to_owned())
    } else {
      Err(InvalidResponse(response.status()))
    }
  }
}

// Album

impl Client {
  pub fn list_albums(&self) -> Result<Albums, ClientError> {
    let albums_raw: AlbumsRaw = self.client.get(self.url.join("album")?)
      .send()?
      .json()?;
    Ok(albums_raw.into())
  }

  pub fn get_album_by_id(&self, id: i32) -> Result<Option<LocalAlbum>, ClientError> {
    let response = self.client.get(self.url.join(&format!("album/{}", id))?)
      .send()?;
    match response.status() {
      StatusCode::OK => Ok(Some(response.json()?)),
      _ => Ok(None)
    }
  }
}

// Track

impl Client {
  pub fn list_tracks(&self) -> Result<Tracks, ClientError> {
    let tracks_raw: TracksRaw = self.client.get(self.url.join("track")?)
      .send()?
      .json()?;
    Ok(tracks_raw.into())
  }

  pub fn get_track_by_id(&self, id: i32) -> Result<Option<LocalTrack>, ClientError> {
    let response = self.client.get(self.url.join(&format!("track/{}", id))?)
      .send()?;
    match response.status() {
      StatusCode::OK => Ok(Some(response.json()?)),
      _ => Ok(None)
    }
  }

  pub fn download_track_by_id(&self, id: i32) -> Result<Option<impl Read>, ClientError> {
    let response = self.client.get(self.url.join(&format!("track/download/{}", id))?)
      .send()?;
    match response.status() {
      StatusCode::OK => Ok(Some(response)),
      _ => Ok(None)
    }
  }
}

// Artist

impl Client {
  pub fn list_artists(&self) -> Result<Vec<LocalArtist>, ClientError> {
    let scan_directories = self.client.get(self.url.join("artist")?)
      .send()?
      .json()?;
    Ok(scan_directories)
  }

  pub fn get_artist_by_id(&self, id: i32) -> Result<Option<LocalArtist>, ClientError> {
    let response = self.client.get(self.url.join(&format!("artist/{}", id))?)
      .send()?;
    match response.status() {
      StatusCode::OK => Ok(Some(response.json()?)),
      _ => Ok(None)
    }
  }
}

// User

impl Client {
  pub fn list_users(&self) -> Result<Vec<User>, ClientError> {
    let users = self.client.get(self.url.join("user")?)
      .send()?
      .json()?;
    Ok(users)
  }

  pub fn get_my_user(&self) -> Result<User, ClientError> {
    let user = self.client.get(self.url.join("user/me")?)
      .send()?
      .json()?;
    Ok(user)
  }

  pub fn get_user_by_id(&self, id: i32) -> Result<Option<User>, ClientError> {
    let response = self.client.get(self.url.join(&format!("user/{}", id))?)
      .send()?;
    match response.status() {
      StatusCode::OK => Ok(Some(response.json()?)),
      _ => Ok(None)
    }
  }

  pub fn create_user(&self, new_user: &NewUser) -> Result<User, ClientError> {
    let user = self.client.post(self.url.join("user")?)
      .json(new_user)
      .send()?
      .json()?;
    Ok(user)
  }

  pub fn delete_user_by_name(&self, name: &String) -> Result<(), ClientError> {
    self.client.delete(self.url.join("user")?)
      .json(name)
      .send()?
      .error_for_status()?;
    Ok(())
  }

  pub fn delete_user_by_id(&self, id: i32) -> Result<(), ClientError> {
    self.client.delete(self.url.join(&format!("user/{}", id))?)
      .send()?
      .error_for_status()?;
    Ok(())
  }
}

// User data

impl Client {
  pub fn set_user_album_rating(&self, album_id: i32, rating: i32) -> Result<UserAlbumRating, ClientError> {
    let rating = self.client.put(self.url.join(&format!("user/data/album/{}/rating/{}", album_id, rating))?)
      .send()?
      .json()?;
    Ok(rating)
  }

  pub fn set_user_track_rating(&self, track_id: i32, rating: i32) -> Result<UserTrackRating, ClientError> {
    let rating = self.client.put(self.url.join(&format!("user/data/album/{}/rating/{}", track_id, rating))?)
      .send()?
      .json()?;
    Ok(rating)
  }

  pub fn set_user_artist_rating(&self, artist_id: i32, rating: i32) -> Result<UserArtistRating, ClientError> {
    let rating = self.client.put(self.url.join(&format!("user/data/album/{}/rating/{}", artist_id, rating))?)
      .send()?
      .json()?;
    Ok(rating)
  }
}

// Sync

impl Client {
  pub fn sync(&self) -> Result<bool, ClientError> {
    use ClientError::*;
    let response: Response = self.client.get(self.url.join("sync")?)
      .send()?;
    match response.status() {
      StatusCode::ACCEPTED => Ok(true),
      StatusCode::OK => Ok(false),
      c => Err(InvalidResponse(c))
    }
  }
}
