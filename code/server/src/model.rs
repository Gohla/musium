use std::fmt::{Display, Error, Formatter};
use std::path::PathBuf;

use crate::schema::{album, album_artist, artist, scan_directory, track, track_artist};

// Track

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, AsChangeset)]
#[belongs_to(ScanDirectory)]
#[belongs_to(Album)]
#[table_name = "track"]
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
}

// Scan directory

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, AsChangeset)]
#[table_name = "scan_directory"]
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
pub struct Album {
  pub id: i32,
  pub name: String,
}

#[derive(Debug, Insertable)]
#[table_name = "album"]
pub struct NewAlbum {
  pub name: String,
}

// Artist

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, AsChangeset)]
#[table_name = "artist"]
pub struct Artist {
  pub id: i32,
  pub name: String,
}

#[derive(Debug, Insertable)]
#[table_name = "artist"]
pub struct NewArtist {
  pub name: String,
}

// Track artist

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations)]
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

// Album artist

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations)]
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
