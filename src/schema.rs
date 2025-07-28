// @generated automatically by Diesel CLI.

diesel::table! {
    crawlers (id) {
        id -> Integer,
        name -> Text,
        url -> Text,
        selector -> Text,
        processing -> Bool,
        updated_at -> Timestamp,
    }
}
