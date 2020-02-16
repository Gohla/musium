use id3::Tag;
use thiserror::Error;
use walkdir::WalkDir;

use crate::model::{NewTrack, ScanDirectory};

#[derive(Debug)]
pub struct Scanner {}

// Creation

impl Scanner {
  pub fn new() -> Self { Self {} }
}

// Scanning

#[derive(Debug, Error)]
pub enum ScanError {
  #[error("Failed to walk directory")]
  WalkDirFail(#[from] walkdir::Error),
  #[error("Failed to read ID3 tag")]
  Id3ReadFail(#[from] id3::Error),
}

impl Scanner {
  pub fn scan(&self, scan_directory: ScanDirectory) -> impl Iterator<Item=Result<NewTrack, ScanError>> {
    let ScanDirectory { id: scan_directory_id, directory } = scan_directory;
    WalkDir::new(&directory)
      .into_iter()
      .filter_map(move |entry| {
        let entry = match entry {
          Ok(entry) => entry,
          Err(e) => return Some(Err(ScanError::WalkDirFail(e))),
        };
        if !entry.file_type().is_file() { return None; }
        let file_name = entry.file_name().to_string_lossy();
        if file_name.ends_with(".mp3") {
          let tag = match Tag::read_from_path(entry.path()) {
            Ok(tag) => tag,
            Err(e) => return Some(Err(ScanError::Id3ReadFail(e))),
          };
          Some(Ok(NewTrack {
            scan_directory_id,
            disc_number: tag.disc().map(|u| u as i32),
            disc_total: tag.total_discs().map(|u| u as i32),
            track_number: tag.track().map(|u| u as i32),
            track_total: tag.total_tracks().map(|u| u as i32),
            title: tag.title().map(|s| s.to_string()),
            file_path: entry.path().strip_prefix(&directory)
              .unwrap_or_else(|_| panic!("BUG: cannot strip prefix, path '{}' is not prefixed by '{}'", entry.path().display(), directory))
              .to_string_lossy().to_string()
          }))
        } else {
          None
        }
      })
  }
}
