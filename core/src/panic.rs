use std::any::Any;

pub fn try_panic_into_string(panic: Box<dyn Any + Send + 'static>) -> Option<String> {
  if let Some(msg) = panic.downcast_ref::<&'static str>() {
    Some(msg.to_string())
  } else if let Some(msg) = panic.downcast_ref::<String>() {
    Some(msg.to_string())
  } else {
    None
  }
}
