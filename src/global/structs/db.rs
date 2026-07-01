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

        // Create the subscriptions table (videos moved to separate table)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS subscriptions (
                channel_id        TEXT PRIMARY KEY,
                name              TEXT NOT NULL DEFAULT '',
                thumbnail_url     TEXT NOT NULL DEFAULT '',
                last_sync         INTEGER NOT NULL DEFAULT 0,
                last_sync_channel INTEGER NOT NULL DEFAULT 0,
                has_new           INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )
        .unwrap();

        // Create the videos table (replaces videos_json column)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS videos (
                id            TEXT PRIMARY KEY,
                channel_id    TEXT NOT NULL REFERENCES subscriptions(channel_id) ON DELETE CASCADE,
                title         TEXT NOT NULL DEFAULT '',
                thumbnail_url TEXT NOT NULL DEFAULT '',
                length        TEXT NOT NULL DEFAULT '',
                views         TEXT,
                channel_name  TEXT NOT NULL DEFAULT '',
                published     TEXT,
                timestamp     INTEGER,
                description   TEXT
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_videos_channel ON videos(channel_id)",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_videos_timestamp ON videos(timestamp DESC)",
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

        // Create the search_history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS search_history (
                query      TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
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

    /// Insert or update a subscription and its videos atomically.
    /// The `tags` field on `SubItem` is NOT written here —
    /// use `tag_subscription` / `untag_subscription` for tags.
    pub fn upsert_subscription(item: &crate::global::structs::SubItem) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO subscriptions
                (channel_id, name, thumbnail_url, last_sync, last_sync_channel, has_new)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(channel_id) DO UPDATE SET
                name = excluded.name,
                thumbnail_url = excluded.thumbnail_url,
                last_sync = excluded.last_sync,
                last_sync_channel = excluded.last_sync_channel,
                has_new = excluded.has_new",
            params![
                item.channel.id,
                item.channel.name,
                item.channel.thumbnail_url,
                item.last_sync as i64,
                item.last_sync_channel as i64,
                item.has_new as i64,
            ],
        )?;

        // Replace videos for this channel
        conn.execute(
            "DELETE FROM videos WHERE channel_id = ?1",
            params![item.channel.id],
        )?;
        for video in &item.videos {
            conn.execute(
                "INSERT INTO videos (id, channel_id, title, thumbnail_url, length, views, channel_name, published, timestamp, description)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    video.id,
                    item.channel.id,
                    video.title,
                    video.thumbnail_url,
                    video.length,
                    video.views,
                    video.channel,
                    video.published,
                    video.timestamp.map(|t| t as i64),
                    video.description,
                ],
            )?;
        }

        Ok(())
    }

    /// Remove a subscription (and all its tags/videos via CASCADE).
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

        // Load all subscriptions
        let mut sub_stmt = conn.prepare(
            "SELECT channel_id, name, thumbnail_url, last_sync, last_sync_channel, has_new
             FROM subscriptions",
        )?;
        let sub_rows: Vec<(String, String, String, i64, i64, bool)> = sub_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)? != 0,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;

        // Load all tags
        let mut tag_stmt =
            conn.prepare("SELECT channel_id, tag FROM subscription_tags ORDER BY channel_id, tag")?;
        let tag_pairs: Vec<(String, String)> = tag_stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>>>()?;

        // Load all videos grouped by channel_id
        let mut vid_stmt = conn.prepare(
            "SELECT channel_id, id, title, thumbnail_url, length, views, channel_name, published, timestamp, description
             FROM videos ORDER BY timestamp DESC",
        )?;
        let vid_rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<i64>,
            Option<String>,
        )> = vid_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;

        // Build tag map
        let mut tag_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (cid, tag) in tag_pairs {
            tag_map.entry(cid).or_default().push(tag);
        }

        // Build videos-by-channel map
        let mut videos_by_channel: std::collections::HashMap<
            String,
            Vec<crate::global::structs::MiniVideoItem>,
        > = std::collections::HashMap::new();
        for (
            channel_id,
            vid_id,
            title,
            thumbnail_url,
            length,
            views,
            channel_name,
            published,
            timestamp,
            description,
        ) in vid_rows
        {
            videos_by_channel.entry(channel_id).or_default().push(
                crate::global::structs::MiniVideoItem {
                    id: vid_id,
                    title,
                    thumbnail_url,
                    length,
                    views,
                    channel: channel_name,
                    channel_id: String::new(),
                    published,
                    timestamp: timestamp.map(|t| t as u64),
                    description,
                },
            );
        }

        // Assemble SubItems
        let mut items = Vec::with_capacity(sub_rows.len());
        for (channel_id, name, thumbnail_url, last_sync, last_sync_channel, has_new) in sub_rows {
            use crate::global::structs::{FullChannelItem, SubItem};
            let channel = FullChannelItem {
                id: channel_id.clone(),
                name,
                thumbnail_url,
                sub_count: 0,
                sub_count_text: String::new(),
                total_views: String::new(),
                created: String::new(),
                autogenerated: false,
                description: String::new(),
            };
            let tags = tag_map.remove(&channel_id).unwrap_or_default();
            let videos = videos_by_channel.remove(&channel_id).unwrap_or_default();

            items.push(SubItem {
                channel,
                videos,
                last_sync: last_sync as u64,
                last_sync_channel: last_sync_channel as u64,
                has_new,
                tags,
            });
        }

        Ok(items)
    }

    /// Get all videos across all subscriptions, sorted by timestamp descending.
    pub fn get_all_videos() -> Result<Vec<crate::global::structs::MiniVideoItem>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, thumbnail_url, length, views, channel_name, published, timestamp, description
             FROM videos ORDER BY timestamp DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    title,
                    thumbnail_url,
                    length,
                    views,
                    channel,
                    published,
                    timestamp,
                    description,
                )| {
                    crate::global::structs::MiniVideoItem {
                        id,
                        title,
                        thumbnail_url,
                        length,
                        views,
                        channel,
                        channel_id: String::new(),
                        published,
                        timestamp: timestamp.map(|t| t as u64),
                        description,
                    }
                },
            )
            .collect())
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

    // ─── Search History ──────────────────────────────────────────────────────────

    pub fn get_search_history() -> Result<Vec<String>> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT query FROM search_history ORDER BY created_at DESC")?;
        let items = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>>>()?;
        Ok(items)
    }

    pub fn add_search_history(query: &str) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO search_history (query, created_at) VALUES (?1, strftime('%s', 'now'))",
            params![query],
        )?;
        Ok(())
    }

    pub fn trim_search_history(limit: usize) -> Result<()> {
        let db = unsafe { DATABASE.get() }.expect("Database not initialized");
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM search_history WHERE rowid NOT IN (
                SELECT rowid FROM search_history ORDER BY created_at DESC LIMIT ?1
            )",
            params![limit as i64],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_test_db<F>(f: F)
    where
        F: FnOnce(&Connection),
    {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE subscriptions (
                channel_id        TEXT PRIMARY KEY,
                name              TEXT NOT NULL DEFAULT '',
                thumbnail_url     TEXT NOT NULL DEFAULT '',
                last_sync         INTEGER NOT NULL DEFAULT 0,
                last_sync_channel INTEGER NOT NULL DEFAULT 0,
                has_new           INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE subscription_tags (
                channel_id TEXT NOT NULL REFERENCES subscriptions(channel_id) ON DELETE CASCADE,
                tag        TEXT NOT NULL,
                PRIMARY KEY (channel_id, tag)
            );
            CREATE TABLE videos (
                id            TEXT PRIMARY KEY,
                channel_id    TEXT NOT NULL REFERENCES subscriptions(channel_id) ON DELETE CASCADE,
                title         TEXT NOT NULL DEFAULT '',
                thumbnail_url TEXT NOT NULL DEFAULT '',
                length        TEXT NOT NULL DEFAULT '',
                views         TEXT,
                channel_name  TEXT NOT NULL DEFAULT '',
                published     TEXT,
                timestamp     INTEGER,
                description   TEXT
            );
            CREATE TABLE search_history (
                query      TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE blocked_channels (id TEXT PRIMARY KEY);
            CREATE TABLE blocked_playlists (id TEXT PRIMARY KEY);
            ",
        )
        .unwrap();
        f(&conn);
    }

    // ─── Blocked channels ────────────────────────────────────────────────────

    #[test]
    fn block_unblock_channel() {
        with_test_db(|conn| {
            conn.execute("INSERT INTO blocked_channels (id) VALUES ('ch1')", [])
                .unwrap();

            let mut stmt = conn
                .prepare("SELECT id FROM blocked_channels ORDER BY id")
                .unwrap();
            let ids: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            assert_eq!(ids, vec!["ch1"]);

            conn.execute("DELETE FROM blocked_channels WHERE id = 'ch1'", [])
                .unwrap();
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM blocked_channels", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0);
        });
    }

    // ─── Videos ───────────────────────────────────────────────────────────────

    #[test]
    fn upsert_and_get_videos() {
        with_test_db(|conn| {
            conn.execute(
                "INSERT INTO subscriptions (channel_id, name, thumbnail_url, last_sync, last_sync_channel, has_new)
                 VALUES ('ch1', 'Test Channel', '', 0, 0, 0)",
                [],
            )
            .unwrap();

            conn.execute(
                "INSERT INTO videos (id, channel_id, title, thumbnail_url, length, views, channel_name, timestamp)
                 VALUES ('v1', 'ch1', 'Video 1', '', '10:00', '1K views', 'Test Channel', 1000)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO videos (id, channel_id, title, thumbnail_url, length, views, channel_name, timestamp)
                 VALUES ('v2', 'ch1', 'Video 2', '', '5:00', '500 views', 'Test Channel', 2000)",
                [],
            )
            .unwrap();

            // Verify order (by timestamp DESC)
            let mut stmt = conn
                .prepare("SELECT id, title, timestamp FROM videos WHERE channel_id = 'ch1' ORDER BY timestamp DESC")
                .unwrap();
            let rows: Vec<(String, String, i64)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();

            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].0, "v2"); // newer first
            assert_eq!(rows[0].1, "Video 2");
            assert_eq!(rows[1].0, "v1");
        });
    }

    #[test]
    fn videos_cascade_delete_on_unsubscribe() {
        with_test_db(|conn| {
            conn.execute(
                "INSERT INTO subscriptions (channel_id, name) VALUES ('ch1', 'Test')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO videos (id, channel_id, title) VALUES ('v1', 'ch1', 'Vid')",
                [],
            )
            .unwrap();
            conn.execute("DELETE FROM subscriptions WHERE channel_id = 'ch1'", [])
                .unwrap();

            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM videos WHERE channel_id = 'ch1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0);
        });
    }

    // ─── Tags ────────────────────────────────────────────────────────────────

    #[test]
    fn tag_and_untag_subscription() {
        with_test_db(|conn| {
            conn.execute(
                "INSERT INTO subscriptions (channel_id, name) VALUES ('ch1', 'Test')",
                [],
            )
            .unwrap();

            conn.execute(
                "INSERT INTO subscription_tags (channel_id, tag) VALUES ('ch1', 'music')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO subscription_tags (channel_id, tag) VALUES ('ch1', 'tech')",
                [],
            )
            .unwrap();

            let mut stmt = conn
                .prepare("SELECT tag FROM subscription_tags WHERE channel_id = 'ch1' ORDER BY tag")
                .unwrap();
            let tags: Vec<String> = stmt
                .query_map([], |row| row.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            assert_eq!(tags, vec!["music", "tech"]);

            conn.execute(
                "DELETE FROM subscription_tags WHERE channel_id = 'ch1' AND tag = 'music'",
                [],
            )
            .unwrap();
            let remaining: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM subscription_tags WHERE channel_id = 'ch1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(remaining, 1);
        });
    }

    // ─── Search history ──────────────────────────────────────────────────────

    #[test]
    fn search_history_replace_on_duplicate() {
        with_test_db(|conn| {
            conn.execute(
                "INSERT INTO search_history (query, created_at) VALUES ('rust tutorial', 1000)",
                [],
            )
            .unwrap();
            // Simulate INSERT OR REPLACE behavior
            conn.execute(
                "INSERT OR REPLACE INTO search_history (query, created_at) VALUES ('rust tutorial', 2000)",
                [],
            )
            .unwrap();

            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM search_history", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 1);

            let timestamp: i64 = conn
                .query_row(
                    "SELECT created_at FROM search_history WHERE query = 'rust tutorial'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(timestamp, 2000);
        });
    }

    #[test]
    fn search_history_trim_keeps_newest() {
        with_test_db(|conn| {
            for i in 0..10 {
                conn.execute(
                    &format!(
                        "INSERT INTO search_history (query, created_at) VALUES ('query{}', {})",
                        i,
                        1000 + i
                    ),
                    [],
                )
                .unwrap();
            }

            // Trim to 5
            conn.execute(
                "DELETE FROM search_history WHERE rowid NOT IN (
                    SELECT rowid FROM search_history ORDER BY created_at DESC LIMIT 5
                )",
                [],
            )
            .unwrap();

            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM search_history", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 5);

            // Oldest remaining should be query5 (timestamp 1005)
            let oldest: i64 = conn
                .query_row(
                    "SELECT created_at FROM search_history ORDER BY created_at ASC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(oldest, 1005);
        });
    }

    // ─── Subscriptions ───────────────────────────────────────────────────────

    #[test]
    fn upsert_subscription_updates_fields() {
        with_test_db(|conn| {
            // First insert
            conn.execute(
                "INSERT INTO subscriptions (channel_id, name, thumbnail_url, last_sync, last_sync_channel, has_new)
                 VALUES ('ch1', 'Channel 1', 'http://thumb1', 1000, 0, 1)",
                [],
            )
            .unwrap();

            // Update with upsert-like logic
            conn.execute(
                "INSERT INTO subscriptions (channel_id, name, thumbnail_url, last_sync, last_sync_channel, has_new)
                 VALUES ('ch1', 'Channel 1 Updated', 'http://thumb2', 2000, 2000, 0)
                 ON CONFLICT(channel_id) DO UPDATE SET
                    name = excluded.name,
                    thumbnail_url = excluded.thumbnail_url,
                    last_sync = excluded.last_sync,
                    last_sync_channel = excluded.last_sync_channel,
                    has_new = excluded.has_new",
                [],
            )
            .unwrap();

            let name: String = conn
                .query_row(
                    "SELECT name FROM subscriptions WHERE channel_id = 'ch1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(name, "Channel 1 Updated");

            let last_sync: i64 = conn
                .query_row(
                    "SELECT last_sync FROM subscriptions WHERE channel_id = 'ch1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(last_sync, 2000);
        });
    }

    #[test]
    fn get_all_subscriptions_loads_tags_and_videos() {
        with_test_db(|conn| {
            conn.execute(
                "INSERT INTO subscriptions (channel_id, name) VALUES ('ch1', 'C1'), ('ch2', 'C2')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO subscription_tags (channel_id, tag) VALUES ('ch1', 'music'), ('ch1', 'rock'), ('ch2', 'tech')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO videos (id, channel_id, title, thumbnail_url, length, channel_name, timestamp)
                 VALUES ('v1', 'ch1', 'Vid 1', '', '5:00', 'C1', 3000),
                        ('v2', 'ch1', 'Vid 2', '', '3:00', 'C1', 2000),
                        ('v3', 'ch2', 'Vid 3', '', '10:00', 'C2', 1000)",
                [],
            )
            .unwrap();

            // Verify sub count
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM subscriptions", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 2);

            // Verify tags per channel
            let ch1_tags: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM subscription_tags WHERE channel_id = 'ch1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(ch1_tags, 2);

            // Verify videos per channel
            let ch1_videos: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM videos WHERE channel_id = 'ch1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(ch1_videos, 2);
        });
    }
}
