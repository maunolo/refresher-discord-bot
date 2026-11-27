-- Selections of players saved per server before any camp: who plays, their category and role.
-- Picking one fills a camp in signup.
CREATE TABLE guild_presets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX guild_presets_guild ON guild_presets (guild_id);

-- rank: 's', 'ss' or NULL. role: 'tank', 'carry', 'support' or NULL.
CREATE TABLE guild_preset_players (
    preset_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    rank TEXT,
    role TEXT,
    position INTEGER NOT NULL,
    PRIMARY KEY (preset_id, user_id)
);
