use std::ops::Try;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[must_use = "this `Result` may be an `Err` variant, which should be handled"]
#[serde(untagged)]
pub enum UResult<T, E> {
  Ok(T),
  Err(E),
}

impl<T, E> Into<Result<T, E>> for UResult<T, E> {
  fn into(self) -> Result<T, E> {
    match self {
      UResult::Ok(t) => Result::Ok(t),
      UResult::Err(e) => Result::Err(e),
    }
  }
}

impl<T, E> Try for UResult<T, E> {
  type Ok = T;
  type Error = E;

  #[inline]
  fn into_result(self) -> Result<T, E> {
    self.into()
  }

  #[inline]
  fn from_error(v: E) -> Self {
    UResult::Err(v)
  }

  #[inline]
  fn from_ok(v: T) -> Self {
    UResult::Ok(v)
  }
}
