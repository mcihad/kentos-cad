//! Yeniden örnekle (docs/adr/0233 §8): a raster on cells of another size
//! along its own axes. The point methods (En yakın, Çift doğrusal, Kübik)
//! read the source at each new cell's centre (`inputs::sample`); the area
//! methods take the source cells the new cell overlaps: their mean weighted
//! by the overlap, the value covering most of it, the least, the largest.

use crate::grid::Grid;
use crate::inputs::{Sampling, View};

/// The least overlap along an axis a source cell counts with (a share of a cell).
pub const LEAST_OVERLAP: f64 = 1e-9;

/// How the new cells are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Point(Sampling),
    Mean,
    Mode,
    Min,
    Max,
}

impl Method {
    pub fn from_key(key: &str) -> Option<Method> {
        Some(match key {
            "mean" => Method::Mean,
            "mode" => Method::Mode,
            "min" => Method::Min,
            "max" => Method::Max,
            other => Method::Point(Sampling::from_key(other)?),
        })
    }
}

/// The new grid (the source's top-left corner and axes, cells `cell` long
/// along them) and a new cell's size in source cells along each axis.
pub fn grid_of(source: &Grid, cell: f64) -> Result<(Grid, f64, f64), String> {
    if !(cell > 0.0) || !cell.is_finite() {
        return Err("Hücre boyu 0'dan büyük olmalı.".into());
    }
    let [x0, a, b, y0, c, d] = source.affine;
    // An axis along x or y is its own length exactly.
    let la = if c == 0.0 {
        a.abs()
    } else {
        (a * a + c * c).sqrt()
    };
    let lb = if b == 0.0 {
        d.abs()
    } else {
        (b * b + d * d).sqrt()
    };
    let (ru, rv) = (cell / la, cell / lb);
    let w = (f64::from(source.width) / ru - 1e-9).ceil().max(1.0);
    let h = (f64::from(source.height) / rv - 1e-9).ceil().max(1.0);
    if w > f64::from(u32::MAX) || h > f64::from(u32::MAX) {
        return Err(too_big(w, h));
    }
    let g = Grid::of([x0, a * ru, b * rv, y0, c * ru, d * rv], w as u32, h as u32)
        .map_err(|_| too_big(w, h))?;
    Ok((g, ru, rv))
}

fn too_big(w: f64, h: f64) -> String {
    format!(
        "Yeni hücre boyuyla raster {w} × {h} hücre olur: genişlik en çok 65 536, hücre sayısı en çok 2³¹. Hücre boyunu büyütün."
    )
}

/// The source cells new cell (i, j) overlaps, along one axis: from
/// `k·r` to `(k + 1)·r` (r a new cell in source cells), each cell's overlap.
#[inline]
fn overlaps(k: u32, r: f64, out: &mut Vec<(i64, f64)>) {
    out.clear();
    let (a, b) = (f64::from(k) * r, f64::from(k + 1) * r);
    let mut i = crate::inputs::floor_i(a);
    while (i as f64) < b {
        let ov = b.min(i as f64 + 1.0) - a.max(i as f64);
        if ov > LEAST_OVERLAP {
            out.push((i, ov));
        }
        i += 1;
    }
}

/// The source rows or columns new cells `k0..k1` overlap (one more each way).
pub fn reach(k0: u32, k1: u32, r: f64) -> (i64, i64) {
    (
        (f64::from(k0) * r).floor() as i64 - 1,
        (f64::from(k1) * r).ceil() as i64 + 1,
    )
}

/// Scratch for the area methods.
#[derive(Default)]
pub struct Scratch {
    us: Vec<(i64, f64)>,
    vs: Vec<(i64, f64)>,
    pairs: Vec<(f64, f64)>,
}

/// Band `b`'s value of new cell (i, j) by an area method (`view` holds the
/// source cells it overlaps); NaN when none of them has a value.
pub fn area_value(
    view: &View,
    b: usize,
    (i, j): (u32, u32),
    (ru, rv): (f64, f64),
    method: Method,
    s: &mut Scratch,
) -> f64 {
    overlaps(i, ru, &mut s.us);
    overlaps(j, rv, &mut s.vs);
    match method {
        Method::Mean => {
            let (mut acc, mut wsum) = (0.0, 0.0);
            for &(y, wy) in &s.vs {
                for &(x, wx) in &s.us {
                    let v = view.at(b, x, y);
                    if !v.is_nan() {
                        let w = wx * wy;
                        acc += w * v;
                        wsum += w;
                    }
                }
            }
            if wsum > 0.0 { acc / wsum } else { f64::NAN }
        }
        Method::Min | Method::Max => {
            let mut best = f64::NAN;
            for &(y, _) in &s.vs {
                for &(x, _) in &s.us {
                    let v = view.at(b, x, y);
                    if !v.is_nan()
                        && (best.is_nan()
                            || (method == Method::Min && v < best)
                            || (method == Method::Max && v > best))
                    {
                        best = v;
                    }
                }
            }
            best
        }
        Method::Mode => {
            s.pairs.clear();
            for &(y, wy) in &s.vs {
                for &(x, wx) in &s.us {
                    let v = view.at(b, x, y);
                    if !v.is_nan() {
                        s.pairs.push((v + 0.0, wx * wy));
                    }
                }
            }
            s.pairs.sort_by(|p, q| p.0.total_cmp(&q.0));
            let (mut best, mut best_w) = (f64::NAN, -1.0);
            let mut k = 0;
            while k < s.pairs.len() {
                let v = s.pairs[k].0;
                let mut w = 0.0;
                while k < s.pairs.len() && s.pairs[k].0 == v {
                    w += s.pairs[k].1;
                    k += 1;
                }
                if w > best_w {
                    best_w = w;
                    best = v;
                }
            }
            best
        }
        Method::Point(_) => f64::NAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_new_grid_keeps_the_corner_and_the_axes() {
        let g = Grid::of([1000.0, 2.0, 0.0, 5000.0, 0.0, -2.0], 10, 7).unwrap();
        let (n, ru, rv) = grid_of(&g, 5.0).unwrap();
        assert_eq!((ru, rv), (2.5, 2.5));
        // 20 m by 14 m: 4 by 3 cells of 5 m (the last row past the source).
        assert_eq!((n.width, n.height), (4, 3));
        assert_eq!(n.affine, [1000.0, 5.0, 0.0, 5000.0, 0.0, -5.0]);
        // Finer: 1 m cells.
        let (n, _, _) = grid_of(&g, 1.0).unwrap();
        assert_eq!((n.width, n.height), (20, 14));
        assert!(grid_of(&g, 0.0).is_err());
    }

    #[test]
    fn area_methods_weigh_the_overlaps() {
        // 3 × 1 source cells 0, 10, 20 with a new cell of 1.5: cells 0 (1) and 1 (0.5).
        let view = View {
            x: 0,
            y: 0,
            w: 3,
            h: 1,
            bands: vec![vec![0.0, 10.0, 20.0]],
        };
        let mut s = Scratch::default();
        let r = (1.5, 1.0);
        assert_eq!(
            area_value(&view, 0, (0, 0), r, Method::Mean, &mut s),
            10.0 / 3.0
        );
        // The second: cell 1 (0.5) and cell 2 (1).
        assert_eq!(
            area_value(&view, 0, (1, 0), r, Method::Mean, &mut s),
            25.0 / 1.5
        );
        assert_eq!(area_value(&view, 0, (1, 0), r, Method::Mode, &mut s), 20.0);
        assert_eq!(area_value(&view, 0, (1, 0), r, Method::Min, &mut s), 10.0);
        assert_eq!(area_value(&view, 0, (0, 0), r, Method::Max, &mut s), 10.0);
    }
}
