use iced::{Application, Command, Element};
use tracing::error;

use crate::page::{login, Page as PageTrait};

pub struct App {
  current_page: Page
}

#[derive(Debug)]
pub enum Page {
  Login(login::Page),
}

#[derive(Debug, Clone)]
pub enum Message {
  Login(login::Message),
}

impl Application for App {
  type Executor = iced::executor::Default;
  type Message = Message;
  type Flags = ();

  fn new(_flags: ()) -> (Self, Command<Self::Message>) {
    let current_page = Page::Login(login::Page::new());
    let app = Self { current_page };
    (app, Command::none())
  }

  fn title(&self) -> String {
    "Musium".to_string()
  }

  fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
    match (&mut self.current_page, message) {
      (Page::Login(p), Message::Login(m)) => { p.update(m).map(|m| Message::Login(m)) }
      (p, m) => {
        error!("[BUG] Requested update with message '{:?}', but that message cannot be handled by the current page '{:?}' or the application itself", m, p);
        Command::none()
      }
    }
  }

  fn view(&mut self) -> Element<'_, Self::Message> {
    match &mut self.current_page {
      Page::Login(p) => p.view().map(|m| Message::Login(m)),
    }
  }
}
