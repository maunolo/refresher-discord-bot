# Deadlock camp bot

Runs a **24- or 30-player Deadlock camp** as a queue: two concurrent 6v6 custom matches, a one-time team refresh vote, S rank captains, pick/ban drafts, SQLite persistence and a points leaderboard. There are also **4-player test** sizes: **4-1v1** (two matches at once) and **4-2v2**.

Admins run the camp from a **web admin page** (sign in with Discord). In Discord, players see the queue and their matches, vote on refreshing the teams, and captains get their draft links. Heroes are picked in Deadlock.

## Setup

1. Create an application and bot in the [Discord Developer Portal](https://discord.com/developers/applications). On the **Bot** page, turn on **Server Members Intent**: the admin page lists the server's members to add players. No other privileged intent is needed.
2. Invite the bot with: View Channels, Send Messages, Embed Links, Connect, Move Members, and Manage Channels if you want it to create the voice channels.
3. In the application's **OAuth2** page, copy the **Client ID**, reset and copy the **Client Secret**, and add the redirect `http://localhost:8080/auth/callback` (or `{WEB_PUBLIC_URL}/auth/callback` if you change the address).
4. Copy `.env.example` to `.env` and set `DISCORD_TOKEN`, `DISCORD_CLIENT_ID` and `DISCORD_CLIENT_SECRET`. Leave `ENABLE_CUSTOM_MATCH_CREATE=false` until you need a real Deadlock lobby (then the IP limit is **10 creates per hour** without a key); `DEADLOCK_API_KEY` is optional. The voice channel, lobby and region values in `.env` are only defaults: each server can change them on the admin page's **Configurações**.
5. Build the admin page once (needs [Node.js](https://nodejs.org) 20 or newer): `cd web`, `npm install`, `npm run build`. The server serves `web/dist` (change it with `WEB_STATIC_DIR`).
6. Install Rust (`https://rustup.rs`), then from the repo root:

```bash
cargo run
```

Open `http://localhost:8080` and sign in. The first start creates `data/camps.db`.

## Deploying on Railway

The repo deploys as one Railway service: the [Dockerfile](Dockerfile) builds the admin page and the bot, and [railway.json](railway.json) tells Railway to use it. Pushing to `main` redeploys.

1. Create a service from this GitHub repo (branch `main`).
2. Add a volume mounted at `/data`; the database is `/data/camps.db` (set in the Dockerfile). Without it, every deploy starts from an empty database.
3. Generate a domain for port `8080`, and add `https://<domain>/auth/callback` to the Discord application's OAuth2 redirects.
4. Variables: `DISCORD_TOKEN`, `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, and `WEB_PUBLIC_URL=https://<domain>`. The other `.env` settings are optional defaults (servers can override them on **Configurações**). `WEB_BIND`, `WEB_STATIC_DIR` and `DATABASE_URL` already come from the image.

`WEB_PUBLIC_URL` must be the domain admins open: sign-in sends Discord `{WEB_PUBLIC_URL}/auth/callback`, so with a custom domain, set it to that domain and register that callback. Signing in on any other domain of the service fails.

Run only one copy of the bot per token: stop a local `cargo run` while the deployed one is up.

## Admin page

Anyone with **Manage Server** in a server the bot is in can sign in and run that server's camps. A sign-in lasts 12 hours; sign in again to pick up permission changes.

- **Camps:** every camp of the server. **Novo camp** opens signup in a text channel you pick, with a size (24, 30, 4-1v1, 4-2v2); a server has one active camp at a time. **Cancelar** keeps the roster, queue and points so the camp can be resumed with **Retomar**. **Encerrar** posts the final ranking and can't be undone.
- **Jogadores:** during signup, pick players from the server's members (everyone not on the roster yet, filtered by the name you type), add everyone in the Queue voice channel at once, mark S rank, remove players, and **Iniciar fila** once the roster is exactly full. After that, the roster with points, consecutive matches and where each player is. S rank can still change; it applies to matches created afterwards. **Ajustar pontos** adds or takes points by hand (for corrections or penalties, up to 50 at a time, never below zero); the bot announces each change in the camp's channel, and it counts for the ranking and the final match.
- **Ao vivo:** both match slots, the queue (→ marks who the next match takes), the last results and the ranking, refreshed every few seconds. **Criar partida**, **Partida final**, ending a refresh vote early (**Manter times** or **Sortear de novo**), **Iniciar draft**, **Iniciar partida**, the winner buttons (which ask for the match's MVP), and **Trocar vencedor** / **Reabrir** / **Trocar MVP** on the latest result. With no match open, **Encerrar camp** ends it from here. Each card links to its page (match details, Fila, Partidas, Ranking). Buttons that can't be used yet say why, and actions that change a lot ask first.
- **Fila:** the queue in order. Move players with ↑, ↓ and *topo*, or drag them on a computer; the new order stays on the page until **Salvar ordem**, which also updates the queue message in Discord. If a match starts or ends meanwhile, saving is blocked so nobody is dropped. After each result, players who were waiting keep this order at the front; the ones who just played join the end by the usual rule.
- **Partidas:** every match of the camp, open ones first. A match's page shows each player's vote (Refresh, Manter or waiting) with tallies, the next step for its stage, **Lembrar quem falta votar** (mentions only the players who haven't voted), **Fechar e manter times**, the draft links and the party code. Finished matches show the result and, for the latest one, the corrections.
- **Ranking:** every player by points, with MVPs, wins, matches played and win rate; manual adjustments show in parentheses, and **Ajustar** opens the same points dialog as Jogadores. Equal points are broken by MVPs; players equal on both share a place.
- **Configurações:** per server. The five voice channels the bot moves players to (*Automático* finds or creates them by name), whether starting a match creates the Deadlock lobby and in which region, whether the bot creates drafts on the draft tool, and the draft timer (none, or 15 to 90 seconds per pick or ban). Until an admin saves, a server uses the `.env` values. Changes apply from the next action; a draft already created keeps its timer.

Each match, the queue and each camp have an **Abrir no Discord** link to their message.

Every action also does its Discord side: the camp and match messages are redrawn, the channel gets an announcement, players are moved between voice channels, and captains get their draft DMs. Actions in the same server run one at a time, so two admins can't step on each other.

## In Discord

The camp's text channel gets:

- **The queue message:** the roster during signup, then the queue in order with who goes next, and the open matches. No buttons.
- **One message per match:** teams with captains (👑), the refresh vote count, the spectator draft link and the party code. Its buttons are only the players': **Refresh** / **Manter times** while the vote is open, and **👑 Link do capitão** once the draft exists.
- Announcements when a match is created, the vote ends, the draft starts, a match goes live, a winner is reported or corrected, and the final ranking.

There are no slash commands. On startup the bot clears the old `/camp` command from each server and strips the admin buttons from existing camp messages; any old admin button still clicked answers with a link to the admin page.

## Match flow

1. Admin fills the roster to exactly 24, 30, or 4 (test), then **Iniciar fila**. The roster is shuffled into the camp's teams, with S rank players split evenly over them (see below), and posted; nobody is placed into a match yet.
2. **Criar partida** pulls the next group from the queue (12, 4 in a 2v2 test, or 2 in a 1v1 test) and sorts Hiddenking / Archmother. It takes the first players in the queue, S rank or not. A second concurrent match can be created while a slot is free. With 30 players, leftover players stay in queue until the next match.
3. Players in a match each vote once in Discord: **Refresh** or **Manter times**. If at least 4 players from either side vote refresh (2 in a 2v2 test, 1 in a 1v1 test), teams are re-sorted, still splitting S rank players evenly. When everyone has voted, or a refresh can no longer pass, voting ends. An admin can also end it early (for when someone can't vote): **Manter times** keeps the teams as they are, **Sortear de novo** re-sorts them as if the refresh had passed (S rank split evenly, new captains, votes dropped). Each match only votes once.
4. **Iniciar draft** creates a pick/ban draft on [draft.deadlock.pro.br](https://draft.deadlock.pro.br/docs) (no API key needed), DMs each captain their own team's private link, posts the spectator link and moves the players into the team voice channels. With the draft tool turned off on **Configurações**, it becomes **Iniciar draft manual**: the bot moves the players and announces the draft, and the admin sends the captains a draft link themselves, like the party code without the automatic lobby.
5. **Iniciar partida** marks the match live. If the server has the lobby turned on (**Configurações**, default `ENABLE_CUSTOM_MATCH_CREATE`), it also creates a Deadlock custom lobby and posts the party code.
6. Admin reports the winner and picks the match's MVP, any one of its players. Winners get +1 point; the MVP is posted with the result and breaks ties in the final ranking. Players re-enter the queue by consecutive-match count (winners before losers in the same count; blocks with more than two players are shuffled). Anyone who sat out has their consecutive count reset to 0. The latest result can be corrected (**Trocar vencedor**) or reopened while its players are still in the queue; reopening also clears its MVP. Its MVP can be changed (**Trocar MVP**) for as long as it is the latest result and the camp runs, since it moves no points or queue places.
7. When no match is open and at least one has been scored, **Partida final** seats the highest-point players (same count as a normal match; ties for the last seats are shuffled) and balances the teams by points. S rank players are not guaranteed a final seat; any who make it are split evenly and captain their teams. After a final is created, no other match can be.
8. **Encerrar camp**, on Ao vivo once no match is open or on the Camps screen, posts the final ranking: gold / silver / bronze for the top three places, by points and then MVPs (players equal on both share a medal). A camp with a match still open can't be ended.

### S rank players and captains

- **Iniciar fila** shuffles the roster into the camp's teams, half a match each, in line: 5 teams of 6 in a 30-player camp, 4 in a 24-player one (4 of 1 in a 1v1 test, 2 of 2 in a 2v2 test). S rank players are split over those teams as evenly as possible, and which teams get one more is random: 8 S rank in 30 players makes three teams with 2 and two with 1. The first match takes the first two teams, the second the next two.
- After that, S rank gives no priority in the queue. Players back from a match go behind everyone waiting, by the usual consecutive-match rule, so a match can have more S rank players than another, or none.
- In a match, S rank players are dealt to the two teams first, alternating, so the teams' counts differ by one at most.
- Every S rank player on a team is its captain (👑). A team with none has one captain, its highest-points player (ties are random). Captains are chosen when teams are made and again after a refresh.
- **Iniciar draft** DMs every captain the private link for their own team; captains of the same team share one link, and the DM names the others so they can agree on who picks. Whoever opens a team link picks and bans for that team, so team links are never posted in the channel. If a DM fails, the bot mentions that captain, who clicks **Link do capitão** to see it privately. Admins can copy every link from the match card on the admin page.
- Each team gets 2 bans and one pick per player, with a 30-second timer per turn unless the server picked another on **Configurações**. To run your own copy of the [draft tool](https://github.com/deadlockprobr/draft), set `DRAFT_BASE_URL`.

### Testing with four accounts

On **Camps**, create a **4-1v1** camp (two matches at once) or **4-2v2** (one match). Add your four Discord accounts on **Jogadores**, then **Iniciar fila**.

- **4-1v1:** **Criar partida** twice for two 1v1s. Refresh needs **1 vote from either side**.
- **4-2v2:** **Criar partida** once. Refresh needs **2 votes from either team**.

When the test matches are done, **Partida final** seats the two (1v1) or four (2v2) highest-point players. Keep `ENABLE_CUSTOM_MATCH_CREATE=false` so starting a match does not call the Deadlock API.

## Voice channels

Pick each server's voice channels on **Configurações** (the `*_VOICE_CHANNEL_ID` values in `.env` are the defaults for servers that never saved). They are used to move players who are already in voice. Left on *Automático*, or if the picked channel was deleted, the bot looks up (or creates) channels named `Queue`, `Match 1 Hiddenking`, `Match 1 Archmother`, `Match 2 Hiddenking` and `Match 2 Archmother`.

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
| `GET /api/guilds/{guild_id}/members` | Every member of the server except bots. Needs the Server Members Intent; without it, a 409 that says so. |
| `GET /api/guilds/{guild_id}/settings` | The server's settings, whether they were saved, its voice channels and the draft timers allowed. |
| `PUT /api/guilds/{guild_id}/settings` | Saves them: the five `*_voice_id` (a voice channel ID or `null` for automatic), `create_lobby`, `server_region`, `use_draft_tool` (default `true`) and `draft_timer_seconds`. A channel from another server or an unlisted timer is a 400. |
| `GET /api/camps/{camp_id}` | Roster with names, points, wins and matches played, queue and who goes next, open matches with teams, votes and draft links, recent results, and Discord links. |
| `GET /api/camps/{camp_id}/matches` | Every match of the camp: open ones by slot, then finished ones newest first with their number. |
| `GET /api/matches/{match_id}` | One match, finished or not. |
| `POST /api/camps/{camp_id}/actions` | Runs an admin action and returns the camp afterwards. Body: `{"type": "create_match"}`, `create_final`, or `close_vote` / `refresh_teams` / `start_draft` / `start_match` / `correct_winner` / `reopen_match` with `"match_id"`, or `report_winner` with `"match_id"`, `"side"` (`hiddenking` / `archmother`) and `"mvp_user_id"`, or `change_mvp` with `"match_id"` and `"user_id"`; for the roster, `add_players` (`"user_ids"`), `remove_player` and `set_s_rank` (`"user_id"`, `"s_rank"`); `adjust_points` with `"user_id"` and `"delta"` (negative takes points); `reorder_queue` with the whole queue in its new order (`"user_ids"`), `remind_voters` with `"match_id"`, and `start_queue`, `cancel_camp`, `resume_camp`, `end_camp`. A refusal (e.g. a draft before the vote ended) is a 409 with the bot's message. |
