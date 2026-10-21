//! Matching the label pins of objects whose rows are gone to the rows that took their place, when the link has no
//! key (docs/adr/0578 §2): a table written again by another program changes the rows' ids; the pins must not go to a
//! guess. The rule, in order:
//!
//! 1. **Geometry.** An old object's candidates are the new rows of the same kind with exactly the same coordinates
//!    (every vertex and the box, as bits). One candidate: it. More than one: ambiguous.
//! 2. **Nearest.** An object with no such row takes, among the new rows no object of step 1 is a candidate for, the
//!    nearest of the same kind by the distance between the middles of the boxes; it must lie within the tolerance
//!    ([`TOLERANCE`], metres), and the next nearest must be farther than it: equal distances are ambiguous.
//! 3. **Else** the pin is lost.
//!
//! A new row two objects both chose goes to neither: both are ambiguous. The answer does not depend on the order of
//! the objects. The independent reference is `scripts/fixtures/pin_match_cases.py`, the cases
//! `fixtures/labels/v1/pin-match.json`.

use std::collections::HashMap;

/// How near a row's middle must be to an object's to take its pins, metres.
pub const TOLERANCE: f64 = 0.05;

/// A feature as the matching sees it: its kind, its vertices and box as bits, the middle of its box.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    pub kind: String,
    /// Every vertex's east and north, then the box's minimum and maximum, as `f64` bits.
    pub bits: Vec<u64>,
    pub middle: [f64; 2],
}

/// What an old object's pins found.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Found {
    /// The one new row with the very same geometry.
    Geometry(usize),
    /// The nearest new row of the kind, within the tolerance, at this distance (metres).
    Nearest(usize, f64),
    /// More than one row would do, or two objects chose one.
    Ambiguous,
    /// No new row.
    None,
}

/// For each of `old`, the row of `new` its pins follow to.
pub fn match_pins(old: &[Feature], new: &[Feature], tolerance: f64) -> Vec<Found> {
    // Step 1: the exact candidates.
    let exact: Vec<Vec<usize>> = old
        .iter()
        .map(|o| {
            (0..new.len())
                .filter(|&j| new[j].kind == o.kind && new[j].bits == o.bits)
                .collect()
        })
        .collect();
    let mut out: Vec<Option<Found>> = vec![None; old.len()];
    let taken: std::collections::HashSet<usize> = exact.iter().flatten().copied().collect();
    for (i, c) in exact.iter().enumerate() {
        out[i] = match c.len() {
            0 => None,
            1 => Some(Found::Geometry(c[0])),
            _ => Some(Found::Ambiguous),
        };
    }
    // Step 2: the nearest of the rest.
    for (i, o) in old.iter().enumerate() {
        if !exact[i].is_empty() {
            continue;
        }
        let mut near: Vec<(f64, usize)> = (0..new.len())
            .filter(|j| !taken.contains(j) && new[*j].kind == o.kind)
            .map(|j| {
                let (dx, dy) = (
                    new[j].middle[0] - o.middle[0],
                    new[j].middle[1] - o.middle[1],
                );
                ((dx * dx + dy * dy).sqrt(), j)
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        out[i] = Some(match near.first() {
            Some(&(d, j)) if d <= tolerance => {
                if near.get(1).is_some_and(|&(d1, _)| d1 == d) {
                    Found::Ambiguous
                } else {
                    Found::Nearest(j, d)
                }
            }
            _ => Found::None,
        });
    }
    // A row two objects chose goes to neither.
    let mut chosen: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, f) in out.iter().enumerate() {
        if let Some(Found::Geometry(j) | Found::Nearest(j, _)) = f {
            chosen.entry(*j).or_default().push(i);
        }
    }
    for owners in chosen.values().filter(|o| o.len() > 1) {
        for &i in owners {
            out[i] = Some(Found::Ambiguous);
        }
    }
    out.into_iter().map(|f| f.unwrap_or(Found::None)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(kind: &str, x: f64, y: f64) -> Feature {
        Feature {
            kind: kind.into(),
            bits: vec![x.to_bits(), y.to_bits()],
            middle: [x, y],
        }
    }

    #[test]
    fn the_same_coordinates_win_over_a_nearer_row() {
        let old = [at("point", 1.0, 1.0)];
        let new = [at("point", 1.01, 1.0), at("point", 1.0, 1.0)];
        assert_eq!(match_pins(&old, &new, TOLERANCE), [Found::Geometry(1)]);
    }

    #[test]
    fn the_nearest_within_the_tolerance_and_a_tie_is_not_a_guess() {
        let old = [at("point", 0.0, 0.0)];
        assert_eq!(
            match_pins(&old, &[at("point", 0.0, 0.05)], TOLERANCE),
            [Found::Nearest(0, 0.05)]
        );
        assert_eq!(
            match_pins(&old, &[at("point", 0.0, 0.0500001)], TOLERANCE),
            [Found::None]
        );
        let tie = [at("point", 0.02, 0.0), at("point", -0.02, 0.0)];
        assert_eq!(match_pins(&old, &tie, TOLERANCE), [Found::Ambiguous]);
        // Another kind is no row for it.
        assert_eq!(
            match_pins(&old, &[at("polygon", 0.0, 0.0)], TOLERANCE),
            [Found::None]
        );
    }

    #[test]
    fn a_row_two_objects_chose_goes_to_neither() {
        let old = [at("point", 0.0, 0.0), at("point", 0.02, 0.0)];
        let new = [at("point", 0.01, 0.0)];
        assert_eq!(
            match_pins(&old, &new, TOLERANCE),
            [Found::Ambiguous, Found::Ambiguous]
        );
    }
}
