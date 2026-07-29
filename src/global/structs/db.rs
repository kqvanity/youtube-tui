use super::models::*;
use super::schema::*;
use diesel::prelude::*;
use diesel::sql_query;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use home::home_dir;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

static mut DATABASE: OnceLock<DatabaseManager> = OnceLock::new();

pub struct DatabaseManager {
    conn: Mutex<SqliteConnection>,
}

impl DatabaseManager {
    pub fn init() {
        let db_path = home_dir()
            .expect("Failed to get home directory")
            .join(".local/share/youtube-tui/youtube-tui.db");

        // Ensure the parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        let database_url = db_path.to_str().unwrap();
        let mut conn = SqliteConnection::establish(database_url)
            .expect(&format!("Error connecting to {}", database_url));

        // Enable foreign keys and WAL mode for better concurrency
        sql_query("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .execute(&mut conn)
            .unwrap();

        // Run migrations
        conn.run_pending_migrations(MIGRATIONS).unwrap();

        unsafe {
            let _ = DATABASE.set(Self {
                conn: Mutex::new(conn),
            });
        }
    }

    // ─── Blocked channels ───────────────────────────────────────────────────────

    /// Block a channel by its ID
    pub fn block_channel(id: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        diesel::insert_into(blocked_channels::table)
            .values(&BlockedChannel { id: id.to_string() })
            .on_conflict_do_nothing()
            .execute(&mut *conn)?;
            
        Ok(())
    }

    /// Unblock a channel by its ID
    pub fn unblock_channel(id: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        diesel::delete(blocked_channels::table.filter(blocked_channels::id.eq(id)))
            .execute(&mut *conn)?;
            
        Ok(())
    }

    /// Check if a channel is blocked
    pub fn is_blocked(id: &str) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let count: i64 = blocked_channels::table
            .filter(blocked_channels::id.eq(id))
            .count()
            .get_result(&mut *conn)?;
            
        Ok(count > 0)
    }

    /// Retrieves all blocked channels as a HashSet for quick lookup
    pub fn get_all_blocked_channels() -> std::result::Result<HashSet<String>, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let channels: Vec<String> = blocked_channels::table
            .select(blocked_channels::id)
            .load(&mut *conn)?;
            
        Ok(channels.into_iter().collect())
    }

    // ─── Blocked playlists ──────────────────────────────────────────────────────

    /// Block a playlist by its ID
    pub fn block_playlist(id: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        diesel::insert_into(blocked_playlists::table)
            .values(&BlockedPlaylist { id: id.to_string() })
            .on_conflict_do_nothing()
            .execute(&mut *conn)?;
            
        Ok(())
    }

    /// Unblock a playlist by its ID
    pub fn unblock_playlist(id: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        diesel::delete(blocked_playlists::table.filter(blocked_playlists::id.eq(id)))
            .execute(&mut *conn)?;
            
        Ok(())
    }

    /// Retrieves all blocked playlists as a HashSet for quick lookup
    pub fn get_all_blocked_playlists() -> std::result::Result<HashSet<String>, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let playlists: Vec<String> = blocked_playlists::table
            .select(blocked_playlists::id)
            .load(&mut *conn)?;
            
        Ok(playlists.into_iter().collect())
    }

    // ─── Subscriptions ──────────────────────────────────────────────────────────

    /// Insert or update a subscription and its videos atomically.
    /// The `tags` field on `SubItem` is NOT written here —
    /// use `tag_subscription` / `untag_subscription` for tags.
    pub fn upsert_subscription(item: &crate::global::structs::SubItem) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        conn.transaction::<_, Box<dyn std::error::Error>, _>(|conn| {
            let sub = Subscription {
                channel_id: item.channel.id.clone(),
                name: item.channel.name.clone(),
                thumbnail_url: item.channel.thumbnail_url.clone(),
                last_sync: item.last_sync as i64,
                last_sync_channel: item.last_sync_channel as i64,
                has_new: if item.has_new { 1 } else { 0 },
            };

            diesel::insert_into(subscriptions::table)
                .values(&sub)
                .on_conflict(subscriptions::channel_id)
                .do_update()
                .set((
                    subscriptions::name.eq(&sub.name),
                    subscriptions::thumbnail_url.eq(&sub.thumbnail_url),
                    subscriptions::last_sync.eq(sub.last_sync),
                    subscriptions::last_sync_channel.eq(sub.last_sync_channel),
                    subscriptions::has_new.eq(sub.has_new),
                ))
                .execute(conn)?;

            diesel::delete(videos::table.filter(videos::channel_id.eq(&item.channel.id)))
                .execute(conn)?;

            let new_videos: Vec<Video> = item.videos.iter().map(|video| Video {
                id: video.id.clone(),
                channel_id: item.channel.id.clone(),
                title: video.title.clone(),
                thumbnail_url: video.thumbnail_url.clone(),
                length: video.length.clone(),
                views: video.views.clone(),
                channel_name: video.channel.clone(),
                published: video.published.clone(),
                timestamp: video.timestamp.map(|t| t as i64),
                description: video.description.clone(),
            }).collect();

            // Insert in chunks of 50 to respect SQLite variable limits safely
            for chunk in new_videos.chunks(50) {
                diesel::insert_into(videos::table)
                    .values(chunk)
                    .execute(conn)?;
            }
            Ok(())
        })?;

        Ok(())
    }

    /// Remove a subscription (and all its tags/videos via CASCADE).
    pub fn remove_subscription(channel_id_val: &str) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let rows = diesel::delete(subscriptions::table.filter(subscriptions::channel_id.eq(channel_id_val)))
            .execute(&mut *conn)?;
            
        Ok(rows > 0)
    }

    /// Returns true if the channel is already in the subscriptions table.
    pub fn is_subscribed(channel_id_val: &str) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let count: i64 = subscriptions::table
            .filter(subscriptions::channel_id.eq(channel_id_val))
            .count()
            .get_result(&mut *conn)?;
            
        Ok(count > 0)
    }

    /// Load all subscriptions (with their tags) from the database.
    /// Each returned `SubItem` has `tags` populated.
    pub fn get_all_subscriptions() -> std::result::Result<Vec<crate::global::structs::SubItem>, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();

        let subs: Vec<Subscription> = subscriptions::table.load(&mut *conn)?;
        let tags: Vec<SubscriptionTag> = subscription_tags::table
            .order_by((subscription_tags::channel_id.asc(), subscription_tags::tag.asc()))
            .load(&mut *conn)?;
        let mut vids: Vec<Video> = videos::table
            .order_by(videos::timestamp.desc())
            .load(&mut *conn)?;

        let mut tag_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for t in tags {
            tag_map.entry(t.channel_id).or_default().push(t.tag);
        }

        let mut videos_by_channel: std::collections::HashMap<String, Vec<crate::global::structs::MiniVideoItem>> = std::collections::HashMap::new();
        for v in vids.drain(..) {
            videos_by_channel
                .entry(v.channel_id.clone())
                .or_default()
                .push(crate::global::structs::MiniVideoItem {
                    id: v.id,
                    title: v.title,
                    thumbnail_url: v.thumbnail_url,
                    length: v.length,
                    views: v.views,
                    channel: v.channel_name,
                    channel_id: v.channel_id,
                    published: v.published,
                    timestamp: v.timestamp.map(|t| t as u64),
                    description: v.description,
                });
        }

        let mut items = Vec::with_capacity(subs.len());
        for sub in subs {
            use crate::global::structs::{FullChannelItem, SubItem};
            let channel = FullChannelItem {
                id: sub.channel_id.clone(),
                name: sub.name,
                thumbnail_url: sub.thumbnail_url,
                sub_count: 0,
                sub_count_text: String::new(),
                total_views: String::new(),
                created: String::new(),
                autogenerated: false,
                description: String::new(),
            };
            let tags = tag_map.remove(&sub.channel_id).unwrap_or_default();
            let videos = videos_by_channel.remove(&sub.channel_id).unwrap_or_default();

            items.push(SubItem {
                channel,
                videos,
                last_sync: sub.last_sync as u64,
                last_sync_channel: sub.last_sync_channel as u64,
                has_new: sub.has_new != 0,
                tags,
            });
        }

        Ok(items)
    }

    /// Get all videos across all subscriptions, sorted by timestamp descending.
    pub fn get_all_videos() -> std::result::Result<Vec<crate::global::structs::MiniVideoItem>, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let vids: Vec<Video> = videos::table
            .order_by(videos::timestamp.desc())
            .load(&mut *conn)?;

        Ok(vids
            .into_iter()
            .map(|v| crate::global::structs::MiniVideoItem {
                id: v.id,
                title: v.title,
                thumbnail_url: v.thumbnail_url,
                length: v.length,
                views: v.views,
                channel: v.channel_name,
                channel_id: String::new(),
                published: v.published,
                timestamp: v.timestamp.map(|t| t as u64),
                description: v.description,
            })
            .collect())
    }

    // ─── Tags ───────────────────────────────────────────────────────────────────

    /// Add a tag to an existing subscription. Returns an error if the channel
    /// is not subscribed.
    pub fn tag_subscription(channel_id_val: &str, tag_val: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        diesel::insert_into(subscription_tags::table)
            .values(&SubscriptionTag {
                channel_id: channel_id_val.to_string(),
                tag: tag_val.to_string(),
            })
            .on_conflict_do_nothing()
            .execute(&mut *conn)?;
            
        Ok(())
    }

    /// Remove a tag from a subscription.
    pub fn untag_subscription(channel_id_val: &str, tag_val: &str) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let rows = diesel::delete(
            subscription_tags::table
                .filter(subscription_tags::channel_id.eq(channel_id_val))
                .filter(subscription_tags::tag.eq(tag_val))
        )
        .execute(&mut *conn)?;
        Ok(rows > 0)
    }

    /// Get all tags for a subscription.
    pub fn get_tags_for(channel_id_val: &str) -> std::result::Result<Vec<String>, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let tags: Vec<String> = subscription_tags::table
            .filter(subscription_tags::channel_id.eq(channel_id_val))
            .order_by(subscription_tags::tag.asc())
            .select(subscription_tags::tag)
            .load(&mut *conn)?;
            
        Ok(tags)
    }

    /// Get all subscriptions that have a specific tag.
    pub fn get_subscriptions_by_tag(tag: &str) -> std::result::Result<Vec<crate::global::structs::SubItem>, Box<dyn std::error::Error>> {
        let all = Self::get_all_subscriptions()?;
        Ok(all
            .into_iter()
            .filter(|item| item.tags.iter().any(|t| t == tag))
            .collect())
    }

    // ─── Search History ──────────────────────────────────────────────────────────

    pub fn get_search_history() -> std::result::Result<Vec<String>, Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        let items: Vec<String> = search_history::table
            .order_by(search_history::created_at.desc())
            .select(search_history::query)
            .load(&mut *conn)?;
            
        Ok(items)
    }

    pub fn add_search_history(query_val: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();
        
        diesel::insert_into(search_history::table)
            .values(&SearchHistoryEntry {
                query: query_val.to_string(),
                created_at: chrono::Utc::now().timestamp(),
            })
            .on_conflict(search_history::query)
            .do_update()
            .set(search_history::created_at.eq(chrono::Utc::now().timestamp()))
            .execute(&mut *conn)?;
            
        Ok(())
    }

    pub fn trim_search_history(limit: usize) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let mut conn = db.conn.lock().unwrap();

        let l = limit as i64;
        sql_query(format!(
            "DELETE FROM search_history WHERE rowid NOT IN (
                SELECT rowid FROM search_history ORDER BY created_at DESC LIMIT {}
            )", l
        ))
        .execute(&mut *conn)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global::structs::{FullChannelItem, MiniVideoItem, SubItem};
    use diesel::sqlite::SqliteConnection;
    use diesel::prelude::*;

    // A small helper to initialize the global DB once for all tests.
    fn setup_test_db() {
        let mut conn = SqliteConnection::establish(":memory:").unwrap();
        conn.run_pending_migrations(MIGRATIONS).unwrap();
        
        unsafe {
            // Force replace if already set, because Rust tests run in parallel or independently
            // and we might need it, but we can't easily reset OnceLock without nightly.
            let _ = DATABASE.set(DatabaseManager {
                conn: std::sync::Mutex::new(conn),
            });
        }
    }

    #[test]
    fn test_all_db_operations() {
        setup_test_db();
        
        // 1. Blocked channels
        DatabaseManager::block_channel("ch1").unwrap();
        assert!(DatabaseManager::is_blocked("ch1").unwrap());
        assert!(!DatabaseManager::is_blocked("ch2").unwrap());
        
        let blocked = DatabaseManager::get_all_blocked_channels().unwrap();
        assert!(blocked.contains("ch1"));
        
        DatabaseManager::unblock_channel("ch1").unwrap();
        assert!(!DatabaseManager::is_blocked("ch1").unwrap());
        
        // 2. Playlists
        DatabaseManager::block_playlist("pl1").unwrap();
        let blocked_pls = DatabaseManager::get_all_blocked_playlists().unwrap();
        assert!(blocked_pls.contains("pl1"));
        DatabaseManager::unblock_playlist("pl1").unwrap();
        
        // 3. Subscriptions
        let sub = SubItem {
            channel: FullChannelItem {
                id: "sub1".to_string(),
                name: "My Channel".to_string(),
                thumbnail_url: "thumb".to_string(),
                sub_count: 0,
                sub_count_text: "".to_string(),
                total_views: "".to_string(),
                created: "".to_string(),
                autogenerated: false,
                description: "".to_string(),
            },
            videos: vec![
                MiniVideoItem {
                    id: "vid1".to_string(),
                    title: "Vid 1".to_string(),
                    thumbnail_url: "".to_string(),
                    length: "1:00".to_string(),
                    views: Some("1".to_string()),
                    channel: "My Channel".to_string(),
                    channel_id: "sub1".to_string(),
                    published: None,
                    timestamp: Some(100),
                    description: None,
                }
            ],
            last_sync: 200,
            last_sync_channel: 200,
            has_new: true,
            tags: vec![],
        };
        
        DatabaseManager::upsert_subscription(&sub).unwrap();
        assert!(DatabaseManager::is_subscribed("sub1").unwrap());
        
        let subs = DatabaseManager::get_all_subscriptions().unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].channel.name, "My Channel");
        assert_eq!(subs[0].videos.len(), 1);
        
        let vids = DatabaseManager::get_all_videos().unwrap();
        assert_eq!(vids.len(), 1);
        assert_eq!(vids[0].id, "vid1");
        
        // 4. Tags
        DatabaseManager::tag_subscription("sub1", "tech").unwrap();
        let tags = DatabaseManager::get_tags_for("sub1").unwrap();
        assert_eq!(tags, vec!["tech".to_string()]);
        
        let sub_by_tag = DatabaseManager::get_subscriptions_by_tag("tech").unwrap();
        assert_eq!(sub_by_tag.len(), 1);
        
        DatabaseManager::untag_subscription("sub1", "tech").unwrap();
        assert!(DatabaseManager::get_tags_for("sub1").unwrap().is_empty());
        
        DatabaseManager::remove_subscription("sub1").unwrap();
        assert!(!DatabaseManager::is_subscribed("sub1").unwrap());
        
        // 5. Search history
        DatabaseManager::add_search_history("query 1").unwrap();
        DatabaseManager::add_search_history("query 2").unwrap();
        let history = DatabaseManager::get_search_history().unwrap();
        assert_eq!(history.len(), 2);
        
        DatabaseManager::trim_search_history(1).unwrap();
        let history_trimmed = DatabaseManager::get_search_history().unwrap();
        assert_eq!(history_trimmed.len(), 1);
    }
}
