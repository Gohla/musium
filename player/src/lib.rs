use std::fmt::Debug;

use async_trait::async_trait;
use thiserror::Error;

pub use musium_audio_output::AudioOutput;
#[cfg(feature = "default_player")]
pub use musium_audio_output_kira::KiraAudioOutput;
pub use musium_client::Client;
#[cfg(feature = "default_player")]
pub use musium_client_http::{HttpClient, HttpRequestError, Url};
use musium_core::error::SyncError;
use musium_core::model::{User, UserLogin};

// Player trait

#[async_trait]
pub trait Player: 'static + Send + Sync + Clone + Debug {
  type Client: Client;
  type AudioOutput: AudioOutput;
  fn get_client(&self) -> &Self::Client;
  fn get_client_mut(&mut self) -> &mut Self::Client;
  fn get_audio_output(&self) -> &Self::AudioOutput;
  fn get_audio_output_mut(&mut self) -> &mut Self::AudioOutput;
  /// Converts this player into its client and audio output, effectively destroying the player. The client and audio
  /// output can then be manually destroyed.
  ///
  /// Dropping this player and all its clones will also properly destroy it, but ignores the panics produced by the
  /// client and audio output (if any), and also does not wait for them to complete first (if they have tasks or threads
  /// that must be completed).
  fn into_client_and_audio_output(self) -> (Self::Client, Self::AudioOutput);


  type LoginError: SyncError;
  async fn login(&self, user_login: &UserLogin) -> Result<User, Self::LoginError>;

  type PlayError: SyncError;
  async fn play_track_by_id(&self, id: i32) -> Result<(), Self::PlayError>;
}

#[derive(Debug, Error)]
pub enum PlayError<CP, AOS, AOP> {
  #[error("Failed to get playback data from the client")]
  ClientPlayTrackFail(#[source] CP),
  #[error("Failed to set audio data to the audio output")]
  SetAudioDataFail(#[source] AOS),
  #[error("Failed to play audio with the audio output")]
  AudioOutputPlayFail(#[source] AOP),
}

// Generic player type

#[derive(Clone, Debug)]
pub struct GenericPlayer<C, AO> {
  client: C,
  audio_output: AO,
}

impl<C: Client, AO: AudioOutput> GenericPlayer<C, AO> {
  pub fn new(client: C, audio_output: AO) -> Self {
    Self {
      client,
      audio_output,
    }
  }
}

#[async_trait]
impl<C: Client, AO: AudioOutput> Player for GenericPlayer<C, AO> {
  type Client = C;
  type AudioOutput = AO;
  #[inline]
  fn get_client(&self) -> &Self::Client { &self.client }
  #[inline]
  fn get_client_mut(&mut self) -> &mut Self::Client { &mut self.client }
  #[inline]
  fn get_audio_output(&self) -> &Self::AudioOutput { &self.audio_output }
  #[inline]
  fn get_audio_output_mut(&mut self) -> &mut Self::AudioOutput { &mut self.audio_output }
  #[inline]
  fn into_client_and_audio_output(self) -> (Self::Client, Self::AudioOutput) { (self.client, self.audio_output) }


  type LoginError = <Self::Client as Client>::LoginError;
  async fn login(&self, user_login: &UserLogin) -> Result<User, Self::LoginError> {
    self.get_client().login(user_login).await
  }

  type PlayError = PlayError<<Self::Client as Client>::PlaybackError, <Self::AudioOutput as AudioOutput>::SetAudioDataError, <Self::AudioOutput as AudioOutput>::PlayError>;
  async fn play_track_by_id(&self, id: i32) -> Result<(), Self::PlayError> {
    use PlayError::*;
    use musium_core::api::PlaySource::*;
    let play_source = self.get_client().play_track_by_id(id).await.map_err(|e| ClientPlayTrackFail(e))?;
    match play_source {
      Some(AudioData { codec, data }) => self.get_audio_output().set_audio_data(codec, data).await.map_err(|e| SetAudioDataFail(e))?,
      Some(ExternallyPlayedOnSpotify) => {}
      None => {}
    };
    self.get_audio_output().play().await.map_err(|e| AudioOutputPlayFail(e))?;
    Ok(())
  }
}

// Default player

#[cfg(feature = "default_player")]
pub type DefaultPlayer = GenericPlayer<HttpClient, KiraAudioOutput>;

#[cfg(feature = "default_player")]
#[derive(Debug, Error)]
pub enum CreateError {
  #[error("Failed to create HTTP client")]
  ClientCreateFail(#[from] musium_client_http::HttpClientCreateError),
  #[error("Failed to create Kira audio output")]
  AudioOutputCreateFail(#[from] musium_audio_output_kira::KiraCreateError),
}

#[cfg(feature = "default_player")]
pub fn create_default_player(url: Url) -> Result<DefaultPlayer, CreateError> {
  Ok(DefaultPlayer::new(musium_client_http::HttpClient::new(url)?, musium_audio_output_kira::KiraAudioOutput::new()?))
}
