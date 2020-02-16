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
        disc_number -> Nullable<Integer>,
        disc_total -> Nullable<Integer>,
        track_number -> Nullable<Integer>,
        track_total -> Nullable<Integer>,
        title -> Nullable<Text>,
        file_path -> Text,
    }
}

joinable!(track -> scan_directory (scan_directory_id));

allow_tables_to_appear_in_same_query!(
    scan_directory,
    track,
);
