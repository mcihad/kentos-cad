//! A volume's ledger (docs/adr/0949, plan/road.md Y43; ROAD-VOL-16, ROAD-VOL-18): its total taken apart by where it
//! comes from — stretches of km, the corridor's regions, the materials — so a large total can be traced to its cause.
//!
//! - **Groups** take whole stretches by their middle: a stretch between two sections is one volume (end areas or
//!   prismoidal), and splitting it would invent a split the method never made. So the groups add up to the total
//!   exactly, and their edges are sections.
//! - **Sums** are compensated (Neumaier): a ledger whose groups do not add up to its total is no ledger.
//! - Volumes up to [`NONE`] (1 cm³) are none for the concentration and the reasons.
//! - **Concentration.** Stretches sorted by volume per metre, densest first, against the share of the length they
//!   cover: the curve says how much of the road holds half and four fifths of the volume, and its Gini index (0: the
//!   volume evenly spread, toward 1: all in a short stretch) 2·∫C − 1. The curve is linear on each stretch (its volume
//!   evenly spread along it), so the shares and the area are exact.
//! - **Why a stretch is large.** Its volume is its length times its mean area. Against the stretches' medians (of those
//!   with volume), the two factors say whether it is large for its section or for its length.
//! - **Rounding** (ROAD-VOL-16): a report's figures are rounded once, at the end. The sum of the rounded stretches can
//!   differ from the rounded total; the difference is reported, not hidden by summing the rounded figures.

use crate::accuracy::quantile;

/// A stretch's volume up to this (m³, 1 cm³) is none: a section's arithmetic leaves 1e-29 m³ of fill where there is
/// none, and such stretches pulled the medians to 1e-29 (an area factor of 1e29) and reordered the curve's tail.
pub const NONE: f64 = 1e-6;

fn kept(v: f64) -> f64 {
    if v > NONE { v } else { 0.0 }
}

/// A Neumaier-compensated sum.
pub fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let (mut s, mut c) = (0.0_f64, 0.0_f64);
    for v in values {
        let t = s + v;
        c += if s.abs() >= v.abs() { (s - t) + v } else { (v - t) + s };
        s = t;
    }
    s + c
}

/// A stretch between two sections: its stations and its figures (none: no volume there).
#[derive(Clone, Debug, PartialEq)]
pub struct Stretch {
    pub from: f64,
    pub to: f64,
    pub values: Vec<Option<f64>>,
}

/// A group of stretches: its range, its stretches' indices and its figures' sums.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub from: f64,
    pub to: f64,
    pub members: Vec<usize>,
    pub values: Vec<f64>,
}

/// The stretches taken into `ranges` by their middle (a range [a, b) holds a middle a ≤ m < b, the last one also b);
/// ranges that take none are left out. Each group's range is its stretches' first and last station.
pub fn group(stretches: &[Stretch], ranges: &[(f64, f64)]) -> Vec<Group> {
    let columns = stretches.first().map_or(0, |s| s.values.len());
    let n = ranges.len();
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, s) in stretches.iter().enumerate() {
        let m = 0.5 * (s.from + s.to);
        if let Some(k) = ranges.iter().enumerate().position(|(k, r)| r.0 <= m && (m < r.1 || (k + 1 == n && m <= r.1))) {
            members[k].push(i);
        }
    }
    members
        .into_iter()
        .filter(|m| !m.is_empty())
        .map(|m| {
            let values = (0..columns).map(|c| sum(m.iter().filter_map(|&i| stretches[i].values[c]))).collect();
            let from = m.iter().map(|&i| stretches[i].from).fold(f64::INFINITY, f64::min);
            let to = m.iter().map(|&i| stretches[i].to).fold(f64::NEG_INFINITY, f64::max);
            Group { from, to, members: m, values }
        })
        .collect()
}

/// Ranges of `step` metres on the stations' multiples of it, covering `first` to `last`.
pub fn steps(first: f64, last: f64, step: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut a = (first / step).floor() * step;
    while a < last || out.is_empty() {
        out.push((a, a + step));
        a += step;
    }
    out
}

/// How a volume crowds along the road.
#[derive(Clone, Debug, PartialEq)]
pub struct Concentration {
    /// 2·∫C − 1 of the densest-first curve: 0 evenly spread, toward 1 in a short stretch.
    pub gini: f64,
    /// The share of the length holding half and four fifths of the volume.
    pub half: f64,
    pub most: f64,
    /// The curve: the share of the length against the share of the volume, densest first, from (0, 0) to (1, 1).
    pub curve: Vec<(f64, f64)>,
}

/// The concentration of `volumes` (≥ 0) over stretches of `lengths`; none without volume or length.
pub fn concentration(lengths: &[f64], volumes: &[f64]) -> Option<Concentration> {
    let volumes: Vec<f64> = volumes.iter().map(|v| kept(*v)).collect();
    let (l, v) = (sum(lengths.iter().copied()), sum(volumes.iter().copied()));
    if !(l > 0.0 && v > 0.0) {
        return None;
    }
    // Densest first; densities equal to 9 significant digits keep the stretches' order (two equal sections computed
    // by different arithmetic otherwise swapped places and moved a point of the curve).
    let density = |i: usize| -> f64 { format!("{:.8e}", volumes[i] / lengths[i]).parse().unwrap_or(0.0) };
    let mut order: Vec<usize> = (0..lengths.len()).filter(|&i| lengths[i] > 0.0).collect();
    order.sort_by(|&a, &b| density(b).total_cmp(&density(a)).then(a.cmp(&b)));
    let mut curve = vec![(0.0, 0.0)];
    let (mut cl, mut cv) = (Vec::new(), Vec::new());
    for &i in &order {
        cl.push(lengths[i]);
        cv.push(volumes[i]);
        curve.push((sum(cl.iter().copied()) / l, sum(cv.iter().copied()) / v));
    }
    let area = sum(curve.windows(2).map(|w| (w[1].0 - w[0].0) * (w[0].1 + w[1].1) / 2.0));
    let reach = |t: f64| {
        curve
            .windows(2)
            .find(|w| w[1].1 >= t)
            .map_or(1.0, |w| if w[1].1 > w[0].1 { w[0].0 + (w[1].0 - w[0].0) * (t - w[0].1) / (w[1].1 - w[0].1) } else { w[0].0 })
    };
    Some(Concentration { gini: 2.0 * area - 1.0, half: reach(0.5), most: reach(0.8), curve })
}

/// A stretch's volume against the stretches': its length and mean area each over their median (of the stretches with
/// volume).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reason {
    pub length: f64,
    pub area: f64,
}

/// Each stretch's reason; none where it has no volume (up to [`NONE`]).
pub fn reasons(lengths: &[f64], volumes: &[f64]) -> Vec<Option<Reason>> {
    let volumes: Vec<f64> = volumes.iter().map(|v| kept(*v)).collect();
    let with: Vec<usize> = (0..lengths.len()).filter(|&i| lengths[i] > 0.0 && volumes[i] > 0.0).collect();
    let median = |f: &dyn Fn(usize) -> f64| {
        let mut v: Vec<f64> = with.iter().map(|&i| f(i)).collect();
        v.sort_by(f64::total_cmp);
        quantile(&v, 0.5)
    };
    let (ml, ma) = (median(&|i| lengths[i]), median(&|i| volumes[i] / lengths[i]));
    (0..lengths.len())
        .map(|i| {
            (lengths[i] > 0.0 && volumes[i] > 0.0 && ml > 0.0 && ma > 0.0)
                .then(|| Reason { length: lengths[i] / ml, area: volumes[i] / lengths[i] / ma })
        })
        .collect()
}

/// `v` rounded to `decimals`, half away from zero.
pub fn round(v: f64, decimals: u32) -> f64 {
    let k = 10f64.powi(decimals as i32);
    (v * k).round() / k
}

/// The rounded figures' sum, the rounded total and their difference.
pub fn rounding(values: &[f64], decimals: u32) -> (f64, f64, f64) {
    let rows = round(sum(values.iter().map(|v| round(*v, decimals))), decimals);
    let total = round(sum(values.iter().copied()), decimals);
    (rows, total, round(rows - total, decimals))
}
