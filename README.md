# Deadlock camp Discord bot

Rust bot that runs a **24- or 30-player Deadlock camp** as a queue: two concurrent 6v6 custom matches, a one-time team refresh vote, SQLite persistence, and a trophy leaderboard. `/camp iniciar` also has **4-player test** sizes: **4-1v1** (two matches at once) and **4-2v2**.

Heroes are picked in Deadlock. Discord only sorts Hiddenking / Archmother, tracks the queue, and creates the lobby code via [Deadlock API](https://deadlock-api.com/).

## Setup

1. Create a Discord application and bot at [Discord Developer Portal](https://discord.com/developers/applications).
2. Enable the **Server Members Intent** is not required. Enable the bot in the guild with:
   - Send Messages, Embed Links, Use Application Commands, View Channels
   - Connect, Move Members
   - Manage Channels (only if you want the bot to create the five voice channels)
3. Invite the bot to your server.
4. Copy `.env.example` to `.env` and set `DISCORD_TOKEN`. Leave `ENABLE_CUSTOM_MATCH_CREATE=false` until you need a real Deadlock lobby (then the IP limit is **10 creates per hour** without a key). `DEADLOCK_API_KEY` is optional.
5. Install Rust (`https://rustup.rs`), then:

```bash
cargo run
```

The first start registers `/camp` on each server (updates immediately) and creates `data/camps.db`. After a command change, restart the bot, then type `/camp iniciar` again (reload Discord with Ctrl+R if the size list is stale).

### Text vs voice

Run `/camp iniciar` in the **text channel** you want for the camp. Roster, queue status, refresh votes, party codes, winner buttons, and standings all stay in that text channel. There is no env var for it.

`QUEUE_VOICE_CHANNEL_ID` and the match IDs below are **voice channels only**, used to move players who are already in voice.

### Voice channels

Set the five channel IDs in `.env`, or leave them blank. If blank, the bot looks up (or creates) channels named:

- `Queue`
- `Match 1 Hiddenking`
- `Match 1 Archmother`
- `Match 2 Hiddenking`
- `Match 2 Archmother`

Players must already be in a voice channel to be moved at **Start draft**. If they join later (Queue is fine), the bot moves them into the team channel automatically. They stay in Queue until an admin clicks **Start draft**. After a winner they return to Queue. **Create match** and team refresh do not move anyone.

## Commands

Admins are anyone with **Manage Server**.

| Command | What it does |
| --- | --- |
| `/camp iniciar` | Create a camp (`24`, `30`, `4-1v1`, or `4-2v2`). Prints a short camp ID. |
| `/camp adicionar` | Add Discord users to the roster (or use the user picker on the camp message). |
| `/camp remover` | Remove a user before the queue starts. |
| `/camp srank` | Toggle a roster player as S rank (or use the S rank picker on the signup message). Works during signup and while running; applies to matches created afterwards. |
| `/camp cancelar` | Soft-cancel. Data is kept. Resume with `/camp retomar CAMPID`. |
| `/camp retomar` | Restore a cancelled camp in this server. |
| `/camp encerrar` | Final standings (after at least one scored match). Not resumable. |
| `/camp info` | Roster, queue order, points, consecutive counts, live matches. |
| `/camp match_info` | Teams, captains, winner, draft link, and party code for one match ID. |

## Match flow

1. Admin fills the roster to exactly 24, 30, or 4 (test), then **Start queue**. The roster is shuffled; nobody is placed into a match yet.
2. Admin clicks **Create match** to pull the next group from the queue (12, 4 in a 2v2 test, or 2 in a 1v1 test) and sort Hiddenking / Archmother. The first two queued S rank players are always pulled in, one per team (see below). Click it again for a second concurrent match when a slot is free. With 30 players, leftover players stay in queue until an admin creates another match.
3. Players on a match each vote once: **Refresh teams** or **No refresh**. If at least 4 players from either side vote refresh (2 in a 2v2 test, 1 in a 1v1 test), teams are re-sorted, still keeping S rank players apart. If everyone has voted (or refresh can no longer pass), voting ends and the admin can start the draft. Each match only votes once.
4. Admin **Start draft** creates a pick/ban draft on [draft.deadlock.pro.br](https://draft.deadlock.pro.br/docs) (no API key needed), DMs each captain their own team's private link, posts the spectator link in the channel, and moves the players into the team voice channels.
5. Admin **Start match** marks the game live. If `ENABLE_CUSTOM_MATCH_CREATE=true`, it also creates a Deadlock custom lobby and posts the party code.
6. Admin reports the winner. Winners get +1 point. Players re-enter the queue by consecutive-match count (winners before losers in the same count; blocks with more than two players are shuffled). Anyone who sat out has their consecutive count reset to 0. The next match is **not** created until an admin clicks **Create match**.
7. When no matches are live and at least one has been scored, admin can click **Final match**. That seats the highest-point players (same count as a normal match; ties for the last seats are shuffled) and balances the teams by points. S rank players are not guaranteed a final seat, but any who make it are kept apart. Refresh voting works the same as a normal match. After a final is created, **Create match** is locked. After it is scored, **End camp**.
8. **End camp** posts gold / silver / bronze for the top distinct point totals.

### S rank players and captains

Admins mark S rank players (⭐) per camp with the picker on the signup message or `/camp srank`.

- Each new match pulls the first two S rank players in queue order, one per team. Other S rank players keep their place in line. Only when there aren't enough regular players to fill a match does a third one get in; the teams then split 2/1.
- Every team has a captain (👑): its S rank player, or, if it has none, its highest-points player (ties are random). Captains are chosen when teams are made and again after a refresh.
- **Start draft** DMs each captain the private link for their own team. Whoever opens a team link picks and bans for that team, so the bot never posts team links in the channel. The spectator link is public, in the channel and on the match message.
- If a DM fails, the bot mentions that captain in the channel, and they click **Link do capitão** on the match message to see their link privately. `/camp match_info` shows admins the spectator link and the organizer page with every link.
- Each team gets 2 bans and one pick per player, with a 30-second timer per turn. To run your own copy of the [draft tool](https://github.com/deadlockprobr/draft), set `DRAFT_BASE_URL`.
- Aim for two S rank players per concurrent match (four in a 24- or 30-player camp). With only two, they play nearly every match.

### Testing with four accounts

Use `/camp iniciar` and pick **4-1v1** to run two matches at once, or **4-2v2** for a single match. Add your four Discord users, then **Start queue**.

- **4-1v1:** click **Create match** twice. You get two 1v1s (Match 1 and Match 2). Refresh needs **1 vote from either side**.
- **4-2v2:** click **Create match** once. You get one Hiddenking vs Archmother match of two players each. Refresh needs **2 votes from either team**.

After a winner, they stay in queue until you create the next match. When both test matches are done, **Final match** seats the two (1v1) or four (2v2) highest-point players. Keep `ENABLE_CUSTOM_MATCH_CREATE=false` so Start match does not call the Deadlock API.

## Persistence

Every queue change, vote, lobby start, and result is written to SQLite in a transaction. Restarting the bot reloads signup/running camps, including who is queued vs in each live match. If a Discord message was deleted, the bot re-posts the embeds without resetting state.

## Honest in-game gap

The Deadlock API cannot seat Discord users onto Hiddenking / Archmother. After the code is posted, players join the Steam lobby and pick heroes themselves.

## Code layout

A Cargo workspace. `cargo run` at the root starts the app crate, which builds the `refresher-discord-bot` binary.

| Path | What lives there |
| --- | --- |
| `crates/core` | Camp rules and storage with no Discord code: config, SQLite access (`db.rs`), queue and team splits (`queue.rs`), the draft and Deadlock lobby APIs. |
| `crates/bot` | The Discord side: `/camp` commands, buttons, embeds, message syncing and voice moves. |
| `crates/web` | The admin web page backend (axum): Discord login, sessions, the camp API, and serving the built page. |
| `web/` | The admin page itself: React + Vite + TypeScript. |
| `crates/app` | `main.rs`: loads `.env`, opens the database and starts the bot, plus the web admin when it's configured. |
| `migrations/` | SQLite migrations, applied on startup. |
| `assets/` | The bot logo. |

`cargo test --workspace` runs every crate's tests.

## Admin web page (in progress)

Camp management is moving from Discord buttons to a web page. For now the page shows the live view (both matches, the queue, recent results and the ranking) and refreshes every few seconds; its buttons stay disabled until admin actions land, so keep using the Discord buttons meanwhile.

To turn it on:

1. In the [Developer Portal](https://discord.com/developers/applications), open the bot's application, go to **OAuth2**, copy the **Client ID** and reset/copy the **Client Secret**.
2. Under **Redirects**, add `http://localhost:8080/auth/callback` (or `{WEB_PUBLIC_URL}/auth/callback` if you change the address).
3. Set `DISCORD_CLIENT_ID` and `DISCORD_CLIENT_SECRET` in `.env`.
4. Build the page once (needs [Node.js](https://nodejs.org) 20 or newer): `cd web`, `npm install`, `npm run build`. The server serves `web/dist` (change it with `WEB_STATIC_DIR`).
5. Restart the bot and open `http://localhost:8080`.

### Working on the page

- **Without Discord:** `cargo run -p refresher-web --example demo --features demo` starts a made-up camp at `http://127.0.0.1:8090`. Open `/demo/login` to sign in as a test admin. Nothing touches Discord or `data/camps.db`.
- **With hot reload:** run the bot (or the demo) and, in `web/`, `npm run dev`. Vite serves the page on `http://localhost:5173` and forwards `/api` and `/auth` to port 8080 (`API_TARGET=http://127.0.0.1:8090 npm run dev` for the demo). To sign in through Discord on 5173, set `WEB_PUBLIC_URL=http://localhost:5173` and add `http://localhost:5173/auth/callback` to the app's redirects.
- `npm run typecheck` checks the TypeScript.

Player names come from the bot: it looks members up on Discord (server nickname, then display name) and keeps them for 10 minutes.

Anyone with **Manage Server** in a server the bot is in can sign in and see that server's camps, the same rule as the `/camp` commands. A sign-in lasts 12 hours; sign in again to pick up permission changes.

| Endpoint | Returns |
| --- | --- |
| `GET /auth/login`, `GET /auth/callback`, `POST /auth/logout` | Discord sign-in and sign-out. |
| `GET /api/me` | The signed-in user and the servers they can manage. |
| `GET /api/guilds/{guild_id}/camps` | That server's camps, newest first. |
| `GET /api/camps/{camp_id}` | Roster with names, queue and who goes next, open matches with teams, votes and draft links, recent results. |
| `GET /api/matches/{match_id}` | One match, finished or not. |

Discord IDs are sent as strings, since they don't fit in a JavaScript number.
