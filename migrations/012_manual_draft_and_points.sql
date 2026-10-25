ALTER TABLE guild_settings ADD COLUMN use_draft_tool INTEGER NOT NULL DEFAULT 1;
ALTER TABLE camp_matches ADD COLUMN draft_manual INTEGER NOT NULL DEFAULT 0;
CREATE TABLE camp_point_adjustments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    camp_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    delta INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_camp_point_adjustments_camp ON camp_point_adjustments (camp_id);
