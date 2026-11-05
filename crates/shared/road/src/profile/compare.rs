//! Two vertical alignments compared (docs/adr/0962; ROAD-PROFILE-09): the stations where a profile's new grade line
//! leaves the old one by more than a tolerance, and by how much at most. What a profile edit makes stale is read from
//! these stretches: the corridor regions, the section stations and the volumes over them.
//!
//! Between the breaks of either line (PVIs, curve ends) both are smooth, so are their difference Δz and its slope Δg
//! (the grades' difference). On each such piece the stations where Δz or Δg is zero are found (sampled at [`SAMPLES`]
//! points, sign changes bisected): between two of them |Δz| is monotone, so its crossings of the tolerance are bisected
//! between consecutive points and none is missed, and its greatest value on a stretch is at an end or where Δg = 0 —
//! a simple root, well conditioned, where a search for the flat top of |Δz| finds the station only to √ε.

use super::vertical::Vertical;

/// The samples taken on each piece between two breaks.
pub const SAMPLES: usize = 64;

/// How a stretch differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Both lines are there and differ by more than the tolerance.
    Changed,
    /// Only the new line is there (the profile was lengthened).
    Added,
    /// Only the old line is there (the profile was shortened).
    Removed,
}

/// A stretch of stations where the lines differ: from, to; for a changed one the greatest |Δz| and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Change {
    pub kind: Kind,
    pub from: f64,
    pub to: f64,
    pub most: Option<f64>,
    pub at: Option<f64>,
}

fn breaks(v: &Vertical, lo: f64, hi: f64, out: &mut Vec<f64>) {
    for p in &v.pieces {
        let (a, b) = p.range();
        for s in [a, b] {
            if s > lo && s < hi {
                out.push(s);
            }
        }
    }
}

/// Where `old` and `new` differ by more than `tolerance` (m), in station order.
pub fn changes(old: &Vertical, new: &Vertical, tolerance: f64) -> Vec<Change> {
    let mut out: Vec<Change> = Vec::new();
    let (lo, hi) = (old.start.max(new.start), old.end.min(new.end));
    // Only one line: lengthened or shortened at either end.
    let mut ends = Vec::new();
    for (a, b, kind) in [
        (new.start, old.start.min(new.end), Kind::Added),
        (old.start, new.start.min(old.end), Kind::Removed),
        (old.end.max(new.start), new.end, Kind::Added),
        (new.end.max(old.start), old.end, Kind::Removed),
    ] {
        if b - a > 1e-9 {
            ends.push(Change { kind, from: a, to: b, most: None, at: None });
        }
    }
    if hi > lo {
        let mut cuts = vec![lo, hi];
        breaks(old, lo, hi, &mut cuts);
        breaks(new, lo, hi, &mut cuts);
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        for w in cuts.windows(2) {
            let (a, b) = (w[0], w[1]);
            if b - a < 1e-9 {
                continue;
            }
            // Within the piece only: a break belongs to the next piece, the piece's end is approached from inside.
            let inner = |s: f64| s.clamp(a, b - (b - a) * 1e-12);
            let diff = |s: f64| match (old.at(inner(s)), new.at(inner(s))) {
                (Some(o), Some(n)) => (n.0 - o.0, n.1 - o.1),
                _ => (0.0, 0.0),
            };
            let d = |s: f64| diff(s).0.abs();
            let over = |s: f64| d(s) - tolerance;
            let samples: Vec<f64> = (0..=SAMPLES).map(|i| a + (b - a) * i as f64 / SAMPLES as f64).collect();
            // The zeros of Δz and of Δg: |Δz| is monotone between consecutive points.
            let mut pts = samples.clone();
            // A sample where Δg is exactly zero is a peak itself.
            let mut peaks: Vec<f64> = samples.iter().copied().filter(|s| diff(*s).1 == 0.0).collect();
            for q in samples.windows(2) {
                let (x0, x1) = (q[0], q[1]);
                for (k, store) in [(0usize, false), (1usize, true)] {
                    let f = |s: f64| if k == 0 { diff(s).0 } else { diff(s).1 };
                    let (f0, f1) = (f(x0), f(x1));
                    if f0 != 0.0 && f1 != 0.0 && (f0 < 0.0) != (f1 < 0.0) {
                        let x = bisect(f, x0, x1);
                        pts.push(x);
                        if store {
                            peaks.push(x);
                        }
                    }
                }
            }
            pts.sort_by(f64::total_cmp);
            let mut start: Option<f64> = (over(pts[0]) > 0.0).then_some(pts[0]);
            for q in pts.windows(2) {
                let (x0, x1) = (q[0], q[1]);
                let (f0, f1) = (over(x0), over(x1));
                if (f0 > 0.0) != (f1 > 0.0) {
                    let x = bisect(over, x0, x1);
                    match start.take() {
                        Some(from) => out.push(stretch(from, x, &peaks, &d)),
                        None => start = Some(x),
                    }
                }
            }
            if let Some(from) = start {
                out.push(stretch(from, b, &peaks, &d));
            }
        }
    }
    // Stretches touching across a break are one.
    let mut merged: Vec<Change> = Vec::new();
    for c in out {
        match merged.last_mut() {
            Some(m) if c.from - m.to <= 1e-9 => {
                m.to = c.to;
                if c.most.unwrap_or(0.0) > m.most.unwrap_or(0.0) + 1e-9 {
                    m.most = c.most;
                    m.at = c.at;
                }
            }
            _ => merged.push(c),
        }
    }
    merged.extend(ends);
    merged.sort_by(|a, b| a.from.total_cmp(&b.from));
    merged
}

/// The station in [x0, x1] where `f` changes sign, to the last bit.
fn bisect(f: impl Fn(f64) -> f64, x0: f64, x1: f64) -> f64 {
    let (mut a, mut b) = (x0, x1);
    let fa = f(a);
    for _ in 0..200 {
        let m = 0.5 * (a + b);
        if m <= a || m >= b {
            break;
        }
        if (f(m) < 0.0) == (fa < 0.0) && f(m) != 0.0 {
            a = m;
        } else {
            b = m;
        }
    }
    0.5 * (a + b)
}

/// A changed stretch [from, to] within one piece: its greatest |Δz| at an end or at a zero of Δg inside (the
/// earliest where several are as great, to 1e-9 m).
fn stretch(from: f64, to: f64, peaks: &[f64], d: &impl Fn(f64) -> f64) -> Change {
    let mut best = (from, d(from));
    for x in peaks.iter().copied().filter(|x| *x > from && *x < to).chain([to]) {
        let v = d(x);
        if v > best.1 + 1e-9 {
            best = (x, v);
        }
    }
    Change { kind: Kind::Changed, from, to, most: Some(best.1), at: Some(best.0) }
}
