use iced::{Command, Element};
use tracing::error;

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

impl Page {
  pub fn new() -> Self { Self::Main(main::Page::new()) }
}

impl crate::page::Page for Page {
  type Message = Message;

  fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
    match (self, message) {
      (Page::Main(p), Message::Main(m)) => { p.update(m).map(|m| Message::Main(m)) }
      (Page::Busy(p), Message::Busy(m)) => { p.update(m).map(|m| Message::Busy(m)) }
      (Page::Failed(p), Message::Failed(m)) => { p.update(m).map(|m| Message::Failed(m)) }
      (p, m) => {
        error!("[BUG] Requested update with message '{:?}', but that message cannot be handled by the current page '{:?}' or the application itself", m, p);
        Command::none()
      }
    }
  }

  fn view(&mut self) -> Element<'_, Self::Message> {
    match self {
      Page::Main(p) => p.view().map(|m| Message::Main(m)),
      Page::Busy(p) => p.view().map(|m| Message::Busy(m)),
      Page::Failed(p) => p.view().map(|m| Message::Failed(m)),
    }
  }
}


pub mod main {
  use iced::{Column, Command, Element};

  use musium_core::model::UserLogin;

  #[derive(Debug)]
  pub struct Page {}

  #[derive(Clone, Debug)]
  pub enum Message {
    Login(UserLogin),
  }

  impl Page {
    pub fn new() -> Self { Self {} }
  }

  impl crate::page::Page for Page {
    type Message = Message;

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
      Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
      Column::new().into()
    }
  }
}


pub mod busy {
  use iced::{Column, Command, Element};

  #[derive(Debug)]
  pub struct Page {}

  #[derive(Clone, Debug)]
  pub enum Message {}

  impl Page {
    pub fn new() -> Self { Self {} }
  }

  impl crate::page::Page for Page {
    type Message = Message;

    fn update(&mut self, _message: Message) -> Command<Self::Message> {
      Command::none()
    }

    fn view(&mut self) -> Element<'_, Message> {
      Column::new().into()
    }
  }
}


pub mod failed {
  use iced::{Column, Command, Element};

  #[derive(Debug)]
  pub struct Page {}

  #[derive(Clone, Debug)]
  pub enum Message {}

  impl Page {
    pub fn new() -> Self { Self {} }
  }

  impl crate::page::Page for Page {
    type Message = Message;

    fn update(&mut self, message: Message) -> Command<Self::Message> {
      Command::none()
    }

    fn view(&mut self) -> Element<'_, Message> {
      Column::new().into()
    }
  }
}
