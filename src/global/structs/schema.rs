// @generated automatically by Diesel CLI.

diesel::table! {
    blocked_channels (id) {
        id -> Text,
    }
}

diesel::table! {
    blocked_playlists (id) {
        id -> Text,
    }
}

diesel::table! {
    search_history (query) {
        query -> Text,
        created_at -> BigInt,
    }
}

diesel::table! {
    subscription_tags (channel_id, tag) {
        channel_id -> Text,
        tag -> Text,
    }
}

diesel::table! {
    subscriptions (channel_id) {
        channel_id -> Text,
        name -> Text,
        thumbnail_url -> Text,
        last_sync -> BigInt,
        last_sync_channel -> BigInt,
        has_new -> BigInt,
    }
}

diesel::table! {
    videos (id) {
        id -> Text,
        channel_id -> Text,
        title -> Text,
        thumbnail_url -> Text,
        length -> Text,
        views -> Nullable<Text>,
        channel_name -> Text,
        published -> Nullable<Text>,
        timestamp -> Nullable<BigInt>,
        description -> Nullable<Text>,
    }
}

diesel::joinable!(subscription_tags -> subscriptions (channel_id));
diesel::joinable!(videos -> subscriptions (channel_id));

diesel::allow_tables_to_appear_in_same_query!(
    blocked_channels,
    blocked_playlists,
    search_history,
    subscription_tags,
    subscriptions,
    videos,
);
