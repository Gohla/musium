use diesel::prelude::*;

use musium_core::model::{Album, AlbumArtist, Artist, Track, TrackArtist, LocalTrack, Source, SourceData};
use musium_core::model::collection::TracksRaw;
use musium_core::schema;

use super::{DatabaseConnection, DatabaseQueryError};
use std::path::PathBuf;
use crate::model::LocalSourceDataEx;

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
    let data: Option<(LocalTrack, Source)> = {
      use schema::local_track::dsl::*;
      local_track
        .filter(track_id.eq(input_id))
        .inner_join(schema::source::table)
        .first::<(LocalTrack, Source)>(&self.connection)
        .optional()?
    };
    if let Some((local_track, source)) = data {
      if let SourceData::Local(local_source_data) = source.data {
        return Ok(local_source_data.track_file_path(&local_track));
      }
    }
    Ok(None)
  }
}
