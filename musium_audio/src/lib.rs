use std::io::{Cursor, Read};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlayError {
  #[error(transparent)]
  ReadFail(#[from] std::io::Error),
  #[error("No default audio output device")]
  NoDefaultOutputDevice,
  #[error(transparent)]
  PlayFail(#[from] rodio::decoder::DecoderError),
}

pub fn play<R: Read>(mut reader: R, volume: f32) -> Result<(), PlayError> {
  use PlayError::*;
  let cursor = {
    // Copy to in-memory buffer and return a cursor, as Rodio requires a Seek implementation.
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Cursor::new(buffer)
  };
  let device = rodio::default_output_device().ok_or(NoDefaultOutputDevice)?;
  let sink = rodio::play_once(&device, cursor)?;
  sink.set_volume(volume);
  sink.sleep_until_end();
  Ok(())
}