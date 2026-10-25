//! Hizmet alanı (docs/adr/0209 §6): what the facilities reach within each
//! break. A piece's place `x` costs the least of reaching it from either
//! end: `g(x)` from its start, `h(x)` from its end; they meet once, so each
//! band is at most one stretch on each side of the meeting point (one when
//! both sides of it are in the band), found exactly from the costs at the
//! piece's ends (a facility or a barrier inside
//! a piece cuts it there). The lines of a band are the network it reaches;
//! its area is those lines buffered by the trim distance and joined (the
//! overlay of ADR 0201, arcs exact), a ring less the band before it.

use super::blocked::Blocker;
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
    /// The time of day the facilities are set off from (Tesise: arrived at), minutes, for a cost with a time profile.
    pub at: Option<f64>,
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

/// `bands_on` with a time profile (docs/adr/0521 §4): a place's cost from either end is the time to it from the
/// time that end is set off at (a reverse search: arrived at), not proportional to the way; `[dg, dh, fw, bw]` the
/// ends' costs and the piece's whole costs each way (infinite where it cannot be travelled). The meeting point is found
/// by halving (the costs from the ends rise and fall along the stretch); a band's ends by what is gone in its time.
fn bands_timed(
    g: &Graph,
    t: &Tree,
    piece: u32,
    [dg, dh, fw, bw]: [f64; 4],
    (a, b): (f64, f64),
    breaks: &[f64],
    mut out: impl FnMut(usize, f64, f64),
) {
    let g_ok = dg.is_finite() && fw.is_finite();
    let h_ok = dh.is_finite() && bw.is_finite();
    if !g_ok && !h_ok {
        return;
    }
    let gx = |x: f64| dg + t.go(g, piece, true, x - a, dg);
    let hx = |x: f64| dh + t.go(g, piece, false, b - x, dh);
    let m = match (g_ok, h_ok) {
        (true, false) => b,
        (false, true) => a,
        _ if gx(b) <= hx(b) => b,
        _ if gx(a) > hx(a) => a,
        _ => {
            let (mut lo, mut hi) = (a, b);
            for _ in 0..200 {
                let mid = lo + (hi - lo) / 2.0;
                if mid <= lo || mid >= hi {
                    break;
                }
                if gx(mid) <= hx(mid) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            lo
        }
    };
    // What is gone in `dt` from an end set off at its cost; nothing for a time before it.
    let gone_g = |dt: f64| {
        if dt < 0.0 {
            f64::NEG_INFINITY
        } else {
            t.reach(g, piece, true, dt, dg)
        }
    };
    let gone_h = |dt: f64| {
        if dt < 0.0 {
            f64::NEG_INFINITY
        } else {
            t.reach(g, piece, false, dt, dh)
        }
    };
    let mut lo = f64::NEG_INFINITY;
    for (k, &hi) in breaks.iter().enumerate() {
        let from_start = (g_ok && a < m).then(|| {
            let (x0, x1) = (a + gone_g(lo - dg), a + gone_g(hi - dg));
            (x0.clamp(a, m), x1.clamp(a, m))
        });
        let from_end = (h_ok && m < b).then(|| {
            let (x0, x1) = (b - gone_h(hi - dh), b - gone_h(lo - dh));
            (x0.clamp(m, b), x1.clamp(m, b))
        });
        match (
            from_start.filter(|s| s.1 > s.0),
            from_end.filter(|e| e.1 > e.0),
        ) {
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
    let last = breaks.last().copied().unwrap_or(f64::INFINITY);
    for (k, p) in g.pieces.iter().enumerate() {
        let piece = k as u32;
        // A piece neither of whose ends the search reached within the last break, and that no facility or barrier
        // splits, has no band: a small area of a large network passes most pieces at two lookups.
        if !(t.at(g, p.from) < last)
            && !(t.at(g, p.to) < last)
            && t.places.split_of(piece).is_none()
        {
            continue;
        }
        // The piece's stretches between the search's places on it.
        t.places.cuts_into(g, piece, &mut ends);
        let fw = g.cost(piece, c, !t.reverse);
        let bw = g.cost(piece, c, t.reverse);
        let (rf, rb) = (fw / p.len, bw / p.len);
        for w in ends.windows(2) {
            let ((na, a), (nb, b)) = (w[0], w[1]);
            // The costs of setting off along the stretch from either end (with turns, the turn onto it).
            let (dg, dh) = (t.enter(g, na, piece, true), t.enter(g, nb, piece, false));
            let mut push = |band, x0, x1| {
                out.push(AreaLine {
                    facility,
                    band,
                    span: Span {
                        piece,
                        a: x0,
                        b: x1,
                    },
                })
            };
            if t.time.is_some() {
                bands_timed(g, t, piece, [dg, dh, fw, bw], (a, b), breaks, push);
            } else {
                bands_on([dg, dh, rf, rb], (a, b), breaks, &mut push);
            }
        }
    }
}

/// The lines of a service area: each band's stretches, by facility when `separate`.
pub fn service_lines(g: &Graph, s: &mut Searcher, q: &AreaQuery) -> Vec<AreaLine> {
    service_lines_cut(g, s, q, false).0
}

/// The lines of a service area and, when `cut`, what cuts it off (docs/adr/0640 §2): the facilities' one search
/// gives both when they are merged.
pub fn service_lines_cut(
    g: &Graph,
    s: &mut Searcher,
    q: &AreaQuery,
    cut: bool,
) -> (Vec<AreaLine>, AreaCut) {
    let mut out = Vec::new();
    let cutoff = q.breaks.last().copied();
    let ask = |origins| Query {
        reverse: q.toward,
        barriers: q.barriers,
        cutoff,
        at: q.at,
        ..Query::from(origins, q.cost)
    };
    let mut said = AreaCut::default();
    if q.separate {
        for f in 0..q.facilities.len() {
            let t = s.search(g, &ask(&q.facilities[f..f + 1]));
            tree_lines(g, &t, q.breaks, Some(f), &mut out);
            s.recycle(t);
        }
        if cut {
            said = service_cut(g, s, q);
        }
    } else {
        let t = s.search(g, &ask(q.facilities));
        tree_lines(g, &t, q.breaks, None, &mut out);
        if cut {
            said = cut_of(g, &t, q);
        }
        s.recycle(t);
    }
    (out, said)
}

/// What cuts a service area off (docs/adr/0640 §2): each blocker on the edge of the area within its last break, in
/// the order the area meets it (its cost, then the network's order), and the edges on parts of the network no facility
/// reaches by any way, even with nothing closed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AreaCut {
    pub blockers: Vec<Blocker>,
    pub apart: usize,
}

impl AreaCut {
    /// What cuts the area off in words (the first few blockers, then how many more), and the edges no facility meets;
    /// none when nothing does.
    pub fn words(&self) -> Option<String> {
        const SAID: usize = 5;
        let mut parts = Vec::new();
        if !self.blockers.is_empty() {
            let said: Vec<String> = self
                .blockers
                .iter()
                .take(SAID)
                .map(Blocker::words)
                .collect();
            let more = self.blockers.len().saturating_sub(SAID);
            let tail = if more > 0 {
                format!(" ve {more} tane daha")
            } else {
                String::new()
            };
            parts.push(format!(
                "alanı son aralığın içinde kesenler: {}{tail}",
                said.join(", ")
            ));
        }
        if self.apart > 0 {
            parts.push(format!(
                "{} kenar hiçbir tesise bağlanmayan parçalarda",
                self.apart
            ));
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }
}

impl crate::api::json::ToJson for AreaCut {
    fn write_json(&self, out: &mut String) {
        out.push_str(r#"{"blocked":"#);
        out.push_str(&super::blocked::by_json(&self.blockers));
        out.push_str(&format!(r#","apart":{}}}"#, self.apart));
    }
}

/// The blockers on the edge of `q`'s area (one search of the facilities together, to the last break): each stretch
/// of a piece the area does not reach from either end, set off from a node the area reaches below the last break,
/// says what stops it there. The piece's way closed from that node: the edge and why; open but not entered: the turn
/// (a device's terminals) from the way the node is reached by. A node passed by nothing (a closed junction, a barrier)
/// the area arrives at within the break says itself. And each node the area reaches whose level keeps a node beside it
/// apart that the area does not reach.
pub fn service_cut(g: &Graph, s: &mut Searcher, q: &AreaQuery) -> AreaCut {
    let Some(&last) = q.breaks.last() else {
        return AreaCut::default();
    };
    let t = s.search(
        g,
        &Query {
            reverse: q.toward,
            barriers: q.barriers,
            cutoff: Some(last),
            at: q.at,
            ..Query::from(q.facilities, q.cost)
        },
    );
    let cut = cut_of(g, &t, q);
    s.recycle(t);
    cut
}

/// [`service_cut`] from the facilities' merged search `t` (to the last break).
fn cut_of(g: &Graph, t: &Tree, q: &AreaQuery) -> AreaCut {
    let Some(&last) = q.breaks.last() else {
        return AreaCut::default();
    };
    let c = t.cost;
    let inside = |d: f64| d < last;
    let edge_id = |p: u32| g.edges[g.pieces[p as usize].edge as usize].id;
    let first_barrier = q.facilities.len();
    let mut found: Vec<(f64, Blocker)> = Vec::new();
    let mut ends: Vec<(u32, f64)> = Vec::new();
    for k in 0..g.pieces.len() {
        let piece = k as u32;
        // A piece whose ends the area reaches neither within the break, and that no facility or barrier splits,
        // says nothing: most of a large network is passed over at two lookups.
        let p = &g.pieces[k];
        if !inside(t.at(g, p.from)) && !inside(t.at(g, p.to)) && t.places.split_of(piece).is_none()
        {
            continue;
        }
        t.places.cuts_into(g, piece, &mut ends);
        // The cost of going along the piece the way it is drawn (`true`) or against it, as the search travels it.
        let way = |frame_forward: bool| g.cost(piece, c, frame_forward != q.toward);
        let len = g.pieces[k].len;
        for w in ends.windows(2) {
            let ((na, oa), (nb, ob)) = (w[0], w[1]);
            let (dg, dh) = (t.enter(g, na, piece, true), t.enter(g, nb, piece, false));
            let from_a = dg.is_finite() && way(true).is_finite() && inside(dg);
            let from_b = dh.is_finite() && way(false).is_finite() && inside(dh);
            // A node passed by nothing at the stretch's far end, arrived at within the break (the search never
            // settles it): its barrier or closed junction.
            let share = if len > 0.0 { (ob - oa) / len } else { 0.0 };
            for (entered, d, far, frame_forward) in
                [(from_a, dg, nb, true), (from_b, dh, na, false)]
            {
                let arrive = d + way(frame_forward) * share;
                if !entered || !t.places.blocked(g, far) || !inside(arrive) {
                    continue;
                }
                if let Some(i) = (0..q.barriers.len())
                    .find(|&i| t.places.node_of.get(first_barrier + i) == Some(&far))
                {
                    found.push((arrive, Blocker::Barrier(i)));
                } else if (far as usize) < g.nodes.len()
                    && let Some(id) = super::blocked::junction_at(g, far)
                {
                    found.push((arrive, Blocker::Junction(id)));
                }
            }
            if from_a || from_b {
                continue;
            }
            for (n, frame_forward) in [(na, true), (nb, false)] {
                let at = t.at(g, n);
                if !inside(at) {
                    continue;
                }
                let blocker = if way(frame_forward).is_infinite() {
                    // The way the area would go along it: from the node in a search from the facilities, toward the
                    // node in one toward them.
                    Blocker::Edge(
                        edge_id(piece),
                        super::blocked::why_of(g, piece, frame_forward != q.toward),
                    )
                } else {
                    let Some(by) = t.arrived_by(g, n) else {
                        continue;
                    };
                    // The arc the stretch is gone along from the node, in the vehicle's direction.
                    let arc = piece * 2 + u32::from(!frame_forward);
                    let (from, to) = if q.toward { (arc ^ 1, by) } else { (by, arc) };
                    if !g.crosses(n, from, to)
                        && let Some(id) = super::blocked::junction_at(g, n)
                    {
                        Blocker::Device(id, edge_id(from / 2), edge_id(to / 2))
                    } else {
                        Blocker::Turn(edge_id(from / 2), edge_id(to / 2))
                    }
                };
                found.push((at, blocker));
            }
        }
    }
    // The levels: a node the area reaches beside one it does not (docs/adr/0640 §3).
    for n in 0..if g.has_level_gaps() {
        g.nodes.len() as u32
    } else {
        0
    } {
        let at = t.at(g, n);
        if !inside(at) {
            continue;
        }
        for &m in g.level_gaps(n) {
            if inside(t.at(g, m)) {
                continue;
            }
            let (Some(&a), Some(&b)) = (g.at_node(n).first(), g.at_node(m).first()) else {
                continue;
            };
            found.push((at, Blocker::Level(edge_id(a / 2), edge_id(b / 2))));
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut blockers: Vec<Blocker> = Vec::new();
    for (_, b) in found {
        if !blockers.contains(&b) {
            blockers.push(b);
        }
    }
    AreaCut {
        blockers,
        apart: g.edges_apart_from(
            &q.facilities
                .iter()
                .map(|f| g.pieces[f.piece as usize].from)
                .collect::<Vec<_>>(),
        ),
    }
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
