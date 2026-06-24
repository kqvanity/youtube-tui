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

        unsafe {
            let _ = DATABASE.set(Self {
                conn: Mutex::new(conn),
            });
        }
    }

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
}
