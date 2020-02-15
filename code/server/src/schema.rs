table! {
    tracks (id) {
        id -> Integer,
        disc_number -> Nullable<Integer>,
        disc_total -> Nullable<Integer>,
        track_number -> Nullable<Integer>,
        track_total -> Nullable<Integer>,
        title -> Text,
        file -> Text,
    }
}
