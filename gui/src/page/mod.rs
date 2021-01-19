use iced::{Command, Element};

pub mod login;

pub trait Page {
  type Message: std::fmt::Debug + Send;

  fn title(&self) -> Option<String> { None }

  fn update(&mut self, message: Self::Message) -> Command<Self::Message>;

  fn view(&mut self) -> Element<'_, Self::Message>;
}
