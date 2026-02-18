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
        embedding -> Nullable<Binary>,
        processing -> Bool,
        num_products -> Integer,
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
        num_products -> Integer,
    }
}

diesel::table! {
    product_benchmark (product_id, benchmark_id) {
        product_id -> Integer,
        benchmark_id -> Integer,
        distance -> Float,
    }
}

diesel::table! {
    product_images (id) {
        id -> Integer,
        product_id -> Integer,
        url -> Text,
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
        embedding -> Nullable<Binary>,
    }
}

diesel::table! {
    products_fts (rowid) {
        rowid -> Integer,
        name -> Nullable<Binary>,
        sku -> Nullable<Binary>,
        category -> Nullable<Binary>,
        description -> Nullable<Binary>,
        #[sql_name = "products_fts"]
        products_fts_col -> Nullable<Binary>,
        rank -> Nullable<Binary>,
    }
}

diesel::table! {
    products_fts_config (k) {
        k -> Binary,
        v -> Nullable<Binary>,
    }
}

diesel::table! {
    products_fts_data (id) {
        id -> Nullable<Integer>,
        block -> Nullable<Binary>,
    }
}

diesel::table! {
    products_fts_docsize (id) {
        id -> Nullable<Integer>,
        sz -> Nullable<Binary>,
    }
}

diesel::table! {
    products_fts_idx (segid, term) {
        segid -> Binary,
        term -> Binary,
        pgno -> Nullable<Binary>,
    }
}

diesel::joinable!(product_benchmark -> benchmarks (benchmark_id));
diesel::joinable!(product_benchmark -> products (product_id));
diesel::joinable!(product_images -> products (product_id));
diesel::joinable!(products -> crawlers (crawler_id));

diesel::allow_tables_to_appear_in_same_query!(
    benchmarks,
    crawlers,
    product_benchmark,
    product_images,
    products,
    products_fts,
    products_fts_config,
    products_fts_data,
    products_fts_docsize,
    products_fts_idx,
);
