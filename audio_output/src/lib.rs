use std::error::Error;

use async_trait::async_trait;

#[async_trait]
pub trait AudioOutput {
  type PlayError: Error;

  async fn play(&self, audio_data: Vec<u8>, volume: f32) -> Result<(), Self::PlayError>;
}
