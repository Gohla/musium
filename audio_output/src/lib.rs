use std::error::Error;
use std::path::Path;

use async_trait::async_trait;

#[async_trait]
pub trait AudioOutput: Send + Sync {
  type SetAudioDataError: 'static + Error + Send + Sync;
  async fn set_audio_data(&self, codec: AudioCodec, data: Vec<u8>) -> Result<(), Self::SetAudioDataError>;

  type IsPlayingError: 'static + Error + Send + Sync;
  async fn is_playing(&self) -> Result<bool, Self::IsPlayingError>;
  type PlayError: 'static + Error + Send + Sync;
  async fn play(&self) -> Result<(), Self::PlayError>;

  type IsPausedError: 'static + Error + Send + Sync;
  async fn is_paused(&self) -> Result<bool, Self::IsPausedError>;
  type PauseError: 'static + Error + Send + Sync;
  async fn pause(&self) -> Result<(), Self::PauseError>;

  type TogglePlayError: 'static + Error + Send + Sync;
  async fn toggle_play(&self) -> Result<bool, Self::TogglePlayError>;

  type IsStoppedError: 'static + Error + Send + Sync;
  async fn is_stopped(&self) -> Result<bool, Self::IsStoppedError>;
  type StopError: 'static + Error + Send + Sync;
  async fn stop(&self) -> Result<(), Self::StopError>;

  type GetVolumeError: 'static + Error + Send + Sync;
  async fn get_volume(&self) -> Result<f64, Self::GetVolumeError>;
  type SetVolumeError: 'static + Error + Send + Sync;
  async fn set_volume(&self, volume: f64) -> Result<(), Self::SetVolumeError>;
}



impl AudioCodec {
  pub fn from_path(path: impl AsRef<Path>) -> Option<AudioCodec> {
    if let Some(extension) = path.as_ref().extension() {
      use AudioCodec::*;
      match extension.to_string_lossy().as_ref() {
        "mp3" => Some(Mp3),
        "ogg" => Some(Ogg),
        "flac" => Some(Flac),
        "wav" => Some(Wav),
        _ => None,
      }
    } else {
      None
    }
  }
}
