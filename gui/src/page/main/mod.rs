#![allow(dead_code, unused_imports, unused_variables)]

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use iced::{self, button, Button, Checkbox, Color, Column, Command, Element, Length, Row, Rule, rule, scrollable, Subscription, Text};
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

  is_paused: bool,
  is_stopped: bool,

  prev_track_button_state: button::State,
  stop_button_state: button::State,
  toggle_play_button_state: button::State,
  next_track_button_state: button::State,
}

#[derive(Debug)]
pub enum Message<P: Player> {
  TrackTab(track::Message<P>),
  SourceTab(source::Message<P>),
  SetCurrentTab(Tab),
  RequestPrevTrack,
  RequestStop,
  ReceiveStop(Result<(), <<P as Player>::AudioOutput as AudioOutput>::StopError>),
  RequestTogglePlay,
  ReceiveTogglePlay(Result<bool, <<P as Player>::AudioOutput as AudioOutput>::TogglePlayError>),
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

pub enum Action {
  ReceivePlay
}

impl<'a> Page {
  pub fn new<P: Player>(logged_in_user: User, player: &P) -> (Self, Command<Message<P>>) {
    let (track_tab, track_tab_command) = track::Tab::new(player);
    let (source_tab, source_tab_command) = source::Tab::new(player);
    let page = Self {
      logged_in_user,
      is_paused: false,
      is_stopped: true,
      ..Self::default()
    };
    let command = Command::batch(vec![
      track_tab_command.map(|m| Message::TrackTab(m)),
      source_tab_command.map(|m| Message::SourceTab(m)),
    ]);
    (page, command)
  }

  pub fn update<P: Player>(&mut self, player: &P, message: Message<P>) -> Command<Message<P>> {
    use Message::*;
    match message {
      TrackTab(m) => {
        let (command, action) = self.track_tab.update(player, m).unwrap();
        self.handle_action(action);
        return command.map(|m| TrackTab(m));
      }
      SourceTab(m) => {
        let (command, action) = self.source_tab.update(player, m).unwrap();
        self.handle_action(action);
        return command.map(|m| SourceTab(m));
      }
      SetCurrentTab(tab) => self.current_tab = tab,

      RequestStop => {
        let player = player.clone();
        return Command::perform(
          async move { player.get_audio_output().stop().await },
          |r| ReceiveStop(r),
        );
      }
      ReceiveStop(r) => match r {
        Ok(_) => {
          self.is_paused = false;
          self.is_stopped = true;
        }
        Err(e) => error!("Failed to stop playback: {:?}", FormatError::new(&e)),
      }
      RequestTogglePlay => {
        let player = player.clone();
        return Command::perform(
          async move { player.get_audio_output().toggle_play().await },
          |r| ReceiveTogglePlay(r),
        );
      }
      ReceiveTogglePlay(r) => match r {
        Ok(is_playing) => {
          self.is_paused = !is_playing;
          self.is_stopped = false;
        }
        Err(e) => error!("Failed to toggle playback: {:?}", FormatError::new(&e)),
      }
      // RequestPrevTrack => {}
      // RequestNextTrack => {}
      m => debug!("Unhandled message: {:?}", m)
    };
    Command::none()
  }

  pub fn handle_action(&mut self, action: Option<Action>) {
    if let Some(action) = action {
      match action {
        Action::ReceivePlay => {
          self.is_paused = false;
          self.is_stopped = false;
        }
      }
    }
  }

  pub fn subscription<P: Player>(&self, player: &P) -> Subscription<Message<P>> {
    self.source_tab.subscription(player).map(|m| Message::SourceTab(m))
  }

  pub fn view<P: Player>(&'a mut self) -> Element<'a, Message<P>> {
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
        .on_press_into(move || Message::RequestPrevTrack, !self.is_stopped))
      .push(Button::new(&mut self.stop_button_state, Text::new("Stop"))
        .on_press_into(move || Message::RequestStop, !self.is_stopped))
      .push(Button::new(&mut self.toggle_play_button_state, Text::new("Play/pause"))
        .on_press_into(move || Message::RequestTogglePlay, !self.is_stopped))
      .push(Button::new(&mut self.next_track_button_state, Text::new("Next track"))
        .on_press_into(move || Message::RequestNextTrack, !self.is_stopped))
      ;
    let content: Element<_> = Column::new()
      .width(Length::Fill)
      .height(Length::Fill)
      .padding(4)
      .spacing(4)
      .push(tabs)
      .push(horizontal_line())
      .push(current_tab)
      .push(horizontal_line())
      .push(Column::new().width(Length::Fill).align_items(Align::Center).push(player_controls))
      .into();
    content//.explain([0.5, 0.5, 0.5])
  }
}

// Common widget functions

fn h1(label: impl Into<String>) -> Text { Text::new(label).size(32) }

fn h2(label: impl Into<String>) -> Text { Text::new(label).size(28) }

fn h3(label: impl Into<String>) -> Text { Text::new(label).size(24) }

fn h4(label: impl Into<String>) -> Text { Text::new(label).size(20) }

fn txt(label: impl Into<String>) -> Text { Text::new(label).size(16) }

fn header_text<'a, M>(label: impl Into<String>) -> Element<'a, M> {
  h4(label)
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

fn cell_checkbox<'a, M: 'a>(is_checked: bool, message_fn: impl 'static + Fn(bool) -> M) -> Element<'a, M> {
  Checkbox::new(is_checked, "", message_fn)
    .into()
}

fn cell_button<'a, M: 'static>(state: &'a mut button::State, label: impl Into<String>, enabled: bool, message_fn: impl 'static + Fn() -> M) -> Element<'a, M> {
  Button::new(state, txt(label))
    .padding(1)
    .on_press_into(message_fn, enabled)
}

fn horizontal_line<M: 'static>() -> Element<'static, M> {
  Rule::horizontal(1)
    .style(HorizontalLine)
    .into()
}

struct HorizontalLine;

impl rule::StyleSheet for HorizontalLine {
  fn style(&self) -> rule::Style {
    rule::Style {
      color: [0.6, 0.6, 0.6, 0.6].into(),
      width: 1,
      radius: 0.0,
      fill_mode: rule::FillMode::Full,
    }
  }
}

fn empty<'a, M: 'a>() -> Element<'a, M> {
  Space::new(Length::Shrink, Length::Shrink).into()
}
