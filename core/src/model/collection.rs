use std::collections::HashMap;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::model::*;

//
// Albums
//

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct AlbumsRaw {
  pub albums: Vec<Album>,
  pub artists: Vec<Artist>,
  pub album_artists: Vec<AlbumArtist>,
}

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

  pub fn len(&self) -> usize {
    self.albums.len()
  }
}

impl From<AlbumsRaw> for Albums {
  fn from(albums: AlbumsRaw) -> Self {
    Albums::from(albums.albums, albums.artists, albums.album_artists)
  }
}

//
// Tracks
//

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct TracksRaw {
  pub albums: Vec<Album>,
  pub tracks: Vec<Track>,
  pub artists: Vec<Artist>,
  pub album_artists: Vec<AlbumArtist>,
  pub track_artists: Vec<TrackArtist>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Tracks {
  pub albums: HashMap<i32, Album>,
  pub tracks: Vec<Track>,
  pub artists: HashMap<i32, Artist>,
  pub album_artists: HashMap<i32, Vec<i32>>,
  pub track_artists: HashMap<i32, Vec<i32>>,
}

impl<'a> Tracks {
  pub fn from(
    albums: Vec<Album>,
    tracks: Vec<Track>,
    artists: Vec<Artist>,
    album_artists: Vec<AlbumArtist>,
    track_artists: Vec<TrackArtist>,
  ) -> Self {
    let albums = albums.into_iter().map(|a| (a.id, a)).collect();
    let artists = artists.into_iter().map(|a| (a.id, a)).collect();
    let track_artists = track_artists.into_iter().map(|ta| (ta.track_id, ta.artist_id)).into_group_map();
    let album_artists = album_artists.into_iter().map(|aa| (aa.album_id, aa.artist_id)).into_group_map();
    Self { tracks, albums, artists, track_artists, album_artists }
  }

  pub fn iter(&'a self) -> impl Iterator<Item=TrackInfo<'a>> + ExactSizeIterator + Clone + 'a {
    let Tracks { tracks, albums, artists, track_artists, album_artists } = &self;
    tracks.into_iter().map(move |track| { TrackInfo { track, albums, artists, track_artists, album_artists } })
  }

  pub fn len(&self) -> usize {
    self.tracks.len()
  }

  pub fn get_track(&self, index: usize) -> Option<&Track> {
    self.tracks.get(index)
  }
}

impl From<TracksRaw> for Tracks {
  fn from(tracks: TracksRaw) -> Self {
    Tracks::from(tracks.albums, tracks.tracks, tracks.artists, tracks.album_artists, tracks.track_artists)
  }
}

pub struct TrackInfo<'a> {
  pub track: &'a Track,
  albums: &'a HashMap<i32, Album>,
  artists: &'a HashMap<i32, Artist>,
  album_artists: &'a HashMap<i32, Vec<i32>>,
  track_artists: &'a HashMap<i32, Vec<i32>>,
}

impl<'a> TrackInfo<'a> {
  #[inline]
  pub fn track(&self) -> &Track {
    self.track
  }

  #[inline]
  pub fn track_artists(&self) -> impl Iterator<Item=&Artist> {
    self.track_artists.get(&self.track.id).into_iter().flat_map(move |ids| ids.into_iter()).filter_map(move |ta| self.artists.get(ta))
  }

  #[inline]
  pub fn album(&self) -> Option<&Album> {
    self.albums.get(&self.track.album_id)
  }

  #[inline]
  pub fn album_artists(&self) -> impl Iterator<Item=&Artist> {
    self.album_artists.get(&self.track.album_id).into_iter().flat_map(move |ids| ids.into_iter()).filter_map(move |ta| self.artists.get(ta))
  }
}
