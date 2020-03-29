use std::io::Read;

use reqwest::blocking::{Client as HttpClient, Response};
use reqwest::StatusCode;
pub use reqwest::Url;
use thiserror::Error;

use musium_core::model::*;

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

// Scan directory

impl Client {
  pub fn list_scan_directories(&self) -> Result<Vec<ScanDirectory>, ClientError> {
    let scan_directories = self.client.get(self.url.join("scan_directory")?)
      .send()?
      .json()?;
    Ok(scan_directories)
  }

  pub fn get_scan_directory_by_id(&self, id: i32) -> Result<Option<ScanDirectory>, ClientError> {
    let response = self.client.get(self.url.join(&format!("scan_directory/{}", id))?)
      .send()?;
    match response.status() {
      StatusCode::OK => Ok(Some(response.json()?)),
      _ => Ok(None)
    }
  }

  pub fn create_scan_directory(&self, new_scan_directory: &NewScanDirectory) -> Result<ScanDirectory, ClientError> {
    let scan_directory = self.client.post(self.url.join("scan_directory")?)
      .json(new_scan_directory)
      .send()?
      .json()?;
    Ok(scan_directory)
  }

  pub fn delete_scan_directory_by_directory(&self, directory: &String) -> Result<(), ClientError> {
    self.client.delete(self.url.join("scan_directory")?)
      .json(directory)
      .send()?
      .error_for_status()?;
    Ok(())
  }

  pub fn delete_scan_directory_by_id(&self, id: i32) -> Result<(), ClientError> {
    self.client.delete(self.url.join(&format!("scan_directory/{}", id))?)
      .send()?
      .error_for_status()?;
    Ok(())
  }
}

// Album

impl Client {
  pub fn list_albums(&self) -> Result<Albums, ClientError> {
    let scan_directories = self.client.get(self.url.join("album")?)
      .send()?
      .json()?;
    Ok(scan_directories)
  }

  pub fn get_album_by_id(&self, id: i32) -> Result<Option<Album>, ClientError> {
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
    let scan_directories = self.client.get(self.url.join("track")?)
      .send()?
      .json()?;
    Ok(scan_directories)
  }

  pub fn get_track_by_id(&self, id: i32) -> Result<Option<Track>, ClientError> {
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
  pub fn list_artists(&self) -> Result<Vec<Artist>, ClientError> {
    let scan_directories = self.client.get(self.url.join("artist")?)
      .send()?
      .json()?;
    Ok(scan_directories)
  }

  pub fn get_artist_by_id(&self, id: i32) -> Result<Option<Artist>, ClientError> {
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

// Scan

impl Client {
  pub fn scan(&self) -> Result<bool, ClientError> {
    use ClientError::*;
    let response: Response = self.client.get(self.url.join("scan")?)
      .send()?;
    match response.status() {
      StatusCode::ACCEPTED => Ok(true),
      StatusCode::OK => Ok(false),
      c => Err(InvalidResponse(c))
    }
  }
}
