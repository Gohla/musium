use std::backtrace::Backtrace;
use std::future::Future;
use std::pin::Pin;

use actix_identity::Identity;
use actix_web::{FromRequest, HttpRequest, HttpResponse, ResponseError, web};
use actix_web::dev::{Payload, PayloadStream};
use actix_web::error::BlockingError;
use actix_web::http::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{event, Level};

use musium_backend::database::{Database, DatabaseConnectError, user::UserAddVerifyError};
use musium_core::api::InternalServerError;
use musium_core::format_error::FormatError;
use musium_core::model::{User, UserLogin};

// Logged-in user

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggedInUser {
  pub user: User,
}

// Login

#[derive(Debug, Error)]
pub enum InternalLoginError {
  #[error("Failed to connect to the database")]
  BackendConnectFail(#[from] DatabaseConnectError, Backtrace),
  #[error("Failed to verify user")]
  UserVerifyFail(#[from] UserAddVerifyError, Backtrace),
  #[error("Blocking thread pool is gone")]
  BlockingThreadPoolGoneFail,
  #[error("Failed to serialize identity")]
  SerializeIdentityFail(#[from] serde_json::Error),
}

impl ResponseError for InternalLoginError {
  fn status_code(&self) -> StatusCode {
    match self {
      _ => StatusCode::INTERNAL_SERVER_ERROR
    }
  }

  fn error_response(&self) -> HttpResponse {
    let format_error = FormatError::new(self);
    event!(Level::ERROR, "{:?}", format_error);
    HttpResponse::build(self.status_code()).json(InternalServerError {
      message: self.to_string()
    })
  }
}

pub async fn login(user_login: web::Json<UserLogin>, identity: Identity, database: web::Data<Database>) -> Result<HttpResponse, InternalLoginError> {
  use InternalLoginError::*;

  let result: Result<Result<Option<User>, InternalLoginError>, BlockingError> = web::block(move || {
    let backend_connected = database.connect()?;
    Ok(backend_connected.verify_user(&*user_login)?)
  }).await;

  match result {
    Err(_) => {
      Err(BlockingThreadPoolGoneFail)
    }
    Ok(Err(e)) => {
      Err(e)
    }
    Ok(Ok(Some(user))) => {
      identity.remember(serde_json::to_string(&LoggedInUser { user: user.clone() })?);
      Ok(HttpResponse::Ok().json(&user))
    }
    Ok(Ok(None)) => {
      Ok(HttpResponse::Unauthorized().finish())
    }
  }
}

// Logout

pub async fn logout(identity: Identity) -> HttpResponse {
  identity.forget();
  HttpResponse::Ok().finish()
}

// Logged-in user extractor

#[derive(Debug, Error)]
pub enum LoggedInUserExtractInternalError {
  #[error("Failed to extract the identity from the request")]
  IdentityExtractFail(#[from] actix_web::Error),
  #[error("Failed to serialize the identity")]
  DeserializeIdentityFail(#[from] serde_json::Error),
  #[error("Not logged in")]
  NotLoggedInFail,
}

impl ResponseError for LoggedInUserExtractInternalError {
  fn status_code(&self) -> StatusCode {
    match self {
      Self::NotLoggedInFail => StatusCode::UNAUTHORIZED,
      _ => StatusCode::INTERNAL_SERVER_ERROR
    }
  }

  fn error_response(&self) -> HttpResponse {
    let status_code = self.status_code();
    match self {
      Self::NotLoggedInFail => HttpResponse::build(status_code).finish(),
      _ => {
        let format_error = FormatError::new(self);
        event!(Level::ERROR, "{:?}", format_error);
        HttpResponse::build(status_code).json(InternalServerError {
          message: self.to_string()
        })
      }
    }
  }
}

impl FromRequest for LoggedInUser {
  type Error = LoggedInUserExtractInternalError;
  type Future = Pin<Box<dyn Future<Output=Result<LoggedInUser, LoggedInUserExtractInternalError>>>>;

  fn from_request(req: &HttpRequest, payload: &mut Payload<PayloadStream>) -> Self::Future {
    use LoggedInUserExtractInternalError::*;
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
