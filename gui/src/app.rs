use iced::{Application, Command, Element};
use tracing::error;

use musium_client::Client;
use musium_core::model::UserLogin;

use crate::page::login;

pub struct Flags {
  pub client: Client,
  pub user_login: UserLogin,
}

pub struct App {
  client: Client,
  current_page: Page,
}

#[derive(Debug)]
enum Page {
  Login(login::Root),
}

#[derive(Clone, Debug)]
pub enum Message {
  Login(login::Message),
}

impl Application for App {
  type Executor = iced::executor::Default;
  type Message = Message;
  type Flags = Flags;

  fn new(flags: Flags) -> (Self, Command<Message>) {
    let current_page = Page::Login(login::Root::new(flags.user_login));
    let app = Self { client: flags.client, current_page };
    (app, Command::none())
  }

  fn title(&self) -> String {
    "Musium".to_string()
  }

  fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
    match (&mut self.current_page, message) {
      (Page::Login(p), Message::Login(m)) => {
        p.update(&mut self.client, m).command.map(|m| Message::Login(m))
      }
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
