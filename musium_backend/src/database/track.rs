use diesel::prelude::*;

use musium_core::model::{Album, AlbumArtist, Artist, Track, TrackArtist};
use musium_core::model::collection::TracksRaw;
use musium_core::schema;

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

  // pub fn get_track_path_by_id(&self, input_id: i32) -> Result<Option<PathBuf>, DatabaseQueryError> {
  //   let track_and_scan_directory: Option<(Track, Source)> = {
  //     use schema::track::dsl::*;
  //     track
  //       .find(input_id)
  //       .inner_join(schema::scan_directory::table)
  //       .first::<(Track, Source)>(&self.connection)
  //       .optional()?
  //   };
  //   Ok(track_and_scan_directory.and_then(|(track, scan_directory)| scan_directory.track_file_path(&track)))
  // }
}
