use std::collections::HashMap;
use std::fmt::{Display, Error, Formatter};
use std::path::PathBuf;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::schema::*;

// Scan directory

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, AsChangeset, Serialize, Deserialize)]
#[table_name = "scan_directory"]
#[changeset_options(treat_none_as_null = "true")]
pub struct ScanDirectory {
  pub id: i32,
  pub directory: String,
  pub enabled: bool,
}

impl ScanDirectory {
  pub fn track_file_path(&self, track: &Track) -> Option<PathBuf> {
    track.file_path.as_ref().map(|file_path| PathBuf::from(&self.directory).join(file_path))
  }
}

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
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

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Serialize, Deserialize)]
#[table_name = "user"]
pub struct User {
  pub id: i32,
  pub name: String,
}

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct UserLogin {
  pub name: String,
  pub password: String,
}

// User-album rating

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, AsChangeset, Serialize, Deserialize)]
#[primary_key(user_id, album_id)]
#[table_name = "user_album_rating"]
#[belongs_to(User, foreign_key = "user_id")]
#[belongs_to(Album)]
#[changeset_options(treat_none_as_null = "true")]
pub struct UserAlbumRating {
  pub user_id: i32,
  pub album_id: i32,
  pub rating: i32,
}

#[derive(Debug, Insertable, Serialize, Deserialize)]
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
#[belongs_to(User, foreign_key = "user_id")]
#[belongs_to(Track)]
#[changeset_options(treat_none_as_null = "true")]
pub struct UserTrackRating {
  pub user_id: i32,
  pub track_id: i32,
  pub rating: i32,
}

#[derive(Debug, Insertable, Serialize, Deserialize)]
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
#[belongs_to(User, foreign_key = "user_id")]
#[belongs_to(Artist)]
#[changeset_options(treat_none_as_null = "true")]
pub struct UserArtistRating {
  pub user_id: i32,
  pub artist_id: i32,
  pub rating: i32,
}

#[derive(Debug, Insertable, Serialize, Deserialize)]
#[table_name = "user_artist_rating"]
pub struct NewUserArtistRating {
  pub user_id: i32,
  pub artist_id: i32,
  pub rating: i32,
}

// Albums, including related data.

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
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

// Tracks, including related data.

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
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
