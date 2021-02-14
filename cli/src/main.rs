use anyhow::{Context, Result};
use dotenv;
use metrics_core::{Builder, Drain, Observe};
use metrics_observer_yaml::{YamlBuilder, YamlObserver};
use metrics_runtime::{Controller, Receiver};
use structopt::StructOpt;
use tracing::trace;
use tracing_subscriber::{EnvFilter, fmt};
use tracing_subscriber::prelude::*;

use musium_client::{Client, PlaySource, Url};
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

  /// Whether to print metrics to stderr before the program exits
  #[structopt(long, env = "MUSIUM_PRINT_METRICS")]
  print_metrics: bool,
}

#[derive(Debug, StructOpt)]
enum Command {
  /// Lists all local sources
  ListLocalSources,
  /// Shows a local source, found by id
  ShowLocalSourceById {
    /// Id of the local source to show
    id: i32,
  },
  /// Creates or enables a local source
  CreateOrEnableLocalSource {
    /// Directory of the local source to create
    directory: String,
  },
  /// Enables or disables a local source, found by id
  SetLocalSourceEnabledById {
    /// Id of the local source
    id: i32,
    /// Whether to enable or disable the local source
    #[structopt(short, long)]
    enabled: bool,
  },

  /// Creates a new Spotify source by requesting authorization with Spotify
  CreateSpotifySource,
  /// Shows me-info for my Spotify source
  ShowSpotifyMe,

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
  let client: Client = Client::new(opt.url_base)
    .with_context(|| "Failed to create client")?;
  // Create an async runtime
  let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_io()
    .build()
    .unwrap();
  // Login
  let user_login = UserLogin { name: opt.name, password: opt.password };
  runtime.block_on(async {
    client.login(&user_login).await
  }).with_context(|| "Failed to login to server")?;
  // Run command
  let command = opt.command;
  let result = runtime.block_on(async {
    run(command, &client).await
  });
  // Print metrics
  if opt.print_metrics {
    controller.observe(&mut observer);
    let output = observer.drain();
    trace!(metrics = %output);
  }
  // Exit
  Ok(result?)
}

async fn run(command: Command, client: &Client) -> Result<()> {
  match command {
    Command::ListLocalSources => {
      for local_source in client.list_local_sources().await? {
        println!("{:?}", local_source);
      }
    }
    Command::ShowLocalSourceById { id } => {
      let local_source = client.get_local_source_by_id(id).await?;
      println!("{:?}", local_source);
    }
    Command::CreateOrEnableLocalSource { directory } => {
      let local_source = client.create_or_enable_local_source(&NewLocalSource { enabled: true, directory }).await?;
      println!("{:?}", local_source);
    }
    Command::SetLocalSourceEnabledById { id, enabled } => {
      client.set_local_source_enabled_by_id(id, enabled).await?;
    }

    Command::CreateSpotifySource => {
      let url = client.create_spotify_source_authorization_url().await?;
      open::that(url)?;
    }
    Command::ShowSpotifyMe => {
      let me_info = client.show_spotify_me().await?;
      println!("{:?}", me_info);
    }

    Command::ListAlbums => {
      for (album, album_artists) in client.list_albums().await?.iter() {
        println!("{:?}", album);
        for artist in album_artists {
          println!("- {:?}", artist);
        }
      }
    }
    Command::ShowAlbumById { id } => {
      let album = client.get_album_by_id(id).await?;
      println!("{:?}", album);
    }

    Command::ListTracks => {
      let tracks = client.list_tracks().await?;
      for info in tracks.iter() {
        println!("- {:?}", info.track);
        for artist in info.track_artists() {
          println!("  * {:?}", artist);
        }
        println!("  * {:?}", info.album());
        for artist in info.album_artists() {
          println!("    - {:?}", artist);
        }
      }
    }
    Command::ShowTrackById { id } => {
      let track = client.get_track_by_id(id).await?;
      println!("{:?}", track);
    }
    Command::PlayTrack { id, volume } => {
      if let Some(play_source) = client.play_track_by_id(id).await? {
        match play_source {
          PlaySource::AudioData(audio_data) => {
            let player = musium_audio::Player::new()
              .with_context(|| "Failed to create audio player")?;
            player.play(audio_data, volume)
              .with_context(|| "Failed to play audio track")?;
          }
          PlaySource::ExternallyPlayed => {
            println!("Track has been played externally");
          }
        }
      } else {
        eprintln!("Could not play track, no track with ID '{}' was found", id);
      }
    }

    Command::ListArtists => {
      for artist in client.list_artists().await? {
        println!("{:?}", artist);
      }
    }
    Command::ShowArtistById { id } => {
      let artist = client.get_artist_by_id(id).await?;
      println!("{:?}", artist);
    }

    Command::ListUsers => {
      for user in client.list_users().await? {
        println!("{:?}", user);
      }
    }
    Command::ShowMyUser => {
      let user = client.get_my_user().await?;
      println!("{:?}", user);
    }
    Command::ShowUserById { id } => {
      let user = client.get_user_by_id(id).await?;
      println!("{:?}", user);
    }
    Command::CreateUser { name, password } => {
      let user = client.create_user(&NewUser { name, password }).await?;
      println!("{:?}", user);
    }
    Command::DeleteUserByName { name } => {
      client.delete_user_by_name(&name).await?;
    }
    Command::DeleteUserById { id } => {
      client.delete_user_by_id(id).await?;
    }

    Command::SetUserAlbumRating { album_id, rating } => {
      let rating = client.set_user_album_rating(album_id, rating).await?;
      println!("{:?}", rating);
    }
    Command::SetUserTrackRating { track_id, rating } => {
      let rating = client.set_user_track_rating(track_id, rating).await?;
      println!("{:?}", rating);
    }
    Command::SetUserArtistRating { artist_id, rating } => {
      let rating = client.set_user_artist_rating(artist_id, rating).await?;
      println!("{:?}", rating);
    }

    Command::Sync => {
      let started_sync = client.sync().await?;
      if started_sync {
        println!("Started synchronizing");
      } else {
        println!("Already synchronizing");
      }
    }
  }
  Ok(())
}
