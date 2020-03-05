CREATE TABLE scan_directory (
  id INTEGER NOT NULL,
  directory TEXT UNIQUE NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT true,

  PRIMARY KEY(id)
);

CREATE TABLE album (
  id INTEGER NOT NULL,
  name TEXT NOT NULL,

  PRIMARY KEY(id),
  UNIQUE(name)
);

CREATE TABLE track (
  id INTEGER NOT NULL,
  scan_directory_id INTEGER NOT NULL,
  album_id INTEGER NOT NULL,
  disc_number INTEGER,
  disc_total INTEGER,
  track_number INTEGER,
  track_total INTEGER,
  title TEXT NOT NULL,
  file_path TEXT,
  -- Hash as BIGINT, such that diesel maps to a BigInt, which is an i64 in rust, which can contain an u32 hash.
  hash BIGINT NOT NULL,

  PRIMARY KEY(id),
  FOREIGN KEY(scan_directory_id) REFERENCES scan_directory(id),
  FOREIGN KEY(album_id) REFERENCES album(id),
  UNIQUE(scan_directory_id, file_path)
);

CREATE TABLE artist (
  id INTEGER NOT NULL,
  name TEXT UNIQUE NOT NULL,

  PRIMARY KEY(id)
);

CREATE TABLE track_artist (
  track_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,

  PRIMARY KEY(track_id, artist_id),
  FOREIGN KEY(track_id) REFERENCES track(id),
  FOREIGN KEY(artist_id) REFERENCES artist(id)
);

CREATE TABLE album_artist (
  album_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,

  PRIMARY KEY(album_id, artist_id),
  FOREIGN KEY(album_id) REFERENCES album(id),
  FOREIGN KEY(artist_id) REFERENCES artist(id)
);

CREATE TABLE user (
  id INTEGER NOT NULL,
  name TEXT NOT NULL,

  PRIMARY KEY(id),
  UNIQUE(name)
);

CREATE TABLE user_album_rating (
  user_id INTEGER NOT NULL,
  album_id INTEGER NOT NULL,
  rating INTEGER NOT NULL,

  PRIMARY KEY(user_id, album_id),
  FOREIGN KEY(user_id) REFERENCES user(id),
  FOREIGN KEY(album_id) REFERENCES album(id)
);

CREATE TABLE user_track_rating (
  user_id INTEGER NOT NULL,
  track_id INTEGER NOT NULL,
  rating INTEGER NOT NULL,

  PRIMARY KEY(user_id, track_id),
  FOREIGN KEY(user_id) REFERENCES user(id),
  FOREIGN KEY(track_id) REFERENCES track(id)
);

CREATE TABLE user_artist_rating (
  user_id INTEGER NOT NULL,
  artist_id INTEGER NOT NULL,
  rating INTEGER NOT NULL,

  PRIMARY KEY(user_id, artist_id),
  FOREIGN KEY(user_id) REFERENCES user(id),
  FOREIGN KEY(artist_id) REFERENCES artist(id)
);
