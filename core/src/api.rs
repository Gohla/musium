use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Error)]
#[error("Internal server error")]
pub struct InternalServerError {
  pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AudioCodec {
  Mp3,
  Ogg,
  Flac,
  Wav,
  Other(String),
  Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlaySource {
  AudioData { codec: AudioCodec, data: Vec<u8> },
  ExternallyPlayedOnSpotify,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PlaySourceKind {
  AudioData,
  ExternalOnSpotify,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyMeInfo {
  pub display_name: String,
}

