#[macro_use] // extern crate with #[macro_use] because diesel does not fully support Rust 2018 yet.
extern crate diesel;

use std::borrow::Borrow;
use std::collections::HashSet;
use std::path::PathBuf;

use diesel::prelude::*;
use itertools::{Either, Itertools};
use thiserror::Error;

use model::{ScanDirectory, Track};

use crate::model::{NewScanDirectory, NewTrack};
use crate::scanner::{ScannedTrack, Scanner};

pub mod schema;
pub mod model;
pub mod scanner;

pub struct Server {
  connection: SqliteConnection,
  scanner: Scanner,
}

// Creation

#[derive(Debug, Error)]
pub enum ServerCreateError {
  #[error(transparent)]
  ConnectionCreateFail(#[from] ConnectionError),
}

impl Server {
  pub fn new<S: AsRef<str>>(database_url: S) -> Result<Server, ServerCreateError> {
    let connection = SqliteConnection::establish(database_url.as_ref())?;
    let scanner = Scanner::new();
    Ok(Server { connection, scanner })
  }
}

// Queries

#[derive(Debug, Error)]
pub enum QueryError {
  #[error(transparent)]
  QueryFail(#[from] diesel::result::Error),
}

impl Server {
  pub fn list_tracks(&self) -> Result<Vec<Track>, QueryError> {
    use schema::track::dsl::*;
    Ok(track.load::<Track>(&self.connection)?)
  }

  pub fn list_scan_directories(&self) -> Result<Vec<ScanDirectory>, QueryError> {
    use schema::scan_directory::dsl::*;
    Ok(scan_directory.load::<ScanDirectory>(&self.connection)?)
  }

  pub fn list_scan_directories_with_tracks(&self) -> Result<impl Iterator<Item=(ScanDirectory, Vec<Track>)>, QueryError> {
    let scan_directories = {
      use schema::scan_directory::dsl::*;
      scan_directory.load::<ScanDirectory>(&self.connection)?
    };
    let tracks = Track::belonging_to(&scan_directories)
      .load::<Track>(&self.connection)?
      .grouped_by(&scan_directories);
    Ok(scan_directories.into_iter().zip(tracks))
  }

  pub fn get_track_by_id(&self, id: i32) -> Result<Option<(ScanDirectory, Track)>, QueryError> {
    use schema::{track, scan_directory};
    if let Some(track) = track::dsl::track.find(id).first::<Track>(&self.connection).optional()? {
      let scan_directory = scan_directory::dsl::scan_directory.filter(scan_directory::dsl::id.eq(track.scan_directory_id)).first::<ScanDirectory>(&self.connection)?;
      Ok(Some((scan_directory, track)))
    } else {
      Ok(None)
    }
  }
}

// Mutation

#[derive(Debug, Error)]
pub enum MutateError {
  #[error(transparent)]
  MutateFail(#[from] diesel::result::Error),
}

impl Server {
  pub fn add_scan_directory<P: Borrow<PathBuf>>(&self, directory: P) -> Result<(), MutateError> {
    use schema::scan_directory;
    let directory = directory.borrow().to_string_lossy().to_string();
    diesel::insert_into(scan_directory::table)
      .values(NewScanDirectory { directory })
      .execute(&self.connection)?;
    Ok(())
  }

  pub fn remove_scan_directory<P: Borrow<PathBuf>>(&self, directory: P) -> Result<bool, MutateError> {
    let directory_input = directory.borrow().to_string_lossy().to_string();
    {
      use schema::scan_directory::dsl::*;
      let result = diesel::delete(scan_directory.filter(directory.like(directory_input)))
        .execute(&self.connection)?;
      Ok(result == 1)
    }
  }
}

// Scanning

#[derive(Debug, Error)]
pub enum ScanError {
  #[error("Failed to list scan directories")]
  ListScanDirectoriesFail(#[from] QueryError),
  #[error("Failed to mutate track")]
  MutateTrackFail(#[from] diesel::result::Error),
  #[error("One or more errors occurred during scanning, but successfully scanned tracks have been added")]
  ScanFail(Vec<scanner::ScanError>),
}

impl Server {
  /// Scans all scan directories for music files, drops all tracks, albums, and artists from the database, and adds all
  /// found tracks, albums, and artists to the database. When a ScanFail error is returned, tracks that were sucessfully
  /// scanned will still have been added to the database.
  pub fn scan(&self) -> Result<(), ScanError> {
    // Scan for all tracks.
    let scan_directories: Vec<ScanDirectory> = self.list_scan_directories()?;
    let (scanned_tracks, scan_errors): (Vec<ScannedTrack>, Vec<scanner::ScanError>) = scan_directories
      .into_iter()
      .flat_map(|scan_directory| self.scanner.scan(scan_directory.directory))
      .partition_map(|r| {
        match r {
          Ok(v) => Either::Left(v),
          Err(v) => Either::Right(v)
        }
      });

    // Split into tracks, albums, artists, and associations between those which can be inserted into the database.
    self.connection.transaction(||{
      let tracks = Vec::new();
      let albums = HashSet::new();
      let artists = HashSet::new();
      let track_artists = HashSet::new();
      let album_artists = HashSet::new();
      for scanned_track in scanned_tracks {
        let scanned_track: ScannedTrack = scanned_track;
        let album = {
          use schema::album;
          diesel::replace_into(album::table).values()
        }
        {
          let new_track = NewTrack {
            scan_directory_id: scanned_track.scan_directory.id,
            album_id: 0,
            disc_number: Option::None,
            disc_total: Option::None,
            track_number: Option::None,
            track_total: Option::None,
            title: Option::None,
            file_path: "".to_string()
          };
        }
      }
      Ok(())
    });


    // {
    //   use schema::track;
    //   diesel::delete(track::table)
    //     .execute(&self.connection)?;
    //   diesel::insert_into(track::table)
    //     .values(scanned_tracks)
    //     .execute(&self.connection)?;
    // }
    if !scan_errors.is_empty() {
      return Err(ScanError::ScanFail(scan_errors));
    }
    Ok(())
  }
}
