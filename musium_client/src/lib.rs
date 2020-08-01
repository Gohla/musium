#![feature(backtrace)]

use reqwest::StatusCode;
use thiserror::Error;

use musium_core::{
  api::SpotifyMeInfo,
  model::{
    *,
    collection::{Albums, AlbumsRaw, Tracks, TracksRaw},
  },
};
use musium_reqwest_util::{HttpClient, HttpClientCreateError, HttpRequestError};
pub use reqwest::Url;

pub struct Client {
  http_client: HttpClient
}

// Creation

impl Client {
  pub fn new(base_url: impl reqwest::IntoUrl) -> Result<Self, HttpClientCreateError> {
    let client_builder = reqwest::Client::builder()
      .cookie_store(true)
      .redirect(reqwest::redirect::Policy::none())
      ;
    let http_client = HttpClient::from_client_builder_url(client_builder, base_url)?;
    Ok(Self { http_client })
  }
}

// Login

impl Client {
  pub async fn login(&self, user_login: &UserLogin) -> Result<User, HttpRequestError> {
    let response = self.http_client.post_simple_with_json("login", user_login).await?;
    Ok(response.json().await?)
  }
}

// Local source

impl Client {
  pub async fn list_local_sources(&self) -> Result<Vec<LocalSource>, HttpRequestError> {
    let response = self.http_client.get_simple("source/local").await?;
    Ok(response.json().await?)
  }

  pub async fn get_local_source_by_id(&self, id: i32) -> Result<Option<LocalSource>, HttpRequestError> {
    let response = self.http_client.get_simple(format!("source/local/{}", id)).await?;
    Ok(response.json().await?)
  }

  pub async fn create_or_enable_local_source(&self, new_local_source: &NewLocalSource) -> Result<LocalSource, HttpRequestError> {
    let response = self.http_client.post_simple_with_json("source/local", new_local_source).await?;
    Ok(response.json().await?)
  }

  pub async fn set_local_source_enabled_by_id(&self, id: i32, enabled: bool) -> Result<Option<LocalSource>, HttpRequestError> {
    let response = self.http_client.post_simple_with_json(format!("source/local/set_enabled/{}", id), &enabled).await?;
    Ok(response.json().await?)
  }
}

// Spotify source

#[derive(Debug, Error)]
pub enum CreateSpotifySourceAuthorizationUrlError {
  #[error(transparent)]
  HttpRequestFail(#[from] HttpRequestError),
  #[error("The LOCATION header is missing from the HTTP response")]
  LocationHeaderMissingFail,
  #[error("Failed to convert the LOCATION header in the HTTP response to a string")]
  LocationHeaderToStringFail(#[from] reqwest::header::ToStrError),
}

impl Client {
  pub async fn create_spotify_source_authorization_url(&self) -> Result<String, CreateSpotifySourceAuthorizationUrlError> {
    use CreateSpotifySourceAuthorizationUrlError::*;
    let response = self.http_client.get_simple("source/spotify/request_authorization").await?;
    if let Some(url) = response.headers().get(reqwest::header::LOCATION) {
      Ok(url.to_str()?.to_owned())
    } else {
      Err(LocationHeaderMissingFail)
    }
  }

  pub async fn show_spotify_me(&self) -> Result<SpotifyMeInfo, HttpRequestError> {
    let response = self.http_client.get_simple("source/spotify/me").await?;
    Ok(response.json().await?)
  }
}

// Album

impl Client {
  pub async fn list_albums(&self) -> Result<Albums, HttpRequestError> {
    let response = self.http_client.get_simple("album").await?;
    let albums_raw: AlbumsRaw = response.json().await?;
    Ok(albums_raw.into())
  }

  pub async fn get_album_by_id(&self, id: i32) -> Result<Option<LocalAlbum>, HttpRequestError> {
    let response = self.http_client.get_simple(format!("album/{}", id)).await?;
    Ok(response.json().await?)
  }
}

// Track

pub enum PlaySource {
  AudioData(Vec<u8>),
  ExternallyPlayed,
}

impl Client {
  pub async fn list_tracks(&self) -> Result<Tracks, HttpRequestError> {
    let response = self.http_client.get_simple("album").await?;
    let tracks_raw: TracksRaw = response.json().await?;
    Ok(tracks_raw.into())
  }

  pub async fn get_track_by_id(&self, id: i32) -> Result<Option<LocalTrack>, HttpRequestError> {
    let response = self.http_client.get_simple(format!("track/{}", id)).await?;
    Ok(response.json().await?)
  }

  pub async fn play_track_by_id(&self, id: i32) -> Result<Option<PlaySource>, HttpRequestError> {
    let response = self.http_client.get(
      format!("track/play/{}", id),
      |r| r,
      &[StatusCode::OK, StatusCode::ACCEPTED, StatusCode::NOT_FOUND],
    ).await?;
    let play_source = match response.status() {
      StatusCode::OK => Some(PlaySource::AudioData(response.bytes().await?.to_vec())),
      StatusCode::ACCEPTED => Some(PlaySource::ExternallyPlayed),
      StatusCode::NOT_FOUND => None,
      _ => unreachable!()
    };
    Ok(play_source)
  }
}

// Artist

impl Client {
  pub async fn list_artists(&self) -> Result<Vec<LocalArtist>, HttpRequestError> {
    let response = self.http_client.get_simple("artist").await?;
    Ok(response.json().await?)
  }

  pub async fn get_artist_by_id(&self, id: i32) -> Result<Option<LocalArtist>, HttpRequestError> {
    let response = self.http_client.get_simple(format!("artist/{}", id)).await?;
    Ok(response.json().await?)
  }
}

// User

impl Client {
  pub async fn list_users(&self) -> Result<Vec<User>, HttpRequestError> {
    let response = self.http_client.get_simple("user").await?;
    Ok(response.json().await?)
  }

  pub async fn get_my_user(&self) -> Result<User, HttpRequestError> {
    let response = self.http_client.get_simple("user/me").await?;
    Ok(response.json().await?)
  }

  pub async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, HttpRequestError> {
    let response = self.http_client.get_simple(format!("user/{}", id)).await?;
    Ok(response.json().await?)
  }

  pub async fn create_user(&self, new_user: &NewUser) -> Result<User, HttpRequestError> {
    let response = self.http_client.post_simple_with_json("user", new_user).await?;
    Ok(response.json().await?)
  }

  pub async fn delete_user_by_name(&self, name: &String) -> Result<(), HttpRequestError> {
    self.http_client.delete_simple_with_json("user", name).await?;
    Ok(())
  }

  pub async fn delete_user_by_id(&self, id: i32) -> Result<(), HttpRequestError> {
    self.http_client.delete_simple(format!("user/{}", id)).await?;
    Ok(())
  }
}

// User data

impl Client {
  pub async fn set_user_album_rating(&self, album_id: i32, rating: i32) -> Result<UserAlbumRating, HttpRequestError> {
    let response = self.http_client.put_simple(format!("user/data/album/{}/rating/{}", album_id, rating)).await?;
    Ok(response.json().await?)
  }

  pub async fn set_user_track_rating(&self, track_id: i32, rating: i32) -> Result<UserTrackRating, HttpRequestError> {
    let response = self.http_client.put_simple(format!("user/data/track/{}/rating/{}", track_id, rating)).await?;
    Ok(response.json().await?)
  }

  pub async fn set_user_artist_rating(&self, artist_id: i32, rating: i32) -> Result<UserArtistRating, HttpRequestError> {
    let response = self.http_client.put_simple(format!("user/data/artist/{}/rating/{}", artist_id, rating)).await?;
    Ok(response.json().await?)
  }
}

// Sync

impl Client {
  pub async fn sync(&self) -> Result<bool, HttpRequestError> {
    let response = self.http_client.get(
      "sync",
      |r| r,
      &[StatusCode::OK, StatusCode::ACCEPTED],
    ).await?;
    match response.status() {
      StatusCode::OK => Ok(false),
      StatusCode::ACCEPTED => Ok(true),
      _ => unreachable!(),
    }
  }
}
