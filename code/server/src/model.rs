use std::fmt::{Display, Error, Formatter};
use std::path::PathBuf;

use crate::schema::*;

// Scan directory

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, AsChangeset)]
#[table_name = "scan_directory"]
#[changeset_options(treat_none_as_null = "true")]
pub struct ScanDirectory {
  pub id: i32,
  pub directory: String,
}

#[derive(Debug, Insertable)]
#[table_name = "scan_directory"]
pub struct NewScanDirectory {
  pub directory: String,
}

// Album

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, AsChangeset)]
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

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, AsChangeset)]
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
  pub file_path: String,
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
  pub file_path: String,
  pub hash: i64,
}

// Artist

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, AsChangeset)]
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

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations)]
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

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations)]
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
pub struct User {
  pub id: i32,
  pub name: String,
}

#[derive(Debug, Insertable)]
#[table_name = "user"]
pub struct NewUser {
  pub name: String,
}

// User-album rating

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, AsChangeset)]
#[primary_key(user_id, album_id)]
#[table_name = "user_album_rating"]
#[belongs_to(User)]
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

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, AsChangeset)]
#[primary_key(user_id, track_id)]
#[table_name = "user_track_rating"]
#[belongs_to(User)]
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

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, AsChangeset)]
#[primary_key(user_id, artist_id)]
#[table_name = "user_artist_rating"]
#[belongs_to(User)]
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
  pub fn track_file_path(&self, track: &Track) -> PathBuf {
    PathBuf::from(&self.directory).join(&track.file_path)
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
    write!(f, " - {}", self.file_path)?;
    Ok(())
  }
}

impl Display for ScanDirectory {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
    f.write_str(&self.directory)
  }
}
