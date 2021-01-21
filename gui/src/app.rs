use iced::{Application, Command, Element, Row};
use tracing::error;

use musium_client::{Client, Url};
use musium_core::model::UserLogin;

use crate::page::login;
use crate::util::Update;

pub struct Flags {
  pub initial_url: Url,
  pub initial_user_login: UserLogin,
  pub client: Client,
}

pub struct App {
  client: Client,
  current_page: Page,
}

#[derive(Debug)]
enum Page {
  Login(login::Page),
  Main,
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
    let current_page = Page::Login(login::Page::new(flags.initial_url, flags.initial_user_login));
    let app = Self { client: flags.client, current_page };
    (app, Command::none())
  }

  fn title(&self) -> String {
    "Musium".to_string()
  }

  fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
    match (&mut self.current_page, message) {
      (Page::Login(p), Message::Login(m)) => {
        let Update { action, command } = p.update(&mut self.client, m);
        if let Some(login::Action::LoggedIn(_)) = action {
          self.current_page = Page::Main;
        }
        command.map(|m| Message::Login(m))
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
      Page::Main => Row::new().into()
    }
  }
}
