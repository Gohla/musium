use std::path::PathBuf;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{App, HttpResponse, HttpServer, middleware, Responder, web};
use anyhow::{Context, Result};
use dotenv;
use metrics_core::{Builder, Drain, Observe};
use metrics_observer_yaml::{YamlBuilder, YamlObserver};
use metrics_runtime::{Controller, Receiver};
use structopt::StructOpt;
use tracing::{Level, trace};
use tracing_subscriber::FmtSubscriber;

use backend::{Backend, BackendConnected};

use crate::auth::{login, logout, me};
use crate::util::ResultExt;
use tracing_log::LogTracer;

pub mod auth;
pub mod util;

#[derive(Debug, StructOpt)]
#[structopt(name = "client", about = "Music Composer client")]
struct Opt {
  /// Database file to use. Relative paths are resolved relative to the current directory
  #[structopt(short, long, env = "DATABASE_URL", parse(from_os_str))]
  database_file: PathBuf,
  /// Password hasher secret key to use
  #[structopt(short, long, env = "PASSWORD_HASHER_SECRET_KEY")]
  password_hasher_secret_key: String,
  /// Minimum level at which tracing events will be printed to stderr
  #[structopt(short, long, default_value = "TRACE")]
  tracing_level: Level,
  /// Whether to print metrics to stderr before the program exits
  #[structopt(short, long)]
  metrics: bool,
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
  // Create backend
  let backend = Backend::new(
    opt.database_file.to_string_lossy(),
    opt.password_hasher_secret_key.as_bytes())
    .with_context(|| "Failed to create backend")?;
  // Run HTTP server
  actix_rt::System::new("server")
    .block_on(async move { serve(backend).await })
    .with_context(|| "HTTP server failed")?;
  // Print metrics
  if opt.metrics {
    controller.observe(&mut observer);
    let output = observer.drain();
    trace!(metrics = %output);
  }
  Ok(())
}

async fn serve(backend: Backend) -> std::io::Result<()> {
  let backend_data = web::Data::new(backend);
  HttpServer::new(move || {
    App::new()
      .wrap(middleware::Logger::default())
      .wrap(IdentityService::new(
        CookieIdentityPolicy::new(&[0; 32]) // TODO: implement secret key.
          .name("auth-cookie")
          .secure(false)
      ))
      .app_data(backend_data.clone())
      .route("/", web::get().to(index))
      .route("/tracks", web::get().to(tracks))
      .route("/login", web::post().to(login))
      .route("/logout", web::delete().to(logout))
      .route("/me", web::get().to(me))
  })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}

async fn index() -> impl Responder {
  HttpResponse::Ok().body("Hello world!")
}

async fn tracks(backend: web::Data<Backend>) -> actix_web::Result<impl Responder> {
  let backend_connected: BackendConnected = backend.connect_to_database().map_internal_err()?;
  let tracks = backend_connected.list_tracks().map_internal_err()?;
  Ok(HttpResponse::Ok().json(tracks))
}
