#[macro_use] // extern crate with #[macro_use] because diesel does not fully support Rust 2018 yet.
extern crate diesel;

use std::borrow::Borrow;
use std::collections::HashSet;
use std::path::PathBuf;

use diesel::prelude::*;
use itertools::{Either, Itertools};
use thiserror::Error;

use model::{ScanDirectory, Track};

use crate::model::{Album, Artist, NewAlbum, NewArtist, NewScanDirectory, NewTrack};
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
      .flat_map(|scan_directory| self.scanner.scan(scan_directory))
      .partition_map(|r| {
        match r {
          Ok(v) => Either::Left(v),
          Err(v) => Either::Right(v)
        }
      });

    // Split into tracks, albums, artists, and associations between those which can be inserted into the database.
    // See: https://github.com/diesel-rs/diesel/issues/771
    // See: http://docs.diesel.rs/diesel/fn.replace_into.html
    // See: http://www.sqlite.org/c3ref/last_insert_rowid.html
    // See: https://stackoverflow.com/questions/52279553/what-is-the-standard-pattern-to-relate-three-tables-many-to-many-relation-with
    self.connection.transaction(|| {
      // TODO: drop tables?
      // Insert tracks and related entities.
      for scanned_track in scanned_tracks {
        let scanned_track: ScannedTrack = scanned_track;
        // Replace ('upsert') album.
        let album: Album = {
          use schema::album::dsl::*;
          let album_name = scanned_track.album;
          let new_album = NewAlbum { name: album_name.clone() };
          diesel::replace_into(album)
            .values(new_album)
            .execute(&self.connection)?;
          album
            .filter(name.eq(album_name))
            .first::<Album>(&self.connection)?
        };
        // Replace ('upsert') track
        let track: Track = {
          use schema::track::dsl::*;
          let new_track = NewTrack {
            scan_directory_id: scanned_track.scan_directory_id,
            album_id: album.id,
            disc_number: scanned_track.disc_number,
            disc_total: scanned_track.disc_total,
            track_number: scanned_track.track_number,
            track_total: scanned_track.track_number,
            title: scanned_track.title,
            file_path: scanned_track.file_path.clone(),
          };
          diesel::replace_into(track)
            .values(new_track)
            .execute(&self.connection)?;
          track
            .filter(scan_directory_id.eq(scanned_track.scan_directory_id))
            .filter(file_path.eq(scanned_track.file_path))
            .first::<Track>(&self.connection)?
        };
        // Replace ('upsert') artist.
        let artists: Result<Vec<Artist>, _> = scanned_track.artist.iter().map(|artist_name| {
          use schema::artist::dsl::*;
          let new_artist = NewArtist { name: artist_name.clone() };
          diesel::replace_into(artist)
            .values(new_artist)
            .execute(&self.connection)?;
          Ok(artist
            .filter(name.eq(artist_name))
            .first::<Artist>(&self.connection)?)
        }).collect();
        let artists = artists?;
      }
      Ok(())
    });

    if !scan_errors.is_empty() {
      return Err(ScanError::ScanFail(scan_errors));
    }
    Ok(())
  }
}
