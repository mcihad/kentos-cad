//! An opening's temporary distances along its wall's face (docs/adr/0701): the references before and after its jambs
//! on the face segment of its edge, nearest first. The caller gives the face segment and the candidates (the segment's
//! ends, the jambs of the edge's other openings, where other walls' faces cross it); this picks the nearest on each
//! side by one rule, so the desktop, the headless server and the shared cases agree.

use crate::vec2::Vec2;

/// Slack, metres: a reference at a jamb (a touching opening) counts on that side.
pub const TIE: f64 = 1e-9;

/// What a reference is; on a tie the opening comes first, then the wall, then the face's end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Opening,
    Wall,
    End,
}

/// A reference along the face: its kind, its owner's order key (an object's id as a big-endian number; 0 for an
/// end) and where it is, `s` metres from the face segment's start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candidate {
    pub kind: Kind,
    pub key: u128,
    pub s: f64,
}

/// The reference nearest at or before `s` (`before`), or at or after it: the largest (smallest) position, a tie within
/// [`TIE`] going to the kind's order, then to the lower key.
pub fn nearest(candidates: &[Candidate], s: f64, before: bool) -> Option<Candidate> {
    let side: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| if before { c.s <= s + TIE } else { c.s >= s - TIE })
        .collect();
    let edge = side
        .iter()
        .map(|c| c.s)
        .reduce(|a, b| if before { a.max(b) } else { a.min(b) })?;
    side.into_iter()
        .filter(|c| (c.s - edge).abs() <= TIE)
        .min_by(|a, b| {
            a.kind
                .cmp(&b.kind)
                .then(a.key.cmp(&b.key))
                .then(if before {
                    b.s.total_cmp(&a.s)
                } else {
                    a.s.total_cmp(&b.s)
                })
        })
        .copied()
}

/// Where segment `c`–`d` crosses `a`–`b`: the parameter along `a`–`b` (0 to 1, [`TIE`] of slack on both), or none for
/// parallel segments or a miss.
pub fn crossing(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<f64> {
    let r = (b.x - a.x, b.y - a.y);
    let q = (d.x - c.x, d.y - c.y);
    let den = r.0 * q.1 - r.1 * q.0;
    if den.abs() <= 1e-12 * r.0.hypot(r.1) * q.0.hypot(q.1) {
        return None;
    }
    let w = (c.x - a.x, c.y - a.y);
    let t = (w.0 * q.1 - w.1 * q.0) / den;
    let v = (w.0 * r.1 - w.1 * r.0) / den;
    ((-TIE..=1.0 + TIE).contains(&t) && (-TIE..=1.0 + TIE).contains(&v)).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(kind: Kind, key: u128, s: f64) -> Candidate {
        Candidate { kind, key, s }
    }

    #[test]
    fn the_nearest_on_each_side_and_a_touching_jamb_counts() {
        let all = [
            c(Kind::End, 0, 0.0),
            c(Kind::End, 0, 7.75),
            c(Kind::Wall, 2, 3.825),
            c(Kind::Opening, 4, 5.275),
        ];
        assert_eq!(nearest(&all, 1.425, true), Some(all[0]));
        assert_eq!(nearest(&all, 2.325, false), Some(all[2]));
        assert_eq!(nearest(&all, 5.275, true), Some(all[3]), "touching");
        assert_eq!(nearest(&all[1..], 0.5, true), None);
    }

    #[test]
    fn a_tie_goes_to_the_opening_then_the_wall_then_the_lower_key() {
        let all = [
            c(Kind::End, 0, 3.0),
            c(Kind::Wall, 9, 3.0),
            c(Kind::Wall, 5, 3.0 + 1e-10),
            c(Kind::Opening, 7, 3.0 - 1e-10),
        ];
        assert_eq!(nearest(&all, 4.0, true).map(|c| c.key), Some(7));
        assert_eq!(nearest(&all[..3], 4.0, true).map(|c| c.key), Some(5));
        assert_eq!(nearest(&all[..1], 4.0, true).map(|c| c.kind), Some(Kind::End));
    }

    #[test]
    fn crossings_of_faces() {
        let (a, b) = (Vec2::new(0.125, 0.125), Vec2::new(7.875, 0.125));
        let t = crossing(a, b, Vec2::new(3.95, 0.0), Vec2::new(3.95, 3.0)).unwrap();
        assert!((t * 7.75 - 3.825).abs() < 1e-12);
        assert_eq!(crossing(a, b, Vec2::new(3.95, 0.2), Vec2::new(3.95, 3.0)), None, "short of it");
        assert_eq!(crossing(a, b, Vec2::new(0.0, 0.125), Vec2::new(1.0, 0.125)), None, "along it");
    }
}
