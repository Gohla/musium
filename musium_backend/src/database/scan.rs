use std::backtrace::Backtrace;
use std::collections::{HashMap, HashSet};

use diesel::prelude::*;
use tracing::{event, instrument, Level};
use thiserror::Error;

use musium_core::model::{Album, AlbumArtist, Artist, NewAlbum, NewAlbumArtist, NewArtist, NewTrack, NewTrackArtist, Source, Track, TrackArtist};
use musium_core::schema;
use itertools::Either;

use crate::scanner::ScannedTrack;

use super::{DatabaseConnection, DatabaseQueryError};

#[derive(Debug, Error)]
pub enum ScanError {
  #[error("Failed to list scan directories")]
  ListScanDirectoriesFail(#[from] DatabaseQueryError, Backtrace),
  #[error("Failed to query database")]
  DatabaseFail(#[from] diesel::result::Error, Backtrace),
  #[error("Attempted to update possibly moved track {0:#?}, but found multiple tracks in the database with the same scan directory and hash: {1:#?}")]
  HashCollisionFail(ScannedTrack, Vec<Track>),
  #[error("One or more errors occurred during scanning, but successfully scanned tracks have been added")]
  ScanFail(Vec<crate::scanner::ScanError>),
}

impl DatabaseConnection<'_> {
  #[instrument]
  /// Scans all scan directories for music files, drops all tracks, albums, and artists from the database, and adds all
  /// found tracks, albums, and artists to the database. When a ScanFail error is returned, tracks that were sucessfully
  /// scanned will still have been added to the database.
  pub fn scan(&self) -> Result<(), ScanError> {
    use ScanError::*;

    let scan_directories: Vec<Source> = time!("scan.list_scan_directories", self.list_sources()?);
    let (scanned_tracks, scan_errors): (Vec<ScannedTrack>, Vec<crate::scanner::ScanError>) = time!("scan.file_scan", {
      scan_directories
        .into_iter()
        .flat_map(|scan_directory| self.backend.scanner.scan(scan_directory))
        .partition_map(|r| {
          match r {
            Ok(v) => Either::Left(v),
            Err(v) => Either::Right(v)
          }
        })
    });
    let mut scanned_file_paths = HashMap::<i32, HashSet<String>>::new();

    self.connection.transaction::<_, ScanError, _>(|| {
      // Insert tracks and related entities.
      for scanned_track in scanned_tracks {
        let scanned_track: ScannedTrack = scanned_track;
        scanned_file_paths.entry(scanned_track.scan_directory_id)
          .or_default()
          .insert(scanned_track.file_path.clone());
        event!(Level::TRACE, ?scanned_track, "Processing scanned track");
        // Get and update album, or insert it.
        let album: Album = {
          use schema::album::dsl::*;
          let album_name = scanned_track.album.clone();
          let select_query = album
            .filter(name.eq(&album_name));
          let db_album = time!("scan.select_album", select_query.first::<Album>(&self.connection).optional()?);
          if let Some(db_album) = db_album {
            let db_album: Album = db_album;
            // TODO: update album columns when they are added.
            //time!("scan.update_album", db_album.save_changes(&self.connection)?)
            db_album
          } else {
            let new_album = NewAlbum { name: album_name.clone() };
            event!(Level::DEBUG, ?new_album, "Inserting album");
            let insert_query = diesel::insert_into(album)
              .values(new_album);
            time!("scan.insert_album", insert_query.execute(&self.connection)?);
            time!("scan.select_inserted_album", select_query.first::<Album>(&self.connection)?)
          }
        };
        // Get and update track, or insert it.
        let track: Track = {
          use schema::track::dsl::*;
          let track_file_path = scanned_track.file_path.clone();
          let select_query = track
            .filter(scan_directory_id.eq(scanned_track.scan_directory_id))
            .filter(file_path.eq(&track_file_path));
          let db_track = time!("scan.select_track", select_query.first::<Track>(&self.connection).optional()?);
          if let Some(db_track) = db_track {
            // A track with the same path as the scanned track was found. Either track meta-data has been updated, or
            // the track has been replaced by a new one.
            let mut db_track: Track = db_track;

            // We check if the track was replaced by checking if the metadata and/or hash is different.
            // TODO: measure how much the metadata has changed, and still update when the metadata has not changed drastically.
            // TODO: use AcousticID as a hash, to measure changes in the hash as well.
            let hash_changed = db_track.check_hash_changed(&scanned_track);
            let metadata_changed = db_track.check_metadata_changed(&album, &scanned_track);

            if hash_changed && metadata_changed {
              // When both the hash and metadata have changed, we assume the file has been replaced by a new one, and
              // instead set the track in the database as removed (NULL file_path), and insert the scanned track as a
              // new one.
              db_track.file_path = None;
              event!(Level::DEBUG, ?db_track, "Track has been replaced, setting the track as removed in the database");
              time!("scan.update_replaced_track", db_track.save_changes::<Track>(&*self.connection)?);
              // Insert replaced track as a new one.
              // TODO: remove duplicate code from other track insertion.
              // TODO: also do the move check here?
              let new_track = NewTrack {
                source_id: scanned_track.scan_directory_id,
                album_id: album.id,
                disc_number: scanned_track.disc_number,
                disc_total: scanned_track.disc_total,
                track_number: scanned_track.track_number,
                track_total: scanned_track.track_total,
                title: scanned_track.title,
                file_path: Some(track_file_path.clone()),
                hash: scanned_track.hash as i64,
              };
              event!(Level::DEBUG, ?new_track, "Inserting replaced track");
              let insert_query = diesel::insert_into(track)
                .values(new_track);
              time!("scan.insert_replaced_track", insert_query.execute(&self.connection)?);
              time!("scan.select_inserted_replaced_track", select_query.first::<Track>(&self.connection)?)
            } else {
              // When the hash is different, but the metadata is not, we assume that the track's audio data has
              // (somehow) changed, and just update the hash. When the hash is the same, but the metadata is not, the
              // metadata of the track was changed, and just update it. If neither was changed, no update will be
              // performed.
              event!(Level::TRACE, ?db_track, "Updating track with values from scanned track");
              let changed = db_track.update_from(&album, &scanned_track);
              if changed {
                event!(Level::DEBUG, ?db_track, "Track has changed, updating the track in the database");
                time!("scan.update_track", db_track.save_changes(&*self.connection)?)
              } else {
                db_track
              }
            }
          } else {
            // Did not find a track with the same path as the scanned track. Either the track is new, or it was moved.
            // We check if the track was moved by searching for the track by hash instead.
            let select_by_hash_query = track
              .filter(scan_directory_id.eq(scanned_track.scan_directory_id))
              .filter(hash.eq(scanned_track.hash as i64));
            let tracks_by_hash: Vec<Track> = time!("scan.select_track_by_hash", select_by_hash_query.load::<Track>(&self.connection)?);
            if tracks_by_hash.is_empty() {
              // No track with the same hash was found: we insert it as a new track.
              let new_track = NewTrack {
                source_id: scanned_track.scan_directory_id,
                album_id: album.id,
                disc_number: scanned_track.disc_number,
                disc_total: scanned_track.disc_total,
                track_number: scanned_track.track_number,
                track_total: scanned_track.track_total,
                title: scanned_track.title,
                file_path: Some(track_file_path.clone()),
                hash: scanned_track.hash as i64,
              };
              event!(Level::DEBUG, ?new_track, "Inserting track");
              let insert_query = diesel::insert_into(track)
                .values(new_track);
              time!("scan.insert_track", insert_query.execute(&self.connection)?);
              time!("scan.select_inserted_track", select_query.first::<Track>(&self.connection)?)
            } else if tracks_by_hash.len() == 1 {
              // A track with the same hash was found: we update the track in the database with the scanned track.
              let mut db_track: Track = tracks_by_hash.into_iter().take(1).next().unwrap();
              event!(Level::TRACE, ?db_track, "Updating moved track with values from scanned track");
              let changed = db_track.update_from(&album, &scanned_track);
              if changed {
                event!(Level::DEBUG, ?db_track, "Updating moved track");
                time!("scan.update_moved_track", db_track.save_changes(&*self.connection)?)
              } else {
                db_track
              }
            } else {
              // Multiple tracks with the same hash were found: for now, we error out.
              return Err(HashCollisionFail(scanned_track.clone(), tracks_by_hash));
            }
          }
        };
        // Process track artists.
        {
          let mut db_artists: HashSet<Artist> = self.update_or_insert_artists(scanned_track.track_artists.into_iter())?;
          use schema::track_artist::dsl::*;
          let db_track_artists: Vec<(TrackArtist, Artist)> = time!("scan.select_track_artists", track_artist
            .filter(track_id.eq(track.id))
            .inner_join(schema::artist::table)
            .load(&self.connection)?);
          for (db_track_artist, db_artist) in db_track_artists {
            if db_artists.contains(&db_artist) {
              // TODO: update track_artist columns if they are added.
              //let mut db_track_artist = db_track_artist;
              //time!("scan.update_track_artist", db_track_artist.save_changes(&self.connection)?)
            } else {
              event!(Level::DEBUG, ?db_track_artist, "Deleting track artist");
              time!("scan.delete_track_artist", diesel::delete(&db_track_artist).execute(&self.connection)?);
            }
            db_artists.remove(&db_artist); // Remove from set, so we know what to insert afterwards.
          }
          for artist in db_artists {
            let new_track_artist = NewTrackArtist { track_id: track.id, artist_id: artist.id };
            event!(Level::DEBUG, ?new_track_artist, "Inserting track artist");
            time!("scan.insert_track_artist", diesel::insert_into(track_artist)
              .values(new_track_artist)
              .execute(&self.connection)?);
          }
        }
        // Process album artists.
        {
          let mut db_artists: HashSet<Artist> = self.update_or_insert_artists(scanned_track.album_artists.into_iter())?;
          use schema::album_artist::dsl::*;
          let db_album_artists: Vec<(AlbumArtist, Artist)> = time!("scan.select_album_artists", album_artist
            .filter(album_id.eq(album.id))
            .inner_join(schema::artist::table)
            .load(&self.connection)?);
          for (db_album_artist, db_artist) in db_album_artists {
            if db_artists.contains(&db_artist) {
              // TODO: update album_artist columns if they are added.
              //let mut db_album_artist = db_album_artist;
              //time!("scan.update_album_artist", db_album_artist.save_changes(&self.connection)?)
            } else {
              event!(Level::DEBUG, ?db_album_artist, "Deleting album artist");
              time!("scan.delete_album_artist", diesel::delete(&db_album_artist).execute(&self.connection)?);
            }
            db_artists.remove(&db_artist); // Remove from set, so we know what to insert afterwards.
          }
          for artist in db_artists {
            let new_album_artist = NewAlbumArtist { album_id: album.id, artist_id: artist.id };
            event!(Level::DEBUG, ?new_album_artist, "Inserting album artist");
            time!("scan.insert_album_artist", diesel::insert_into(album_artist)
              .values(new_album_artist)
              .execute(&self.connection)?);
          }
        }
      }
      // Remove all tracks from the database that have a path that was not scanned.
      {
        let db_track_data: Vec<(i32, i32, Option<String>)> = {
          use schema::track::dsl::*;
          track
            .select((id, scan_directory_id, file_path))
            .filter(file_path.is_not_null())
            .load::<(i32, i32, Option<String>)>(&self.connection)?
        };
        for (db_track_id, db_scan_directory_id, db_file_path) in db_track_data {
          if let (Some(db_file_path), Some(scanned_file_paths)) = (db_file_path, scanned_file_paths.get(&db_scan_directory_id)) {
            if !scanned_file_paths.contains(&db_file_path) {
              event!(Level::DEBUG, ?db_track_id, ?db_file_path, "Track '{}' at '{}' has not been scanned: setting it as removed in the database", db_track_id, db_file_path);
              {
                use schema::track::dsl::*;
                time!("scan.update_removed_track", diesel::update(track)
                  .filter(id.eq(db_track_id))
                  .set(file_path.eq::<Option<String>>(None))
                  .execute(&self.connection)?);
              }
            }
          }
        }
      }
      Ok(())
    })?;
    if !scan_errors.is_empty() {
      return Err(ScanError::ScanFail(scan_errors));
    }
    Ok(())
  }

  fn update_or_insert_artists<I: IntoIterator<Item=String>>(&self, artist_names: I) -> Result<HashSet<Artist>, diesel::result::Error> {
    artist_names.into_iter().map(|artist_name| {
      use schema::artist::dsl::*;
      let select_query = artist
        .filter(name.eq(&artist_name));
      let db_artist = time!("scan.select_artist", select_query.first::<Artist>(&self.connection).optional()?);
      let db_artist = if let Some(db_artist) = db_artist {
        let db_artist: Artist = db_artist;
        // TODO: update artist columns when they are added.
        //time!("scan.update_artist", db_artist.save_changes(&self.connection)?)
        db_artist
      } else {
        let new_artist = NewArtist { name: artist_name.clone() };
        let insert_query = diesel::insert_into(artist)
          .values(new_artist);
        time!("scan.insert_artist", insert_query.execute(&self.connection)?);
        time!("scan.select_inserted_artist", select_query.first::<Artist>(&self.connection)?)
      };
      Ok(db_artist)
    }).collect()
  }
}

