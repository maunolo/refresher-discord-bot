-- Each finished match's scoreboard from the Deadlock API, for the public site's match history.
-- `status`: `pending` (the bot is looking it up), `needs_id` (an admin has to give the Deadlock
-- match ID), `ready` (`data` holds the scoreboard) or `failed` (the API never had it; an admin
-- can try again).
CREATE TABLE match_stats (
    match_id TEXT PRIMARY KEY,
    deadlock_match_id INTEGER,
    status TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at INTEGER,
    last_error TEXT,
    data TEXT,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (match_id) REFERENCES camp_matches (id)
);
CREATE INDEX idx_match_stats_due ON match_stats (status, next_attempt_at);

-- Which server member plays on which Steam account, so scoreboards show Discord names.
CREATE TABLE deadlock_accounts (
    guild_id INTEGER NOT NULL,
    account_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (guild_id, account_id)
);

-- The site's ranking and matches now come from the published camp itself.
ALTER TABLE site_content DROP COLUMN players;
ALTER TABLE site_content DROP COLUMN matches;
