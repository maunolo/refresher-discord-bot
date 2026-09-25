CREATE TABLE camp_matches_new (
    id TEXT PRIMARY KEY,
    camp_id TEXT NOT NULL,
    slot INTEGER NOT NULL,
    status TEXT NOT NULL,
    refresh_used INTEGER NOT NULL DEFAULT 0,
    message_id INTEGER,
    party_id TEXT,
    party_code TEXT,
    winner_side TEXT,
    completed_at INTEGER,
    is_final INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (camp_id) REFERENCES camps (id)
);

INSERT INTO camp_matches_new (
    id, camp_id, slot, status, refresh_used, message_id, party_id, party_code,
    winner_side, completed_at, is_final
)
SELECT CAST(id AS TEXT), camp_id, slot, status, refresh_used, message_id, party_id, party_code,
       winner_side, completed_at, is_final
FROM camp_matches;

CREATE TABLE camp_match_players_new (
    match_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    side TEXT NOT NULL,
    points_after INTEGER,
    PRIMARY KEY (match_id, user_id),
    FOREIGN KEY (match_id) REFERENCES camp_matches_new (id)
);

INSERT INTO camp_match_players_new (match_id, user_id, side, points_after)
SELECT CAST(match_id AS TEXT), user_id, side, points_after
FROM camp_match_players;

CREATE TABLE camp_refresh_votes_new (
    match_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    choice TEXT NOT NULL DEFAULT 'refresh',
    PRIMARY KEY (match_id, user_id),
    FOREIGN KEY (match_id) REFERENCES camp_matches_new (id)
);

INSERT INTO camp_refresh_votes_new (match_id, user_id, choice)
SELECT CAST(match_id AS TEXT), user_id, choice
FROM camp_refresh_votes;

DROP TABLE camp_refresh_votes;
DROP TABLE camp_match_players;
DROP TABLE camp_matches;

ALTER TABLE camp_matches_new RENAME TO camp_matches;
ALTER TABLE camp_match_players_new RENAME TO camp_match_players;
ALTER TABLE camp_refresh_votes_new RENAME TO camp_refresh_votes;
