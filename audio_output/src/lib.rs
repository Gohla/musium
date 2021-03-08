use std::error::Error;

use async_trait::async_trait;

#[async_trait]
pub trait AudioOutput: Send + Sync {
  type PlayError: 'static + Error + Send + Sync;
  async fn set_audio_data(&self, audio_data: Vec<u8>, play: bool) -> Result<(), Self::PlayError>;

  async fn is_playing(&self) -> bool;
  async fn play(&self);

  async fn is_paused(&self) -> bool;
  async fn pause(&self);

  async fn is_stopped(&self) -> bool;
  async fn stop(&self);

  async fn get_volume(&self) -> f32;
  async fn set_volume(&self, volume: f32);
}
