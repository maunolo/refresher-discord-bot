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
}
