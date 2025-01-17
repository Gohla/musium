use iced::{Application, Command, Element, Subscription};
use tracing::error;
use url::Url;

use musium_core::model::UserLogin;
use musium_player::{HttpClient, Player};

use crate::page::{login, main};
use crate::util::Update;

pub struct Flags<P: Player> {
  pub initial_url: Url,
  pub initial_user_login: UserLogin,
  pub player: P,
}

pub struct App<P: Player<Client=HttpClient>> {
  player: P,
  current_page: Page<P>,
}

#[derive(Debug)]
enum Page<P: Player<Client=HttpClient>> {
  Login(login::Page<P>),
  Main(main::Page),
}

#[derive(Debug)]
pub enum Message<P: Player> {
  LoginPage(login::Message<P>),
  MainPage(main::Message<P>),
}

impl<P: Player<Client=HttpClient>> Application for App<P> {
  type Executor = iced::executor::Default;
  type Message = Message<P>;
  type Flags = Flags<P>;

  fn new(flags: Flags<P>) -> (Self, Command<Message<P>>) {
    let current_page = Page::Login(login::Page::new(flags.initial_url, flags.initial_user_login));
    let app = Self { player: flags.player, current_page };
    (app, Command::none())
  }

  fn title(&self) -> String {
    "Musium".to_string()
  }

  fn update(&mut self, message: Message<P>) -> Command<Message<P>> {
    match (&mut self.current_page, message) {
      (Page::Login(p), Message::LoginPage(m)) => {
        let Update { action, command } = p.update(&mut self.player, m);
        let command = command.map(|m| Message::LoginPage(m));
        if let Some(login::Action::LoggedIn(user)) = action {
          let (main_page, main_command) = main::Page::new(user, &mut self.player);
          let main_command = main_command.map(|m| Message::MainPage(m));
          self.current_page = Page::Main(main_page);
          Command::batch(vec![command, main_command])
        } else {
          command
        }
      }
      (Page::Main(p), Message::MainPage(m)) => p.update(&mut self.player, m).map(|m| Message::MainPage(m)),
      (p, m) => {
        error!("[BUG] Requested update with message '{:?}', but that message cannot be handled by the current page '{:?}' or the application itself", m, p);
        Command::none()
      }
    }
  }

  fn subscription(&self) -> Subscription<Message<P>> {
    match &self.current_page {
      Page::Login(_) => { Subscription::none() }
      Page::Main(p) => { p.subscription(&self.player).map(|m| Message::MainPage(m)) }
    }
  }

  fn view(&mut self) -> Element<'_, Message<P>> {
    match &mut self.current_page {
      Page::Login(p) => p.view().map(|m| Message::LoginPage(m)),
      Page::Main(p) => p.view().map(|m| Message::MainPage(m)),
    }
  }
}
