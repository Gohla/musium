use std::future::Future;
use std::pin::Pin;

use actix_identity::Identity;
use actix_web::{FromRequest, HttpRequest, HttpResponse, ResponseError, web};
use actix_web::dev::{Payload, PayloadStream};
use actix_web::error::BlockingError;
use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use musium_backend::database::{Database, DatabaseConnectError, user::UserAddVerifyError};
use musium_core::model::{User, UserLogin};

// Handlers

#[derive(Debug, Error)]
pub enum LoginError {
  #[error(transparent)]
  BackendConnectFail(#[from] DatabaseConnectError),
  #[error(transparent)]
  UserVerifyFail(#[from] UserAddVerifyError),
  #[error("Thread pool is gone")]
  ThreadPoolGoneFail,
  #[error(transparent)]
  SerializeIdentityFail(#[from] serde_json::Error),
}

impl ResponseError for LoginError {
  fn status_code(&self) -> StatusCode {
    match self {
      LoginError::UserVerifyFail(_) => StatusCode::UNAUTHORIZED,
      _ => StatusCode::INTERNAL_SERVER_ERROR
    }
  }
}

pub async fn login(user_login: web::Json<UserLogin>, identity: Identity, database: web::Data<Database>) -> Result<HttpResponse, LoginError> {
  use LoginError::*;

  let user: Result<Option<User>, BlockingError<LoginError>> = web::block(move || {
    let backend_connected = database.connect()?;
    Ok(backend_connected.verify_user(&*user_login)?)
  }).await;

  match user {
    Err(BlockingError::Error(e)) => {
      Err(e)
    }
    Err(BlockingError::Canceled) => {
      Err(ThreadPoolGoneFail)
    }
    Ok(Some(user)) => {
      identity.remember(serde_json::to_string(&LoggedInUser { user: user.clone() })?);
      Ok(HttpResponse::Ok().json(&user))
    }
    Ok(None) => {
      Ok(HttpResponse::Unauthorized().finish())
    }
  }
}

pub async fn logout(identity: Identity) -> HttpResponse {
  identity.forget();
  HttpResponse::Ok().finish()
}

// Logged-in user wrapper, required for FromRequest implementation.

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggedInUser {
  pub user: User,
}

#[derive(Debug, Error)]
pub enum LoggedInUserExtractError {
  #[error(transparent)]
  JsonSerDeIdentityFail(#[from] serde_json::Error),
  #[error(transparent)]
  IdentityExtractFail(#[from] actix_web::Error),
  #[error("Not logged in")]
  NotLoggedInFail,
}

impl ResponseError for LoggedInUserExtractError {
  fn status_code(&self) -> StatusCode {
    match self {
      LoggedInUserExtractError::NotLoggedInFail => StatusCode::UNAUTHORIZED,
      _ => StatusCode::INTERNAL_SERVER_ERROR
    }
  }
}

impl FromRequest for LoggedInUser {
  type Error = LoggedInUserExtractError;
  type Future = Pin<Box<dyn Future<Output=Result<LoggedInUser, LoggedInUserExtractError>>>>;
  type Config = ();

  fn from_request(req: &HttpRequest, payload: &mut Payload<PayloadStream>) -> Self::Future {
    use LoggedInUserExtractError::*;
    let identity = Identity::from_request(req, payload);
    Box::pin(async move {
      if let Some(serialized_identity) = identity.await?.identity() {
        let logged_in_user = serde_json::from_str(&serialized_identity)?;
        Ok(logged_in_user)
      } else {
        Err(NotLoggedInFail)
      }
    })
  }
}
