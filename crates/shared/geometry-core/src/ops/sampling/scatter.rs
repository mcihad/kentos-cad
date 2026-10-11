//! Rastgele noktalar (docs/adr/0220 §4): seeded points in areas and on
//! paths, one sequence for the whole run, objects in order. In an area a
//! candidate is the first of up to 10 000 draws in the area's box that
//! falls in or on the area; on paths it is a length drawn along them. With
//! a least distance a candidate nearer a written point (squared distances,
//! the same operation everywhere) is refused, and after so many refusals
//! the point is not written.

use std::collections::HashMap;

use super::inside::Inside;
use super::random::Sequence;
use super::walk::Walk;
use crate::jsmath::js_min;
use crate::vec2::Vec2;

/// How many draws a candidate in an area may take.
pub const DRAWS: usize = 10_000;

/// What the points go in or on.
pub enum Target {
    Areas(Inside),
    Paths(Vec<Walk>),
    Nothing,
}

/// A point written: where, its elevation (on a path), its distance along its path and the path's place in the object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dot {
    pub p: Vec2,
    pub z: Option<f64>,
    pub s: Option<f64>,
    pub path: usize,
}

/// The run's points, object by object, and what could not be written: the
/// points the least distance refused, the too thin areas and the points
/// given up in them.
#[derive(Debug, Default)]
pub struct Scattered {
    pub dots: Vec<Vec<Dot>>,
    pub refused: usize,
    pub thin: usize,
    pub thin_lost: usize,
}

/// The points written so far, by cells of the least distance.
struct Written {
    min: f64,
    cells: HashMap<(i64, i64), Vec<Vec2>>,
}

impl Written {
    fn cell(&self, p: Vec2) -> (i64, i64) {
        (
            (p.x / self.min).floor() as i64,
            (p.y / self.min).floor() as i64,
        )
    }

    fn too_close(&self, p: Vec2) -> bool {
        if !(self.min > 0.0) {
            return false;
        }
        let (cx, cy) = self.cell(p);
        let d2 = self.min * self.min;
        for i in cx - 1..=cx + 1 {
            for j in cy - 1..=cy + 1 {
                if let Some(list) = self.cells.get(&(i, j))
                    && list.iter().any(|q| {
                        let (ex, ey) = (p.x - q.x, p.y - q.y);
                        ex * ex + ey * ey < d2
                    })
                {
                    return true;
                }
            }
        }
        false
    }

    fn add(&mut self, p: Vec2) {
        if self.min > 0.0 {
            let c = self.cell(p);
            self.cells.entry(c).or_default().push(p);
        }
    }
}

/// The path and the distance along it that `s` along all of them falls in: their lengths taken off in turn.
fn locate(walks: &[Walk], s: f64) -> (usize, f64) {
    let mut rest = s;
    let last = walks.len() - 1;
    for (k, w) in walks.iter().enumerate() {
        if rest < w.length || k == last {
            return (k, js_min(rest, w.length));
        }
        rest -= w.length;
    }
    (last, 0.0)
}

/// The run's points: `counts[i]` for target i, at least `min` apart, `tries` candidates a point when they must be.
pub fn scatter(targets: &[Target], counts: &[u64], min: f64, tries: u32, seed: u64) -> Scattered {
    let mut seq = Sequence::new(seed);
    let mut written = Written {
        min,
        cells: HashMap::new(),
    };
    let tries = if min > 0.0 { tries.max(1) } else { 1 };
    let mut near = Vec::new();
    let mut out = Scattered::default();
    for (target, &n) in targets.iter().zip(counts) {
        let mut dots = Vec::new();
        match target {
            Target::Areas(inside) => {
                let b = inside.bounds;
                let (w, h) = (b.max_x - b.min_x, b.max_y - b.min_y);
                let mut k = 0u64;
                'points: while k < n {
                    k += 1;
                    for _ in 0..tries {
                        let mut found = None;
                        for _ in 0..DRAWS {
                            let u = seq.unit();
                            let v = seq.unit();
                            let p = Vec2::new(b.min_x + u * w, b.min_y + v * h);
                            if inside.holds(p, &mut near) {
                                found = Some(p);
                                break;
                            }
                        }
                        let Some(p) = found else {
                            // Too thin for its box: its remaining points are given up.
                            out.thin += 1;
                            out.thin_lost += (n - k + 1) as usize;
                            break 'points;
                        };
                        if written.too_close(p) {
                            continue;
                        }
                        written.add(p);
                        dots.push(Dot {
                            p,
                            z: None,
                            s: None,
                            path: 0,
                        });
                        continue 'points;
                    }
                    out.refused += 1;
                }
            }
            Target::Paths(walks) => {
                let total: f64 = walks.iter().map(|w| w.length).sum();
                if walks.is_empty() || !(total > 0.0) {
                    out.dots.push(dots);
                    continue;
                }
                'line: for _ in 0..n {
                    for _ in 0..tries {
                        let u = seq.unit();
                        let (k, s) = locate(walks, u * total);
                        let spot = walks[k].at(s);
                        if written.too_close(spot.p) {
                            continue;
                        }
                        written.add(spot.p);
                        dots.push(Dot {
                            p: spot.p,
                            z: spot.z,
                            s: Some(s),
                            path: k,
                        });
                        continue 'line;
                    }
                    out.refused += 1;
                }
            }
            Target::Nothing => {}
        }
        out.dots.push(dots);
    }
    out
}
