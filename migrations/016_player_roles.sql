-- A player's main role on a server (tank, carry or support), kept from camp to camp. Splitting a
-- match into teams tries to give each team one of each.
CREATE TABLE guild_player_roles (
    guild_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    role TEXT NOT NULL,
    PRIMARY KEY (guild_id, user_id)
);
