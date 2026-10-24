ALTER TABLE camps ADD COLUMN match_size INTEGER NOT NULL DEFAULT 12;
UPDATE camps SET match_size = 4 WHERE target_size = 4;
UPDATE camps SET match_size = 12 WHERE target_size != 4;
