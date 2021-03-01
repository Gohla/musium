use std::error::Error;

use async_trait::async_trait;

#[async_trait]
pub trait AudioOutput: Send + Sync {
  type PlayError: 'static + Error + Send + Sync;
  async fn play(&self, audio_data: Vec<u8>, volume: f32) -> Result<(), Self::PlayError>;

  type StopError: 'static + Error + Send + Sync;
  fn stop(self) -> Result<(), Self::StopError>;
}
