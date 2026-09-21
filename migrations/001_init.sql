CREATE TABLE camps (
    id TEXT PRIMARY KEY,
    guild_id INTEGER NOT NULL,
    channel_id INTEGER NOT NULL,
    board_message_id INTEGER,
    target_size INTEGER NOT NULL,
    status TEXT NOT NULL,
    match_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX idx_camps_guild_status ON camps (guild_id, status);

CREATE TABLE camp_players (
    camp_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    points INTEGER NOT NULL DEFAULT 0,
    location TEXT NOT NULL DEFAULT 'queued',
    queue_position INTEGER,
    consecutive_matches INTEGER NOT NULL DEFAULT 0,
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (camp_id, user_id),
    FOREIGN KEY (camp_id) REFERENCES camps (id)
);

CREATE TABLE camp_matches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    camp_id TEXT NOT NULL,
    slot INTEGER NOT NULL,
    status TEXT NOT NULL,
    refresh_used INTEGER NOT NULL DEFAULT 0,
    message_id INTEGER,
    party_id TEXT,
    party_code TEXT,
    winner_side TEXT,
    completed_at INTEGER,
    FOREIGN KEY (camp_id) REFERENCES camps (id)
);

CREATE TABLE camp_match_players (
    match_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    side TEXT NOT NULL,
    points_after INTEGER,
    PRIMARY KEY (match_id, user_id),
    FOREIGN KEY (match_id) REFERENCES camp_matches (id)
);

CREATE TABLE camp_refresh_votes (
    match_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    PRIMARY KEY (match_id, user_id),
    FOREIGN KEY (match_id) REFERENCES camp_matches (id)
);
