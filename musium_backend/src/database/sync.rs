use std::backtrace::Backtrace;
use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use itertools::Either;
use thiserror::Error;
use tracing::{event, instrument, Level};

use musium_core::model::{Album, AlbumArtist, Artist, LocalAlbum, NewAlbum, NewAlbumArtist, NewArtist, NewTrack, NewTrackArtist, Source, SourceData, Track, TrackArtist, NewLocalAlbum};
use musium_core::schema;

use crate::sync::local::{LocalSyncError, LocalSyncTrack};

use super::{DatabaseConnection, DatabaseQueryError};

#[derive(Debug, Error)]
pub enum SyncError {
  #[error("Failed to list sources")]
  ListScanDirectoriesFail(#[from] DatabaseQueryError, Backtrace),
  #[error("Failed to query database")]
  DatabaseFail(#[from] diesel::result::Error, Backtrace),
  #[error("Attempted to update possibly moved local track {0:#?}, but found multiple tracks in the database with the same source and hash: {1:#?}")]
  HashCollisionFail(LocalSyncTrack, Vec<Track>),
  #[error("One or more errors occurred during local synchronization, but the database has already received a partial update")]
  LocalSyncFail(Vec<LocalSyncError>),
}

impl DatabaseConnection<'_> {
  #[instrument]
  /// Synchronize with all sources, adding/removing/changing tracks/albums/artists in the database. When a LocalSyncFail
  /// error is returned, the database has already received a partial update.
  pub fn sync(&self) -> Result<(), SyncError> {
    use SyncError::*;

    let (local_sync_tracks, local_sync_errors) = self.local_sync()?;
    let mut synced_file_paths = HashMap::<i32, HashSet<String>>::new();

    self.connection.transaction::<_, SyncError, _>(|| {
      // Insert tracks and related entities.
      for local_sync_track in local_sync_tracks {
        event!(Level::TRACE, ?local_sync_track, "Processing local sync track");
        synced_file_paths.entry(local_sync_track.source_id)
          .or_default()
          .insert(local_sync_track.file_path.clone());
        let album = self.sync_local_album(&local_sync_track)?;
        let track = self.sync_local_track(&album, &local_sync_track)?;
        self.sync_local_track_artists(&track, &local_sync_track)?;
        self.sync_local_album_artists(&album, &local_sync_track)?;
      }
      self.sync_local_removed_tracks(synced_file_paths)?;
      Ok(())
    })?;
    if !local_sync_errors.is_empty() {
      return Err(LocalSyncFail(local_sync_errors));
    }
    Ok(())
  }

  fn local_sync(&self) -> Result<(Vec<LocalSyncTrack>, Vec<LocalSyncError>), SyncError> {
    let sources: Vec<Source> = time!("sync.list_sources", self.list_sources()?);
    Ok(time!("sync.local_sync", {
      sources
        .into_iter()
        .flat_map(|source| {
          match source.data {
            SourceData::Local(local_source_data) => self.backend.local_sync.sync(source.id, local_source_data),
            _ => vec![],
          }
        })
        .partition_map(|r| {
          match r {
            Ok(v) => Either::Left(v),
            Err(v) => Either::Right(v)
          }
        })
    }))
  }

  fn sync_local_album(&self, local_sync_track: &LocalSyncTrack) -> Result<Album, SyncError> {
    let album_name = local_sync_track.album.clone();
    let select_query = {
      use schema::album::dsl::*;
      album.filter(name.eq(&album_name));
    };
    let db_albums: Vec<Album> = time!("sync.select_album", select_query.load::<Album>(&self.connection)?);
    let db_albums_len = db_albums.len();
    Ok(if db_albums_len == 0 {
      // No album with the same name was found: insert it.
      let new_album = NewAlbum { name: album_name.clone() };
      event!(Level::DEBUG, ?new_album, "Inserting album");
      let insert_album_query = {
        use schema::album::dsl::*;
        diesel::insert_into(album)
          .values(new_album);
      };
      time!("sync.insert_album", insert_album_query.execute(&self.connection)?);
      let album = time!("sync.select_inserted_album", select_query.first::<Album>(&self.connection)?);
      // Insert local album corresponding to album.
      let new_local_album = NewLocalAlbum { album_id: album.id, source_id: local_sync_track.source_id };
      event!(Level::DEBUG, ?new_local_album, "Inserting local album");
      let insert_local_album_query = {
        use schema::local_album::dsl::*;
        diesel::insert_into(local_album)
          .values(new_local_album);
      };
      time!("sync.insert_local_album", insert_local_album_query.execute(&self.connection)?);
      album
    } else if db_albums_len == 1 {
      // One album with the same name was found.
      let db_album = db_albums.into_iter().next().unwrap();
      let select_local_album_query = {
        use schema::local_album::dsl::*;
        local_album.find((db_album.id, local_sync_track.source_id))
      };
      let db_local_album = time!("sync.select_local_album", select_local_album_query.first(&self.connection).optional()?);
      if let Some(db_local_album) = db_local_album {
        let db_local_album: LocalAlbum = db_local_album;
        // A local album was found for the album: update it.
        // TODO: update local album columns when they are added.
      } else {
        // No local album was found for the album: insert it.
        let new_local_album = NewLocalAlbum { album_id: db_album.id, source_id: local_sync_track.source_id };
        event!(Level::DEBUG, ?new_local_album, "Inserting local album");
        let insert_local_album_query = {
          use schema::local_album::dsl::*;
          diesel::insert_into(local_album)
            .values(new_local_album);
        };
        time!("sync.insert_local_album", insert_local_album_query.execute(&self.connection)?);
      }
      db_album
    } else {
      // Multiple albums with the same name were found.
      // TODO: handle multiple albums with the same name. This cannot happen currently, but should be supported later.
      event!(Level::ERROR, ?db_albums, "Multiple albums with the same name were found, which is currently not supported");
    })
  }

  fn sync_local_track(&self, album: &Album, local_sync_track: &LocalSyncTrack) -> Result<Track, SyncError> {
    use SyncError::*;
    use schema::track::dsl::*;

    let track_file_path = local_sync_track.file_path.clone();
    let select_query = track
      .filter(scan_directory_id.eq(local_sync_track.scan_directory_id))
      .filter(file_path.eq(&track_file_path));
    let db_track = time!("sync.select_track", select_query.first::<Track>(&self.connection).optional()?);
    Ok(if let Some(db_track) = db_track {
      // A track with the same path as the scanned track was found. Either track meta-data has been updated, or
      // the track has been replaced by a new one.
      let mut db_track: Track = db_track;

      // We check if the track was replaced by checking if the metadata and/or hash is different.
      // TODO: measure how much the metadata has changed, and still update when the metadata has not changed drastically.
      // TODO: use AcousticID as a hash, to measure changes in the hash as well.
      let hash_changed = db_track.check_hash_changed(&local_sync_track);
      let metadata_changed = db_track.check_metadata_changed(&album, &local_sync_track);

      if hash_changed && metadata_changed {
        // When both the hash and metadata have changed, we assume the file has been replaced by a new one, and
        // instead set the track in the database as removed (NULL file_path), and insert the scanned track as a
        // new one.
        db_track.file_path = None;
        event!(Level::DEBUG, ?db_track, "Track has been replaced, setting the track as removed in the database");
        time!("sync.update_replaced_track", db_track.save_changes::<Track>(&*self.connection)?);
        // Insert replaced track as a new one.
        // TODO: remove duplicate code from other track insertion.
        // TODO: also do the move check here?
        let new_track = NewTrack {
          source_id: local_sync_track.scan_directory_id,
          album_id: album.id,
          disc_number: local_sync_track.disc_number,
          disc_total: local_sync_track.disc_total,
          track_number: local_sync_track.track_number,
          track_total: local_sync_track.track_total,
          title: local_sync_track.title,
          file_path: Some(track_file_path.clone()),
          hash: local_sync_track.hash as i64,
        };
        event!(Level::DEBUG, ?new_track, "Inserting replaced track");
        let insert_query = diesel::insert_into(track)
          .values(new_track);
        time!("sync.insert_replaced_track", insert_query.execute(&self.connection)?);
        time!("sync.select_inserted_replaced_track", select_query.first::<Track>(&self.connection)?)
      } else {
        // When the hash is different, but the metadata is not, we assume that the track's audio data has
        // (somehow) changed, and just update the hash. When the hash is the same, but the metadata is not, the
        // metadata of the track was changed, and just update it. If neither was changed, no update will be
        // performed.
        event!(Level::TRACE, ?db_track, "Updating track with values from scanned track");
        let changed = db_track.update_from(&album, &local_sync_track);
        if changed {
          event!(Level::DEBUG, ?db_track, "Track has changed, updating the track in the database");
          time!("sync.update_track", db_track.save_changes(&*self.connection)?)
        } else {
          db_track
        }
      }
    } else {
      // Did not find a track with the same path as the scanned track. Either the track is new, or it was moved.
      // We check if the track was moved by searching for the track by hash instead.
      let select_by_hash_query = track
        .filter(scan_directory_id.eq(local_sync_track.scan_directory_id))
        .filter(hash.eq(local_sync_track.hash as i64));
      let tracks_by_hash: Vec<Track> = time!("sync.select_track_by_hash", select_by_hash_query.load::<Track>(&self.connection)?);
      if tracks_by_hash.is_empty() {
        // No track with the same hash was found: we insert it as a new track.
        let new_track = NewTrack {
          source_id: local_sync_track.scan_directory_id,
          album_id: album.id,
          disc_number: local_sync_track.disc_number,
          disc_total: local_sync_track.disc_total,
          track_number: local_sync_track.track_number,
          track_total: local_sync_track.track_total,
          title: local_sync_track.title,
          file_path: Some(track_file_path.clone()),
          hash: local_sync_track.hash as i64,
        };
        event!(Level::DEBUG, ?new_track, "Inserting track");
        let insert_query = diesel::insert_into(track)
          .values(new_track);
        time!("sync.insert_track", insert_query.execute(&self.connection)?);
        time!("sync.select_inserted_track", select_query.first::<Track>(&self.connection)?)
      } else if tracks_by_hash.len() == 1 {
        // A track with the same hash was found: we update the track in the database with the scanned track.
        let mut db_track: Track = tracks_by_hash.into_iter().take(1).next().unwrap();
        event!(Level::TRACE, ?db_track, "Updating moved track with values from scanned track");
        let changed = db_track.update_from(&album, &local_sync_track);
        if changed {
          event!(Level::DEBUG, ?db_track, "Updating moved track");
          time!("sync.update_moved_track", db_track.save_changes(&*self.connection)?)
        } else {
          db_track
        }
      } else {
        // Multiple tracks with the same hash were found: for now, we error out.
        return Err(HashCollisionFail(local_sync_track.clone(), tracks_by_hash));
      }
    })
  }

  fn update_or_insert_artists<I: IntoIterator<Item=String>>(&self, artist_names: I) -> Result<HashSet<Artist>, diesel::result::Error> {
    artist_names.into_iter().map(|artist_name| {
      use schema::artist::dsl::*;
      let select_query = artist
        .filter(name.eq(&artist_name));
      let db_artist = time!("sync.select_artist", select_query.first::<Artist>(&self.connection).optional()?);
      let db_artist = if let Some(db_artist) = db_artist {
        let db_artist: Artist = db_artist;
        // TODO: update artist columns when they are added.
        //time!("sync.update_artist", db_artist.save_changes(&self.connection)?)
        db_artist
      } else {
        let new_artist = NewArtist { name: artist_name.clone() };
        let insert_query = diesel::insert_into(artist)
          .values(new_artist);
        time!("sync.insert_artist", insert_query.execute(&self.connection)?);
        time!("sync.select_inserted_artist", select_query.first::<Artist>(&self.connection)?)
      };
      Ok(db_artist)
    }).collect()
  }

  fn sync_local_track_artists(&self, track: &Track, local_sync_track: &LocalSyncTrack) -> Result<(), SyncError> {
    let mut db_artists: HashSet<Artist> = self.update_or_insert_artists(local_sync_track.track_artists.into_iter())?;
    use schema::track_artist::dsl::*;
    let db_track_artists: Vec<(TrackArtist, Artist)> = time!("sync.select_track_artists", track_artist
      .filter(track_id.eq(track.id))
      .inner_join(schema::artist::table)
      .load(&self.connection)?);
    for (db_track_artist, db_artist) in db_track_artists {
      if db_artists.contains(&db_artist) {
        // TODO: update track_artist columns if they are added.
        //let mut db_track_artist = db_track_artist;
        //time!("sync.update_track_artist", db_track_artist.save_changes(&self.connection)?)
      } else {
        event!(Level::DEBUG, ?db_track_artist, "Deleting track artist");
        time!("sync.delete_track_artist", diesel::delete(&db_track_artist).execute(&self.connection)?);
      }
      db_artists.remove(&db_artist); // Remove from set, so we know what to insert afterwards.
    }
    for artist in db_artists {
      let new_track_artist = NewTrackArtist { track_id: track.id, artist_id: artist.id };
      event!(Level::DEBUG, ?new_track_artist, "Inserting track artist");
      time!("sync.insert_track_artist", diesel::insert_into(track_artist)
        .values(new_track_artist)
        .execute(&self.connection)?);
    }
    Ok(())
  }

  fn sync_local_album_artists(&self, album: &Album, local_sync_track: &LocalSyncTrack) -> Result<(), SyncError> {
    let mut db_artists: HashSet<Artist> = self.update_or_insert_artists(local_sync_track.album_artists.into_iter())?;
    use schema::album_artist::dsl::*;
    let db_album_artists: Vec<(AlbumArtist, Artist)> = time!("sync.select_album_artists", album_artist
      .filter(album_id.eq(album.id))
      .inner_join(schema::artist::table)
      .load(&self.connection)?);
    for (db_album_artist, db_artist) in db_album_artists {
      if db_artists.contains(&db_artist) {
        // TODO: update album_artist columns if they are added.
        //let mut db_album_artist = db_album_artist;
        //time!("sync.update_album_artist", db_album_artist.save_changes(&self.connection)?)
      } else {
        event!(Level::DEBUG, ?db_album_artist, "Deleting album artist");
        time!("sync.delete_album_artist", diesel::delete(&db_album_artist).execute(&self.connection)?);
      }
      db_artists.remove(&db_artist); // Remove from set, so we know what to insert afterwards.
    }
    for artist in db_artists {
      let new_album_artist = NewAlbumArtist { album_id: album.id, artist_id: artist.id };
      event!(Level::DEBUG, ?new_album_artist, "Inserting album artist");
      time!("sync.insert_album_artist", diesel::insert_into(album_artist)
        .values(new_album_artist)
        .execute(&self.connection)?);
    }
    Ok(())
  }

  fn sync_local_removed_tracks(&self, non_synced_tracks_per_source: HashMap::<i32, HashSet<String>>) -> Result<(), SyncError> {
    let db_track_data: Vec<(i32, i32, Option<String>)> = {
      use schema::track::dsl::*;
      track
        .select((id, scan_directory_id, file_path))
        .filter(file_path.is_not_null())
        .load::<(i32, i32, Option<String>)>(&self.connection)?
    };
    for (db_track_id, db_scan_directory_id, db_file_path) in db_track_data {
      if let (Some(db_file_path), Some(scanned_file_paths)) = (db_file_path, non_synced_tracks_per_source.get(&db_scan_directory_id)) {
        if !scanned_file_paths.contains(&db_file_path) {
          event!(Level::DEBUG, ?db_track_id, ?db_file_path, "Track '{}' at '{}' has not been scanned: setting it as removed in the database", db_track_id, db_file_path);
          {
            use schema::track::dsl::*;
            time!("sync.update_removed_track", diesel::update(track)
                  .filter(id.eq(db_track_id))
                  .set(file_path.eq::<Option<String>>(None))
                  .execute(&self.connection)?);
          }
        }
      }
    }
    Ok(())
  }
}

