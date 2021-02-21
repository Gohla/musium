use iced::{Application, Command, Element};
use tracing::error;

use musium_audio_output_rodio::RodioAudioOutput;
use musium_client_http::{HttpClient, Url};
use musium_core::model::UserLogin;

use crate::page::{login, track};
use crate::util::Update;

pub struct Flags {
  pub initial_url: Url,
  pub initial_user_login: UserLogin,
  pub client: HttpClient,
  pub audio_player: Option<RodioAudioOutput>,
}

pub struct App {
  client: HttpClient,
  audio_player: Option<RodioAudioOutput>,
  current_page: Page,
}

#[derive(Debug)]
enum Page {
  Login(login::Page),
  Track(track::Page),
}

#[derive(Clone, Debug)]
pub enum Message {
  Login(login::Message),
  Track(track::Message),
}

impl Application for App {
  type Executor = iced::executor::Default;
  type Message = Message;
  type Flags = Flags;

  fn new(flags: Flags) -> (Self, Command<Message>) {
    let current_page = Page::Login(login::Page::new(flags.initial_url, flags.initial_user_login));
    let app = Self { client: flags.client, audio_player: flags.audio_player, current_page };
    (app, Command::none())
  }

  fn title(&self) -> String {
    "Musium".to_string()
  }

  fn update(&mut self, message: Message) -> Command<Message> {
    match (&mut self.current_page, message) {
      (Page::Login(p), Message::Login(m)) => {
        let Update { action, command } = p.update(&mut self.client, m);
        let command = command.map(|m| Message::Login(m));
        if let Some(login::Action::LoggedIn(user)) = action {
          let (track_page, track_command) = track::Page::new(user, &mut self.client);
          let track_command = track_command.map(|m| Message::Track(m));
          self.current_page = Page::Track(track_page);
          Command::batch(vec![command, track_command])
        } else {
          command
        }
      }
      (Page::Track(p), Message::Track(m)) => p.update(&mut self.client, &mut self.audio_player, m).into_command().map(|m| Message::Track(m)),
      (p, m) => {
        error!("[BUG] Requested update with message '{:?}', but that message cannot be handled by the current page '{:?}' or the application itself", m, p);
        Command::none()
      }
    }
  }

  fn view(&mut self) -> Element<'_, Message> {
    match &mut self.current_page {
      Page::Login(p) => p.view().map(|m| Message::Login(m)),
      Page::Track(p) => p.view().map(|m| Message::Track(m)),
    }
  }
}
