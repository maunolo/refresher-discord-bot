-- DeadChaps drafts have one invite link for both captains. The per-team and organiser links
-- of draft.deadlock.pro.br drafts stay in their old columns, unused.
ALTER TABLE camp_matches ADD COLUMN draft_join_url TEXT;
