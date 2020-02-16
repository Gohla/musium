use std::fmt::{Display, Error, Formatter};

use crate::schema::{scan_directory, track};
use std::path::PathBuf;

// Track

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable, Associations)]
#[belongs_to(ScanDirectory)]
#[table_name = "track"]
pub struct Track {
  pub id: i32,
  pub scan_directory_id: i32,
  pub disc_number: Option<i32>,
  pub disc_total: Option<i32>,
  pub track_number: Option<i32>,
  pub track_total: Option<i32>,
  pub title: Option<String>,
  pub file_path: String,
}

#[derive(Debug, Insertable)]
#[table_name = "track"]
pub struct NewTrack {
  pub scan_directory_id: i32,
  pub disc_number: Option<i32>,
  pub disc_total: Option<i32>,
  pub track_number: Option<i32>,
  pub track_total: Option<i32>,
  pub title: Option<String>,
  pub file_path: String,
}

// Scan directory

#[derive(Clone, PartialOrd, PartialEq, Debug, Identifiable, Queryable)]
#[table_name = "scan_directory"]
pub struct ScanDirectory {
  pub id: i32,
  pub directory: String,
}

#[derive(Debug, Insertable)]
#[table_name = "scan_directory"]
pub struct NewScanDirectory {
  pub directory: String,
}

// Implementations

impl Display for Track {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
    write!(f, "{:>6}:", self.id)?;
    match (self.disc_number, self.disc_total) {
      (Some(number), Some(total)) => write!(f, " ({}/{})", number, total)?,
      (Some(number), _) => write!(f, "   ({})", number)?,
      _ => write!(f, "      ")?,
    }
    match (self.track_number, self.track_total) {
      (Some(number), Some(total)) => write!(f, " {:>3}/{:>3}.", number, total)?,
      (Some(number), _) => write!(f, "     {:>3}.", number)?,
      _ => write!(f, "         ")?,
    }
    if let Some(ref title) = self.title {
      write!(f, " {:<50}", title)?;
    } else {
      write!(f, " {:<50}", "<no title>")?;
    }
    write!(f, " - {}", self.file_path)?;
    Ok(())
  }
}

impl ScanDirectory {
  pub fn track_file_path(&self, track: &Track) -> PathBuf {
    PathBuf::from(&self.directory).join(&track.file_path)
  }
}

impl Display for ScanDirectory {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
    f.write_str(&self.directory)
  }
}
