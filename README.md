# Deadlock camp bot

Runs a **24- or 30-player Deadlock camp** as a queue: two concurrent 6v6 custom matches, a one-time team refresh vote, S rank captains, pick/ban drafts, SQLite persistence and a points leaderboard. There are also **4-player test** sizes: **4-1v1** (two matches at once) and **4-2v2**.

Admins run the camp from a **web admin page** (sign in with Discord). In Discord, players see the queue and their matches, vote on refreshing the teams, and captains get their draft links. Heroes are picked in Deadlock.

## Setup

1. Create an application and bot in the [Discord Developer Portal](https://discord.com/developers/applications). No privileged intents are needed.
2. Invite the bot with: View Channels, Send Messages, Embed Links, Connect, Move Members, and Manage Channels if you want it to create the voice channels.
3. In the application's **OAuth2** page, copy the **Client ID**, reset and copy the **Client Secret**, and add the redirect `http://localhost:8080/auth/callback` (or `{WEB_PUBLIC_URL}/auth/callback` if you change the address).
4. Copy `.env.example` to `.env` and set `DISCORD_TOKEN`, `DISCORD_CLIENT_ID` and `DISCORD_CLIENT_SECRET`. Leave `ENABLE_CUSTOM_MATCH_CREATE=false` until you need a real Deadlock lobby (then the IP limit is **10 creates per hour** without a key); `DEADLOCK_API_KEY` is optional.
5. Build the admin page once (needs [Node.js](https://nodejs.org) 20 or newer): `cd web`, `npm install`, `npm run build`. The server serves `web/dist` (change it with `WEB_STATIC_DIR`).
6. Install Rust (`https://rustup.rs`), then from the repo root:

```bash
cargo run
```

Open `http://localhost:8080` and sign in. The first start creates `data/camps.db`.

## Admin page

Anyone with **Manage Server** in a server the bot is in can sign in and run that server's camps. A sign-in lasts 12 hours; sign in again to pick up permission changes.

- **Camps:** every camp of the server. **Novo camp** opens signup in a text channel you pick, with a size (24, 30, 4-1v1, 4-2v2); a server has one active camp at a time. **Cancelar** keeps the roster, queue and points so the camp can be resumed with **Retomar**. **Encerrar** posts the final ranking and can't be undone.
- **Jogadores:** during signup, search server members by name, add everyone in the Queue voice channel at once, mark S rank, remove players, and **Iniciar fila** once the roster is exactly full. After that, the roster with points, consecutive matches and where each player is. S rank can still change; it applies to matches created afterwards.
- **Ao vivo:** both match slots, the queue (→ marks who the next match takes), the last results and the ranking, refreshed every few seconds. **Criar partida**, **Partida final**, **Encerrar votação e manter times**, **Iniciar draft**, **Iniciar partida**, the winner buttons, and **Trocar vencedor** / **Reabrir** on the latest result. Buttons that can't be used yet say why.

Every action also does its Discord side: the camp and match messages are redrawn, the channel gets an announcement, players are moved between voice channels, and captains get their draft DMs. Actions in the same server run one at a time, so two admins can't step on each other.

## In Discord

The camp's text channel gets:

- **The queue message:** the roster during signup, then the queue in order with who goes next, and the open matches. No buttons.
- **One message per match:** teams with captains (👑), the refresh vote count, the spectator draft link and the party code. Its buttons are only the players': **Refresh** / **Manter times** while the vote is open, and **👑 Link do capitão** once the draft exists.
- Announcements when a match is created, the vote ends, the draft starts, a match goes live, a winner is reported or corrected, and the final ranking.

There are no slash commands. On startup the bot clears the old `/camp` command from each server and strips the admin buttons from existing camp messages; any old admin button still clicked answers with a link to the admin page.

## Match flow

1. Admin fills the roster to exactly 24, 30, or 4 (test), then **Iniciar fila**. The roster is shuffled and posted; nobody is placed into a match yet.
2. **Criar partida** pulls the next group from the queue (12, 4 in a 2v2 test, or 2 in a 1v1 test) and sorts Hiddenking / Archmother. The first two queued S rank players are always pulled in, one per team (see below). A second concurrent match can be created while a slot is free. With 30 players, leftover players stay in queue until the next match.
3. Players in a match each vote once in Discord: **Refresh** or **Manter times**. If at least 4 players from either side vote refresh (2 in a 2v2 test, 1 in a 1v1 test), teams are re-sorted, still keeping S rank players apart. When everyone has voted, or a refresh can no longer pass, voting ends. An admin can also end it early with **Encerrar votação e manter times** (for when someone can't vote); the teams stay as they are. Each match only votes once.
4. **Iniciar draft** creates a pick/ban draft on [draft.deadlock.pro.br](https://draft.deadlock.pro.br/docs) (no API key needed), DMs each captain their own team's private link, posts the spectator link and moves the players into the team voice channels.
5. **Iniciar partida** marks the match live. With `ENABLE_CUSTOM_MATCH_CREATE=true`, it also creates a Deadlock custom lobby and posts the party code.
6. Admin reports the winner. Winners get +1 point. Players re-enter the queue by consecutive-match count (winners before losers in the same count; blocks with more than two players are shuffled). Anyone who sat out has their consecutive count reset to 0. The latest result can be corrected (**Trocar vencedor**) or reopened while its players are still in the queue.
7. When no match is open and at least one has been scored, **Partida final** seats the highest-point players (same count as a normal match; ties for the last seats are shuffled) and balances the teams by points. S rank players are not guaranteed a final seat, but any who make it are kept apart. After a final is created, no other match can be.
8. **Encerrar** on the Camps screen posts gold / silver / bronze for the top distinct point totals.

### S rank players and captains

- Each new match pulls the first two S rank players in queue order, one per team. Other S rank players keep their place in line. Only when there aren't enough regular players to fill a match does a third one get in; the teams then split 2/1.
- Every team has a captain (👑): its S rank player, or, if it has none, its highest-points player (ties are random). Captains are chosen when teams are made and again after a refresh.
- **Iniciar draft** DMs each captain the private link for their own team. Whoever opens a team link picks and bans for that team, so team links are never posted in the channel. If a DM fails, the bot mentions that captain, who clicks **Link do capitão** to see it privately. Admins can copy every link from the match card on the admin page.
- Each team gets 2 bans and one pick per player, with a 30-second timer per turn. To run your own copy of the [draft tool](https://github.com/deadlockprobr/draft), set `DRAFT_BASE_URL`.
- Aim for two S rank players per concurrent match (four in a 24- or 30-player camp). With only two, they play nearly every match.

### Testing with four accounts

On **Camps**, create a **4-1v1** camp (two matches at once) or **4-2v2** (one match). Add your four Discord accounts on **Jogadores**, then **Iniciar fila**.

- **4-1v1:** **Criar partida** twice for two 1v1s. Refresh needs **1 vote from either side**.
- **4-2v2:** **Criar partida** once. Refresh needs **2 votes from either team**.

When the test matches are done, **Partida final** seats the two (1v1) or four (2v2) highest-point players. Keep `ENABLE_CUSTOM_MATCH_CREATE=false` so starting a match does not call the Deadlock API.

## Voice channels

The `*_VOICE_CHANNEL_ID` settings are voice channels only, used to move players who are already in voice. Leave them blank and the bot looks up (or creates) channels named `Queue`, `Match 1 Hiddenking`, `Match 1 Archmother`, `Match 2 Hiddenking` and `Match 2 Archmother`.

Players are moved into their team channels when the draft starts; anyone who joins voice later (Queue is fine) is moved automatically. After a winner they return to Queue. Creating a match and refreshing teams move no one.

## Persistence

Every queue change, vote, draft, lobby start and result is written to SQLite in a transaction. Restarting the bot reloads signup and running camps, including who is queued and who is in each match. If a Discord message was deleted, the bot posts it again without resetting anything.

## Honest in-game gap

The Deadlock API cannot seat Discord users onto Hiddenking / Archmother. After the code is posted, players join the Steam lobby and pick heroes themselves.

## Development

A Cargo workspace. `cargo run` at the root starts the app crate, which builds the `refresher-discord-bot` binary; `cargo test --workspace` runs every crate's tests.

| Path | What lives there |
| --- | --- |
| `crates/core` | Camp rules and storage with no Discord code: config, SQLite access (`db.rs`), queue and team splits (`queue.rs`), the draft and Deadlock lobby APIs, and the `BotLink` interface the web side uses. |
| `crates/bot` | The Discord side: the shared admin actions (`actions.rs`), message embeds and syncing, voice moves, the players' buttons, and the `BotLink` implementation. |
| `crates/web` | The admin page backend (axum): Discord login, sessions, the camp API, and serving the built page. |
| `web/` | The admin page: React + Vite + TypeScript. |
| `crates/app` | `main.rs`: loads `.env`, opens the database, and runs the bot and the web server together. |
| `migrations/` | SQLite migrations, applied on startup. Kept byte-exact in git (see `.gitattributes`): sqlx refuses to start if an applied migration changes. |
| `assets/` | The bot logo. |

### Working on the page

- **Without Discord:** `cargo run -p refresher-web --example demo --features demo` starts a made-up camp at `http://127.0.0.1:8090`. Open `/demo/login` to sign in as a test admin. Actions change only the in-memory camp; nothing touches Discord or `data/camps.db`.
- **With hot reload:** run the bot (or the demo) and, in `web/`, `npm run dev`. Vite serves the page on `http://localhost:5173` and forwards `/api` and `/auth` to port 8080 (`API_TARGET=http://127.0.0.1:8090 npm run dev` for the demo). To sign in through Discord on 5173, set `WEB_PUBLIC_URL=http://localhost:5173` and add `http://localhost:5173/auth/callback` to the app's redirects.
- `npm run typecheck` checks the TypeScript.

Player names come from the bot: it looks members up on Discord (server nickname, then display name) and keeps them for 10 minutes.

### API

Discord IDs are sent as strings, since they don't fit in a JavaScript number. Every endpoint except login needs a session and only covers servers the admin manages.

| Endpoint | Returns |
| --- | --- |
| `GET /auth/login`, `GET /auth/callback`, `POST /auth/logout` | Discord sign-in and sign-out. |
| `GET /api/me` | The signed-in user and the servers they can manage. |
| `GET /api/guilds/{guild_id}/camps` | That server's camps, newest first. |
| `POST /api/guilds/{guild_id}/camps` | Opens signup for a new camp: `{"channel_id": "…", "size": "24"}` (`24`, `30`, `4-1v1`, `4-2v2`). |
| `GET /api/guilds/{guild_id}/channels` | The server's text channels. |
| `GET /api/guilds/{guild_id}/members?query=` | Members whose name starts with `query` (up to 10). |
| `GET /api/camps/{camp_id}` | Roster with names, queue and who goes next, open matches with teams, votes and draft links, recent results. |
| `GET /api/matches/{match_id}` | One match, finished or not. |
| `POST /api/camps/{camp_id}/actions` | Runs an admin action and returns the camp afterwards. Body: `{"type": "create_match"}`, `create_final`, or `close_vote` / `start_draft` / `start_match` / `correct_winner` / `reopen_match` with `"match_id"`, or `report_winner` with `"match_id"` and `"side"` (`hiddenking` / `archmother`); for the roster, `add_players` (`"user_ids"`), `remove_player` and `set_s_rank` (`"user_id"`, `"s_rank"`); and `start_queue`, `cancel_camp`, `resume_camp`, `end_camp`. A refusal (e.g. a draft before the vote ended) is a 409 with the bot's message. |
