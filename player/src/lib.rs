mod worker_task;

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

  async fn is_paused(&self) -> Result<bool, <Self::AudioOutput as AudioOutput>::IsPausedError>;
  async fn pause(&self) -> Result<(), <Self::AudioOutput as AudioOutput>::PauseError>;
  async fn toggle_play(&self) -> Result<bool, <Self::AudioOutput as AudioOutput>::TogglePlayError>;
  async fn is_stopped(&self) -> Result<bool, <Self::AudioOutput as AudioOutput>::IsStoppedError>;
  async fn stop(&self) -> Result<(), <Self::AudioOutput as AudioOutput>::StopError>;
  async fn get_position_relative(&self) -> Result<Option<f64>, <Self::AudioOutput as AudioOutput>::GetPositionRelativeError>;
  async fn seek_to_relative(&self, position_relative: f64) -> Result<(), <Self::AudioOutput as AudioOutput>::SeekToRelativeError>;
  async fn get_volume(&self) -> Result<f64, <Self::AudioOutput as AudioOutput>::GetVolumeError>;
  async fn set_volume(&self, volume: f64) -> Result<(), <Self::AudioOutput as AudioOutput>::SetVolumeError>;
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


  type LoginError = C::LoginError;
  async fn login(&self, user_login: &UserLogin) -> Result<User, Self::LoginError> {
    self.get_client().login(user_login).await
  }


  type PlayError = PlayError<C::PlaybackError, AO::SetAudioDataError, AO::PlayError>;
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


  async fn is_paused(&self) -> Result<bool, AO::IsPausedError> {
    self.get_audio_output().is_paused().await
  }

  async fn pause(&self) -> Result<(), AO::PauseError> {
    self.get_audio_output().pause().await
  }

  async fn toggle_play(&self) -> Result<bool, AO::TogglePlayError> {
    self.get_audio_output().toggle_play().await
  }

  async fn is_stopped(&self) -> Result<bool, AO::IsStoppedError> {
    self.get_audio_output().is_stopped().await
  }

  async fn stop(&self) -> Result<(), AO::StopError> {
    self.get_audio_output().stop().await
  }

  async fn get_position_relative(&self) -> Result<Option<f64>, AO::GetPositionRelativeError> {
    self.get_audio_output().get_position_relative().await
  }

  async fn seek_to_relative(&self, position_relative: f64) -> Result<(), AO::SeekToRelativeError> {
    self.get_audio_output().seek_to_relative(position_relative).await
  }

  async fn get_volume(&self) -> Result<f64, AO::GetVolumeError> {
    self.get_audio_output().get_volume().await
  }

  async fn set_volume(&self, volume: f64) -> Result<(), AO::SetVolumeError> {
    self.get_audio_output().set_volume(volume).await
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
