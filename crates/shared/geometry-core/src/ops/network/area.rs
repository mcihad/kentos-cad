//! Hizmet alanı (docs/adr/0209 §6): what the facilities reach within each
//! break. A piece's place `x` costs the least of reaching it from either
//! end: `g(x)` from its start, `h(x)` from its end; they meet once, so each
//! band is at most one stretch on each side of the meeting point (one when
//! both sides of it are in the band), found exactly from the costs at the
//! piece's ends (a facility or a barrier inside
//! a piece cuts it there). The lines of a band are the network it reaches;
//! its area is those lines buffered by the trim distance and joined (the
//! overlay of ADR 0201, arcs exact), a ring less the band before it.

use super::graph::{Graph, Location};
use super::search::{Query, Searcher, Span, Tree};
use crate::geom::arrangement::Area;
use crate::ops::geoprocess::buffer::union_pieces;
use crate::ops::geoprocess::{subtract_all, union_buffers};

/// A stretch of the network a facility (none: the facilities together) reaches within band `band`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AreaLine {
    pub facility: Option<usize>,
    pub band: usize,
    pub span: Span,
}

/// A service area's question.
#[derive(Clone, Copy, Debug)]
pub struct AreaQuery<'a> {
    pub facilities: &'a [Location],
    /// Ascending, above zero, in the cost's unit.
    pub breaks: &'a [f64],
    pub cost: usize,
    /// Tesise: the costs of reaching the facilities rather than from them.
    pub toward: bool,
    /// Ayrı: each facility on its own; else together (each place by its cheapest facility).
    pub separate: bool,
    pub barriers: &'a [Location],
}

/// The band a cost falls in: the first break it does not exceed.
fn band_of(breaks: &[f64], c: f64) -> Option<usize> {
    breaks.iter().position(|b| c <= *b)
}

/// The stretches of `[a, b]` on a piece in each band, given `[dg, dh, rf, rb]`: the costs at its ends and the
/// per-metre rates travelling it forward from `a` (`rf`) and backward from `b` (`rb`), each infinite where it cannot be
/// travelled.
fn bands_on(
    costs: [f64; 4],
    (a, b): (f64, f64),
    breaks: &[f64],
    mut out: impl FnMut(usize, f64, f64),
) {
    let [dg, dh, rf, rb] = costs;
    let g_ok = dg.is_finite() && rf.is_finite();
    let h_ok = dh.is_finite() && rb.is_finite();
    if !g_ok && !h_ok {
        return;
    }
    // Where g and h meet: before it g is the lesser, after it h.
    let m = match (g_ok, h_ok) {
        (true, false) => b,
        (false, true) => a,
        _ if rf + rb > 0.0 => ((dh - dg + rf * a + rb * b) / (rf + rb)).clamp(a, b),
        _ => {
            if dh < dg {
                a
            } else {
                b
            }
        }
    };
    let mut lo = f64::NEG_INFINITY;
    for (k, &hi) in breaks.iter().enumerate() {
        // From the start: lo < g(x) ≤ hi on [a, m].
        let from_start = (g_ok && a < m).then(|| {
            let (x0, x1) = if rf > 0.0 {
                (a + (lo - dg) / rf, a + (hi - dg) / rf)
            } else if lo < dg && dg <= hi {
                (a, m)
            } else {
                (m, m)
            };
            (x0.clamp(a, m), x1.clamp(a, m))
        });
        // From the end: lo < h(x) ≤ hi on [m, b].
        let from_end = (h_ok && m < b).then(|| {
            let (x0, x1) = if rb > 0.0 {
                (b - (hi - dh) / rb, b - (lo - dh) / rb)
            } else if lo < dh && dh <= hi {
                (m, b)
            } else {
                (m, m)
            };
            (x0.clamp(m, b), x1.clamp(m, b))
        });
        match (
            from_start.filter(|s| s.1 > s.0),
            from_end.filter(|e| e.1 > e.0),
        ) {
            // The two sides of the meeting point in one band are one stretch.
            (Some(s), Some(e)) if s.1 == m && e.0 == m => out(k, s.0, e.1),
            (s, e) => {
                if let Some(s) = s {
                    out(k, s.0, s.1);
                }
                if let Some(e) = e {
                    out(k, e.0, e.1);
                }
            }
        }
        lo = hi;
    }
}

/// The lines a tree reaches within `breaks`, piece by piece in the pieces' order.
fn tree_lines(
    g: &Graph,
    t: &Tree,
    breaks: &[f64],
    facility: Option<usize>,
    out: &mut Vec<AreaLine>,
) {
    let c = t.cost;
    let mut ends: Vec<(u32, f64)> = Vec::new();
    for (k, p) in g.pieces.iter().enumerate() {
        let piece = k as u32;
        // The piece's stretches between the search's places on it.
        t.places.cuts_into(g, piece, &mut ends);
        let fw = g.cost(piece, c, !t.reverse);
        let bw = g.cost(piece, c, t.reverse);
        let (rf, rb) = (fw / p.len, bw / p.len);
        for w in ends.windows(2) {
            let ((na, a), (nb, b)) = (w[0], w[1]);
            bands_on(
                [t.at(na), t.at(nb), rf, rb],
                (a, b),
                breaks,
                |band, x0, x1| {
                    out.push(AreaLine {
                        facility,
                        band,
                        span: Span {
                            piece,
                            a: x0,
                            b: x1,
                        },
                    })
                },
            );
        }
    }
}

/// The lines of a service area: each band's stretches, by facility when `separate`.
pub fn service_lines(g: &Graph, s: &mut Searcher, q: &AreaQuery) -> Vec<AreaLine> {
    let mut out = Vec::new();
    let cutoff = q.breaks.last().copied();
    let ask = |origins| Query {
        reverse: q.toward,
        barriers: q.barriers,
        cutoff,
        ..Query::from(origins, q.cost)
    };
    if q.separate {
        for f in 0..q.facilities.len() {
            let t = s.search(g, &ask(&q.facilities[f..f + 1]));
            tree_lines(g, &t, q.breaks, Some(f), &mut out);
            s.recycle(t);
        }
    } else {
        let t = s.search(g, &ask(q.facilities));
        tree_lines(g, &t, q.breaks, None, &mut out);
        s.recycle(t);
    }
    out
}

/// The cost at which a line's band starts and ends (the break before it, or 0, and its own).
pub fn band_bounds(breaks: &[f64], band: usize) -> (f64, f64) {
    (if band == 0 { 0.0 } else { breaks[band - 1] }, breaks[band])
}

/// The areas of a service area: for each facility (none: together) and band, the lines of that band and the bands
/// below it buffered by `trim` and joined; `rings` takes each band less the one below it. Each band's disc is joined
/// from its lines' buffers, a piece's touching stretches as one: an overlay's own result is never joined again with
/// buffers whose end circles lie on its boundary (TODOS.md `GEO-01`).
pub fn service_polygons(
    g: &Graph,
    lines: &[AreaLine],
    bands: usize,
    trim: f64,
    rings: bool,
) -> Vec<(Option<usize>, usize, Vec<Area>)> {
    let mut groups: Vec<Option<usize>> = Vec::new();
    for l in lines {
        if !groups.contains(&l.facility) {
            groups.push(l.facility);
        }
    }
    let mut out = Vec::new();
    let mut edges = Vec::new();
    for f in groups {
        let mut below: Vec<Area> = Vec::new();
        for band in 0..bands {
            // The stretches at or below the band, a piece's touching ones joined.
            let mut reach: Vec<(u32, f64, f64)> = lines
                .iter()
                .filter(|l| l.facility == f && l.band <= band)
                .map(|l| (l.span.piece, l.span.a, l.span.b))
                .collect();
            if !lines.iter().any(|l| l.facility == f && l.band == band) {
                continue;
            }
            reach.sort_by(|x, y| x.0.cmp(&y.0).then(x.1.total_cmp(&y.1)));
            let mut joined: Vec<(u32, f64, f64)> = Vec::with_capacity(reach.len());
            for r in reach {
                match joined.last_mut() {
                    Some(j) if j.0 == r.0 && r.1 <= j.2 => {
                        if r.2 > j.2 {
                            j.2 = r.2;
                        }
                    }
                    _ => joined.push(r),
                }
            }
            let mut cores = Vec::new();
            for (piece, a, b) in joined {
                edges.clear();
                g.span_edges(piece, a, b, &mut edges);
                cores.extend_from_slice(&edges);
            }
            let disc = union_buffers(&union_pieces(&cores, trim), &cores, trim);
            let shape = if rings && !below.is_empty() {
                subtract_all(&disc, &below)
            } else {
                disc.clone()
            };
            below = disc;
            if !shape.is_empty() {
                out.push((f, band, shape));
            }
        }
    }
    out
}

/// Where `band_of` puts cost `c` (for the tools' notes).
pub fn band_for(breaks: &[f64], c: f64) -> Option<usize> {
    band_of(breaks, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stretches(dg: f64, dh: f64, rf: f64, rb: f64, breaks: &[f64]) -> Vec<(usize, f64, f64)> {
        let mut out = Vec::new();
        bands_on([dg, dh, rf, rb], (0.0, 100.0), breaks, |k, a, b| {
            out.push((k, a, b))
        });
        out
    }

    #[test]
    fn a_piece_reached_from_both_ends_splits_its_bands_where_the_costs_meet() {
        // Reached at 0 from the start and at 40 from the end, a metre a metre: they meet at 70; the end's last
        // 10 m are within 50 too.
        let s = stretches(0.0, 40.0, 1.0, 1.0, &[50.0, 100.0]);
        // Band 1 meets itself at 70: one stretch.
        assert_eq!(s, vec![(0, 0.0, 50.0), (0, 90.0, 100.0), (1, 50.0, 90.0)]);
        // From the start only (one way), within 30: the first 30 m.
        assert_eq!(
            stretches(0.0, f64::INFINITY, 1.0, 1.0, &[30.0]),
            vec![(0, 0.0, 30.0)]
        );
        // Not reached within the breaks at all.
        assert!(stretches(200.0, 300.0, 1.0, 1.0, &[50.0]).is_empty());
        // A free piece (cost 0) is in the band of its ends' cost.
        assert_eq!(
            stretches(10.0, f64::INFINITY, 0.0, 0.0, &[5.0, 20.0]),
            vec![(1, 0.0, 100.0)]
        );
    }
}
