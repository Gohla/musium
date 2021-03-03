#![allow(dead_code, unused_imports, unused_variables)]

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use iced::{button, Button, Color, Column, Command, Element, Length, Row, Rule, scrollable, Text};
use iced_native::{Align, HorizontalAlignment, Space, VerticalAlignment};
use itertools::Itertools;
use tracing::{debug, error, info};

use musium_core::format_error::FormatError;
use musium_core::model::{Album, Track, User};
use musium_core::model::collection::{TrackInfo, Tracks};
use musium_player::*;

use crate::page::main::tracks::TrackViewModel;
use crate::util::{ButtonEx, Update};
use crate::widget::table::TableBuilder;

mod tracks;
mod source;

#[derive(Default, Debug)]
pub struct Page {
  logged_in_user: User,

  tracks_tab: tracks::Tab,

  refresh_library_button_state: button::State,
  refreshing_library: bool,

  prev_track_button_state: button::State,
  playpause_button_state: button::State,
  next_track_button_state: button::State,
}

#[derive(Debug)]
pub enum Message {
  RequestLibraryRefresh,
  ReceiveLibraryRefresh(Result<Vec<TrackViewModel>, RefreshLibraryFail>),
  RequestPlayTrack(i32),
  ReceivePlayResult(Result<(), PlayError>),
  RequestPrevTrack,
  RequestPlayPause,
  RequestNextTrack,
}

pub enum Action {}

impl<'a> Page {
  pub fn new(logged_in_user: User, player: &Player) -> (Self, Command<Message>) {
    let mut page = Self {
      logged_in_user,
      ..Self::default()
    };
    let command = page.refresh_library(player);
    (page, command)
  }

  pub fn update(&mut self, player: &Player, message: Message) -> Update<Message, Action> {
    match message {
      Message::RequestLibraryRefresh => {
        self.refreshing_library = true;
        return Update::command(self.refresh_library(player));
      }
      Message::ReceiveLibraryRefresh(result) => {
        match result {
          Ok(track_view_models) => {
            debug!("Received {} tracks", track_view_models.len());
            self.tracks_tab.update_tracks(track_view_models);
          }
          Err(e) => {
            let format_error = FormatError::new(&e);
            error!("Receiving tracks failed: {:?}", format_error);
          }
        }
        self.refreshing_library = false;
      }
      Message::RequestPlayTrack(id) => {
        let player = player.clone();
        return Update::command(Command::perform(
          async move { player.play_track_by_id(id, 0.1).await },
          |r| Message::ReceivePlayResult(r),
        ));
      }
      Message::ReceivePlayResult(result) => match result {
        Ok(_) => {
          debug!("Track played successfully");
        }
        Err(e) => {
          let format_error = FormatError::new(&e);
          error!("Playing track failed: {:?}", format_error);
        }
      }
      m => debug!("Unhandled message: {:?}", m)
    }
    Update::none()
  }

  pub fn view(&'a mut self) -> Element<'a, Message> {
    let top = Row::new()
      .spacing(2)
      .width(Length::Fill)
      .push(Row::new()
        .width(Length::Fill)
        .push(Text::new("Musium").color([0.5, 0.5, 0.5]))
        .push(Text::new("|"))
        .push(Text::new("all tracks"))
      )
      .push(Row::new()
        .width(Length::Shrink)
        .align_items(Align::End)
        .push(Button::new(&mut self.refresh_library_button_state, Text::new("Refresh library"))
          .on_press_into(|| Message::RequestLibraryRefresh, !self.refreshing_library)
        )
      )
      ;
    let content = self.tracks_tab.view();
    let player_controls = Row::new()
      .spacing(2)
      .push(Button::new(&mut self.prev_track_button_state, Text::new("Prev track"))
        .on_press_into(|| Message::RequestPrevTrack, true))
      .push(Button::new(&mut self.playpause_button_state, Text::new("Play/pause"))
        .on_press_into(|| Message::RequestPlayPause, true))
      .push(Button::new(&mut self.next_track_button_state, Text::new("Next track"))
        .on_press_into(|| Message::RequestNextTrack, true))
      ;
    let content: Element<_> = Column::new()
      .width(Length::Fill)
      .height(Length::Fill)
      .align_items(Align::Center)
      .padding(4)
      .spacing(4)
      .push(top)
      .push(Rule::horizontal(1))
      .push(content)
      .push(Rule::horizontal(1))
      .push(player_controls)
      .into();
    content//.explain([0.5, 0.5, 0.5])
  }

  fn refresh_library(&mut self, player: &Player) -> Command<Message> {
    let player = player.clone();
    Command::perform(
      async move {
        let library_ref = player.refresh_library().await?;
        let tracks_view_models: Vec<_> = library_ref.iter().map(|ti| ti.into()).collect();
        Ok(tracks_view_models)
      },
      |r| Message::ReceiveLibraryRefresh(r),
    )
  }
}

// Common widget functions

fn h1(label: impl Into<String>) -> Text { Text::new(label).size(36) }
fn h2(label: impl Into<String>) -> Text { Text::new(label).size(32) }
fn h3(label: impl Into<String>) -> Text { Text::new(label).size(28) }
fn h4(label: impl Into<String>) -> Text { Text::new(label).size(24) }
fn h5(label: impl Into<String>) -> Text { Text::new(label).size(20) }
fn txt(label: impl Into<String>) -> Text { Text::new(label).size(16) }

fn header_text<'a, M>(label: impl Into<String>) -> Element<'a, M> {
  h3(label)
    .width(Length::Fill)
    .height(Length::Fill)
    .horizontal_alignment(HorizontalAlignment::Left)
    .vertical_alignment(VerticalAlignment::Center)
    .into()
}

fn cell_text<'a, M>(label: impl Into<String>) -> Element<'a, M> {
  txt(label)
    .width(Length::Fill)
    .height(Length::Fill)
    .horizontal_alignment(HorizontalAlignment::Left)
    .vertical_alignment(VerticalAlignment::Center)
    .into()
}

fn empty<'a, M: 'a>() -> Element<'a, M> {
  Space::new(Length::Shrink, Length::Shrink).into()
}


