#![feature(deadline_api)]

use anyhow::{Context, Result};
use dotenv;
use iced::Application;
use metrics_core::{Builder, Drain, Observe};
use metrics_observer_yaml::{YamlBuilder, YamlObserver};
use metrics_runtime::{Controller, Receiver};
use structopt::StructOpt;
use tracing::trace;
use tracing_subscriber::{EnvFilter, fmt};
use tracing_subscriber::prelude::*;

use app::{App, Flags};
use musium_audio::Player;
use musium_client::{Client, Url};
use musium_core::model::*;

mod app;
mod page;
mod util;
mod widget;

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
  let client = Client::new(opt.url_base.clone())
    .with_context(|| "Failed to create client")?;
  let audio_player = Player::new()
    .with_context(|| "Failed to create audio player")?;
  // Run GUI
  let user_login = UserLogin { name: opt.name, password: opt.password };
  App::run(iced::Settings::with_flags(Flags { client, audio_player, initial_url: opt.url_base, initial_user_login: user_login })).unwrap();
  // TODO: run takes control of the application, the rest will not run. Should put this in a tread!
  // Print metrics
  if opt.print_metrics {
    controller.observe(&mut observer);
    let output = observer.drain();
    trace!(metrics = %output);
  }
  // Exit
  Ok(())
}
