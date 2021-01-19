use iced::{Application, Column, Command, Element};

#[derive(Debug, Clone)]
pub enum Message {}

pub struct App {}

impl Application for App {
  type Executor = iced::executor::Default;
  type Message = Message;
  type Flags = ();

  fn new(_flags: ()) -> (Self, Command<Self::Message>) { (Self {}, Command::none()) }

  fn title(&self) -> String {
    "Musium".to_string()
  }

  fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
    Command::none()
  }

  fn view(&mut self) -> Element<'_, Self::Message> {
    Column::new().into()
  }
}
