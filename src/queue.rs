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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CampLayout {
    pub match_size: usize,
    pub refresh_votes: usize,
}

impl CampLayout {
    /// 24/30 camps run two concurrent 6v6s. Size 4 is a 2v2 test camp.
    pub fn from_roster_size(n: i64) -> Self {
        if n == 4 {
            Self {
                match_size: 4,
                refresh_votes: 2,
            }
        } else {
            Self {
                match_size: 12,
                refresh_votes: 4,
            }
        }
    }
}

pub fn shuffle_ids(ids: &mut [i64], rng: &mut impl Rng) {
    ids.shuffle(rng);
}

/// Split an even roster into Hiddenking and Archmother (6v6 or 2v2).
pub fn split_teams(mut ids: Vec<i64>, rng: &mut impl Rng) -> (Vec<i64>, Vec<i64>) {
    ids.shuffle(rng);
    let mid = ids.len() / 2;
    let hiddenking = ids[..mid].to_vec();
    let archmother = ids[mid..].to_vec();
    (hiddenking, archmother)
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
