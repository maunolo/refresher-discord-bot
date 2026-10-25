CREATE TABLE guild_settings (
    guild_id INTEGER PRIMARY KEY,
    queue_voice_id INTEGER,
    match1_hiddenking_voice_id INTEGER,
    match1_archmother_voice_id INTEGER,
    match2_hiddenking_voice_id INTEGER,
    match2_archmother_voice_id INTEGER,
    create_lobby INTEGER NOT NULL DEFAULT 0,
    server_region TEXT,
    draft_timer_seconds INTEGER NOT NULL DEFAULT 30,
    updated_at INTEGER NOT NULL
);
