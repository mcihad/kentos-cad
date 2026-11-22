//! Kesit (docs/adr/8061 §5 "Kesit", docs/adr/8073 §3): a plane across the model, one side of it kept, or a box about
//! the chosen objects. It gives the half-spaces the renderer clips with (what is cut away is not drawn; the cut faces
//! are filled by [`crate::caps`], docs/adr/8131) and the part of a ray that is kept, so a click never meets what is cut
//! away. A plane may also be made parallel to a clicked face ([`Section::from_face`]); its handles ([`Handle`], a
//! plane's one, a box's six) move it in the scene (docs/adr/8132). The planes are in the source's space; [`shown`]
//! takes one into the shown space of a vertical exaggeration.

use kentos_scene3d::math::{cross, dot, length, scale, sub};
use kentos_scene3d::{Aabb, Exaggeration};

/// How far in front of a clicked face a plane made from it passes, metres (towards where the face was seen from): the
/// face itself is not cut, whichever side its corners' last bits fall on.
pub const FACE_GAP: f64 = 0.001;

/// How thin a section box's face may bring it, metres.
pub const THINNEST_BOX: f64 = 0.01;

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
    /// A plane parallel to a clicked face, not along an axis (docs/adr/8132): `origin` the point clicked, `normal` the
    /// face's (unit, towards where it was seen from); the plane `depth` behind the face along it (negative: in front),
    /// the side behind the plane kept, the other when `flip`.
    Face { origin: [f64; 3], normal: [f64; 3], depth: f64, flip: bool },
    /// A box, what is outside it cut away.
    Box(Aabb),
}

/// A handle of a section in the scene (docs/adr/8132): where it stands (on the plane, or a box's face, across the
/// middle of the model) and the way it points, unit, towards the side its plane cuts away.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Handle {
    pub at: [f64; 3],
    pub dir: [f64; 3],
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
            // The plane n·x = n·origin − depth; behind it n·x is less.
            Section::Face { origin, normal: n, depth, flip } => {
                let c = dot(n, origin) - depth;
                let s = if flip { 1.0 } else { -1.0 };
                vec![[s * n[0], s * n[1], s * n[2], -s * c]]
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

impl Section {
    /// The section a click on a face makes (docs/adr/8132): a plane parallel to it, [`FACE_GAP`] in front of it, the
    /// side behind kept; `point` the point clicked, `normal` the face's (either way), `eye` where it is seen from and
    /// `bounds` the model's box. A face along an axis, leaning off it by less than half the gap across the model, makes
    /// that axis's plane (a level cut, a north–south or an east–west one); none for a face without a normal.
    pub fn from_face(point: [f64; 3], normal: [f64; 3], eye: [f64; 3], bounds: &Aabb) -> Option<Section> {
        let l = length(normal);
        if !(l > 0.0 && l.is_finite()) {
            return None;
        }
        let mut n = scale(normal, 1.0 / l);
        if dot(n, sub(eye, point)) < 0.0 {
            n = scale(n, -1.0);
        }
        let reach = bounds.diagonal().max(1.0);
        for axis in 0..3 {
            // The sine of the angle off the axis.
            let lean = (1.0 - n[axis] * n[axis]).max(0.0).sqrt();
            if lean * reach <= FACE_GAP / 2.0 {
                let cut = match axis {
                    0 => Cut::NorthSouth,
                    1 => Cut::EastWest,
                    _ => Cut::Level,
                };
                // Seen from the upper side, the lower kept (unflipped), the plane a gap above the face.
                let up = n[axis] > 0.0;
                let at = point[axis] + if up { FACE_GAP } else { -FACE_GAP };
                return Some(Section::Plane { cut, at, flip: !up });
            }
        }
        Some(Section::Face { origin: point, normal: n, depth: -FACE_GAP, flip: false })
    }

    /// A plane's place, as its card's slider gives it: a plane's along its axis, a face's depth.
    pub fn place(&self) -> Option<f64> {
        match *self {
            Section::Plane { at, .. } => Some(at),
            Section::Face { depth, .. } => Some(depth),
            Section::None | Section::Box(_) => None,
        }
    }

    /// A plane put at `v` (its place as [`Section::place`] says).
    pub fn set_place(&mut self, v: f64) {
        match self {
            Section::Plane { at, .. } => *at = v,
            Section::Face { depth, .. } => *depth = v,
            Section::None | Section::Box(_) => {}
        }
    }

    /// The places a plane takes across the model's box `bounds`: along its axis the box's span; a face's depths from
    /// where it meets the box's corner nearest the viewer to where it meets the farthest.
    pub fn range(&self, bounds: &Aabb) -> Option<(f64, f64)> {
        if bounds.is_empty() {
            return None;
        }
        match *self {
            Section::Plane { cut, .. } => Some((bounds.min[cut.axis()], bounds.max[cut.axis()])),
            Section::Face { origin, normal, .. } => {
                let (lo, hi) = corners(bounds).iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), c| {
                    let d = dot(normal, sub(*c, origin));
                    (lo.min(d), hi.max(d))
                });
                Some((-hi, -lo))
            }
            Section::None | Section::Box(_) => None,
        }
    }

    /// Its handles in the scene across the model's box `bounds`: a plane's one where the box's middle falls on it,
    /// pointing to the side it cuts away; a box's six at its faces' middles, pointing out (west, east, south, north,
    /// down, up: its planes' order).
    pub fn handles(&self, bounds: &Aabb) -> Vec<Handle> {
        let middle = bounds.center();
        match *self {
            Section::None => Vec::new(),
            _ if bounds.is_empty() => Vec::new(),
            Section::Plane { cut, at, flip } => {
                let k = cut.axis();
                let mut on = middle;
                on[k] = at;
                let mut dir = [0.0; 3];
                dir[k] = if flip { -1.0 } else { 1.0 };
                vec![Handle { at: on, dir }]
            }
            Section::Face { origin, normal, depth, flip } => {
                let off = dot(normal, sub(middle, origin)) + depth;
                vec![Handle { at: sub(middle, scale(normal, off)), dir: if flip { scale(normal, -1.0) } else { normal } }]
            }
            Section::Box(b) => {
                let centre = b.center();
                (0..6)
                    .map(|h| {
                        let (k, low) = (h / 2, h % 2 == 0);
                        let mut at = centre;
                        at[k] = if low { b.min[k] } else { b.max[k] };
                        let mut dir = [0.0; 3];
                        dir[k] = if low { -1.0 } else { 1.0 };
                        Handle { at, dir }
                    })
                    .collect()
            }
        }
    }

    /// Its handle `handle` moved `along` metres the way it points (docs/adr/8132): a plane within the model's box
    /// `bounds` ([`Section::range`]); a box's face, the box no thinner than [`THINNEST_BOX`].
    pub fn moved(&mut self, handle: usize, along: f64, bounds: &Aabb) {
        if !along.is_finite() {
            return;
        }
        let range = self.range(bounds);
        match self {
            Section::None => {}
            Section::Plane { at, flip, .. } => {
                let v = *at + if *flip { -along } else { along };
                *at = range.map_or(v, |(lo, hi)| v.clamp(lo, hi));
            }
            // Pointing along the normal the plane comes nearer the viewer: less deep.
            Section::Face { depth, flip, .. } => {
                let v = *depth + if *flip { along } else { -along };
                *depth = range.map_or(v, |(lo, hi)| v.clamp(lo, hi));
            }
            Section::Box(b) => {
                let (k, low) = (handle / 2, handle.is_multiple_of(2));
                if k > 2 {
                    return;
                }
                if low {
                    b.min[k] = (b.min[k] - along).min(b.max[k] - THINNEST_BOX);
                } else {
                    b.max[k] = (b.max[k] + along).max(b.min[k] + THINNEST_BOX);
                }
            }
        }
    }
}

/// A box's eight corners.
fn corners(b: &Aabb) -> [[f64; 3]; 8] {
    std::array::from_fn(|i| {
        [if i & 1 == 0 { b.min[0] } else { b.max[0] }, if i & 2 == 0 { b.min[1] } else { b.max[1] }, if i & 4 == 0 { b.min[2] } else { b.max[2] }]
    })
}

/// A triangle's normal, of unit length; none for one without area.
pub fn normal_of(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Option<[f64; 3]> {
    let n = cross(sub(b, a), sub(c, a));
    let l = length(n);
    (l > 0.0 && l.is_finite()).then(|| scale(n, 1.0 / l))
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

    /// A 30 m model: a click on a slab's top seen from above makes a level cut a millimetre over it (what is below
    /// kept); on a wall's west face seen from the west a north–south one a millimetre west of it (the east kept); on a
    /// face turned 45° a plane parallel to it a millimetre in front, the face and what is behind it kept. A face leaning
    /// a millionth of a radian off an axis is that axis's plane; one leaning a thousandth is not; a face without area
    /// makes none.
    #[test]
    fn a_click_on_a_face_makes_a_plane_parallel_to_it_a_hair_in_front() {
        let model = Aabb { min: [0.0, 0.0, 0.0], max: [20.0, 20.0, 9.0] };
        let top = Section::from_face([4.0, 4.0, 3.0], [0.0, 0.0, -1.0], [4.0, 4.0, 50.0], &model).expect("a plane");
        assert_eq!(top, Section::Plane { cut: Cut::Level, at: 3.0 + FACE_GAP, flip: false });
        let west = Section::from_face([0.0, 7.0, 1.0], [1.0, 0.0, 0.0], [-30.0, 7.0, 1.0], &model).expect("a plane");
        assert_eq!(west, Section::Plane { cut: Cut::NorthSouth, at: -FACE_GAP, flip: true });
        let p = west.planes();
        assert!(keeps(&p, [0.0, 7.0, 1.0]) && keeps(&p, [3.0, 7.0, 1.0]) && !keeps(&p, [-0.01, 7.0, 1.0]));
        let r = std::f64::consts::FRAC_1_SQRT_2;
        let turned = Section::from_face([10.0, 10.0, 2.0], [-r, -r, 0.0], [40.0, 40.0, 10.0], &model).expect("a plane");
        let Section::Face { normal, depth, flip, .. } = turned else { panic!("a face's plane: {turned:?}") };
        assert!((normal[0] - r).abs() < 1e-12 && (normal[1] - r).abs() < 1e-12 && depth == -FACE_GAP && !flip);
        let p = turned.planes();
        assert!(keeps(&p, [10.0, 10.0, 2.0]) && keeps(&p, [9.0, 9.0, 0.0]) && !keeps(&p, [10.01, 10.01, 2.0]));
        let lean = |a: f64| [a.sin(), 0.0, a.cos()];
        let nearly = Section::from_face([1.0, 1.0, 3.0], lean(1e-6), [1.0, 1.0, 50.0], &model);
        assert!(matches!(nearly, Some(Section::Plane { cut: Cut::Level, .. })), "{nearly:?}");
        let leaning = Section::from_face([1.0, 1.0, 3.0], lean(1e-3), [1.0, 1.0, 50.0], &model);
        assert!(matches!(leaning, Some(Section::Face { .. })), "{leaning:?}");
        assert_eq!(Section::from_face([0.0; 3], [0.0; 3], [1.0; 3], &model), None);
        assert_eq!(normal_of([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), Some([0.0, 0.0, 1.0]));
        assert_eq!(normal_of([0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]), None);
    }

    /// A face's plane moves along its normal between where it meets the model's nearest and farthest corners, its
    /// handle pointing to the viewer's side; flipped, the other side is kept and the handle points away.
    #[test]
    fn a_face_s_plane_moves_along_its_normal_within_the_model_and_flips() {
        let model = Aabb { min: [0.0, 0.0, 0.0], max: [10.0, 10.0, 3.0] };
        let r = std::f64::consts::FRAC_1_SQRT_2;
        let mut s = Section::from_face([5.0, 5.0, 1.0], [r, r, 0.0], [30.0, 30.0, 5.0], &model).expect("a plane");
        // Corners from (0, 0) to (10, 10): n·(c − origin) from −5√2 to 5√2.
        let (lo, hi) = s.range(&model).expect("a range");
        assert!((lo + 5.0 * 2f64.sqrt()).abs() < 1e-12 && (hi - 5.0 * 2f64.sqrt()).abs() < 1e-12);
        let h = s.handles(&model);
        assert_eq!(h.len(), 1);
        assert!((h[0].dir[0] - r).abs() < 1e-12, "towards the viewer");
        // The model's middle (5, 5, 1.5) is on the face's line: the handle stands a gap in front of it.
        assert!((h[0].at[0] - (5.0 + FACE_GAP * r)).abs() < 1e-9 && (h[0].at[2] - 1.5).abs() < 1e-12);
        // Two metres back along its arrow: two metres deeper, then no deeper than the far corner.
        s.moved(0, -2.0, &model);
        assert!((s.place().expect("a place") - (2.0 - FACE_GAP)).abs() < 1e-12);
        let p = s.planes();
        assert!(keeps(&p, [3.0, 3.0, 1.0]) && !keeps(&p, [4.5, 4.5, 1.0]));
        s.moved(0, -100.0, &model);
        assert!((s.place().expect("a place") - hi).abs() < 1e-12);
        s.set_place(0.5);
        let Section::Face { flip, .. } = &mut s else { panic!("a face's plane") };
        *flip = true;
        let p = s.planes();
        assert!(!keeps(&p, [3.0, 3.0, 1.0]) && keeps(&p, [5.0, 5.0, 1.0]), "flipped: the viewer's side kept");
        assert!((s.handles(&model)[0].dir[0] + r).abs() < 1e-12, "flipped: pointing away");
        s.moved(0, 1.0, &model);
        assert!((s.place().expect("a place") - 1.5).abs() < 1e-12, "flipped, along its arrow it goes deeper");
    }

    /// A level cut's handle stands at the model's middle on the plane, pointing up (what it cuts away), and moves the
    /// plane within the model; a box's six stand at its faces' middles pointing out and move each face, the box never
    /// thinner than a centimetre.
    #[test]
    fn handles_stand_on_the_plane_or_the_box_s_faces_and_move_them() {
        let model = Aabb { min: [0.0, 0.0, 0.0], max: [10.0, 4.0, 6.0] };
        let mut level = Section::Plane { cut: Cut::Level, at: 2.0, flip: false };
        assert_eq!(level.handles(&model), vec![Handle { at: [5.0, 2.0, 2.0], dir: [0.0, 0.0, 1.0] }]);
        level.moved(0, 1.5, &model);
        assert_eq!(level.place(), Some(3.5));
        level.moved(0, 10.0, &model);
        assert_eq!(level.place(), Some(6.0), "within the model");
        let mut above = Section::Plane { cut: Cut::Level, at: 2.0, flip: true };
        assert_eq!(above.handles(&model)[0].dir, [0.0, 0.0, -1.0]);
        above.moved(0, 0.5, &model);
        assert_eq!(above.place(), Some(1.5), "flipped, along its arrow it goes down");
        let mut b = Section::Box(Aabb { min: [1.0, 1.0, 0.0], max: [3.0, 2.0, 3.0] });
        let h = b.handles(&model);
        assert_eq!(h.len(), 6);
        assert_eq!(h[0], Handle { at: [1.0, 1.5, 1.5], dir: [-1.0, 0.0, 0.0] });
        assert_eq!(h[5], Handle { at: [2.0, 1.5, 3.0], dir: [0.0, 0.0, 1.0] });
        b.moved(1, 2.0, &model);
        b.moved(0, -5.0, &model);
        let Section::Box(after) = b else { panic!("a box") };
        assert_eq!(after.max[0], 5.0);
        assert!((after.min[0] - (5.0 - THINNEST_BOX)).abs() < 1e-12, "no thinner than a centimetre");
        assert!(Section::None.handles(&model).is_empty() && level.handles(&Aabb::EMPTY).is_empty());
    }
}
