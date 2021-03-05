use diesel::prelude::*;

use musium_core::model::Artist;
use musium_core::schema;

use super::{DatabaseConnection, DatabaseQueryError};

impl DatabaseConnection {
  pub fn list_artists(&self) -> Result<Vec<Artist>, DatabaseQueryError> {
    use schema::artist::dsl::*;
    Ok(artist.load::<Artist>(&self.connection)?)
  }

  pub fn get_artist_by_id(&self, input_id: i32) -> Result<Option<Artist>, DatabaseQueryError> {
    use schema::artist::dsl::*;
    Ok(artist.find(input_id).first::<Artist>(&self.connection).optional()?)
  }
}
