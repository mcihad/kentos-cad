//! A parallel axis (docs/adr/0901 §8): the source alignment offset by d (right positive). It keeps the source's
//! stations; its place at a station is the source's moved along the normal. On an element of curvature k it is a
//! curve of curvature k/(1 + k·d) while 1 + k·d > 0; where 1 + k·d ≤ 0 it folds into a cusp and is refused with the
//! stations where it does, the element and its tightest radius on that side (the fix: a radius over |d|, or an offset
//! under that radius). Its length is Σ (L + d·Δθ) over the elements. Parts crossing (a kink's inner side, a hairpin)
//! are found on the offset sampled at most every metre (and 0.02 rad) and refined by Newton's method to the two
//! stations whose offset places are one point; the first such pair along the alignment is said (docs/adr/0970).

use super::chain::Alignment;
use super::element::Kind;

/// Where a parallel axis does not hold.
#[derive(Clone, Debug, PartialEq)]
pub enum OffsetError {
    /// It folds into a cusp between these stations, on the element `element` whose tightest radius on the offset's
    /// side is `radius` (m).
    Cusp { from: f64, to: f64, element: String, radius: f64 },
    /// It crosses itself: its places at the stations `a` < `b` are one point. `kink`: the PI of the source's kink (a
    /// direction jump, a PI without a curve) between them, when there is one.
    Crosses { a: f64, b: f64, kink: Option<String> },
}

/// The offset place at a station.
pub fn place(al: &Alignment, station: f64, d: f64) -> Option<[f64; 2]> {
    let at = al.at(station)?;
    // Right of the direction: (sin θ, −cos θ).
    Some([at.x + d * at.direction.sin(), at.y - d * at.direction.cos()])
}

/// The offset's length.
pub fn length(al: &Alignment, d: f64) -> f64 {
    al.elements.iter().map(|e| e.length + d * e.turn()).sum()
}

/// Whether the source offset by `d` holds: no cusp, no crossing.
pub fn check(al: &Alignment, d: f64) -> Result<(), OffsetError> {
    if d == 0.0 {
        return Ok(());
    }
    // 1 + k·d with d right positive: a left turn (k > 0) grows on the right. The left side is −d.
    let fold = |k: f64| 1.0 + k * d <= 0.0;
    for e in &al.elements {
        if e.kind == Kind::Tangent {
            continue;
        }
        let (a, b) = (fold(e.k0), fold(e.k1));
        if a || b {
            // Where k·d = −1 along a clothoid's linear curvature.
            let cross = |k_at: f64| {
                let rate = e.rate();
                if rate == 0.0 { 0.0 } else { ((k_at - e.k0) / rate).clamp(0.0, e.length) }
            };
            let s_fold = cross(-1.0 / d);
            let (from, to) = match (a, b) {
                (true, true) => (0.0, e.length),
                (true, false) => (0.0, s_fold),
                _ => (s_fold, e.length),
            };
            // The tightest radius on the folding side: 1/|k| at the end that folds furthest.
            let k = [(a, e.k0), (b, e.k1)].iter().filter(|(f, _)| *f).map(|(_, k)| k.abs()).fold(0.0, f64::max);
            return Err(OffsetError::Cusp {
                from: e.station + from,
                to: e.station + to,
                element: e.id.clone(),
                radius: 1.0 / k,
            });
        }
    }
    // Far parts crossing: the sampled offset's segments, on a grid of cells.
    let mut stations = Vec::new();
    for e in &al.elements {
        let turn = e.k0.abs().max(e.k1.abs()) * e.length;
        let n = ((e.length / 1.0).ceil().max((turn / 0.02).ceil()) as usize).clamp(1, 20_000);
        for i in 0..n {
            stations.push(e.station + e.length * i as f64 / n as f64);
        }
    }
    stations.push(al.length);
    let pts: Vec<[f64; 2]> = stations.iter().filter_map(|s| place(al, *s, d)).collect();
    let cell = (al.length / pts.len() as f64 * 4.0).max(d.abs()).max(1e-6);
    let key = |p: [f64; 2]| ((p[0] / cell).floor() as i64, (p[1] / cell).floor() as i64);
    let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> = std::collections::HashMap::new();
    for i in 0..pts.len() - 1 {
        let (a, b) = (key(pts[i]), key(pts[i + 1]));
        for x in a.0.min(b.0)..=a.0.max(b.0) {
            for y in a.1.min(b.1)..=a.1.max(b.1) {
                grid.entry((x, y)).or_default().push(i);
            }
        }
    }
    let mut first: Option<(f64, f64)> = None;
    let mut keep = |a: f64, b: f64| {
        let (a, b) = (a.min(b), a.max(b));
        if b - a > 1e-6 && first.is_none_or(|f| (a, b) < f) {
            first = Some((a, b));
        }
    };
    // A kink's inner side always crosses: the two offset lines meet |d|·tan(|δ|/2) before and after it, which the
    // sampling misses where that is under a metre; seeded here, settled by Newton like the sampled ones.
    for w in al.elements.windows(2) {
        let jump = super::chain::wrap(w[1].theta0 - w[0].theta(w[0].length));
        if jump.abs() > 1e-9 && jump * d < 0.0 {
            let reach = d.abs() * (jump.abs() / 2.0).tan();
            let s = w[1].station;
            if let Some((a, b)) = refine(al, d, (s - reach).max(0.0), (s + reach).min(al.length)) {
                keep(a, b);
            }
        }
    }
    for segs in grid.values().filter(|_| pts.len() >= 4) {
        for (n, &i) in segs.iter().enumerate() {
            for &j in &segs[n + 1..] {
                let (i, j) = (i.min(j), i.max(j));
                if j <= i + 1 {
                    continue;
                }
                let Some((t, u)) = crossing(pts[i], pts[i + 1], pts[j], pts[j + 1]) else {
                    continue;
                };
                let guess = |k: usize, t: f64| stations[k] + t * (stations[k + 1] - stations[k]);
                // Settled on the same point twice (a = b) is no crossing: the sampled one stands then.
                let (a, b) = match refine(al, d, guess(i, t), guess(j, u)) {
                    Some((a, b)) if (a - b).abs() > 1e-6 => (a, b),
                    _ => (guess(i, t), guess(j, u)),
                };
                keep(a, b);
            }
        }
    }
    match first {
        Some((a, b)) => {
            let kink = al
                .seams()
                .into_iter()
                .find(|s| s.direction > 1e-9 && s.station > a && s.station < b)
                .map(|s| s.after.split('/').next().unwrap_or("").to_owned());
            Err(OffsetError::Crosses { a, b, kink })
        }
        None => Ok(()),
    }
}

/// The offset place at a station and its derivative (1 + κ·d)·(cos θ, sin θ), κ left positive and d right positive.
fn with_tangent(al: &Alignment, s: f64, d: f64) -> Option<([f64; 2], [f64; 2])> {
    let at = al.at(s)?;
    let (sin, cos) = at.direction.sin_cos();
    let grow = 1.0 + at.curvature * d;
    Some(([at.x + d * sin, at.y - d * cos], [grow * cos, grow * sin]))
}

/// Newton's method on P(a) − P(b) = 0 from a sampled crossing: the two stations whose offset places are one point,
/// within 10⁻⁹ m; none where it does not settle.
fn refine(al: &Alignment, d: f64, mut a: f64, mut b: f64) -> Option<(f64, f64)> {
    for _ in 0..60 {
        let ((pa, ta), (pb, tb)) = (with_tangent(al, a, d)?, with_tangent(al, b, d)?);
        let f = [pa[0] - pb[0], pa[1] - pb[1]];
        if f[0].hypot(f[1]) < 1e-11 {
            return Some((a, b));
        }
        // [ta  −tb]·[da db]ᵀ = −f.
        let det = -ta[0] * tb[1] + tb[0] * ta[1];
        if det.abs() < 1e-14 {
            return None;
        }
        let da = (f[0] * tb[1] - tb[0] * f[1]) / det;
        let db = (ta[1] * f[0] - ta[0] * f[1]) / det;
        a = (a + da).clamp(0.0, al.length);
        b = (b + db).clamp(0.0, al.length);
    }
    let ((pa, _), (pb, _)) = (with_tangent(al, a, d)?, with_tangent(al, b, d)?);
    ((pa[0] - pb[0]).hypot(pa[1] - pb[1]) < 1e-9).then_some((a, b))
}

/// Where segments ab and cd cross strictly inside both: the parameters along each.
fn crossing(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<(f64, f64)> {
    let (r, q) = ([b[0] - a[0], b[1] - a[1]], [d[0] - c[0], d[1] - c[1]]);
    let den = r[0] * q[1] - r[1] * q[0];
    if den == 0.0 {
        return None;
    }
    let w = [c[0] - a[0], c[1] - a[1]];
    let t = (w[0] * q[1] - w[1] * q[0]) / den;
    let u = (w[0] * r[1] - w[1] * r[0]) / den;
    (t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0).then_some((t, u))
}
