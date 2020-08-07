use std::io::Cursor;

use rodio::Device;
use thiserror::Error;

pub struct Player {
  device: Device,
}

#[derive(Debug, Error)]
pub enum CreateError {
  #[error("No default audio output device")]
  NoDefaultOutputDevice,
}

impl Player {
  pub fn new() -> Result<Self, CreateError> {
    let device = rodio::default_output_device().ok_or(CreateError::NoDefaultOutputDevice)?;
    Ok(Self { device })
  }
}

#[derive(Debug, Error)]
pub enum PlayError {
  #[error(transparent)]
  ReadFail(#[from] std::io::Error),
  #[error(transparent)]
  PlayFail(#[from] rodio::decoder::DecoderError),
}

impl Player {
  pub fn play(&self, audio_data: Vec<u8>, volume: f32) -> Result<(), PlayError> {
    let cursor = Cursor::new(audio_data);
    let sink = rodio::play_once(&self.device, cursor)?;
    sink.set_volume(volume);
    sink.sleep_until_end();
    Ok(())
  }
}
