#![feature(deadline_api)]

use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dotenv;
use metrics_core::{Builder, Drain, Observe};
use metrics_observer_yaml::{YamlBuilder, YamlObserver};
use metrics_runtime::{Controller, Receiver};
use structopt::StructOpt;
use tracing::trace;
use tracing_subscriber::{EnvFilter, fmt};
use tracing_subscriber::prelude::*;
use tui::{backend::CrosstermBackend, Terminal};

use musium_client::{Client, Url};
use musium_core::model::*;
use musium_core::model::collection::{Albums, Tracks};

use crate::app::App;

mod util;
mod app;

#[derive(Debug, StructOpt)]
#[structopt(name = "cli", about = "Musium CLI")]
struct Opt {
  /// Base URL to use for sending HTTP requests to the server
  #[structopt(long, env = "MUSIUM_URL_BASE")]
  url_base: Url,
  /// Username for logging into the server
  #[structopt(long, env = "MUSIUM_LOGIN_NAME")]
  name: String,
  /// Password for logging into the server
  #[structopt(long, env = "MUSIUM_LOGIN_PASSWORD")]
  password: String,

  /// Whether to print metrics to stderr before the program exits
  #[structopt(long, env = "MUSIUM_PRINT_METRICS")]
  print_metrics: bool,
}

fn main() -> Result<()> {
  // Load environment variables from .env file, before parsing command-line arguments, as some options can use
  // environment variables as defaults.
  dotenv::dotenv().ok();
  // Parse command-line arguments.
  let opt: Opt = Opt::from_args();
  // Setup tracing
  let fmt_layer = fmt::layer()
    .with_writer(std::io::stderr)
    ;
  let filter_layer = EnvFilter::from_default_env();
  tracing_subscriber::registry()
    .with(filter_layer)
    .with(fmt_layer)
    .init();
  // Setup metrics
  let metrics_receiver: Receiver = Receiver::builder().build()
    .with_context(|| "Failed to initialize metrics receiver")?;
  let controller: Controller = metrics_receiver.controller();
  let mut observer: YamlObserver = YamlBuilder::new().build();
  metrics_receiver.install();
  // Create client
  let client = Client::new(opt.url_base)
    .with_context(|| "Failed to create client")?;
  let user_login = UserLogin { name: opt.name, password: opt.password };
  // Run TUI
  let app = app::App::new();
  let result = run(client, user_login, app, Duration::from_millis(250));
  // Print metrics
  if opt.print_metrics {
    controller.observe(&mut observer);
    let output = observer.drain();
    trace!(metrics = %output);
  }
  // Exit
  Ok(result?)
}

fn run(client: Client, user_login: UserLogin, mut app: App, tick_rate: Duration) -> Result<()> {
  enable_raw_mode()?;
  let mut stdout = stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // Messages for communicating from crossterm and the musium client, to the terminal rendering loop.
  enum TerminalMessage<I> {
    Tick,
    Input(I),
    LoggedIn,
    AlbumsReceived(Albums),
    TracksReceived(Tracks),
    ArtistsReceived(Vec<Artist>),
  }
  let (terminal_event_tx, terminal_event_rx) = mpsc::channel();

  // Thread for crossterm input handling.
  let crossterm_terminal_event_tx = terminal_event_tx.clone();
  thread::spawn(move || {
    use crossterm::event::Event;
    loop {
      if let Ok(true) = event::poll(Duration::from_millis(100)) {
        if let Ok(event) = event::read() {
          match event {
            Event::Key(key) => crossterm_terminal_event_tx.send(TerminalMessage::Input(key)).unwrap(),
            _ => {}
          }
        }
      }
    }
  });

  // Thread for client communication.
  #[derive(Debug)]
  enum ClientMessage {
    Login,
    RequestAlbums,
    RequestTracks,
    RequestArtists,
    PlayTrack(i32),
  }
  let (client_tx, mut client_rx) = tokio::sync::mpsc::unbounded_channel();
  let client_runtime = tokio::runtime::Runtime::new()?;
  client_runtime.spawn(async move {
    loop {
      if let Some(message) = client_rx.recv().await {
        match message {
          ClientMessage::Login => {
            let client = client.clone();
            let tx = terminal_event_tx.clone();
            let user_login = user_login.clone();
            tokio::runtime::Handle::current().spawn(async move {
              client.login(&user_login).await.unwrap();
              tx.send(TerminalMessage::LoggedIn).unwrap();
            });
          }
          ClientMessage::RequestAlbums => {
            let client = client.clone();
            let tx = terminal_event_tx.clone();
            tokio::runtime::Handle::current().spawn(async move {
              let albums = client.list_albums().await.unwrap();
              tx.send(TerminalMessage::AlbumsReceived(albums)).unwrap();
            });
          }
          ClientMessage::RequestTracks => {
            let client = client.clone();
            let tx = terminal_event_tx.clone();
            tokio::runtime::Handle::current().spawn(async move {
              let tracks = client.list_tracks().await.unwrap();
              tx.send(TerminalMessage::TracksReceived(tracks)).unwrap();
            });
          }
          ClientMessage::RequestArtists => {
            let client = client.clone();
            let tx = terminal_event_tx.clone();
            tokio::runtime::Handle::current().spawn(async move {
              let artists = client.list_artists().await.unwrap();
              tx.send(TerminalMessage::ArtistsReceived(artists)).unwrap();
            });
          }
          ClientMessage::PlayTrack(track_id) => {
            let client = client.clone();
            tokio::runtime::Handle::current().spawn(async move {
              client.play_track_by_id(track_id).await.unwrap();
            });
          }
        }
      }
    }
  });
  // Immediately ask the client to login.
  client_tx.send(ClientMessage::Login)?;

  // Main loop that draws the terminal and handles messages.
  terminal.clear()?;
  loop {
    terminal.draw(|f| app.draw(f))?;
    let result = terminal_event_rx.recv_deadline(Instant::now() + tick_rate);
    let terminal_event = match result {
      Ok(terminal_event) => terminal_event,
      Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
      _ => TerminalMessage::Tick,
    };
    match terminal_event {
      TerminalMessage::Tick => app.tick(),
      TerminalMessage::Input(event) => {
        match event.code {
          KeyCode::Char('q') => {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
            terminal.show_cursor()?;
            break;
          }
          KeyCode::BackTab => app.prev_tab(),
          KeyCode::Tab => app.next_tab(),
          KeyCode::Home => app.up(usize::MAX),
          KeyCode::PageUp => app.up(10),
          KeyCode::Up => app.up(1),
          KeyCode::Down => app.down(1),
          KeyCode::PageDown => app.down(10),
          KeyCode::End => app.down(usize::MAX),
          KeyCode::Enter => {
            if let Some(track) = app.get_selected_track() {
              client_tx.send(ClientMessage::PlayTrack(track.id))?;
            }
          }
          KeyCode::Char('r') => {
            client_tx.send(ClientMessage::RequestAlbums)?;
            client_tx.send(ClientMessage::RequestTracks)?;
            client_tx.send(ClientMessage::RequestArtists)?;
          }
          _ => {}
        }
      }
      TerminalMessage::LoggedIn => {
        app.set_logged_in();
        client_tx.send(ClientMessage::RequestAlbums)?;
        client_tx.send(ClientMessage::RequestTracks)?;
        client_tx.send(ClientMessage::RequestArtists)?;
      }
      TerminalMessage::AlbumsReceived(albums) => app.set_albums(albums),
      TerminalMessage::TracksReceived(tracks) => app.set_tracks(tracks),
      TerminalMessage::ArtistsReceived(artists) => app.set_artists(artists),
    }
  }

  Ok(())
}
