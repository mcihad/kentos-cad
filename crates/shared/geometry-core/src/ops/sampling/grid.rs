//! Ağ oluştur (docs/adr/0220 §2): rectangular, hexagonal and triangular
//! cells and a rectangular grid's lines over a box, rows from the top and
//! columns from the left, on the lattice of multiples of the size (a map's
//! grid) or from the box's corner. A corner is worked out from whole
//! lattice numbers only, so neighbouring cells share it bit for bit.

use super::MOST;
use crate::geometry::Bounds;
use crate::jsmath::{js_max, js_min};
use crate::vec2::Vec2;

/// The most cells a grid looks at.
pub const MOST_SCANNED: f64 = 4_000_000.0;

pub const TOO_BIG_SCAN: &str =
    "Ağ çok büyük: 4 000 000'dan çok hücre taranırdı; aralığı büyütün ya da kapsamı küçültün.";
pub const TOO_BIG: &str =
    "Ağ çok büyük: 1 000 000'dan çok parça yazılırdı; aralığı büyütün ya da kapsamı küçültün.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rect,
    Hexagon,
    Triangle,
    Lines,
}

impl Kind {
    pub fn from_key(key: &str) -> Option<Kind> {
        match key {
            "rect" => Some(Kind::Rect),
            "hexagon" => Some(Kind::Hexagon),
            "triangle" => Some(Kind::Triangle),
            "lines" => Some(Kind::Lines),
            _ => None,
        }
    }
}

/// What a grid is: its kind, the east and north spacing (rectangles and
/// lines), the side (hexagons and triangles) and whether it sits on the
/// lattice of multiples.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub kind: Kind,
    pub dx: f64,
    pub dy: f64,
    pub side: f64,
    pub snap: bool,
}

/// A piece of the grid: a cell's row and column, a horizontal line's row,
/// a vertical line's column (from 1); a cell's ring counter-clockwise or a
/// line's two ends.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub row: Option<u32>,
    pub col: Option<u32>,
    pub pts: Vec<Vec2>,
}

/// The start on an axis: the box's side, or the multiple of `step` at or below it.
fn start(lo: f64, step: f64, snap: bool) -> f64 {
    if snap { (lo / step).floor() * step } else { lo }
}

/// How many steps from `from` reach `hi`, at least one.
fn steps(from: f64, hi: f64, step: f64) -> f64 {
    let n = ((hi - from) / step).ceil();
    if n >= 1.0 { n } else { 1.0 }
}

/// Whether a counter-clockwise convex ring's inside and the box's overlap:
/// no separating axis among the box's two and the ring's edges' normals
/// (touching is apart).
pub fn meets_box(ring: &[Vec2], b: &Bounds) -> bool {
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in ring {
        x0 = js_min(x0, p.x);
        y0 = js_min(y0, p.y);
        x1 = js_max(x1, p.x);
        y1 = js_max(y1, p.y);
    }
    if !(x0 < b.max_x && b.min_x < x1 && y0 < b.max_y && b.min_y < y1) {
        return false;
    }
    let corners = [
        Vec2::new(b.min_x, b.min_y),
        Vec2::new(b.max_x, b.min_y),
        Vec2::new(b.max_x, b.max_y),
        Vec2::new(b.min_x, b.max_y),
    ];
    let n = ring.len();
    for i in 0..n {
        let (a, c) = (ring[i], ring[(i + 1) % n]);
        // The edge's outward normal: the box is apart when its corners are all beyond the edge's line or on it.
        let (nx, ny) = (c.y - a.y, a.x - c.x);
        if corners
            .iter()
            .all(|q| (q.x - a.x) * nx + (q.y - a.y) * ny >= 0.0)
        {
            return false;
        }
    }
    true
}

/// What chooses cells (Nesnelere değenler): a cell's ring in, keep or not out.
pub type Keep<'a> = Option<&'a mut dyn FnMut(&[Vec2]) -> bool>;

/// The grid over the box; `keep` (Nesnelere değenler) chooses cells.
pub fn pieces(b: &Bounds, spec: &Spec, keep: Keep<'_>) -> Result<Vec<Piece>, &'static str> {
    match spec.kind {
        Kind::Rect => rects(b, spec, keep),
        Kind::Lines => lines(b, spec),
        Kind::Hexagon => hexagons(b, spec, keep),
        Kind::Triangle => triangles(b, spec, keep),
    }
}

fn rects(b: &Bounds, spec: &Spec, mut keep: Keep<'_>) -> Result<Vec<Piece>, &'static str> {
    let (dx, dy) = (spec.dx, spec.dy);
    let (ox, oy) = (start(b.min_x, dx, spec.snap), start(b.min_y, dy, spec.snap));
    let (nc, nr) = (steps(ox, b.max_x, dx), steps(oy, b.max_y, dy));
    let total = nc * nr;
    if keep.is_none() && !(total <= MOST as f64) {
        return Err(TOO_BIG);
    }
    if !(total <= MOST_SCANNED) {
        return Err(TOO_BIG_SCAN);
    }
    let (nc, nr) = (nc as u32, nr as u32);
    let mut out = Vec::new();
    for r in 1..=nr {
        let ya = oy + f64::from(nr - r) * dy;
        let yb = oy + f64::from(nr - r + 1) * dy;
        for c in 1..=nc {
            let xa = ox + f64::from(c - 1) * dx;
            let xb = ox + f64::from(c) * dx;
            let ring = vec![
                Vec2::new(xa, ya),
                Vec2::new(xb, ya),
                Vec2::new(xb, yb),
                Vec2::new(xa, yb),
            ];
            if keep.as_mut().is_none_or(|k| k(&ring)) {
                out.push(Piece {
                    row: Some(r),
                    col: Some(c),
                    pts: ring,
                });
                if out.len() > MOST {
                    return Err(TOO_BIG);
                }
            }
        }
    }
    Ok(out)
}

fn lines(b: &Bounds, spec: &Spec) -> Result<Vec<Piece>, &'static str> {
    let (dx, dy) = (spec.dx, spec.dy);
    let (ox, oy) = (start(b.min_x, dx, spec.snap), start(b.min_y, dy, spec.snap));
    let (nc, nr) = (steps(ox, b.max_x, dx), steps(oy, b.max_y, dy));
    if !(nc + nr + 2.0 <= MOST as f64) {
        return Err(TOO_BIG);
    }
    let (nc, nr) = (nc as u32, nr as u32);
    let (east, north) = (ox + f64::from(nc) * dx, oy + f64::from(nr) * dy);
    let mut out = Vec::with_capacity((nc + nr + 2) as usize);
    for r in 1..=nr + 1 {
        let y = oy + f64::from(nr + 1 - r) * dy;
        out.push(Piece {
            row: Some(r),
            col: None,
            pts: vec![Vec2::new(ox, y), Vec2::new(east, y)],
        });
    }
    for c in 1..=nc + 1 {
        let x = ox + f64::from(c - 1) * dx;
        out.push(Piece {
            row: None,
            col: Some(c),
            pts: vec![Vec2::new(x, oy), Vec2::new(x, north)],
        });
    }
    Ok(out)
}

/// A whole lattice number as a float's floor or ceiling allows (the scanned count bounds them).
fn whole(v: f64) -> i64 {
    v as i64
}

/// Flat-topped hexagons of side a (§2): a lattice of a/2 across and
/// h = a·√3/2 up; column c's centre at 3c + 2, row r's at 2r + 1 + (c mod 2).
fn hexagons(b: &Bounds, spec: &Spec, mut keep: Keep<'_>) -> Result<Vec<Piece>, &'static str> {
    let a = spec.side;
    let (u, h) = (a / 2.0, (a * 3f64.sqrt()) / 2.0);
    let (ox, oy) = if spec.snap {
        (start(b.min_x, 3.0 * a, true), start(b.min_y, 2.0 * h, true))
    } else {
        (b.min_x, b.min_y)
    };
    let c0 = ((b.min_x - ox) / (3.0 * u)).floor() - 1.0;
    let c1 = ((b.max_x - ox) / (3.0 * u)).ceil();
    let r0 = ((b.min_y - oy) / (2.0 * h)).floor() - 1.0;
    let r1 = ((b.max_y - oy) / (2.0 * h)).ceil();
    if !((c1 - c0 + 1.0) * (r1 - r0 + 1.0) <= MOST_SCANNED) {
        return Err(TOO_BIG_SCAN);
    }
    let (c0, c1, r0, r1) = (whole(c0), whole(c1), whole(r0), whole(r1));
    let at = |k: i64, m: i64| Vec2::new(ox + (k as f64) * u, oy + (m as f64) * h);
    let mut out = Vec::new();
    for r in (r0..=r1).rev() {
        for c in c0..=c1 {
            let (kx, ky) = (3 * c + 2, 2 * r + 1 + c.rem_euclid(2));
            let ring = vec![
                at(kx + 1, ky - 1),
                at(kx + 2, ky),
                at(kx + 1, ky + 1),
                at(kx - 1, ky + 1),
                at(kx - 2, ky),
                at(kx - 1, ky - 1),
            ];
            if meets_box(&ring, b) && keep.as_mut().is_none_or(|k| k(&ring)) {
                out.push(Piece {
                    row: Some((r1 - r + 1) as u32),
                    col: Some((c - c0 + 1) as u32),
                    pts: ring,
                });
                if out.len() > MOST {
                    return Err(TOO_BIG);
                }
            }
        }
    }
    Ok(out)
}

/// Equilateral triangles of side a (§2): rows h = a·√3/2 high, odd rows
/// shifted by a/2; in a row up and down triangles take turns.
fn triangles(b: &Bounds, spec: &Spec, mut keep: Keep<'_>) -> Result<Vec<Piece>, &'static str> {
    let a = spec.side;
    let (u, h) = (a / 2.0, (a * 3f64.sqrt()) / 2.0);
    let (ox, oy) = if spec.snap {
        (start(b.min_x, a, true), start(b.min_y, 2.0 * h, true))
    } else {
        (b.min_x, b.min_y)
    };
    let j0 = ((b.min_x - ox) / a).floor() - 1.0;
    let j1 = ((b.max_x - ox) / a).ceil();
    let r0 = ((b.min_y - oy) / h).floor();
    let r1 = js_max(((b.max_y - oy) / h).ceil() - 1.0, r0);
    if !((j1 - j0 + 1.0) * 2.0 * (r1 - r0 + 1.0) <= MOST_SCANNED) {
        return Err(TOO_BIG_SCAN);
    }
    let (j0, j1, r0, r1) = (whole(j0), whole(j1), whole(r0), whole(r1));
    let at = |k: i64, m: i64| Vec2::new(ox + (k as f64) * u, oy + (m as f64) * h);
    let mut out = Vec::new();
    for r in (r0..=r1).rev() {
        let shift = r.rem_euclid(2);
        for j in j0..=j1 {
            let x = 2 * j + shift;
            let up = vec![at(x, r), at(x + 2, r), at(x + 1, r + 1)];
            let down = vec![at(x + 2, r), at(x + 3, r + 1), at(x + 1, r + 1)];
            for (k, ring) in [(0, up), (1, down)] {
                if meets_box(&ring, b) && keep.as_mut().is_none_or(|f| f(&ring)) {
                    out.push(Piece {
                        row: Some((r1 - r + 1) as u32),
                        col: Some((2 * (j - j0) + k + 1) as u32),
                        pts: ring,
                    });
                    if out.len() > MOST {
                        return Err(TOO_BIG);
                    }
                }
            }
        }
    }
    Ok(out)
}
