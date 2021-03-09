use std::cell::RefCell;
use std::rc::Rc;

use iced::{Align, button, Button, Column, Command, Element, HorizontalAlignment, Length, Row, Rule, scrollable, Space, Text, VerticalAlignment};
use itertools::Itertools;
use tracing::{debug, error};

use musium_core::format_error::FormatError;
use musium_core::model::collection::{TrackInfo, Tracks};
use musium_core::panic::panic_into_string;
use musium_player::{Client, ClientT, Player, PlayError};

use crate::page::main::{cell_button, cell_text, empty, h1, header_text, horizontal_line};
use crate::util::{ButtonEx, Update};
use crate::widget::table::TableBuilder;

#[derive(Default, Debug)]
pub struct Tab {
  tracks: Rc<RefCell<Vec<TrackViewModel>>>,
  rows_scrollable_state: scrollable::State,

  refreshing: bool,
  refresh_button_state: button::State,
}

#[derive(Debug)]
pub enum Message {
  RequestRefresh,
  ReceiveRefresh(Result<Vec<TrackViewModel>, <Client as ClientT>::TrackError>),
  RequestPlayTrack(i32),
  ReceivePlayResult(Result<(), PlayError>),
}

impl<'a> Tab {
  pub fn new(player: &Player) -> (Self, Command<Message>) {
    let mut tab = Self {
      ..Self::default()
    };
    let command = tab.refresh(player);
    (tab, command)
  }

  pub fn update(&mut self, player: &Player, message: Message) -> Update<Message, super::Action> {
    match message {
      Message::RequestRefresh => {
        return Update::command(self.refresh(player));
      }
      Message::ReceiveRefresh(r) => {
        self.refreshing = false;
        match r {
          Ok(tracks) => {
            debug!("Received {} tracks", tracks.len());
            self.tracks = Rc::new(RefCell::new(tracks))
          }
          Err(e) => error!("Receiving tracks failed: {:?}", FormatError::new(&e)),
        };
      }
      Message::RequestPlayTrack(track_id) => {
        return Update::command(Self::play_track(track_id, player));
      }
      Message::ReceivePlayResult(r) => match r {
        r @ Ok(_) => {
          debug!("Track played successfully");
          return Update::action(super::Action::ReceivePlay);
        }
        Err(e) => error!("Playing track failed: {:?}", FormatError::new(&e)),
      }
    }
    Update::none()
  }

  pub fn view(&'a mut self) -> Element<'a, Message> {
    let header = Row::new()
      .spacing(2)
      .width(Length::Fill)
      .align_items(Align::Center)
      .push(Row::new()
        .width(Length::Fill)
        .align_items(Align::Center)
        .push(h1("Tracks"))
      )
      .push(Row::new()
        .push(Button::new(&mut self.refresh_button_state, Text::new("Refresh")).on_press_into(|| Message::RequestRefresh, !self.refreshing))
      )
      ;
    let table: Element<_> = TableBuilder::new(self.tracks.clone())
      .spacing(1)
      .header_row_height(27)
      .row_height(17)
      .push_column(5, empty(), Box::new(move |t| {
        play_button(&mut t.play_button_state, t.id)
      }))
      .push_column(5, header_text("#"), Box::new(|t|
        if let Some(track_number) = &t.track_number { cell_text(track_number) } else { empty() }
      ))
      .push_column(25, header_text("Title"), Box::new(|t|
        cell_text(t.title.clone())
      ))
      .push_column(25, header_text("Track Artists"), Box::new(|t|
        if let Some(track_artists) = &t.track_artists { cell_text(track_artists.clone()) } else { empty() }
      ))
      .push_column(25, header_text("Album"), Box::new(|t|
        if let Some(album) = &t.album { cell_text(album.clone()) } else { empty() }
      ))
      .push_column(25, header_text("Album Artists"), Box::new(|t|
        if let Some(album_artists) = &t.album_artists { cell_text(album_artists.clone()) } else { empty() }
      ))
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

  fn refresh(&mut self, player: &Player) -> Command<Message> {
    self.refreshing = true;
    let player = player.clone();
    Command::perform(
      async move {
        let tracks = player.get_client().list_tracks().await?;
        let tracks_view_models = tokio::task::spawn_blocking(move || {
          let tracks: Tracks = tracks.into();
          let tracks_view_models: Vec<_> = tracks.iter().map(|ti| ti.into()).collect();
          tracks_view_models
        }).await.unwrap_or_else(|e| {
          error!("Tracks view model creation task panicked; returning empty list of tracks. Panic was: {:?}", e.try_into_panic().map(|p| panic_into_string(p)));
          Vec::new()
        });
        Ok(tracks_view_models)
      },
      |r| Message::ReceiveRefresh(r),
    )
  }

  fn play_track(track_id: i32, player: &Player) -> Command<Message> {
    let player = player.clone();
    Command::perform(
      async move { player.play_track_by_id(track_id).await },
      |r| Message::ReceivePlayResult(r),
    )
  }
}

// View model

#[derive(Default, Debug)]
pub struct TrackViewModel {
  id: i32,
  play_button_state: button::State,
  track_number: Option<String>,
  title: String,
  track_artists: Option<String>,
  album: Option<String>,
  album_artists: Option<String>,
}

impl<'a> From<TrackInfo<'a>> for TrackViewModel {
  fn from(track_info: TrackInfo<'a>) -> Self {
    let track_artists = track_info.track_artists().map(|a| a.name.clone()).join(", ");
    let track_artists = if track_artists.is_empty() { None } else { Some(track_artists) };
    let album_artists = track_info.album_artists().map(|a| a.name.clone()).join(", ");
    let album_artists = if album_artists.is_empty() { None } else { Some(album_artists) };
    Self {
      id: track_info.track.id,
      track_number: track_info.track.track_number.map(|tn| tn.to_string()),
      title: track_info.track.title.clone(),
      track_artists,
      album: track_info.album().map(|a| a.name.clone()),
      album_artists,
      ..Self::default()
    }
  }
}

// Widget functions

fn play_button<'a>(state: &'a mut button::State, track_id: i32) -> Element<'a, Message> {
  cell_button(state, "Play", true, move || Message::RequestPlayTrack(track_id))
}
