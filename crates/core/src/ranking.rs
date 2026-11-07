//! The camp ranking: points first, then MVPs as the tie-breaker.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing {
    pub user_id: i64,
    pub points: i64,
    /// Finished matches where an admin picked this player as MVP.
    pub mvps: i64,
}

/// Best first, each with its place (1 is gold). Places count distinct (points, MVPs) pairs:
/// with the same points, more MVPs ranks above; equal on both shares the place.
pub fn rank(players: &[Standing]) -> Vec<(Standing, usize)> {
    let mut sorted = players.to_vec();
    sorted.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.mvps.cmp(&a.mvps))
            .then(a.user_id.cmp(&b.user_id))
    });
    let mut place = 0;
    let mut last = None;
    sorted
        .into_iter()
        .map(|s| {
            if last != Some((s.points, s.mvps)) {
                place += 1;
                last = Some((s.points, s.mvps));
            }
            (s, place)
        })
        .collect()
}

/// What the latest result gave: a point to each winner and an MVP to its MVP.
pub struct LatestResult<'a> {
    pub winners: &'a [i64],
    pub mvp: Option<i64>,
}

/// `rank`, with how many places each player moved with `latest`, up positive: their place now
/// against their place in the standings just before it.
pub fn rank_with_movement(players: &[Standing], latest: &LatestResult) -> Vec<(Standing, usize, i64)> {
    let before: Vec<Standing> = players
        .iter()
        .map(|s| Standing {
            points: s.points - i64::from(latest.winners.contains(&s.user_id)),
            mvps: s.mvps - i64::from(latest.mvp == Some(s.user_id)),
            ..*s
        })
        .collect();
    let places_before: std::collections::HashMap<i64, usize> =
        rank(&before).into_iter().map(|(s, place)| (s.user_id, place)).collect();
    rank(players)
        .into_iter()
        .map(|(s, place)| {
            let was = places_before.get(&s.user_id).copied().unwrap_or(place);
            (s, place, was as i64 - place as i64)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standing(user_id: i64, points: i64, mvps: i64) -> Standing {
        Standing { user_id, points, mvps }
    }

    #[test]
    fn mvps_break_ties_in_points() {
        let ranked = rank(&[
            standing(1, 3, 0),
            standing(2, 3, 2),
            standing(3, 5, 0),
            standing(4, 3, 2),
            standing(5, 1, 4),
        ]);
        let places: Vec<(i64, usize)> = ranked.iter().map(|(s, place)| (s.user_id, *place)).collect();
        // 5 points first; then 3 points with 2 MVPs (a shared place), then 3 with none.
        // MVPs never lift a player over someone with more points.
        assert_eq!(places, [(3, 1), (2, 2), (4, 2), (1, 3), (5, 4)]);
    }

    #[test]
    fn movement_compares_with_the_standings_before_the_latest_result() {
        // Before it: 1 led with 2 points, 2 and 3 shared second with 1, 4 had none.
        // The latest result: 2 and 4 won, and 4 was MVP.
        let now = [standing(1, 2, 0), standing(2, 2, 0), standing(3, 1, 0), standing(4, 1, 1)];
        let ranked = rank_with_movement(&now, &LatestResult { winners: &[2, 4], mvp: Some(4) });
        let moves: Vec<(i64, usize, i64)> = ranked.iter().map(|(s, place, moved)| (s.user_id, *place, *moved)).collect();
        // 1 keeps first (now shared), 2 climbs to it, 4 jumps over 3 with the MVP, 3 drops.
        assert_eq!(moves, [(1, 1, 0), (2, 1, 1), (4, 2, 1), (3, 3, -1)]);
    }
}
