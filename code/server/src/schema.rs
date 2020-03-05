table! {
    album (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    album_artist (album_id, artist_id) {
        album_id -> Integer,
        artist_id -> Integer,
    }
}

table! {
    artist (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    scan_directory (id) {
        id -> Integer,
        directory -> Text,
        enabled -> Bool,
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
        title -> Text,
        file_path -> Nullable<Text>,
        hash -> BigInt,
    }
}

table! {
    track_artist (track_id, artist_id) {
        track_id -> Integer,
        artist_id -> Integer,
    }
}

table! {
    user (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    user_album_rating (user_id, album_id) {
        user_id -> Integer,
        album_id -> Integer,
        rating -> Integer,
    }
}

table! {
    user_artist_rating (user_id, artist_id) {
        user_id -> Integer,
        artist_id -> Integer,
        rating -> Integer,
    }
}

table! {
    user_track_rating (user_id, track_id) {
        user_id -> Integer,
        track_id -> Integer,
        rating -> Integer,
    }
}

joinable!(album_artist -> album (album_id));
joinable!(album_artist -> artist (artist_id));
joinable!(track -> album (album_id));
joinable!(track -> scan_directory (scan_directory_id));
joinable!(track_artist -> artist (artist_id));
joinable!(track_artist -> track (track_id));
joinable!(user_album_rating -> album (album_id));
joinable!(user_album_rating -> user (user_id));
joinable!(user_artist_rating -> artist (artist_id));
joinable!(user_artist_rating -> user (user_id));
joinable!(user_track_rating -> track (track_id));
joinable!(user_track_rating -> user (user_id));

allow_tables_to_appear_in_same_query!(
    album,
    album_artist,
    artist,
    scan_directory,
    track,
    track_artist,
    user,
    user_album_rating,
    user_artist_rating,
    user_track_rating,
);
