//! Kesit (docs/adr/8061 §5 "Kesit", docs/adr/8073 §3): a plane across the model, one side of it kept, or a box about
//! the chosen objects. It gives the half-spaces the renderer clips with (what is cut away is not drawn; the cut faces
//! are filled by [`crate::caps`], docs/adr/8131) and the part of a ray that is kept, so a click never meets what is cut
//! away. The planes are in the
//! source's space; [`shown`] takes one into the shown space of a vertical exaggeration.

use kentos_scene3d::{Aabb, Exaggeration};

/// The plane's direction: a horizontal cut at a height, or a vertical one along a north–south or an east–west line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    /// Horizontal, at a height (the model seen from above as a plan).
    Level,
    /// Vertical along a north–south line, at an easting.
    NorthSouth,
    /// Vertical along an east–west line, at a northing.
    EastWest,
}

impl Cut {
    /// The axis the plane's position is measured along (0 east, 1 north, 2 up).
    pub fn axis(self) -> usize {
        match self {
            Cut::NorthSouth => 0,
            Cut::EastWest => 1,
            Cut::Level => 2,
        }
    }
}

/// The section.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Section {
    #[default]
    None,
    /// A plane at `at` along its axis; the lower side kept (below a level cut, west of a north–south one, south of an
    /// east–west one), the other when `flip`.
    Plane { cut: Cut, at: f64, flip: bool },
    /// A box, what is outside it cut away.
    Box(Aabb),
}

impl Section {
    /// The half-spaces kept, each `[a, b, c, d]` keeping the points with a·x + b·y + c·z + d ≥ 0, the normal of unit
    /// length.
    pub fn planes(&self) -> Vec<[f64; 4]> {
        match *self {
            Section::None => Vec::new(),
            Section::Plane { cut, at, flip } => {
                let mut n = [0.0; 3];
                let s = if flip { 1.0 } else { -1.0 };
                n[cut.axis()] = s;
                vec![[n[0], n[1], n[2], -s * at]]
            }
            Section::Box(b) => (0..3)
                .flat_map(|k| {
                    let (mut lo, mut hi) = ([0.0; 4], [0.0; 4]);
                    lo[k] = 1.0;
                    lo[3] = -b.min[k];
                    hi[k] = -1.0;
                    hi[3] = b.max[k];
                    [lo, hi]
                })
                .collect(),
        }
    }
}

/// A plane of the source's space in the shown space of `ex`: a height z is shown at datum + k·(z − datum), so the
/// normal's height part is divided by k and the constant moved with the datum; the normal is made unit again.
pub fn shown(p: [f64; 4], ex: &Exaggeration) -> [f64; 4] {
    let k = ex.factor;
    let c = p[2] / k;
    let d = p[3] + p[2] * ex.datum - c * ex.datum;
    let len = (p[0] * p[0] + p[1] * p[1] + c * c).sqrt();
    if len == 0.0 || !len.is_finite() {
        return p;
    }
    [p[0] / len, p[1] / len, c / len, d / len]
}

/// The part of the ray `o + t·d` (t ≥ 0) that every plane keeps, `[t0, t1]`; none when no part is kept. The planes
/// keep a convex region, so the part is one interval.
pub fn kept(planes: &[[f64; 4]], o: [f64; 3], d: [f64; 3]) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0f64, f64::INFINITY);
    for p in planes {
        let at = p[0] * o[0] + p[1] * o[1] + p[2] * o[2] + p[3];
        let rate = p[0] * d[0] + p[1] * d[1] + p[2] * d[2];
        if rate == 0.0 {
            if at < 0.0 {
                return None;
            }
            continue;
        }
        let t = -at / rate;
        if rate > 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
    }
    (t0 <= t1).then_some((t0, t1))
}

/// The box about `b` a section box takes: `b` and a twentieth of its diagonal round it, so the chosen objects' own
/// faces are not cut (a view's choice, not a measure).
pub fn about(b: &Aabb) -> Aabb {
    let m = b.diagonal() / 20.0;
    Aabb { min: [b.min[0] - m, b.min[1] - m, b.min[2] - m], max: [b.max[0] + m, b.max[1] + m, b.max[2] + m] }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keeps(planes: &[[f64; 4]], p: [f64; 3]) -> bool {
        planes.iter().all(|q| q[0] * p[0] + q[1] * p[1] + q[2] * p[2] + q[3] >= 0.0)
    }

    #[test]
    fn a_level_cut_keeps_what_is_below_it_and_flipped_what_is_above() {
        let below = Section::Plane { cut: Cut::Level, at: 3.0, flip: false }.planes();
        assert!(keeps(&below, [0.0, 0.0, 2.9]) && !keeps(&below, [0.0, 0.0, 3.1]));
        let above = Section::Plane { cut: Cut::Level, at: 3.0, flip: true }.planes();
        assert!(!keeps(&above, [0.0, 0.0, 2.9]) && keeps(&above, [0.0, 0.0, 3.1]));
        let west = Section::Plane { cut: Cut::NorthSouth, at: 10.0, flip: false }.planes();
        assert!(keeps(&west, [9.0, 50.0, 0.0]) && !keeps(&west, [11.0, 50.0, 0.0]));
        let south = Section::Plane { cut: Cut::EastWest, at: -4.0, flip: false }.planes();
        assert!(keeps(&south, [100.0, -5.0, 0.0]) && !keeps(&south, [100.0, -3.0, 0.0]));
    }

    #[test]
    fn a_box_keeps_its_inside_and_a_ray_s_kept_part_is_inside_it() {
        let b = Aabb { min: [0.0, 0.0, 0.0], max: [10.0, 4.0, 3.0] };
        let planes = Section::Box(b).planes();
        assert_eq!(planes.len(), 6);
        assert!(keeps(&planes, [5.0, 2.0, 1.0]) && !keeps(&planes, [11.0, 2.0, 1.0]) && !keeps(&planes, [5.0, 2.0, -0.1]));
        // Along east from x = −5: inside from 5 to 15.
        let (t0, t1) = kept(&planes, [-5.0, 2.0, 1.0], [1.0, 0.0, 0.0]).expect("it passes through");
        assert!((t0 - 5.0).abs() < 1e-12 && (t1 - 15.0).abs() < 1e-12);
        assert_eq!(kept(&planes, [-5.0, 9.0, 1.0], [1.0, 0.0, 0.0]), None, "it passes beside the box");
        assert_eq!(kept(&[], [0.0; 3], [1.0, 0.0, 0.0]), Some((0.0, f64::INFINITY)));
        let a = about(&b);
        assert!(a.min[0] < 0.0 && a.max[2] > 3.0);
    }

    #[test]
    fn a_plane_in_the_shown_space_cuts_where_the_source_s_did() {
        let ex = Exaggeration { factor: 3.0, datum: 100.0 };
        let p = Section::Plane { cut: Cut::Level, at: 104.0, flip: false }.planes()[0];
        let s = shown(p, &ex);
        // 104 is shown at 112; the shown plane keeps 111.9, not 112.1.
        assert!(keeps(&[s], [0.0, 0.0, 111.9]) && !keeps(&[s], [0.0, 0.0, 112.1]));
        let v = Section::Plane { cut: Cut::NorthSouth, at: 5.0, flip: true }.planes()[0];
        assert_eq!(shown(v, &ex), v, "a vertical plane is the same shown");
    }
}
