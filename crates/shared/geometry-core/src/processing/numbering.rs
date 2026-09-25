//! Corner numbering (`apps/web/src/processing/builtin/numbering.ts`, "Köşe noktalarını
//! numarala"): the order a ring is walked in, one number per location across
//! many shapes (neighbouring parcels share their common corners, points
//! already on the target layer keep theirs), the outward direction at a
//! corner and where the text beside it goes. The names themselves ("P00017")
//! are text and stay in TypeScript: a corner here says whose number it
//! takes, the run's k-th new one or an existing point's (docs/adr/0008, S4).

use std::collections::HashMap;

use crate::api::Op;
use crate::geometry::signed_area;
use crate::jsmath::{js_cmp, js_hypot, js_max, or, stable_sort};
use crate::op;
use crate::text::{Font, width_em};
use crate::vec2::Vec2;

/// Where a ring's numbering starts (`StartCorner`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StartCorner {
    /// Nearest the north-west: highest northing minus easting.
    Northwest,
    /// Highest northing.
    North,
    /// The shape's first vertex.
    First,
    /// Nearest a point given by the user.
    Point,
}

impl StartCorner {
    /// The TypeScript's value (`'northwest' | 'north' | 'first' | 'point'`).
    pub fn parse(s: &str) -> Result<StartCorner, String> {
        match s {
            "northwest" => Ok(StartCorner::Northwest),
            "north" => Ok(StartCorner::North),
            "first" => Ok(StartCorner::First),
            "point" => Ok(StartCorner::Point),
            _ => Err(format!("bilinmeyen başlangıç köşesi “{s}”")),
        }
    }

    /// The code the typed entry points use: 0 north-west, 1 north, 2 first, 3 point.
    pub fn from_code(code: u32) -> StartCorner {
        match code {
            0 => StartCorner::Northwest,
            1 => StartCorner::North,
            3 => StartCorner::Point,
            _ => StartCorner::First,
        }
    }
}

/// How corners are walked and merged (`NumberingOptions` without the name format).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerWalk {
    /// Counter-clockwise (`dir === 'ccw'`), else clockwise.
    pub ccw: bool,
    pub start: StartCorner,
    /// The point a `Point` start is nearest to (none: every vertex ties).
    pub point: Option<Vec2>,
    /// Corners closer than this (m) are one point with one number.
    pub tolerance: f64,
    /// Merge corners shared by several shapes (and with existing points).
    pub shared: bool,
}

/// One ring of a shape to number: closed (a polygon's outer ring or a hole) or an open path.
#[derive(Clone, Copy, Debug)]
pub struct CornerRing<'a> {
    pub pts: &'a [Vec2],
    pub closed: bool,
}

/// A numbered corner: where, which way is outside, and whose number it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    pub p: Vec2,
    /// Unit vector pointing away from the shape at this corner (for text placement).
    pub out: Vec2,
    /// k ≥ 0: the run's k-th new number (made where it first appears);
    /// −1 − j: existing point j's.
    pub refers: f64,
}

crate::json_struct!(out Corner { p, out, refers => "ref" });

/// How strongly a vertex is "the start": larger wins.
fn start_key(start: StartCorner, point: Option<Vec2>, p: Vec2, i: usize) -> f64 {
    match start {
        // Ties go north.
        StartCorner::Northwest => p.y - p.x + p.y * 1e-12,
        StartCorner::North => p.y - p.x * 1e-12,
        StartCorner::Point => point.map_or(0.0, |q| -js_hypot(p.x - q.x, p.y - q.y)),
        StartCorner::First => -(i as f64),
    }
}

/// Indices of a ring in walking order: turned to the requested direction
/// and rotated to start at the chosen corner. An open path is walked from
/// whichever end is the better start (direction does not apply). An empty
/// ring has no corners, whichever way it would be walked.
pub fn ring_order(
    pts: &[Vec2],
    closed: bool,
    ccw_dir: bool,
    start: StartCorner,
    point: Option<Vec2>,
) -> Vec<usize> {
    let n = pts.len();
    let idx: Vec<usize> = (0..n).collect();
    if n == 0 {
        return idx;
    }
    let key = |k: usize| start_key(start, point, pts[k], k);
    if !closed {
        if n < 2 {
            return idx;
        }
        return if key(n - 1) > key(0) && start != StartCorner::First {
            idx.into_iter().rev().collect()
        } else {
            idx
        };
    }
    let ccw = signed_area(pts) > 0.0;
    let walk: Vec<usize> = if ccw == ccw_dir {
        idx
    } else {
        std::iter::once(0).chain((1..n).rev()).collect()
    };
    let mut best = 0;
    for k in 1..walk.len() {
        if key(walk[k]) > key(walk[best]) {
            best = k;
        }
    }
    walk[best..].iter().chain(&walk[..best]).copied().collect()
}

/// A cell coordinate as a map key: −0 is 0 (`${-0}` is "0") and every NaN one key.
fn cell_key(x: f64) -> u64 {
    if x == 0.0 {
        0
    } else if x.is_nan() {
        f64::NAN.to_bits()
    } else {
        x.to_bits()
    }
}

/// Spatial hash of numbered points, merging within the tolerance: the first
/// point found within one cell size, in the order the neighbouring cells and
/// their points are visited, gives its number.
struct PointIndex {
    cell: f64,
    grid: HashMap<(u64, u64), Vec<(Vec2, f64)>>,
}

impl PointIndex {
    fn new(tolerance: f64) -> PointIndex {
        PointIndex {
            cell: js_max(tolerance, 1e-9),
            grid: HashMap::new(),
        }
    }

    fn find(&self, p: Vec2) -> Option<f64> {
        let gx = (p.x / self.cell).floor();
        let gy = (p.y / self.cell).floor();
        // The neighbouring cells by offset: counting from gx − 1 up to gx + 1
        // never ended in TypeScript beyond 2^53, where gx + 1 is gx.
        for di in [-1.0, 0.0, 1.0] {
            for dj in [-1.0, 0.0, 1.0] {
                let Some(list) = self.grid.get(&(cell_key(gx + di), cell_key(gy + dj))) else {
                    continue;
                };
                for &(q, refers) in list {
                    if js_hypot(q.x - p.x, q.y - p.y) <= self.cell {
                        return Some(refers);
                    }
                }
            }
        }
        None
    }

    fn add(&mut self, p: Vec2, refers: f64) {
        let k = (
            cell_key((p.x / self.cell).floor()),
            cell_key((p.y / self.cell).floor()),
        );
        self.grid.entry(k).or_default().push((p, refers));
    }
}

/// Outward unit direction at corner i of a ring (bisector of the two edge
/// normals); `ccw`: the ring is closed and counter-clockwise.
fn outward(pts: &[Vec2], i: usize, closed: bool, ccw: bool) -> Vec2 {
    let n = pts.len();
    // Right of travel is outside a counter-clockwise ring.
    let s = if !closed || ccw { 1.0 } else { -1.0 };
    let normal = |a: Vec2, b: Vec2| {
        let l = or(js_hypot(b.x - a.x, b.y - a.y), 1.0);
        Vec2::new(((b.y - a.y) / l) * s, (-(b.x - a.x) / l) * s)
    };
    let prev = if i > 0 {
        Some(normal(pts[i - 1], pts[i]))
    } else if closed {
        Some(normal(pts[n - 1], pts[0]))
    } else {
        None
    };
    let next = if i + 1 < n {
        Some(normal(pts[i], pts[i + 1]))
    } else if closed {
        Some(normal(pts[n - 1], pts[0]))
    } else {
        None
    };
    let sx = prev.map_or(0.0, |v| v.x) + next.map_or(0.0, |v| v.x);
    let sy = prev.map_or(0.0, |v| v.y) + next.map_or(0.0, |v| v.y);
    let l = js_hypot(sx, sy);
    if l > 1e-12 {
        Vec2::new(sx / l, sy / l)
    } else {
        Vec2::new(0.0, 1.0)
    }
}

/// Numbers the corners of every shape (its rings: the outer ring, then the
/// holes; or one open path). Shapes are taken in order of their start
/// corner (north-west first for the compass starts, nearest first for a
/// point, as given for "first"); each ring is walked from its start in the
/// chosen direction. With `shared`, a corner within the tolerance of an
/// `existing` point or of an earlier corner takes that one's number.
pub fn number_corners(
    inputs: &[Vec<CornerRing<'_>>],
    o: &CornerWalk,
    existing: &[Vec2],
) -> Vec<Corner> {
    let mut index = PointIndex::new(o.tolerance);
    if o.shared {
        for (j, &p) in existing.iter().enumerate() {
            index.add(p, -1.0 - j as f64);
        }
    }
    let first = o.start == StartCorner::First;
    let mut ordered: Vec<(usize, f64)> = inputs
        .iter()
        .enumerate()
        .map(|(i, rings)| {
            let score = rings.first().map_or(f64::NEG_INFINITY, |r| {
                let order = ring_order(r.pts, r.closed, o.ccw, o.start, o.point);
                order.first().map_or(f64::NEG_INFINITY, |&k| {
                    start_key(o.start, o.point, r.pts[k], if first { i } else { k })
                })
            });
            (i, score)
        })
        .collect();
    stable_sort(&mut ordered, &mut |a, b| {
        if first {
            a.0.cmp(&b.0)
        } else {
            js_cmp(b.1, a.1)
        }
    });
    let mut out = Vec::new();
    let mut made = 0.0;
    for &(i, _) in &ordered {
        for r in &inputs[i] {
            // The ring's orientation once, not per corner.
            let ccw = r.closed && signed_area(r.pts) > 0.0;
            for k in ring_order(r.pts, r.closed, o.ccw, o.start, o.point) {
                let p = r.pts[k];
                let known = if o.shared { index.find(p) } else { None };
                let refers = known.unwrap_or_else(|| {
                    let n = made;
                    made += 1.0;
                    index.add(p, n);
                    n
                });
                out.push(Corner {
                    p,
                    out: outward(r.pts, k, r.closed, ccw),
                    refers,
                });
            }
        }
    }
    out
}

/// Where the text beside a numbered corner goes: outside the corner along
/// its bisector, shifted left by its width (`text` measured in the drawing's
/// typeface, `crate::text`) when outside is to the west and down by its
/// height when outside is to the south.
pub fn corner_text_at(p: Vec2, out: Vec2, text: &str, height: f64, font: Font) -> Vec2 {
    let w = width_em(text, font) * height;
    Vec2::new(
        p.x + out.x * height * 1.2 - (if out.x < 0.0 { w } else { 0.0 }),
        p.y + out.y * height * 1.2 - (if out.y < 0.0 { height } else { 0.0 }),
    )
}

/// A ring as the named operations take it (`NumberingInput.rings`).
#[derive(Clone, Debug, PartialEq)]
pub struct RingInput {
    pub pts: Vec<Vec2>,
    pub closed: bool,
}

crate::json_struct!(RingInput { pts, closed });

/// A shape as the named operations take it (`NumberingInput`).
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeInput {
    pub rings: Vec<RingInput>,
}

crate::json_struct!(ShapeInput { rings });

/// The walk as the named operations take it: `{ dir, start, point, tolerance, shared }`.
#[derive(Clone, Debug, PartialEq)]
pub struct WalkInput {
    pub dir: String,
    pub start: String,
    pub point: Option<Vec2>,
    pub tolerance: f64,
    pub shared: bool,
}

crate::json_struct!(WalkInput {
    dir,
    start,
    point,
    tolerance,
    shared
});

impl WalkInput {
    fn walk(&self) -> Result<CornerWalk, String> {
        Ok(CornerWalk {
            ccw: self.dir == "ccw",
            start: StartCorner::parse(&self.start)?,
            point: self.point,
            tolerance: self.tolerance,
            shared: self.shared,
        })
    }
}

/// A corner and the text beside it, as `cornerTextAt` takes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerAt {
    pub p: Vec2,
    pub out: Vec2,
}

crate::json_struct!(CornerAt { p, out });

pub(crate) static OPS: &[Op] = &[
    op!("ringOrder", |pts: Vec<Vec2>,
                      closed: bool,
                      dir: String,
                      start: String,
                      point: Option<Vec2>| {
        StartCorner::parse(&start).map(|start| ring_order(&pts, closed, dir == "ccw", start, point))
    }),
    op!(
        "numberCorners",
        |inputs: Vec<ShapeInput>, walk: WalkInput, existing: Vec<Vec2>| {
            walk.walk().map(|w| {
                let rings: Vec<Vec<CornerRing>> = inputs
                    .iter()
                    .map(|s| {
                        s.rings
                            .iter()
                            .map(|r| CornerRing {
                                pts: &r.pts,
                                closed: r.closed,
                            })
                            .collect()
                    })
                    .collect();
                number_corners(&rings, &w, &existing)
            })
        }
    ),
    op!(
        "cornerTextAt",
        |c: CornerAt, text: String, height: f64, font: Option<String>| {
            corner_text_at(
                c.p,
                c.out,
                &text,
                height,
                font.as_deref().map_or(Font::DEFAULT, Font::from_id),
            )
        }
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    /// Counter-clockwise square with its first vertex at the south-west corner.
    fn square(x: f64, y: f64, s: f64) -> Vec<Vec2> {
        vec![v(x, y), v(x + s, y), v(x + s, y + s), v(x, y + s)]
    }

    fn walk(shared: bool, tolerance: f64) -> CornerWalk {
        CornerWalk {
            ccw: false,
            start: StartCorner::Northwest,
            point: None,
            tolerance,
            shared,
        }
    }

    #[test]
    fn rings_turn_and_start_where_asked() {
        let sq = square(0.0, 0.0, 10.0);
        let nw = StartCorner::Northwest;
        assert_eq!(ring_order(&sq, true, false, nw, None), [3, 2, 1, 0]);
        assert_eq!(ring_order(&sq, true, true, nw, None), [3, 0, 1, 2]);
        let first = StartCorner::First;
        assert_eq!(ring_order(&sq, true, false, first, None), [0, 3, 2, 1]);
        let near = Some(v(11.0, -1.0));
        assert_eq!(
            ring_order(&sq, true, false, StartCorner::Point, near),
            [1, 0, 3, 2]
        );
        let path = [v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0)];
        assert_eq!(
            ring_order(&path, false, false, StartCorner::North, None),
            [2, 1, 0]
        );
        // An empty ring has no corners either way.
        assert!(ring_order(&[], true, true, nw, None).is_empty());
    }

    #[test]
    fn neighbours_share_corners_and_existing_points_keep_theirs() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(10.0, 0.0, 10.0);
        let shapes = vec![
            vec![CornerRing {
                pts: &a,
                closed: true,
            }],
            vec![CornerRing {
                pts: &b,
                closed: true,
            }],
        ];
        let c = number_corners(&shapes, &walk(true, 0.001), &[]);
        let refs: Vec<f64> = c.iter().map(|x| x.refers).collect();
        // West square NW, NE, SE, SW; then the east square from its NW (the west's NE).
        assert_eq!(refs, [0.0, 1.0, 2.0, 3.0, 1.0, 4.0, 5.0, 2.0]);
        assert_eq!(c[0].p, v(0.0, 10.0));
        let unshared = number_corners(&shapes, &walk(false, 0.001), &[]);
        assert_eq!(unshared.last().map(|x| x.refers), Some(7.0));
        let kept = number_corners(&shapes[..1], &walk(true, 0.001), &[v(0.0, 10.0)]);
        assert_eq!(kept[0].refers, -1.0);
        assert_eq!(kept[1].refers, 0.0);
    }

    #[test]
    fn far_from_the_origin_with_no_tolerance_the_search_ends() {
        let a = square(1e10, 0.0, 10.0);
        let b = square(1e10 + 10.0, 0.0, 10.0);
        let shapes = vec![
            vec![CornerRing {
                pts: &a,
                closed: true,
            }],
            vec![CornerRing {
                pts: &b,
                closed: true,
            }],
        ];
        let c = number_corners(&shapes, &walk(true, 0.0), &[]);
        let last = c.iter().map(|x| x.refers as usize).max();
        assert_eq!(last, Some(5), "six numbers for the eight corners");
    }

    #[test]
    fn outward_points_away_and_text_clears_the_corner() {
        let sq = square(0.0, 0.0, 10.0);
        let o = outward(&sq, 3, true, true);
        assert!((o.x + std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        assert!((o.y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-12);
        // A lone point has no edges: up.
        assert_eq!(outward(&[v(1.0, 1.0)], 0, false, false), v(0.0, 1.0));
        let mono = Font::from_id("plex-mono");
        let at = corner_text_at(v(0.0, 0.0), v(-1.0, -1.0), "100001", 2.0, mono);
        assert_eq!(
            at,
            v(
                -2.4 - crate::text::width_em("100001", mono) * 2.0,
                -2.4 - 2.0
            )
        );
        // East of the corner nothing is measured: the text starts there.
        assert_eq!(
            corner_text_at(v(0.0, 0.0), v(1.0, 0.0), "100001", 2.0, mono),
            v(2.4, 0.0)
        );
    }
}
