use iced::{Command, Element};
use tracing::error;

use musium_client::Client;
use musium_core::model::UserLogin;

use crate::util::Update;

#[derive(Debug)]
pub struct Root {
  current_page: Page,
}

#[derive(Debug)]
pub enum Page {
  Main(main::Page),
  Busy(busy::Page),
  Failed(failed::Page),
}

#[derive(Clone, Debug)]
pub enum Message {
  Main(main::Message),
  Busy(busy::Message),
  Failed(failed::Message),
}

#[derive(Debug)]
pub enum Action {
  LoggedIn
}

impl Root {
  pub fn new(user_login: UserLogin) -> Self {
    Self {
      current_page: Page::Main(main::Page::new(user_login))
    }
  }
}

impl Root {
  pub fn update(&mut self, client: &mut Client, message: Message) -> Update<Message, Action> {
    match (&mut self.current_page, message) {
      (Page::Main(p), Message::Main(m)) => {
        let (action, update) = p.update(client, m).map_command(|m| Message::Main(m)).take_action();
        if let Some(main::Action::LoginRequestSent) = action {
          self.current_page = Page::Busy(busy::Page::new())
        }
        update
      }
      (Page::Busy(p), Message::Busy(m)) => {
        p.update(m).map_command(|m| Message::Busy(m)).discard_action()
      }
      (Page::Failed(p), Message::Failed(m)) => {
        p.update(m).map_command(|m| Message::Failed(m)).discard_action()
      }
      (p, m) => {
        error!("[BUG] Requested update with message '{:?}', but that message cannot be handled by the current page '{:?}' or the application itself", m, p);
        Update::none()
      }
    }
  }

  pub fn view(&mut self) -> Element<'_, Message> {
    match &mut self.current_page {
      Page::Main(p) => p.view().map(|m| Message::Main(m)),
      Page::Busy(p) => p.view().map(|m| Message::Busy(m)),
      Page::Failed(p) => p.view().map(|m| Message::Failed(m)),
    }
  }
}


pub mod main {
  use iced::{Button, button, Column, Command, Element, Row, Text, text_input, TextInput};

  use musium_client::{Client, HttpRequestError};
  use musium_core::model::{UserLogin, User};

  use crate::util::Update;

  #[derive(Debug)]
  pub struct Page {
    name_state: text_input::State,
    password_state: text_input::State,
    login_state: button::State,
    user_login: UserLogin,
  }

  #[derive(Clone, Debug)]
  pub enum Message {
    SetName(String),
    SetPassword(String),
    Login(UserLogin),
    LoginResponseReceived(Result<User, HttpRequestError>)
  }

  #[derive(Debug)]
  pub enum Action {
    LoginRequestSent,
  }

  impl Page {
    pub fn new(user_login: UserLogin) -> Self {
      Self {
        name_state: text_input::State::default(),
        password_state: text_input::State::default(),
        login_state: button::State::default(),
        user_login,
      }
    }

    pub fn update(&mut self, client: &mut Client, message: Message) -> Update<Message, Action> {
      match message {
        Message::SetName(name) => self.user_login.name = name,
        Message::SetPassword(password) => self.user_login.password = password,
        Message::Login(user_login) => {
          let command = Command::perform(client.login(&user_login), super::busy::Message::LoginResponseReceived);
          return Update::new(command, Action::LoginRequestSent);
        }
      }
      Update::none()
    }

    pub fn view(&mut self) -> Element<'_, Message> {
      Row::new()
        .push(TextInput::new(&mut self.name_state, "Name", &self.user_login.name, Message::SetName))
        .push(TextInput::new(&mut self.password_state, "Password", &self.user_login.password, Message::SetPassword)
          .password()
        )
        .push(Button::new(&mut self.login_state, Text::new("Login"))
          .on_press(Message::Login(self.user_login.clone()))
        )
        .into()
    }
  }
}


pub mod busy {
  use iced::{Column, Command, Element};

  use musium_client::HttpRequestError;
  use musium_core::model::User;

  use crate::util::Update;

  #[derive(Debug)]
  pub struct Page {}

  #[derive(Clone, Debug)]
  pub enum Message {

  }

  #[derive(Debug)]
  pub enum Action {
    Success,
    Fail,
  }

  impl Page {
    pub fn new() -> Self { Self {} }

    pub fn update(&mut self, _message: Message) -> Update<Message, Action> {
      Update::none()
    }

    pub fn view(&mut self) -> Element<'_, Message> {
      Column::new().into()
    }
  }
}


pub mod failed {
  use iced::{Column, Command, Element};

  use crate::util::Update;

  #[derive(Debug)]
  pub struct Page {}

  #[derive(Clone, Debug)]
  pub enum Message {}

  #[derive(Debug)]
  pub enum Action {
    Return
  }

  impl Page {
    pub fn new() -> Self { Self {} }

    pub fn update(&mut self, _message: Message) -> Update<Message, Action> {
      Update::none()
    }

    pub fn view(&mut self) -> Element<'_, Message> {
      Column::new().into()
    }
  }
}
