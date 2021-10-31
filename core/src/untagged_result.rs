use std::ops::{ControlFlow, FromResidual, Try};

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
  type Output = T;
  type Residual = E;

  #[inline]
  fn from_output(v: T) -> Self {
    UResult::Ok(v)
  }

  #[inline]
  fn branch(self) -> ControlFlow<E, T> {
    match self {
      UResult::Ok(t) => ControlFlow::Continue(t),
      UResult::Err(e) => ControlFlow::Break(e),
    }
  }
}

impl<T, E> FromResidual for UResult<T, E> {
  #[inline]
  fn from_residual(v: E) -> Self {
    UResult::Err(v)
  }
}
