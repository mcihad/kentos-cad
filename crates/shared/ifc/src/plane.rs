//! The writer's plane geometry: the circle an arc's three points lie on, and rings nested as outer rings and holes.

/// The circle through an arc's start `a`, its middle `m` and its end `b` (docs/adr/8169): its centre, its radius and
/// whether it runs counter-clockwise; none when the three are on a line. Worked from `a`, so far from the origin the
/// small differences keep their digits.
pub(crate) fn circle(a: [f64; 2], m: [f64; 2], b: [f64; 2]) -> Option<([f64; 2], f64, bool)> {
    let (p, q) = ([m[0] - a[0], m[1] - a[1]], [b[0] - a[0], b[1] - a[1]]);
    let d = 2.0 * (p[0] * q[1] - p[1] * q[0]);
    if d.abs() < 1e-12 {
        return None;
    }
    let (pp, qq) = (p[0] * p[0] + p[1] * p[1], q[0] * q[0] + q[1] * q[1]);
    let c = [(q[1] * pp - p[1] * qq) / d, (p[0] * qq - q[0] * pp) / d];
    Some(([a[0] + c[0], a[1] + c[1]], crate::math::hypot(c[0], c[1]), d > 0.0))
}

/// Which of `rings` are outer rings and which holes (even-odd: a ring inside an odd number of others is a hole), each
/// hole with the nearest outer ring holding it: (outer, holes).
pub(crate) fn nest(rings: &[Vec<[f64; 2]>]) -> Vec<(usize, Vec<usize>)> {
    let inside = |p: [f64; 2], r: &[[f64; 2]]| {
        let mut odd = false;
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            {
                odd = !odd;
            }
        }
        odd
    };
    // A ring's point: the middle of its first edge, off any other ring's corner.
    let probe = |r: &[[f64; 2]]| {
        let (a, b) = (r[0], r[1 % r.len()]);
        [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
    };
    let holders: Vec<Vec<usize>> = (0..rings.len())
        .map(|i| {
            (0..rings.len())
                .filter(|&j| j != i && rings[i].len() > 1 && inside(probe(&rings[i]), &rings[j]))
                .collect()
        })
        .collect();
    let mut out: Vec<(usize, Vec<usize>)> = (0..rings.len())
        .filter(|&i| holders[i].len().is_multiple_of(2))
        .map(|i| (i, Vec::new()))
        .collect();
    for (i, held) in holders.iter().enumerate() {
        if held.len() % 2 == 1 {
            // The outer ring holding it directly: the one held by as many as it less one.
            if let Some(o) = held.iter().find(|&&j| holders[j].len() + 1 == held.len())
                && let Some(slot) = out.iter_mut().find(|(k, _)| k == o)
            {
                slot.1.push(i);
            }
        }
    }
    out
}
