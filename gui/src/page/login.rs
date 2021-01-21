use std::sync::Arc;

use iced::{Button, button, Column, Command, Element, Row, Text, text_input, TextInput};

use musium_client::{Client, HttpRequestError};
use musium_core::model::{User, UserLogin};

use crate::util::Update;

#[derive(Default, Debug)]
pub struct Page {
  name_input: text_input::State,
  password_input: text_input::State,
  login_button: button::State,
  user_login: UserLogin,
  state: State,
}

#[derive(Clone, Debug)]
pub enum Message {
  SetName(String),
  SetPassword(String),
  SendLoginRequest(UserLogin),
  LoginResponseReceived(Result<User, Arc<HttpRequestError>>),
  Return,
}

#[derive(Debug)]
pub enum Action { LoggedIn(User) }

#[derive(Debug)]
enum State { Idle, Busy, Failed(Arc<HttpRequestError>) }

impl Default for State { fn default() -> Self { Self::Idle } }

impl Page {
  pub fn new(user_login: UserLogin) -> Self {
    Self {
      user_login,
      ..Self::default()
    }
  }

  pub fn update(&mut self, client: &mut Client, message: Message) -> Update<Message, Action> {
    match message {
      Message::SetName(name) => self.user_login.name = name,
      Message::SetPassword(password) => self.user_login.password = password,
      Message::SendLoginRequest(user_login) => {
        let client = client.clone();
        let command = Command::perform(async move { client.login(&user_login).await }, |r| Message::LoginResponseReceived(r.map_err(|e| Arc::new(e))));
        self.state = State::Busy;
        return Update::command(command);
      }
      Message::LoginResponseReceived(response) => match response {
        Ok(user) => {
          self.state = State::Idle;
          return Update::action(Action::LoggedIn(user));
        }
        Err(e) => self.state = State::Failed(e),
      },
      Message::Return => self.state = State::Idle,
    }
    Update::none()
  }

  pub fn view(&mut self) -> Element<'_, Message> {
    match &self.state {
      State::Idle => Row::new()
        .push(TextInput::new(&mut self.name_input, "Name", &self.user_login.name, Message::SetName))
        .push(TextInput::new(&mut self.password_input, "Password", &self.user_login.password, Message::SetPassword)
          .password()
        )
        .push(Button::new(&mut self.login_button, Text::new("Login"))
          .on_press(Message::SendLoginRequest(self.user_login.clone()))
        )
        .into(),
      State::Busy => Row::new()
        .push(Text::new("Logging in..."))
        .into(),
      State::Failed(e) => Column::new()
        .push(Text::new("Logging in failed sadface"))
        .push(Text::new(format!("{:?}", e)))
        .push(Button::new(&mut self.login_button, Text::new("Return"))
          .on_press(Message::Return)
        )
        .into(),
    }
  }
}

// use iced::{Command, Element};
// use tracing::error;
//
// use musium_client::Client;
// use musium_core::model::UserLogin;
//
// use crate::util::Update;
//
// #[derive(Debug)]
// pub struct Root {
//   current_page: Page,
// }
//
// #[derive(Debug)]
// pub enum Page {
//   Main(main::Page),
//   Busy(busy::Page),
//   Failed(failed::Page),
// }
//
// #[derive(Clone, Debug)]
// pub enum Message {
//   Main(main::Message),
//   Busy(busy::Message),
//   Failed(failed::Message),
// }
//
// #[derive(Debug)]
// pub enum Action {
//   LoggedIn
// }
//
// impl Root {
//   pub fn new(user_login: UserLogin) -> Self {
//     Self {
//       current_page: Page::Main(main::Page::new(user_login))
//     }
//   }
// }
//
// impl Root {
//   pub fn update(&mut self, client: &mut Client, message: Message) -> Update<Message, Action> {
//     match (&mut self.current_page, message) {
//       (Page::Main(p), Message::Main(m)) => {
//         let (action, update) = p.update(client, m).map_command(|m| Message::Main(m)).take_action();
//         if let Some(main::Action::LoginRequestSent) = action {
//           self.current_page = Page::Busy(busy::Page::new())
//         }
//         update
//       }
//       (Page::Busy(p), Message::Busy(m)) => {
//         p.update(m).map_command(|m| Message::Busy(m)).discard_action()
//       }
//       (Page::Failed(p), Message::Failed(m)) => {
//         p.update(m).map_command(|m| Message::Failed(m)).discard_action()
//       }
//       (p, m) => {
//         error!("[BUG] Requested update with message '{:?}', but that message cannot be handled by the current page '{:?}' or the application itself", m, p);
//         Update::none()
//       }
//     }
//   }
//
//   pub fn view(&mut self) -> Element<'_, Message> {
//     match &mut self.current_page {
//       Page::Main(p) => p.view().map(|m| Message::Main(m)),
//       Page::Busy(p) => p.view().map(|m| Message::Busy(m)),
//       Page::Failed(p) => p.view().map(|m| Message::Failed(m)),
//     }
//   }
// }
//
//
// pub mod main {
//
// }
//
//
// pub mod busy {
//   use iced::{Column, Command, Element};
//
//   use musium_client::HttpRequestError;
//   use musium_core::model::User;
//
//   use crate::util::Update;
//
//   #[derive(Debug)]
//   pub struct Page {}
//
//   #[derive(Clone, Debug)]
//   pub enum Message {
//
//   }
//
//   #[derive(Debug)]
//   pub enum Action {
//     Success,
//     Fail,
//   }
//
//   impl Page {
//     pub fn new() -> Self { Self {} }
//
//     pub fn update(&mut self, _message: Message) -> Update<Message, Action> {
//       Update::none()
//     }
//
//     pub fn view(&mut self) -> Element<'_, Message> {
//       Column::new().into()
//     }
//   }
// }
//
//
// pub mod failed {
//   use iced::{Column, Command, Element};
//
//   use crate::util::Update;
//
//   #[derive(Debug)]
//   pub struct Page {}
//
//   #[derive(Clone, Debug)]
//   pub enum Message {}
//
//   #[derive(Debug)]
//   pub enum Action {
//     Return
//   }
//
//   impl Page {
//     pub fn new() -> Self { Self {} }
//
//     pub fn update(&mut self, _message: Message) -> Update<Message, Action> {
//       Update::none()
//     }
//
//     pub fn view(&mut self) -> Element<'_, Message> {
//       Column::new().into()
//     }
//   }
// }
