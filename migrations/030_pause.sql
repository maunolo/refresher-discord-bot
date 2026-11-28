-- The pause screen each stream's replay overlay shows (crates/core/src/pause.rs): a countdown
-- to `ends_at_ms` (Unix milliseconds) under a message, until an admin takes it off.
CREATE TABLE pause_screens (
    guild_id INTEGER NOT NULL,
    stream INTEGER NOT NULL,
    message TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    ends_at_ms INTEGER NOT NULL,
    PRIMARY KEY (guild_id, stream)
);
