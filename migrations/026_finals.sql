-- The camp's finals: once an admin enters them, every new match is played by the top of the
-- ranking instead of the front of the queue. Camps that already made their one final are in them.
ALTER TABLE camps ADD COLUMN finals INTEGER NOT NULL DEFAULT 0;
UPDATE camps SET finals = 1 WHERE id IN (SELECT camp_id FROM camp_matches WHERE is_final != 0);
