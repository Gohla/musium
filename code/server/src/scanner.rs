use std::fs::File;
use std::io::{Read, BufReader};

use id3::Tag;
use thiserror::Error;
use walkdir::WalkDir;

use crate::model::ScanDirectory;

#[derive(Default, Debug)]
pub struct Scanner {}

// Creation

impl Scanner {
  pub fn new() -> Self {
    Self {}
  }
}

// Scanning

#[derive(Clone, Debug)]
pub struct ScannedTrack {
  pub scan_directory_id: i32,
  pub disc_number: Option<i32>,
  pub disc_total: Option<i32>,
  pub track_number: Option<i32>,
  pub track_total: Option<i32>,
  pub title: String,
  pub album: String,
  pub track_artists: Vec<String>,
  // OPTO: smallvec
  pub album_artists: Vec<String>,
  // OPTO: smallvec
  pub file_path: String,
  pub hash: u32,
}

#[derive(Debug, Error)]
pub enum ScanError {
  #[error("Failed to walk directory")]
  WalkDirFail(#[from] walkdir::Error),
  #[error("Failed to open file for reading")]
  FileOpenFail(std::io::Error),
  #[error("Failed to read from file")]
  FileReadFail(std::io::Error),
  #[error("Failed to read ID3 tag")]
  Id3ReadFail(#[from] id3::Error),
  #[error("File '{0}' does not have a title")]
  NoTitleFail(String),
  #[error("File '{0}' does not have an album")]
  NoAlbumFail(String),
}

impl Scanner {
  pub fn scan(&self, scan_directory: ScanDirectory) -> impl Iterator<Item=Result<ScannedTrack, ScanError>> {
    use ScanError::*;
    let ScanDirectory { id: scan_directory_id, directory } = scan_directory;
    WalkDir::new(&directory)
      .into_iter()
      .filter_map(move |entry| {
        let entry = match entry {
          Ok(entry) => entry,
          Err(e) => return Some(Err(WalkDirFail(e))),
        };
        if !entry.file_type().is_file() { return None; }
        let file_name = entry.file_name().to_string_lossy();
        if file_name.ends_with(".mp3") {
          let file = match File::open(entry.path()) {
            Ok(file) => file,
            Err(e) => return Some(Err(FileReadFail(e))),
          };
          let mut buf_reader = BufReader::new(file);
          let tag = match Tag::read_from(&mut buf_reader) {
            Ok(tag) => tag,
            Err(e) => return Some(Err(Id3ReadFail(e))),
          };
          if let Err(e) = Tag::skip(&mut buf_reader) {
            return Some(Err(Id3ReadFail(e)));
          }
          let mut buffer = Vec::new();
          if let Err(e) = buf_reader.read_to_end(&mut buffer) {
            return Some(Err(FileReadFail(e)));
          }

          let file_path = entry.path()
            .strip_prefix(&directory)
            .unwrap_or_else(|_| panic!("BUG: cannot strip prefix, path '{}' is not prefixed by '{}'", entry.path().display(), directory))
            .to_string_lossy()
            .to_string();
          let title = if let Some(title) = tag.title() {
            title.to_string()
          } else {
            return Some(Err(NoTitleFail(file_path.clone())));
          };
          let album = if let Some(album) = tag.album() {
            album.to_string()
          } else {
            return Some(Err(NoAlbumFail(file_path.clone())));
          };
          let mut hasher = crc32fast::Hasher::new();
          hasher.update(&buffer);
          let hash = hasher.finalize();

          Some(Ok(ScannedTrack {
            scan_directory_id,
            disc_number: tag.disc().map(|u| u as i32),
            disc_total: tag.total_discs().map(|u| u as i32),
            track_number: tag.track().map(|u| u as i32),
            track_total: tag.total_tracks().map(|u| u as i32),
            title,
            album,
            track_artists: tag.artist().map_or(vec![], |a| vec![a.to_string()]), // TODO: support multiple artists.
            album_artists: tag.album_artist().map_or(vec![], |a| vec![a.to_string()]), // TODO: support multiple artists.
            file_path,
            hash,
          }))
        } else {
          None
        }
      })
  }
}
