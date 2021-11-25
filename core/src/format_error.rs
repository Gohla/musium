use core::fmt;
use core::fmt::{Debug, Formatter, Write};
use std::backtrace::BacktraceStatus;
use std::error::Error;
use std::vec;

use self::ChainState::*;

// Formatting of errors (selectively copied from https://github.com/dtolnay/anyhow/blob/master/src/fmt.rs)

pub struct FormatError<'a, E: Error> {
  error: &'a E,
}

impl<'a, E: Error> FormatError<'a, E> {
  pub fn new(error: &'a E) -> Self {
    Self { error }
  }
}

impl<'a, E: Error> Debug for FormatError<'a, E> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    if f.alternate() {
      return Debug::fmt(self.error, f);
    }

    if let Some(cause) = self.error.source() {
      write!(f, "\n\nCaused by:")?;
      let multiple = cause.source().is_some();
      for (n, error) in Chain::new(cause).enumerate() {
        writeln!(f)?;
        let mut indented = Indented {
          inner: f,
          number: if multiple { Some(n) } else { None },
          started: false,
        };
        write!(indented, "{}", error)?;
      }
    }

    if let Some(backtrace) = self.error.backtrace() {
      if let BacktraceStatus::Captured = backtrace.status() {
        let mut backtrace = backtrace.to_string();
        write!(f, "\n\n")?;
        if backtrace.starts_with("stack backtrace:") {
          // Capitalize to match "Caused by:"
          backtrace.replace_range(0..1, "S");
        } else {
          // "stack backtrace:" prefix was removed in
          // https://github.com/rust-lang/backtrace-rs/pull/286
          writeln!(f, "Stack backtrace:")?;
        }
        backtrace.truncate(backtrace.trim_end().len());
        write!(f, "{}", backtrace)?;
      }
    }

    Ok(())
  }
}

// Indentation

struct Indented<'a, D> {
  inner: &'a mut D,
  number: Option<usize>,
  started: bool,
}

impl<T> Write for Indented<'_, T>
  where
    T: Write,
{
  fn write_str(&mut self, s: &str) -> fmt::Result {
    for (i, line) in s.split('\n').enumerate() {
      if !self.started {
        self.started = true;
        match self.number {
          Some(number) => write!(self.inner, "{: >5}: ", number)?,
          None => self.inner.write_str("    ")?,
        }
      } else if i > 0 {
        self.inner.write_char('\n')?;
        if self.number.is_some() {
          self.inner.write_str("       ")?;
        } else {
          self.inner.write_str("    ")?;
        }
      }

      self.inner.write_str(line)?;
    }

    Ok(())
  }
}

// Error source chain (selectively copied from https://github.com/dtolnay/anyhow/blob/master/src/chain.rs)

#[derive(Clone)]
struct Chain<'a> {
  state: ChainState<'a>,
}

#[derive(Clone)]
enum ChainState<'a> {
  Linked {
    next: Option<&'a (dyn Error + 'static)>,
  },
  Buffered {
    rest: vec::IntoIter<&'a (dyn Error + 'static)>,
  },
}

impl<'a> Chain<'a> {
  fn new(head: &'a (dyn Error + 'static)) -> Self {
    Chain {
      state: ChainState::Linked { next: Some(head) },
    }
  }
}

impl<'a> Iterator for Chain<'a> {
  type Item = &'a (dyn Error + 'static);

  fn next(&mut self) -> Option<Self::Item> {
    match &mut self.state {
      Linked { next } => {
        let error = (*next)?;
        *next = error.source();
        Some(error)
      }
      Buffered { rest } => rest.next(),
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.len();
    (len, Some(len))
  }
}

impl DoubleEndedIterator for Chain<'_> {
  fn next_back(&mut self) -> Option<Self::Item> {
    match &mut self.state {
      Linked { mut next } => {
        let mut rest = Vec::new();
        while let Some(cause) = next {
          next = cause.source();
          rest.push(cause);
        }
        let mut rest = rest.into_iter();
        let last = rest.next_back();
        self.state = Buffered { rest };
        last
      }
      Buffered { rest } => rest.next_back(),
    }
  }
}

impl ExactSizeIterator for Chain<'_> {
  fn len(&self) -> usize {
    match &self.state {
      Linked { mut next } => {
        let mut len = 0;
        while let Some(cause) = next {
          next = cause.source();
          len += 1;
        }
        len
      }
      Buffered { rest } => rest.len(),
    }
  }
}

impl Default for Chain<'_> {
  fn default() -> Self {
    Chain {
      state: ChainState::Buffered {
        rest: Vec::new().into_iter(),
      },
    }
  }
}
