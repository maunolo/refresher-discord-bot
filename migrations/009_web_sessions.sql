CREATE TABLE web_sessions (
    token_hash TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    display_name TEXT NOT NULL,
    avatar TEXT,
    admin_guilds TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX idx_web_sessions_expires ON web_sessions (expires_at);
