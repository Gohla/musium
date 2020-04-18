use std::path::PathBuf;

use musium_core::model::*;
use musium_core::schema::*;

// Helper macros

macro_rules! update {
  ($t:expr, $u:expr, $c:expr) => {
    if $t != $u {
      //event!(Level::TRACE, old = ?$t, new = ?$u, "Value changed");
      $t = $u;
      $c = true;
    }
  }
}

// Scan directory

pub trait LocalSourceDataEx {
  fn track_file_path(&self, track: &LocalTrack) -> Option<PathBuf>;
  //fn update_from(&mut self, enabled: bool) -> bool;
}

impl LocalSourceDataEx for LocalSourceData {
  fn track_file_path(&self, track: &LocalTrack) -> Option<PathBuf> {
    track.file_path.as_ref().map(|file_path| PathBuf::from(&self.directory).join(file_path))
  }

  // fn update_from(&mut self, enabled: bool) -> bool {
  //   let mut changed = false;
  //   update!(self.enabled, enabled, changed);
  //   changed
  // }
}

// Track

pub trait TrackEx {
  fn check_hash_changed(&mut self, scanned_track: &ScannedTrack) -> bool;
  fn check_metadata_changed(&mut self, album: &LocalAlbum, scanned_track: &ScannedTrack) -> bool;
  fn update_from(&mut self, album: &LocalAlbum, scanned_track: &ScannedTrack) -> bool;
}

impl TrackEx for LocalTrack {
  fn check_hash_changed(&mut self, scanned_track: &ScannedTrack) -> bool {
    self.hash != scanned_track.hash as i64
  }

  fn check_metadata_changed(&mut self, album: &LocalAlbum, scanned_track: &ScannedTrack) -> bool {
    if self.source_id != scanned_track.scan_directory_id { return true; }
    if self.album_id != album.id { return true; }
    if self.disc_number != scanned_track.disc_number { return true; }
    if self.disc_total != scanned_track.disc_total { return true; }
    if self.track_number != scanned_track.track_number { return true; }
    if self.track_total != scanned_track.track_total { return true; }
    if self.title != scanned_track.title { return true; }
    return false;
  }

  fn update_from(&mut self, album: &LocalAlbum, scanned_track: &ScannedTrack) -> bool {
    let mut changed = false;
    update!(self.source_id, scanned_track.scan_directory_id, changed);
    update!(self.album_id, album.id, changed);
    update!(self.disc_number, scanned_track.disc_number, changed);
    update!(self.disc_total, scanned_track.disc_total, changed);
    update!(self.track_number, scanned_track.track_number, changed);
    update!(self.track_total, scanned_track.track_total, changed);
    update!(self.title, scanned_track.title.clone(), changed);
    if let Some(file_path) = &mut self.file_path {
      if file_path != &scanned_track.file_path {
        *file_path = scanned_track.file_path.clone();
        changed = true;
      }
    } else {
      self.file_path = Some(scanned_track.file_path.clone());
      changed = true;
    }
    update!(self.hash, scanned_track.hash as i64, changed);
    changed
  }
}

// Internal user (includes password hash and salt)

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, Identifiable, Queryable, AsChangeset)]
#[table_name = "user"]
#[changeset_options(treat_none_as_null = "true")]
pub(crate) struct InternalUser {
  pub id: i32,
  pub name: String,
  pub hash: Vec<u8>,
  pub salt: Vec<u8>,
}

impl Into<User> for InternalUser {
  fn into(self) -> User {
    User {
      id: self.id,
      name: self.name,
    }
  }
}

#[derive(Debug, Insertable)]
#[table_name = "user"]
pub struct InternalNewUser {
  pub name: String,
  pub hash: Vec<u8>,
  pub salt: Vec<u8>,
}
