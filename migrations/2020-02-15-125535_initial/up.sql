CREATE TABLE tracks (
  id INTEGER NOT NULL PRIMARY KEY,
  disc_number INTEGER,
  disc_total INTEGER,
  track_number INTEGER,
  track_total INTEGER,
  title TEXT NOT NULL,
  file TEXT NOT NULL
)
