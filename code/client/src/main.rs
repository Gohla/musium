use std::fs::File;
use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use dotenv;
use metrics_core::{Builder, Drain, Observe};
use metrics_observer_yaml::{YamlBuilder, YamlObserver};
use metrics_runtime::{Controller, Receiver};
use structopt::StructOpt;
use tracing::{Level, trace};
use tracing_subscriber::FmtSubscriber;

use backend::Backend;

#[derive(Debug, StructOpt)]
#[structopt(name = "client", about = "Music Composer client")]
struct Opt {
  #[structopt(subcommand)]
  command: Command,
  /// Database file to use. Relative paths are resolved relative to the current directory
  #[structopt(short, long, env = "DATABASE_URL", parse(from_os_str))]
  database_file: PathBuf,
  /// Minimum level at which tracing events will be printed to stderr
  #[structopt(short, long, default_value = "TRACE")]
  tracing_level: Level,
  /// Whether to print metrics to stderr before the program exits
  #[structopt(short, long)]
  metrics: bool,
}

#[derive(Debug, StructOpt)]
enum Command {
  /// Lists all scan directories in the database
  ListScanDirectories,
  /// Add a scan directory to the database
  AddScanDirectory {
    /// Scan directory to add
    #[structopt(parse(from_os_str))]
    directory: PathBuf,
  },
  /// Removes a scan directory from the database
  RemoveScanDirectory {
    /// Scan directory to remove
    #[structopt(parse(from_os_str))]
    directory: PathBuf,
  },

  /// Lists all albums in the database
  ListAlbums,

  /// Lists all tracks in the database
  ListTracks,
  /// Plays a track
  PlayTrack {
    /// ID of the track to play
    track_id: i32,
    #[structopt(short, long, default_value = "0.2")]
    volume: f32,
  },

  /// Lists all artists in the database
  ListArtists,

  /// Scan for music files in all scan directories, and add their tracks to the database
  Scan,

  /// Lists all users in the database
  ListUsers,
  /// Add a user to the database
  AddUser {
    /// Name of the user to add
    name: String,
    /// Password of the user to add
    password: String
  },
  /// Removes a user from the database
  RemoveUser {
    /// Name of the user to remove
    name: String,
  },

  /// Sets the user-rating for an album
  SetUserAlbumRating {
    /// ID of the user to set the rating for
    user_id: i32,
    /// ID of the album to set the rating for
    album_id: i32,
    /// The rating to set
    rating: i32,
  },
  /// Sets the user-rating for an track
  SetUserTrackRating {
    /// ID of the user to set the rating for
    user_id: i32,
    /// ID of the track to set the rating for
    track_id: i32,
    /// The rating to set
    rating: i32,
  },
  /// Sets the user-rating for an artist
  SetUserArtistRating {
    /// ID of the user to set the rating for
    user_id: i32,
    /// ID of the artist to set the rating for
    artist_id: i32,
    /// The rating to set
    rating: i32,
  },
}

fn main() -> Result<()> {
  // Load environment variables from .env file, before parsing command-line arguments, as some options can use
  // environment variables as defaults.
  dotenv::dotenv().ok();
  // Parse command-line arguments.
  let opt: Opt = Opt::from_args();
  // Setup tracing
  let subscriber = FmtSubscriber::builder()
    .with_writer(io::stderr)
    .with_max_level(opt.tracing_level.clone())
    .finish();
  tracing::subscriber::set_global_default(subscriber)
    .with_context(|| "Failed to initialize global tracing subscriber")?;
  // Setup metrics
  let metrics_receiver: Receiver = Receiver::builder().build()
    .with_context(|| "Failed to initialize metrics receiver")?;
  let controller: Controller = metrics_receiver.controller();
  let mut observer: YamlObserver = YamlBuilder::new().build();
  metrics_receiver.install();
  // Copy over relevant options before passing ownership to run.
  let metrics = opt.metrics;
  // Run the application
  let result = run(opt);
  // Print metrics
  if metrics {
    controller.observe(&mut observer);
    let output = observer.drain();
    trace!(metrics = %output);
  }
  // Exit
  Ok(result?)
}

fn run(opt: Opt) -> Result<()> {
  let backend: Backend = Backend::new(opt.database_file.to_string_lossy(), vec![])
    .with_context(|| "Failed to initialize backend")?;
  let backend_connected= backend.connect_to_database()
    .with_context(|| "Failed to connect to backend database")?;
  match opt.command {
    Command::ListScanDirectories => {
      for scan_directory in backend_connected.list_scan_directories().with_context(|| "Failed to list scan directories")? {
        println!("{}", scan_directory);
      }
    }
    Command::AddScanDirectory { directory } => {
      backend_connected.add_scan_directory(&directory).with_context(|| "Failed to add scan directory")?;
      eprintln!("Added scan directory '{}'", directory.display());
    }
    Command::RemoveScanDirectory { directory } => {
      let removed = backend_connected.remove_scan_directory(&directory).with_context(|| "Failed to remove scan directory")?;
      if removed {
        eprintln!("Removed scan directory '{}'", directory.display());
      } else {
        eprintln!("Could not remove scan directory '{}', it was not found", directory.display());
      }
    }

    Command::ListAlbums => {
      for (album, album_artists) in backend_connected.list_albums().with_context(|| "Failed to list albums")?.iter() {
        println!("{:?}", album);
        for artist in album_artists {
          println!("  {:?}", artist);
        }
      }
    }

    Command::ListTracks => {
      for (scan_directory, track, track_artists, album, album_artists) in backend_connected.list_tracks().with_context(|| "Failed to list tracks")?.iter() {
        println!("{:?}", scan_directory);
        println!("  {:?}", track);
        for artist in track_artists {
          println!("    {:?}", artist);
        }
        println!("    {:?}", album);
        for artist in album_artists {
          println!("      {:?}", artist);
        }
      }
    }
    Command::PlayTrack { track_id, volume } => {
      if let Some((scan_directory, track)) = backend_connected.get_track_by_id(track_id).with_context(|| "Failed to get track")? {
        println!("* {}", scan_directory);
        println!("  - {}", track);
        if let Some(file_path) = scan_directory.track_file_path(&track) {
          let device = rodio::default_output_device()
            .with_context(|| "No audio device was found")?;
          let file = File::open(file_path)
            .with_context(|| "Failed to open audio file for playback")?;
          let sink = rodio::play_once(&device, file)
            .with_context(|| "Failed to start audio playback")?;
          sink.set_volume(volume);
          sink.sleep_until_end();
        } else {
          eprintln!("Could not play track with ID '{}', it does not have a file path, indicating that the track was removed", track_id);
        }
      } else {
        eprintln!("Could not play track, no track with ID '{}' was found", track_id);
      }
    }

    Command::ListArtists => {
      for artist in backend_connected.list_artists().with_context(|| "Failed to list artists")?.iter() {
        println!("{:?}", artist);
      }
    }

    Command::Scan => {
      backend_connected.scan()
        .with_context(|| "Failed to scan music files")?;
    }
    Command::ListUsers => {
      for user in backend_connected.list_users()
        .with_context(|| "Failed to list users")? {
        println!("{:?}", user);
      }
    }
    Command::AddUser { name, password } => {
      let user = backend_connected.add_user(name, password)
        .with_context(|| "Failed to add user")?;
      eprintln!("Added {:?}", user);
    }
    Command::RemoveUser { name } => {
      let removed = backend_connected.remove_user(&name)
        .with_context(|| "Failed to remove user")?;
      if removed {
        eprintln!("Removed user with name '{}'", name);
      } else {
        eprintln!("Could not remove user with name '{}', it was not found", name);
      }
    }
    Command::SetUserAlbumRating { user_id, album_id, rating } => {
      let user_album_rating = backend_connected.set_user_album_rating(user_id, album_id, rating)
        .with_context(|| "Failed to set user album rating")?;
      eprintln!("Set {:?}", user_album_rating);
    }
    Command::SetUserTrackRating { user_id, track_id, rating } => {
      let user_track_rating = backend_connected.set_user_track_rating(user_id, track_id, rating)
        .with_context(|| "Failed to set user track rating")?;
      eprintln!("Set {:?}", user_track_rating);
    }
    Command::SetUserArtistRating { user_id, artist_id, rating } => {
      let user_artist_rating = backend_connected.set_user_artist_rating(user_id, artist_id, rating)
        .with_context(|| "Failed to set user artist rating")?;
      eprintln!("Set {:?}", user_artist_rating);
    }
  }
  Ok(())
}
