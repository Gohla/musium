pub trait ErrorExt {
  fn into_internal_err(self) -> actix_web::Error;
}

impl<E: std::error::Error + 'static> ErrorExt for E {
  fn into_internal_err(self) -> actix_web::Error {
    actix_web::error::ErrorInternalServerError(self)
  }
}


pub trait ResultExt<T> {
  fn map_internal_err(self) -> actix_web::Result<T>;
}

impl<T, E: std::error::Error + 'static> ResultExt<T> for Result<T, E> {
  fn map_internal_err(self) -> actix_web::Result<T> {
    self.map_err(|e| e.into_internal_err())
  }
}
