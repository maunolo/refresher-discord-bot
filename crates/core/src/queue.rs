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
    /// `24`, `30`, `4-1v1` or `4-2v2`, as the admin page sends them.
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

/// The queue when a camp starts: the roster shuffled into the camp's teams, `team_size` places of
/// the line each, with S-rank players spread over the teams as evenly as possible (which teams get
/// one more is random). The first match takes the first two teams, the next match the two after.
pub fn starting_queue(players: &[(i64, bool)], team_size: usize, rng: &mut impl Rng) -> Vec<i64> {
    let teams = players.len().div_ceil(team_size.max(1));
    if teams == 0 {
        return Vec::new();
    }
    let mut s_rank: Vec<i64> = players.iter().filter(|p| p.1).map(|p| p.0).collect();
    let mut others: Vec<i64> = players.iter().filter(|p| !p.1).map(|p| p.0).collect();
    s_rank.shuffle(rng);
    others.shuffle(rng);

    let mut per_team = vec![s_rank.len() / teams; teams];
    let mut one_more: Vec<usize> = (0..teams).collect();
    one_more.shuffle(rng);
    for team in one_more.into_iter().take(s_rank.len() % teams) {
        per_team[team] += 1;
    }

    let mut order = Vec::with_capacity(players.len());
    for count in per_team {
        let mut team: Vec<i64> = s_rank.drain(..count).collect();
        let fill = team_size.saturating_sub(count).min(others.len());
        team.extend(others.drain(..fill));
        team.shuffle(rng);
        order.extend(team);
    }
    order
}

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
    pub hiddenking_captains: Vec<i64>,
    pub archmother_captains: Vec<i64>,
}

/// Who leaves the queue for the next match: the first `match_size` of `queued`, in queue order.
/// S rank gives no priority; `starting_queue` spreads S-rank players over the camp's teams instead.
pub fn pick_match_seats(queued: &[Seat], match_size: usize) -> Vec<Seat> {
    queued.iter().take(match_size).copied().collect()
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

/// Every S-rank player on the team is a captain. With none, the one with the most points; ties are random.
fn pick_captains(team: &[Seat], rng: &mut impl Rng) -> Vec<i64> {
    let s_rank: Vec<i64> = team.iter().filter(|s| s.s_rank).map(|s| s.user_id).collect();
    if !s_rank.is_empty() {
        return s_rank;
    }
    let Some(best) = team.iter().map(|s| s.points).max() else {
        return Vec::new();
    };
    let top: Vec<i64> = team
        .iter()
        .filter(|s| s.points == best)
        .map(|s| s.user_id)
        .collect();
    top.choose(rng).copied().into_iter().collect()
}

/// Split a match roster into Hiddenking and Archmother and name each team's captains.
///
/// S-rank players are dealt first, alternating teams, so the two teams' counts differ by one at
/// most. `balanced` (final match) snake-drafts everyone else by points with ties shuffled;
/// otherwise they are shuffled. A coin flip decides which side is Hiddenking.
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
    let hiddenking_captains = pick_captains(&team_a, rng);
    let archmother_captains = pick_captains(&team_b, rng);
    TeamSplit {
        hiddenking: team_a.iter().map(|s| s.user_id).collect(),
        archmother: team_b.iter().map(|s| s.user_id).collect(),
        hiddenking_captains,
        archmother_captains,
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

    /// S-rank players in each team of the starting queue, in line order.
    fn s_rank_per_team(order: &[i64], s_rank: &[i64], team_size: usize) -> Vec<usize> {
        order
            .chunks(team_size)
            .map(|team| team.iter().filter(|id| s_rank.contains(id)).count())
            .collect()
    }

    #[test]
    fn starting_queue_spreads_s_rank_over_the_teams() {
        // 30 players, 8 S rank, teams of 6: three teams get 2, two get 1, and which ones varies.
        let s_rank: Vec<i64> = (0..8).collect();
        let players: Vec<(i64, bool)> = (0..30).map(|i| (i, i < 8)).collect();
        let mut layouts = std::collections::HashSet::new();
        for seed in 0..50 {
            let order = starting_queue(&players, 6, &mut StdRng::seed_from_u64(seed));
            let mut sorted = order.clone();
            sorted.sort();
            assert_eq!(sorted, (0..30).collect::<Vec<_>>(), "everyone once");
            let per_team = s_rank_per_team(&order, &s_rank, 6);
            let mut counts = per_team.clone();
            counts.sort();
            assert_eq!(counts, [1, 1, 2, 2, 2]);
            layouts.insert(per_team);
        }
        assert!(layouts.len() > 1, "the teams with one more change from camp to camp");
    }

    #[test]
    fn starting_queue_handles_every_camp_size() {
        for (size, team_size, s_count, expected) in [
            (24, 6, 4, vec![1, 1, 1, 1]),
            (24, 6, 0, vec![0, 0, 0, 0]),
            (30, 6, 10, vec![2, 2, 2, 2, 2]),
            (4, 1, 2, vec![0, 0, 1, 1]),
            (4, 2, 2, vec![1, 1]),
            (4, 2, 4, vec![2, 2]),
        ] {
            let s_rank: Vec<i64> = (0..s_count).collect();
            let players: Vec<(i64, bool)> = (0..size).map(|i| (i, i < s_count)).collect();
            let order = starting_queue(&players, team_size, &mut StdRng::seed_from_u64(7));
            assert_eq!(order.len(), size as usize);
            let mut counts = s_rank_per_team(&order, &s_rank, team_size);
            counts.sort();
            assert_eq!(counts, expected, "{size} players, teams of {team_size}, {s_count} S rank");
        }
    }

    #[test]
    fn pick_takes_the_front_of_the_queue_s_rank_or_not() {
        let queued: Vec<Seat> = (0..16).map(|i| seat(i, 0, [5, 13, 14].contains(&i))).collect();
        assert_eq!(ids(&pick_match_seats(&queued, 12)), (0..12).collect::<Vec<_>>());
    }

    #[test]
    fn split_keeps_teams_even_and_s_rank_spread() {
        for match_size in [2_i64, 4, 12] {
            for s_count in 0..=match_size {
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

                        for (team, captains, s_on_team) in [
                            (&split.hiddenking, &split.hiddenking_captains, hk_s),
                            (&split.archmother, &split.archmother_captains, am_s),
                        ] {
                            if s_on_team > 0 {
                                let mut s_rank: Vec<i64> =
                                    team.iter().copied().filter(|id| *id < s_count).collect();
                                let mut captains = captains.clone();
                                s_rank.sort();
                                captains.sort();
                                assert_eq!(captains, s_rank, "every S-rank player on the team captains it");
                            } else {
                                assert_eq!(captains.len(), 1);
                                assert!(team.contains(&captains[0]));
                            }
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
            let (team, captains) = if split.hiddenking.contains(&0) {
                (&split.archmother, &split.archmother_captains)
            } else {
                (&split.hiddenking, &split.hiddenking_captains)
            };
            assert_eq!(captains, &[*team.iter().max().unwrap()]);
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
