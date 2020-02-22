use std::fs::File;
use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use metrics_core::{Builder, Drain, Observe};
use metrics_observer_yaml::{YamlBuilder, YamlObserver};
use metrics_runtime::{Controller, Receiver};
use structopt::StructOpt;
use tracing::{Level, trace};
use tracing_subscriber::FmtSubscriber;

use server::Server;

#[derive(Debug, StructOpt)]
#[structopt(name = "music_composer", about = "Music Composer")]
struct Opt {
  #[structopt(subcommand)]
  command: Command,
  /// Database file to use. Relative paths are resolved relative to the current directory
  #[structopt(short, long, default_value = "database.sql", parse(from_os_str))]
  database_file: PathBuf,
}

#[derive(Debug, StructOpt)]
enum Command {
  /// Lists all tracks in the database
  #[structopt()]
  ListTracks,
  /// Lists all scan directories in the database
  #[structopt()]
  ListScanDirectories,
  /// Lists all scan directories in the database, along with their tracks
  #[structopt()]
  ListScanDirectoriesWithTracks,
  /// Plays a track
  #[structopt()]
  PlayTrack {
    /// ID of the track to play
    track_id: i32,
    #[structopt(short, long, default_value = "0.2")]
    volume: f32,
  },
  /// Add a scan directory to the database
  #[structopt()]
  AddScanDirectory {
    /// Scan directory to add
    #[structopt(parse(from_os_str))]
    directory: PathBuf,
  },
  /// Removes a scan directory to the database
  #[structopt()]
  RemoveScanDirectory {
    /// Scan directory to remove
    #[structopt(parse(from_os_str))]
    directory: PathBuf,
  },
  /// Scan for music files in all scan directories, and add their tracks to the database
  #[structopt()]
  Scan,
}

fn main() -> Result<()> {
  let subscriber = FmtSubscriber::builder()
    .with_writer(io::stderr)
    .with_max_level(Level::TRACE)
    .finish();
  tracing::subscriber::set_global_default(subscriber)
    .with_context(|| "Failed to initialize global tracing subscriber")?;

  let metrics_receiver: Receiver = Receiver::builder().build()
    .with_context(|| "Failed to initialize metrics receiver")?;
  let controller: Controller = metrics_receiver.controller();
  let mut observer: YamlObserver = YamlBuilder::new().build();
  metrics_receiver.install();

  let opt: Opt = Opt::from_args();
  let result = run(opt);

  controller.observe(&mut observer);
  let output = observer.drain();
  trace!(metrics = %output);

  Ok(result?)
}

fn run(opt: Opt) -> Result<()> {
  let server: Server = Server::new(opt.database_file.to_string_lossy())
    .with_context(|| "Failed to initialize server")?;
  match opt.command {
    Command::ListTracks => {
      for track in server.list_tracks().with_context(|| "Failed to list tracks")? {
        println!("{}", track);
      }
    }
    Command::ListScanDirectories => {
      for scan_directory in server.list_scan_directories().with_context(|| "Failed to list scan directories")? {
        println!("{}", scan_directory);
      }
    }
    Command::ListScanDirectoriesWithTracks => {
      for (scan_directory, tracks) in server.list_scan_directories_with_tracks().with_context(|| "Failed to list scan directories")? {
        println!("* {}", scan_directory);
        for track in tracks {
          println!("  - {}", track);
        }
      }
    }
    Command::PlayTrack { track_id, volume } => {
      if let Some((scan_directory, track)) = server.get_track_by_id(track_id)? {
        println!("* {}", scan_directory);
        println!("  - {}", track);
        let device = rodio::default_output_device()
          .with_context(|| "No audio device was found")?;
        let file = File::open(scan_directory.track_file_path(&track))
          .with_context(|| "Failed to open audio file for playback")?;
        let sink = rodio::play_once(&device, file)
          .with_context(|| "Failed to start audio playback")?;
        sink.set_volume(volume);
        sink.sleep_until_end();
      } else {
        eprintln!("Could not play track, no track with ID '{}' was found", track_id);
      }
    }
    Command::AddScanDirectory { directory } => {
      server.add_scan_directory(&directory).with_context(|| "Failed to add scan directory")?;
      eprintln!("Added scan directory '{}'", directory.display());
    }
    Command::RemoveScanDirectory { directory } => {
      let removed = server.remove_scan_directory(&directory).with_context(|| "Failed to remove scan directory")?;
      if removed {
        eprintln!("Removed scan directory '{}'", directory.display());
      } else {
        eprintln!("Could not remove scan directory '{}', it was not found", directory.display());
      }
    }
    Command::Scan => {
      server.scan().with_context(|| "Failed to scan music files")?;
    }
  }
  Ok(())
}
