//! A vertical alignment read off a drawn profile (plan/road.md Y10.6b, docs/adr/0908 §13; ROAD-IO-14): a line or an
//! open polyline drawn in a profile view whose left edge is at a station and whose `at` is at a datum elevation, a
//! metre of station a drawing unit and a metre of elevation `exaggeration` units (`road.profiles.draw`'s view). Each
//! straight segment is a grade; a corner between two grades a PVI, with a symmetric parabola of `length` when one is
//! given; an arc a vertical curve between its ends: its PVI where the tangents at its ends meet, a symmetric parabola
//! when the PVI is midway between the ends, an asymmetric one otherwise (ADR 0902's curves). A corner where the grade
//! does not change is not a PVI. Two arcs in a row have no grade between them and are refused.

use super::vertical::{Pvi, VCurve};

/// The profile view a line is read in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// The drawing point at `station` and `datum`.
    pub at: [f64; 2],
    pub station: f64,
    pub datum: f64,
    pub exaggeration: f64,
}

impl View {
    /// A drawing point as (station, elevation).
    pub fn read(&self, p: [f64; 2]) -> [f64; 2] {
        [self.station + (p[0] - self.at[0]), self.datum + (p[1] - self.at[1]) / self.exaggeration]
    }
}

/// Why a line is not a profile.
#[derive(Clone, Debug, PartialEq)]
pub enum TraceError {
    /// Fewer than two points at different stations.
    TooFew,
    /// The station does not grow at a vertex (0-based) read left to right.
    NotIncreasing { vertex: usize },
    /// Two arcs in a row (0-based segment of the second).
    Compound { segment: usize },
    /// An arc's end tangents do not meet between its ends (0-based segment).
    NoMeet { segment: usize },
}

/// An arc's centre and turn in the drawing from its chord and bulge.
fn arc(a: [f64; 2], b: [f64; 2], bulge: f64) -> ([f64; 2], f64) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let chord = dx.hypot(dy);
    let turn = 4.0 * bulge.atan();
    let r = chord / (2.0 * (0.5 * turn).sin()).abs();
    let h = r * (0.5 * turn).cos() * turn.signum();
    ([0.5 * (a[0] + b[0]) - dy / chord * h, 0.5 * (a[1] + b[1]) + dx / chord * h], turn)
}

/// The tangent along the arc at `p` (the direction it is walked in).
fn tangent(centre: [f64; 2], p: [f64; 2], turn: f64) -> [f64; 2] {
    let (rx, ry) = (p[0] - centre[0], p[1] - centre[1]);
    if turn > 0.0 { [-ry, rx] } else { [ry, -rx] }
}

/// The PVIs of the line `points` (with `bulges`, one per segment) in `view`; a corner's parabola `length` (0: none).
/// A line drawn right to left is read left to right.
pub fn from_drawing(points: &[[f64; 2]], bulges: &[f64], view: View, length: f64) -> Result<Vec<Pvi>, TraceError> {
    let mut pts: Vec<[f64; 2]> = points.to_vec();
    let mut bs: Vec<f64> = (0..pts.len().saturating_sub(1)).map(|i| bulges.get(i).copied().unwrap_or(0.0)).collect();
    if pts.len() < 2 {
        return Err(TraceError::TooFew);
    }
    if pts[pts.len() - 1][0] < pts[0][0] {
        pts.reverse();
        bs.reverse();
        for b in &mut bs {
            *b = -*b;
        }
    }
    let read: Vec<[f64; 2]> = pts.iter().map(|p| view.read(*p)).collect();
    for k in 1..read.len() {
        if read[k][0] <= read[k - 1][0] {
            return Err(if read.len() == 2 { TraceError::TooFew } else { TraceError::NotIncreasing { vertex: k } });
        }
    }
    // (station, elevation, curve) in order.
    let mut out: Vec<(f64, f64, VCurve)> = vec![(read[0][0], read[0][1], VCurve::None)];
    let n = bs.len();
    for i in 0..n {
        if bs[i] != 0.0 {
            if i > 0 && bs[i - 1] != 0.0 {
                return Err(TraceError::Compound { segment: i });
            }
            let (a, b) = (pts[i], pts[i + 1]);
            let (c, turn) = arc(a, b, bs[i]);
            let (ta, tb) = (tangent(c, a, turn), tangent(c, b, turn));
            // The tangents as (station, elevation) directions.
            let (da, db) = ([ta[0], ta[1] / view.exaggeration], [tb[0], tb[1] / view.exaggeration]);
            let (p, q) = (read[i], read[i + 1]);
            let cross = da[0] * db[1] - da[1] * db[0];
            if cross.abs() < 1e-15 || da[0] <= 0.0 || db[0] <= 0.0 {
                return Err(TraceError::NoMeet { segment: i });
            }
            let t = ((q[0] - p[0]) * db[1] - (q[1] - p[1]) * db[0]) / cross;
            let v = [p[0] + t * da[0], p[1] + t * da[1]];
            let (l1, l2) = (v[0] - p[0], q[0] - v[0]);
            if !(l1 > 0.0 && l2 > 0.0) {
                return Err(TraceError::NoMeet { segment: i });
            }
            let curve = if (l1 - l2).abs() <= 1e-9 * (l1 + l2) {
                VCurve::Parabola { length: l1 + l2 }
            } else {
                VCurve::Asymmetric { length_in: l1, length_out: l2 }
            };
            out.push((v[0], v[1], curve));
        } else if i + 1 < n && bs[i + 1] == 0.0 {
            // A corner between two grades; none where the grade does not change.
            let (p, v, q) = (read[i], read[i + 1], read[i + 2]);
            let (g1, g2) = ((v[1] - p[1]) / (v[0] - p[0]), (q[1] - v[1]) / (q[0] - v[0]));
            if (g2 - g1).abs() > 1e-12 {
                let curve = if length > 0.0 { VCurve::Parabola { length } } else { VCurve::None };
                out.push((v[0], v[1], curve));
            }
        }
    }
    let last = read[read.len() - 1];
    out.push((last[0], last[1], VCurve::None));
    let count = out.len();
    Ok(out
        .into_iter()
        .enumerate()
        .map(|(k, (station, elevation, curve))| Pvi {
            id: if k == 0 {
                "a".into()
            } else if k + 1 == count {
                "b".into()
            } else {
                format!("v{k}")
            },
            station,
            elevation,
            curve,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: View = View { at: [100.0, 50.0], station: 20.0, datum: 90.0, exaggeration: 10.0 };

    #[test]
    fn corners_are_pvis_and_straight_runs_are_not() {
        // Stations 20, 70, 120, 220; elevations 90, 92, 94 (one grade), 91.
        let pts = [[100.0, 50.0], [150.0, 70.0], [200.0, 90.0], [300.0, 60.0]];
        let pvis = from_drawing(&pts, &[0.0; 3], VIEW, 40.0).expect("a profile");
        let got: Vec<_> = pvis.iter().map(|p| (p.id.as_str(), p.station, p.elevation, p.curve)).collect();
        assert_eq!(
            got,
            [
                ("a", 20.0, 90.0, VCurve::None),
                ("v1", 120.0, 94.0, VCurve::Parabola { length: 40.0 }),
                ("b", 220.0, 91.0, VCurve::None)
            ]
        );
        let back: Vec<[f64; 2]> = pts.iter().rev().copied().collect();
        assert_eq!(from_drawing(&back, &[0.0; 3], VIEW, 40.0).expect("a profile"), pvis);
    }

    #[test]
    fn a_symmetric_arc_is_a_symmetric_parabola() {
        // A crest: up 20 units over 100, an arc of 90° to down 20 over 100, unexaggerated.
        let view = View { exaggeration: 1.0, ..VIEW };
        let r = 50.0_f64;
        let s = std::f64::consts::FRAC_1_SQRT_2 * r;
        let pts = [[0.0, 0.0], [100.0 - s, 100.0 - s], [100.0 + s, 100.0 - s], [200.0, 0.0]];
        let bulge = -(std::f64::consts::FRAC_PI_8).tan();
        let pvis = from_drawing(&pts, &[0.0, bulge, 0.0], view, 0.0).expect("a profile");
        assert_eq!(pvis.len(), 3);
        assert!((pvis[1].station - (view.station + 100.0 - view.at[0])).abs() < 1e-9, "{pvis:?}");
        match pvis[1].curve {
            VCurve::Parabola { length } => assert!((length - 2.0 * s).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn two_arcs_in_a_row_are_refused() {
        let pts = [[100.0, 50.0], [150.0, 60.0], [200.0, 50.0]];
        assert_eq!(from_drawing(&pts, &[0.1, -0.1], VIEW, 0.0), Err(TraceError::Compound { segment: 1 }));
        let back = [[100.0, 50.0], [90.0, 60.0], [200.0, 50.0]];
        assert_eq!(from_drawing(&back, &[0.0, 0.0], VIEW, 0.0), Err(TraceError::NotIncreasing { vertex: 1 }));
    }
}
