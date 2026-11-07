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
/// the line each. S-rank players are spread over the teams as evenly as possible (which teams get
/// one more is random), and so is each role, so every team has one of each where the roster
/// allows. The first match takes the first two teams, the next match the two after.
pub fn starting_queue(players: &[Seat], team_size: usize, rng: &mut impl Rng) -> Vec<i64> {
    let team_size = team_size.max(1);
    let teams = players.len().div_ceil(team_size);
    if teams == 0 {
        return Vec::new();
    }
    let size: Vec<usize> = (0..teams)
        .map(|team| team_size.min(players.len() - team * team_size))
        .collect();
    let (s_rank, others): (Vec<Seat>, Vec<Seat>) = players.iter().copied().partition(|p| p.s_rank);

    let each = s_rank.len() / teams;
    let mut s_rank_places = vec![each; teams];
    let mut one_more: Vec<usize> = (0..teams).filter(|team| size[*team] > each).collect();
    one_more.shuffle(rng);
    for team in one_more.into_iter().take(s_rank.len() % teams) {
        s_rank_places[team] += 1;
    }
    let other_places: Vec<usize> = (0..teams)
        .map(|team| size[team].saturating_sub(s_rank_places[team]))
        .collect();

    let mut lineup: Vec<Vec<Seat>> = vec![Vec::new(); teams];
    deal(&mut lineup, s_rank, s_rank_places, rng);
    deal(&mut lineup, others, other_places, rng);
    lineup
        .into_iter()
        .flat_map(|mut team| {
            team.shuffle(rng);
            team.into_iter().map(|s| s.user_id)
        })
        .collect()
}

/// Seats `players` on `teams`, `places[t]` more on team `t`, spreading each role: the rarest role
/// first, each player on a team with room and the fewest of their role so far (ties at random).
/// Players without a role take the places left.
fn deal(teams: &mut [Vec<Seat>], players: Vec<Seat>, mut places: Vec<usize>, rng: &mut impl Rng) {
    let mut roles = Role::ALL.to_vec();
    roles.shuffle(rng);
    // Stable: equally rare roles keep the shuffled order.
    roles.sort_by_key(|role| players.iter().filter(|p| p.role == Some(*role)).count());
    let mut order: Vec<Seat> = Vec::with_capacity(players.len());
    for role in roles.into_iter().map(Some).chain([None]) {
        let mut group: Vec<Seat> = players.iter().copied().filter(|p| p.role == role).collect();
        group.shuffle(rng);
        order.extend(group);
    }
    for player in order {
        let with_room = (0..teams.len()).filter(|team| places[*team] > 0);
        let fewest = |team: &usize| match player.role {
            Some(role) => teams[*team].iter().filter(|s| s.role == Some(role)).count(),
            None => 0,
        };
        let least = with_room.clone().map(|team| fewest(&team)).min();
        let candidates: Vec<usize> = with_room.filter(|team| Some(fewest(team)) == least).collect();
        // Places always add up to the players; the fallback only guards against losing anyone.
        let team = candidates.choose(rng).copied().unwrap_or(teams.len() - 1);
        teams[team].push(player);
        places[team] = places[team].saturating_sub(1);
    }
}

/// A player's main role, set per server. Splitting a match tries to give each team one of each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// Tank or bruiser.
    Tank,
    Carry,
    /// Utility or support.
    Support,
}

impl Role {
    pub const ALL: [Role; 3] = [Role::Tank, Role::Carry, Role::Support];

    /// `tank`, `carry` or `support`, as stored and as the admin page sends it.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Tank => "tank",
            Role::Carry => "carry",
            Role::Support => "support",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Role::ALL.into_iter().find(|role| role.as_str() == name)
    }
}

/// A player being seated in a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seat {
    pub user_id: i64,
    pub points: i64,
    pub s_rank: bool,
    pub role: Option<Role>,
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

/// A player a refresh can seat: one of the match's or one waiting in the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub seat: Seat,
    pub consecutive_matches: i64,
}

/// Who plays a match after its refresh, as many as `playing`. Match players with fewer
/// consecutive matches than the match's highest keep their seat: they were waiting. The other
/// seats are drawn at random among the match players at that highest count and the queued players
/// with no more than it, so a refresh never brings in someone who played more in a row.
pub fn refresh_roster(playing: &[Candidate], queued: &[Candidate], rng: &mut impl Rng) -> Vec<Seat> {
    let Some(highest) = playing.iter().map(|c| c.consecutive_matches).max() else {
        return Vec::new();
    };
    let (kept, drawn): (Vec<&Candidate>, Vec<&Candidate>) =
        playing.iter().partition(|c| c.consecutive_matches < highest);
    let pool: Vec<Seat> = drawn
        .into_iter()
        .chain(queued.iter().filter(|c| c.consecutive_matches <= highest))
        .map(|c| c.seat)
        .collect();
    let mut roster: Vec<Seat> = kept.into_iter().map(|c| c.seat).collect();
    roster.extend(pool.choose_multiple(rng, playing.len() - roster.len()).copied());
    roster
}

/// Roles of `Role::ALL` nobody on the team plays.
fn missing_roles(team: &[Seat]) -> usize {
    Role::ALL
        .iter()
        .filter(|role| !team.iter().any(|s| s.role == Some(**role)))
        .count()
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
/// Of every way to split the roster in half, it keeps those where the teams' S-rank counts
/// differ by one at most. Among them, the ones where the teams miss the fewest roles (each team
/// should have a tank or bruiser, a carry and a utility or support), the worse-off team counting
/// next. `balanced` (final match) then keeps the closest point totals. One of those that are left
/// is picked at random, sides included. A match has 12 players at most, 924 splits.
pub fn split_match(seats: &[Seat], balanced: bool, rng: &mut impl Rng) -> TeamSplit {
    let half = seats.len() / 2;
    let s_rank = seats.iter().filter(|s| s.s_rank).count();
    // Bit `i` of a mask puts `seats[i]` on the first team.
    let teams = |mask: u32| -> (Vec<Seat>, Vec<Seat>) {
        let (a, b): (Vec<(usize, &Seat)>, Vec<(usize, &Seat)>) =
            seats.iter().enumerate().partition(|(i, _)| mask & (1 << i) != 0);
        (a.into_iter().map(|(_, s)| *s).collect(), b.into_iter().map(|(_, s)| *s).collect())
    };
    let mut best: Vec<u32> = Vec::new();
    let mut best_score = None;
    for mask in 0u32..1 << seats.len() {
        if mask.count_ones() as usize != half {
            continue;
        }
        let (team_a, team_b) = teams(mask);
        let a_s_rank = team_a.iter().filter(|s| s.s_rank).count();
        if a_s_rank.abs_diff(s_rank - a_s_rank) > 1 {
            continue;
        }
        let (a_missing, b_missing) = (missing_roles(&team_a), missing_roles(&team_b));
        let gap = if balanced {
            let total = |team: &[Seat]| team.iter().map(|s| s.points).sum::<i64>();
            (total(&team_a) - total(&team_b)).abs()
        } else {
            0
        };
        let score = (a_missing + b_missing, a_missing.max(b_missing), gap);
        if best_score.is_none_or(|best| score < best) {
            best_score = Some(score);
            best.clear();
        }
        if best_score == Some(score) {
            best.push(mask);
        }
    }
    let (team_a, team_b) = teams(best[rng.gen_range(0..best.len())]);

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
            role: None,
        }
    }

    fn with_role(seat: Seat, role: Role) -> Seat {
        Seat { role: Some(role), ..seat }
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
        let players: Vec<Seat> = (0..30).map(|i| seat(i, 0, i < 8)).collect();
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
            let players: Vec<Seat> = (0..size).map(|i| seat(i, 0, i < s_count)).collect();
            let order = starting_queue(&players, team_size, &mut StdRng::seed_from_u64(7));
            assert_eq!(order.len(), size as usize);
            let mut counts = s_rank_per_team(&order, &s_rank, team_size);
            counts.sort();
            assert_eq!(counts, expected, "{size} players, teams of {team_size}, {s_count} S rank");
        }
    }

    #[test]
    fn starting_queue_gives_every_team_each_role() {
        // 30 players in 5 teams of 6: exactly 5 tanks and 5 supports, 12 carries, 8 without a
        // role. S rank: 3 tanks, 3 supports and 2 without a role.
        let role = |i: i64| match i {
            0..5 => Some(Role::Tank),
            5..10 => Some(Role::Support),
            10..22 => Some(Role::Carry),
            _ => None,
        };
        let s_rank = [0, 1, 2, 5, 6, 7, 22, 23];
        let players: Vec<Seat> = (0..30)
            .map(|i| Seat { role: role(i), ..seat(i, 0, s_rank.contains(&i)) })
            .collect();
        let mut layouts = std::collections::HashSet::new();
        for seed in 0..100 {
            let order = starting_queue(&players, 6, &mut StdRng::seed_from_u64(seed));
            let mut sorted = order.clone();
            sorted.sort();
            assert_eq!(sorted, (0..30).collect::<Vec<_>>(), "everyone once");
            for team in order.chunks(6) {
                for wanted in Role::ALL {
                    assert!(team.iter().any(|id| role(*id) == Some(wanted)), "{wanted:?} in {team:?}");
                }
            }
            let mut counts = s_rank_per_team(&order, &s_rank, 6);
            counts.sort();
            assert_eq!(counts, [1, 1, 2, 2, 2], "S rank still even");
            layouts.insert(order);
        }
        assert!(layouts.len() > 90, "still random");
    }

    #[test]
    fn starting_queue_spreads_a_short_role_one_per_team() {
        // Three tanks for five teams: three teams get one, none gets two.
        let players: Vec<Seat> = (0..30)
            .map(|i| if i < 3 { with_role(seat(i, 0, false), Role::Tank) } else { seat(i, 0, false) })
            .collect();
        for seed in 0..50 {
            let order = starting_queue(&players, 6, &mut StdRng::seed_from_u64(seed));
            let tanks: Vec<usize> = order.chunks(6).map(|team| team.iter().filter(|id| **id < 3).count()).collect();
            assert!(tanks.iter().all(|n| *n <= 1), "{tanks:?}");
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

    fn candidate(user_id: i64, consecutive_matches: i64) -> Candidate {
        Candidate {
            seat: seat(user_id, 0, false),
            consecutive_matches,
        }
    }

    #[test]
    fn refresh_keeps_waiters_and_draws_the_rest_from_match_and_queue() {
        // Match: 0-5 waited (0 in a row), 6-11 just played (1). Queue: 12-17 just played (1),
        // 18-19 played two in a row (2).
        let playing: Vec<Candidate> = (0..12).map(|i| candidate(i, i64::from(i >= 6))).collect();
        let queued: Vec<Candidate> = (12..20).map(|i| candidate(i, if i < 18 { 1 } else { 2 })).collect();
        let mut drawn_in = std::collections::HashSet::new();
        for seed in 0..100 {
            let roster = refresh_roster(&playing, &queued, &mut StdRng::seed_from_u64(seed));
            let mut ids = ids(&roster);
            ids.sort();
            assert_eq!(ids.len(), 12);
            ids.dedup();
            assert_eq!(ids.len(), 12, "nobody twice");
            assert!((0..6).all(|id| ids.contains(&id)), "waiters keep their seat");
            assert!(ids.iter().all(|id| *id < 18), "nobody with more in a row than the match");
            drawn_in.extend(ids.into_iter().filter(|id| *id >= 6));
        }
        assert_eq!(drawn_in.len(), 12, "every player at the match's count can be drawn");
    }

    #[test]
    fn refresh_without_eligible_queue_reshuffles_the_same_players() {
        let playing: Vec<Candidate> = (0..4).map(|i| candidate(i, 1)).collect();
        let queued = [candidate(4, 2), candidate(5, 3)];
        let mut roster = ids(&refresh_roster(&playing, &queued, &mut StdRng::seed_from_u64(3)));
        roster.sort();
        assert_eq!(roster, [0, 1, 2, 3]);
        assert!(refresh_roster(&playing, &[], &mut StdRng::seed_from_u64(3)).len() == 4);
    }

    fn team_points(seats: &[Seat], team: &[i64]) -> i64 {
        seats.iter().filter(|s| team.contains(&s.user_id)).map(|s| s.points).sum()
    }

    fn team_roles(seats: &[Seat], team: &[i64]) -> Vec<Role> {
        let mut roles: Vec<Role> = seats
            .iter()
            .filter(|s| team.contains(&s.user_id))
            .filter_map(|s| s.role)
            .collect();
        roles.sort_by_key(|r| r.as_str());
        roles.dedup();
        roles
    }

    #[test]
    fn balanced_split_closes_the_point_gap() {
        // 12 down to 1 point: 78 in all, so two teams of 39.
        let seats: Vec<Seat> = (0..12).map(|i| seat(i, 12 - i, false)).collect();
        for seed in 0..20 {
            let split = split_match(&seats, true, &mut StdRng::seed_from_u64(seed));
            assert_eq!(team_points(&seats, &split.hiddenking), 39);
            assert_eq!(team_points(&seats, &split.archmother), 39);
        }
    }

    #[test]
    fn every_team_gets_each_role_when_the_match_has_two() {
        // Two of each role, three of them S rank, and six players without a role.
        let roles = [Role::Tank, Role::Tank, Role::Carry, Role::Carry, Role::Support, Role::Support];
        let seats: Vec<Seat> = (0..12)
            .map(|i| {
                let seat = seat(i, (i * 5) % 7, [0, 2, 4].contains(&i));
                roles.get(i as usize).map_or(seat, |role| with_role(seat, *role))
            })
            .collect();
        for balanced in [false, true] {
            let mut layouts = std::collections::HashSet::new();
            for seed in 0..50 {
                let split = split_match(&seats, balanced, &mut StdRng::seed_from_u64(seed));
                assert_eq!(team_roles(&seats, &split.hiddenking).len(), 3, "{split:?}");
                assert_eq!(team_roles(&seats, &split.archmother).len(), 3, "{split:?}");
                let hk_s = split.hiddenking.iter().filter(|id| [0, 2, 4].contains(*id)).count();
                assert!(hk_s == 1 || hk_s == 2, "S rank still split evenly");
                let mut hk = split.hiddenking.clone();
                hk.sort();
                layouts.insert(hk);
            }
            assert!(layouts.len() > 1, "still random among the good splits");
        }
    }

    #[test]
    fn a_missing_role_is_missed_by_one_team_only_and_shared_out() {
        // One tank and one support: each team gets one of the two, not both on one side.
        let seats: Vec<Seat> = (0..4)
            .map(|i| match i {
                0 => with_role(seat(i, 0, false), Role::Tank),
                1 => with_role(seat(i, 0, false), Role::Support),
                _ => with_role(seat(i, 0, false), Role::Carry),
            })
            .collect();
        for seed in 0..20 {
            let split = split_match(&seats, false, &mut StdRng::seed_from_u64(seed));
            assert_eq!(team_roles(&seats, &split.hiddenking).len(), 2, "{split:?}");
            assert_eq!(team_roles(&seats, &split.archmother).len(), 2, "{split:?}");
        }
    }

    #[test]
    fn roles_come_before_points_in_the_final() {
        // The only split with a tank on each side is 50 points apart; it still wins.
        let seats = vec![
            with_role(seat(0, 50, false), Role::Tank),
            with_role(seat(1, 0, false), Role::Tank),
            seat(2, 50, false),
            seat(3, 0, false),
        ];
        for seed in 0..20 {
            let split = split_match(&seats, true, &mut StdRng::seed_from_u64(seed));
            let (a, b) = (team_points(&seats, &split.hiddenking), team_points(&seats, &split.archmother));
            assert_eq!(a.abs_diff(b), 0, "{split:?}");
        }
    }
}
