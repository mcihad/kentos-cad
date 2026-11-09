//! A stockpile's volume over its base (plan/map-cad.md MAP-15, docs/adr/3140; Pix4D's volume base surfaces): the part
//! of a surface inside a closed boundary measured against a plane drawn from the boundary's outer corners: the lowest,
//! the highest or the average of their heights on the surface, the plane fitted to them (the least squares of the
//! vertical differences), or a height given; or against the corners triangulated (docs/adr/3940: Pix4D's
//! Triangulated, each triangle its plane). Above the base is cut (the pile), below it fill, Pix4D's words.
//!
//! Each kept triangle clips the boundary's rings (Sutherland–Hodgman with the triangle as the window: a concave ring
//! gives a polygon whose doubled edges along the window add no area), a triangulated base's triangles clip the part
//! again; the part is split where the difference of the surface's plane and the base is zero, and the linear
//! difference is integrated exactly over each piece from its area and first moments. The outer ring turns
//! counterclockwise and the holes clockwise, so a hole's pieces subtract. Coordinates are taken from the boundary's
//! first corner: large map coordinates keep their digits.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::predicates::incircle;

use crate::surface::{Breakline, Region, Ring, SpotPoint, Surface, SurfaceInput};

/// The base of the volume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Base {
    /// Level at the lowest corner height.
    Lowest,
    /// Level at the highest corner height.
    Highest,
    /// Level at the corners' average height.
    Average,
    /// The plane fitted to the corners: the least squares of the vertical differences.
    Fit,
    /// Level at a given height.
    Elevation(f64),
    /// The outer corners triangulated with their heights (docs/adr/3940): [`base_tin`].
    Triangulated,
}

/// A plane `z = a·x + b·y + c` in the project's coordinates.
pub type Plane = [f64; 3];

/// Why a base is not drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseError {
    /// The outer corner (its index from 0) is off the surface.
    Off(usize),
    /// The corners lie on a line: no plane fits them.
    Collinear,
    /// The corners are not triangulated (the surface core's code: a ring crossing itself, `constraints_cross`).
    Untriangulated(&'static str),
}

/// The boundary's outer corners' heights on the surface.
pub fn corner_heights(surface: &Surface, region: &Region) -> Result<Vec<f64>, BaseError> {
    region
        .outer
        .iter()
        .enumerate()
        .map(|(k, p)| surface.height(p[0], p[1]).z.ok_or(BaseError::Off(k)))
        .collect()
}

/// The base plane from the outer corners and their heights (at least three).
pub fn base_plane(base: Base, corners: &[[f64; 2]], heights: &[f64]) -> Result<Plane, BaseError> {
    let level = |z: f64| Ok([0.0, 0.0, z]);
    match base {
        Base::Elevation(z) => level(z),
        Base::Lowest => level(heights.iter().copied().fold(f64::INFINITY, f64::min)),
        Base::Highest => level(heights.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        Base::Average => level(heights.iter().sum::<f64>() / heights.len() as f64),
        Base::Fit => fit(corners, heights),
        // Its triangles' planes are the base, not one plane: [`base_tin`] and [`measure_tin`] (here as for corners on
        // a line: no plane).
        Base::Triangulated => Err(BaseError::Collinear),
    }
}

/// A triangulated base (docs/adr/3940): its triangles, each three corners `[x, y, z]` counterclockwise from its
/// smallest place (x, then y), the list ordered by them; and the diagonals two triangulations would draw differently
/// (four corners on one circle), each its two ends by place, ordered.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tin {
    pub triangles: Vec<[[f64; 3]; 3]>,
    pub undecided: Vec<[[f64; 2]; 2]>,
}

/// The outer corners with their heights triangulated inside the ring: the surface core's constrained Delaunay
/// triangulation (`kentos.cdt/1`) of the ring as a closed breakline clipped by the ring, so a concave ring's notch
/// takes no triangle. Where four corners are on one circle and no edge of the ring decides, the core's diagonal is
/// taken and the edge said (`undecided`).
pub fn base_tin(corners: &[[f64; 2]], heights: &[f64]) -> Result<Tin, BaseError> {
    let n = corners.len().min(heights.len());
    let ring: Vec<[f64; 3]> = (0..n).map(|k| [corners[k][0], corners[k][1], heights[k]]).collect();
    let input = SurfaceInput {
        points: ring.iter().map(|c| SpotPoint { x: c[0], y: c[1], z: c[2], source: None }).collect(),
        breaklines: vec![Breakline { points: ring.clone(), closed: true, source: None }],
        boundary: Some(Ring { points: corners[..n].to_vec(), source: None }),
        ..SurfaceInput::default()
    };
    let s = crate::surface::build(&input).map_err(|e| BaseError::Untriangulated(e.code))?;
    let at = |i: u32| s.vertices[i as usize];
    let by_place = |a: &[f64; 3], b: &[f64; 3]| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1]));
    let mut triangles: Vec<[[f64; 3]; 3]> = s
        .triangles
        .iter()
        .map(|t| {
            let c = t.map(at);
            let first = (0..3).min_by(|&i, &j| by_place(&c[i], &c[j])).unwrap_or(0);
            [c[first], c[(first + 1) % 3], c[(first + 2) % 3]]
        })
        .collect();
    triangles.sort_by(|a, b| by_place(&a[0], &b[0]).then(by_place(&a[1], &b[1])).then(by_place(&a[2], &b[2])));
    // The edges two kept triangles share that are no piece of the ring: four corners on one circle leave them open.
    let v2 = |i: u32| Vec2::new(at(i)[0], at(i)[1]);
    let mut sides: std::collections::BTreeMap<[u32; 2], Vec<u32>> = std::collections::BTreeMap::new();
    for t in &s.triangles {
        for k in 0..3 {
            let (a, b, far) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
            sides.entry([a.min(b), a.max(b)]).or_default().push(far);
        }
    }
    let mut undecided = Vec::new();
    for (e, far) in &sides {
        if let [c, d] = far[..]
            && s.constrained.binary_search(e).is_err()
            && incircle(v2(e[0]), v2(e[1]), v2(c), v2(d)) == 0.0
        {
            let (p, q) = (at(e[0]), at(e[1]));
            let (p, q) = if by_place(&p, &q).is_le() { (p, q) } else { (q, p) };
            undecided.push([[p[0], p[1]], [q[0], q[1]]]);
        }
    }
    undecided.sort_by(|a, b| a[0][0].total_cmp(&b[0][0]).then(a[0][1].total_cmp(&b[0][1])));
    Ok(Tin { triangles, undecided })
}

/// The plane fitted to the corners: centred on their mean, `a` and `b` from the 2 × 2 normal equations, `c` through
/// the mean point.
fn fit(corners: &[[f64; 2]], heights: &[f64]) -> Result<Plane, BaseError> {
    let n = corners.len().min(heights.len());
    if n < 3 {
        return Err(BaseError::Collinear);
    }
    let o = corners[0];
    let nf = n as f64;
    let (mut su, mut sv, mut sz) = (0.0, 0.0, 0.0);
    for k in 0..n {
        su += corners[k][0] - o[0];
        sv += corners[k][1] - o[1];
        sz += heights[k];
    }
    let (mu, mv, mz) = (su / nf, sv / nf, sz / nf);
    let (mut suu, mut svv, mut suv, mut suz, mut svz) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for k in 0..n {
        let (u, v, z) = (corners[k][0] - o[0] - mu, corners[k][1] - o[1] - mv, heights[k] - mz);
        suu += u * u;
        svv += v * v;
        suv += u * v;
        suz += u * z;
        svz += v * z;
    }
    let det = suu * svv - suv * suv;
    let spread = suu + svv;
    if !(det > 1e-12 * spread * spread) {
        return Err(BaseError::Collinear);
    }
    let a = (suz * svv - svz * suv) / det;
    let b = (svz * suu - suz * suv) / det;
    // Through the mean point (o + mean, mz), in the project's coordinates.
    let (px, py) = (o[0] + mu, o[1] + mv);
    Ok([a, b, mz - a * px - b * py])
}

/// What the base leaves inside the boundary: its area, the part over the surface, the cut (above the base) and fill
/// (below it) volumes and the areas where they are.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Measured {
    pub area: f64,
    pub covered: f64,
    pub cut: f64,
    pub fill: f64,
    pub cut_area: f64,
    pub fill_area: f64,
}

type P = [f64; 2];

pub(crate) fn signed_area(ring: &[P]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1]).sum::<f64>() / 2.0
}

/// A ring turned counterclockwise (`ccw`) or clockwise.
pub(crate) fn oriented(mut ring: Vec<P>, ccw: bool) -> Vec<P> {
    if (signed_area(&ring) > 0.0) != ccw {
        ring.reverse();
    }
    ring
}

/// The area and first moments (∫x, ∫y) of a polygon, signed by its turn.
pub(crate) fn moments(poly: &[P]) -> (f64, f64, f64) {
    let n = poly.len();
    let (mut a, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let w = p[0] * q[1] - q[0] * p[1];
        a += w;
        sx += (p[0] + q[0]) * w;
        sy += (p[1] + q[1]) * w;
    }
    (a / 2.0, sx / 6.0, sy / 6.0)
}

/// The part of `poly` where `f ≥ 0` (Sutherland–Hodgman against one half-plane), `f` linear.
pub(crate) fn keep(poly: &[P], f: &dyn Fn(P) -> f64) -> Vec<P> {
    let n = poly.len();
    let mut out = Vec::with_capacity(n + 2);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let (fp, fq) = (f(p), f(q));
        if fp >= 0.0 {
            out.push(p);
        }
        if (fp >= 0.0) != (fq >= 0.0) {
            let t = fp / (fp - fq);
            out.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]);
        }
    }
    out
}

/// The surface's part inside `region` against the base `plane`.
pub fn measure(surface: &Surface, region: &Region, plane: Plane) -> Measured {
    measure_over(surface, region, &[], plane)
}

/// The surface's part inside `region` against a triangulated base (docs/adr/3940).
pub fn measure_tin(surface: &Surface, region: &Region, tin: &Tin) -> Measured {
    measure_over(surface, region, &tin.triangles, [0.0; 3])
}

/// The plane `[a, b, c]` through a triangle's corners, in coordinates shifted by `o`, and the shifted corners.
fn plane_of(t: &[[f64; 3]; 3], o: P) -> ([P; 3], [f64; 3]) {
    let v = t.map(|c| ([c[0] - o[0], c[1] - o[1]], c[2]));
    let (e1, e2) = ([v[1].0[0] - v[0].0[0], v[1].0[1] - v[0].0[1]], [v[2].0[0] - v[0].0[0], v[2].0[1] - v[0].0[1]]);
    let det = e1[0] * e2[1] - e1[1] * e2[0];
    let (dz1, dz2) = (v[1].1 - v[0].1, v[2].1 - v[0].1);
    let a = (dz1 * e2[1] - dz2 * e1[1]) / det;
    let b = (e1[0] * dz2 - e2[0] * dz1) / det;
    (v.map(|c| c.0), [a, b, v[0].1 - a * v[0].0[0] - b * v[0].0[1]])
}

/// [`measure`] and [`measure_tin`]: against `plane` when `tin` is empty, else against each of its triangles' planes
/// inside it (counterclockwise triangles).
fn measure_over(surface: &Surface, region: &Region, tin: &[[[f64; 3]; 3]], plane: Plane) -> Measured {
    let Some(&o) = region.outer.first() else {
        return Measured::default();
    };
    let shift = |p: &P| [p[0] - o[0], p[1] - o[1]];
    let mut rings = vec![oriented(region.outer.iter().map(shift).collect(), true)];
    rings.extend(region.holes.iter().map(|h| oriented(h.iter().map(shift).collect(), false)));
    let mut m = Measured { area: rings.iter().map(|r| signed_area(r)).sum(), ..Measured::default() };
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in &rings[0] {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    // The base in the shifted coordinates: the plane (no window), or each triangle its window and plane.
    let bases: Vec<(Option<[P; 3]>, [f64; 3])> = if tin.is_empty() {
        vec![(None, [plane[0], plane[1], plane[0] * o[0] + plane[1] * o[1] + plane[2]])]
    } else {
        tin.iter().map(|t| plane_of(t, o)).map(|(w, p)| (Some(w), p)).collect()
    };
    for t in &surface.triangles {
        let mut v = t.map(|i| {
            let c = surface.vertices[i as usize];
            ([c[0] - o[0], c[1] - o[1]], c[2])
        });
        if v.iter().all(|(p, _)| p[0] < lo[0]) || v.iter().all(|(p, _)| p[0] > hi[0])
            || v.iter().all(|(p, _)| p[1] < lo[1]) || v.iter().all(|(p, _)| p[1] > hi[1])
        {
            continue;
        }
        let turn = |v: &[(P, f64); 3]| {
            (v[1].0[0] - v[0].0[0]) * (v[2].0[1] - v[0].0[1]) - (v[1].0[1] - v[0].0[1]) * (v[2].0[0] - v[0].0[0])
        };
        if turn(&v) < 0.0 {
            v.swap(1, 2);
        }
        let det = turn(&v);
        if det <= 0.0 {
            continue;
        }
        // The surface's plane over the triangle, then the difference d = surface − base.
        let (e1, e2) = ([v[1].0[0] - v[0].0[0], v[1].0[1] - v[0].0[1]], [v[2].0[0] - v[0].0[0], v[2].0[1] - v[0].0[1]]);
        let (dz1, dz2) = (v[1].1 - v[0].1, v[2].1 - v[0].1);
        let sa = (dz1 * e2[1] - dz2 * e1[1]) / det;
        let sb = (e1[0] * dz2 - e2[0] * dz1) / det;
        let sc = v[0].1 - sa * v[0].0[0] - sb * v[0].0[1];
        let window = [v[0].0, v[1].0, v[2].0];
        for ring in &rings {
            let poly = clipped(ring.clone(), &window);
            if poly.len() < 3 {
                continue;
            }
            for (base_window, base) in &bases {
                let poly = match base_window {
                    None => poly.clone(),
                    Some(w) if apart(&window, w) => continue,
                    Some(w) => clipped(poly.clone(), w),
                };
                if poly.len() < 3 {
                    continue;
                }
                let (da, db, dc) = (sa - base[0], sb - base[1], sc - base[2]);
                let up = move |p: P| da * p[0] + db * p[1] + dc;
                let down = move |p: P| -(da * p[0] + db * p[1] + dc);
                m.covered += moments(&poly).0;
                for (above, side) in [(true, &up as &dyn Fn(P) -> f64), (false, &down)] {
                    let part = keep(&poly, side);
                    if part.len() < 3 {
                        continue;
                    }
                    let (a, sx, sy) = moments(&part);
                    let integral = da * sx + db * sy + dc * a;
                    if above {
                        m.cut += integral;
                        m.cut_area += a;
                    } else {
                        m.fill -= integral;
                        m.fill_area += a;
                    }
                }
            }
        }
    }
    m
}

/// `poly` clipped by the counterclockwise triangle `w` (Sutherland–Hodgman); fewer than three corners: nothing left.
fn clipped(mut poly: Vec<P>, w: &[P; 3]) -> Vec<P> {
    for k in 0..3 {
        let (a, b) = (w[k], w[(k + 1) % 3]);
        let left = move |p: P| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        poly = keep(&poly, &left);
        if poly.len() < 3 {
            return Vec::new();
        }
    }
    poly
}

/// Whether two triangles' boxes are apart (then they share no area).
fn apart(a: &[P; 3], b: &[P; 3]) -> bool {
    let lo = |t: &[P; 3], k: usize| t.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
    let hi = |t: &[P; 3], k: usize| t.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
    (0..2).any(|k| hi(a, k) < lo(b, k) || hi(b, k) < lo(a, k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fitted_plane_goes_through_points_on_a_plane() {
        let corners = [[1000.0, 2000.0], [1010.0, 2000.0], [1010.0, 2010.0], [1000.0, 2012.0]];
        let z = |p: &[f64; 2]| 0.02 * p[0] - 0.01 * p[1] + 7.0;
        let heights: Vec<f64> = corners.iter().map(z).collect();
        let p = base_plane(Base::Fit, &corners, &heights).expect("fits");
        assert!((p[0] - 0.02).abs() < 1e-12 && (p[1] + 0.01).abs() < 1e-12, "{p:?}");
        for c in &corners {
            assert!((p[0] * c[0] + p[1] * c[1] + p[2] - z(c)).abs() < 1e-9);
        }
        let line = [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]];
        assert_eq!(base_plane(Base::Fit, &line, &[1.0, 2.0, 3.0]), Err(BaseError::Collinear));
        assert_eq!(base_plane(Base::Lowest, &corners, &[3.0, 1.0, 2.0, 5.0]), Ok([0.0, 0.0, 1.0]));
        assert_eq!(base_plane(Base::Highest, &corners, &[3.0, 1.0, 2.0, 5.0]), Ok([0.0, 0.0, 5.0]));
        assert_eq!(base_plane(Base::Average, &corners, &[3.0, 1.0, 2.0, 6.0]), Ok([0.0, 0.0, 3.0]));
    }

    #[test]
    fn a_concave_ring_s_doubled_edges_add_nothing() {
        // An L clipped by a triangle that covers its notch: Sutherland–Hodgman keeps a bridge, the area is right.
        let l = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 1.0], [1.0, 1.0], [1.0, 4.0], [0.0, 4.0]];
        let all = keep(&l, &|_p: P| 1.0);
        assert!((moments(&all).0 - 7.0).abs() < 1e-12);
        // Keep x + y ≤ 3: the L's part under the line.
        let under = keep(&l, &|p: P| 3.0 - p[0] - p[1]);
        // Inside the L and under x + y = 3: the triangle 0..3 less the notch square's part (1..2 × 1..2 under the line).
        assert!((moments(&under).0 - (4.5 - 0.5)).abs() < 1e-12, "{}", moments(&under).0);
    }

    /// A pyramid 6 m high on a 20 × 20 m square at height 100 (five points, four triangles), over the level base at 100
    /// inside the whole square: its volume a third of base times height, 800 m³; nothing below. Over a base at 103 the
    /// top above 103 (an eighth of the volume, 100 m³) is cut and the rest of the box under 103 is fill.
    #[test]
    fn a_pyramid_s_volume_is_a_third_of_base_times_height() {
        use crate::surface::{SpotPoint, SurfaceInput, build};
        let o = (487_600.0, 4_420_300.0);
        let pt = |x: f64, y: f64, z: f64| SpotPoint { x: o.0 + x, y: o.1 + y, z, source: None };
        let input = SurfaceInput {
            points: vec![pt(0.0, 0.0, 100.0), pt(20.0, 0.0, 100.0), pt(20.0, 20.0, 100.0), pt(0.0, 20.0, 100.0), pt(10.0, 10.0, 106.0)],
            ..SurfaceInput::default()
        };
        let surface = build(&input).expect("builds");
        let square = Region {
            outer: vec![[o.0, o.1], [o.0 + 20.0, o.1], [o.0 + 20.0, o.1 + 20.0], [o.0, o.1 + 20.0]],
            holes: Vec::new(),
        };
        let m = measure(&surface, &square, [0.0, 0.0, 100.0]);
        assert!((m.area - 400.0).abs() < 1e-9 && (m.covered - 400.0).abs() < 1e-9, "{m:?}");
        assert!((m.cut - 800.0).abs() < 1e-9 && m.fill.abs() < 1e-9, "{m:?}");
        let m = measure(&surface, &square, [0.0, 0.0, 103.0]);
        assert!((m.cut - 100.0).abs() < 1e-9, "{m:?}");
        // Under 103: the box 400 × 3 less what the pyramid fills of it (800 − 100).
        assert!((m.fill - (1200.0 - 700.0)).abs() < 1e-9, "{m:?}");
        assert!((m.cut_area - 100.0).abs() < 1e-9 && (m.fill_area - 300.0).abs() < 1e-9, "{m:?}");
        // A hole at the top (the middle 10 × 10, clockwise or not) takes its share away.
        let holed = Region {
            holes: vec![vec![[o.0 + 5.0, o.1 + 5.0], [o.0 + 15.0, o.1 + 5.0], [o.0 + 15.0, o.1 + 15.0], [o.0 + 5.0, o.1 + 15.0]]],
            ..square.clone()
        };
        let m = measure(&surface, &holed, [0.0, 0.0, 103.0]);
        assert!(m.cut.abs() < 1e-9 && (m.area - 300.0).abs() < 1e-9, "{m:?}");
        // The square's corners triangulated (all at 100): the level base again, its diagonal said undecided.
        let tin = base_tin(&square.outer, &[100.0; 4]).expect("triangulates");
        assert_eq!(tin.triangles.len(), 2);
        assert_eq!(tin.undecided.len(), 1, "four corners on one circle");
        let m = measure_tin(&surface, &square, &tin);
        assert!((m.cut - 800.0).abs() < 1e-9 && m.fill.abs() < 1e-9 && (m.covered - 400.0).abs() < 1e-9, "{m:?}");
    }

    /// Corners on a tilted plane: the triangulated base is that plane, the volume the fitted plane's. A concave L's
    /// triangles cover the L and not its notch.
    #[test]
    fn a_triangulated_base_is_the_corners_planes() {
        use crate::surface::{SpotPoint, SurfaceInput, build};
        let o = (487_000.0, 4_420_000.0);
        let ground = |x: f64, y: f64| 100.0 + 0.01 * x + 0.005 * y;
        let mut points = Vec::new();
        for j in 0..=6 {
            for i in 0..=6 {
                let (x, y) = (f64::from(i) * 10.0 + if j % 2 == 1 { 3.0 } else { 0.0 }, f64::from(j) * 10.0);
                let bump = if (i, j) == (3, 3) { 4.0 } else { 0.0 };
                points.push(SpotPoint { x: o.0 + x, y: o.1 + y, z: ground(x, y) + bump, source: None });
            }
        }
        let surface = build(&SurfaceInput { points, ..SurfaceInput::default() }).expect("builds");
        let pentagon: Vec<[f64; 2]> =
            [(14.0, 16.0), (46.0, 14.0), (50.0, 34.0), (31.0, 49.0), (12.0, 36.0)].map(|(x, y)| [o.0 + x, o.1 + y]).to_vec();
        let region = Region { outer: pentagon.clone(), holes: Vec::new() };
        let heights = corner_heights(&surface, &region).expect("on the surface");
        let tin = base_tin(&pentagon, &heights).expect("triangulates");
        assert_eq!((tin.triangles.len(), tin.undecided.len()), (3, 0));
        let plane = base_plane(Base::Fit, &pentagon, &heights).expect("fits");
        let (a, b) = (measure_tin(&surface, &region, &tin), measure(&surface, &region, plane));
        assert!((a.cut - b.cut).abs() < 1e-6 && (a.fill - b.fill).abs() < 1e-6, "{a:?} {b:?}");
        assert!(a.cut > 0.0, "the bump is cut");
        // The L: four triangles, their area the L's, none in its notch.
        let ell: Vec<[f64; 2]> = [(12.0, 12.0), (48.0, 12.0), (48.0, 26.0), (26.0, 26.0), (26.0, 48.0), (12.0, 48.0)]
            .map(|(x, y)| [o.0 + x, o.1 + y])
            .to_vec();
        let tin = base_tin(&ell, &[100.0; 6]).expect("triangulates");
        let area: f64 = tin
            .triangles
            .iter()
            .map(|t| ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1])) / 2.0)
            .sum();
        assert_eq!(tin.triangles.len(), 4);
        assert!((area - (36.0 * 14.0 + 14.0 * 22.0)).abs() < 1e-6, "{area}");
        // A ring crossing itself is not triangulated.
        let bow = [[0.0, 0.0], [10.0, 10.0], [10.0, 0.0], [0.0, 10.0]];
        assert_eq!(base_tin(&bow, &[1.0; 4]), Err(BaseError::Untriangulated("constraints_cross")));
    }
}
