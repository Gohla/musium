use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::error::Error as StdError;

#[derive(Serialize, Deserialize, Debug, Error)]
#[error("Internal server error")]
pub struct InternalServerError {
  pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyMeInfo {
  pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SyncStatus {
  Idle,
  Busy(f32),
  Failed,
}
