use home::home_dir;
use rusqlite::{params, Connection, Result};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

static mut DATABASE: OnceLock<DatabaseManager> = OnceLock::new();

pub struct DatabaseManager {
    conn: Mutex<Connection>,
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

        let conn = Connection::open(db_path).unwrap();

        // Enable foreign keys and WAL mode for better concurrency
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .unwrap();

        // Create the blocked_channels table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocked_channels (
                id TEXT PRIMARY KEY
            )",
            [],
        )
        .unwrap();

        // Create the blocked_playlists table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocked_playlists (
                id TEXT PRIMARY KEY
            )",
            [],
        )
        .unwrap();

        // Create the subscriptions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subscriptions (
                channel_id        TEXT PRIMARY KEY,
                name              TEXT NOT NULL DEFAULT '',
                thumbnail_url     TEXT NOT NULL DEFAULT '',
                last_sync         INTEGER NOT NULL DEFAULT 0,
                last_sync_channel INTEGER NOT NULL DEFAULT 0,
                has_new           INTEGER NOT NULL DEFAULT 0,
                videos_json       TEXT NOT NULL DEFAULT '[]'
            )",
            [],
        )
        .unwrap();

        // Create the subscription_tags table (many-to-many)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subscription_tags (
                channel_id TEXT NOT NULL REFERENCES subscriptions(channel_id) ON DELETE CASCADE,
                tag        TEXT NOT NULL,
                PRIMARY KEY (channel_id, tag)
            )",
            [],
        )
        .unwrap();

        unsafe {
            let _ = DATABASE.set(Self {
                conn: Mutex::new(conn),
            });
        }
    }

    // ─── Blocked channels ───────────────────────────────────────────────────────

    /// Block a channel by its ID
    pub fn block_channel(id: &str) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO blocked_channels (id) VALUES (?1)",
            params![id],
        )?;
        Ok(())
    }

    /// Unblock a channel by its ID
    pub fn unblock_channel(id: &str) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        conn.execute("DELETE FROM blocked_channels WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Check if a channel is blocked
    pub fn is_blocked(id: &str) -> Result<bool> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT 1 FROM blocked_channels WHERE id = ?1")?;
        let exists = stmt.exists(params![id])?;
        Ok(exists)
    }

    /// Retrieves all blocked channels as a HashSet for quick lookup
    pub fn get_all_blocked_channels() -> Result<HashSet<String>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM blocked_channels")?;

        let blocked = stmt.query_map([], |row| row.get(0))?;

        let mut set = HashSet::new();
        for id in blocked {
            set.insert(id?);
        }

        Ok(set)
    }

    // ─── Blocked playlists ──────────────────────────────────────────────────────

    /// Block a playlist by its ID
    pub fn block_playlist(id: &str) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO blocked_playlists (id) VALUES (?1)",
            params![id],
        )?;
        Ok(())
    }

    /// Unblock a playlist by its ID
    pub fn unblock_playlist(id: &str) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        conn.execute("DELETE FROM blocked_playlists WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Retrieves all blocked playlists as a HashSet for quick lookup
    pub fn get_all_blocked_playlists() -> Result<HashSet<String>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM blocked_playlists")?;

        let blocked = stmt.query_map([], |row| row.get(0))?;

        let mut set = HashSet::new();
        for id in blocked {
            set.insert(id?);
        }

        Ok(set)
    }

    // ─── Subscriptions ──────────────────────────────────────────────────────────

    /// Insert or update a subscription. The `tags` field on `SubItem` is NOT
    /// written here — use `tag_subscription` / `untag_subscription` for tags.
    pub fn upsert_subscription(item: &crate::global::structs::SubItem) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let videos_json = serde_json::to_string(&item.videos).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO subscriptions 
                (channel_id, name, thumbnail_url, last_sync, last_sync_channel, has_new, videos_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(channel_id) DO UPDATE SET
                name = excluded.name,
                thumbnail_url = excluded.thumbnail_url,
                last_sync = excluded.last_sync,
                last_sync_channel = excluded.last_sync_channel,
                has_new = excluded.has_new,
                videos_json = excluded.videos_json",
            params![
                item.channel.id,
                item.channel.name,
                item.channel.thumbnail_url,
                item.last_sync as i64,
                item.last_sync_channel as i64,
                item.has_new as i64,
                videos_json,
            ],
        )?;
        Ok(())
    }

    /// Remove a subscription (and all its tags via CASCADE).
    pub fn remove_subscription(channel_id: &str) -> Result<bool> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let rows = conn.execute(
            "DELETE FROM subscriptions WHERE channel_id = ?1",
            params![channel_id],
        )?;
        Ok(rows > 0)
    }

    /// Returns true if the channel is already in the subscriptions table.
    pub fn is_subscribed(channel_id: &str) -> Result<bool> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT 1 FROM subscriptions WHERE channel_id = ?1")?;
        stmt.exists(params![channel_id])
    }

    /// Load all subscriptions (with their tags) from the database.
    /// Each returned `SubItem` has `tags` populated.
    pub fn get_all_subscriptions() -> Result<Vec<crate::global::structs::SubItem>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();

        // Load base subscription rows
        let mut stmt = conn.prepare(
            "SELECT channel_id, name, thumbnail_url, last_sync, last_sync_channel, has_new, videos_json
             FROM subscriptions",
        )?;

        struct Row {
            channel_id: String,
            name: String,
            thumbnail_url: String,
            last_sync: i64,
            last_sync_channel: i64,
            has_new: bool,
            videos_json: String,
        }

        let rows: Vec<Row> = stmt
            .query_map([], |row| {
                Ok(Row {
                    channel_id: row.get(0)?,
                    name: row.get(1)?,
                    thumbnail_url: row.get(2)?,
                    last_sync: row.get(3)?,
                    last_sync_channel: row.get(4)?,
                    has_new: row.get::<_, i64>(5)? != 0,
                    videos_json: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        // Load all tags in one query
        let mut tag_stmt =
            conn.prepare("SELECT channel_id, tag FROM subscription_tags ORDER BY channel_id, tag")?;
        let pairs: Vec<(String, String)> = tag_stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>>>()?;

        // Build tag map
        let mut tag_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (cid, tag) in pairs {
            tag_map.entry(cid).or_default().push(tag);
        }

        // Assemble SubItems
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            use crate::global::structs::{FullChannelItem, MiniVideoItem, SubItem};

            let videos: Vec<MiniVideoItem> =
                serde_json::from_str(&row.videos_json).unwrap_or_default();

            let channel = FullChannelItem {
                id: row.channel_id.clone(),
                name: row.name,
                thumbnail_url: row.thumbnail_url,
                sub_count: 0,
                sub_count_text: String::new(),
                total_views: String::new(),
                created: String::new(),
                autogenerated: false,
                description: String::new(),
            };

            let tags = tag_map.remove(&row.channel_id).unwrap_or_default();

            items.push(SubItem {
                channel,
                videos,
                last_sync: row.last_sync as u64,
                last_sync_channel: row.last_sync_channel as u64,
                has_new: row.has_new,
                tags,
            });
        }

        Ok(items)
    }

    // ─── Tags ───────────────────────────────────────────────────────────────────

    /// Add a tag to an existing subscription. Returns an error if the channel
    /// is not subscribed.
    pub fn tag_subscription(channel_id: &str, tag: &str) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO subscription_tags (channel_id, tag) VALUES (?1, ?2)",
            params![channel_id, tag],
        )?;
        Ok(())
    }

    /// Remove a tag from a subscription.
    pub fn untag_subscription(channel_id: &str, tag: &str) -> Result<bool> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let rows = conn.execute(
            "DELETE FROM subscription_tags WHERE channel_id = ?1 AND tag = ?2",
            params![channel_id, tag],
        )?;
        Ok(rows > 0)
    }

    /// Get all tags for a subscription.
    pub fn get_tags_for(channel_id: &str) -> Result<Vec<String>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT tag FROM subscription_tags WHERE channel_id = ?1 ORDER BY tag")?;
        let tags = stmt
            .query_map(params![channel_id], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?;
        Ok(tags)
    }

    /// Get all subscriptions that have a specific tag.
    pub fn get_subscriptions_by_tag(tag: &str) -> Result<Vec<crate::global::structs::SubItem>> {
        let all = Self::get_all_subscriptions()?;
        Ok(all
            .into_iter()
            .filter(|item| item.tags.iter().any(|t| t == tag))
            .collect())
    }
}
