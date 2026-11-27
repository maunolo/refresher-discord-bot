use rand::seq::SliceRandom;
use rand::Rng;

const CAMP_ID_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuePlayer {
    pub user_id: i64,
    /// Finished matches in the camp, the one being requeued included.
    pub matches_played: i64,
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
    /// Refresh votes that re-sort a match, from either team: half its players.
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
                refresh_votes: 6,
            },
        }
    }
}

/// A player the finals can seat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Finalist {
    pub user_id: i64,
    pub points: i64,
    /// Finished matches as MVP: the ranking's tie-breaker for equal points.
    pub mvps: i64,
    /// Finished matches in the camp: fewer ranks higher among equal points and MVPs.
    pub matches_played: i64,
    /// Finished matches won: the ranking's last tie-breaker, more first.
    pub wins: i64,
    /// Played the camp's latest finals match.
    pub played_last_final: bool,
    /// `None` while in a match.
    pub queue_position: Option<i64>,
}

/// The top `n` of the ranking: by points, then MVPs, then fewer matches played, then wins. Among
/// players equal on all four, whoever played the latest finals match goes first, so the finals keep their
/// players from one match to the next; then the queue's order, players in a match last.
pub fn select_finalists(players: &[Finalist], n: usize) -> Vec<i64> {
    let mut sorted = players.to_vec();
    sorted.sort_by_key(|p| {
        (
            std::cmp::Reverse(p.points),
            std::cmp::Reverse(p.mvps),
            p.matches_played,
            std::cmp::Reverse(p.wins),
            !p.played_last_final,
            p.queue_position.unwrap_or(i64::MAX),
            p.user_id,
        )
    });
    sorted.into_iter().take(n).map(|p| p.user_id).collect()
}

/// The queue when a camp starts: the roster shuffled into the camp's teams, `team_size` places of
/// the line each. SS-rank players are spread over the teams as evenly as possible (which teams get
/// one more is random), then S-rank players, and so is each role, so every team has one of each
/// where the roster allows. The first match takes the first two teams, the next match the two after.
pub fn starting_queue(players: &[Seat], team_size: usize, rng: &mut impl Rng) -> Vec<i64> {
    let team_size = team_size.max(1);
    let teams = players.len().div_ceil(team_size);
    if teams == 0 {
        return Vec::new();
    }
    let size: Vec<usize> = (0..teams)
        .map(|team| team_size.min(players.len() - team * team_size))
        .collect();
    let (ss_rank, rest): (Vec<Seat>, Vec<Seat>) = players.iter().copied().partition(|p| p.ss_rank);
    let (s_rank, others): (Vec<Seat>, Vec<Seat>) = rest.into_iter().partition(|p| p.s_rank);

    let ss_rank_places = spread(ss_rank.len(), &size, &vec![0; teams], rng);
    let room: Vec<usize> = (0..teams).map(|team| size[team] - ss_rank_places[team]).collect();
    // Teams with one SS rank fewer get the S rank first.
    let s_rank_places = spread(s_rank.len(), &room, &ss_rank_places, rng);
    let other_places: Vec<usize> = (0..teams)
        .map(|team| room[team].saturating_sub(s_rank_places[team]))
        .collect();

    let mut lineup: Vec<Vec<Seat>> = vec![Vec::new(); teams];
    deal(&mut lineup, ss_rank, ss_rank_places, rng);
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

/// How many of `count` players each team gets, as evenly as `room` allows: one at a time to a team
/// with room and the fewest so far, `already` (top players placed before) included; ties at random.
fn spread(count: usize, room: &[usize], already: &[usize], rng: &mut impl Rng) -> Vec<usize> {
    let mut places = vec![0; room.len()];
    for _ in 0..count {
        let open = (0..room.len()).filter(|team| places[*team] < room[*team]);
        let so_far = |team: usize| already[team] + places[team];
        let Some(fewest) = open.clone().map(so_far).min() else {
            break;
        };
        let candidates: Vec<usize> = open.filter(|team| so_far(*team) == fewest).collect();
        if let Some(team) = candidates.choose(rng) {
            places[*team] += 1;
        }
    }
    places
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
    /// Above S rank: captains their team, two a team at most.
    pub ss_rank: bool,
    pub role: Option<Role>,
}

/// The most SS-rank players a team of a match takes.
pub const MAX_SS_RANK_PER_TEAM: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamSplit {
    pub hiddenking: Vec<i64>,
    pub archmother: Vec<i64>,
    pub hiddenking_captains: Vec<i64>,
    pub archmother_captains: Vec<i64>,
}

/// A player a match can seat, from the queue or, in a refresh, from the match itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub seat: Seat,
    /// Finished matches in the camp: fewer plays first.
    pub matches_played: i64,
}

/// Who leaves the queue for the next match, `match_size` of `queued` (in queue order): the fewest
/// matches played in the camp first, the queue's order among equals. S rank gives no priority;
/// `starting_queue` spreads S-rank players over the camp's teams instead. SS rank past two a team
/// waits for a later match, unless the match can't be filled without them.
pub fn pick_match_seats(queued: &[Candidate], match_size: usize) -> Vec<Seat> {
    let mut order = queued.to_vec();
    order.sort_by_key(|c| c.matches_played);
    take_seats(&order, match_size)
}

/// Who plays a match after its refresh, as many as `playing`: the match's players and the queued
/// ones together, the fewest matches played first, so a queued player who played less takes the
/// seat of a match player who played more. The seats left for players tied at the last count are
/// drawn at random among them, in the match or in the queue. SS rank as in `pick_match_seats`.
pub fn refresh_roster(playing: &[Candidate], queued: &[Candidate], rng: &mut impl Rng) -> Vec<Seat> {
    let mut order: Vec<Candidate> = playing.iter().chain(queued).copied().collect();
    order.shuffle(rng);
    order.sort_by_key(|c| c.matches_played);
    take_seats(&order, playing.len())
}

/// The first `n` of `order`, skipping SS-rank players past `MAX_SS_RANK_PER_TEAM` a team; the
/// skipped ones only fill seats nobody else can.
fn take_seats(order: &[Candidate], n: usize) -> Vec<Seat> {
    let cap = MAX_SS_RANK_PER_TEAM.min(n / 2) * 2;
    let mut seats: Vec<Seat> = Vec::with_capacity(n);
    let mut skipped: Vec<Seat> = Vec::new();
    let mut ss_rank = 0;
    for candidate in order {
        if seats.len() == n {
            break;
        }
        if candidate.seat.ss_rank {
            if ss_rank == cap {
                skipped.push(candidate.seat);
                continue;
            }
            ss_rank += 1;
        }
        seats.push(candidate.seat);
    }
    let short = n - seats.len();
    seats.extend(skipped.into_iter().take(short));
    seats
}

/// Roles of `Role::ALL` nobody on the team plays.
fn missing_roles(team: &[Seat]) -> usize {
    Role::ALL
        .iter()
        .filter(|role| !team.iter().any(|s| s.role == Some(**role)))
        .count()
}

/// Every SS-rank player on the team is a captain. With none, one captain: the S-rank player with
/// the most points, or the player with the most points when the team has no S rank either; ties
/// are random.
fn pick_captains(team: &[Seat], rng: &mut impl Rng) -> Vec<i64> {
    let ss_rank: Vec<i64> = team.iter().filter(|s| s.ss_rank).map(|s| s.user_id).collect();
    if !ss_rank.is_empty() {
        return ss_rank;
    }
    let any_s_rank = team.iter().any(|s| s.s_rank);
    let pool: Vec<&Seat> = team.iter().filter(|s| s.s_rank || !any_s_rank).collect();
    let Some(best) = pool.iter().map(|s| s.points).max() else {
        return Vec::new();
    };
    let top: Vec<i64> = pool
        .iter()
        .filter(|s| s.points == best)
        .map(|s| s.user_id)
        .collect();
    top.choose(rng).copied().into_iter().collect()
}

/// Split a match roster into Hiddenking and Archmother and name each team's captains.
///
/// Of every way to split the roster in half, it keeps those where the teams' SS-rank counts
/// differ by one at most (so with up to four, two a team at most), then their S-rank counts, then
/// both together. Among them, the ones where the teams miss the fewest roles (each team should
/// have a tank or bruiser, a carry and a utility or support), the worse-off team counting next.
/// `balanced` (finals) then keeps the closest point totals. One of those that are left is picked
/// at random, sides included. A match has 12 players at most, 924 splits.
pub fn split_match(seats: &[Seat], balanced: bool, rng: &mut impl Rng) -> TeamSplit {
    let half = seats.len() / 2;
    // How far two counts are past differing by one.
    let uneven = |a: usize, b: usize| a.abs_diff(b).saturating_sub(1);
    let count = |team: &[Seat], rank: fn(&Seat) -> bool| team.iter().filter(|s| rank(s)).count();
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
        let (a_ss, b_ss) = (count(&team_a, |s| s.ss_rank), count(&team_b, |s| s.ss_rank));
        let (a_s, b_s) = (count(&team_a, |s| s.s_rank), count(&team_b, |s| s.s_rank));
        let ranks = (uneven(a_ss, b_ss), uneven(a_s, b_s), uneven(a_ss + a_s, b_ss + b_s));
        let (a_missing, b_missing) = (missing_roles(&team_a), missing_roles(&team_b));
        let gap = if balanced {
            let total = |team: &[Seat]| team.iter().map(|s| s.points).sum::<i64>();
            (total(&team_a) - total(&team_b)).abs()
        } else {
            0
        };
        let score = (ranks, a_missing + b_missing, a_missing.max(b_missing), gap);
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

/// Rebuild the queue after a match ends: the fewest matches played in the camp first.
///
/// Among players with the same count, the waiters (`waiters`, in queue order) keep their order in
/// front, then the match's winners, then its losers. A winner or loser block with more than 2
/// players is shuffled.
pub fn requeue(waiters: Vec<QueuePlayer>, finished: Vec<QueuePlayer>, rng: &mut impl Rng) -> Vec<QueuePlayer> {
    type Blocks = (Vec<QueuePlayer>, Vec<QueuePlayer>, Vec<QueuePlayer>);
    let mut by_count: std::collections::BTreeMap<i64, Blocks> = std::collections::BTreeMap::new();
    for waiter in waiters {
        by_count.entry(waiter.matches_played).or_default().0.push(waiter);
    }
    for player in finished {
        let entry = by_count.entry(player.matches_played).or_default();
        match player.just_won {
            Some(true) => entry.1.push(player),
            _ => entry.2.push(player),
        }
    }

    let mut result = Vec::new();
    for (_count, (waiting, mut winners, mut losers)) in by_count {
        shuffle_if_many(&mut winners, rng);
        shuffle_if_many(&mut losers, rng);
        result.extend(waiting);
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
            ss_rank: false,
            role: None,
        }
    }

    fn ss(user_id: i64, points: i64) -> Seat {
        Seat { ss_rank: true, ..seat(user_id, points, false) }
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
    fn starting_queue_spreads_ss_rank_before_s_rank() {
        // 24 players in 4 teams of 6: 6 SS rank (0-5) and 6 S rank (6-11).
        let players: Vec<Seat> = (0..24)
            .map(|i| if i < 6 { ss(i, 0) } else { seat(i, 0, i < 12) })
            .collect();
        for seed in 0..50 {
            let order = starting_queue(&players, 6, &mut StdRng::seed_from_u64(seed));
            let ss_rank = s_rank_per_team(&order, &[0, 1, 2, 3, 4, 5], 6);
            let both = s_rank_per_team(&order, &(0..12).collect::<Vec<_>>(), 6);
            assert!(ss_rank.iter().all(|n| (1..=2).contains(n)), "{ss_rank:?}");
            assert!(both.iter().all(|n| *n == 3), "{both:?}");
        }
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
    fn finalists_are_the_top_of_the_ranking() {
        let finalist = |user_id, points, matches_played, mvps, played_last_final, queue_position| Finalist {
            user_id,
            points,
            matches_played,
            mvps,
            wins: 1,
            played_last_final,
            queue_position,
        };
        let players = [
            finalist(1, 3, 2, 0, false, Some(0)),
            finalist(2, 5, 2, 0, false, Some(1)),
            finalist(3, 3, 2, 1, false, Some(2)),
            finalist(4, 3, 2, 0, false, Some(3)),
            finalist(5, 3, 2, 0, true, Some(4)),
            finalist(6, 4, 2, 0, false, None),
            finalist(7, 3, 1, 0, false, Some(5)),
        ];
        // Points, then MVPs, then fewer matches; equal on all three, the latest finals' players,
        // then the queue's order.
        assert_eq!(select_finalists(&players, 7), [2, 6, 3, 7, 5, 1, 4]);
        assert_eq!(select_finalists(&players, 4), [2, 6, 3, 7]);
        assert_eq!(select_finalists(&players[..2], 4), [2, 1], "everyone when there are too few");
        // Then more wins, before the latest finals and the queue.
        let mut players = players;
        players[3].wins = 2;
        assert_eq!(select_finalists(&players, 7), [2, 6, 3, 7, 4, 5, 1]);
    }

    #[test]
    fn pick_takes_the_front_of_the_queue_s_rank_or_not() {
        let queued: Vec<Candidate> = (0..16)
            .map(|i| Candidate { seat: seat(i, 0, [5, 13, 14].contains(&i)), matches_played: 1 })
            .collect();
        assert_eq!(ids(&pick_match_seats(&queued, 12)), (0..12).collect::<Vec<_>>());
    }

    #[test]
    fn pick_takes_the_fewest_matches_played_first() {
        // The queue's front played 3; 14 and 15, at its end, played 1.
        let queued: Vec<Candidate> = (0..16)
            .map(|i| candidate(i, match i {
                14 | 15 => 1,
                0..4 => 3,
                _ => 2,
            }))
            .collect();
        let mut picked = ids(&pick_match_seats(&queued, 12));
        picked.sort();
        assert_eq!(picked, [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn pick_leaves_ss_rank_past_two_a_team_in_the_queue() {
        // 0-5 SS rank at the front: only four of them fit a 6v6.
        let queued: Vec<Candidate> = (0..16)
            .map(|i| Candidate { seat: if i < 6 { ss(i, 0) } else { seat(i, 0, false) }, matches_played: 0 })
            .collect();
        assert_eq!(ids(&pick_match_seats(&queued, 12)), [0, 1, 2, 3, 6, 7, 8, 9, 10, 11, 12, 13]);
        // Five SS rank and three others for a 4v4: with nobody else to play, the fifth still does.
        let short: Vec<Candidate> = queued[..5].iter().chain(&queued[6..9]).copied().collect();
        assert_eq!(ids(&pick_match_seats(&short, 8)), [0, 1, 2, 3, 6, 7, 8, 4]);
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
                            // Without SS rank, one captain: an S-rank player when the team has one.
                            assert_eq!(captains.len(), 1);
                            assert!(team.contains(&captains[0]));
                            assert_eq!(captains[0] < s_count, s_on_team > 0);
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
    fn ss_rank_captains_and_splits_two_a_team() {
        // Four SS rank (0-3) and four S rank (4-7): two and two on each side.
        let seats: Vec<Seat> = (0..12)
            .map(|i| if i < 4 { ss(i, i) } else { seat(i, i, i < 8) })
            .collect();
        for balanced in [false, true] {
            for seed in 0..50 {
                let split = split_match(&seats, balanced, &mut StdRng::seed_from_u64(seed));
                for (team, captains) in [
                    (&split.hiddenking, &split.hiddenking_captains),
                    (&split.archmother, &split.archmother_captains),
                ] {
                    let mut ss_rank: Vec<i64> = team.iter().copied().filter(|id| *id < 4).collect();
                    let mut captains = captains.clone();
                    ss_rank.sort();
                    captains.sort();
                    assert_eq!(ss_rank.len(), 2, "{split:?}");
                    assert_eq!(team.iter().filter(|id| (4..8).contains(*id)).count(), 2, "{split:?}");
                    assert_eq!(captains, ss_rank, "every SS-rank player on the team captains it");
                }
            }
        }
    }

    #[test]
    fn s_and_ss_rank_spread_together() {
        // One SS and one S rank: never both on the same side.
        let seats: Vec<Seat> = (0..12).map(|i| if i == 0 { ss(i, 0) } else { seat(i, 0, i == 1) }).collect();
        for seed in 0..50 {
            let split = split_match(&seats, false, &mut StdRng::seed_from_u64(seed));
            assert_ne!(split.hiddenking.contains(&0), split.hiddenking.contains(&1), "{split:?}");
            let s_side = if split.hiddenking.contains(&1) { &split.hiddenking_captains } else { &split.archmother_captains };
            assert_eq!(s_side, &[1], "the S-rank player captains the side without SS rank");
        }
    }

    fn candidate(user_id: i64, matches_played: i64) -> Candidate {
        Candidate {
            seat: seat(user_id, 0, false),
            matches_played,
        }
    }

    #[test]
    fn refresh_pulls_in_queued_players_who_played_less() {
        // Match: 0-9 played 1, 10-11 played 3. Queue: 12-13 played 0, 14-17 played 3.
        let playing: Vec<Candidate> = (0..12).map(|i| candidate(i, if i < 10 { 1 } else { 3 })).collect();
        let queued: Vec<Candidate> = (12..18).map(|i| candidate(i, if i < 14 { 0 } else { 3 })).collect();
        for seed in 0..20 {
            let mut roster = ids(&refresh_roster(&playing, &queued, &mut StdRng::seed_from_u64(seed)));
            roster.sort();
            assert_eq!(roster, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 13]);
        }
    }

    #[test]
    fn refresh_keeps_waiters_and_draws_the_rest_from_match_and_queue() {
        // Match: 0-5 waited (0 matches), 6-11 played 1. Queue: 12-17 played 1, 18-19 played 2.
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
            assert!(ids.iter().all(|id| *id < 18), "nobody who played more than the match");
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

    fn waiting(user_id: i64, matches_played: i64) -> QueuePlayer {
        QueuePlayer { user_id, matches_played, queue_position: 0, just_won: None }
    }

    #[test]
    fn requeue_puts_the_fewest_matches_played_first() {
        // Waiting: 1 and 2 played 1, 3 played 2. The match: 4-6 won and 7-9 lost, now at 2 each.
        let waiters = vec![waiting(1, 1), waiting(2, 1), waiting(3, 2)];
        let finished: Vec<QueuePlayer> = (4..10)
            .map(|i| QueuePlayer { just_won: Some(i < 7), ..waiting(i, 2) })
            .collect();
        for seed in 0..20 {
            let queue = requeue(waiters.clone(), finished.clone(), &mut StdRng::seed_from_u64(seed));
            let order: Vec<i64> = queue.iter().map(|p| p.user_id).collect();
            assert_eq!(order[..3], [1, 2, 3], "waiters first among equals");
            let mut winners = order[3..6].to_vec();
            winners.sort();
            assert_eq!(winners, [4, 5, 6], "winners before losers");
            assert_eq!(queue.iter().map(|p| p.queue_position).collect::<Vec<_>>(), (0..9).collect::<Vec<_>>());
        }
        // A waiter who played more goes behind a match player who played less.
        let queue = requeue(vec![waiting(1, 5)], vec![QueuePlayer { just_won: Some(false), ..waiting(2, 1) }], &mut StdRng::seed_from_u64(1));
        assert_eq!(queue.iter().map(|p| p.user_id).collect::<Vec<_>>(), [2, 1]);
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
