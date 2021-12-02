use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::path::Path;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[error("Internal server error")]
pub struct InternalServerError {
  pub message: String,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub enum AudioCodec {
  Mp3,
  Ogg,
  Flac,
  Wav,
}

impl AudioCodec {
  pub fn from_path(path: impl AsRef<Path>) -> Option<AudioCodec> {
    if let Some(extension) = path.as_ref().extension() {
      Self::from_extension(extension)
    } else {
      None
    }
  }

  pub fn from_extension(extension: &OsStr) -> Option<AudioCodec> {
    use AudioCodec::*;
    match extension.to_string_lossy().as_ref() {
      "mp3" => Some(Mp3),
      "ogg" => Some(Ogg),
      "flac" => Some(Flac),
      "wav" => Some(Wav),
      _ => None,
    }
  }

  pub fn from_mime(mime: &str) -> Option<AudioCodec> {
    use AudioCodec::*;
    match mime {
      "audio/mpeg" => Some(Mp3),
      "audio/ogg" => Some(Ogg),
      "audio/flac" => Some(Flac),
      "audio/wav" => Some(Wav),
      _ => None,
    }
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub enum PlaySource {
  AudioData { codec: Option<AudioCodec>, data: Vec<u8> },
  ExternallyPlayedOnSpotify,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug)]
pub enum PlaySourceKind {
  AudioData,
  ExternalOnSpotify,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug)]
pub enum SyncStatus {
  Idle,
  Started(Option<f32>),
  Busy(Option<f32>),
  Completed,
  Failed,
}

impl Display for SyncStatus {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
    match self {
      SyncStatus::Idle => f.write_str("idle"),
      SyncStatus::Started(progress) => {
        f.write_str("started")?;
        if let Some(progress) = progress {
          f.write_str(&format!(" {0:.1}", progress * 100f32))?;
        }
        Ok(())
      }
      SyncStatus::Busy(progress) => {
        f.write_str("busy")?;
        if let Some(progress) = progress {
          f.write_str(&format!(" {0:.1}", progress * 100f32))?;
        }
        Ok(())
      }
      SyncStatus::Completed => f.write_str("completed"),
      SyncStatus::Failed => f.write_str("failed"),
    }
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct SpotifyMeInfo {
  pub display_name: String,
}

