use std::path::PathBuf;

use anyhow::{Context, Result};
use dotenv;
use metrics_core::{Builder, Drain, Observe};
use metrics_observer_yaml::{YamlBuilder, YamlObserver};
use metrics_runtime::{Controller, Receiver};
use structopt::StructOpt;
use tracing::info;
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;

use musium_backend::database::Database;
use musium_core::model::NewUser;

use crate::serve::serve;

pub mod serve;
pub mod auth;
pub mod api;
pub mod scanner;
pub mod util;

#[derive(Debug, StructOpt)]
#[structopt(name = "server", about = "Musium server")]
struct Opt {
  /// Database file to use. Relative paths are resolved relative to the current directory
  #[structopt(long, env = "MUSIUM_DATABASE_URL", parse(from_os_str))]
  database_file: PathBuf,

  /// Address (IP:port) to bind the HTTP server to
  #[structopt(long, env = "MUSIUM_BIND_ADDRESS", default_value = "127.0.0.1:8088")]
  bind_address: String,
  /// Password hasher secret key to use
  #[structopt(long, env = "MUSIUM_PASSWORD_HASHER_SECRET_KEY")]
  password_hasher_secret_key: String,
  /// Cookie identity secret key to use
  #[structopt(long, env = "MUSIUM_COOKIE_IDENTITY_SECRET_KEY")]
  cookie_identity_secret_key: String,

  /// Name of the admin user that is created by default.
  #[structopt(long, env = "MUSIUM_LOGIN_NAME")]
  admin_name: String,
  /// Password of the admin user that is created by default.
  #[structopt(long, env = "MUSIUM_LOGIN_PASSWORD")]
  admin_password: String,

  /// Minimum level at which tracing events will be printed to stderr
  #[structopt(long, env = "MUSIUM_TRACING_LEVEL", default_value = "WARN")]
  tracing_level: tracing::Level,
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
  let subscriber = FmtSubscriber::builder()
    .with_writer(std::io::stderr)
    .with_max_level(opt.tracing_level.clone())
    .finish();
  tracing::subscriber::set_global_default(subscriber)
    .with_context(|| "Failed to initialize global tracing subscriber")?;
  // Setup log to forward to tracing.
  LogTracer::init()
    .with_context(|| "Failed to initialize log to tracing forwarder")?;
  // Setup metrics
  let metrics_receiver: Receiver = Receiver::builder().build()
    .with_context(|| "Failed to initialize metrics receiver")?;
  let controller: Controller = metrics_receiver.controller();
  let mut observer: YamlObserver = YamlBuilder::new().build();
  metrics_receiver.install();
  // Create database
  let database = Database::new(
    opt.database_file.to_string_lossy(),
    opt.password_hasher_secret_key.as_bytes())
    .with_context(|| "Failed to create database")?;
  database.connect()
    .with_context(|| "Failed to connect to database to create the admin user")?
    .create_user(NewUser { name: opt.admin_name, password: opt.admin_password })
    .ok();
  // Run HTTP server
  let bind_address = opt.bind_address.clone();
  let cookie_identity_secret_key = opt.cookie_identity_secret_key.clone();
  actix_rt::System::new("server")
    .block_on(async move { serve(database, bind_address, cookie_identity_secret_key).await })
    .with_context(|| "HTTP server failed")?;
  // Print metrics
  if opt.print_metrics {
    controller.observe(&mut observer);
    let output = observer.drain();
    info!(metrics = %output);
  }
  Ok(())
}
