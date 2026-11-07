-- The secret in a server's OBS overlay links. Overlays are read-only pages that need no login,
-- so whoever has a link sees what they show; admins can replace the secret to cut old links off.
CREATE TABLE overlay_links (
    guild_id INTEGER PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);
