use rand::seq::SliceRandom;
use rand::Rng;

const CAMP_ID_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuePlayer {
    pub user_id: i64,
    pub consecutive_matches: i64,
    pub queue_position: i64,
    /// `Some(true)` won the match being requeued, `Some(false)` lost, `None` was waiting.
    pub just_won: Option<bool>,
}

pub fn generate_camp_id(rng: &mut impl Rng) -> String {
    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CAMP_ID_ALPHABET.len());
            CAMP_ID_ALPHABET[idx] as char
        })
        .collect()
}

/// The camp sizes an admin can pick. The two 4-player sizes are for testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CampSize {
    TwentyFour,
    Thirty,
    FourOneVOne,
    FourTwoVTwo,
}

impl CampSize {
    /// `24`, `30`, `4-1v1` or `4-2v2`, as the `/camp iniciar` choices read.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "24" => Some(Self::TwentyFour),
            "30" => Some(Self::Thirty),
            "4-1v1" => Some(Self::FourOneVOne),
            "4-2v2" => Some(Self::FourTwoVTwo),
            _ => None,
        }
    }

    pub fn target_size(self) -> i64 {
        match self {
            Self::FourOneVOne | Self::FourTwoVTwo => 4,
            Self::TwentyFour => 24,
            Self::Thirty => 30,
        }
    }

    pub fn match_size(self) -> i64 {
        match self {
            Self::FourOneVOne => 2,
            Self::FourTwoVTwo => 4,
            Self::TwentyFour | Self::Thirty => 12,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampLayout {
    pub match_size: usize,
    pub refresh_votes: usize,
}

impl CampLayout {
    pub fn from_match_size(match_size: i64) -> Self {
        match match_size {
            2 => Self {
                match_size: 2,
                refresh_votes: 1,
            },
            4 => Self {
                match_size: 4,
                refresh_votes: 2,
            },
            _ => Self {
                match_size: 12,
                refresh_votes: 4,
            },
        }
    }
}

pub fn shuffle_ids(ids: &mut [i64], rng: &mut impl Rng) {
    ids.shuffle(rng);
}

/// Top `n` players by points. Equal scores are shuffled so ties for the last seats are fair.
pub fn select_finalists(players: &[(i64, i64)], n: usize, rng: &mut impl Rng) -> Vec<i64> {
    let mut by_points: std::collections::BTreeMap<i64, Vec<i64>> =
        std::collections::BTreeMap::new();
    for (user_id, points) in players {
        by_points.entry(*points).or_default().push(*user_id);
    }
    let mut result = Vec::with_capacity(n.min(players.len()));
    for (_points, mut ids) in by_points.into_iter().rev() {
        ids.shuffle(rng);
        for user_id in ids {
            result.push(user_id);
            if result.len() == n {
                return result;
            }
        }
    }
    result
}

/// S-rank players pulled into each new match: one per team.
const S_RANK_PER_MATCH: usize = 2;

/// A player being seated in a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seat {
    pub user_id: i64,
    pub points: i64,
    pub s_rank: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamSplit {
    pub hiddenking: Vec<i64>,
    pub archmother: Vec<i64>,
    pub hiddenking_captain: Option<i64>,
    pub archmother_captain: Option<i64>,
}

/// Pick who leaves the queue for the next match: the first two queued S-rank players, then the
/// other seats in queue order. Extra S-rank players keep their place in line unless there are not
/// enough regular players left to fill the match.
pub fn pick_match_seats(queued: &[Seat], match_size: usize) -> Vec<Seat> {
    let mut picked: Vec<Seat> = queued
        .iter()
        .filter(|s| s.s_rank)
        .take(S_RANK_PER_MATCH.min(match_size))
        .copied()
        .collect();
    let open = match_size - picked.len();
    picked.extend(queued.iter().filter(|s| !s.s_rank).take(open).copied());
    if picked.len() < match_size {
        let open = match_size - picked.len();
        let extra: Vec<Seat> = queued
            .iter()
            .filter(|s| !picked.iter().any(|p| p.user_id == s.user_id))
            .take(open)
            .copied()
            .collect();
        picked.extend(extra);
    }
    picked
}

/// Highest points first. Equal scores are shuffled.
fn rank_by_points(seats: Vec<Seat>, rng: &mut impl Rng) -> Vec<Seat> {
    let mut by_points: std::collections::BTreeMap<i64, Vec<Seat>> =
        std::collections::BTreeMap::new();
    for seat in seats {
        by_points.entry(seat.points).or_default().push(seat);
    }
    let mut ranked = Vec::new();
    for (_points, mut group) in by_points.into_iter().rev() {
        group.shuffle(rng);
        ranked.extend(group);
    }
    ranked
}

fn ordered(mut seats: Vec<Seat>, balanced: bool, rng: &mut impl Rng) -> Vec<Seat> {
    if balanced {
        rank_by_points(seats, rng)
    } else {
        seats.shuffle(rng);
        seats
    }
}

/// Snake: A B B A A B B A … so 1st/2nd, 3rd/4th, … land on opposite teams.
fn snake(ranked: Vec<Seat>) -> (Vec<Seat>, Vec<Seat>) {
    let mut team_a = Vec::new();
    let mut team_b = Vec::new();
    for (i, seat) in ranked.into_iter().enumerate() {
        if i % 4 == 0 || i % 4 == 3 {
            team_a.push(seat);
        } else {
            team_b.push(seat);
        }
    }
    (team_a, team_b)
}

fn total_points(seats: &[Seat]) -> i64 {
    seats.iter().map(|s| s.points).sum()
}

/// The team's S-rank player is captain. With none (or several), the most points wins; ties are random.
fn pick_captain(team: &[Seat], rng: &mut impl Rng) -> Option<i64> {
    let s_rank: Vec<&Seat> = team.iter().filter(|s| s.s_rank).collect();
    let pool: Vec<&Seat> = if s_rank.is_empty() {
        team.iter().collect()
    } else {
        s_rank
    };
    let best = pool.iter().map(|s| s.points).max()?;
    let top: Vec<i64> = pool
        .iter()
        .filter(|s| s.points == best)
        .map(|s| s.user_id)
        .collect();
    top.choose(rng).copied()
}

/// Split a match roster into Hiddenking and Archmother and name each team's captain.
///
/// S-rank players are dealt first, alternating teams, so two of them only share a team when the
/// match has more than two. `balanced` (final match) snake-drafts everyone else by points with ties
/// shuffled; otherwise they are shuffled. A coin flip decides which side is Hiddenking.
pub fn split_match(seats: &[Seat], balanced: bool, rng: &mut impl Rng) -> TeamSplit {
    let half = seats.len() / 2;
    let (s_rank, others): (Vec<Seat>, Vec<Seat>) = seats.iter().copied().partition(|s| s.s_rank);
    let (mut team_a, mut team_b) = snake(ordered(s_rank, balanced, rng));
    let a_open = half.saturating_sub(team_a.len());

    let rest = ordered(others, balanced, rng);
    let (rest_a, rest_b) = if balanced {
        // The snake halves differ by at most one seat, and so do the open seats. Of the pairings
        // that fill both teams, keep the one with the closer point totals.
        let (x, y) = snake(rest);
        let gap = |a: &[Seat], b: &[Seat]| {
            (total_points(&team_a) + total_points(a) - total_points(&team_b) - total_points(b)).abs()
        };
        let keep_ok = x.len() == a_open;
        let swap_ok = y.len() == a_open;
        if swap_ok && (!keep_ok || gap(&y, &x) < gap(&x, &y)) {
            (y, x)
        } else {
            (x, y)
        }
    } else {
        let mut rest = rest;
        let rest_b = rest.split_off(a_open.min(rest.len()));
        (rest, rest_b)
    };
    team_a.extend(rest_a);
    team_b.extend(rest_b);

    if rng.gen_bool(0.5) {
        std::mem::swap(&mut team_a, &mut team_b);
    }
    let hiddenking_captain = pick_captain(&team_a, rng);
    let archmother_captain = pick_captain(&team_b, rng);
    TeamSplit {
        hiddenking: team_a.iter().map(|s| s.user_id).collect(),
        archmother: team_b.iter().map(|s| s.user_id).collect(),
        hiddenking_captain,
        archmother_captain,
    }
}

fn shuffle_if_many<T>(items: &mut [T], rng: &mut impl Rng) {
    if items.len() > 2 {
        items.shuffle(rng);
    }
}

/// Rebuild the queue after a match ends.
///
/// Waiters (just_won = None) are reset to consecutive 0 and keep their current order at the front.
/// Finished players are grouped by consecutive_matches (already incremented), winners before losers
/// inside each count. A winner or loser block with more than 2 players is shuffled.
pub fn requeue(waiters: Vec<QueuePlayer>, finished: Vec<QueuePlayer>, rng: &mut impl Rng) -> Vec<QueuePlayer> {
    let mut waiters = waiters;
    for waiter in &mut waiters {
        waiter.consecutive_matches = 0;
        waiter.just_won = None;
    }

    let mut by_count: std::collections::BTreeMap<i64, (Vec<QueuePlayer>, Vec<QueuePlayer>)> =
        std::collections::BTreeMap::new();
    for player in finished {
        let entry = by_count.entry(player.consecutive_matches).or_default();
        match player.just_won {
            Some(true) => entry.0.push(player),
            _ => entry.1.push(player),
        }
    }

    let mut result = waiters;
    for (_count, (mut winners, mut losers)) in by_count {
        shuffle_if_many(&mut winners, rng);
        shuffle_if_many(&mut losers, rng);
        result.extend(winners);
        result.extend(losers);
    }

    for (index, player) in result.iter_mut().enumerate() {
        player.queue_position = index as i64;
        player.just_won = None;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn seat(user_id: i64, points: i64, s_rank: bool) -> Seat {
        Seat {
            user_id,
            points,
            s_rank,
        }
    }

    fn ids(seats: &[Seat]) -> Vec<i64> {
        seats.iter().map(|s| s.user_id).collect()
    }

    #[test]
    fn pick_takes_first_two_s_rank_then_queue_order() {
        let queued: Vec<Seat> = (0..16).map(|i| seat(i, 0, [5, 13, 14].contains(&i))).collect();
        let mut picked = ids(&pick_match_seats(&queued, 12));
        picked.sort();
        // 5 and 13 are the first two S-rank; 14 keeps waiting. Ten regulars fill the rest.
        assert_eq!(picked, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 13]);
    }

    #[test]
    fn pick_seats_extra_s_rank_when_regulars_run_out() {
        let queued: Vec<Seat> = (0..12).map(|i| seat(i, 0, i < 3)).collect();
        let mut picked = ids(&pick_match_seats(&queued, 12));
        picked.sort();
        assert_eq!(picked, (0..12).collect::<Vec<_>>());
    }

    #[test]
    fn pick_without_s_rank_keeps_queue_order() {
        let queued: Vec<Seat> = (0..14).map(|i| seat(i, 0, false)).collect();
        assert_eq!(ids(&pick_match_seats(&queued, 12)), (0..12).collect::<Vec<_>>());
    }

    #[test]
    fn split_keeps_teams_even_and_s_rank_apart() {
        for match_size in [2_i64, 4, 12] {
            for s_count in 0..=match_size.min(4) {
                for balanced in [false, true] {
                    for seed in 0..50 {
                        let mut rng = StdRng::seed_from_u64(seed);
                        let seats: Vec<Seat> = (0..match_size)
                            .map(|i| seat(i, (i * 7) % 5, i < s_count))
                            .collect();
                        let split = split_match(&seats, balanced, &mut rng);
                        let half = (match_size / 2) as usize;
                        assert_eq!(split.hiddenking.len(), half);
                        assert_eq!(split.archmother.len(), half);

                        let hk_s = split.hiddenking.iter().filter(|id| **id < s_count).count();
                        let am_s = split.archmother.iter().filter(|id| **id < s_count).count();
                        assert!(hk_s.abs_diff(am_s) <= 1, "s_rank {hk_s} vs {am_s}");

                        let hk_cap = split.hiddenking_captain.unwrap();
                        let am_cap = split.archmother_captain.unwrap();
                        assert!(split.hiddenking.contains(&hk_cap));
                        assert!(split.archmother.contains(&am_cap));
                        if hk_s > 0 {
                            assert!(hk_cap < s_count, "S-rank player should captain Hiddenking");
                        }
                        if am_s > 0 {
                            assert!(am_cap < s_count, "S-rank player should captain Archmother");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn captain_without_s_rank_has_most_points() {
        for seed in 0..50 {
            let mut rng = StdRng::seed_from_u64(seed);
            let seats: Vec<Seat> = (0..12).map(|i| seat(i, i, i == 0)).collect();
            let split = split_match(&seats, false, &mut rng);
            let (team, captain) = if split.hiddenking.contains(&0) {
                (&split.archmother, split.archmother_captain)
            } else {
                (&split.hiddenking, split.hiddenking_captain)
            };
            assert_eq!(captain, team.iter().copied().max());
        }
    }

    #[test]
    fn balanced_split_without_s_rank_is_snake_draft() {
        let seats: Vec<Seat> = (0..12).map(|i| seat(i, 12 - i, false)).collect();
        let split = split_match(&seats, true, &mut StdRng::seed_from_u64(1));
        let mut a = split.hiddenking.clone();
        let mut b = split.archmother.clone();
        a.sort();
        b.sort();
        let snake_a = vec![0, 3, 4, 7, 8, 11];
        let snake_b = vec![1, 2, 5, 6, 9, 10];
        assert!((a == snake_a && b == snake_b) || (a == snake_b && b == snake_a));
    }
}
