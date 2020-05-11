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
    local_album (album_id, local_source_id) {
        album_id -> Integer,
        local_source_id -> Integer,
    }
}

table! {
    local_artist (artist_id, local_source_id) {
        artist_id -> Integer,
        local_source_id -> Integer,
    }
}

table! {
    local_source (id) {
        id -> Integer,
        enabled -> Bool,
        directory -> Text,
    }
}

table! {
    local_track (track_id, local_source_id) {
        track_id -> Integer,
        local_source_id -> Integer,
        file_path -> Nullable<Text>,
        hash -> BigInt,
    }
}

table! {
    source (id) {
        id -> Integer,
        enabled -> Bool,
        data -> Text,
    }
}

table! {
    spotify_album (album_id, spotify_id) {
        album_id -> Integer,
        spotify_id -> Text,
    }
}

table! {
    spotify_album_source (album_id, spotify_source_id) {
        album_id -> Integer,
        spotify_source_id -> Integer,
    }
}

table! {
    spotify_artist (artist_id, spotify_id) {
        artist_id -> Integer,
        spotify_id -> Text,
    }
}

table! {
    spotify_artist_source (artist_id, spotify_source_id) {
        artist_id -> Integer,
        spotify_source_id -> Integer,
    }
}

table! {
    spotify_source (id) {
        id -> Integer,
        user_id -> Integer,
        enabled -> Bool,
        refresh_token -> Text,
        access_token -> Text,
        expiry_date -> Timestamp,
    }
}

table! {
    spotify_track (track_id, spotify_id) {
        track_id -> Integer,
        spotify_id -> Text,
    }
}

table! {
    spotify_track_source (track_id, spotify_source_id) {
        track_id -> Integer,
        spotify_source_id -> Integer,
    }
}

table! {
    track (id) {
        id -> Integer,
        album_id -> Integer,
        disc_number -> Nullable<Integer>,
        disc_total -> Nullable<Integer>,
        track_number -> Nullable<Integer>,
        track_total -> Nullable<Integer>,
        title -> Text,
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
        hash -> Binary,
        salt -> Binary,
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
joinable!(local_album -> album (album_id));
joinable!(local_album -> local_source (local_source_id));
joinable!(local_artist -> artist (artist_id));
joinable!(local_artist -> local_source (local_source_id));
joinable!(local_track -> local_source (local_source_id));
joinable!(local_track -> track (track_id));
joinable!(spotify_album -> album (album_id));
joinable!(spotify_album_source -> album (album_id));
joinable!(spotify_album_source -> spotify_source (spotify_source_id));
joinable!(spotify_artist -> artist (artist_id));
joinable!(spotify_artist_source -> artist (artist_id));
joinable!(spotify_artist_source -> spotify_source (spotify_source_id));
joinable!(spotify_source -> user (user_id));
joinable!(spotify_track -> track (track_id));
joinable!(spotify_track_source -> spotify_source (spotify_source_id));
joinable!(spotify_track_source -> track (track_id));
joinable!(track -> album (album_id));
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
    local_album,
    local_artist,
    local_source,
    local_track,
    source,
    spotify_album,
    spotify_album_source,
    spotify_artist,
    spotify_artist_source,
    spotify_source,
    spotify_track,
    spotify_track_source,
    track,
    track_artist,
    user,
    user_album_rating,
    user_artist_rating,
    user_track_rating,
);
