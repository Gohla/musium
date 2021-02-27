#![allow(dead_code, unused_imports, unused_variables)]

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use iced::{button, Button, Color, Column, Command, Element, Length, Row, scrollable, Text};
use iced_native::{HorizontalAlignment, Space, VerticalAlignment};
use itertools::Itertools;
use tracing::{debug, error, info};

use musium_core::format_error::FormatError;
use musium_core::model::{Album, Track, User};
use musium_core::model::collection::{TrackInfo, Tracks};
use musium_player::{Client, HttpRequestError, Player, PlayerT, RodioPlayError};
use musium_player::player::PlayError;

use crate::util::Update;
use crate::widget::table::TableBuilder;

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

#[derive(Default, Debug)]
pub struct Page {
  logged_in_user: User,

  scrollable_state: scrollable::State,

  tracks: Rc<RefCell<Vec<TrackViewModel>>>,
  list_tracks_state: ListTracksState,
}

#[derive(Clone, Debug)]
pub enum Message {
  RequestPlayTrack(i32),
  ReceiveTracks(Result<Tracks, Arc<HttpRequestError>>),
  ReceivePlayResult(Result<(), Arc<PlayError<HttpRequestError, RodioPlayError>>>),
}

pub enum Action {}

#[derive(Debug)]
enum ListTracksState { Idle, Busy, Failed(Arc<HttpRequestError>) }

impl Default for ListTracksState { fn default() -> Self { Self::Idle } }

impl<'a> Page {
  pub fn new(logged_in_user: User, player: &Player) -> (Self, Command<Message>) {
    let mut page = Self {
      logged_in_user,
      ..Self::default()
    };
    let command = page.update_tracks(player);
    (page, command)
  }

  pub fn update(&mut self, player: &Player, message: Message) -> Update<Message, Action> {
    match message {
      Message::RequestPlayTrack(id) => {
        let player = player.clone();
        return Update::command(Command::perform(
          async move { player.play_track_by_id(id, 0.1).await },
          |r| Message::ReceivePlayResult(r.map_err(|e| Arc::new(e))),
        ));
      }
      Message::ReceiveTracks(result) => match result {
        Ok(tracks) => {
          debug!("Received {} tracks", tracks.len());
          self.tracks = Rc::new(RefCell::new(tracks.iter().map(|ti| ti.into()).collect()));
          self.list_tracks_state = ListTracksState::Idle;
        }
        Err(e) => {
          let format_error = FormatError::new(e.as_ref());
          error!("Receiving tracks failed: {:?}", format_error);
          self.list_tracks_state = ListTracksState::Failed(e);
        }
      },
      Message::ReceivePlayResult(result) => match result {
        Ok(_) => {
          debug!("Track played successfully");
        }
        Err(e) => {
          let format_error = FormatError::new(e.as_ref());
          error!("Playing track failed: {:?}", format_error);
        }
      }
    }
    Update::none()
  }

  pub fn view(&'a mut self) -> Element<'a, Message> {
    let table: Element<_> = TableBuilder::new(self.tracks.clone())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
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
      .build(&mut self.scrollable_state)
      .into();
    let content: Element<_> = Column::new()
      .width(Length::Fill)
      .height(Length::Fill)
      .padding(4)
      .spacing(4)
      .push(table)
      .into();
    content
  }

  fn update_tracks(&mut self, player: &Player) -> Command<Message> {
    self.list_tracks_state = ListTracksState::Busy;
    let client = player.get_client().clone();
    Command::perform(
      async move { client.list_tracks().await },
      |r| Message::ReceiveTracks(r.map_err(|e| Arc::new(e))),
    )
  }
}

fn header_text<'a, M>(label: impl Into<String>) -> Element<'a, M> {
  Text::new(label)
    .width(Length::Fill)
    .height(Length::Fill)
    .horizontal_alignment(HorizontalAlignment::Left)
    .vertical_alignment(VerticalAlignment::Center)
    .size(26)
    .into()
}

fn play_button<'a>(state: &'a mut button::State, track_id: i32) -> Element<'a, Message> {
  Button::new(state, Text::new("Play"))
    .on_press(Message::RequestPlayTrack(track_id))
    .into()
}

fn cell_text<'a, M>(label: impl Into<String>) -> Element<'a, M> {
  Text::new(label)
    .width(Length::Fill)
    .height(Length::Fill)
    .horizontal_alignment(HorizontalAlignment::Left)
    .vertical_alignment(VerticalAlignment::Center)
    .size(16)
    .into()
}

fn empty<'a, M: 'a>() -> Element<'a, M> {
  Space::new(Length::Shrink, Length::Shrink)
    .into()
}
