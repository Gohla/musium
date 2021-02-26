use std::error::Error;

use musium_core::{
  api::SpotifyMeInfo,
  model::{
    Artist,
    collection::{
      Albums,
      Tracks,
    },
    LocalAlbum,
    LocalSource,
    LocalTrack,
    NewLocalSource,
    NewUser,
    User,
    UserAlbumRating,
    UserArtistRating,
    UserLogin,
    UserTrackRating,
  },
};

use async_trait::async_trait;

#[derive(Clone, Debug)]
pub enum PlaySource {
  AudioData(Vec<u8>),
  ExternallyPlayed,
}

#[async_trait]
pub trait Client: Send + Sync {
  type LoginError: Error;
  async fn login(&self, user_login: &UserLogin) -> Result<User, Self::LoginError>;

  type LocalSourceError: Error;
  async fn list_local_sources(&self) -> Result<Vec<LocalSource>, Self::LocalSourceError>;
  async fn get_local_source_by_id(&self, id: i32) -> Result<Option<LocalSource>, Self::LocalSourceError>;
  async fn create_or_enable_local_source(&self, new_local_source: &NewLocalSource) -> Result<LocalSource, Self::LocalSourceError>;
  async fn set_local_source_enabled_by_id(&self, id: i32, enabled: bool) -> Result<Option<LocalSource>, Self::LocalSourceError>;

  type SpotifySourceError: Error;
  async fn create_spotify_source_authorization_url(&self) -> Result<String, Self::SpotifySourceError>;
  async fn show_spotify_me(&self) -> Result<SpotifyMeInfo, Self::SpotifySourceError>;

  type AlbumError: Error;
  async fn list_albums(&self) -> Result<Albums, Self::AlbumError>;
  async fn get_album_by_id(&self, id: i32) -> Result<Option<LocalAlbum>, Self::AlbumError>;

  type TrackError: 'static + Error + Send + Sync;
  async fn list_tracks(&self) -> Result<Tracks, Self::TrackError>;
  async fn get_track_by_id(&self, id: i32) -> Result<Option<LocalTrack>, Self::TrackError>;
  async fn play_track_by_id(&self, id: i32) -> Result<Option<PlaySource>, Self::TrackError>;

  type ArtistError: Error;
  async fn list_artists(&self) -> Result<Vec<Artist>, Self::ArtistError>;
  async fn get_artist_by_id(&self, id: i32) -> Result<Option<Artist>, Self::ArtistError>;

  type UserError: Error;
  async fn list_users(&self) -> Result<Vec<User>, Self::UserError>;
  async fn get_my_user(&self) -> Result<User, Self::UserError>;
  async fn get_user_by_id(&self, id: i32) -> Result<Option<User>, Self::UserError>;
  async fn create_user(&self, new_user: &NewUser) -> Result<User, Self::UserError>;
  async fn delete_user_by_name(&self, name: &String) -> Result<(), Self::UserError>;
  async fn delete_user_by_id(&self, id: i32) -> Result<(), Self::UserError>;

  type UserDataError: Error;
  async fn set_user_album_rating(&self, album_id: i32, rating: i32) -> Result<UserAlbumRating, Self::UserDataError>;
  async fn set_user_track_rating(&self, track_id: i32, rating: i32) -> Result<UserTrackRating, Self::UserDataError>;
  async fn set_user_artist_rating(&self, artist_id: i32, rating: i32) -> Result<UserArtistRating, Self::UserDataError>;

  type SyncError: Error;
  async fn sync(&self) -> Result<bool, Self::SyncError>;
}
