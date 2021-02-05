use std::sync::Arc;

use iced::{Column, Command, Element, Length, Scrollable, scrollable, Text};
use tracing::{error, info};

use musium_client::{Client, HttpRequestError};
use musium_core::format_error::FormatError;
use musium_core::model::collection::Tracks;
use musium_core::model::User;

use crate::util::Update;
use crate::widget::table::Table;

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

  pub fn update(&mut self, client: &mut Client, message: Message) -> Update<Message, Action> {
    match message {
      Message::TracksReceived(result) => match result {
        Ok(tracks) => {
          self.tracks = tracks;
          self.list_tracks_state = ListTracksState::Idle;
        }
        Err(e) => {
          let format_error = FormatError::new(e.as_ref());
          error!("{:?}", format_error);
          self.list_tracks_state = ListTracksState::Failed(e);
        }
      }
    }
    Update::none()
  }

  pub fn view(&mut self) -> Element<'_, Message> {
    let table = Table::new()
      .push_column(Length::Units(15), Text::new("A"))
      .push_column(Length::Units(15), Text::new("BB"))
      .push_column(Length::Units(15), Text::new("CCC"))
      .push_row(vec![Text::new("a1").into(), Text::new("bb1").into(), Text::new("ccc1").into()])
      .push_row(vec![Text::new("a2").into(), Text::new("bb2").into(), Text::new("ccc2").into()])
      ;
    // let mut tracks = Scrollable::new(&mut self.scrollable_state);
    // for (track, track_artists, album, album_artists) in self.tracks.iter().take(1024) {
    //   tracks = tracks.push(Text::new(track.title.clone()));
    // }
    Column::new()
      .push(table)
      // .push(tracks)
      .into()
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
