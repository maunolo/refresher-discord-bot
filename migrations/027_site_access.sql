-- Whether admins opened the public site of a server to everyone, while `SITE_LOCK` would
-- otherwise ask for a login. Locked until an admin unlocks it.
CREATE TABLE site_access (
    guild_id INTEGER PRIMARY KEY,
    unlocked INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);
