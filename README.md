# Deadlock camp bot

Runs a **24- or 30-player Deadlock camp** as a queue: two concurrent 6v6 custom matches, a one-time team refresh vote, S rank captains, pick/ban drafts, SQLite persistence and a points leaderboard. There are also **4-player test** sizes: **4-1v1** (two matches at once) and **4-2v2**.

Admins run the camp from a **web admin page** (sign in with Discord). In Discord, players see the queue and their matches, vote on refreshing the teams, and captains get their draft links. Heroes are picked in Deadlock.

## Setup

1. Create an application and bot in the [Discord Developer Portal](https://discord.com/developers/applications). On the **Bot** page, turn on **Server Members Intent**: the admin page lists the server's members to add players. No other privileged intent is needed.
2. Invite the bot with: View Channels, Send Messages, Embed Links, Connect, Move Members, and Manage Channels if you want it to create the voice channels.
3. In the application's **OAuth2** page, copy the **Client ID**, reset and copy the **Client Secret**, and add the redirect `http://localhost:8080/auth/callback` (or `{WEB_PUBLIC_URL}/auth/callback` if you change the address).
4. Copy `.env.example` to `.env` and set `DISCORD_TOKEN`, `DISCORD_CLIENT_ID` and `DISCORD_CLIENT_SECRET`. Leave `ENABLE_CUSTOM_MATCH_CREATE=false` until you need a real Deadlock lobby (then the IP limit is **10 creates per hour** without a key); `DEADLOCK_API_KEY` is optional, but without it the [match scoreboards](#match-scoreboards) can ask Steam only 3 times an hour. Set `DEADCHAPS_API_KEY` for the bot to create drafts (see [Drafts](#drafts)). The voice channel, lobby and region values in `.env` are only defaults: each server can change them on the admin page's **Configurações**.
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
4. Variables: `DISCORD_TOKEN`, `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `WEB_PUBLIC_URL=https://<domain>`, and `DEADCHAPS_API_KEY` for drafts. The other `.env` settings are optional defaults (servers can override them on **Configurações**). `WEB_BIND`, `WEB_STATIC_DIR`, `SITE_DIR` and `DATABASE_URL` already come from the image. For the public site, see [Public site](#public-site).

`WEB_PUBLIC_URL` must be the domain admins open: sign-in sends Discord `{WEB_PUBLIC_URL}/auth/callback`, so with a custom domain, set it to that domain and register that callback. Signing in on any other domain of the service fails.

Run only one copy of the bot per token: stop a local `cargo run` while the deployed one is up.

To preview a branch without a second bot on the token, deploy it as a separate service with no Discord variables running the demo (see [Working on the page](#working-on-the-page)): variables `BUILD_DEMO=true` (the image then also builds `./refresher-demo`), `DEMO_BIND=0.0.0.0:8080` and `WEB_PUBLIC_URL=https://<its domain>`, and start command `./refresher-demo`. Open `/demo/login` to sign in and `/site/` for the public site. Its camp lives in memory and starts over on each deploy.

## Admin page

Anyone with **Manage Server** in a server the bot is in can sign in and run that server's camps. A sign-in lasts 12 hours; sign in again to pick up permission changes.

- **Camps:** every camp of the server. **Novo camp** opens signup in a text channel you pick, with a size (24, 30, 4-1v1, 4-2v2); a server has one active camp at a time. **Cancelar** keeps the roster, queue and points so the camp can be resumed with **Retomar**. **Encerrar** posts the final ranking and can't be undone.
- **Jogadores:** during signup, pick players from the server's members (everyone not on the roster yet, filtered by the name you type), add everyone in the Queue voice channel at once, mark S rank, pick each player's main role (see [Roles](#roles)), remove players, and **Iniciar fila** once the roster is exactly full. After that, the roster with points, consecutive matches and where each player is. S rank and roles can still change; they apply to teams made afterwards (a new match or a refresh). **Ajustar pontos** adds or takes points by hand (for corrections or penalties, up to 50 at a time, never below zero); the bot announces each change in the camp's channel, and it counts for the ranking and the final match. **Remover** takes a player out of the running camp: out of the queue now, or, if they're in a match, once it ends (they don't go back to the queue or the Queue voice channel). They're never picked again, the final included, but keep their points and matches in the ranking, marked *Removido*. **Devolver** brings them back at the end of the queue. The bot announces both in the camp's channel.
- **Ao vivo:** both match slots, the queue (→ marks who the next match takes), the last results and the ranking. **Criar partida**, **Partida final**, ending a refresh vote early (**Manter times** or **Sortear de novo**), **Iniciar draft**, **Iniciar partida**, the winner buttons (which ask for the match's MVP), **Trocar vencedor** / **Reabrir** / **Trocar MVP** on the latest result, and **Cancelar partida** on an open match (voting, draft or live): it's deleted, no one gets points, and its players go back to the queue where they were before it, even after a refresh. The bot marks its Discord message as cancelled and moves whoever is in voice to Queue. To swap out a player who was just picked, remove them (Jogadores) and cancel the match; the next match then comes from the queue without them. Cancelling the final lets you create it again. With no match open, **Encerrar camp** ends it from here. Each card links to its page (match details, Fila, Partidas, Ranking). Buttons that can't be used yet say why, and actions that change a lot ask first.
- **Fila:** the queue in order. Move players with ↑, ↓ and *topo*, or drag them on a computer; the new order stays on the page until **Salvar ordem**, which also updates the queue message in Discord. If a match starts or ends meanwhile, saving is blocked so nobody is dropped. After each result, players who were waiting keep this order at the front; the ones who just played join the end by the usual rule.
- **Partidas:** every match of the camp, open ones first. A match's page shows each player's vote (Refresh, Manter or waiting) with tallies, the next step for its stage, **Lembrar quem falta votar** (mentions only the players who haven't voted), **Fechar e manter times**, the draft links and the party code. Finished matches show the result and, for the latest one, the corrections.
- **Ranking:** every player by points, with MVPs, wins, matches played and win rate; manual adjustments show in parentheses, and **Ajustar** opens the same points dialog as Jogadores. Equal points are broken by MVPs; players equal on both share a place.
- **Overlays:** per server, the OBS browser sources (see [OBS overlays](#obs-overlays)): the links of the vote and ranking overlays, and the commentators' cameras. Set the VDO.Ninja room (pasting its link fills the name and password, and checks the password against the link's `hash`), list one camera per commentator (a name for the screen and the VDO.Ninja ID; pasting their link takes the ID, **Gerar ID** makes one, and **Link do comentarista** copies the link they open to send their camera), and pick each stream's left and right camera. **Salvar câmeras** updates open overlays within seconds. Each stream has its OBS link, a **Prévia** over the commentators' art, and **Teste do layout** with colored boxes coming and going instead of cameras. **Replays** (with the `R2_*` variables set, see [Replay clips](#replay-clips)): upload a clip (MP4 with H.264 and AAC, or WebM, up to 1 GB; the page reads its length and refuses a video the browser can't show), then **Tocar na 1**, **Tocar na 2** or **Nas duas** plays it on those streams' replay overlays; each stream shows what it plays, with **Parar**. **Prévia** plays a clip on the page, and **Apagar** deletes it from the bucket.
- **Configurações:** per server. The five voice channels the bot moves players to (*Automático* finds or creates them by name), whether starting a match creates the Deadlock lobby and in which region, whether the bot creates drafts on DeadChaps, the drafts' rules (see [Drafts](#drafts)), and whether captains get the draft link by DM. Until an admin saves, a server uses the `.env` values. Changes apply from the next action; a draft already created keeps its rules.
- **Site:** only for the server in `SITE_GUILD_ID`. The selected camp's four info cards for the [public site](#public-site), in Portuguese, English and Spanish, and **Usar os dados deste camp no site**, which makes the site show this camp's ranking and match history. It also says how many finished matches have their [scoreboard](#match-scoreboards) yet.
- **Regras:** the queue and match rules written out in Portuguese: starting the queue, who plays, the team split, the refresh vote and what a refresh changes, the requeue after a result, the final, and a 30-player example. It opens even with no camp selected.

Each match, the queue and each camp have an **Abrir no Discord** link to their message.

The page updates as soon as anything changes: an admin's action here or in another tab, a player's vote or button in Discord, someone joining the Queue voice channel during signup. The server streams a notice per change (`/api/guilds/{guild_id}/events`) and the page reloads what it shows; the header says **Tempo real** while that stream is up. If it drops, the header says **Reconectando…** and the page reloads every 4 seconds until it's back. A reload that brings nothing new doesn't redraw the page, and the action status floats at the bottom instead of pushing the page down.

Every action also does its Discord side: the camp and match messages are redrawn, the channel gets an announcement, players are moved between voice channels, and captains get their draft DMs. Actions in the same server run one at a time, so two admins can't step on each other.

## In Discord

The camp's text channel gets:

- **The queue message:** the roster during signup, then the queue in order with who goes next, and the open matches. No buttons.
- **One message per match:** teams with captains (👑), the refresh vote count, the spectator draft link and the party code. Its buttons are only the players': **Refresh** / **Manter times** while the vote is open, and **👑 Link do capitão** once the draft exists.
- Announcements when a match is created, the vote ends, the draft starts, a match goes live, a winner is reported or corrected, and the final ranking.

There are no slash commands. On startup the bot clears the old `/camp` command from each server and strips the admin buttons from existing camp messages; any old admin button still clicked answers with a link to the admin page.

## Match flow

1. Admin fills the roster to exactly 24, 30, or 4 (test), then **Iniciar fila**. The roster is shuffled into the camp's teams, with S rank players and roles split evenly over them (see below), and posted; nobody is placed into a match yet.
2. **Criar partida** pulls the next group from the queue (12, 4 in a 2v2 test, or 2 in a 1v1 test) and sorts Hiddenking / Archmother, giving each team one of each role where it can (see [Roles](#roles)). It takes the first players in the queue, S rank or not, and moves them into their team voice channels so each team can talk over the refresh vote. A second concurrent match can be created while a slot is free. Its dialog sets what the match is worth: points for each winner and for each loser, 0 to 10 (1 and 0 by default, then whatever the camp's last match used). The match's message in Discord and its card show it. With 30 players, leftover players stay in queue until the next match.
3. Players in a match each vote once in Discord: **Refresh** or **Manter times**. If at least 4 players from either side vote refresh (2 in a 2v2 test, 1 in a 1v1 test), teams are re-sorted, still splitting S rank players evenly, and the refresh can bring players in from the queue (see [Refresh](#refresh)). When everyone has voted, or a refresh can no longer pass, voting ends. An admin can also end it early (for when someone can't vote): **Manter times** keeps the teams as they are, **Sortear de novo** re-sorts them as if the refresh had passed (same queue rule, S rank split evenly, new captains, votes dropped). Each match only votes once.
4. **Iniciar draft** creates a private pick/ban draft on [DeadChaps](https://www.deadchaps.gg/draft) (needs `DEADCHAPS_API_KEY`), gives the captains its invite link (by DM, unless turned off on **Configurações**, and always on the match message's **Link do capitão** button), posts the spectator link and puts anyone out of place back in the team voice channels. With the draft tool turned off on **Configurações**, it becomes **Iniciar draft manual**: the bot gathers the players the same way and announces the draft, and the admin sends the captains a draft link themselves, like the party code without the automatic lobby.
5. **Iniciar partida** marks the match live. If the server has the lobby turned on (**Configurações**, default `ENABLE_CUSTOM_MATCH_CREATE`), it also creates a Deadlock custom lobby and posts the party code.
6. Admin reports the winner and picks the match's MVP, any one of its players. Each winner gets the match's win points and each loser its loss points; the MVP is posted with the result and breaks ties in the final ranking. Players re-enter the queue by consecutive-match count (winners before losers in the same count; blocks with more than two players are shuffled). Anyone who sat out has their consecutive count reset to 0. The latest result can be corrected (**Trocar vencedor**) or reopened while its players are still in the queue; reopening also clears its MVP. Its MVP can be changed (**Trocar MVP**) for as long as it is the latest result and the camp runs, since it moves no points or queue places.
7. When no match is open and at least one has been scored, **Partida final** seats the highest-point players (same count as a normal match; ties for the last seats are shuffled), worth what its dialog sets, and balances the teams by points, after giving each team one of each role where it can. S rank players are not guaranteed a final seat; any who make it are split evenly and captain their teams. After a final is created, no other match can be.
8. **Encerrar camp**, on Ao vivo once no match is open or on the Camps screen, posts the final ranking: gold / silver / bronze for the top three places, by points and then MVPs (players equal on both share a medal). A camp with a match still open can't be ended.

### Refresh

A refresh draws new teams, and in a normal match it can swap players with the queue:

- The match's players with the fewest consecutive matches keep their seats: they had been waiting. Precisely, everyone below the match's highest consecutive count stays.
- The other seats are drawn at random among the match's players at that highest count and every queued player with no more than it. Nobody who has played more matches in a row than the match's players comes in.
- Whoever loses their seat goes back to the front of the queue, in the Queue voice channel; whoever comes in goes to their team's channel. The announcement lists both.
- With nobody eligible in the queue (often in a 24-player camp, where both matches empty it), the refresh re-sorts the same players.
- The final match only re-sorts its finalists, so it stays the highest-point players.

Example, 30 players: match 3 has the 6 who sat out match 1 (0 in a row) and 6 from match 1 (1 in a row); the other 6 from match 1 wait in the queue. A refresh keeps the 6 who sat out and draws the other 6 seats among the 12 from match 1.

### Roles

Each player can have a main role: **Tank/Bruiser**, **Carry** or **Utility/Support**. Admins pick it on **Jogadores** (during signup and while the camp runs). It belongs to the server, not the camp, so a player keeps it in every later camp there. Players without one simply don't count toward any role.

When a match's teams are made (a new match, a refresh, the final), the bot looks at every way to split the players in half and keeps the best:

1. S rank split evenly, as always.
2. The fewest roles missing from the teams. With two of each role in the match, both teams get one of each. When a role is short, the gap is shared out: with one tank and one support, each team gets one of them rather than one team getting both.
3. The final only: the closest point totals.
4. Among the splits left, one at random, sides included.

**Iniciar fila** also spreads roles over the camp's teams of 6, after S rank: rarest role first, each player to the team with the fewest of their role, so every team has one of each where the roster allows and every early match starts with at least two of each. After that, roles only decide who goes to which side: who plays comes from the queue order, and a refresh still draws by the consecutive-match rule; the split then does its best with those players. Discord shows each player's role next to their name (🛡️ Tank/Bruiser, 🎯 Carry, ✨ Utility/Support), and the admin page shows it on match cards and the queue.

### S rank players and captains

- **Iniciar fila** shuffles the roster into the camp's teams, half a match each, in line: 5 teams of 6 in a 30-player camp, 4 in a 24-player one (4 of 1 in a 1v1 test, 2 of 2 in a 2v2 test). S rank players are split over those teams as evenly as possible, and which teams get one more is random: 8 S rank in 30 players makes three teams with 2 and two with 1. Roles are then spread over the same teams (see [Roles](#roles)). The first match takes the first two teams, the second the next two.
- After that, S rank gives no priority in the queue. Players back from a match go behind everyone waiting, by the usual consecutive-match rule, so a match can have more S rank players than another, or none.
- In a match, the S rank players are split between the two teams, so the teams' counts differ by one at most. Roles never override this.
- Every S rank player on a team is its captain (👑). A team with none has one captain, its highest-points player (ties are random). Captains are chosen when teams are made and again after a refresh.
- **Iniciar draft** DMs every captain the draft's invite link and the side to take in the lobby; the DM names the team's other captains so they can agree on who picks. Anyone with the invite link can take a slot, so it is never posted in the channel. If a DM fails, the bot mentions that captain, who clicks **Link do capitão** to see it privately. With **Mandar o link do draft por DM aos capitães** off on **Configurações**, nobody is DM'd: the announcement sends captains to that button, which only they can open. Admins can copy both links from the match card on the admin page.

### Drafts

Drafts are created on [DeadChaps](https://www.deadchaps.gg/draft) through its [API](https://www.deadchaps.gg/api), with the key in `DEADCHAPS_API_KEY`. The key needs the `drafts.write` scope; DeadChaps hands keys out on its Discord. Without a key, **Iniciar draft** fails and says so: turn the draft tool off on **Configurações** to share draft links by hand.

- Each draft is private. Anyone can watch from the spectator link; the invite link lets whoever has it take a slot on either side.
- Hiddenking is DeadChaps' first team, and each player gets one pick. The rest is per server, on **Configurações** → **Draft** (defaults in brackets):
  - **Tempo por escolha e banimento**: 15 to 90 seconds per turn [30], or none, which DeadChaps caps at 5 minutes a turn with a 10-minute reserve.
  - **Tempo de reserva por time**: extra time each team spends once a turn runs out, from none to 10 minutes [90 seconds, DeadChaps' default].
  - **Bans por time**: 0 (no bans) to 3 [2].
  - **Visualização do draft**: DeadChaps' cinematic or competitive draft page [cinematic].
  - **Mandar o link do draft por DM aos capitães** [on].
- Drafts are always private, so only the captains, through the invite link, can take a slot; the side names stay Hiddenking and Archmother.
- A 1v1 test match drafts as 2v2, DeadChaps' smallest lobby.
- The match card on the admin page also links the draft's OBS overlay (the spectator link plus `/obs`), to add as a 1920×1080 browser source when streaming the draft. **CSS do OBS** copies the source's Custom CSS (`web/src/obs.ts`, the same rules as Haunter's `overlay/draft-overlay.css`): OBS's default, DeadChaps' boxes laid out on the streamer's draft scene (bans in the gold band, greyed out; picks in the bottom panel; the turn timer between the bottom lines), and each hero as its card art (the battered card for bans). A hero missing there keeps DeadChaps' own image.

### Testing with four accounts

On **Camps**, create a **4-1v1** camp (two matches at once) or **4-2v2** (one match). Add your four Discord accounts on **Jogadores**, then **Iniciar fila**.

- **4-1v1:** **Criar partida** twice for two 1v1s. Refresh needs **1 vote from either side**.
- **4-2v2:** **Criar partida** once. Refresh needs **2 votes from either team**.

When the test matches are done, **Partida final** seats the two (1v1) or four (2v2) highest-point players. Keep `ENABLE_CUSTOM_MATCH_CREATE=false` so starting a match does not call the Deadlock API.

## OBS overlays

Pages for an OBS browser source, on a transparent background so only the panels show over the scene (a camera, the game, the camp's art). They update by themselves, from the same live updates as the admin page, and need no login: each server has a secret in its links, on **Overlays**. Anyone with a link sees the overlay, so **Gerar links novos** replaces the secret if one leaks, for every overlay; the old links stop working. The page shows the link with its secret hidden.

- **Votação** (`/overlay/{secret}/votacao`): a match's refresh vote, from the match's creation until its draft starts. Both teams with avatars, captains, S rank and roles, who has voted (never how), each side's Refresh votes against the 4 needed, and the votes to keep the teams. When the vote ends it shows the outcome for 10 seconds, the new teams with who came in from the queue or the teams kept, then fades out. Nothing shows while no vote is open. Suggested source size: 700 × 480, or 700 × 960 for both matches at once; `?partida=1` or `?partida=2` keeps one match's vote.
- **Ranking** (`/overlay/{secret}/ranking`): the top 10 by points, then MVPs, with the bot's places (equal on both share a place). Each row has wins and losses; MVPs aren't shown but still break ties in points, so they set the order. The top corner says Refresher League and how many matches the camp has played. Once the camp ends, its final ranking stays up until another camp starts; before the first result it says the ranking comes after it. Suggested source size: 440 × 700; `?top=15` shows more (up to 30).

- **Câmeras** (`/overlay/{secret}/cameras1` and `cameras2`, one per stream): the commentators' VDO.Ninja cameras that stream picks on **Overlays**, in Haunter's gold frame with each name below, at 1920 × 1080 (the scene's art goes below it in OBS). Both side by side; while only one is live it moves to the middle, and with none the left frame says *aguardando*. A camera has to be gone for 2.5 seconds before it leaves, so a blip doesn't flash. `?debug=1` logs VDO.Ninja's events on screen, `?demo=1` shows colored boxes instead of the cameras. Keys, with the source on Interact: 1 or 2 force that many cameras, A goes back to automatic, D toggles the log. The link carries the room's password, as the VDO.Ninja links do, so keep it off the stream.

- **Replay** (`/overlay/{secret}/replay1` and `replay2`, one per stream): the clip an admin plays on **Overlays**, full screen at 1920 × 1080 with a REPLAY tag and the clip's name. It fades in, plays with its sound (turn on *Control audio via OBS* so it reaches the mixer), and fades out at its end; transparent the rest of the time. Stopped from the page, it fades out at once. A replay that ended before the overlay loaded doesn't play again, and one that started a moment before joins where the other stream is.

The party code and draft links are never on an overlay. `overlay/` has Haunter's original local version of the cameras overlay and its art.

## Replay clips

Replays are stored in a private Cloudflare R2 bucket. The admin page uploads a clip straight from the browser to the bucket and the OBS overlays play it straight from there, both with links the server signs (`crates/core/src/r2.rs`, AWS Signature V4) that expire: an hour to upload, 12 hours to play. Videos never go through the server or Railway.

- Variables: `R2_ACCOUNT_ID`, `R2_BUCKET`, and an R2 API token's `R2_ACCESS_KEY_ID` and `R2_SECRET_ACCESS_KEY` with **Object Read & Write** on that bucket only. Without all four, replays are off and **Overlays** says so.
- The bucket's CORS policy (R2 → bucket → Settings → CORS Policy) must let the admin page upload:

  ```json
  [{ "AllowedOrigins": ["https://bot.refresherleague.com.br"], "AllowedMethods": ["PUT", "GET", "HEAD"], "AllowedHeaders": ["Content-Type"], "MaxAgeSeconds": 3600 }]
  ```

- The bucket needs no public access or custom domain. R2 doesn't charge for downloads; storage beyond its free 10 GB is $0.015 per GB a month.
- The demo uses a real bucket when the `R2_*` variables are set; uploading from it needs `http://127.0.0.1:8090` in the CORS policy too.

## Public site

The league's public site ([refresherleague.com.br](https://refresherleague.com.br/)) lives in `site/` and is served by the bot, replacing the old Hostinger copy and its PHP staff panel. Its pages and assets are static files. Its data comes from the camp published on the admin page's **Site** tab:

- **Ranking:** the camp's players by points, then MVPs, with their wins and matches, named as in Discord.
- **Match history:** the camp's finished matches that have their Deadlock [scoreboard](#match-scoreboards), oldest first: heroes, K/D/A, souls, damage, objective damage, healing and final items. Players show their Discord name if their Steam account is linked, else their Steam name. The MVP is the admin's pick. Heroes and items the site has no icon for use the Deadlock API's.
- **Info cards:** what admins typed on the Site tab for that camp.

The pages load them as `info.js`, `jogadores.js` and `partida.js` (`window.INFO`, `window.JOGADORES`, `window.PARTIDAS`); the same data is JSON at `/api/info`, `/api/ranking` and `/api/matches`, open to any domain. Browsers re-check all of it on every visit, so changes show up on the next page load.

- It's always at `{WEB_PUBLIC_URL}/site/`, so it can be checked before its domain points here. Each domain in `SITE_HOSTS` shows it at its root and nothing of the admin page.
- `SITE_GUILD_ID` is the server whose admins see the **Site** tab and pick the camp it shows.
- While `SITE_LOCK=user:password` is set, the whole site asks for that login, like the old test server's `.htaccess`. Remove it to open the site.
- The footer's **Staff** link (`/staff/`) opens the admin page's Site tab.

To move the domain here: add `refresherleague.com.br` (and `www.`, if used) as custom domains on the Railway service, create the DNS records Railway shows at the domain's DNS host (Hostinger), and set `SITE_HOSTS=refresherleague.com.br,www.refresherleague.com.br`.

## Match scoreboards

When a match ends, the bot fetches its scoreboard from the [Deadlock API](https://api.deadlock-api.com/docs) for the public site's match history, and shows it on the match's page in **Partidas**.

- **With the bot's lobby** (**Criar o lobby ao iniciar a partida** on): the bot turns the lobby's party into the Deadlock match ID, trying every 2 minutes for 12 minutes, then waits for the match's data. Valve publishes it a few minutes after the game, so the bot tries over about two hours, more often at first.
- **Without it**, or if the party never turns into a match: the match's page asks for the **ID da partida**, from the Deadlock match history or the end-of-match screen. The bot looks it up within seconds, and giving an ID again retries a lookup that failed or replaces a wrong one.
- **Steam accounts:** the scoreboard lists players by Steam account. On the match's page, pick which player of that side played on each account; the link holds for every later match on the server. When all but one of a team are linked (always, in a 1v1), the bot links the last one itself.
- **Rate limits:** only tries that let the API ask Steam for the match count against a tight limit: 3 an hour per IP without `DEADLOCK_API_KEY`, 300 with one. The other tries read the API's own store, which allows about 100 a second. So the bot asks Steam no sooner than 5 minutes after the match ended (Valve hasn't published it before) and, without a key, at most every 15 minutes per match and twice an hour across all matches, leaving one spare. A try due to ask Steam when the hour's two are spent still reads the store, and if the match isn't there it doesn't count: it comes back when a lookup frees up. With a key, every try from 5 minutes asks Steam (up to 250 an hour).

The healing column is `player_healing` from the API, which can differ from the in-game scoreboard; every other column matches it.

## Voice channels

Pick each server's voice channels on **Configurações** (the `*_VOICE_CHANNEL_ID` values in `.env` are the defaults for servers that never saved). They are used to move players who are already in voice. Left on *Automático*, or if the picked channel was deleted, the bot looks up (or creates) channels named `Queue`, `Match 1 Hiddenking`, `Match 1 Archmother`, `Match 2 Hiddenking` and `Match 2 Archmother`.

Players are moved into their team channels when their match is created, so each team can talk over the refresh vote. A refresh moves the new teams, sends whoever lost their seat back to Queue, and brings whoever came in from the queue to their team. Starting the draft puts anyone out of place back in their team channel. While the match is open, a player who joins voice or switches channels is moved to their team automatically. After a winner they return to Queue.

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
| `site/` | The public league site: static pages and assets. The bot builds its data scripts (`crates/web/src/site.rs`). |
| `crates/app` | `main.rs`: loads `.env`, opens the database, and runs the bot and the web server together. |
| `migrations/` | SQLite migrations, applied on startup. Kept byte-exact in git (see `.gitattributes`): sqlx refuses to start if an applied migration changes. |
| `assets/` | The bot logo. |

### Working on the page

- **Without Discord:** `cargo run -p refresher-web --example demo --features demo` starts a made-up camp at `http://127.0.0.1:8090`. Open `/demo/login` to sign in as a test admin. Actions change only the in-memory camp; nothing touches Discord or `data/camps.db`. `POST /demo/discord-vote` casts a missing refresh vote as a player in Discord would, to watch the page update by itself.
- **With hot reload:** run the bot (or the demo) and, in `web/`, `npm run dev`. Vite serves the page on `http://localhost:5173` and forwards `/api` and `/auth` to port 8080 (`API_TARGET=http://127.0.0.1:8090 npm run dev` for the demo). To sign in through Discord on 5173, set `WEB_PUBLIC_URL=http://localhost:5173` and add `http://localhost:5173/auth/callback` to the app's redirects.
- `npm run typecheck` checks the TypeScript.
- The OBS overlays are a second page (`web/overlay.html`, `web/src/overlay/`). In the demo, copy a link from **Overlays**; `npm run dev` serves `/overlay/...` too.

Player names come from the bot: it looks members up on Discord (server nickname, then display name) and keeps them for 10 minutes.

### API

Discord IDs are sent as strings, since they don't fit in a JavaScript number. Every endpoint except login and the overlays' needs a session and only covers servers the admin manages. The overlays' endpoints take the server's overlay secret instead.

| Endpoint | Returns |
| --- | --- |
| `GET /auth/login`, `GET /auth/callback`, `POST /auth/logout` | Discord sign-in and sign-out. |
| `GET /api/me` | The signed-in user and the servers they can manage. |
| `GET /api/guilds/{guild_id}/camps` | That server's camps, newest first. |
| `POST /api/guilds/{guild_id}/camps` | Opens signup for a new camp: `{"channel_id": "…", "size": "24"}` (`24`, `30`, `4-1v1`, `4-2v2`). |
| `GET /api/guilds/{guild_id}/channels` | The server's text channels. |
| `GET /api/guilds/{guild_id}/members` | Every member of the server except bots. Needs the Server Members Intent; without it, a 409 that says so. |
| `GET /api/guilds/{guild_id}/overlays`, `POST` to replace | The server's OBS overlay links (`vote_url`, `ranking_url`, and `camera_urls` for streams 1 and 2), made on first use. `POST` makes new ones; the old ones stop working. |
| `GET /api/guilds/{guild_id}/cameras`, `PUT` to save | The commentators' cameras: `room`, `password`, `sources` (`id`, the VDO.Ninja ID, and `name`) and `streams`, two of them, each with a `label` and the `left` and `right` source IDs (or `null`). Saving cleans IDs and the room up as VDO.Ninja writes them and refuses (400) an ID used twice, a stream showing a camera not on the list, or the same camera on both sides. |
| `GET /api/overlay/{secret}/votes` | No session. The refresh votes of the server's camp, from each match's creation until its draft starts: `state` (`open`, then `refreshed` or `kept`), `needed`, `keep`, `voted`, `total`, and per team its Refresh votes and players (name, avatar, captain, S rank, role, whether they voted). A wrong or replaced secret is a 404. |
| `GET /api/overlay/{secret}/ranking` | No session. The ranking of the server's active camp, else its latest one: `match_count`, `ended`, and per player best first `place`, `points`, `mvps`, `wins`, `played`, `moved` (places moved with the latest result, `null` before a second one), `scored_last` (got points from it) and `last_points`, with name and avatar. |
| `GET /api/guilds/{guild_id}/clips` | The replay clips (`enabled` is false without the `R2_*` variables): each with `title`, `content_type`, `size_bytes`, `duration_ms`, `created_at` and a signed `url` to preview it, and what each stream plays (`playing`: `stream`, `play_id`, `clip_id`, `title`, `duration_ms`, `elapsed_ms`). |
| `POST /api/guilds/{guild_id}/clips` | Body: `title`, `content_type` (`video/mp4` or `video/webm`) and `size_bytes` (up to 1 GB). Returns `clip_id` and `upload_url`, a signed link the browser `PUT`s the file to (for an hour, with the same `Content-Type`). |
| `POST /api/guilds/{guild_id}/clips/{clip_id}/ready` | Body: `duration_ms`. After the upload: the server checks the file is in the bucket and lists the clip. |
| `POST /api/guilds/{guild_id}/clips/{clip_id}/play` | Body: `streams`, `[1]`, `[2]` or `[1, 2]`. Plays the clip on those replay overlays now. |
| `POST /api/guilds/{guild_id}/replay/stop` | Body: `streams`. Takes the replay off those overlays. |
| `POST /api/guilds/{guild_id}/clips/{clip_id}/delete` | Deletes the clip and its file; a stream playing it stops. |
| `GET /api/overlay/{secret}/replay/{stream}` | No session. Stream 1's or 2's last replay (`playing`, or `null` once stopped): `play_id`, `title`, a signed `url`, `duration_ms` and `elapsed_ms` by the server's clock. |
| `GET /api/overlay/{secret}/cameras/{stream}` | No session. Stream 1's or 2's cameras for its overlay: `room`, `password` and `cameras` (`id`, `name`), left then right. |
| `GET /api/overlay/{secret}/events` | No session. The same `change` events as below, for the overlay's server. |
| `GET /api/guilds/{guild_id}/events` | Live updates as server-sent events: a `change` event each time the server's camp changes (admin actions, Discord buttons, the Queue voice channel), and a keep-alive comment every 15 seconds. |
| `GET /api/guilds/{guild_id}/settings` | The server's settings, whether they were saved, its voice channels, and the draft timers and reserves allowed and the most bans. |
| `PUT /api/guilds/{guild_id}/settings` | Saves them: the five `*_voice_id` (a voice channel ID or `null` for automatic), `create_lobby`, `server_region`, `use_draft_tool` (default `true`), `draft_timer_seconds`, `draft_reserve_seconds` (default 90), `draft_bans_per_team` (0 to 3, default 2), `draft_view` (`cinematic` or `competitive`, default `cinematic`) and `dm_captains` (default `true`). A channel from another server, an unlisted timer or reserve, too many bans or an unknown view is a 400. |
| `GET /api/camps/{camp_id}` | Roster with names, points, wins and matches played, queue and who goes next, open matches with teams, votes and draft links, recent results, and Discord links. |
| `GET /api/camps/{camp_id}/site`, `PUT` to save | The site server's camps only. `published`, the site's `url`, `info` (`data`, `formato`, `premiacao`, `times`, each `pt`/`en`/`es`), `finished_matches` and how many have `scoreboards`. `PUT` takes `{"info": …}` and returns it cleaned: empty cards become "A definir"/"TBD"/"Por definir". |
| `POST /api/camps/{camp_id}/site/publish` | Makes the camp the one the public site shows. |
| `GET /api/matches/{match_id}/stats` | The match's Deadlock scoreboard lookup: `status` (`pending`, `needs_id`, `ready`, `failed`, or `none` before it ends), `deadlock_match_id`, `attempts`, `next_attempt_at`, `error`, and once ready the `scoreboard` (`duration_s`, `winner_side`, and per player the Steam `account_id` and name, `side`, hero, K/D/A, souls and the linked `user_id`). |
| `PUT /api/matches/{match_id}/stats` | `{"deadlock_match_id": "139086680"}`: looks the finished match up by that ID; the same ID again retries. Not a number is a 400; a match still open is a 409. |
| `PUT /api/guilds/{guild_id}/deadlock-accounts/{account_id}` | `{"user_id": "…"}` links the Steam account to that member for the server's scoreboards; `null` unlinks it. |
| `GET /api/info`, `/api/ranking`, `/api/matches` (site domains, or `/site/api/…`) | No session, any origin. The public site's data for the published camp: the info cards; the ranking (`posicao`, `nome`, `pontos`, `mvps`, `vitorias`, `partidas`, `avatar`); the match history (`partidas`, oldest first, and `itens`), in the shapes `info.js`, `jogadores.js` and `partida.js` give the pages. |
| `GET /api/camps/{camp_id}/matches` | Every match of the camp: open ones by slot, then finished ones newest first with their number. |
| `GET /api/matches/{match_id}` | One match, finished or not. |
| `POST /api/camps/{camp_id}/actions` | Runs an admin action and returns the camp afterwards. Body: `{"type": "create_match"}` or `create_final`, with optional `"win_points"` and `"loss_points"` (0 to 10; 1 and 0 when left out), or `close_vote` / `refresh_teams` / `start_draft` / `start_match` / `correct_winner` / `reopen_match` / `cancel_match` with `"match_id"`, or `report_winner` with `"match_id"`, `"side"` (`hiddenking` / `archmother`) and `"mvp_user_id"`, or `change_mvp` with `"match_id"` and `"user_id"`; for the roster, `add_players` (`"user_ids"`), `remove_player` and `restore_player` (`"user_id"`; removing before the queue starts takes the player off the roster, afterwards out of the queue and new matches), `set_s_rank` (`"user_id"`, `"s_rank"`), `set_role` (`"user_id"`, `"role"`: `tank` / `carry` / `support`, or `null` to clear it); `adjust_points` with `"user_id"` and `"delta"` (negative takes points); `reorder_queue` with the whole queue in its new order (`"user_ids"`), `remind_voters` with `"match_id"`, and `start_queue`, `cancel_camp`, `resume_camp`, `end_camp`. A refusal (e.g. a draft before the vote ended) is a 409 with the bot's message. |
