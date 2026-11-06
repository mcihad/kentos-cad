//! An alignment given as elements (a LandXML `CoordGeom`, docs/adr/0908 §7) back to KentOS's PIs: each run of curves
//! between tangents is one PI where its entry and exit tangents meet, with the circular curve's radius and the
//! clothoids' lengths; two tangents meeting at an angle a PI without a curve. KentOS's PI form takes a curve with a
//! clothoid on each side at most, on one side of the axis; a compound or reverse curve without a tangent between,
//! and a clothoid not from a tangent to the curve (an ove one), it does not take, and says which element.

use super::chain::PiInput;

/// An element's kind: a curve's radius and turn and its centre where known; a clothoid's radii (none: a tangent end).
#[derive(Clone, Debug, PartialEq)]
pub enum FitKind {
    Line,
    Curve { radius: f64, clockwise: bool, center: Option<[f64; 2]> },
    Spiral { radius_start: Option<f64>, radius_end: Option<f64>, clockwise: bool },
}

/// An element as given: its kind, start and end, its PI (a clothoid's must be given, a curve's may be), its length.
#[derive(Clone, Debug, PartialEq)]
pub struct FitElement {
    pub kind: FitKind,
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub pi: Option<[f64; 2]>,
    pub length: f64,
}

/// Why elements make no PIs: the element's index (from 0) and what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitError {
    Empty,
    /// A curve with neither its centre nor its PI, or a clothoid without its PI.
    Unplaced { element: usize },
    /// Curves one after another without a tangent between that are not one clothoid–curve–clothoid run (a compound or
    /// reverse curve, two curves), or a run turning both ways.
    Compound { element: usize },
    /// A clothoid not from a tangent to the run's curve (an ove clothoid), or whose radius is not the curve's.
    Spiral { element: usize },
    /// A run whose entry and exit tangents do not meet (parallel).
    NoMeet { element: usize },
    /// An element going back on itself or of no length.
    Degenerate { element: usize },
}

fn unit(a: [f64; 2], b: [f64; 2]) -> Option<[f64; 2]> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l = dx.hypot(dy);
    (l > 1e-12).then(|| [dx / l, dy / l])
}

/// The tangent's direction at an element's start (`at_end` false) or end.
fn direction(e: &FitElement, i: usize, at_end: bool) -> Result<[f64; 2], FitError> {
    let none = FitError::Degenerate { element: i };
    match &e.kind {
        FitKind::Line => unit(e.start, e.end).ok_or(none),
        FitKind::Curve { clockwise, center, .. } => {
            if let Some(c) = center {
                let p = if at_end { e.end } else { e.start };
                let r = unit(*c, p).ok_or(none)?;
                Ok(if *clockwise { [r[1], -r[0]] } else { [-r[1], r[0]] })
            } else if let Some(pi) = e.pi {
                if at_end { unit(pi, e.end).ok_or(none) } else { unit(e.start, pi).ok_or(none) }
            } else {
                Err(FitError::Unplaced { element: i })
            }
        }
        FitKind::Spiral { .. } => {
            let pi = e.pi.ok_or(FitError::Unplaced { element: i })?;
            if at_end { unit(pi, e.end).ok_or(none) } else { unit(e.start, pi).ok_or(none) }
        }
    }
}

fn same_radius(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0)
}

/// The PIs of `elements` (ids `a`, `p1`, …, `b`).
pub fn pis_of(elements: &[FitElement]) -> Result<Vec<PiInput>, FitError> {
    let Some(first) = elements.first() else { return Err(FitError::Empty) };
    let mut out = vec![PiInput { id: "a".into(), x: first.start[0], y: first.start[1], radius: 0.0, spiral_in: 0.0, spiral_out: 0.0, ove: false }];
    let mut n = 0;
    let mut push = |out: &mut Vec<PiInput>, p: [f64; 2], radius: f64, spiral_in: f64, spiral_out: f64| {
        n += 1;
        out.push(PiInput { id: format!("p{n}"), x: p[0], y: p[1], radius, spiral_in, spiral_out, ove: false });
    };
    let mut i = 0;
    while i < elements.len() {
        let e = &elements[i];
        if e.length <= 0.0 {
            return Err(FitError::Degenerate { element: i });
        }
        if e.kind == FitKind::Line {
            // A kink where the next line turns.
            if let Some(next) = elements.get(i + 1)
                && next.kind == FitKind::Line
            {
                let (d0, d1) = (direction(e, i, true)?, direction(next, i + 1, false)?);
                if (d0[0] * d1[1] - d0[1] * d1[0]).abs() > 1e-12 || d0[0] * d1[0] + d0[1] * d1[1] < 0.0 {
                    push(&mut out, e.end, 0.0, 0.0, 0.0);
                }
            }
            i += 1;
            continue;
        }
        // A run of curves: up to the next line.
        let from = i;
        let mut to = i;
        while to + 1 < elements.len() && elements[to + 1].kind != FitKind::Line {
            to += 1;
        }
        let run = &elements[from..=to];
        let turn = |k: &FitKind| match k {
            FitKind::Curve { clockwise, .. } | FitKind::Spiral { clockwise, .. } => *clockwise,
            FitKind::Line => false,
        };
        if let Some(j) = run.iter().position(|r| turn(&r.kind) != turn(&run[0].kind)) {
            return Err(FitError::Compound { element: from + j });
        }
        let curves: Vec<usize> = run.iter().enumerate().filter(|(_, r)| matches!(r.kind, FitKind::Curve { .. })).map(|(j, _)| j).collect();
        let (radius, spiral_in, spiral_out) = match (run.len(), curves.as_slice()) {
            // A curve alone, or with a clothoid before, after or on both sides.
            (1, [0]) | (2, [0]) | (2, [1]) | (3, [1]) => {
                let c = curves[0];
                let FitKind::Curve { radius: r, .. } = run[c].kind else { unreachable!() };
                let mut s_in = 0.0;
                let mut s_out = 0.0;
                for (j, piece) in run.iter().enumerate() {
                    if let FitKind::Spiral { radius_start, radius_end, .. } = piece.kind {
                        let (tangent, curve) = if j < c { (radius_start, radius_end) } else { (radius_end, radius_start) };
                        if tangent.is_some() || !curve.is_some_and(|v| same_radius(v, r)) {
                            return Err(FitError::Spiral { element: from + j });
                        }
                        if j < c { s_in = piece.length } else { s_out = piece.length }
                    }
                }
                (r, s_in, s_out)
            }
            // Two clothoids meeting at their sharpest.
            (2, []) => {
                let (FitKind::Spiral { radius_start: a0, radius_end: a1, .. }, FitKind::Spiral { radius_start: b0, radius_end: b1, .. }) =
                    (&run[0].kind, &run[1].kind)
                else {
                    return Err(FitError::Compound { element: from });
                };
                match (a0, a1, b0, b1) {
                    (None, Some(r), Some(q), None) if same_radius(*r, *q) => (*r, run[0].length, run[1].length),
                    _ => return Err(FitError::Spiral { element: from }),
                }
            }
            (_, []) => return Err(FitError::Spiral { element: from }),
            _ => return Err(FitError::Compound { element: from + curves.get(1).copied().unwrap_or(0) }),
        };
        let (p0, d0) = (run[0].start, direction(&run[0], from, false)?);
        let (p1, d1) = (run[run.len() - 1].end, direction(&run[run.len() - 1], to, true)?);
        let cross = d0[0] * d1[1] - d0[1] * d1[0];
        if cross.abs() < 1e-12 {
            return Err(FitError::NoMeet { element: from });
        }
        let t = ((p1[0] - p0[0]) * d1[1] - (p1[1] - p0[1]) * d1[0]) / cross;
        push(&mut out, [p0[0] + t * d0[0], p0[1] + t * d0[1]], radius, spiral_in, spiral_out);
        i = to + 1;
    }
    let last = &elements[elements.len() - 1];
    out.push(PiInput { id: "b".into(), x: last.end[0], y: last.end[1], radius: 0.0, spiral_in: 0.0, spiral_out: 0.0, ove: false });
    Ok(out)
}
