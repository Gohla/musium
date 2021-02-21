use std::io::Cursor;

use rodio::{OutputStream, OutputStreamHandle};
use thiserror::Error;

pub struct Player {
  _stream: OutputStream,
  stream_handle: OutputStreamHandle,
}

#[derive(Debug, Error)]
pub enum CreateError {
  #[error("No default audio output device")]
  NoDefaultOutputDevice,
}

impl Player {
  pub fn new() -> Result<Self, CreateError> {
    let (stream, stream_handle) = rodio::OutputStream::try_default().map_err(|_| CreateError::NoDefaultOutputDevice)?;
    Ok(Self { _stream: stream, stream_handle })
  }
}

#[derive(Debug, Error)]
pub enum PlayError {
  #[error(transparent)]
  ReadFail(#[from] std::io::Error),
  #[error(transparent)]
  PlayFail(#[from] rodio::PlayError),
}

impl Player {
  pub fn play(&self, audio_data: Vec<u8>, volume: f32) -> Result<(), PlayError> {
    let cursor = Cursor::new(audio_data);
    let sink = self.stream_handle.play_once(cursor)?;
    sink.set_volume(volume);
    sink.sleep_until_end();
    Ok(())
  }
}
