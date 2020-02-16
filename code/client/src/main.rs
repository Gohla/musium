use std::path::PathBuf;

use anyhow::{Context, Result};
use structopt::StructOpt;

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
  /// Lists all music scan directories in the database
  #[structopt()]
  ListScanDirectories,
  /// Add a music scan directory to the database
  #[structopt()]
  AddScanDirectory {
    /// Music scan directory to add
    #[structopt(parse(from_os_str))]
    directory: PathBuf,
  },
  /// Removes a music scan directory to the database
  #[structopt()]
  RemoveScanDirectory {
    /// Music scan directory to remove
    #[structopt(parse(from_os_str))]
    directory: PathBuf,
  },
  /// Scan for music files in all scan directories, and add their tracks to the database
  #[structopt()]
  Scan,
}

fn main() -> Result<()> {
  let opt: Opt = Opt::from_args();
  let server: Server = Server::new(opt.database_file.to_string_lossy())
    .with_context(|| "Failed to initialize server")?;
  match opt.command {
    Command::ListTracks => {
      for track in server.list_tracks().with_context(|| "Failed to list tracks")? {
        println!("{}", track);
      }
    },
    Command::ListScanDirectories => {
      for scan_directory in server.list_scan_directories().with_context(|| "Failed to list scan directories")? {
        println!("{}", scan_directory);
      }
    },
    Command::AddScanDirectory { directory } => {
      server.add_scan_directory(&directory).with_context(|| "Failed to add scan directory")?;
      println!("Added scan directory '{}'", directory.display());
    },
    Command::RemoveScanDirectory { directory } => {
      let removed = server.remove_scan_directory(&directory).with_context(|| "Failed to remove scan directory")?;
      if removed {
        println!("Removed scan directory '{}'", directory.display());
      } else {
        println!("Could not remove scan directory '{}', it was not found", directory.display());
      }
    },
    Command::Scan => {
      server.scan().with_context(|| "Failed to scan music files")?;
    },
  }
  Ok(())
}
