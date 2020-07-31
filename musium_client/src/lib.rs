use reqwest::{
  Client as HttpClient,
  header::ToStrError,
  redirect,
  Response,
  StatusCode,
};
pub use reqwest::Url;
use thiserror::Error;

use musium_core::{
  api::{InternalServerError, SpotifyMeInfo},
  model::{
    *,
    collection::{Albums, AlbumsRaw, Tracks, TracksRaw},
  },
  untagged_result::UResult,
};

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
  InternalServerFail(#[from] InternalServerError),
  #[error(transparent)]
  HeaderValueToStringFail(#[from] ToStrError),
  #[error("Unexpected response {0:?} from the server")]
  UnexpectedResponseFail(StatusCode),
}

// Login

impl Client {
  pub async fn login(&self, user_login: &UserLogin) -> Result<User, ClientError> {
    let url = self.url.join("login")?;
    let user = self.client.post(url)
      .json(user_login)
      .send().await?
      .json().await?;
    Ok(user)
  }
}

// Local source

impl Client {
  pub async fn list_local_sources(&self) -> Result<Vec<LocalSource>, ClientError> {
    let local_sources: UResult<_, InternalServerError> = self.client.get(self.url.join("source/local")?)
      .send().await?
      .json().await?;
    Ok(local_sources?)
  }

  pub async fn get_local_source_by_id(&self, id: i32) -> Result<Option<LocalSource>, ClientError> {
    let local_source = self.client.get(self.url.join(&format!("source/local/{}", id))?)
      .send().await?
      .json().await?;
    Ok(local_source)
  }

  pub async fn create_or_enable_local_source(&self, new_local_source: &NewLocalSource) -> Result<LocalSource, ClientError> {
    let local_source = self.client.post(self.url.join("source/local")?)
      .json(new_local_source)
      .send().await?
      .json().await?;
    Ok(local_source)
  }

  pub async fn set_local_source_enabled_by_id(&self, id: i32, enabled: bool) -> Result<Option<LocalSource>, ClientError> {
    let local_source = self.client.post(self.url.join(&format!("source/local/set_enabled/{}", id))?)
      .json(&enabled)
      .send().await?
      .json().await?;
    Ok(local_source)
  }
}

// Spotify source

impl Client {
  pub async fn create_spotify_source_authorization_url(&self) -> Result<String, ClientError> {
    use ClientError::*;
    let response = self.client.get(self.url.join("source/spotify/request_authorization")?)
      .send().await?
      .error_for_status()?;
    if let Some(url) = response.headers().get(reqwest::header::LOCATION) {
      Ok(url.to_str()?.to_owned())
    } else {
      Err(UnexpectedResponseFail(response.status()))
    }
  }

  pub async fn show_spotify_me(&self) -> Result<SpotifyMeInfo, ClientError> {
    let me_info: SpotifyMeInfo = self.client.get(self.url.join("source/spotify/me")?)
      .send().await?
      .json().await?;
    Ok(me_info)
  }
}

// Album

impl Client {
  pub async fn list_albums(&self) -> Result<Albums, ClientError> {
    let albums_raw: AlbumsRaw = self.client.get(self.url.join("album")?)
      .send().await?
      .json().await?;
    Ok(albums_raw.into())
  }

  pub async fn get_album_by_id(&self, id: i32) -> Result<Option<LocalAlbum>, ClientError> {
    let local_album = self.client.get(self.url.join(&format!("album/{}", id))?)
      .send().await?
      .json().await?;
    Ok(local_album)
  }
}

// Track

pub enum PlaySource {
  AudioData(Vec<u8>),
  ExternallyPlayed,
}

impl Client {
  pub async fn list_tracks(&self) -> Result<Tracks, ClientError> {
    let tracks_raw: TracksRaw = self.client.get(self.url.join("track")?)
      .send().await?
      .json().await?;
    Ok(tracks_raw.into())
  }

  pub async fn get_track_by_id(&self, id: i32) -> Result<Option<LocalTrack>, ClientError> {
    let track = self.client.get(self.url.join(&format!("track/{}", id))?)
      .send().await?
      .json().await?;
    Ok(track)
  }

  pub async fn play_track_by_id(&self, id: i32) -> Result<Option<PlaySource>, ClientError> {
    let response = self.client.get(self.url.join(&format!("track/play/{}", id))?)
      .send().await?;
    let play_source = match response.status() {
      StatusCode::OK => Some(PlaySource::AudioData(response.bytes().await?.to_vec())),
      StatusCode::ACCEPTED => Some(PlaySource::ExternallyPlayed),
      StatusCode::NOT_FOUND => None,
      c => return Err(ClientError::UnexpectedResponseFail(c)),
    };
    Ok(play_source)
  }
}

// Artist

impl Client {
  pub async fn list_artists(&self) -> Result<Vec<LocalArtist>, ClientError> {
    let scan_directories = self.client.get(self.url.join("artist")?)
      .send().await?
      .json().await?;
    Ok(scan_directories)
  }

  pub async fn get_artist_by_id(&self, id: i32) -> Result<Option<LocalArtist>, ClientError> {
    let artist = self.client.get(self.url.join(&format!("artist/{}", id))?)
      .send().await?
      .json().await?;
    Ok(artist)
  }
}

// User

impl Client {
  pub async fn list_users(&self) -> Result<Vec<User>, ClientError> {
    let users = self.client.get(self.url.join("user")?)
      .send().await?
      .json().await?;
    Ok(users)
  }

  pub async fn get_my_user(&self) -> Result<User, ClientError> {
    let user = self.client.get(self.url.join("user/me")?)
      .send().await?
      .json().await?;
    Ok(user)
  }

  pub async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, ClientError> {
    let user = self.client.get(self.url.join(&format!("user/{}", id))?)
      .send().await?
      .json().await?;
    Ok(user)
  }

  pub async fn create_user(&self, new_user: &NewUser) -> Result<User, ClientError> {
    let user = self.client.post(self.url.join("user")?)
      .json(new_user)
      .send().await?
      .json().await?;
    Ok(user)
  }

  pub async fn delete_user_by_name(&self, name: &String) -> Result<(), ClientError> {
    self.client.delete(self.url.join("user")?)
      .json(name)
      .send().await?
      .error_for_status()?;
    Ok(())
  }

  pub async fn delete_user_by_id(&self, id: i32) -> Result<(), ClientError> {
    self.client.delete(self.url.join(&format!("user/{}", id))?)
      .send().await?
      .error_for_status()?;
    Ok(())
  }
}

// User data

impl Client {
  pub async fn set_user_album_rating(&self, album_id: i32, rating: i32) -> Result<UserAlbumRating, ClientError> {
    let rating = self.client.put(self.url.join(&format!("user/data/album/{}/rating/{}", album_id, rating))?)
      .send().await?
      .json().await?;
    Ok(rating)
  }

  pub async fn set_user_track_rating(&self, track_id: i32, rating: i32) -> Result<UserTrackRating, ClientError> {
    let rating = self.client.put(self.url.join(&format!("user/data/album/{}/rating/{}", track_id, rating))?)
      .send().await?
      .json().await?;
    Ok(rating)
  }

  pub async fn set_user_artist_rating(&self, artist_id: i32, rating: i32) -> Result<UserArtistRating, ClientError> {
    let rating = self.client.put(self.url.join(&format!("user/data/album/{}/rating/{}", artist_id, rating))?)
      .send().await?
      .json().await?;
    Ok(rating)
  }
}

// Sync

impl Client {
  pub async fn sync(&self) -> Result<bool, ClientError> {
    use ClientError::*;
    let response: Response = self.client.get(self.url.join("sync")?)
      .send().await?;
    match response.status() {
      StatusCode::ACCEPTED => Ok(true),
      StatusCode::OK => Ok(false),
      c => Err(UnexpectedResponseFail(c))
    }
  }
}
