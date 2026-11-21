-- Set when an admin takes a player out of a running camp: out of the queue and new matches,
-- their points and matches kept. Their location is 'removed' once they're out of any match.
ALTER TABLE camp_players ADD COLUMN removed_at INTEGER;
