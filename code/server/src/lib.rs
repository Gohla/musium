#[macro_use] // macro_use because diesel developers refuse to make create compatible with Rust 2018.
extern crate diesel;

use std::borrow::Borrow;
use std::path::PathBuf;

use diesel::prelude::*;
use itertools::{Either, Itertools};
use thiserror::Error;

use model::{ScanDirectory, Track};

use crate::model::NewScanDirectory;
use crate::scanner::Scanner;

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

// Listing

#[derive(Debug, Error)]
pub enum ListError {
  #[error(transparent)]
  QueryFail(#[from] diesel::result::Error),
}

impl Server {
  pub fn list_tracks(&self) -> Result<Vec<Track>, ListError> {
    use schema::track::dsl::*;
    Ok(track.load::<Track>(&self.connection)?)
  }

  pub fn list_scan_directories(&self) -> Result<Vec<ScanDirectory>, ListError> {
    use schema::scan_directory::dsl::*;
    Ok(scan_directory.load::<ScanDirectory>(&self.connection)?)
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
  ListScanDirectoriesFail(#[from] ListError),
  #[error("Failed to mutate track")]
  MutateTrackFail(#[from] diesel::result::Error),
  #[error("One or more errors occurred during scanning, but successfully scanned tracks have been added")]
  ScanFail(Vec<scanner::ScanError>),
}

impl Server {
  /// Scans all scan directories for music files, drops all tracks from the database, and adds all found tracks to the
  /// database. When a ScanFail error is returned, tracks that were sucessfully scanned will still have been added to
  /// the database.
  pub fn scan(&self) -> Result<(), ScanError> {
    let scan_directories = self.list_scan_directories()?;
    let (new_tracks, scan_errors): (Vec<_>, Vec<_>) = scan_directories
      .into_iter()
      .flat_map(|scan_directory| self.scanner.scan(scan_directory))
      .partition_map(|r| {
        match r {
          Ok(v) => Either::Left(v),
          Err(v) => Either::Right(v)
        }
      });
    {
      use schema::track;
      diesel::delete(track::table)
        .execute(&self.connection)?;
      diesel::insert_into(track::table)
        .values(new_tracks)
        .execute(&self.connection)?;
    }
    if !scan_errors.is_empty() {
      return Err(ScanError::ScanFail(scan_errors));
    }
    Ok(())
  }
}
