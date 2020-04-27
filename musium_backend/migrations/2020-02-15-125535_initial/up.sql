-- Albums, tracks, artists, and relations between them.

CREATE TABLE album
(
    id   INTEGER NOT NULL,
    name TEXT    NOT NULL,

    PRIMARY KEY (id)
);

CREATE TABLE track
(
    id           INTEGER NOT NULL,
    album_id     INTEGER NOT NULL,
    disc_number  INTEGER,
    disc_total   INTEGER,
    track_number INTEGER,
    track_total  INTEGER,
    title        TEXT    NOT NULL,

    PRIMARY KEY (id),
    FOREIGN KEY (album_id) REFERENCES album (id)
);

CREATE TABLE artist
(
    id   INTEGER NOT NULL,
    name TEXT    NOT NULL,

    PRIMARY KEY (id)
);

CREATE TABLE track_artist
(
    track_id  INTEGER NOT NULL,
    artist_id INTEGER NOT NULL,

    PRIMARY KEY (track_id, artist_id),
    FOREIGN KEY (track_id) REFERENCES track (id),
    FOREIGN KEY (artist_id) REFERENCES artist (id)
);

CREATE TABLE album_artist
(
    album_id  INTEGER NOT NULL,
    artist_id INTEGER NOT NULL,

    PRIMARY KEY (album_id, artist_id),
    FOREIGN KEY (album_id) REFERENCES album (id),
    FOREIGN KEY (artist_id) REFERENCES artist (id)
);


-- Local source, which synchronizes files in a directory on the filesystem.

CREATE TABLE local_source
(
    id        INTEGER NOT NULL,
    enabled   BOOLEAN NOT NULL DEFAULT true,
    directory TEXT    NOT NULL,

    PRIMARY KEY (id),
    UNIQUE (directory)
);

-- Linking albums/tracks/artists to a local source with additional data.

CREATE TABLE local_album
(
    album_id        INTEGER NOT NULL,
    local_source_id INTEGER NOT NULL,
    -- TODO: MusicBrainz album ID.

    PRIMARY KEY (album_id, local_source_id),
    FOREIGN KEY (album_id) REFERENCES album (id),
    FOREIGN KEY (local_source_id) REFERENCES local_source (id)
);

CREATE TABLE local_track
(
    track_id        INTEGER NOT NULL,
    local_source_id INTEGER NOT NULL,
    file_path       TEXT,               -- Can be null to indicate that the track has been removed/replaced.
    hash            BIGINT  NOT NULL,   -- Hash as BIGINT, such that diesel maps to BigInt, which is an i64 containing an u32 hash in the positive bits.
    -- TODO: MusicBrainz track ID.
    -- TODO: AcousticID.

    PRIMARY KEY (track_id, local_source_id),
    FOREIGN KEY (track_id) REFERENCES track (id),
    FOREIGN KEY (local_source_id) REFERENCES local_source (id),
    UNIQUE (local_source_id, file_path) -- Every track belonging to the same source must have a unique (or null) file path.
);

CREATE TABLE local_artist
(
    artist_id       INTEGER NOT NULL,
    local_source_id INTEGER NOT NULL,
    -- TODO: MusicBrainz artist ID.

    PRIMARY KEY (artist_id, local_source_id),
    FOREIGN KEY (artist_id) REFERENCES artist (id),
    FOREIGN KEY (local_source_id) REFERENCES local_source (id)
);


-- Spotify source, which synchronizes Spotify albums/tracks/artists from the followed artists of a user.

CREATE TABLE spotify_source
(
    id            INTEGER  NOT NULL,
    user_id       INTEGER  NOT NULL,
    enabled       BOOLEAN  NOT NULL DEFAULT true,
    refresh_token TEXT     NOT NULL,
    access_token  TEXT     NOT NULL,
    expiry_date   DATETIME NOT NULL,

    PRIMARY KEY (id),
    FOREIGN KEY (user_id) REFERENCES user (id),
    UNIQUE (user_id)
);

-- Linking albums/tracks/artists to a Spotify ID and additional data.

CREATE TABLE spotify_album
(
    album_id   INTEGER NOT NULL,
    spotify_id TEXT    NOT NULL,

    PRIMARY KEY (album_id, spotify_id),
    FOREIGN KEY (album_id) REFERENCES album (id)
);

CREATE TABLE spotify_track
(
    track_id   INTEGER NOT NULL,
    spotify_id TEXT    NOT NULL,

    PRIMARY KEY (track_id, spotify_id),
    FOREIGN KEY (track_id) REFERENCES track (id)
);

CREATE TABLE spotify_artist
(
    artist_id  INTEGER NOT NULL,
    spotify_id TEXT    NOT NULL,

    PRIMARY KEY (artist_id, spotify_id),
    FOREIGN KEY (artist_id) REFERENCES artist (id)
);

-- Linking albums/tracks/artists to a Spotify source, which is linked to a specific user.

CREATE TABLE spotify_album_source
(
    album_id          INTEGER NOT NULL,
    spotify_source_id INTEGER NOT NULL,

    PRIMARY KEY (album_id, spotify_source_id),
    FOREIGN KEY (album_id) REFERENCES album (id),
    FOREIGN KEY (spotify_source_id) REFERENCES spotify_source (id)
);

CREATE TABLE spotify_track_source
(
    track_id          INTEGER NOT NULL,
    spotify_source_id INTEGER NOT NULL,

    PRIMARY KEY (track_id, spotify_source_id),
    FOREIGN KEY (track_id) REFERENCES track (id),
    FOREIGN KEY (spotify_source_id) REFERENCES spotify_source (id)
);

CREATE TABLE spotify_artist_source
(
    artist_id         INTEGER NOT NULL,
    spotify_source_id INTEGER NOT NULL,

    PRIMARY KEY (artist_id, spotify_source_id),
    FOREIGN KEY (artist_id) REFERENCES artist (id),
    FOREIGN KEY (spotify_source_id) REFERENCES spotify_source (id)
);


-- User

CREATE TABLE user
(
    id   INTEGER NOT NULL,
    name TEXT    NOT NULL,
    hash BLOB    NOT NULL,
    salt BLOB    NOT NULL,

    PRIMARY KEY (id),
    UNIQUE (name)
);


-- User data, connected to user + album/track/artist.

CREATE TABLE user_album_rating
(
    user_id  INTEGER NOT NULL,
    album_id INTEGER NOT NULL,
    rating   INTEGER NOT NULL,

    PRIMARY KEY (user_id, album_id),
    FOREIGN KEY (user_id) REFERENCES user (id),
    FOREIGN KEY (album_id) REFERENCES album (id)
);

CREATE TABLE user_track_rating
(
    user_id  INTEGER NOT NULL,
    track_id INTEGER NOT NULL,
    rating   INTEGER NOT NULL,

    PRIMARY KEY (user_id, track_id),
    FOREIGN KEY (user_id) REFERENCES user (id),
    FOREIGN KEY (track_id) REFERENCES track (id)
);

CREATE TABLE user_artist_rating
(
    user_id   INTEGER NOT NULL,
    artist_id INTEGER NOT NULL,
    rating    INTEGER NOT NULL,

    PRIMARY KEY (user_id, artist_id),
    FOREIGN KEY (user_id) REFERENCES user (id),
    FOREIGN KEY (artist_id) REFERENCES artist (id)
);
