#![feature(backtrace)]

#[macro_use] // extern crate with #[macro_use] because diesel does not fully support Rust 2018 yet.
extern crate diesel;

use std::backtrace::Backtrace;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::time::Instant;

use diesel::prelude::*;
use itertools::{Either, Itertools};
use metrics::timing;
use thiserror::Error;
use tracing::{event, instrument, Level};

use model::{ScanDirectory, Track};

use crate::model::*;
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

// Database queries

#[derive(Debug, Error)]
pub enum DatabaseQueryError {
  #[error("Failed to execute a database query")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
}

// Scan directory database queries

impl Server {
  pub fn list_scan_directories(&self) -> Result<Vec<ScanDirectory>, DatabaseQueryError> {
    use schema::scan_directory::dsl::*;
    Ok(scan_directory.load::<ScanDirectory>(&self.connection)?)
  }

  pub fn add_scan_directory<P: Borrow<PathBuf>>(&self, directory: P) -> Result<ScanDirectory, DatabaseQueryError> {
    use schema::scan_directory;
    let directory = directory.borrow().to_string_lossy().to_string();
    time!("add_scan_directory.insert", diesel::insert_into(scan_directory::table)
      .values(NewScanDirectory { directory: directory.clone() })
      .execute(&self.connection)?);
    let select_query = scan_directory::table
      .filter(scan_directory::directory.eq(&directory));
    Ok(time!("add_scan_directory.select", select_query.first::<ScanDirectory>(&self.connection)?))
  }

  pub fn remove_scan_directory<P: Borrow<PathBuf>>(&self, directory: P) -> Result<bool, DatabaseQueryError> {
    use schema::scan_directory;
    let directory = directory.borrow().to_string_lossy().to_string();
    let result = time!("remove_scan_directory.delete", diesel::delete(scan_directory::table.filter(scan_directory::directory.like(&directory)))
      .execute(&self.connection))?;
    Ok(result == 1)
  }
}

// Album database queries

pub struct Albums {
  pub albums: Vec<Album>,
  pub artists: HashMap<i32, Artist>,
  pub album_artists: HashMap<i32, Vec<i32>>,
}

impl Albums {
  pub fn from(
    albums: Vec<Album>,
    artists: Vec<Artist>,
    album_artists: Vec<AlbumArtist>,
  ) -> Self {
    let artists = artists.into_iter().map(|a| (a.id, a)).collect();
    let album_artists = album_artists.into_iter().map(|aa| (aa.album_id, aa.artist_id)).into_group_map();
    Self { albums, artists, album_artists }
  }

  pub fn iter(&self) -> impl Iterator<Item=(&Album, impl Iterator<Item=&Artist>)> + '_ {
    let Albums { albums, artists, album_artists } = &self;
    albums.into_iter().filter_map(move |album| {
      let album_artists: &Vec<i32> = album_artists.get(&album.id)?;
      let album_artists: Vec<&Artist> = album_artists.into_iter().filter_map(|aa| artists.get(aa)).collect();
      return Some((album, album_artists.into_iter()));
    })
  }
}

impl Server {
  pub fn list_albums(&self) -> Result<Albums, DatabaseQueryError> {
    let albums = schema::album::table.load::<Album>(&self.connection)?;
    let artists = schema::artist::table.load::<Artist>(&self.connection)?;
    let album_artists = schema::album_artist::table.load::<AlbumArtist>(&self.connection)?;
    Ok(Albums::from(albums, artists, album_artists))
  }
}

// Track database queries

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

impl Server {
  pub fn list_tracks(&self) -> Result<Tracks, DatabaseQueryError> {
    let tracks = schema::track::table.load::<Track>(&self.connection)?;
    let scan_directories = schema::scan_directory::table.load::<ScanDirectory>(&self.connection)?;
    let albums = schema::album::table.load::<Album>(&self.connection)?;
    let artists = schema::artist::table.load::<Artist>(&self.connection)?;
    let track_artists = schema::track_artist::table.load::<TrackArtist>(&self.connection)?;
    let album_artists = schema::album_artist::table.load::<AlbumArtist>(&self.connection)?;
    Ok(Tracks::from(tracks, scan_directories, albums, artists, track_artists, album_artists))
  }

  pub fn get_track_by_id(&self, id: i32) -> Result<Option<(ScanDirectory, Track)>, DatabaseQueryError> {
    use schema::{track, scan_directory};
    if let Some(track) = track::dsl::track.find(id).first::<Track>(&self.connection).optional()? {
      let scan_directory = scan_directory::dsl::scan_directory.filter(scan_directory::dsl::id.eq(track.scan_directory_id)).first::<ScanDirectory>(&self.connection)?;
      Ok(Some((scan_directory, track)))
    } else {
      Ok(None)
    }
  }
}

// Artist database queries

impl Server {
  pub fn list_artists(&self) -> Result<Vec<Artist>, DatabaseQueryError> {
    use schema::artist::dsl::*;
    Ok(artist.load::<Artist>(&self.connection)?)
  }
}

// User database queries

impl Server {
  pub fn list_users(&self) -> Result<Vec<User>, DatabaseQueryError> {
    use schema::user::dsl::*;
    Ok(user.load::<User>(&self.connection)?)
  }

  pub fn add_user<S: Into<String>>(&self, name: S) -> Result<User, DatabaseQueryError> {
    use schema::user;
    let name = name.into();
    time!("add_user.insert", diesel::insert_into(user::table)
      .values(NewUser { name: name.clone() })
      .execute(&self.connection)?);
    let select_query = user::table
      .filter(user::name.eq(&name));
    Ok(time!("add_user.select", select_query.first::<User>(&self.connection)?))
  }

  pub fn remove_user<S: AsRef<str>>(&self, name: S) -> Result<bool, DatabaseQueryError> {
    use schema::user;
    let name = name.as_ref();
    let result = time!("remove_user.delete", diesel::delete(user::table.filter(user::name.like(name)))
      .execute(&self.connection)?);
    Ok(result == 1)
  }
}

// User data database queries

impl Server {
  pub fn set_user_album_rating(&self, user_id: i32, album_id: i32, rating: i32) -> Result<UserAlbumRating, DatabaseQueryError> {
    use schema::user_album_rating;
    let select_query = user_album_rating::table
      .filter(user_album_rating::user_id.eq(user_id))
      .filter(user_album_rating::album_id.eq(album_id));
    let db_user_album_rating = time!("set_user_album_rating.select", select_query.first::<UserAlbumRating>(&self.connection).optional()?);
    if let Some(db_user_album_rating) = db_user_album_rating {
      let mut db_user_album_rating: UserAlbumRating = db_user_album_rating;
      db_user_album_rating.rating = rating;
      Ok(time!("set_user_album_rating.update", db_user_album_rating.save_changes(&self.connection)?))
    } else {
      time!("set_user_album_rating.insert", diesel::insert_into(user_album_rating::table)
        .values(NewUserAlbumRating { user_id, album_id, rating })
        .execute(&self.connection)?);
      Ok(time!("set_user_album_rating.select_inserted", select_query.first::<UserAlbumRating>(&self.connection)?))
    }
  }

  pub fn set_user_track_rating(&self, user_id: i32, track_id: i32, rating: i32) -> Result<UserTrackRating, DatabaseQueryError> {
    use schema::user_track_rating;
    let select_query = user_track_rating::table
      .filter(user_track_rating::user_id.eq(user_id))
      .filter(user_track_rating::track_id.eq(track_id));
    let db_user_track_rating = time!("set_user_track_rating.select", select_query.first::<UserTrackRating>(&self.connection).optional()?);
    if let Some(db_user_track_rating) = db_user_track_rating {
      let mut db_user_track_rating: UserTrackRating = db_user_track_rating;
      db_user_track_rating.rating = rating;
      Ok(time!("set_user_track_rating.update", db_user_track_rating.save_changes(&self.connection)?))
    } else {
      time!("set_user_track_rating.insert", diesel::insert_into(user_track_rating::table)
        .values(NewUserTrackRating { user_id, track_id, rating })
        .execute(&self.connection)?);
      Ok(time!("set_user_track_rating.select_inserted", select_query.first::<UserTrackRating>(&self.connection)?))
    }
  }

  pub fn set_user_artist_rating(&self, user_id: i32, artist_id: i32, rating: i32) -> Result<UserArtistRating, DatabaseQueryError> {
    use schema::user_artist_rating;
    let select_query = user_artist_rating::table
      .filter(user_artist_rating::user_id.eq(user_id))
      .filter(user_artist_rating::artist_id.eq(artist_id));
    let db_user_artist_rating = time!("set_user_artist_rating.select", select_query.first::<UserArtistRating>(&self.connection).optional()?);
    if let Some(db_user_artist_rating) = db_user_artist_rating {
      let mut db_user_artist_rating: UserArtistRating = db_user_artist_rating;
      db_user_artist_rating.rating = rating;
      Ok(time!("set_user_artist_rating.update", db_user_artist_rating.save_changes(&self.connection)?))
    } else {
      time!("set_user_artist_rating.insert", diesel::insert_into(user_artist_rating::table)
        .values(NewUserArtistRating { user_id, artist_id, rating })
        .execute(&self.connection)?);
      Ok(time!("set_user_artist_rating.select_inserted", select_query.first::<UserArtistRating>(&self.connection)?))
    }
  }
}

// Scanning

#[derive(Debug, Error)]
pub enum ScanError {
  #[error("Failed to list scan directories")]
  ListScanDirectoriesFail(#[from] DatabaseQueryError, Backtrace),
  #[error("Failed to query database")]
  DatabaseFail(#[from] diesel::result::Error, Backtrace),
  #[error("Attempted to update possibly moved track {0:?}, but found multiple tracks in the database with the same scan directory and hash: {1:?}")]
  HashCollisionFail(ScannedTrack, Vec<Track>),
  #[error("One or more errors occurred during scanning, but successfully scanned tracks have been added")]
  ScanFail(Vec<scanner::ScanError>),
}

impl Server {
  #[instrument]
  /// Scans all scan directories for music files, drops all tracks, albums, and artists from the database, and adds all
  /// found tracks, albums, and artists to the database. When a ScanFail error is returned, tracks that were sucessfully
  /// scanned will still have been added to the database.
  pub fn scan(&self) -> Result<(), ScanError> {
    use ScanError::*;

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
        event!(Level::TRACE, ?scanned_track, "Processing scanned track");
        // Get and update album, or insert it.
        let album: Album = {
          use schema::album::dsl::*;
          let album_name = scanned_track.album.clone();
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
            event!(Level::DEBUG, ?new_album, "Inserting album");
            let insert_query = diesel::insert_into(album)
              .values(new_album);
            time!("scan.insert_album", insert_query.execute(&self.connection)?);
            time!("scan.select_inserted_album", select_query.first::<Album>(&self.connection)?)
          }
        };
        // Get and update track, or insert it.
        let track: Track = {
          use schema::track::dsl::*;
          let track_file_path = scanned_track.file_path.clone();
          let select_query = track
            .filter(scan_directory_id.eq(scanned_track.scan_directory_id))
            .filter(file_path.eq(&track_file_path));
          let db_track = time!("scan.select_track", select_query.first::<Track>(&self.connection).optional()?);
          if let Some(db_track) = db_track {
            let mut db_track: Track = db_track;
            let changed = db_track.update_from(&album, &scanned_track);
            if changed {
              event!(Level::DEBUG, ?db_track, "Updating track");
              time!("scan.update_track", db_track.save_changes(&self.connection)?)
            } else {
              db_track
            }
          } else {
            // First attempt to find a track with the same hash, and in the same scan directory, as the scanned track.
            let select_by_hash_query = track
              .filter(scan_directory_id.eq(scanned_track.scan_directory_id))
              .filter(hash.eq(scanned_track.hash as i64));
            let tracks_by_hash: Vec<Track> = time!("scan.select_track_by_hash", select_by_hash_query.load::<Track>(&self.connection)?);
            if tracks_by_hash.is_empty() {
              // No track with the same has was found, insert a new track.
              let new_track = NewTrack {
                scan_directory_id: scanned_track.scan_directory_id,
                album_id: album.id,
                disc_number: scanned_track.disc_number,
                disc_total: scanned_track.disc_total,
                track_number: scanned_track.track_number,
                track_total: scanned_track.track_total,
                title: scanned_track.title,
                file_path: track_file_path.clone(),
                hash: scanned_track.hash as i64,
              };
              event!(Level::DEBUG, ?new_track, "Inserting track");
              let insert_query = diesel::insert_into(track)
                .values(new_track);
              time!("scan.insert_track", insert_query.execute(&self.connection)?);
              time!("scan.select_inserted_track", select_query.first::<Track>(&self.connection)?)
            } else if tracks_by_hash.len() == 1 {
              let mut db_track: Track = tracks_by_hash.into_iter().take(1).next().unwrap();
              let changed = db_track.update_from(&album, &scanned_track);
              if changed {
                event!(Level::DEBUG, ?db_track, "Updating moved track");
                time!("scan.update_moved_track", db_track.save_changes(&self.connection)?)
              } else {
                db_track
              }
            } else {
              // For now, if we find multiple tracks with the same hash, we error out.
              return Err(HashCollisionFail(scanned_track.clone(), tracks_by_hash));
            }
          }
        };
        // Process track artists.
        {
          let mut db_artists: HashSet<Artist> = self.update_or_insert_artists(scanned_track.track_artists.into_iter())?;
          use schema::track_artist::dsl::*;
          let db_track_artists: Vec<(TrackArtist, Artist)> = time!("scan.select_track_artists", track_artist
            .filter(track_id.eq(track.id))
            .inner_join(schema::artist::table)
            .load(&self.connection)?);
          for (db_track_artist, db_artist) in db_track_artists {
            if db_artists.contains(&db_artist) {
              // TODO: update track_artist columns if they are added.
              //let mut db_track_artist = db_track_artist;
              //time!("scan.update_track_artist", db_track_artist.save_changes(&self.connection)?)
            } else {
              event!(Level::DEBUG, ?db_track_artist, "Deleting track artist");
              time!("scan.delete_track_artist", diesel::delete(&db_track_artist).execute(&self.connection)?);
            }
            db_artists.remove(&db_artist); // Remove from set, so we know what to insert afterwards.
          }
          for artist in db_artists {
            let new_track_artist = NewTrackArtist { track_id: track.id, artist_id: artist.id };
            event!(Level::DEBUG, ?new_track_artist, "Inserting track artist");
            time!("scan.insert_track_artist", diesel::insert_into(track_artist)
              .values(new_track_artist)
              .execute(&self.connection)?);
          }
        }
        // Process album artists.
        {
          let mut db_artists: HashSet<Artist> = self.update_or_insert_artists(scanned_track.album_artists.into_iter())?;
          use schema::album_artist::dsl::*;
          let db_album_artists: Vec<(AlbumArtist, Artist)> = time!("scan.select_album_artists", album_artist
            .filter(album_id.eq(album.id))
            .inner_join(schema::artist::table)
            .load(&self.connection)?);
          for (db_album_artist, db_artist) in db_album_artists {
            if db_artists.contains(&db_artist) {
              // TODO: update album_artist columns if they are added.
              //let mut db_album_artist = db_album_artist;
              //time!("scan.update_album_artist", db_album_artist.save_changes(&self.connection)?)
            } else {
              event!(Level::DEBUG, ?db_album_artist, "Deleting album artist");
              time!("scan.delete_album_artist", diesel::delete(&db_album_artist).execute(&self.connection)?);
            }
            db_artists.remove(&db_artist); // Remove from set, so we know what to insert afterwards.
          }
          for artist in db_artists {
            let new_album_artist = NewAlbumArtist { album_id: album.id, artist_id: artist.id };
            event!(Level::DEBUG, ?new_album_artist, "Inserting album artist");
            time!("scan.insert_album_artist", diesel::insert_into(album_artist)
              .values(new_album_artist)
              .execute(&self.connection)?);
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

  fn update_or_insert_artists<I: IntoIterator<Item=String>>(&self, artist_names: I) -> Result<HashSet<Artist>, diesel::result::Error> {
    artist_names.into_iter().map(|artist_name| {
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
    }).collect()
  }
}

// Implementations

impl Debug for Server {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Ok(write!(f, "Server")?)
  }
}
