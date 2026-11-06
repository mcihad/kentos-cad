//! Far-view detail levels of long strokes (docs/adr/1064 §2): a path of many vertices drawn as it is near, and
//! simplified by Douglas–Peucker for each of the curves' far ceilings (`batch::CURVE_LEVELS`), within
//! `batch::CURVE_TOLERANCE_PX` of the drawn path wherever that level is drawn. Only the screen's strokes take them
//! (`BatchSink::set_line_lod`); the source geometry, snapping, picking, measures and paper never see them.
//!
//! One pass ranks every vertex (its significance: the distance it was split at, never more than its parent's), so a
//! level is the vertices ranked above its tolerance and the levels nest: what a coarser level keeps a finer one keeps.
//! Ranking with the distance to the chord's segment (not its line) keeps a path's spikes and turns back.

use kentos_geometry_core::Vec2;

/// Fewest vertices a path must have (a closed path's ring) before it comes with levels.
pub(crate) const LEAST_POINTS: usize = 8;
/// A path comes with levels only when its coarsest level drops at least a quarter of its segments: else the levels'
/// numbers would cost memory for the same drawing.
const MOST_KEPT: f64 = 0.75;

/// Squared distance from `p` to the segment `a`–`b` (to `a` when the segment is a point).
fn segment_distance2(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (px, py) = (p.x - a.x, p.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        ((px * dx + py * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (ex, ey) = (px - t * dx, py - t * dy);
    ex * ex + ey * ey
}

/// The squared significance of each of `pts` (the path's vertices, a closed path's first repeated at its end): its
/// ends `INFINITY`, every other vertex the squared distance Douglas–Peucker splits its range at, capped by the range's
/// own.
pub(crate) fn significance(pts: &[Vec2]) -> Vec<f64> {
    let n = pts.len();
    let mut sig = vec![0.0; n];
    if n == 0 {
        return sig;
    }
    sig[0] = f64::INFINITY;
    sig[n - 1] = f64::INFINITY;
    let mut stack = vec![(0usize, n - 1, f64::INFINITY)];
    while let Some((a, b, cap)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (mut far, mut at) = (-1.0, a + 1);
        for (i, &p) in pts.iter().enumerate().take(b).skip(a + 1) {
            let d = segment_distance2(p, pts[a], pts[b]);
            if d > far {
                far = d;
                at = i;
            }
        }
        let s = if far < cap { far } else { cap };
        sig[at] = s;
        stack.push((a, at, s));
        stack.push((at, b, s));
    }
    sig
}

/// The vertices each level keeps (indices into `pts`, ends included), one list per world tolerance in `tolerances`
/// (finest first); none when the path is too short or its coarsest level would keep more than [`MOST_KEPT`] of it.
pub(crate) fn levels(pts: &[Vec2], tolerances: &[f64]) -> Option<Vec<Vec<usize>>> {
    if pts.len() < LEAST_POINTS {
        return None;
    }
    let sig = significance(pts);
    let keep = |t: f64| -> Vec<usize> {
        let t2 = t * t;
        (0..pts.len()).filter(|&i| sig[i] > t2).collect()
    };
    let coarsest = keep(tolerances.iter().copied().fold(0.0, f64::max));
    if (coarsest.len() - 1) as f64 > MOST_KEPT * (pts.len() - 1) as f64 {
        return None;
    }
    Some(tolerances.iter().map(|&t| keep(t)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(n: usize, seed: u64) -> Vec<Vec2> {
        let mut s = seed;
        let mut y = 0.0;
        (0..n)
            .map(|i| {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                y += (s % 1201) as f64 / 1000.0 - 0.6;
                Vec2::new(i as f64 * 0.625, y)
            })
            .collect()
    }

    /// Largest distance from any of `pts` to the kept path.
    fn deviation(pts: &[Vec2], kept: &[usize]) -> f64 {
        let mut most: f64 = 0.0;
        for w in kept.windows(2) {
            for p in &pts[w[0]..=w[1]] {
                most = most.max(segment_distance2(*p, pts[w[0]], pts[w[1]]).sqrt());
            }
        }
        most
    }

    #[test]
    fn each_level_keeps_its_tolerance_and_its_ends_and_the_levels_nest() {
        let pts = walk(400, 19);
        let tol = [0.0625, 0.25, 1.0];
        let levels = levels(&pts, &tol).expect("a long noisy path has levels");
        for (k, kept) in levels.iter().enumerate() {
            assert_eq!(kept.first(), Some(&0));
            assert_eq!(kept.last(), Some(&399));
            assert!(deviation(&pts, kept) <= tol[k], "level {k}: {}", deviation(&pts, kept));
            if k > 0 {
                assert!(levels[k].iter().all(|i| levels[k - 1].contains(i)), "level {k} nests in {}", k - 1);
                assert!(levels[k].len() <= levels[k - 1].len());
            }
        }
        assert!(levels[2].len() < 100, "coarsest keeps {}", levels[2].len());
    }

    #[test]
    fn a_spike_and_a_turn_back_are_kept() {
        // Along x, out to 10 and back: the turn back's point lies on the chord's line but far from its segment.
        let mut pts: Vec<Vec2> = (0..=10).map(|i| Vec2::new(f64::from(i), 0.0)).collect();
        pts.push(Vec2::new(5.0, 0.0));
        pts.push(Vec2::new(5.0, 3.0));
        pts.push(Vec2::new(5.0, 0.0));
        let levels = levels(&pts, &[0.5]).expect("levels");
        let kept: Vec<Vec2> = levels[0].iter().map(|&i| pts[i]).collect();
        assert!(kept.contains(&Vec2::new(10.0, 0.0)), "{kept:?}");
        assert!(kept.contains(&Vec2::new(5.0, 3.0)), "{kept:?}");
        assert!(deviation(&pts, &levels[0]) <= 0.5);
    }

    #[test]
    fn a_short_path_or_one_that_would_not_shrink_has_no_levels() {
        assert!(levels(&walk(7, 3), &[1.0]).is_none());
        // A zigzag much larger than the tolerance keeps all of its vertices.
        let zig: Vec<Vec2> = (0..40).map(|i| Vec2::new(f64::from(i), f64::from(i % 2) * 10.0)).collect();
        assert!(levels(&zig, &[0.25]).is_none());
    }

    /// A dashed stroke of `pts` through a sink with the levels on or off: its batch's JSON and numbers.
    fn stroked(pts: &[Vec2], on: bool) -> (kentos_geometry_core::api::json::Json, Vec<f32>) {
        use super::super::batch::BatchSink;
        use super::super::prim::{PrimUnit, Sink as _, StrokeStyle};
        let style = StrokeStyle {
            color: "#000000".into(),
            opacity: 1.0,
            width: 0.3,
            unit: PrimUnit::Px,
            dash: Some(vec![4.0, 2.0]),
            dash_offset: 0.0,
            line_type_loss: None,
            cap: "round".into(),
            join: "round".into(),
            blur: 0.0,
            level: 0.0,
            ground: false,
        };
        let mut sink = BatchSink::new(Vec2::new(0.0, 0.0));
        sink.set_line_lod(on);
        sink.stroke(&style, pts, false);
        let out = sink.finish();
        let Ok(kentos_geometry_core::api::json::Json::Arr(mut list)) =
            kentos_geometry_core::api::json::Json::parse(&out.json)
        else {
            panic!("{}", out.json);
        };
        assert_eq!(list.len(), 1, "{}", out.json);
        (list.remove(0), out.data)
    }

    #[test]
    fn a_long_stroke_keeps_its_numbers_at_level_0_and_its_dash_starts_at_every_level() {
        use kentos_geometry_core::api::json::Json;
        use super::super::batch::CURVE_LEVELS;
        let pts = walk(400, 19);
        let (off, plain) = stroked(&pts, false);
        assert!(matches!(off.get("lod"), Json::Null), "{off:?}");
        let (on, data) = stroked(&pts, true);
        let Json::Arr(lod) = on.get("lod").clone() else {
            panic!("levels: {on:?}");
        };
        let n: Vec<usize> = lod
            .iter()
            .map(|v| match v {
                Json::Num(x) => *x as usize,
                _ => usize::MAX,
            })
            .collect();
        // Nothing drawn at every scale; level 0 the plain numbers, number for number; then one level per ceiling.
        assert_eq!(n.len(), 2 + CURVE_LEVELS.len(), "{n:?}");
        assert_eq!(n[0], 0);
        assert_eq!(&data[..n[1]], &plain[..], "level 0 is the plain stroke");
        // Every level's segment starts where level 0 has a segment start, with its length along the path.
        let starts: std::collections::HashMap<(u32, u32), f32> = plain
            .chunks(6)
            .map(|s| ((s[0].to_bits(), s[1].to_bits()), s[4]))
            .collect();
        let mut at = n[1];
        for &len in &n[2..] {
            assert!(len > 0 && len < n[1], "{n:?}");
            for s in data[at..at + len].chunks(6) {
                assert_eq!(starts.get(&(s[0].to_bits(), s[1].to_bits())), Some(&s[4]), "a level's dash phase");
            }
            at += len;
        }
    }

    #[test]
    fn a_closed_ring_keeps_its_corners() {
        // A square of 4 m with a vertex every 0.5 m, its first repeated at its end.
        let mut ring = Vec::new();
        for side in 0..4 {
            for k in 0..8 {
                let t = f64::from(k) * 0.5;
                ring.push(match side {
                    0 => Vec2::new(t, 0.0),
                    1 => Vec2::new(4.0, t),
                    2 => Vec2::new(4.0 - t, 4.0),
                    _ => Vec2::new(0.0, 4.0 - t),
                });
            }
        }
        ring.push(ring[0]);
        let levels = levels(&ring, &[0.01]).expect("levels");
        let kept: Vec<Vec2> = levels[0].iter().map(|&i| ring[i]).collect();
        assert_eq!(
            kept,
            vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(4.0, 0.0),
                Vec2::new(4.0, 4.0),
                Vec2::new(0.0, 4.0),
                Vec2::new(0.0, 0.0)
            ]
        );
    }
}
