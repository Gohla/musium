use std::path::PathBuf;

use diesel::prelude::*;

use musium_core::model::{Album, AlbumArtist, Artist, LocalSource, LocalTrack, Track, TrackArtist};
use musium_core::model::collection::TracksRaw;
use musium_core::schema;

use crate::model::LocalSourceEx;

use super::{DatabaseConnection, DatabaseQueryError};

impl DatabaseConnection<'_> {
  pub fn list_tracks(&self) -> Result<TracksRaw, DatabaseQueryError> {
    let tracks = schema::track::table.load::<Track>(&self.connection)?;
    let albums = schema::album::table.load::<Album>(&self.connection)?;
    let artists = schema::artist::table.load::<Artist>(&self.connection)?;
    let track_artists = schema::track_artist::table.load::<TrackArtist>(&self.connection)?;
    let album_artists = schema::album_artist::table.load::<AlbumArtist>(&self.connection)?;
    Ok(TracksRaw { albums, tracks, artists, album_artists, track_artists })
  }

  pub fn get_track_by_id(&self, input_id: i32) -> Result<Option<Track>, DatabaseQueryError> {
    use schema::track::dsl::*;
    Ok(track.find(input_id).first::<Track>(&self.connection).optional()?)
  }

  pub fn get_track_path_by_id(&self, input_id: i32) -> Result<Option<PathBuf>, DatabaseQueryError> {
    let data: Option<(LocalTrack, LocalSource)> = {
      use schema::local_track::dsl::*;
      local_track
        .filter(track_id.eq(input_id))
        .inner_join(schema::local_source::table)
        .first::<(LocalTrack, LocalSource)>(&self.connection)
        .optional()?
    };
    if let Some((local_track, local_source)) = data {
      return Ok(local_source.track_file_path(&local_track));
    }
    Ok(None)
  }
}
