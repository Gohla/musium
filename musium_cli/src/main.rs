use std::io;

use anyhow::{Context, Result};
use dotenv;
use metrics_core::{Builder, Drain, Observe};
use metrics_observer_yaml::{YamlBuilder, YamlObserver};
use metrics_runtime::{Controller, Receiver};
use structopt::StructOpt;
use tracing::{Level, trace};
use tracing_subscriber::FmtSubscriber;

use musium_client::{Client, Url};
use musium_core::model::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "cli", about = "Musium CLI")]
struct Opt {
  #[structopt(subcommand)]
  command: Command,

  /// Base URL to use for sending HTTP requests to the server
  #[structopt(long, env = "MUSIUM_URL_BASE")]
  url_base: Url,
  /// Username for logging into the server
  #[structopt(long, env = "MUSIUM_LOGIN_NAME")]
  name: String,
  /// Password for logging into the server
  #[structopt(long, env = "MUSIUM_LOGIN_PASSWORD")]
  password: String,

  /// Minimum level at which tracing events will be printed to stderr
  #[structopt(long, env = "MUSIUM_TRACING_LEVEL", default_value = "WARN")]
  tracing_level: Level,
  /// Whether to print metrics to stderr before the program exits
  #[structopt(long, env = "MUSIUM_PRINT_METRICS")]
  print_metrics: bool,
}

#[derive(Debug, StructOpt)]
enum Command {
  /// Lists all sources
  ListSources,
  /// Shows a source, found by id
  ShowSourceById {
    /// Id of the source to show
    id: i32,
  },
  /// Creates a local source
  CreateLocalSource {
    /// Directory of the local source to create
    directory: String,
  },
  /// Deletes a source, found by id
  DeleteSourceById {
    /// Id of the source to remove
    id: i32,
  },

  /// Lists all albums
  ListAlbums,
  /// Shows an album, found by id
  ShowAlbumById {
    id: i32,
  },

  /// Lists all tracks
  ListTracks,
  /// Shows a track, found by id
  ShowTrackById {
    id: i32,
  },
  /// Plays a track
  PlayTrack {
    /// ID of the track to play
    id: i32,
    /// Volume to play the track at, with 1.0 being full volume, and 0.0 being no volume
    #[structopt(short, long, default_value = "0.1")]
    volume: f32,
  },

  /// Lists all artists
  ListArtists,
  /// Shows an artist, found by id
  ShowArtistById {
    id: i32,
  },

  /// Lists all users
  ListUsers,
  /// Shows your (logged-in) user
  ShowMyUser,
  /// Shows a user, found by id
  ShowUserById {
    id: i32,
  },
  /// Creates a new user
  CreateUser {
    /// Name of the user to add
    name: String,
    /// Password of the user to add
    password: String,
  },
  /// Deletes a user, found by name
  DeleteUserByName {
    /// Name of the user to delete
    name: String,
  },
  /// Deletes a user, found by id
  DeleteUserById {
    /// Id of the user to delete
    id: i32,
  },

  /// Sets the user-rating for an album
  SetUserAlbumRating {
    /// ID of the album to set the rating for
    album_id: i32,
    /// The rating to set
    rating: i32,
  },
  /// Sets the user-rating for an track
  SetUserTrackRating {
    /// ID of the track to set the rating for
    track_id: i32,
    /// The rating to set
    rating: i32,
  },
  /// Sets the user-rating for an artist
  SetUserArtistRating {
    /// ID of the artist to set the rating for
    artist_id: i32,
    /// The rating to set
    rating: i32,
  },

  /// Initiate a scan in all scan directories to add/remove/update tracks
  Sync,
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
  // Create client
  let client: Client = Client::new(opt.url_base)
    .with_context(|| "Failed to create client")?;
  client.login(&UserLogin { name: opt.name, password: opt.password })
    .with_context(|| "Failed to login to server")?;
  // Run the application
  let result = run(opt.command, &client);
  // Print metrics
  if opt.print_metrics {
    controller.observe(&mut observer);
    let output = observer.drain();
    trace!(metrics = %output);
  }
  // Exit
  Ok(result?)
}

fn run(command: Command, client: &Client) -> Result<()> {
  match command {
    Command::ListSources => {
      for source in client.list_sources()? {
        println!("{:?}", source);
      }
    }
    Command::ShowSourceById { id } => {
      let source = client.get_source_by_id(id)?;
      println!("{:?}", source);
    }
    Command::CreateLocalSource { directory } => {
      let source = client.create_source(&NewSource { enabled: true, data: SourceData::Local(LocalSourceData { directory }) })?;
      println!("{:?}", source);
    }
    Command::DeleteSourceById { id } => {
      client.delete_source_by_id(id)?;
    }

    Command::ListAlbums => {
      for (album, album_artists) in client.list_albums()?.iter() {
        println!("{:?}", album);
        for artist in album_artists {
          println!("- {:?}", artist);
        }
      }
    }
    Command::ShowAlbumById { id } => {
      let album = client.get_album_by_id(id)?;
      println!("{:?}", album);
    }

    Command::ListTracks => {
      let tracks = client.list_tracks()?;
      for (track, track_artists, album, album_artists) in tracks.iter() {
        println!("- {:?}", track);
        for artist in track_artists {
          println!("  * {:?}", artist);
        }
        println!("  * {:?}", album);
        for artist in album_artists {
          println!("    - {:?}", artist);
        }
      }
    }
    Command::ShowTrackById { id } => {
      let track = client.get_track_by_id(id)?;
      println!("{:?}", track);
    }
    Command::PlayTrack { id, volume } => {
      let track_reader = client.download_track_by_id(id)?;
      if let Some(track_reader) = track_reader {
        musium_audio::play(track_reader, volume)
          .with_context(|| "Failed to play audio track")?;
      } else {
        eprintln!("Could not play track, no track with ID '{}' was found", id);
      }
    }

    Command::ListArtists => {
      for artist in client.list_artists()? {
        println!("{:?}", artist);
      }
    }
    Command::ShowArtistById { id } => {
      let artist = client.get_artist_by_id(id)?;
      println!("{:?}", artist);
    }

    Command::ListUsers => {
      for user in client.list_users()? {
        println!("{:?}", user);
      }
    }
    Command::ShowMyUser => {
      let user = client.get_my_user()?;
      println!("{:?}", user);
    }
    Command::ShowUserById { id } => {
      let user = client.get_user_by_id(id)?;
      println!("{:?}", user);
    }
    Command::CreateUser { name, password } => {
      let user = client.create_user(&NewUser { name, password })?;
      println!("{:?}", user);
    }
    Command::DeleteUserByName { name } => {
      client.delete_user_by_name(&name)?;
    }
    Command::DeleteUserById { id } => {
      client.delete_user_by_id(id)?;
    }

    Command::SetUserAlbumRating { album_id, rating } => {
      let rating = client.set_user_album_rating(album_id, rating)?;
      println!("{:?}", rating);
    }
    Command::SetUserTrackRating { track_id, rating } => {
      let rating = client.set_user_track_rating(track_id, rating)?;
      println!("{:?}", rating);
    }
    Command::SetUserArtistRating { artist_id, rating } => {
      let rating = client.set_user_artist_rating(artist_id, rating)?;
      println!("{:?}", rating);
    }

    Command::Sync => {
      let started_sync = client.sync()?;
      if started_sync {
        println!("Started synchronizing");
      } else {
        println!("Already synchronizing");
      }
    }
  }
  Ok(())
}
