use std::backtrace::Backtrace;
use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use itertools::{Either, Itertools};
use thiserror::Error;
use tracing::{event, instrument, Level};

use musium_core::model::{Album, Artist, LocalAlbum, LocalArtist, LocalSource, LocalTrack, NewLocalAlbum, NewLocalArtist, NewLocalTrack, NewTrack, Track};
use musium_core::schema;
use musium_filesystem_sync::{FilesystemSyncError, FilesystemSyncTrack};

use crate::database::DatabaseConnection;
use crate::database::sync::{SelectAlbumError, SelectArtistError};
use crate::model::{LocalTrackEx, TrackEx, UpdateTrackFrom};

#[derive(Debug, Error)]
pub enum LocalSyncError {
  #[error("Failed to query database")]
  DatabaseQueryFail(#[from] diesel::result::Error, Backtrace),
  #[error("Failed to select an album")]
  SelectAlbumFail(#[from] SelectAlbumError, Backtrace),
  #[error("Failed to select an artist")]
  SelectArtistFail(#[from] SelectArtistError, Backtrace),
  #[error("Attempted to update possibly moved locally synchronized track {0:#?}, but found multiple local tracks in the database with the same source and hash: {1:#?}")]
  HashCollisionFail(FilesystemSyncTrack, Vec<LocalTrack>),
}

impl DatabaseConnection<'_> {
  #[instrument(skip(self, local_sources))]
  pub(crate) fn local_sync(&self, local_sources: Vec<LocalSource>) -> Result<Vec<FilesystemSyncError>, LocalSyncError> {
    let (filesystem_sync_tracks, filesystem_sync_errors) = self.get_filesystem_sync_tracks(local_sources)?;
    let mut synced_file_paths = HashMap::<i32, HashSet<String>>::new();
    // Insert tracks and related entities.
    for (local_source_id, local_sync_track) in filesystem_sync_tracks {
      event!(Level::TRACE, ?local_sync_track, "Processing local sync track");
      synced_file_paths.entry(local_source_id)
        .or_default()
        .insert(local_sync_track.file_path.clone());

      let album = self.sync_local_album(local_source_id, &local_sync_track)?;
      let artist_ids: Result<HashSet<_>, _> = local_sync_track.album_artists.iter()
        .map(|album_artist_name| self.sync_local_artist(local_source_id, album_artist_name.clone()).map(|artist| artist.id))
        .collect();
      let artist_ids = artist_ids?;
      self.sync_album_artists(&album, artist_ids)?;

      let track = self.sync_local_track(local_source_id, &album, &local_sync_track)?;
      let artist_ids: Result<HashSet<_>, _> = local_sync_track.track_artists.iter()
        .map(|track_artist_name| self.sync_local_artist(local_source_id, track_artist_name.clone()).map(|artist| artist.id))
        .collect();
      let artist_ids = artist_ids?;
      self.sync_track_artists(&track, artist_ids)?;
    }
    self.cleanup_local_tracks(synced_file_paths)?;
    Ok(filesystem_sync_errors)
  }

  fn get_filesystem_sync_tracks(&self, local_sources: Vec<LocalSource>) -> Result<(Vec<(i32, FilesystemSyncTrack)>, Vec<FilesystemSyncError>), LocalSyncError> {
    let do_local_sync = {
      || local_sources
        .into_iter()
        .flat_map(|local_source|
          musium_filesystem_sync::sync(local_source.directory.clone()).map(move |track| track.map(|track| (local_source.id, track)))
        )
        .partition_map(|r| {
          match r {
            Ok(v) => Either::Left(v),
            Err(v) => Either::Right(v)
          }
        })
    };
    Ok(time!("sync.local_sync", do_local_sync()))
  }

  fn sync_local_album(&self, local_source_id: i32, filesystem_sync_track: &FilesystemSyncTrack) -> Result<Album, LocalSyncError> {
    let db_album = self.select_one_or_insert_album(&filesystem_sync_track.album)?.into();

    let select_local_album_query = {
      use schema::local_album::dsl::*;
      local_album.find((db_album.id, local_source_id))
    };
    let db_local_album: Option<LocalAlbum> = time!("sync.select_local_album", select_local_album_query.first(&self.connection).optional()?);
    if let Some(_db_local_album) = db_local_album {
      // A local album was found for the album: update it.
      // TODO: update local album columns when they are added.
    } else {
      // No local album was found for the album: insert it.
      let new_local_album = NewLocalAlbum { album_id: db_album.id, local_source_id };
      event!(Level::DEBUG, ?new_local_album, "Inserting local album");
      let insert_local_album_query = {
        use schema::local_album::dsl::*;
        diesel::insert_into(local_album).values(new_local_album)
      };
      time!("sync.insert_local_album", insert_local_album_query.execute(&self.connection)?);
    };
    Ok(db_album)

    // TODO: when there are multiple albums with the same name, but no local albums for any of them: create a local
    //       album for the first one and emit a persistent warning that the user may have to disambiguate manually.
    //       Return the selected album.
    // TODO: when there are multiple albums with the same name, and local albums for some of them, try to match the
    //       local album with an external ID such as the MusicBrainz Album ID. If a match was found, return that
    //       album. If no match was found, and there are local albums for all albums, create a new album and local
    //       album. If no match was found, but there is no local album for some albums, take the first of those albums,
    //       create a local album for it, and emit a persistent warning that the user may have to disambiguate manually.
  }

  fn sync_local_track(&self, local_source_id: i32, album: &Album, local_sync_track: &FilesystemSyncTrack) -> Result<Track, LocalSyncError> {
    use LocalSyncError::*;

    let track_file_path = local_sync_track.file_path.clone();

    let local_track_select_query = {
      use schema::local_track::dsl::*;
      local_track
        .filter(local_source_id.eq(local_source_id))
        .filter(file_path.eq(&track_file_path))
    };
    let db_local_track = time!("sync.select_local_track", local_track_select_query.first::<LocalTrack>(&self.connection).optional()?);
    let db_track = if let Some(db_local_track) = db_local_track {
      // A local track with the same path as the locally synchronized track was found. Either track meta-data has been
      // updated, or the track has been replaced by a new one.
      let mut db_local_track: LocalTrack = db_local_track;

      // Get track corresponding to the local track. There is always one due to referential integrity.
      let track_select_query = {
        use schema::track::dsl::*;
        track.find(db_local_track.track_id)
      };
      let mut db_track: Track = time!("sync.select_track", track_select_query.first::<Track>(&self.connection)?);

      // We check if the track was replaced by checking if the metadata and/or hash is different.
      // TODO: measure how much the metadata has changed, and still update when the metadata has not changed drastically.
      // TODO: use AcousticID as a hash, to measure changes in the hash as well.
      let hash_changed = db_local_track.check_hash_changed(&local_sync_track);
      let metadata_changed = db_track.check_metadata_changed(&album, &local_sync_track);
      if hash_changed && metadata_changed {
        // When both the hash and metadata have changed, we assume the file has been replaced by a new one, and
        // instead set the track in the database as removed (NULL file_path), and insert the scanned track as a
        // new one.
        db_local_track.file_path = None;
        event!(Level::DEBUG, ?db_local_track, "Local track has been replaced, setting the local track as removed in the database");
        time!("sync.update_replaced_local_track", db_local_track.save_changes::<LocalTrack>(&*self.connection)?);
        // Insert replaced track as a new one.
        // TODO: also do the move check here?
        self.insert_new_track_and_local_track(local_source_id, &album, &local_sync_track)?
      } else if hash_changed {
        // When the hash is different, but the metadata is not, we assume that the track's audio data has (somehow)
        // changed, and just update the hash.
        event!(Level::TRACE, ?db_local_track, "Updating hash of local track");
        db_local_track.hash = local_sync_track.hash as i64;
        time!("sync.update_local_track_hash", db_local_track.save_changes::<LocalTrack>(&*self.connection)?);
        db_track
      } else if metadata_changed {
        // When the hash is the same, but the metadata is not, the metadata of the track was changed, and we just update it.
        event!(Level::TRACE, ?db_track, "Updating track with values from locally synchronized track");
        if db_track.update_from(&album, local_sync_track) {
          event!(Level::DEBUG, ?db_track, "Track has changed, updating the track in the database");
          time!("sync.update_track", db_track.save_changes(&*self.connection)?)
        } else {
          db_track
        }
      } else {
        // Neither hash nor metadata was changed: no update is performed.
        db_track
      }
    } else {
      // Did not find a track with the same path as the locally synchronized track. Either the track is new, or it was moved.
      // We check if the track was moved by searching for the track by hash instead.
      let select_by_hash_query = {
        use schema::local_track::dsl::*;
        local_track
          .filter(local_source_id.eq(local_source_id))
          .filter(hash.eq(local_sync_track.hash as i64))
      };
      let tracks_by_hash: Vec<LocalTrack> = time!("sync.select_local_tracks_by_hash", select_by_hash_query.load::<LocalTrack>(&self.connection)?);
      match tracks_by_hash.len() {
        0 => {
          // No track with the same hash was found: we insert it as a new track.
          self.insert_new_track_and_local_track(local_source_id, &album, &local_sync_track)?
        }
        1 => {
          // A track with the same hash was found: we update the local track in the database with the locally synchronized track.
          let mut db_local_track: LocalTrack = tracks_by_hash.into_iter().take(1).next().unwrap();
          event!(Level::TRACE, ?db_local_track, "Updating moved local track with values from locally synchronized track");
          if db_local_track.update_from(&local_sync_track) {
            event!(Level::DEBUG, ?db_local_track, "Updating moved local track");
            time!("sync.update_moved_local_track", db_local_track.save_changes::<LocalTrack>(&*self.connection)?);
          }

          // Get track corresponding to the local track. There is always one due to referential integrity.
          let track_select_query = {
            use schema::track::dsl::*;
            track.find(db_local_track.track_id)
          };
          let mut db_track: Track = time!("sync.select_track", track_select_query.first::<Track>(&self.connection)?);

          // Update the corresponding track as well.
          event!(Level::TRACE, ?db_track, "Updating track with values from locally synchronized track");
          if db_track.update_from(&album, local_sync_track) {
            event!(Level::DEBUG, ?db_track, "Track has changed, updating the track in the database");
            time!("sync.update_track", db_track.save_changes(&*self.connection)?)
          } else {
            db_track
          }
        }
        _ => {
          // Multiple tracks with the same hash were found: for now, we error out.
          return Err(HashCollisionFail(local_sync_track.clone(), tracks_by_hash));
        }
      }
    };
    Ok(db_track)
  }

  fn insert_new_track_and_local_track(&self, local_source_id: i32, album: &Album, local_sync_track: &FilesystemSyncTrack) -> Result<Track, LocalSyncError> {
    let db_track = self.insert_track(NewTrack {
      album_id: album.id,
      disc_number: local_sync_track.disc_number,
      disc_total: local_sync_track.disc_total,
      track_number: local_sync_track.track_number,
      track_total: local_sync_track.track_total,
      title: local_sync_track.title.clone(),
    })?;
    let new_local_track = NewLocalTrack {
      track_id: db_track.id,
      local_source_id,
      file_path: Some(local_sync_track.file_path.clone()),
      hash: local_sync_track.hash as i64,
    };
    event!(Level::DEBUG, ?new_local_track, "Inserting local track");
    let local_track_insert_query = diesel::insert_into(schema::local_track::table).values(new_local_track);
    time!("sync.insert_local_track", local_track_insert_query.execute(&self.connection)?);
    Ok(db_track)
  }

  fn sync_local_artist(&self, local_source_id: i32, artist_name: String) -> Result<Artist, LocalSyncError> {
    let db_artist = match self.select_one_artist_by_name(&artist_name)? {
      None => {
        // No artist with the same name was found: insert it.
        let db_artist = self.insert_artist(&artist_name)?;
        // Insert local artist corresponding to artist.
        let new_local_artist = NewLocalArtist { artist_id: db_artist.id, local_source_id };
        event!(Level::DEBUG, ?new_local_artist, "Inserting local artist");
        let insert_local_artist_query = {
          use schema::local_artist::dsl::*;
          diesel::insert_into(local_artist).values(new_local_artist)
        };
        time!("sync.insert_local_artist", insert_local_artist_query.execute(&self.connection)?);
        db_artist
      }
      Some(db_artist) => {
        // One artist with the same name was found.
        let select_local_artist_query = {
          use schema::local_artist::dsl::*;
          local_artist.find((db_artist.id, local_source_id))
        };
        let db_local_artist = time!("sync.select_local_artist", select_local_artist_query.first(&self.connection).optional()?);
        if let Some(db_local_artist) = db_local_artist {
          let _db_local_artist: LocalArtist = db_local_artist;
          // A local artist was found for the artist: update it.
          // TODO: update local artist columns when they are added.
        } else {
          // No local artist was found for the artist: insert it.
          let new_local_artist = NewLocalArtist { artist_id: db_artist.id, local_source_id };
          event!(Level::DEBUG, ?new_local_artist, "Inserting local artist");
          let insert_local_artist_query = {
            use schema::local_artist::dsl::*;
            diesel::insert_into(local_artist).values(new_local_artist)
          };
          time!("sync.insert_local_artist", insert_local_artist_query.execute(&self.connection)?);
        }
        db_artist
      }
    };
    Ok(db_artist)

    // TODO: when there are multiple artists with the same name, but no local artists for any of them: create a local
    //       artist for the first one and emit a persistent warning that the user may have to disambiguate manually.
    //       Return the selected artist.
    // TODO: when there are multiple artists with the same name, and local artists for some of them, try to match the
    //       local artist with an external ID such as the MusicBrainz Artist ID. If a match was found, return that
    //       artist. If no match was found, and there are local artists for all artists, create a new artist and local
    //       artist. If no match was found, but there is no local artist for some artists, take the first of those artists,
    //       create a local artist for it, and emit a persistent warning that the user may have to disambiguate manually.
  }

  fn cleanup_local_tracks(&self, synced_file_paths: HashMap::<i32, HashSet<String>>) -> Result<(), LocalSyncError> {
    let db_local_track_data: Vec<(i32, i32, Option<String>)> = {
      use schema::local_track::dsl::*;
      local_track
        .select((track_id, local_source_id, file_path))
        .filter(file_path.is_not_null())
        .load::<(i32, i32, Option<String>)>(&self.connection)?
    };
    for (db_track_id, db_local_source_id, db_file_path) in db_local_track_data {
      if let (Some(db_file_path), Some(synced_file_paths)) = (db_file_path, synced_file_paths.get(&db_local_source_id)) {
        if !synced_file_paths.contains(&db_file_path) {
          event!(Level::DEBUG, ?db_track_id, ?db_file_path, "Local track '{}' at '{}' was not seen during synchronization: setting it as removed in the database", db_track_id, db_file_path);
          let update_query = {
            use schema::local_track::dsl::*;
            diesel::update(local_track)
              .filter(track_id.eq(db_track_id))
              .filter(local_source_id.eq(db_local_source_id))
              .set(file_path.eq::<Option<String>>(None))
          };
          time!("sync.update_removed_local_track", update_query.execute(&self.connection)?);
        }
      }
    }
    Ok(())
  }
}
