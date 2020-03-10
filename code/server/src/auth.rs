use std::future::Future;
use std::pin::Pin;

use actix_identity::Identity;
use actix_web::{FromRequest, HttpRequest, HttpResponse, web};
use actix_web::dev::{Payload, PayloadStream};
use actix_web::error::BlockingError;
use serde::{Deserialize, Serialize};

use backend::{Backend, BackendConnected};
use backend::model::User;

use crate::util::ResultExt;

#[derive(Debug, Deserialize)]
pub struct LoginData {
  pub name: String,
  pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggedInUser {
  pub user: User,
}

pub async fn login(login_data: web::Json<LoginData>, identity: Identity, backend: web::Data<Backend>) -> actix_web::Result<HttpResponse> {
  let user: Result<Option<User>, BlockingError<anyhow::Error>> = web::block(move || {
    let backend_connected: BackendConnected = backend.connect_to_database()?;
    Ok(backend_connected.verify_user(&login_data.name, &login_data.password)?)
  }).await;

  if let Some(user) = user? {
    let logged_in_user = LoggedInUser { user };
    let serialized_identity = serde_json::to_string(&logged_in_user).map_internal_err()?;
    identity.remember(serialized_identity);
    Ok(HttpResponse::Ok().finish())
  } else {
    Ok(HttpResponse::Unauthorized().finish())
  }
}

pub async fn logout(identity: Identity) -> HttpResponse {
  identity.forget();
  HttpResponse::Ok().finish()
}

pub async fn me(logged_in_user: LoggedInUser) -> HttpResponse {
  HttpResponse::Ok().json(logged_in_user)
}

impl FromRequest for LoggedInUser {
  type Error = actix_web::Error;
  type Future = Pin<Box<dyn Future<Output=actix_web::Result<LoggedInUser>>>>;
  type Config = ();

  fn from_request(req: &HttpRequest, payload: &mut Payload<PayloadStream>) -> Self::Future {
    let identity = Identity::from_request(req, payload);
    Box::pin(async move {
      if let Some(serialized_identity) = identity.await?.identity() {
        let logged_in_user = serde_json::from_str(&serialized_identity)?;
        Ok(logged_in_user)
      } else {
        Err(actix_web::error::ErrorUnauthorized("Not logged in"))
      }
    })
  }
}