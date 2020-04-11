use std::fmt::{Display, Error, Formatter};

use serde::{Deserialize, Serialize};

use crate::schema::*;

pub mod collection;

//
// Sources
//

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, AsChangeset, Serialize, Deserialize)]
#[table_name = "source"]
#[changeset_options(treat_none_as_null = "true")]
pub struct Source {
  pub id: i32,
  pub enabled: bool,
  pub data: String, // TODO: use Sources enum
}

#[derive(Clone, PartialOrd, PartialEq, Debug, Serialize, Deserialize)]
pub enum Sources {
  Local { directory: String }
}

// impl Source {
//   pub fn track_file_path(&self, track: &Track) -> Option<PathBuf> {
//     track.file_path.as_ref().map(|file_path| PathBuf::from(&self.directory).join(file_path))
//   }
// }

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[table_name = "source"]
pub struct NewSource {
  pub enabled: bool,
  pub data: String, // TODO: use Sources enum
}


//
// Album/Track/Artist data, and relations between them.
//

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
#[belongs_to(Album)]
#[table_name = "track"]
#[changeset_options(treat_none_as_null = "true")]
pub struct Track {
  pub id: i32,
  pub album_id: i32,
  pub disc_number: Option<i32>,
  pub disc_total: Option<i32>,
  pub track_number: Option<i32>,
  pub track_total: Option<i32>,
  pub title: String,
}

#[derive(Debug, Insertable)]
#[table_name = "track"]
pub struct NewTrack {
  pub album_id: i32,
  pub disc_number: Option<i32>,
  pub disc_total: Option<i32>,
  pub track_number: Option<i32>,
  pub track_total: Option<i32>,
  pub title: String,
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

//
// Local data
//

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, Serialize, Deserialize)]
#[primary_key(album_id, source_id)]
#[table_name = "local_album"]
#[belongs_to(Album)]
#[belongs_to(Source)]
pub struct LocalAlbum {
  pub album_id: i32,
  pub source_id: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "local_album"]
pub struct NewLocalAlbum {
  pub album_id: i32,
  pub source_id: i32,
}

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, AsChangeset, Serialize, Deserialize)]
#[primary_key(track_id, source_id)]
#[table_name = "local_track"]
#[belongs_to(Track)]
#[belongs_to(Source)]
#[changeset_options(treat_none_as_null = "true")]
pub struct LocalTrack {
  pub track_id: i32,
  pub source_id: i32,
  pub file_path: Option<String>,
  pub hash: i64,
}

#[derive(Debug, Insertable)]
#[table_name = "local_track"]
pub struct NewLocalTrack {
  pub track_id: i32,
  pub source_id: i32,
  pub file_path: Option<String>,
  pub hash: i64,
}

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, Serialize, Deserialize)]
#[primary_key(artist_id, source_id)]
#[table_name = "local_artist"]
#[belongs_to(Artist)]
#[belongs_to(Source)]
pub struct LocalArtist {
  pub artist_id: i32,
  pub source_id: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "local_artist"]
pub struct NewLocalArtist {
  pub artist_id: i32,
  pub source_id: i32,
}

//
// Spotify data
//

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, Serialize, Deserialize)]
#[primary_key(album_id, spotify_id)]
#[table_name = "spotify_album"]
#[belongs_to(Album)]
pub struct SpotifyAlbum {
  pub album_id: i32,
  pub spotify_id: String,
}

#[derive(Debug, Insertable)]
#[table_name = "spotify_album"]
pub struct NewSpotifyAlbum {
  pub album_id: i32,
  pub spotify_id: String,
}

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, Serialize, Deserialize)]
#[primary_key(track_id, spotify_id)]
#[table_name = "spotify_track"]
#[belongs_to(Track)]
pub struct SpotifyTrack {
  pub track_id: i32,
  pub spotify_id: String,
}

#[derive(Debug, Insertable)]
#[table_name = "spotify_track"]
pub struct NewSpotifyTrack {
  pub track_id: i32,
  pub spotify_id: String,
}

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations, Serialize, Deserialize)]
#[primary_key(artist_id, spotify_id)]
#[table_name = "spotify_artist"]
#[belongs_to(Artist)]
pub struct SpotifyArtist {
  pub artist_id: i32,
  pub spotify_id: String,
}

#[derive(Debug, Insertable)]
#[table_name = "spotify_artist"]
pub struct NewSpotifyArtist {
  pub artist_id: i32,
  pub spotify_id: String,
}

//
// Generic data
//

#[derive(Clone, PartialOrd, PartialEq, Debug, Serialize, Deserialize)]
pub enum AlbumSource {
  Local(LocalAlbum),
  Spotify(SpotifyAlbum),
}

#[derive(Clone, PartialOrd, PartialEq, Debug, Serialize, Deserialize)]
pub enum TrackSource {
  Local(LocalTrack),
  Spotify(SpotifyTrack),
}

#[derive(Clone, PartialOrd, PartialEq, Debug, Serialize, Deserialize)]
pub enum ArtistSource {
  Local(LocalArtist),
  Spotify(SpotifyArtist),
}

//
// User and user data
//

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

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct NewUser {
  pub name: String,
  pub password: String,
}

// User-album rating

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, Associations, AsChangeset, Serialize, Deserialize)]
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
#[belongs_to(User)] /*, foreign_key = "user_id"*/
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
#[belongs_to(User)]
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

//
// Display implementations
//

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
    // if let Some(file_path) = &self.file_path {
    //   write!(f, " - {}", file_path)?;
    // }
    Ok(())
  }
}

// impl Display for Source {
//   fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
//     f.write_str(&self.directory)
//   }
// }
