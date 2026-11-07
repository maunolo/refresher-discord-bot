-- More of DeadChaps' draft options per server, and whether captains get the draft link by DM.
-- The defaults are what drafts used before: DeadChaps' 90-second reserve, 2 bans, its cinematic
-- view, and the DM.
ALTER TABLE guild_settings ADD COLUMN draft_reserve_seconds INTEGER NOT NULL DEFAULT 90;
ALTER TABLE guild_settings ADD COLUMN draft_bans_per_team INTEGER NOT NULL DEFAULT 2;
ALTER TABLE guild_settings ADD COLUMN draft_view TEXT NOT NULL DEFAULT 'cinematic';
ALTER TABLE guild_settings ADD COLUMN dm_captains INTEGER NOT NULL DEFAULT 1;
