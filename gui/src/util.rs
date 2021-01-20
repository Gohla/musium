use std::fmt::Debug;

use iced::Command;

pub struct Update<M, A> {
  pub command: Command<M>,
  pub action: Option<A>,
}

impl<M: Debug + Send, A> Update<M, A> {
  pub fn new(command: Command<M>, action: A) -> Self { Self { command, action: Some(action) } }

  pub fn command(command: Command<M>) -> Self { Self { command, action: None } }

  pub fn action(action: A) -> Self { Self { command: Command::none(), action: Some(action) } }

  pub fn none() -> Self { Self { command: Command::none(), action: None } }


  #[cfg(not(target_arch = "wasm32"))]
  pub fn map_command<MM>(mut self, f: impl Fn(M) -> MM + 'static + Send + Sync) -> Update<MM, A> where M: 'static, MM: Send + Debug {
    Update::new(self.command.map(f), self.action)
  }

  #[cfg(target_arch = "wasm32")]
  pub fn map_command<MM>(mut self, f: impl Fn(M) -> MM + 'static) -> Update<MM, A> where M: 'static {
    Update::new(self.command.map(f), self.action)
  }


  pub fn take_action<AA>(mut self) -> (Option<A>, Update<M, AA>) {
    (self.action, Update::new(self.command, None))
  }

  pub fn discard_action<AA>(mut self) -> Update<M, AA> {
    Update::new(self.command, None)
  }

  pub fn map_action<AA>(mut self, f: impl Fn(A) -> AA) -> Update<M, AA> {
    Update::new(self.command, self.action.map(f))
  }
}
