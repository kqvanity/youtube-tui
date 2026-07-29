use super::schema::*;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = subscriptions)]
pub struct Subscription {
    pub channel_id: String,
    pub name: String,
    pub thumbnail_url: String,
    pub last_sync: i64,
    pub last_sync_channel: i64,
    pub has_new: i64,
}

#[derive(Queryable, Selectable, Insertable, Associations)]
#[diesel(table_name = videos)]
#[diesel(belongs_to(Subscription, foreign_key = channel_id))]
pub struct Video {
    pub id: String,
    pub channel_id: String,
    pub title: String,
    pub thumbnail_url: String,
    pub length: String,
    pub views: Option<String>,
    pub channel_name: String,
    pub published: Option<String>,
    pub timestamp: Option<i64>,
    pub description: Option<String>,
}

#[derive(Queryable, Selectable, Insertable, Associations)]
#[diesel(table_name = subscription_tags)]
#[diesel(belongs_to(Subscription, foreign_key = channel_id))]
pub struct SubscriptionTag {
    pub channel_id: String,
    pub tag: String,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = search_history)]
pub struct SearchHistoryEntry {
    pub query: String,
    pub created_at: i64,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = blocked_channels)]
pub struct BlockedChannel {
    pub id: String,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = blocked_playlists)]
pub struct BlockedPlaylist {
    pub id: String,
}
