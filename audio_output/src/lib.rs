use std::error::Error;

use async_trait::async_trait;

#[async_trait]
pub trait AudioOutput: Send + Sync {
  type PlayError: 'static + Error + Send + Sync;
  async fn set_audio_data(&self, audio_data: Vec<u8>, play: bool) -> Result<(), Self::PlayError>;

  type OtherError: 'static + Error + Send + Sync;

  async fn is_playing(&self) -> Result<bool, Self::OtherError>;
  async fn play(&self) -> Result<(), Self::OtherError>;

  async fn is_paused(&self) -> Result<bool, Self::OtherError>;
  async fn pause(&self) -> Result<(), Self::OtherError>;

  async fn toggle_play(&self) -> Result<bool, Self::OtherError>;

  async fn is_stopped(&self) -> Result<bool, Self::OtherError>;
  async fn stop(&self) -> Result<(), Self::OtherError>;

  async fn get_volume(&self) -> Result<f32, Self::OtherError>;
  async fn set_volume(&self, volume: f32) -> Result<(), Self::OtherError>;
}
