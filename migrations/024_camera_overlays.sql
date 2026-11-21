-- Each server's commentator cameras for the stream overlays (crates/core/src/cameras.rs), as JSON:
-- the VDO.Ninja room, its sources, and which ones each of the two streams shows.
CREATE TABLE camera_overlays (
    guild_id INTEGER PRIMARY KEY,
    config TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
