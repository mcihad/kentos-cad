//! Layers that turn at a wall's ends (docs/adr/0771 §3, Uçta döner): at an open wall's free end (its end cap) and at a
//! door's or a window's jambs (the reveal, söve) the components marked `wrap`, the outermost ones of each face, turn
//! across the end; the components inside stop short of it by their thickness. One meaning for the end cap and the
//! opening's return, so plaster and insulation run round both alike.
//!
//! Distances across the wall are to its path's left (`o[0]` the left face down to `o[n]` the right face); along it,
//! `pull` is how far before the end (the jamb or the path's end) something stops. With `L` turning layers from the left
//! face and `R` from the right, their thicknesses' sums `TL` and `TR`, the turns meet at `m`: halfway between the
//! innermost turning layers' inner lines when both faces turn, the far face when one does.
//!
//! - The left layer `c` (`c < L`) runs to `a[c]` (the layers outside it's thickness) and turns down to `m` between
//!   `a[c]` and `a[c + 1]`; the right layers mirror it.
//! - A line between two left turning layers, or a turning layer and the inside, stops where its outer layer turns
//!   (`a[k]`); an inner component's lines stop at `TL` above `m` and `TR` below it; the faces run to the end.
//! - Across the end, at each turning layer's inner side, a closing line from its inner line to `m`; along `m` a line
//!   where the two faces' turns meet.
//!
//! The walls are cut on their base line's arc length, so arcs turn as straight walls do.

use super::{Body, Cut, Line, Path, Solver, Span, line_path, nearest_s};
use crate::geom::opening::beside;
use crate::geom::wall::{self, Spec};
use crate::jsmath::js_max;

/// A stretch of the wall near an end, `p0` to `p1` before it, between the distances `upper` and `lower` to the path's
/// left: a turning layer's turn, or what an inner component keeps where the two faces' turns differ.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Patch {
    pub comp: usize,
    pub p0: f64,
    pub p1: f64,
    pub upper: f64,
    pub lower: f64,
}

/// How a wall's layers turn at its ends; none when none turns.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Wraps {
    /// Per line, how far before the end it stops.
    pub pull: Vec<f64>,
    /// Per component, where its band stops.
    pub band: Vec<f64>,
    pub patches: Vec<Patch>,
    /// Closing lines across the end: the component it belongs to, its pull, from `upper` to `lower`.
    pub closures: Vec<(usize, f64, f64, f64)>,
    /// The line along `m` from the end to this pull, and the component it is drawn with; none when one face turns.
    pub middle: Option<(usize, f64, f64)>,
    /// The deepest pull.
    pub reach: f64,
}

/// The wall's turning layers' plan; none when no component turns, the components do not fit, or the turning layers are
/// not the faces' outermost (the contract refuses those).
pub(super) fn plan(spec: &Spec) -> Option<Wraps> {
    let comps = &spec.components;
    let n = comps.len();
    if n < 2 || !comps.iter().any(|c| c.wrap) {
        return None;
    }
    let t = wall::thicknesses(spec)?;
    let o = wall::boundaries(spec);
    let l = comps.iter().take_while(|c| c.wrap).count();
    let r = comps.iter().rev().take_while(|c| c.wrap).count();
    if l + r >= n || comps[l..n - r].iter().any(|c| c.wrap) {
        return None;
    }
    let mut a = vec![0.0; n + 1];
    for j in 0..l {
        a[j + 1] = a[j] + t[j];
    }
    let mut b = vec![0.0; n + 1];
    for k in (n - r..n).rev() {
        b[k] = b[k + 1] + t[k];
    }
    let (tl, tr) = (a[l], b[n - r]);
    let m = match (l > 0, r > 0) {
        (true, true) => (o[l] + o[n - r]) / 2.0,
        (true, false) => o[n],
        _ => o[0],
    };
    let eps = 1e-9 * js_max(1.0, js_max(o[0].abs(), o[n].abs()));
    let inner = |d: f64| {
        if d > m + eps {
            tl
        } else if d < m - eps {
            tr
        } else {
            js_max(tl, tr)
        }
    };
    let pull = (0..=n)
        .map(|k| {
            if k == 0 || k == n {
                0.0
            } else if k <= l {
                a[k]
            } else if k >= n - r {
                b[k]
            } else {
                inner(o[k])
            }
        })
        .collect();
    let mut patches = Vec::new();
    let mut band = Vec::with_capacity(n);
    for c in 0..n {
        if c < l {
            band.push(a[c]);
            patches.push(Patch {
                comp: c,
                p0: a[c],
                p1: a[c + 1],
                upper: o[c + 1],
                lower: m,
            });
        } else if c >= n - r {
            band.push(b[c + 1]);
            patches.push(Patch {
                comp: c,
                p0: b[c + 1],
                p1: b[c],
                upper: m,
                lower: o[c],
            });
        } else if o[c + 1] >= m - eps {
            band.push(tl);
        } else if o[c] <= m + eps {
            band.push(tr);
        } else {
            band.push(js_max(tl, tr));
            if tl < tr {
                patches.push(Patch {
                    comp: c,
                    p0: tl,
                    p1: tr,
                    upper: o[c],
                    lower: m,
                });
            } else if tr < tl {
                patches.push(Patch {
                    comp: c,
                    p0: tr,
                    p1: tl,
                    upper: m,
                    lower: o[c + 1],
                });
            }
        }
    }
    let mut closures = Vec::new();
    for j in 1..=l {
        closures.push((j - 1, a[j], o[j], m));
    }
    for k in n - r..n {
        closures.push((k, b[k], o[k], m));
    }
    let middle = (l > 0 && r > 0).then(|| (l - 1, 0.0, js_max(tl, tr)));
    Some(Wraps {
        pull,
        band,
        patches,
        closures,
        middle,
        reach: js_max(tl, tr),
    })
}

/// An end of the wall the layers turn at: where along its base line, and which way the wall lies from it (+1: the
/// wall is beyond `e`, as at a path's start or a hole's far jamb).
#[derive(Clone, Copy, Debug)]
pub(super) struct EndSide {
    pub e: f64,
    pub into: f64,
}

impl EndSide {
    /// The base line's interval `p0` to `p1` before the end.
    fn interval(self, p0: f64, p1: f64) -> (f64, f64) {
        let (x, y) = (self.e + self.into * p0, self.e + self.into * p1);
        if x <= y { (x, y) } else { (y, x) }
    }
}

/// What the turns add to a wall's drawing at its ends.
#[derive(Default)]
pub(super) struct Turned {
    /// Per line, the stretches before its ends to leave out.
    pub gaps: Vec<Vec<Span>>,
    /// Per component, its band's cuts before its ends.
    pub cuts: Vec<Vec<(Span, Span)>>,
    /// Per component, the turns and steps to add.
    pub bodies: Vec<Vec<Body>>,
    pub closures: Vec<(usize, Line)>,
    /// The holes the layers turn at, by their index.
    pub wrapped: Vec<usize>,
}

fn span_on(lp: &Path, base: &Path, s0: f64, s1: f64, d: f64) -> Span {
    let on = |s: f64| nearest_s(lp, beside(base, s, d));
    Span {
        a: on(s0),
        b: on(s1),
        mid: on((s0 + s1) / 2.0),
    }
}

impl Solver<'_> {
    /// The ends wall `i`'s layers turn at: its free ends (not joined, its caps closed) and its through holes' jambs,
    /// those with room for the turns along the wall (the others square, as before).
    pub(super) fn turned(
        &self,
        i: usize,
        lines: &[Line],
        holes: &mut [Cut],
        w: &Wraps,
        base: &Path,
    ) -> Turned {
        let spec = self.spec(i);
        let n = lines.len() - 1;
        let o = wall::boundaries(spec);
        let (left, right) = (o[0], o[n]);
        let eps = 1e-9 * js_max(1.0, js_max(left.abs(), right.abs()));
        let paths: Vec<Option<Path>> = lines
            .iter()
            .map(|l| line_path(l, spec.closed).map(|p| p.0))
            .collect();
        let mut out = Turned {
            gaps: vec![Vec::new(); n + 1],
            cuts: vec![Vec::new(); n],
            bodies: vec![Vec::new(); n],
            closures: Vec::new(),
            wrapped: Vec::new(),
        };
        let len = base.length;
        let mut sides: Vec<EndSide> = Vec::new();
        if !spec.closed && !spec.caps_open && len > 2.0 * w.reach {
            if !self.joined[i][0] {
                sides.push(EndSide { e: 0.0, into: 1.0 });
            }
            if !self.joined[i][1] {
                sides.push(EndSide { e: len, into: -1.0 });
            }
        }
        for side in &sides {
            for (k, lp) in paths.iter().enumerate() {
                let (Some(lp), p) = (lp, w.pull[k]) else {
                    continue;
                };
                if p > 0.0 {
                    let (a, b) = side.interval(0.0, p);
                    out.gaps[k].push(span_on(lp, base, a, b, o[k]));
                }
            }
            for c in 0..n {
                let (Some(pa), Some(pb)) = (&paths[c], &paths[c + 1]) else {
                    continue;
                };
                let p = w.band[c];
                if p > 0.0 {
                    let (a, b) = side.interval(0.0, p);
                    out.cuts[c].push((
                        span_on(pa, base, a, b, o[c]),
                        span_on(pb, base, a, b, o[c + 1]),
                    ));
                }
            }
        }
        // The through holes' jambs, when the turns fit before the wall's ends and other holes; a pocket door's stay
        // square, its pocket opening there (docs/adr/0777 §2).
        let through: Vec<usize> = (0..holes.len())
            .filter(|&h| holes[h].lo <= right + eps && holes[h].hi >= left - eps && holes[h].open == [None, None])
            .collect();
        for &h in &through {
            let (s0, s1) = (holes[h].s0, holes[h].s1);
            let fits = (spec.closed || (s0 - w.reach > eps && s1 + w.reach < len - eps))
                && through.iter().filter(|&&g| g != h).all(|&g| {
                    holes[g].s1 + w.reach < s0 - w.reach || holes[g].s0 - w.reach > s1 + w.reach
                });
            if !fits {
                continue;
            }
            // Each line's span grown by its pull on both sides; the bands cut by their own.
            for (k, lp) in paths.iter().enumerate() {
                if let Some(lp) = lp {
                    holes[h].spans[k] = span_on(lp, base, s0 - w.pull[k], s1 + w.pull[k], o[k]);
                }
            }
            for c in 0..n {
                let (Some(pa), Some(pb)) = (&paths[c], &paths[c + 1]) else {
                    continue;
                };
                let p = w.band[c];
                out.cuts[c].push((
                    span_on(pa, base, s0 - p, s1 + p, o[c]),
                    span_on(pb, base, s0 - p, s1 + p, o[c + 1]),
                ));
            }
            out.wrapped.push(h);
            sides.push(EndSide { e: s0, into: -1.0 });
            sides.push(EndSide { e: s1, into: 1.0 });
        }
        for side in &sides {
            for p in &w.patches {
                let (a, b) = side.interval(p.p0, p.p1);
                if let Some(body) = self.between(i, base, a, b, p.upper, p.lower) {
                    out.bodies[p.comp].push(body);
                }
            }
            for &(comp, pull, upper, lower) in &w.closures {
                let s = side.e + side.into * pull;
                out.closures.push((
                    comp,
                    Line {
                        pts: vec![beside(base, s, upper), beside(base, s, lower)],
                        bulges: vec![0.0, 0.0],
                    },
                ));
            }
            if let Some((comp, p0, p1)) = w.middle {
                let (a, b) = side.interval(p0, p1);
                let m = w
                    .patches
                    .iter()
                    .find(|p| p.comp == comp)
                    .map_or(0.0, |p| p.lower);
                if let Some(line) = self.stretch(i, base, a, b, m) {
                    out.closures.push((comp, line));
                }
            }
        }
        out
    }
}
