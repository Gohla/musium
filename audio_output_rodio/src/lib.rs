use std::io::Cursor;

use rodio::{OutputStream, OutputStreamHandle};
use thiserror::Error;

pub use musium_audio_output::AudioOutput;

pub struct RodioAudioOutput {
  _stream: OutputStream,
  stream_handle: OutputStreamHandle,
}

#[derive(Debug, Error)]
pub enum RodioCreateError {
  #[error("No default local audio output device was found")]
  NoDefaultOutputDevice,
}

impl RodioAudioOutput {
  pub fn new() -> Result<Self, RodioCreateError> {
    let (stream, stream_handle) = rodio::OutputStream::try_default().map_err(|_| RodioCreateError::NoDefaultOutputDevice)?;
    Ok(Self { _stream: stream, stream_handle })
  }
}

#[derive(Debug, Error)]
pub enum RodioPlayError {
  #[error(transparent)]
  ReadFail(#[from] std::io::Error),
  #[error(transparent)]
  PlayFail(#[from] rodio::PlayError),
}

impl AudioOutput for RodioAudioOutput {
  type PlayError = RodioPlayError;
  fn play(&self, audio_data: Vec<u8>, volume: f32) -> Result<(), Self::PlayError> {
    let cursor = Cursor::new(audio_data);
    let sink = self.stream_handle.play_once(cursor)?;
    sink.set_volume(volume);
    sink.sleep_until_end();
    Ok(())
  }
}
