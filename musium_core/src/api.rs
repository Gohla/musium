use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyMeInfo {
  pub display_name: String,
}
