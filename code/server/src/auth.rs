use serde::{Deserialize};

#[derive(Debug, Deserialize)]
pub struct AuthData {
  pub name: String,
  pub password: String,
}
