#![feature(backtrace)]

#[macro_use] // extern crate with #[macro_use] because diesel does not fully support Rust 2018 yet.
extern crate diesel;

use std::backtrace::Backtrace;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Instant;

use diesel::prelude::*;
use itertools::{Either, Itertools};
use metrics::timing;
use thiserror::Error;

use model::{ScanDirectory, Track};

use crate::model::{Album, AlbumArtist, Artist, NewAlbum, NewAlbumArtist, NewArtist, NewScanDirectory, NewTrack, NewTrackArtist, TrackArtist};
use crate::scanner::{ScannedTrack, Scanner};

pub mod schema;
pub mod model;
pub mod scanner;

macro_rules! time {
  ($s:expr, $e:expr) => {{
    let start = Instant::now();
    let result = $e;
    timing!($s, start.elapsed());
    result
  }}
}


pub struct Server {
  connection: SqliteConnection,
  scanner: Scanner,
}

// Creation

#[derive(Debug, Error)]
pub enum ServerCreateError {
  #[error("Failed to create database connection")]
  ConnectionCreateFail(#[from] ConnectionError, Backtrace),
}

impl Server {
  pub fn new<S: AsRef<str>>(database_url: S) -> Result<Server, ServerCreateError> {
    let connection = SqliteConnection::establish(database_url.as_ref())?;
    let scanner = Scanner::new();
    Ok(Server { connection, scanner })
  }
}

// Queries

#[derive(Default, Clone, Debug)]
pub struct Tracks {
  pub tracks: Vec<Track>,
  pub scan_directories: HashMap<i32, ScanDirectory>,
  pub albums: HashMap<i32, Album>,
  pub artists: HashMap<i32, Artist>,
  pub track_artists: HashMap<i32, Vec<i32>>,
  pub album_artists: HashMap<i32, Vec<i32>>,
}

impl Tracks {
  pub fn from(
    tracks: Vec<Track>,
    scan_directories: Vec<ScanDirectory>,
    albums: Vec<Album>,
    artists: Vec<Artist>,
    track_artists: Vec<TrackArtist>,
    album_artists: Vec<AlbumArtist>,
  ) -> Self {
    let scan_directories = scan_directories.into_iter().map(|sd| (sd.id, sd)).collect();
    let albums = albums.into_iter().map(|a| (a.id, a)).collect();
    let artists = artists.into_iter().map(|a| (a.id, a)).collect();
    let track_artists = track_artists.into_iter().map(|ta| (ta.track_id, ta.artist_id)).into_group_map();
    let album_artists = album_artists.into_iter().map(|aa| (aa.album_id, aa.artist_id)).into_group_map();
    Self { tracks, scan_directories, albums, artists, track_artists, album_artists }
  }

  pub fn iter(&self) -> impl Iterator<Item=(&ScanDirectory, &Track, impl Iterator<Item=&Artist>, &Album, impl Iterator<Item=&Artist>)> + '_ {
    let Tracks { tracks, scan_directories, albums, artists, track_artists, album_artists } = &self;
    tracks.into_iter().filter_map(move |track| {
      let scan_directory = scan_directories.get(&track.scan_directory_id)?;
      let track_artists: &Vec<i32> = track_artists.get(&track.id)?;
      let track_artists: Vec<&Artist> = track_artists.into_iter().filter_map(|ta| artists.get(ta)).collect();
      let album = albums.get(&track.album_id)?;
      let album_artists: &Vec<i32> = album_artists.get(&album.id)?;
      let album_artists: Vec<&Artist> = album_artists.into_iter().filter_map(|aa| artists.get(aa)).collect();
      return Some((scan_directory, track, track_artists.into_iter(), album, album_artists.into_iter()));
    })
  }
}

#[derive(Debug, Error)]
pub enum QueryError {
  #[error("Failed to execute a database query")]
  QueryFail(#[from] diesel::result::Error, Backtrace),
}

impl Server {
  pub fn list_scan_directories(&self) -> Result<Vec<ScanDirectory>, QueryError> {
    use schema::scan_directory::dsl::*;
    Ok(scan_directory.load::<ScanDirectory>(&self.connection)?)
  }

  pub fn list_tracks(&self) -> Result<Vec<Track>, QueryError> {
    use schema::track::dsl::*;
    Ok(track.load::<Track>(&self.connection)?)
  }

  pub fn list_tracks_with_associated(&self) -> Result<Tracks, QueryError> {
    let tracks = schema::track::table.load::<Track>(&self.connection)?;
    let scan_directories = schema::scan_directory::table.load::<ScanDirectory>(&self.connection)?;
    let albums = schema::album::table.load::<Album>(&self.connection)?;
    let artists = schema::artist::table.load::<Artist>(&self.connection)?;
    let track_artists = schema::track_artist::table.load::<TrackArtist>(&self.connection)?;
    let album_artists = schema::album_artist::table.load::<AlbumArtist>(&self.connection)?;
    Ok(Tracks::from(tracks, scan_directories, albums, artists, track_artists, album_artists))
  }

  pub fn list_albums_with_associated(&self) -> Result<impl Iterator<Item=(Album, impl Iterator<Item=Artist> + Debug)>, QueryError> {
    let albums: Vec<Album> = {
      use schema::album::dsl::*;
      album.load::<Album>(&self.connection)?
    };
    let album_artists: Vec<Vec<(AlbumArtist, Artist)>> = {
      use schema::artist::dsl::*;
      AlbumArtist::belonging_to(&albums)
        .inner_join(artist)
        .load(&self.connection)?
        .grouped_by(&albums)
    };
    Ok(
      albums.into_iter()
        .zip(album_artists)
        .map(|(album, album_artists)| (album, album_artists.into_iter().map(|(_, artist)| artist)))
    )
  }

  pub fn list_albums(&self) -> Result<Vec<Album>, QueryError> {
    use schema::album::dsl::*;
    Ok(album.load::<Album>(&self.connection)?)
  }

  pub fn list_artists(&self) -> Result<Vec<Artist>, QueryError> {
    use schema::artist::dsl::*;
    Ok(artist.load::<Artist>(&self.connection)?)
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
  #[error("Failed to execute a database query")]
  MutateFail(#[from] diesel::result::Error, Backtrace),
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
  ListScanDirectoriesFail(#[from] QueryError, Backtrace),
  #[error("Failed to query database")]
  DatabaseFail(#[from] diesel::result::Error, Backtrace),
  #[error("One or more errors occurred during scanning, but successfully scanned tracks have been added")]
  ScanFail(Vec<scanner::ScanError>),
}

impl Server {
  /// Scans all scan directories for music files, drops all tracks, albums, and artists from the database, and adds all
  /// found tracks, albums, and artists to the database. When a ScanFail error is returned, tracks that were sucessfully
  /// scanned will still have been added to the database.
  pub fn scan(&self) -> Result<(), ScanError> {
    let scan_directories: Vec<ScanDirectory> = time!("scan.list_scan_directories", self.list_scan_directories()?);
    let (scanned_tracks, scan_errors): (Vec<ScannedTrack>, Vec<scanner::ScanError>) = time!("scan.file_scan", {
      scan_directories
        .into_iter()
        .flat_map(|scan_directory| self.scanner.scan(scan_directory))
        .partition_map(|r| {
          match r {
            Ok(v) => Either::Left(v),
            Err(v) => Either::Right(v)
          }
        })
    });
    self.connection.transaction::<_, ScanError, _>(|| {
      // Insert tracks and related entities.
      for scanned_track in scanned_tracks {
        let scanned_track: ScannedTrack = scanned_track;
        // Get and update album, or insert it.
        let album: Album = {
          use schema::album::dsl::*;
          let album_name = scanned_track.album;
          let select_query = album
            .filter(name.eq(&album_name));
          let db_album = time!("scan.select_album", select_query.first::<Album>(&self.connection).optional()?);
          if let Some(db_album) = db_album {
            let db_album: Album = db_album;
            // TODO: update album columns when they are added.
            //time!("scan.update_album", db_album.save_changes(&self.connection)?)
            db_album
          } else {
            let new_album = NewAlbum { name: album_name.clone() };
            let insert_query = diesel::insert_into(album)
              .values(new_album);
            time!("scan.insert_album", insert_query.execute(&self.connection)?);
            time!("scan.select_inserted_album", select_query.first::<Album>(&self.connection)?)
          }
        };
        // Get and update track, or insert it.
        let track: Track = {
          use schema::track::dsl::*;
          let track_file_path = scanned_track.file_path;
          let select_query = track
            .filter(scan_directory_id.eq(scanned_track.scan_directory_id))
            .filter(file_path.eq(&track_file_path));
          let db_track = time!("scan.select_track", select_query.first::<Track>(&self.connection).optional()?);
          if let Some(db_track) = db_track {
            let mut db_track: Track = db_track;
            db_track.album_id = album.id;
            db_track.disc_number = scanned_track.disc_number;
            db_track.disc_total = scanned_track.disc_total;
            db_track.track_number = scanned_track.disc_number;
            db_track.track_total = scanned_track.track_total;
            db_track.title = scanned_track.title;
            time!("scan.update_track", db_track.save_changes(&self.connection)?)
          } else {
            let new_track = NewTrack {
              scan_directory_id: scanned_track.scan_directory_id,
              album_id: album.id,
              disc_number: scanned_track.disc_number,
              disc_total: scanned_track.disc_total,
              track_number: scanned_track.track_number,
              track_total: scanned_track.track_number,
              title: scanned_track.title,
              file_path: track_file_path.clone(),
            };
            let insert_query = diesel::insert_into(track)
              .values(new_track);
            time!("scan.insert_track", insert_query.execute(&self.connection)?);
            time!("scan.select_inserted_track", select_query.first::<Track>(&self.connection)?)
          }
        };
        // Get and update artists from track, or insert them.
        let track_artists = self.update_or_insert_artists(scanned_track.track_artists.into_iter())?;
        // Insert track-artist association if it doesn't exist.
        for db_artist in track_artists {
          let db_artist: Artist = db_artist;
          use schema::track_artist::dsl::*;
          let select_query = track_artist
            .filter(track_id.eq(track.id))
            .filter(artist_id.eq(db_artist.id));
          let db_track_artist = time!("scan.select_track_artist", select_query.first::<TrackArtist>(&self.connection).optional()?);
          if db_track_artist.is_none() {
            let new_track_artist = NewTrackArtist { track_id: track.id, artist_id: db_artist.id };
            let insert_query = diesel::insert_into(track_artist)
              .values(new_track_artist);
            time!("scan.insert_track_artist", insert_query.execute(&self.connection)?);
            time!("scan.select_inserted_track_artist", select_query.first::<TrackArtist>(&self.connection)?);
          }
        }
        // Get and update artists from albums, or insert them.
        let album_artists = self.update_or_insert_artists(scanned_track.album_artists.into_iter())?;
        // Insert album-artist association if it doesn't exist.
        for db_artist in album_artists {
          let db_artist: Artist = db_artist;
          use schema::album_artist::dsl::*;
          let select_query = album_artist
            .filter(album_id.eq(album.id))
            .filter(artist_id.eq(db_artist.id));
          let db_album_artist = time!("scan.select_album_artist", select_query.first::<AlbumArtist>(&self.connection).optional()?);
          if db_album_artist.is_none() {
            let new_album_artist = NewAlbumArtist { album_id: album.id, artist_id: db_artist.id };
            let insert_query = diesel::insert_into(album_artist)
              .values(new_album_artist);
            time!("scan.insert_album_artist", insert_query.execute(&self.connection)?);
            time!("scan.select_inserted_album_artist", select_query.first::<AlbumArtist>(&self.connection)?);
          }
        }
      }
      Ok(())
    })?;
    if !scan_errors.is_empty() {
      return Err(ScanError::ScanFail(scan_errors));
    }
    Ok(())
  }

  fn update_or_insert_artists<I: IntoIterator<Item=String>>(&self, artist_names: I) -> Result<Vec<Artist>, diesel::result::Error> {
    let artists: Result<Vec<Artist>, diesel::result::Error> = artist_names.into_iter().map(|artist_name| {
      use schema::artist::dsl::*;
      let select_query = artist
        .filter(name.eq(&artist_name));
      let db_artist = time!("scan.select_artist", select_query.first::<Artist>(&self.connection).optional()?);
      let db_artist = if let Some(db_artist) = db_artist {
        let db_artist: Artist = db_artist;
        // TODO: update artist columns when they are added.
        //time!("scan.update_artist", db_artist.save_changes(&self.connection)?)
        db_artist
      } else {
        let new_artist = NewArtist { name: artist_name.clone() };
        let insert_query = diesel::insert_into(artist)
          .values(new_artist);
        time!("scan.insert_artist", insert_query.execute(&self.connection)?);
        time!("scan.select_inserted_artist", select_query.first::<Artist>(&self.connection)?)
      };
      Ok(db_artist)
    }).collect();
    artists
  }
}
