-- Enable WAL and foreign keys (run as part of migration bootstrap)
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS blocked_channels (
    id TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS blocked_playlists (
    id TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS subscriptions (
    channel_id        TEXT NOT NULL PRIMARY KEY,
    name              TEXT NOT NULL DEFAULT '',
    thumbnail_url     TEXT NOT NULL DEFAULT '',
    last_sync         INTEGER NOT NULL DEFAULT 0,
    last_sync_channel INTEGER NOT NULL DEFAULT 0,
    has_new           INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS videos (
    id            TEXT NOT NULL PRIMARY KEY,
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

CREATE INDEX IF NOT EXISTS idx_videos_channel   ON videos(channel_id);
CREATE INDEX IF NOT EXISTS idx_videos_timestamp ON videos(timestamp DESC);

CREATE TABLE IF NOT EXISTS subscription_tags (
    channel_id TEXT NOT NULL REFERENCES subscriptions(channel_id) ON DELETE CASCADE,
    tag        TEXT NOT NULL,
    PRIMARY KEY (channel_id, tag)
);

CREATE TABLE IF NOT EXISTS search_history (
    query      TEXT NOT NULL PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
