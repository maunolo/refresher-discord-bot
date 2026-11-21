-- The queue, as comma-separated user IDs in order, right before the match took its players:
-- cancelling the match puts it back.
ALTER TABLE camp_matches ADD COLUMN queue_before TEXT;
