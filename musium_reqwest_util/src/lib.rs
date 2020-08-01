#![feature(backtrace)]

use std::backtrace::Backtrace;

use reqwest::{Client, ClientBuilder, IntoUrl, Method, RequestBuilder, Response, StatusCode, Url};
use serde::Serialize;
use thiserror::Error;

use musium_core::api::InternalServerError;

pub struct HttpClient {
  client: Client,
  base_url: Url,
}

// Creation

#[derive(Debug, Error)]
pub enum HttpClientCreateError {
  #[error(transparent)]
  ClientCreateFail(#[from] reqwest::Error),
  #[error(transparent)]
  BaseUrlCreateFail(#[from] url::ParseError),
}

impl HttpClient {
  pub fn new(client: Client, base_url: Url) -> Self {
    Self { client, base_url }
  }

  pub fn from_client_builder_url(client_builder: ClientBuilder, base_url: impl IntoUrl) -> Result<Self, HttpClientCreateError> {
    let client = client_builder.build()?;
    let base_url = base_url.into_url()?;
    Ok(Self { client, base_url })
  }

  pub fn from_client_url(client: Client, base_url: impl IntoUrl) -> Result<Self, HttpClientCreateError> {
    let base_url = base_url.into_url()?;
    Ok(Self { client, base_url })
  }
}

// Requests

#[derive(Debug, Error)]
pub enum HttpRequestError {
  #[error(transparent)]
  UrlJoinFail(#[from] url::ParseError),
  #[error(transparent)]
  RequestFail(#[from] reqwest::Error),
  #[error("Server responded with an internal error")]
  InternalServerFail(#[from] InternalServerError, Backtrace),
  #[error("Server responded with unexpected status code: {0}")]
  UnexpectedStatusCode(StatusCode, Backtrace),
}

impl HttpClient {
  pub async fn send_request(
    &self,
    method: Method,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    use HttpRequestError::*;
    let url = self.base_url.join(url_suffix.as_ref())?;
    let response = f_request(self.client.request(method, url)).send().await?;
    match response.status() {
      c @ StatusCode::INTERNAL_SERVER_ERROR => {
        let json: Result<InternalServerError, _> = response.json().await;
        return Err(if let Ok(internal_server_error) = json {
          InternalServerFail(internal_server_error, Backtrace::capture())
        } else {
          UnexpectedStatusCode(c, Backtrace::capture())
        });
      }
      c if !expected_status_codes.as_ref().contains(&c) => {
        return Err(UnexpectedStatusCode(c, Backtrace::capture()));
      }
      _ => {}
    }
    Ok(response)
  }

  pub async fn send_request_simple(
    &self,
    method: Method,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request(method, url_suffix, |r| r, &[StatusCode::OK]).await
  }

  pub async fn send_request_simple_with_json(
    &self,
    method: Method,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.send_request(method, url_suffix, |r| r.json(json), &[StatusCode::OK]).await
  }
}

// Request shorthands

impl HttpClient {
  pub async fn get(
    &self,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request(Method::GET, url_suffix, f_request, expected_status_codes).await
  }

  pub async fn get_simple(
    &self,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request_simple(Method::GET, url_suffix).await
  }

  pub async fn get_simple_with_json(
    &self,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.send_request_simple_with_json(Method::GET, url_suffix, json).await
  }


  pub async fn post(
    &self,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request(Method::POST, url_suffix, f_request, expected_status_codes).await
  }

  pub async fn post_simple(
    &self,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request_simple(Method::POST, url_suffix).await
  }

  pub async fn post_simple_with_json(
    &self,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.send_request_simple_with_json(Method::POST, url_suffix, json).await
  }


  pub async fn put(
    &self,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request(Method::PUT, url_suffix, f_request, expected_status_codes).await
  }

  pub async fn put_simple(
    &self,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request_simple(Method::PUT, url_suffix).await
  }

  pub async fn put_simple_with_json(
    &self,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.send_request_simple_with_json(Method::PUT, url_suffix, json).await
  }


  pub async fn delete(
    &self,
    url_suffix: impl AsRef<str>,
    f_request: impl FnOnce(RequestBuilder) -> RequestBuilder,
    expected_status_codes: impl AsRef<[StatusCode]>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request(Method::DELETE, url_suffix, f_request, expected_status_codes).await
  }

  pub async fn delete_simple(
    &self,
    url_suffix: impl AsRef<str>,
  ) -> Result<Response, HttpRequestError> {
    self.send_request_simple(Method::DELETE, url_suffix).await
  }

  pub async fn delete_simple_with_json(
    &self,
    url_suffix: impl AsRef<str>,
    json: &(impl Serialize + ?Sized),
  ) -> Result<Response, HttpRequestError> {
    self.send_request_simple_with_json(Method::DELETE, url_suffix, json).await
  }
}
