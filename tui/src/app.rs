use itertools::Itertools;
use tui::{
  backend::Backend,
  Frame,
  style::{Color, Style},
  text::Spans,
  widgets::Tabs,
};
use tui::layout::{Alignment, Constraint, Layout, Rect};
use tui::text::Span;
use tui::widgets::{Paragraph, Row, Table, TableState, Wrap};

use musium_core::model::{Artist, Track};
use musium_core::model::collection::{Albums, Tracks};

use crate::util::{TabsState, TabState};

pub struct App {
  tabs: TabsState<MainTabs>,

  logged_in: bool,
  albums: Albums,
  albums_state: TableState,
  tracks: Tracks,
  tracks_state: TableState,
  artists: Vec<Artist>,
  artists_state: TableState,
}

#[derive(Copy, Clone)]
enum MainTabs {
  Album,
  Track,
  Artist,
}

impl App {
  pub fn new() -> Self {
    let tabs = TabsState::new(MainTabs::Album);
    let logged_in = false;
    let albums = Albums::default();
    let mut albums_state = TableState::default();
    albums_state.select(Some(0));
    let tracks = Tracks::default();
    let mut tracks_state = TableState::default();
    tracks_state.select(Some(0));
    let artists = vec![];
    let mut artists_state = TableState::default();
    artists_state.select(Some(0));
    Self {
      tabs,
      logged_in,
      albums,
      albums_state,
      tracks,
      tracks_state,
      artists,
      artists_state,
    }
  }
}

// Drawing

impl App {
  pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
    if !self.logged_in {
      let text = vec![
        Spans::from(Span::styled("Logging in", Style::default())),
      ];
      let widget = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
      f.render_widget(widget, f.size());
      return;
    }

    let chunks = Layout::default()
      .constraints([Constraint::Length(2), Constraint::Min(0)].as_ref())
      .split(f.size());
    let titles = MainTabs::iter().map(|s| Spans::from(s.name())).collect();
    let tabs = Tabs::new(titles)
      .highlight_style(Style::default().fg(Color::Yellow))
      .divider("|")
      .select(self.tabs.index());
    f.render_widget(tabs, chunks[0]);
    self.draw_main_tab(f, self.tabs.state(), chunks[1]);
  }

  fn draw_main_tab<B: Backend>(&mut self, f: &mut Frame<B>, tab: MainTabs, area: Rect) {
    use MainTabs::*;
    match tab {
      Album => self.draw_albums(f, area),
      Track => self.draw_tracks(f, area),
      Artist => self.draw_artists(f, area),
    }
  }

  fn draw_albums<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
    let widget = Table::new(
      ["ID", "Name", "Artists"].iter(),
      self.albums.iter().map(|(album, artists)| {
        Row::Data(vec![
          album.id.to_string(),
          album.name.clone(),
          artists.map(|a| &a.name).join(", "),
        ].into_iter())
      }),
    )
      .widths(&[Constraint::Min(6), Constraint::Percentage(50), Constraint::Percentage(50)])
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().fg(Color::Yellow))
      .highlight_symbol(">")
      .column_spacing(1);
    f.render_stateful_widget(widget, area, &mut self.albums_state);
  }

  fn draw_tracks<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
    let widget = Table::new(
      ["ID", "Disc #", "Track #", "Title", "Artists", "Album", "Album artists"].iter(),
      self.tracks.iter().map(|(track, artists, album, album_artists)| {
        Row::Data(vec![
          track.id.to_string(),
          track.disc_number.map_or("".to_string(), |n| n.to_string()),
          track.track_number.map_or("".to_string(), |n| n.to_string()),
          track.title.clone(),
          artists.map(|a| &a.name).join(", "),
          album.name.clone(),
          album_artists.map(|a| &a.name).join(", "),
        ].into_iter())
      }),
    )
      .widths(&[Constraint::Min(6), Constraint::Min(1), Constraint::Min(3), Constraint::Percentage(30), Constraint::Percentage(30), Constraint::Percentage(20), Constraint::Percentage(20)])
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().fg(Color::Yellow))
      .highlight_symbol(">")
      .column_spacing(1);
    f.render_stateful_widget(widget, area, &mut self.tracks_state);
  }

  fn draw_artists<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
    let widget = Table::new(
      ["ID", "Name"].iter(),
      self.artists.iter().map(|artist| {
        Row::Data(vec![
          artist.id.to_string(),
          artist.name.clone(),
        ].into_iter())
      }),
    )
      .widths(&[Constraint::Min(6), Constraint::Percentage(100)])
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().fg(Color::Yellow))
      .highlight_symbol(">")
      .column_spacing(1);
    f.render_stateful_widget(widget, area, &mut self.artists_state);
  }
}

// System -> App interaction

impl App {
  pub fn tick(&mut self) {}
  pub fn next_tab(&mut self) { self.tabs.next(); }
  pub fn prev_tab(&mut self) { self.tabs.prev(); }
  pub fn up(&mut self, offset: usize) {
    match self.tabs.state() {
      MainTabs::Album => self.albums_state.select(Some(self.albums_state.selected().unwrap_or(0).saturating_sub(offset))),
      MainTabs::Track => self.tracks_state.select(Some(self.tracks_state.selected().unwrap_or(0).saturating_sub(offset))),
      MainTabs::Artist => self.artists_state.select(Some(self.artists_state.selected().unwrap_or(0).saturating_sub(offset))),
    }
  }
  pub fn down(&mut self, offset: usize) {
    match self.tabs.state() {
      MainTabs::Album => self.albums_state.select(Some(self.albums_state.selected().unwrap_or(0).saturating_add(offset).min(self.albums.len().saturating_sub(1)))),
      MainTabs::Track => self.tracks_state.select(Some(self.tracks_state.selected().unwrap_or(0).saturating_add(offset).min(self.tracks.len().saturating_sub(1)))),
      MainTabs::Artist => self.artists_state.select(Some(self.artists_state.selected().unwrap_or(0).saturating_add(offset).min(self.artists.len().saturating_sub(1)))),
    }
  }
  pub fn set_logged_in(&mut self) { self.logged_in = true; }
  pub fn set_albums(&mut self, albums: Albums) { self.albums = albums; }
  pub fn set_tracks(&mut self, tracks: Tracks) { self.tracks = tracks; }
  pub fn set_artists(&mut self, artists: Vec<Artist>) { self.artists = artists; }
}

// App -> System interaction

impl App {
  pub fn get_selected_track(&self) -> Option<&Track> {
    if let MainTabs::Track = self.tabs.state() {
      if let Some(selected_index) = self.tracks_state.selected() {
        return self.tracks.get_track(selected_index)
      }
    }
    None
  }
}

// Enum boilerplate

impl TabState for MainTabs {
  fn name(&self) -> &str {
    use MainTabs::*;
    match self {
      Album => "Album",
      Track => "Track",
      Artist => "Artist",
    }
  }

  fn index(&self) -> usize {
    use MainTabs::*;
    match self {
      Album => 0,
      Track => 1,
      Artist => 2,
    }
  }

  fn next(&self) -> Self {
    use MainTabs::*;
    match self {
      Album => Track,
      Track => Artist,
      Artist => Album,
    }
  }

  fn prev(&self) -> Self {
    use MainTabs::*;
    match self {
      Album => Artist,
      Track => Album,
      Artist => Track,
    }
  }
}

impl MainTabs {
  pub fn iter() -> impl Iterator<Item=&'static Self> {
    use MainTabs::*;
    static TABS: [MainTabs; 3] = [Album, Track, Artist];
    TABS.iter()
  }
}
