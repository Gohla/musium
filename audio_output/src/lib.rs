use std::error::Error;

pub trait AudioOutput {
  type PlayError: Error;

  fn play(&self, audio_data: Vec<u8>, volume: f32) -> Result<(), Self::PlayError>;
}
