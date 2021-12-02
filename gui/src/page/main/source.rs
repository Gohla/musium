use std::cell::RefCell;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use iced::{Align, button, Button, Checkbox, Column, Command, Element, Length, Row, Rule, scrollable, Subscription, Text};
use iced::futures::{self, stream::BoxStream};
use iced_native::subscription::Recipe;
use itertools::Itertools;
use tracing::{debug, error};

use musium_core::api::SyncStatus;
use musium_core::format_error::FormatError;
use musium_core::model::{LocalSource, SpotifySource};
use musium_player::{Client, HttpRequestError, Player};

use crate::page::main::{cell_button, cell_checkbox, cell_text, h1, h2, header_text, horizontal_line};
use crate::util::{ButtonEx, Update};
use crate::widget::table::TableBuilder;

#[derive(Default, Debug)]
pub struct Tab {
  local_sources: LocalSources,
  spotify_sources: SpotifySources,

  refreshing: bool,
  refresh_button_state: button::State,

  syncing: bool,
  sync_subscription_active: bool,
  sync_all_button_state: button::State,
}

#[derive(Debug)]
pub enum Message<P: Player> {
  RequestRefresh,
  ReceiveRefresh(Result<Vec<LocalSourceViewModel>, <P::Client as Client>::LocalSourceError>, Result<Vec<SpotifySourceViewModel>, <P::Client as Client>::SpotifySourceError>),

  RequestSetLocalSourceEnabled(i32, bool),
  ReceiveSetLocalSourceEnabled(Result<Option<LocalSource>, <P::Client as Client>::LocalSourceError>, i32, bool),
  RequestSetSpotifySourceEnabled(i32, bool),
  ReceiveSetSpotifySourceEnabled(Result<Option<SpotifySource>, <P::Client as Client>::SpotifySourceError>, i32, bool),

  RequestSync,
  RequestLocalSourcesSync,
  RequestLocalSourceSync(i32),
  RequestSpotifySourcesSync,
  RequestSpotifySourceSync(i32),
  ReceiveSyncStatus(Result<SyncStatus, <P::Client as Client>::SyncError>),
}

impl<'a> Tab {
  pub fn new<P: Player>(player: &P) -> (Self, Command<Message<P>>) {
    let mut tab = Self {
      ..Self::default()
    };
    let command = tab.refresh(player);
    (tab, command)
  }

  pub fn update<P: Player>(&mut self, player: &P, message: Message<P>) -> Update<Message<P>, super::Action> {
    use Message::*;
    match message {
      RequestRefresh => {
        return Update::command(self.refresh(player));
      }
      ReceiveRefresh(rl, rs) => {
        self.refreshing = false;
        match rl {
          Ok(sources) => {
            debug!("Received {} local sources", sources.len());
            self.local_sources.update(sources);
          }
          Err(e) => error!("Receiving local sources failed: {:?}", FormatError::new(&e)),
        };
        match rs {
          Ok(sources) => {
            debug!("Received {} Spotify sources", sources.len());
            self.spotify_sources.update(sources);
          }
          Err(e) => error!("Receiving Spotify sources failed: {:?}", FormatError::new(&e)),
        };
      }

      RequestSetLocalSourceEnabled(local_source_id, enabled) => {
        let player = player.clone();
        return Update::command(Command::perform(async move {
          player.get_client().set_local_source_enabled_by_id(local_source_id, enabled).await
        }, move |r| ReceiveSetLocalSourceEnabled(r, local_source_id, enabled)));
      }
      ReceiveSetLocalSourceEnabled(result, local_source_id, enabled) => {
        let local_source_sting = format!("local source with ID '{}'", local_source_id);
        match result {
          Ok(Some(source)) => {
            let mut guard = self.local_sources.sources.borrow_mut();
            if let Some(mut local_source_view_model) = guard.iter_mut().find(|s| s.source.id == local_source_id) {
              local_source_view_model.source.enabled = enabled;
              debug!("{} {}", if enabled { "Enabled" } else { "Disabled" }, local_source_sting);
            } else {
              error!("Failed to {} {}; it was not found in the GUI", enable_str(enabled), local_source_sting);
            }
          }
          Ok(None) => error!("Failed to {} {}; it was not found", enable_str(enabled), local_source_sting),
          Err(e) => error!("Failed to {} {}; an unexpected error occurred: {:?}", enable_str(enabled), local_source_sting, FormatError::new(&e)),
        };
      }
      RequestSetSpotifySourceEnabled(spotify_source_id, enabled) => {
        let player = player.clone();
        return Update::command(Command::perform(async move {
          player.get_client().set_spotify_source_enabled_by_id(spotify_source_id, enabled).await
        }, move |r| ReceiveSetSpotifySourceEnabled(r, spotify_source_id, enabled)));
      }
      ReceiveSetSpotifySourceEnabled(result, spotify_source_id, enabled) => {
        let spotify_source_sting = format!("Spotify source with ID '{}'", spotify_source_id);
        match result {
          Ok(Some(source)) => {
            let mut guard = self.spotify_sources.sources.borrow_mut();
            if let Some(mut spotify_source_view_model) = guard.iter_mut().find(|s| s.source.id == spotify_source_id) {
              spotify_source_view_model.source.enabled = enabled;
              debug!("{} {}", if enabled { "Enabled" } else { "Disabled" }, spotify_source_sting);
            } else {
              error!("Failed to {} {}; it was not found in the GUI", enable_str(enabled), spotify_source_sting);
            }
          }
          Ok(None) => error!("Failed to {} {}; it was not found", enable_str(enabled), spotify_source_sting),
          Err(e) => error!("Failed to {} {}; an unexpected error occurred: {:?}", enable_str(enabled), spotify_source_sting, FormatError::new(&e)),
        };
      }

      RequestSync => {
        self.syncing = true;
        let player = player.clone();
        return Update::command(Command::perform(async move {
          player.get_client().sync_all_sources().await
        }, move |r| ReceiveSyncStatus(r)));
      }
      RequestLocalSourcesSync => {
        self.syncing = true;
        let player = player.clone();
        return Update::command(Command::perform(async move {
          player.get_client().sync_local_sources().await
        }, move |r| ReceiveSyncStatus(r)));
      }
      RequestLocalSourceSync(local_source_id) => {
        self.syncing = true;
        let player = player.clone();
        return Update::command(Command::perform(async move {
          player.get_client().sync_local_source(local_source_id).await
        }, move |r| ReceiveSyncStatus(r)));
      }
      RequestSpotifySourcesSync => {
        self.syncing = true;
        let player = player.clone();
        return Update::command(Command::perform(async move {
          player.get_client().sync_spotify_sources().await
        }, move |r| ReceiveSyncStatus(r)));
      }
      RequestSpotifySourceSync(spotify_source_id) => {
        self.syncing = true;
        let player = player.clone();
        return Update::command(Command::perform(async move {
          player.get_client().sync_spotify_source(spotify_source_id).await
        }, move |r| ReceiveSyncStatus(r)));
      }
      ReceiveSyncStatus(result) => {
        self.sync_subscription_active = true;
        match result {
          Ok(sync_status) => {
            debug!("Received sync status: {}", sync_status);
            match sync_status {
              SyncStatus::Idle | SyncStatus::Completed | SyncStatus::Failed => {
                self.syncing = false;
                self.sync_subscription_active = false;
              }
              _ => self.syncing = true,
            }
          }
          Err(e) => {
            error!("Requesting sync failed unexpectedly: {:?}", FormatError::new(&e));
            self.syncing = false;
            self.sync_subscription_active = false;
          }
        };
      }
    }
    Update::none()
  }

  pub fn subscription<P: Player>(&self, player: &P) -> Subscription<Message<P>> {
    if self.sync_subscription_active {
      let player = player.clone();
      Subscription::from_recipe(Sync { player }).map(|r| Message::ReceiveSyncStatus(r))
    } else {
      Subscription::none()
    }
  }

  pub fn view<P: Player>(&'a mut self) -> Element<'a, Message<P>> {
    let header = Row::new()
      .spacing(2)
      .width(Length::Fill)
      .align_items(Align::Center)
      .push(Row::new()
        .width(Length::Fill)
        .align_items(Align::Center)
        .push(h1("Sources"))
      )
      .push(Row::new()
        .push(Button::new(&mut self.refresh_button_state, Text::new("Refresh")).on_press_into(|| Message::RequestRefresh, !self.refreshing))
        .push(Button::new(&mut self.sync_all_button_state, Text::new("Sync all")).on_press_into(|| Message::RequestSync, !self.syncing))
      )
      ;
    let local_sources = self.local_sources.view(self.syncing);
    let spotify_sources = self.spotify_sources.view(self.syncing);
    Column::new()
      .width(Length::Fill)
      .height(Length::Fill)
      .spacing(4)
      .align_items(Align::Center)
      .push(header)
      .push(horizontal_line())
      .push(local_sources)
      .push(horizontal_line())
      .push(spotify_sources)
      .into()
  }

  fn refresh<P: Player>(&mut self, player: &P) -> Command<Message<P>> {
    self.refreshing = true;
    let player = player.clone();
    Command::perform(
      async move {
        let local_sources = player.clone().get_client().list_local_sources().await
          .map(|s| s.into_iter().map(|s| s.into()).collect_vec());
        let spotify_sources = player.get_client().list_spotify_sources().await
          .map(|s| s.into_iter().map(|s| s.into()).collect_vec());
        (local_sources, spotify_sources)
      },
      |(l, s)| Message::ReceiveRefresh(l, s),
    )
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

  fn view<P: Player>(&'a mut self, syncing: bool) -> Element<'a, Message<P>> {
    let header = Row::new()
      .spacing(2)
      .width(Length::Fill)
      .align_items(Align::Center)
      .push(Row::new()
        .width(Length::Fill)
        .align_items(Align::Center)
        .push(h2("Local sources"))
      )
      .push(Row::new()
        .push(Button::new(&mut self.sync_button_state, Text::new("Sync all local sources"))
          .on_press_into(move || Message::RequestLocalSourcesSync, !syncing)
        )
      )
      ;
    let table: Element<_> = TableBuilder::new(self.sources.clone())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
      .push_column(5, header_text("ID"), Box::new(|t| {
        cell_text(t.source.id.to_string())
      }))
      .push_column(25, header_text("Directory"), Box::new(|t|
        cell_text(t.source.directory.clone())
      ))
      .push_column(5, header_text("Enabled"), Box::new(|t| {
        let id = t.source.id;
        cell_checkbox(t.source.enabled, move |e| Message::RequestSetLocalSourceEnabled(id, e))
      }))
      .push_column(5, header_text("Sync"), Box::new(move |t| {
        let id = t.source.id;
        cell_button(&mut t.sync_button_state, "Sync", !syncing, move || Message::RequestLocalSourceSync(id))
      }))
      .build(&mut self.rows_scrollable_state)
      .into();
    Column::new()
      .width(Length::Fill)
      .height(Length::Fill)
      .spacing(4)
      .align_items(Align::Center)
      .push(header)
      .push(horizontal_line())
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

  fn view<P: Player>(&'a mut self, syncing: bool) -> Element<'a, Message<P>> {
    let header = Row::new()
      .spacing(2)
      .width(Length::Fill)
      .align_items(Align::Center)
      .push(Row::new()
        .width(Length::Fill)
        .align_items(Align::Center)
        .push(h2("Spotify sources"))
      )
      .push(Row::new()
        .push(Button::new(&mut self.sync_button_state, Text::new("Sync all Spotify sources"))
          .on_press_into(move || Message::RequestSpotifySourcesSync, !syncing)
        )
      )
      ;
    let table: Element<_> = TableBuilder::new(self.sources.clone())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
      .push_column(5, header_text("ID"), Box::new(|t| {
        cell_text(t.source.id.to_string())
      }))
      .push_column(25, header_text("User ID"), Box::new(|t|
        cell_text(t.source.user_id.to_string())
      ))
      .push_column(5, header_text("Enabled"), Box::new(|t| {
        let id = t.source.id;
        cell_checkbox(t.source.enabled, move |e| Message::RequestSetSpotifySourceEnabled(id, e))
      }))
      .push_column(5, header_text("Sync"), Box::new(move |t| {
        let id = t.source.id;
        cell_button(&mut t.sync_button_state, "Sync", !syncing, move || Message::RequestSpotifySourceSync(id))
      }))
      .build(&mut self.rows_scrollable_state)
      .into();
    Column::new()
      .width(Length::Fill)
      .height(Length::Fill)
      .spacing(4)
      .align_items(Align::Center)
      .push(header)
      .push(horizontal_line())
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

// Sync subscription

struct Sync<P: Player> {
  player: P,
}

impl<H, I, P: Player> Recipe<H, I> for Sync<P> where
  H: Hasher
{
  type Output = Result<SyncStatus, <<P as Player>::Client as Client>::SyncError>;

  fn hash(&self, state: &mut H) {
    // Only one sync subscription may be active, so hash just the marker struct.
    struct Marker;
    std::any::TypeId::of::<Marker>().hash(state);
  }

  fn stream(self: Box<Self>, input: BoxStream<I>) -> BoxStream<Self::Output> {
    Box::pin(futures::stream::unfold((self.player, false, false), |(player, stop, delay)| async move {
      if stop {
        return None;
      }
      if delay {
        tokio::time::sleep(Duration::from_millis(1000)).await;
      }
      let sync_status_result = player.clone().get_client().get_sync_status().await;
      let stop = match sync_status_result {
        Ok(SyncStatus::Idle) | Ok(SyncStatus::Completed) | Ok(SyncStatus::Failed) | Err(_) => true,
        _ => false,
      };
      Some((sync_status_result, (player, stop, true)))
    }))
  }
}

// Utility

fn enable_str(enable: bool) -> &'static str {
  if enable { "enable" } else { "disable" }
}
