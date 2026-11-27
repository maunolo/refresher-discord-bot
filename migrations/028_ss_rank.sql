-- SS rank: above S rank. SS-rank players captain their team, two a team of a match at most.
ALTER TABLE camp_players ADD COLUMN is_ss_rank INTEGER NOT NULL DEFAULT 0;
