-- Replay clips (crates/core/src/clips.rs): videos in the R2 bucket, listed once uploaded.
CREATE TABLE clips (
    id TEXT PRIMARY KEY,
    guild_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    object_key TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    duration_ms INTEGER,
    ready INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_clips_guild ON clips (guild_id, created_at);

-- The clip each stream's replay overlay was last told to play.
CREATE TABLE clip_plays (
    guild_id INTEGER NOT NULL,
    stream INTEGER NOT NULL,
    play_id TEXT NOT NULL,
    clip_id TEXT NOT NULL,
    started_at_ms INTEGER NOT NULL,
    PRIMARY KEY (guild_id, stream)
);
