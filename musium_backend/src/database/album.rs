use diesel::prelude::*;

use musium_core::model::{Album, AlbumArtist, Artist};
use musium_core::model::collection::AlbumsRaw;
use musium_core::schema;

use super::{DatabaseConnection, DatabaseQueryError};

impl DatabaseConnection<'_> {
  pub fn list_albums(&self) -> Result<AlbumsRaw, DatabaseQueryError> {
    let albums = schema::album::table.load::<Album>(&self.connection)?;
    let artists = schema::artist::table.load::<Artist>(&self.connection)?;
    let album_artists = schema::album_artist::table.load::<AlbumArtist>(&self.connection)?;
    Ok(AlbumsRaw { albums, artists, album_artists })
  }

  pub fn get_album_by_id(&self, input_id: i32) -> Result<Option<Album>, DatabaseQueryError> {
    use schema::album::dsl::*;
    Ok(album.find(input_id).first::<Album>(&self.connection).optional()?)
  }
}
