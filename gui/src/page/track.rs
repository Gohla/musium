#![allow(dead_code, unused_imports, unused_variables)]

use std::sync::Arc;

use iced::{Color, Column, Command, Element, Length, Row, scrollable, Text};
use iced_native::{HorizontalAlignment, Space, VerticalAlignment};
use itertools::Itertools;
use tracing::{debug, error};

use musium_client::{Client, HttpRequestError};
use musium_core::format_error::FormatError;
use musium_core::model::collection::Tracks;
use musium_core::model::User;

use crate::util::Update;
use crate::widget::table::TableBuilder;

#[derive(Default, Debug)]
pub struct Page {
  logged_in_user: User,

  scrollable_state: scrollable::State,

  tracks: Tracks,
  list_tracks_state: ListTracksState,
}

#[derive(Clone, Debug)]
pub enum Message {
  TracksReceived(Result<Tracks, Arc<HttpRequestError>>),
}

pub enum Action {}

#[derive(Debug)]
enum ListTracksState { Idle, Busy, Failed(Arc<HttpRequestError>) }

impl Default for ListTracksState { fn default() -> Self { Self::Idle } }

impl Page {
  pub fn new(logged_in_user: User, client: &mut Client) -> (Self, Command<Message>) {
    let mut page = Self {
      logged_in_user,
      ..Self::default()
    };
    let command = page.update_tracks(client);
    (page, command)
  }

  pub fn update(&mut self, _client: &mut Client, message: Message) -> Update<Message, Action> {
    match message {
      Message::TracksReceived(result) => match result {
        Ok(tracks) => {
          debug!("Received {} tracks", tracks.len());
          self.tracks = tracks;
          self.list_tracks_state = ListTracksState::Idle;
        }
        Err(e) => {
          let format_error = FormatError::new(e.as_ref());
          error!("Receiving tracks failed: {:?}", format_error);
          self.list_tracks_state = ListTracksState::Failed(e);
        }
      }
    }
    Update::none()
  }

  pub fn view(&mut self) -> Element<'_, Message> {
    let table: Element<_> = TableBuilder::new(self.tracks.iter())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
      .push_column(5, header_text("#"), Box::new(|t|
        if let Some(track_number) = t.track.track_number { cell_text(track_number.to_string()) } else { empty() }
      ))
      .push_column(25, header_text("Title"), Box::new(|t|
        cell_text(t.track.title.clone())
      ))
      .push_column(25, header_text("Track Artists"), Box::new(|t|
        cell_text(t.track_artists().map(|a| a.name.clone()).join(", "))
      ))
      .push_column(25, header_text("Album"), Box::new(|t|
        if let Some(album) = t.album() { cell_text(album.name.clone()) } else { empty() }
      ))
      .push_column(25, header_text("Album Artists"), Box::new(|t|
        cell_text(t.album_artists().map(|a| a.name.clone()).join(", "))
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

  fn update_tracks(&mut self, client: &mut Client) -> Command<Message> {
    self.list_tracks_state = ListTracksState::Busy;
    let client = client.clone();
    Command::perform(
      async move { client.list_tracks().await },
      |r| Message::TracksReceived(r.map_err(|e| Arc::new(e))),
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
