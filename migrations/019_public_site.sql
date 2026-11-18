-- The public league site (site/): what staff type in the dashboard for a camp, as JSON in the
-- shapes the site's pages read (window.INFO, window.JOGADORES, window.PARTIDAS).
CREATE TABLE site_content (
    camp_id TEXT PRIMARY KEY,
    info TEXT NOT NULL,
    players TEXT NOT NULL,
    matches TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (camp_id) REFERENCES camps (id)
);

-- The camp a server shows on the public site.
CREATE TABLE site_published (
    guild_id INTEGER PRIMARY KEY,
    camp_id TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
