use std::fmt::Debug;

use async_trait::async_trait;

use musium_core::api::AudioCodec;
use musium_core::error::SyncError;

#[async_trait]
pub trait AudioOutput: 'static + Send + Sync + Clone + Debug {
  type SetAudioDataError: SyncError;
  async fn set_audio_data(&self, codec: Option<AudioCodec>, data: Vec<u8>) -> Result<(), Self::SetAudioDataError>;

  type IsPlayingError: SyncError;
  async fn is_playing(&self) -> Result<bool, Self::IsPlayingError>;
  type PlayError: SyncError;
  async fn play(&self) -> Result<(), Self::PlayError>;

  type IsPausedError: SyncError;
  async fn is_paused(&self) -> Result<bool, Self::IsPausedError>;
  type PauseError: SyncError;
  async fn pause(&self) -> Result<(), Self::PauseError>;

  type TogglePlayError: SyncError;
  async fn toggle_play(&self) -> Result<bool, Self::TogglePlayError>;

  type IsStoppedError: SyncError;
  async fn is_stopped(&self) -> Result<bool, Self::IsStoppedError>;
  type StopError: SyncError;
  async fn stop(&self) -> Result<(), Self::StopError>;


  type GetDurationError: SyncError;
  async fn get_duration(&self) -> Result<Option<f64>, Self::GetDurationError>;
  type GetPositionError: SyncError;
  async fn get_position(&self) -> Result<Option<f64>, Self::GetPositionError>;
  type SeekToError: SyncError;
  async fn seek_to(&self, position: f64) -> Result<(), Self::SeekToError>;


  type GetVolumeError: SyncError;
  async fn get_volume(&self) -> Result<f64, Self::GetVolumeError>;
  type SetVolumeError: SyncError;
  async fn set_volume(&self, volume: f64) -> Result<(), Self::SetVolumeError>;
}
