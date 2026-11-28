-- How loud each stream's pause and replay overlay plays its sound (crates/core/src/volume.rs), in
-- percent. A stream without a row plays at 100.
CREATE TABLE overlay_volumes (
    guild_id INTEGER NOT NULL,
    stream INTEGER NOT NULL,
    volume INTEGER NOT NULL,
    PRIMARY KEY (guild_id, stream)
);
