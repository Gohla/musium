// use tracing::{event, Level};


// pub trait ResultExt<T, E> {
//   fn report_if_error(self) -> Result<T, E>;
// }
//
// impl<T, E: std::error::Error + 'static> ResultExt<T, E> for Result<T, E> {
//   fn report_if_error(self) -> Result<T, E> {
//     match &self {
//       Err(e) => {
//         let format_error = FormatError::new(e);
//         event!(Level::ERROR, "{:?}", format_error);
//       }
//       _ => {}
//     }
//     self
//   }
// }

// #[inline]
// pub fn report_if_error<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
//   let result = f();
//   if let Err(e) = &result {
//     let format_error = musium_core::format_error::FormatError::new(e);
//     event!(Level::ERROR, "{:?}", format_error);
//   }
//   result
// }

// macro_rules! report_if_error {
//   (|| -> $t:ty $e:expr) => {{
//     let result = (|| -> Result<_, $t> { $e })();
//     if let Err(e) = &result {
//       let format_error = musium_core::format_error::FormatError::new(e);
//       event!(Level::ERROR, "{:?}", format_error);
//     }
//     result
//   }}
// }
