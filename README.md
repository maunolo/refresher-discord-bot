# Deadlock camp Discord bot

Rust bot that runs a **24- or 30-player Deadlock camp** as a queue: two concurrent 6v6 custom matches, a one-time team refresh vote, SQLite persistence, and a trophy leaderboard. `/camp start` also has **4-player test** sizes: **4-1v1** (two matches at once) and **4-2v2**.

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

The first start registers `/camp` on each server (updates immediately) and creates `data/camps.db`. After a command change, restart the bot, then type `/camp start` again (reload Discord with Ctrl+R if the size list is stale).

### Text vs voice

Run `/camp start` in the **text channel** you want for the camp. Roster, queue status, refresh votes, party codes, winner buttons, and standings all stay in that text channel. There is no env var for it.

`QUEUE_VOICE_CHANNEL_ID` and the match IDs below are **voice channels only**, used to move players who are already in voice.

### Voice channels

Set the five channel IDs in `.env`, or leave them blank. If blank, the bot looks up (or creates) channels named:

- `Queue`
- `Match 1 Hiddenking`
- `Match 1 Archmother`
- `Match 2 Hiddenking`
- `Match 2 Archmother`

Players must already be in a voice channel to be moved at **Start match**. If they join later (Queue is fine), the bot moves them into the team channel automatically. They stay in Queue until an admin clicks **Start match**. After a winner they return to Queue. **Create match** and team refresh do not move anyone.

## Commands

Admins are anyone with **Manage Server**.

| Command | What it does |
| --- | --- |
| `/camp start` | Create a camp (`24`, `30`, `4-1v1`, or `4-2v2`). Prints a short camp ID. |
| `/camp add` | Add Discord users to the roster (or use the user picker on the camp message). |
| `/camp remove` | Remove a user before the queue starts. |
| `/camp cancel` | Soft-cancel. Data is kept. Resume with `/camp resume CAMPID`. |
| `/camp resume` | Restore a cancelled camp in this server. |
| `/camp end` | Final standings (after at least one scored match). Not resumable. |
| `/camp info` | Roster, queue order, points, consecutive counts, live matches. |

## Match flow

1. Admin fills the roster to exactly 24, 30, or 4 (test), then **Start queue**. The roster is shuffled; nobody is placed into a match yet.
2. Admin clicks **Create match** to pull the next group from the queue (12, 4 in a 2v2 test, or 2 in a 1v1 test) and sort Hiddenking / Archmother. Click it again for a second concurrent match when a slot is free. With 30 players, leftover players stay in queue until an admin creates another match.
3. Players on a match each vote once: **Refresh teams** or **No refresh**. If at least 4 players from either side vote refresh (2 in a 2v2 test, 1 in a 1v1 test), teams are re-sorted. If everyone has voted (or refresh can no longer pass), voting ends and the admin can start. Each match only votes once.
4. Admin **Start match** marks the game live and moves those players into the team voice channels. If `ENABLE_CUSTOM_MATCH_CREATE=true`, it also creates a Deadlock custom lobby and posts the party code. Players join in-game and draft heroes.
5. Admin reports the winner. Winners get +1 point. Players re-enter the queue by consecutive-match count (winners before losers in the same count; blocks with more than two players are shuffled). Anyone who sat out has their consecutive count reset to 0. The next match is **not** created until an admin clicks **Create match**.
6. **End camp** posts gold / silver / bronze for the top distinct point totals.

### Testing with four accounts

Use `/camp start` and pick **4-1v1** to run two matches at once, or **4-2v2** for a single match. Add your four Discord users, then **Start queue**.

- **4-1v1:** click **Create match** twice. You get two 1v1s (Match 1 and Match 2). Refresh needs **1 vote from either side**.
- **4-2v2:** click **Create match** once. You get one Hiddenking vs Archmother match of two players each. Refresh needs **2 votes from either team**.

After a winner, they stay in queue until you create the next match. Keep `ENABLE_CUSTOM_MATCH_CREATE=false` so Start match does not call the Deadlock API.

## Persistence

Every queue change, vote, lobby start, and result is written to SQLite in a transaction. Restarting the bot reloads signup/running camps, including who is queued vs in each live match. If a Discord message was deleted, the bot re-posts the embeds without resetting state.

## Honest in-game gap

The Deadlock API cannot seat Discord users onto Hiddenking / Archmother. After the code is posted, players join the Steam lobby and pick heroes themselves.
