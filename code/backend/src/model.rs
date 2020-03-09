use std::fmt::{Display, Error, Formatter};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::scanner::ScannedTrack;
use crate::schema::*;

// Helper macros

macro_rules! update {
  ($t:expr, $u:expr, $c:expr) => {
    if $t != $u {
      //event!(Level::TRACE, old = ?$t, new = ?$u, "Value changed");
      $t = $u;
      $c = true;
    }
  }
}

// Scan directory

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, AsChangeset, Serialize, Deserialize)]
#[table_name = "scan_directory"]
#[changeset_options(treat_none_as_null = "true")]
pub struct ScanDirectory {
  pub id: i32,
  pub directory: String,
  pub enabled: bool,
}

#[derive(Debug, Insertable)]
#[table_name = "scan_directory"]
pub struct NewScanDirectory {
  pub directory: String,
  pub enabled: bool,
}

// Album

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, AsChangeset, Serialize, Deserialize)]
#[table_name = "album"]
#[changeset_options(treat_none_as_null = "true")]
pub struct Album {
  pub id: i32,
  pub name: String,
}

#[derive(Debug, Insertable)]
#[table_name = "album"]
pub struct NewAlbum {
  pub name: String,
}

// Track

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, AsChangeset, Serialize, Deserialize)]
#[belongs_to(ScanDirectory)]
#[belongs_to(Album)]
#[table_name = "track"]
#[changeset_options(treat_none_as_null = "true")]
pub struct Track {
  pub id: i32,
  pub scan_directory_id: i32,
  pub album_id: i32,
  pub disc_number: Option<i32>,
  pub disc_total: Option<i32>,
  pub track_number: Option<i32>,
  pub track_total: Option<i32>,
  pub title: String,
  pub file_path: Option<String>,
  pub hash: i64,
}

#[derive(Debug, Insertable)]
#[table_name = "track"]
pub struct NewTrack {
  pub scan_directory_id: i32,
  pub album_id: i32,
  pub disc_number: Option<i32>,
  pub disc_total: Option<i32>,
  pub track_number: Option<i32>,
  pub track_total: Option<i32>,
  pub title: String,
  pub file_path: Option<String>,
  pub hash: i64,
}

// Artist

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, AsChangeset, Serialize, Deserialize)]
#[table_name = "artist"]
#[changeset_options(treat_none_as_null = "true")]
pub struct Artist {
  pub id: i32,
  pub name: String,
}

#[derive(Debug, Insertable)]
#[table_name = "artist"]
pub struct NewArtist {
  pub name: String,
}

// Track-artist

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, Serialize, Deserialize)]
#[primary_key(track_id, artist_id)]
#[table_name = "track_artist"]
#[belongs_to(Track)]
#[belongs_to(Artist)]
pub struct TrackArtist {
  pub track_id: i32,
  pub artist_id: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "track_artist"]
pub struct NewTrackArtist {
  pub track_id: i32,
  pub artist_id: i32,
}

// Album-artist

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, Serialize, Deserialize)]
#[primary_key(album_id, artist_id)]
#[table_name = "album_artist"]
#[belongs_to(Album)]
#[belongs_to(Artist)]
pub struct AlbumArtist {
  pub album_id: i32,
  pub artist_id: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "album_artist"]
pub struct NewAlbumArtist {
  pub album_id: i32,
  pub artist_id: i32,
}

// User

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, AsChangeset)]
#[table_name = "user"]
#[changeset_options(treat_none_as_null = "true")]
pub(crate) struct InternalUser {
  pub id: i32,
  pub name: String,
  pub hash: Vec<u8>,
  pub salt: Vec<u8>,
}

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Serialize, Deserialize)]
#[table_name = "user"]
pub struct User {
  pub id: i32,
  pub name: String,
}

#[derive(Debug, Insertable)]
#[table_name = "user"]
pub struct NewUser {
  pub name: String,
  pub hash: Vec<u8>,
  pub salt: Vec<u8>,
}

// User-album rating

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, AsChangeset, Serialize, Deserialize)]
#[primary_key(user_id, album_id)]
#[table_name = "user_album_rating"]
#[belongs_to(InternalUser, foreign_key = "user_id")]
#[belongs_to(Album)]
#[changeset_options(treat_none_as_null = "true")]
pub struct UserAlbumRating {
  pub user_id: i32,
  pub album_id: i32,
  pub rating: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "user_album_rating"]
pub struct NewUserAlbumRating {
  pub user_id: i32,
  pub album_id: i32,
  pub rating: i32,
}

// User-track rating

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, AsChangeset, Serialize, Deserialize)]
#[primary_key(user_id, track_id)]
#[table_name = "user_track_rating"]
#[belongs_to(InternalUser, foreign_key = "user_id")]
#[belongs_to(Track)]
#[changeset_options(treat_none_as_null = "true")]
pub struct UserTrackRating {
  pub user_id: i32,
  pub track_id: i32,
  pub rating: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "user_track_rating"]
pub struct NewUserTrackRating {
  pub user_id: i32,
  pub track_id: i32,
  pub rating: i32,
}

// User-artist rating

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, AsChangeset, Serialize, Deserialize)]
#[primary_key(user_id, artist_id)]
#[table_name = "user_artist_rating"]
#[belongs_to(InternalUser, foreign_key = "user_id")]
#[belongs_to(Artist)]
#[changeset_options(treat_none_as_null = "true")]
pub struct UserArtistRating {
  pub user_id: i32,
  pub artist_id: i32,
  pub rating: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "user_artist_rating"]
pub struct NewUserArtistRating {
  pub user_id: i32,
  pub artist_id: i32,
  pub rating: i32,
}

// Implementations

impl ScanDirectory {
  pub fn track_file_path(&self, track: &Track) -> Option<PathBuf> {
    track.file_path.as_ref().map(|file_path| PathBuf::from(&self.directory).join(file_path))
  }

  pub fn update_from(
    &mut self,
    enabled: bool,
  ) -> bool {
    let mut changed = false;
    update!(self.enabled, enabled, changed);
    changed
  }
}

impl Track {
  pub fn check_hash_changed(&mut self, scanned_track: &ScannedTrack) -> bool {
    self.hash != scanned_track.hash as i64
  }

  pub fn check_metadata_changed(&mut self, album: &Album, scanned_track: &ScannedTrack) -> bool {
    if self.scan_directory_id != scanned_track.scan_directory_id { return true; }
    if self.album_id != album.id { return true; }
    if self.disc_number != scanned_track.disc_number { return true; }
    if self.disc_total != scanned_track.disc_total { return true; }
    if self.track_number != scanned_track.track_number { return true; }
    if self.track_total != scanned_track.track_total { return true; }
    if self.title != scanned_track.title { return true; }
    return false;
  }

  pub fn update_from(
    &mut self,
    album: &Album,
    scanned_track: &ScannedTrack,
  ) -> bool {
    let mut changed = false;
    update!(self.scan_directory_id, scanned_track.scan_directory_id, changed);
    update!(self.album_id, album.id, changed);
    update!(self.disc_number, scanned_track.disc_number, changed);
    update!(self.disc_total, scanned_track.disc_total, changed);
    update!(self.track_number, scanned_track.track_number, changed);
    update!(self.track_total, scanned_track.track_total, changed);
    update!(self.title, scanned_track.title.clone(), changed);
    if let Some(file_path) = &mut self.file_path {
      if file_path != &scanned_track.file_path {
        *file_path = scanned_track.file_path.clone();
        changed = true;
      }
    } else {
      self.file_path = Some(scanned_track.file_path.clone());
      changed = true;
    }
    update!(self.hash, scanned_track.hash as i64, changed);
    changed
  }
}

// Display implementations

impl Display for Track {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
    write!(f, "{:>6}:", self.id)?;
    match (self.disc_number, self.disc_total) {
      (Some(number), Some(total)) => write!(f, " ({}/{})", number, total)?,
      (Some(number), _) => write!(f, "   ({})", number)?,
      _ => write!(f, "      ")?,
    }
    match (self.track_number, self.track_total) {
      (Some(number), Some(total)) => write!(f, " {:>3}/{:>3}.", number, total)?,
      (Some(number), _) => write!(f, "     {:>3}.", number)?,
      _ => write!(f, "         ")?,
    }
    write!(f, " {:<50}", self.title)?;
    if let Some(file_path) = &self.file_path {
      write!(f, " - {}", file_path)?;
    }
    Ok(())
  }
}

impl Display for ScanDirectory {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
    f.write_str(&self.directory)
  }
}
