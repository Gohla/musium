table! {
    album (id) {
        id -> Integer,
        name -> Nullable<Text>,
    }
}

table! {
    album_artist (album_id) {
        album_id -> Integer,
        artist_id -> Integer,
    }
}

table! {
    artist (id) {
        id -> Integer,
        name -> Nullable<Text>,
    }
}

table! {
    scan_directory (id) {
        id -> Integer,
        directory -> Text,
    }
}

table! {
    track (id) {
        id -> Integer,
        scan_directory_id -> Integer,
        album_id -> Integer,
        disc_number -> Nullable<Integer>,
        disc_total -> Nullable<Integer>,
        track_number -> Nullable<Integer>,
        track_total -> Nullable<Integer>,
        title -> Nullable<Text>,
        file_path -> Text,
    }
}

table! {
    track_artist (track_id, artist_id) {
        track_id -> Integer,
        artist_id -> Integer,
    }
}

joinable!(album_artist -> album (album_id));
joinable!(album_artist -> artist (artist_id));
joinable!(track -> album (album_id));
joinable!(track -> scan_directory (scan_directory_id));
joinable!(track_artist -> artist (artist_id));
joinable!(track_artist -> track (track_id));

allow_tables_to_appear_in_same_query!(
    album,
    album_artist,
    artist,
    scan_directory,
    track,
    track_artist,
);
