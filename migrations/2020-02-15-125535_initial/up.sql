CREATE TABLE track (
  id INTEGER NOT NULL PRIMARY KEY,
  scan_directory_id INTEGER NOT NULL,
  disc_number INTEGER,
  disc_total INTEGER,
  track_number INTEGER,
  track_total INTEGER,
  title TEXT,
  file_path TEXT NOT NULL,
  FOREIGN KEY(scan_directory_id) REFERENCES scan_directory(id)
);

CREATE TABLE scan_directory (
  id INTEGER NOT NULL PRIMARY KEY,
  directory TEXT UNIQUE NOT NULL
);
