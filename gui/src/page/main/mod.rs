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

use crate::page::main::track::TrackViewModel;
use crate::util::{ButtonEx, Update};
use crate::widget::table::TableBuilder;

mod track;
mod source;

#[derive(Default, Debug)]
pub struct Page {
  logged_in_user: User,

  track_tab: track::Tab,
  track_tab_button_state: button::State,
  source_tab: source::Tab,
  source_tab_button_state: button::State,
  current_tab: Tab,

  prev_track_button_state: button::State,
  playpause_button_state: button::State,
  next_track_button_state: button::State,
}

#[derive(Debug)]
pub enum Message {
  TrackTab(track::Message),
  SourceTab(source::Message),
  SetCurrentTab(Tab),
  RequestPrevTrack,
  RequestPlayPause,
  RequestNextTrack,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Tab {
  Track,
  Source,
}

impl Default for Tab {
  fn default() -> Self { Self::Track }
}

pub enum Action {}

impl<'a> Page {
  pub fn new(logged_in_user: User, player: &Player) -> (Self, Command<Message>) {
    let (track_tab, track_tab_command) = track::Tab::new(player);
    let (source_tab, source_tab_command) = source::Tab::new(player);
    let page = Self {
      logged_in_user,
      ..Self::default()
    };
    let command = Command::batch(vec![
      track_tab_command.map(|m| Message::TrackTab(m)),
      source_tab_command.map(|m| Message::SourceTab(m)),
    ]);
    (page, command)
  }

  pub fn update(&mut self, player: &Player, message: Message) -> Update<Message, Action> {
    use Message::*;
    match message {
      TrackTab(m) => { return self.track_tab.update(player, m).map_command(|m| TrackTab(m)); }
      SourceTab(m) => { return self.source_tab.update(player, m).map_command(|m| SourceTab(m)); }
      SetCurrentTab(tab) => self.current_tab = tab,
      m => debug!("Unhandled message: {:?}", m)
    };
    Update::none()
  }

  pub fn view(&'a mut self) -> Element<'a, Message> {
    let tabs = Row::new()
      .spacing(2)
      .align_items(Align::Center)
      .push(Button::new(&mut self.track_tab_button_state, Text::new("Tracks"))
        .on_press_into(|| Message::SetCurrentTab(Tab::Track), self.current_tab != Tab::Track))
      .push(Button::new(&mut self.source_tab_button_state, Text::new("Sources"))
        .on_press_into(|| Message::SetCurrentTab(Tab::Source), self.current_tab != Tab::Source))
      ;
    let current_tab = match self.current_tab {
      Tab::Track => self.track_tab.view().map(|m| Message::TrackTab(m)),
      Tab::Source => self.source_tab.view().map(|m| Message::SourceTab(m)),
    };
    let player_controls = Row::new()
      .spacing(2)
      .align_items(Align::Center)
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
      .padding(4)
      .spacing(4)
      .push(tabs)
      .push(Rule::horizontal(1))
      .push(current_tab)
      .push(Rule::horizontal(1))
      .push(Column::new().width(Length::Fill).align_items(Align::Center).push(player_controls))
      .into();
    content.explain([0.5, 0.5, 0.5])
  }
}

// Common widget functions

fn h1(label: impl Into<String>) -> Text { Text::new(label).size(32) }

fn h2(label: impl Into<String>) -> Text { Text::new(label).size(28) }

fn h3(label: impl Into<String>) -> Text { Text::new(label).size(24) }

fn h4(label: impl Into<String>) -> Text { Text::new(label).size(20) }

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
