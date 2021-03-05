use std::path::PathBuf;

use diesel::prelude::*;

use musium_core::model::{LocalSource, LocalTrack};
use musium_core::schema;

use crate::model::LocalSourceEx;

use super::{DatabaseConnection, DatabaseQueryError};

impl DatabaseConnection {
  pub fn get_local_track_path_by_track_id(&self, input_track_id: i32) -> Result<Option<PathBuf>, DatabaseQueryError> {
    let data: Option<(LocalTrack, LocalSource)> = {
      use schema::local_track::dsl::*;
      local_track
        .filter(track_id.eq(input_track_id))
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
