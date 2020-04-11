use std::fs::File;
use std::io::{BufReader, Read, Seek};

use thiserror::Error;
use walkdir::WalkDir;

use musium_core::model::Source;

#[derive(Default, Clone, Debug)]
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
  #[error("Failed to seek file")]
  FileSeekFail(std::io::Error),
  #[error("Failed to check for ID3v2 tag")]
  Id3v2CheckFail(id3::Error),
  #[error("Failed to check for ID3v2 tag")]
  Id3v1CheckFail(id3::Error),
  #[error("Failed to skip ID3v2 tag (for hashing audio data only)")]
  Id3v2SkipFail(id3::Error),
  #[error("Failed to read ID3v2 tag")]
  Id3v2ReadFail(id3::Error),
  #[error("Failed to read ID3v1 tag")]
  Id3v1ReadFail(id3::Error),
  #[error("File '{0}' does not have a title")]
  NoTitleFail(String),
  #[error("File '{0}' does not have an album")]
  NoAlbumFail(String),
}

impl Scanner {
  pub fn scan(&self, scan_directory: Source) -> impl Iterator<Item=Result<ScannedTrack, ScanError>> {
    use ScanError::*;
    let Source { id: scan_directory_id, directory, .. } = scan_directory;
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
          // Open the MP3 file for reading.
          let mut buf_reader = {
            let file = match File::open(entry.path()) {
              Ok(file) => file,
              Err(e) => return Some(Err(FileReadFail(e))),
            };
            BufReader::new(file) // Buffer file reads to reduce number of system calls.
          };

          // Check which ID3 tags are present in the file.
          let has_id3v2_tag = match id3::Tag::is_candidate(&mut buf_reader) {
            Ok(b) => b,
            Err(e) => return Some(Err(Id3v2CheckFail(e))),
          };
          let has_id3v1_tag = match id3::v1::Tag::is_candidate(&mut buf_reader) {
            Ok(b) => b,
            Err(e) => return Some(Err(Id3v1CheckFail(e))),
          };
          if !has_id3v2_tag && !has_id3v1_tag {
            return None;
          }

          // Create file path, relative to scan directory.
          let file_path = entry.path()
            .strip_prefix(&directory)
            .unwrap_or_else(|_| panic!("BUG: cannot strip prefix, path '{}' is not prefixed by '{}'", entry.path().display(), directory))
            .to_string_lossy()
            .to_string();

          // Create hasher for hashing the audio data of the file.
          let mut hasher = crc32fast::Hasher::new();

          // Create scanned track from the ID3v1/2 tag.
          let scanned_track = if has_id3v2_tag {
            // Prefer ID3v2 tag, over ID3v1.
            let tag = match id3::Tag::read_from(&mut buf_reader) {
              Ok(tag) => tag,
              Err(e) => return Some(Err(Id3v2ReadFail(e))),
            };

            // Title and album must be present.
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

            // Reset reader to start and skip the ID3v2 tag to get to the audio data.
            match buf_reader.seek(std::io::SeekFrom::Start(0)) {
              Err(e) => return Some(Err(FileSeekFail(e))),
              _ => {}
            }
            match id3::Tag::skip(&mut buf_reader) {
              Err(e) => return Some(Err(Id3v2SkipFail(e))),
              _ => {}
            }
            // Read file to buffer.
            let mut buffer = Vec::new();
            if let Err(e) = buf_reader.read_to_end(&mut buffer) {
              return Some(Err(FileReadFail(e)));
            }
            // Calculate hash over the audio data.
            hasher.update(Scanner::skip_id3v1(&buffer)); // Possibly skip the ID3v1 tag which is at the end of the file.
            let hash = hasher.finalize();

            ScannedTrack {
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
            }
          } else if has_id3v1_tag {
            let tag = match id3::v1::Tag::read_from(&mut buf_reader) {
              Ok(tag) => tag,
              Err(e) => return Some(Err(Id3v1ReadFail(e))),
            };

            // Read file to buffer.
            let mut buffer = Vec::new();
            if let Err(e) = buf_reader.read_to_end(&mut buffer) {
              return Some(Err(FileReadFail(e)));
            }
            // Calculate hash over the audio data.
            hasher.update(Scanner::skip_id3v1(&buffer)); // Skip the ID3v1 tag which is at the end of the file.
            let hash = hasher.finalize();

            ScannedTrack {
              scan_directory_id,
              disc_number: None,
              disc_total: None,
              track_number: tag.track.map(|u| u as i32),
              track_total: None,
              title: tag.title,
              album: tag.album,
              track_artists: vec![tag.artist], // TODO: support multiple artists.
              album_artists: vec![],
              file_path,
              hash,
            }
          } else {
            return None;
          };

          Some(Ok(scanned_track))
        } else {
          None
        }
      })
  }

  fn skip_id3v1(buffer: &[u8]) -> &[u8] {
    let len = buffer.len();
    if len >= 355 && &buffer[len - 355..len - 355 + 4] == b"TAG+" {
      &buffer[0..len - 355]
    } else if len >= 128 && &buffer[len - 128..len - 128 + 3] == b"TAG" {
      &buffer[0..len - 128]
    } else {
      buffer
    }
  }
}
