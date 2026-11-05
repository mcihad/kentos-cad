//! Where an alignment edit moves the line, and where each old station lands (docs/adr/0963; ROAD-ALIGN-10). The
//! element mapping (`mapping`) matches elements by id and calls a whole tangent changed when only its end moved; what
//! hangs on a station (a PVI, a section line, a region's end) asks instead whether its place is still on the line.
//!
//! An old station s has a place P(s); its foot on the new line gives a new station s′ and an offset. Where the offset
//! is within the tolerance the station maps to s′ (shifted by s′ − s); where it is not, or P(s) has no foot, the place
//! left the line and what hangs there is unresolved. Each element of the line is sampled at [`SAMPLES`] points, the
//! tolerance's crossings bisected, the greatest offset of a stretch refined by golden-section search (its value is well
//! conditioned, its place only to √ε, so the place is not reported).

use super::chain::Alignment;

/// The samples on each element.
pub const SAMPLES: usize = 64;

/// A stretch of one line's stations off the other line: from, to, and the greatest offset where every place of it has
/// a foot on the other line (none where some place has none).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stretch {
    pub from: f64,
    pub to: f64,
    pub most: Option<f64>,
}

/// A stretch of old stations still on the line, and how far their stations moved at its ends (new − old).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Run {
    pub from: f64,
    pub to: f64,
    pub shift_from: f64,
    pub shift_to: f64,
}

/// An edit's effect on the line: the old stations off the new line, the new stations off the old line, the runs
/// between.
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
    pub removed: Vec<Stretch>,
    pub added: Vec<Stretch>,
    pub runs: Vec<Run>,
}

/// Where the place of `line`'s station `s` falls on `other`: its station there and offset; none without a foot. A
/// place on the other line's very end (within 1e-7 m) is on it: `locate` calls a point there beyond the end, but the
/// shared end of two lines has not moved.
pub fn foot(line: &Alignment, other: &Alignment, s: f64) -> Option<(f64, f64)> {
    let p = line.at(s)?;
    let got = other.locate([p.x, p.y]);
    got.nearest.map(|f| (f.at.station, f.offset)).or_else(|| {
        got.beyond
            .map(|(_, f)| f)
            .filter(|f| (p.x - f.at.x).hypot(p.y - f.at.y) <= 1e-7)
            .map(|f| (f.at.station, 0.0))
    })
}

/// The new station of old station `s`, or none where its place left the line by more than `tolerance`.
pub fn map(before: &Alignment, after: &Alignment, s: f64, tolerance: f64) -> Option<f64> {
    foot(before, after, s).filter(|(_, o)| o.abs() <= tolerance).map(|(t, _)| t)
}

/// The stretches of `line` off `other` by more than `tolerance`.
pub fn off(line: &Alignment, other: &Alignment, tolerance: f64) -> Vec<Stretch> {
    let gap = |s: f64| foot(line, other, s).map_or(f64::INFINITY, |(_, o)| o.abs());
    let over = |s: f64| gap(s) > tolerance;
    let mut out: Vec<Stretch> = Vec::new();
    let mut start: Option<f64> = None;
    let mut cuts: Vec<f64> = line.elements.iter().map(|e| e.station).collect();
    cuts.push(line.length);
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b - a < 1e-12 {
            continue;
        }
        let samples: Vec<f64> = (0..=SAMPLES).map(|i| a + (b - a) * i as f64 / SAMPLES as f64).collect();
        if start.is_none() && over(samples[0]) {
            start = Some(samples[0]);
        }
        for q in samples.windows(2) {
            let (x0, x1) = (q[0], q[1]);
            if over(x0) != over(x1) {
                let (mut lo, mut hi) = (x0, x1);
                for _ in 0..200 {
                    let m = 0.5 * (lo + hi);
                    if m <= lo || m >= hi {
                        break;
                    }
                    if over(m) == over(x0) {
                        lo = m;
                    } else {
                        hi = m;
                    }
                }
                let x = 0.5 * (lo + hi);
                match start.take() {
                    Some(from) => out.push(Stretch { from, to: x, most: None }),
                    None => start = Some(x),
                }
            }
        }
    }
    if let Some(from) = start {
        out.push(Stretch { from, to: line.length, most: None });
    }
    // A stretch of no length is a single place on the edge of the tolerance: none.
    out.retain(|st| st.to - st.from > 1e-9);
    for st in &mut out {
        st.most = most(st.from, st.to, &gap);
    }
    out
}

/// The greatest gap on [from, to]: the best of 257 samples refined by golden-section search; none where a place has no
/// foot.
fn most(from: f64, to: f64, gap: &impl Fn(f64) -> f64) -> Option<f64> {
    const N: usize = 256;
    let xs: Vec<f64> = (0..=N).map(|i| from + (to - from) * i as f64 / N as f64).collect();
    let vals: Vec<f64> = xs.iter().map(|x| gap(*x)).collect();
    if vals.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let k = (0..=N).fold(0, |b, i| if vals[i] > vals[b] { i } else { b });
    let (mut a, mut b) = (xs[k.saturating_sub(1)], xs[(k + 1).min(N)]);
    let r = 0.5 * (5f64.sqrt() - 1.0);
    for _ in 0..200 {
        let (x1, x2) = (b - r * (b - a), a + r * (b - a));
        if gap(x1) >= gap(x2) {
            b = x2;
        } else {
            a = x1;
        }
    }
    Some(gap(0.5 * (a + b)).max(vals[k]))
}

/// The edit's effect: `before`'s stretches off `after`, `after`'s off `before`, and the runs of `before` between the
/// first with their shifts.
pub fn moved(before: &Alignment, after: &Alignment, tolerance: f64) -> Moved {
    let removed = off(before, after, tolerance);
    let added = off(after, before, tolerance);
    let mut runs = Vec::new();
    let mut at = 0.0;
    // The station shift reads the nearest station on the new line, its end included at any distance: a run's end is
    // where the line leaves the tolerance, so whether it has a foot there is decided by rounding.
    let shift = |s: f64| {
        before
            .at(s)
            .and_then(|p| {
                let got = after.locate([p.x, p.y]);
                got.nearest.or(got.beyond.map(|(_, f)| f)).map(|f| f.at.station - s)
            })
            .unwrap_or(0.0)
    };
    for r in removed.iter().map(|r| (r.from, r.to)).chain([(before.length, before.length)]) {
        if r.0 - at > 1e-9 {
            runs.push(Run { from: at, to: r.0, shift_from: shift(at), shift_to: shift(r.0) });
        }
        at = r.1;
    }
    Moved { removed, added, runs }
}
