use std::backtrace::Backtrace;

use diesel::prelude::*;
use thiserror::Error;
use tracing::{event, Level};

use musium_core::model::{Album, NewSpotifyAlbum, SpotifyAlbum, SpotifySource};
use musium_core::schema;

use crate::database::{DatabaseConnection, DatabaseQueryError};
use crate::database::sync::GetOrCreateAlbumError;
use crate::model::SpotifySourceEx;

#[derive(Debug, Error)]
pub enum SpotifySyncError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error(transparent)]
  SpotifyApiFail(#[from] musium_spotify_sync::ApiError),
  #[error(transparent)]
  GetOrCreateAlbumFail(#[from] GetOrCreateAlbumError),
}

impl From<DatabaseQueryError> for SpotifySyncError {
  fn from(e: DatabaseQueryError) -> Self {
    match e {
      DatabaseQueryError::DatabaseQueryFail(e, bt) => Self::DatabaseQueryFail(e, bt)
    }
  }
}

impl DatabaseConnection<'_> {
  pub(crate) async fn spotify_sync(&self, spotify_sources: Vec<SpotifySource>) -> Result<(), SpotifySyncError> {
    for mut spotify_source in spotify_sources {
      let mut authorization = spotify_source.to_spotify_authorization();
      let albums = self.database.spotify_sync.get_albums_of_followed_artists(&mut authorization).await?;
      for album in albums {
        self.sync_spotify_album(&album)?;
      }
      if spotify_source.update_from_spotify_authorization(authorization) {
        spotify_source.save_changes::<SpotifySource>(&*self.connection)?;
      }
    }
    Ok(())
  }

  fn sync_spotify_album(&self, spotify_album: &musium_spotify_sync::Album) -> Result<Album, SpotifySyncError> {
    let spotify_album_id = &spotify_album.id;
    let select_query = {
      use schema::spotify_album::dsl::*;
      spotify_album.filter(spotify_id.eq(spotify_album_id))
    };
    let db_spotify_album: Option<SpotifyAlbum> = time!("sync_spotify_album.select_spotify_album", select_query.first(&self.connection).optional()?);
    if let Some(db_spotify_album) = db_spotify_album {
      // A Spotify album was found for the Spotify album ID: update it.
      // TODO: update Spotify album columns when they are added.
      // TODO: select album with a join on the previous query?
      Ok(self.get_album_by_id(db_spotify_album.album_id)?.unwrap())
    } else {
      // No Spotify album was found for the Spotify album ID: get or create an album, then create a Spotify album.
      let db_album = self.get_or_create_album_by_name(&spotify_album.name)?;
      let new_spotify_album = NewSpotifyAlbum { album_id: db_album.id, spotify_id: spotify_album_id.clone() };
      event!(Level::DEBUG, ?new_spotify_album, "Inserting Spotify album");
      let insert_spotify_album_query = {
        use schema::spotify_album::dsl::*;
        diesel::insert_into(spotify_album).values(new_spotify_album)
      };
      time!("sync_spotify_album.insert_spotify_album", insert_spotify_album_query.execute(&self.connection)?);
      Ok(db_album)
    }
  }
}
