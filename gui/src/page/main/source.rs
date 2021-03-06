use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use iced::{Align, button, Button, Checkbox, Column, Command, Element, Length, Row, Rule, scrollable, Text};

use musium_core::model::{LocalSource, SpotifySource};
use musium_player::{Client, ClientT, Player};

use crate::page::main::{cell_text, h1, h2, header_text};
use crate::util::{ButtonEx, Update};
use crate::widget::table::TableBuilder;

#[derive(Default, Debug)]
pub struct Tab {
  local_sources: LocalSources,
  spotify_sources: SpotifySources,

  refreshing: bool,
  refresh_button_state: button::State,

  syncing: bool,
  sync_all_button_state: button::State,
}


#[derive(Debug)]
pub enum Message {
  RequestRefresh,
  ReceiveLocalSources(Result<Vec<LocalSourceViewModel>, <Client as ClientT>::LocalSourceError>),
  ReceiveSpotifySources(Result<Vec<SpotifySourceViewModel>, <Client as ClientT>::SpotifySourceError>),

  SetLocalSourceEnabled(i32, bool),
  SetSpotifySourceEnabled(i32, bool),

  RequestSync,
  RequestLocalSourcesSync,
  RequestLocalSourceSync(i32),
  RequestSpotifySourcesSync,
  RequestSpotifySourceSync(i32),
  ReceiveSyncStatus,
}

impl<'a> Tab {
  pub fn update(&mut self, player: &Player, message: Message) -> Update<Message, super::Action> {
    match message {
      Message::RequestRefresh => {}
      Message::ReceiveLocalSources(r) => {}
      Message::ReceiveSpotifySources(r) => {}

      Message::SetLocalSourceEnabled(_, _) => {}
      Message::SetSpotifySourceEnabled(_, _) => {}

      Message::RequestSync => {}
      Message::RequestLocalSourcesSync => {}
      Message::RequestLocalSourceSync(_) => {}
      Message::RequestSpotifySourcesSync => {}
      Message::RequestSpotifySourceSync(_) => {}
      Message::ReceiveSyncStatus => {}
    }
    Update::none()
  }

  pub fn view(&'a mut self) -> Element<'a, Message> {
    let header = Row::new()
      .width(Length::Fill)
      .align_items(Align::Center)
      .spacing(2)
      .push(h1("Sources"))
      .push(Button::new(&mut self.refresh_button_state, Text::new("Refresh all sources")).on_press_into(|| Message::RequestRefresh, !self.refreshing))
      .push(Button::new(&mut self.sync_all_button_state, Text::new("Sync all sources")).on_press_into(|| Message::RequestSync, !self.syncing))
      ;

    let local_sources = self.local_sources.view(self.syncing);
    let spotify_sources = self.spotify_sources.view(self.syncing);

    Column::new()
      .width(Length::Fill)
      .height(Length::Fill)
      .align_items(Align::Center)
      .padding(4)
      .spacing(4)
      .push(header)
      .push(Rule::horizontal(1))
      .push(local_sources)
      .push(Rule::horizontal(1))
      .push(spotify_sources)
      .into()
  }

  fn refresh_sources(&mut self, player: &Player) -> Command<Message> {
    let local_sources_command = {
      let player = player.clone();
      Command::perform(
        async move {
          let sources = player.get_client().list_local_sources().await?;
          let view_models: Vec<_> = sources.into_iter().map(|s| s.into()).collect();
          Ok(view_models)
        },
        |r| Message::ReceiveLocalSources(r),
      )
    };
    let spotify_sources_command = {
      let player = player.clone();
      Command::perform(
        async move {
          let sources = player.get_client().list_spotify_sources().await?;
          let view_models: Vec<_> = sources.into_iter().map(|s| s.into()).collect();
          Ok(view_models)
        },
        |r| Message::ReceiveSpotifySources(r),
      )
    };
    Command::batch(vec![local_sources_command, spotify_sources_command])
  }
}

// Local sources

#[derive(Default, Debug)]
struct LocalSources {
  sources: Rc<RefCell<Vec<LocalSourceViewModel>>>,
  rows_scrollable_state: scrollable::State,
  sync_button_state: button::State,
}

impl<'a> LocalSources {
  pub fn update(&mut self, sources: Vec<LocalSourceViewModel>) {
    self.sources = Rc::new(RefCell::new(sources));
  }

  fn view(&'a mut self, syncing: bool) -> Element<'a, Message> {
    let table: Element<_> = TableBuilder::new(self.sources.clone())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
      .push_column(5, header_text("ID"), Box::new(|t| {
        cell_text(t.source.id.to_string())
      }))
      .push_column(5, header_text("Directory"), Box::new(|t|
        cell_text(t.source.directory.clone())
      ))
      .push_column(25, header_text("Enabled"), Box::new(|t| {
        let id = t.source.id;
        Checkbox::new(t.source.enabled, "Enabled", move |e| Message::SetLocalSourceEnabled(id, e)).into()
      }))
      .push_column(25, header_text("Sync"), Box::new(move |t| {
        let id = t.source.id;
        Button::new(&mut t.sync_button_state, Text::new("Sync"))
          .on_press_into(move || Message::RequestLocalSourceSync(id), !syncing)
      }))
      .build(&mut self.rows_scrollable_state)
      .into();

    Column::new()
      .width(Length::Fill)
      .align_items(Align::Center)
      .spacing(2)
      .push(Row::new()
        .spacing(2)
        .push(h2("Local sources"))
        .push(Button::new(&mut self.sync_button_state, Text::new("Sync local sources"))
          .on_press_into(move || Message::RequestLocalSourcesSync, !syncing)
        )
      )
      .push(Rule::horizontal(1))
      .push(table)
      .into()
  }
}

#[derive(Debug)]
pub struct LocalSourceViewModel {
  source: LocalSource,
  sync_button_state: button::State,
}

impl<'a> From<LocalSource> for LocalSourceViewModel {
  fn from(source: LocalSource) -> Self { Self { source, sync_button_state: button::State::default() } }
}

// Spotify sources

#[derive(Default, Debug)]
struct SpotifySources {
  sources: Rc<RefCell<Vec<SpotifySourceViewModel>>>,
  rows_scrollable_state: scrollable::State,
  sync_button_state: button::State,
}

impl<'a> SpotifySources {
  pub fn update(&mut self, sources: Vec<SpotifySourceViewModel>) {
    self.sources = Rc::new(RefCell::new(sources));
  }
  
  fn view(&'a mut self, syncing: bool) -> Element<'a, Message> {
    let table: Element<_> = TableBuilder::new(self.sources.clone())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
      .push_column(5, header_text("ID"), Box::new(|t| {
        cell_text(t.source.id.to_string())
      }))
      .push_column(5, header_text("User ID"), Box::new(|t|
        cell_text(t.source.user_id.to_string())
      ))
      .push_column(25, header_text("Enabled"), Box::new(|t| {
        let id = t.source.id;
        Checkbox::new(t.source.enabled, "Enabled", move |e| Message::SetSpotifySourceEnabled(id, e)).into()
      }))
      .push_column(25, header_text("Sync"), Box::new(move |t| {
        let id = t.source.id;
        Button::new(&mut t.sync_button_state, Text::new("Sync"))
          .on_press_into(move || Message::RequestSpotifySourceSync(id), !syncing)
      }))
      .build(&mut self.rows_scrollable_state)
      .into();

    Column::new()
      .width(Length::Fill)
      .align_items(Align::Center)
      .spacing(2)
      .push(Row::new()
        .spacing(2)
        .push(h2("Local sources"))
        .push(Button::new(&mut self.sync_button_state, Text::new("Sync local sources"))
          .on_press_into(|| Message::RequestLocalSourcesSync, !syncing)
        )
      )
      .push(Rule::horizontal(1))
      .push(table)
      .into()
  }
}

#[derive(Debug)]
pub struct SpotifySourceViewModel {
  source: SpotifySource,
  sync_button_state: button::State,
}

impl<'a> From<SpotifySource> for SpotifySourceViewModel {
  fn from(source: SpotifySource) -> Self { Self { source, sync_button_state: button::State::default() } }
}
