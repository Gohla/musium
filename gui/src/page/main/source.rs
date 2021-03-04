use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

use iced::{Align, button, Button, Column, Element, Length, Row, Rule, scrollable, Text};

use musium_core::model::{LocalSource, SpotifySource};
use musium_player::Player;

use crate::page::main::{cell_text, h1, h2, header_text};
use crate::util::{ButtonEx, Update};
use crate::widget::table::TableBuilder;

#[derive(Default, Debug)]
pub struct Tab {
  local_sources: Rc<RefCell<Vec<LocalSourceViewModel>>>,
  local_sources_scrollable_state: scrollable::State,
  spotify_sources: Rc<RefCell<Vec<SpotifySourceViewModel>>>,
  spotify_sources_scrollable_state: scrollable::State,

  refreshing: bool,
  refresh_button_state: button::State,

  syncing: bool,
  sync_all_button_state: button::State,
  sync_local_sources_button_state: button::State,
  sync_spotify_sources_button_state: button::State,
}

#[derive(Debug)]
pub enum Message {
  RequestRefresh,
  ReceiveRefresh(Result<(Vec<LocalSourceViewModel>, Vec<SpotifySourceViewModel>), dyn Error>),

  RequestSync,
  RequestLocalSourcesSync,
  RequestLocalSourceSync(i32),
  RequestSpotifySourcesSync,
  RequestSpotifySourceSync(i32),
  ReceiveSyncResult,
}

impl<'a> Tab {
  pub fn update_local_sources(&mut self, local_sources: Vec<LocalSourceViewModel>) {
    self.local_sources = Rc::new(RefCell::new(local_sources));
  }

  pub fn update_spotify_sources(&mut self, spotify_sources: Vec<SpotifySourceViewModel>) {
    self.spotify_sources = Rc::new(RefCell::new(spotify_sources));
  }

  pub fn update(&mut self, player: &Player, message: Message) -> Update<Message, super::Action> {
    match message {
      Message::RequestRefresh => {}
      Message::ReceiveRefresh(r) => {}
      Message::RequestSync => {}
      Message::RequestLocalSourcesSync => {}
      Message::RequestLocalSourceSync(_) => {}
      Message::RequestSpotifySourcesSync => {}
      Message::RequestSpotifySourceSync(_) => {}
      Message::ReceiveSyncResult => {}
    }
    Update::none()
  }

  pub fn view(&'a mut self) -> Element<'a, Message> {
    let header = Row::new()
      .width(Length::Fill)
      .align_items(Align::Center)
      .spacing(2)
      .push(h1("Sources"))
      .push(Button::new(&mut self.refresh_button_state, Text::new("Refresh all sources"))
        .on_press_into(|| Message::RequestRefresh, !self.refreshing))
      .push(Button::new(&mut self.sync_all_button_state, Text::new("Sync all sources")
        .on_press_into(|| Message::RequestSync, !self.syncing),
      ))
      ;
    let local_sources = self.view_local_sources();
    let spotify_sources = self.view_spotify_sources();

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

  fn view_local_sources(&'a mut self) -> Element<'a, Message> {
    let table = TableBuilder::new(self.local_sources.clone())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
      .push_column(5, header_text("ID"), Box::new(move |t| {
        cell_text(t.source.id)
      }))
      .push_column(5, header_text("Directory"), Box::new(|t|
        cell_text(t.source.directory.clone())
      ))
      .push_column(25, header_text("Enabled"), Box::new(|t|
        // TODO: change into toggle and send messages
        cell_text(t.source.enabled)
      ))
      .push_column(25, header_text("Sync"), Box::new(|t|
        Button::new(state, Text::new("Sync"))
          .on_press_into(move || Message::RequestLocalSourceSync(t.source.id), !self.syncing)
      ))
      .build(&mut self.spotify_sources_scrollable_state)
      .into();

    Column::new()
      .width(Length::Fill)
      .align_items(Align::Center)
      .spacing(2)
      .push(Row::new()
        .spacing(2)
        .push(h2("Local sources"))
        .push(Button::new(&mut self.sync_local_sources_button_state, Text::new("Sync local sources"))
          .on_press_into(|| Message::RequestLocalSourcesSync, !self.syncing)
        )
      )
      .push(Rule::horizontal(1))
      .push(table)
      .into()
  }

  fn view_spotify_sources(&'a mut self) -> Element<'a, Message> {
    let table = TableBuilder::new(self.spotify_sources.clone())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
      .push_column(5, header_text("ID"), Box::new(move |t| {
        cell_text(t.source.id)
      }))
      .push_column(5, header_text("User ID"), Box::new(|t|
        cell_text(t.source.user_id)
      ))
      .push_column(25, header_text("Enabled"), Box::new(|t|
        // TODO: change into toggle and send messages
        cell_text(t.source.enabled)
      ))
      .push_column(25, header_text("Sync"), Box::new(|t|
        Button::new(state, Text::new("Sync"))
          .on_press_into(move || Message::RequestSpotifySourceSync(t.source.id), !self.syncing)
      ))
      .build(&mut self.scrollable_state)
      .into();

    Column::new()
      .width(Length::Fill)
      .align_items(Align::Center)
      .spacing(2)
      .push(Row::new()
        .spacing(2)
        .push(h2("Local sources"))
        .push(Button::new(&mut self.sync_spotify_sources_button_state, Text::new("Sync local sources"))
          .on_press_into(|| Message::RequestLocalSourcesSync, !self.syncing)
        )
      )
      .push(Rule::horizontal(1))
      .push(table)
      .into()
  }
}

// View models

#[derive(Default, Debug)]
pub struct LocalSourceViewModel {
  source: LocalSource,
  sync_button_state: button::State,
}

impl<'a> From<LocalSource> for LocalSourceViewModel {
  fn from(source: LocalSource) -> Self { Self { source, ..Self::default() } }
}

#[derive(Default, Debug)]
pub struct SpotifySourceViewModel {
  source: SpotifySource,
  sync_button_state: button::State,
}

impl<'a> From<SpotifySource> for SpotifySourceViewModel {
  fn from(source: SpotifySource) -> Self { Self { source, ..Self::default() } }
}
