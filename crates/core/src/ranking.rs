//! The camp ranking: points first, then MVPs, then fewer matches played.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing {
    pub user_id: i64,
    pub points: i64,
    /// Finished matches of the camp this player played.
    pub matches: i64,
    /// Finished matches where an admin picked this player as MVP.
    pub mvps: i64,
}

impl Standing {
    /// What decides the place: equal on all of it shares the place.
    fn key(&self) -> (i64, i64, i64) {
        (self.points, self.mvps, self.matches)
    }
}

/// Best first, each with its place (1 is gold). With the same points, more MVPs ranks above, then
/// fewer matches played; equal on all three shares the place.
pub fn rank(players: &[Standing]) -> Vec<(Standing, usize)> {
    let mut sorted = players.to_vec();
    sorted.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.mvps.cmp(&a.mvps))
            .then(a.matches.cmp(&b.matches))
            .then(a.user_id.cmp(&b.user_id))
    });
    let mut place = 0;
    let mut last = None;
    sorted
        .into_iter()
        .map(|s| {
            if last != Some(s.key()) {
                place += 1;
                last = Some(s.key());
            }
            (s, place)
        })
        .collect()
}

/// What the latest result gave: each of its players' points (the match's win or loss points), a
/// match played, and an MVP to its MVP.
pub struct LatestResult<'a> {
    pub gained: &'a [(i64, i64)],
    pub mvp: Option<i64>,
}

/// `rank`, with how many places each player moved with `latest`, up positive: their place now
/// against their place in the standings just before it.
pub fn rank_with_movement(players: &[Standing], latest: &LatestResult) -> Vec<(Standing, usize, i64)> {
    let before: Vec<Standing> = players
        .iter()
        .map(|s| Standing {
            points: s.points - latest.gained.iter().find(|(id, _)| *id == s.user_id).map_or(0, |(_, p)| *p),
            matches: s.matches - i64::from(latest.gained.iter().any(|(id, _)| *id == s.user_id)),
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
        Standing { user_id, points, matches: 0, mvps }
    }

    #[test]
    fn fewer_matches_break_ties_in_points_and_mvps() {
        let played = |user_id, points, matches, mvps| Standing { user_id, points, matches, mvps };
        let ranked = rank(&[
            played(1, 3, 3, 2),
            played(2, 3, 3, 0),
            played(3, 4, 5, 0),
            played(4, 3, 2, 0),
            played(5, 3, 2, 0),
        ]);
        let places: Vec<(i64, usize)> = ranked.iter().map(|(s, place)| (s.user_id, *place)).collect();
        // 3 points: MVPs first, then fewer matches; equal on all three shares the place.
        assert_eq!(places, [(3, 1), (1, 2), (4, 3), (5, 3), (2, 4)]);
    }

    #[test]
    fn mvps_break_ties_in_points_and_matches() {
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
        // Before it: 1 led with 2 points, 2 and 3 shared second with 1, 4 had none; everyone had
        // played one match. The latest result: 2 and 4 won, and 4 was MVP; 2 and 4 played it.
        let played = |user_id, points, matches, mvps| Standing { user_id, points, matches, mvps };
        let now = [played(1, 2, 1, 0), played(2, 2, 2, 0), played(3, 1, 1, 0), played(4, 1, 2, 1)];
        let ranked = rank_with_movement(&now, &LatestResult { gained: &[(2, 1), (4, 1)], mvp: Some(4) });
        let moves: Vec<(i64, usize, i64)> = ranked.iter().map(|(s, place, moved)| (s.user_id, *place, *moved)).collect();
        // 1 keeps first, with fewer matches than 2, who stays second; 4 jumps over 3 with the MVP,
        // and 3, no longer tied with 2, drops two.
        assert_eq!(moves, [(1, 1, 0), (2, 2, 0), (4, 3, 0), (3, 4, -2)]);
    }

    #[test]
    fn movement_takes_back_what_the_match_was_worth() {
        // Before: 1 led with 6, 2 had 5. A match worth 3 to win and 1 to lose: 2 won, 1 lost.
        let now = [standing(1, 7, 0), standing(2, 8, 0)];
        let ranked = rank_with_movement(&now, &LatestResult { gained: &[(1, 1), (2, 3)], mvp: None });
        let moves: Vec<(i64, usize, i64)> = ranked.iter().map(|(s, place, moved)| (s.user_id, *place, *moved)).collect();
        assert_eq!(moves, [(2, 1, 1), (1, 2, -1)]);
    }
}
