//! Düzenli noktalar (docs/adr/0220 §3): the lattice's points in each area
//! (in or on it, holes out), on multiples of the spacing or at the middles
//! of cells from the areas' corner, each point shifted by its own seeded
//! numbers when asked.

use super::MOST;
use super::inside::Inside;
use super::random::at_lattice;
use crate::geometry::Bounds;
use crate::vec2::Vec2;

/// The most lattice points the tool tries.
pub const MOST_TRIED: f64 = 16_000_000.0;

pub const TOO_MANY_TRIED: &str =
    "Aralık bu alanlar için çok küçük: 16 000 000'dan çok kafes noktası sınanırdı.";
pub const TOO_MANY: &str = "1 000 000'dan çok nokta yazılırdı; aralığı büyütün.";

/// The lattice: point (i, j) at `(ox + (i + half) · dx, oy + (j + half) · dy)`.
#[derive(Clone, Copy, Debug)]
pub struct Lattice {
    pub ox: f64,
    pub oy: f64,
    pub dx: f64,
    pub dy: f64,
    pub half: f64,
}

impl Lattice {
    /// On multiples (`snap`), or at the middles of cells from the corner of `all`.
    pub fn of(dx: f64, dy: f64, snap: bool, all: &Bounds) -> Lattice {
        if snap {
            Lattice {
                ox: 0.0,
                oy: 0.0,
                dx,
                dy,
                half: 0.0,
            }
        } else {
            Lattice {
                ox: all.min_x,
                oy: all.min_y,
                dx,
                dy,
                half: 0.5,
            }
        }
    }

    pub fn point(&self, i: i64, j: i64) -> Vec2 {
        Vec2::new(
            self.ox + (i as f64 + self.half) * self.dx,
            self.oy + (j as f64 + self.half) * self.dy,
        )
    }

    /// The lattice numbers whose points, shifted by up to `reach`, can fall in `lo …= hi` on an axis.
    fn range(lo: f64, hi: f64, o: f64, step: f64, half: f64, reach: f64) -> (f64, f64) {
        (
            ((lo - reach - o) / step - half).floor(),
            ((hi + reach - o) / step - half).ceil(),
        )
    }

    fn ranges(&self, b: &Bounds, reach: f64) -> ((f64, f64), (f64, f64)) {
        (
            Lattice::range(b.min_x, b.max_x, self.ox, self.dx, self.half, reach),
            Lattice::range(b.min_y, b.max_y, self.oy, self.dy, self.half, reach),
        )
    }
}

/// Each object's points (none for an object with no area); `shift` moves
/// each point up to that far on each axis by its seeded numbers.
pub fn regular(
    objects: &[Option<Inside>],
    dx: f64,
    dy: f64,
    snap: bool,
    shift: f64,
    seed: u64,
) -> Result<Vec<Vec<Vec2>>, &'static str> {
    let mut all: Option<Bounds> = None;
    for o in objects.iter().flatten() {
        let b = o.bounds;
        all = Some(match all {
            None => b,
            Some(a) => Bounds {
                min_x: crate::jsmath::js_min(a.min_x, b.min_x),
                min_y: crate::jsmath::js_min(a.min_y, b.min_y),
                max_x: crate::jsmath::js_max(a.max_x, b.max_x),
                max_y: crate::jsmath::js_max(a.max_y, b.max_y),
            },
        });
    }
    let Some(all) = all else {
        return Ok(objects.iter().map(|_| Vec::new()).collect());
    };
    let lattice = Lattice::of(dx, dy, snap, &all);
    let reach = if shift > 0.0 { shift } else { 0.0 };
    let mut tried = 0.0;
    for o in objects.iter().flatten() {
        let ((i0, i1), (j0, j1)) = lattice.ranges(&o.bounds, reach);
        tried += (i1 - i0 + 1.0) * (j1 - j0 + 1.0);
    }
    if !(tried <= MOST_TRIED) {
        return Err(TOO_MANY_TRIED);
    }
    let mut near = Vec::new();
    let mut written = 0usize;
    let mut out = Vec::with_capacity(objects.len());
    for o in objects {
        let mut pts = Vec::new();
        if let Some(o) = o {
            let ((i0, i1), (j0, j1)) = lattice.ranges(&o.bounds, reach);
            let (i0, i1, j0, j1) = (i0 as i64, i1 as i64, j0 as i64, j1 as i64);
            for j in (j0..=j1).rev() {
                for i in i0..=i1 {
                    let mut p = lattice.point(i, j);
                    if shift > 0.0 {
                        let mut seq = at_lattice(seed, i, j);
                        let (ux, uy) = (seq.unit(), seq.unit());
                        p = Vec2::new(
                            p.x + (2.0 * ux - 1.0) * shift,
                            p.y + (2.0 * uy - 1.0) * shift,
                        );
                    }
                    if o.holds(p, &mut near) {
                        pts.push(p);
                        written += 1;
                        if written > MOST {
                            return Err(TOO_MANY);
                        }
                    }
                }
            }
        }
        out.push(pts);
    }
    Ok(out)
}
