use iced::{Command, Element};
use tracing::error;

use musium_core::model::UserLogin;

use crate::util::{Component, Update};

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
  pub fn new(user_login: UserLogin) -> Self { Self { current_page: Page::Main(main::Page::new(user_login)) } }
}

impl Component for Root {
  type Message = Message;
  type Action = Action;

  fn update(&mut self, message: Self::Message) -> Update<Message, Action> {
    match (&mut self.current_page, message) {
      (Page::Main(p), Message::Main(m)) => {
        let (action, update) = p.update(m).map_command(|m| Message::Main(m)).take_action();
        if let Some(main::Action::Start) = action {
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

  fn view(&mut self) -> Element<'_, Self::Message> {
    match &mut self.current_page {
      Page::Main(p) => p.view().map(|m| Message::Main(m)),
      Page::Busy(p) => p.view().map(|m| Message::Busy(m)),
      Page::Failed(p) => p.view().map(|m| Message::Failed(m)),
    }
  }
}


pub mod main {
  use iced::{Button, button, Column, Command, Element, Row, Text, text_input, TextInput};

  use musium_core::model::UserLogin;

  use crate::util::{Component, Update};

  #[derive(Default, Debug)]
  pub struct Page {
    name_state: text_input::State,
    password_state: text_input::State,
    user_login: UserLogin,
    login_state: button::State,
  }

  #[derive(Clone, Debug)]
  pub enum Message {
    SetName(String),
    SetPassword(String),
    Login(UserLogin),
  }

  #[derive(Debug)]
  pub enum Action {
    Start,
  }

  impl Page {
    pub fn new(user_login: UserLogin) -> Self {
      Self {
        user_login,
        ..Self::default()
      }
    }
  }

  impl Component for Page {
    type Message = Message;
    type Action = Action;

    fn update(&mut self, message: Self::Message) -> Update<Message, Action> {
      match message {
        Message::SetName(name) => self.user_login.name = name,
        Message::SetPassword(password) => self.user_login.password = password,
        Message::Login(user_login) => return Update::action(Action::Start),
      }
      Update::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
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

  use crate::util::{Component, Update};

  #[derive(Debug)]
  pub struct Page {}

  #[derive(Clone, Debug)]
  pub enum Message {}

  #[derive(Debug)]
  pub enum Action {
    Success,
    Fail,
  }

  impl Page {
    pub fn new() -> Self { Self {} }
  }

  impl Component for Page {
    type Message = Message;
    type Action = Action;

    fn update(&mut self, _message: Message) -> Update<Message, Action> {
      Update::none()
    }

    fn view(&mut self) -> Element<'_, Message> {
      Column::new().into()
    }
  }
}


pub mod failed {
  use iced::{Column, Command, Element};

  use crate::util::{Component, Update};

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
  }

  impl Component for Page {
    type Message = Message;
    type Action = Action;

    fn update(&mut self, _message: Message) -> Update<Message, Action> {
      Update::none()
    }

    fn view(&mut self) -> Element<'_, Message> {
      Column::new().into()
    }
  }
}
