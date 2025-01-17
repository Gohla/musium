#![allow(dead_code)]

use std::fmt::Debug;

use iced::{Button, Command, Element};

pub struct Update<M, A> {
  pub command: Command<M>,
  pub action: Option<A>,
}

impl<M: Debug + Send, A> Update<M, A> {
  pub fn new(command: Command<M>, action: Option<A>) -> Self { Self { command, action } }

  pub fn command(command: Command<M>) -> Self { Self { command, action: None } }

  pub fn action(action: A) -> Self { Self { command: Command::none(), action: Some(action) } }

  pub fn none() -> Self { Self { command: Command::none(), action: None } }


  pub fn unwrap(self) -> (Command<M>, Option<A>) { (self.command, self.action) }


  pub fn into_command(self) -> Command<M> { self.command }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn map_command<MM>(self, f: impl Fn(M) -> MM + 'static + Send + Sync) -> Update<MM, A> where M: 'static, MM: Send + Debug {
    Update::new(self.command.map(f), self.action)
  }

  #[cfg(target_arch = "wasm32")]
  pub fn map_command<MM>(self, f: impl Fn(M) -> MM + 'static) -> Update<MM, A> where M: 'static {
    Update::new(self.command.map(f), self.action)
  }


  pub fn into_action(self) -> Option<A> { self.action }

  pub fn take_action<AA>(self) -> (Option<A>, Update<M, AA>) {
    (self.action, Update::new(self.command, None))
  }

  pub fn discard_action<AA>(self) -> Update<M, AA> {
    Update::new(self.command, None)
  }

  pub fn map_action<AA>(self, f: impl Fn(A) -> AA) -> Update<M, AA> {
    Update::new(self.command, self.action.map(f))
  }
}

pub trait ButtonEx<'a> {
  fn on_press_into<M: 'static>(self, message_fn: impl 'static + Fn() -> M, enabled: bool) -> Element<'a, M>;
}

impl<'a> ButtonEx<'a> for Button<'a, ()> {
  fn on_press_into<M: 'static>(self, message_fn: impl 'static + Fn() -> M, enabled: bool) -> Element<'a, M> {
    let button: Element<_> = if enabled {
      self.on_press(()).into()
    } else {
      self.into()
    };
    button.map(move |_| message_fn())
  }
}
