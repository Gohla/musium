#![allow(dead_code, unused_imports, unused_variables)]

use std::sync::Arc;

use iced::{Align, Button, button, Column, Command, Element, HorizontalAlignment, Length, Row, Text, text_input, TextInput};
use tracing::{error, debug};

use musium_client::{Client, HttpRequestError, Url};
use musium_core::format_error::FormatError;
use musium_core::model::{User, UserLogin};

use crate::util::Update;

#[derive(Default, Debug)]
pub struct Page {
  url_input: text_input::State,
  name_input: text_input::State,
  password_input: text_input::State,
  login_button: button::State,

  url: String,
  parsed_url: Option<Url>,
  url_parse_error: Option<url::ParseError>,
  user_login: UserLogin,

  state: State,
}

#[derive(Clone, Debug)]
pub enum Message {
  SetUrl(String),
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
  pub fn new(url: Url, user_login: UserLogin) -> Self {
    Self {
      url: url.to_string(),
      parsed_url: Some(url),
      user_login,
      ..Self::default()
    }
  }

  pub fn update(&mut self, client: &mut Client, message: Message) -> Update<Message, Action> {
    match message {
      Message::SetUrl(url) => {
        self.url = url.clone();
        match Url::parse(&url) {
          Ok(url) => {
            self.parsed_url = Some(url);
            self.url_parse_error = None;
          }
          Err(e) => {
            self.parsed_url = None;
            self.url_parse_error = Some(e)
          }
        }
      }
      Message::SetName(name) => self.user_login.name = name,
      Message::SetPassword(password) => self.user_login.password = password,
      Message::SendLoginRequest(user_login) => {
        let client = client.clone();
        let command = Command::perform(
          async move { client.login(&user_login).await },
          |r| Message::LoginResponseReceived(r.map_err(|e| Arc::new(e))),
        );
        self.state = State::Busy;
        return Update::command(command);
      }
      Message::LoginResponseReceived(result) => match result {
        Ok(user) => {
          debug!("Logged in as: {}", user.name);
          self.state = State::Idle;
          return Update::action(Action::LoggedIn(user));
        }
        Err(e) => {
          let format_error = FormatError::new(e.as_ref());
          error!("Failed to log in: {:?}", format_error);
          self.state = State::Failed(e)
        }
      },
      Message::Return => self.state = State::Idle,
    }
    Update::none()
  }

  pub fn view(&mut self) -> Element<'_, Message> {
    let title = Text::new("Musium")
      .width(Length::Fill)
      .size(100)
      .color([0.5, 0.5, 0.5])
      .horizontal_alignment(HorizontalAlignment::Center);

    let content: Element<_> = match &self.state {
      State::Idle => {
        let spacing = 5;
        let align = Align::Center;
        let label_size = 20;
        let label_width = Length::Units(100);
        let input_size = 30;
        let input_width = Length::Units(400);
        let input_padding = 5;
        Column::new().spacing(spacing).align_items(align)
          .push(Row::new().spacing(spacing).align_items(align)
            .push(Text::new("Server URL")
              .size(label_size)
              .width(label_width)
            )
            .push(TextInput::new(&mut self.url_input, "Server URL", &self.url, Message::SetUrl)
              .size(input_size)
              .width(input_width)
              .padding(input_padding)
            )
          )
          .push(Row::new().spacing(spacing).align_items(align)
            .push(Text::new("Name")
              .size(label_size)
              .width(label_width)
            )
            .push(TextInput::new(&mut self.name_input, "Name", &self.user_login.name, Message::SetName)
              .size(input_size)
              .width(input_width)
              .padding(input_padding)
            )
          )
          .push(Row::new().spacing(spacing).align_items(align)
            .push(Text::new("Password")
              .size(label_size)
              .width(label_width)
            )
            .push(TextInput::new(&mut self.password_input, "Password", &self.user_login.password, Message::SetPassword)
              .size(input_size)
              .width(input_width)
              .padding(input_padding)
              .password()
            )
          )
          .push(Button::new(&mut self.login_button, Text::new("Login").size(30).width(label_width).horizontal_alignment(HorizontalAlignment::Center))
            .on_press(Message::SendLoginRequest(self.user_login.clone()))
          )
          .into()
      }
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
    };

    let view = Column::new()
      .spacing(20)
      .align_items(Align::Center)
      .push(title)
      .push(content);
    view.into()
  }
}
