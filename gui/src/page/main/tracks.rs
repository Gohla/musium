use std::cell::RefCell;
use std::rc::Rc;

use iced::{button, Button, Element, HorizontalAlignment, Length, scrollable, Space, Text, VerticalAlignment};
use itertools::Itertools;

use musium_core::model::collection::TrackInfo;

use crate::page::main::Message;
use crate::util::ButtonEx;
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
pub struct Tab {
  tracks: Rc<RefCell<Vec<TrackViewModel>>>,
  scrollable_state: scrollable::State,
}

impl<'a> Tab {
  pub fn update_tracks(&mut self, track_view_models: Vec<TrackViewModel>) {
    self.tracks = Rc::new(RefCell::new(track_view_models));
  }

  pub fn view(&'a mut self) -> Element<'a, super::Message> {
    TableBuilder::new(self.tracks.clone())
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
      .into()
  }
}

fn play_button<'a>(state: &'a mut button::State, track_id: i32) -> Element<'a, super::Message> {
  Button::new(state, Text::new("Play"))
    .on_press_into(move || super::Message::RequestPlayTrack(track_id), true)
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
  Space::new(Length::Shrink, Length::Shrink).into()
}
