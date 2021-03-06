use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde::__private::Formatter;
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Error)]
#[error("Internal server error")]
pub struct InternalServerError {
  pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyMeInfo {
  pub display_name: String,
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
