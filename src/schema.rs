// @generated automatically by Diesel CLI.

diesel::table! {
    benchmarks (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        sku -> Text,
        category -> Text,
        units -> Text,
        price -> Double,
        amount -> Double,
        description -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    crawlers (id) {
        id -> Integer,
        hub_id -> Integer,
        name -> Text,
        url -> Text,
        selector -> Text,
        processing -> Bool,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    products (id) {
        id -> Integer,
        crawler_id -> Integer,
        name -> Text,
        sku -> Text,
        category -> Nullable<Text>,
        units -> Nullable<Text>,
        price -> Double,
        amount -> Nullable<Double>,
        description -> Nullable<Text>,
        url -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(products -> crawlers (crawler_id));

diesel::allow_tables_to_appear_in_same_query!(benchmarks, crawlers, products,);
