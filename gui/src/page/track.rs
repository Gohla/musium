#![allow(dead_code, unused_imports, unused_variables)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use iced::{button, Button, Color, Column, Command, Element, Length, Row, scrollable, Text};
use iced_native::{HorizontalAlignment, Space, VerticalAlignment};
use itertools::Itertools;
use tracing::{debug, error, info};

use musium_audio::Player;
use musium_client::{Client, HttpRequestError, PlaySource};
use musium_core::format_error::FormatError;
use musium_core::model::collection::Tracks;
use musium_core::model::User;

use crate::util::Update;
use crate::widget::table::TableBuilder;

#[derive(Default, Debug)]
pub struct Page {
  logged_in_user: User,

  scrollable_state: scrollable::State,
  play_button_states: Arc<Mutex<HashMap<i32, button::State>>>,

  tracks: Tracks,
  list_tracks_state: ListTracksState,
}

#[derive(Clone, Debug)]
pub enum Message {
  RequestPlayTrack(i32),
  ReceiveTracks(Result<Tracks, Arc<HttpRequestError>>),
  ReceivePlaySource(Result<Option<PlaySource>, Arc<HttpRequestError>>),
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

  pub fn update(&mut self, client: &mut Client, audio_player: &mut Player, message: Message) -> Update<Message, Action> {
    match message {
      Message::RequestPlayTrack(id) => {
        let client = client.clone();
        return Update::command(Command::perform(
          async move { client.play_track_by_id(id).await },
          |r| Message::ReceivePlaySource(r.map_err(|e| Arc::new(e))),
        ));
      }
      Message::ReceiveTracks(result) => match result {
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
      },
      Message::ReceivePlaySource(result) => match result {
        Ok(Some(play_source)) => match play_source {
          PlaySource::AudioData(audio_data) => {
            if let Err(e) = audio_player.play(audio_data, 0.2) {
              let format_error = FormatError::new(&e);
              error!("Playing track failed: {:?}", format_error);
            } else {
              info!("Track played locally");
            }
          }
          PlaySource::ExternallyPlayed => {
            info!("Track played externally");
          }
        }
        Ok(none) => {
          error!("Received None play source");
        }
        Err(e) => {
          let format_error = FormatError::new(e.as_ref());
          error!("Receiving play source failed: {:?}", format_error);
        }
      }
    }
    Update::none()
  }

  pub fn view(&mut self) -> Element<'_, Message> {
    let play_button_states = self.play_button_states.clone();
    let table: Element<_> = TableBuilder::new(self.tracks.iter())
      .spacing(2)
      .header_row_height(26)
      .row_height(16)
      .push_column(5, empty(), Box::new(move |t|
        play_button(play_button_states.clone(), t.track.id)
      ))
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

fn play_button(button_states: Arc<Mutex<HashMap<i32, button::State>>>, track_id: i32) -> Element<'static, Message> {
  let mut button_states = button_states.lock().unwrap();
  Button::new(button_states.entry(track_id).or_default(), Text::new("Play"))
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
