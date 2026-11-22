//! Altlık model (docs/adr/8101; plan/ifc-uyum.md IFC-09, plan/bim-plan.md B18): another discipline's IFC model placed
//! read-only in a KentOS project, and its plan at a level.
//!
//! - **Placement** ([`place`]): by the file's map conversion onto the project's system — the file's local metres turned,
//!   scaled and shifted onto the map (IFC 4.3's `IfcMapConversion`, `IfcMapConversionScaled`), then into the project's
//!   drawing coordinates by the inverse of the project's own conversion (the one its IFC files are written with,
//!   [`crate::crs`]). Two systems that differ, a file with no conversion, a project with no system, height datums that
//!   differ or a scale the file's length unit does not explain are each a refusal with its reason: nothing is
//!   reprojected or guessed (CLAUDE.md §5). Placing at the origin — the file's metres as the drawing's — is the user's
//!   explicit choice, never a fallback.
//! - **Plan** ([`plan`]): the bodies cut by the level's cut plane (each triangle crossing it gives a segment) and, under
//!   it down to the level's floor, the edges where a body's faces turn (or end), seen from above. No hidden line is
//!   removed: an underlay is drawn faint, the drawing over it. A mesh is a view of the body (CLAUDE.md §23.3): these
//!   lines are a picture, never a measure.

use crate::crs::Crs;
use crate::math::hypot;

/// How the file is placed in the project.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    /// By its map conversion onto the project's system.
    #[default]
    Map,
    /// Its metres as the drawing's: its origin on the project's, its axes the project's, its zero the project's ±0.00.
    Origin,
}

/// Where a file's or a project's model stands in a projected system: the system's name, the height datum, the model
/// origin's eastings, northings and height, the model's x axis (east, north), and how many of the system's metres a
/// model metre is along x, y and up.
#[derive(Clone, Debug, PartialEq)]
pub struct MapFrame {
    pub crs: String,
    pub vertical: Option<String>,
    pub origin: [f64; 3],
    pub x_axis: [f64; 2],
    pub factors: [f64; 3],
}

impl From<&Crs> for MapFrame {
    /// A system as KentOS writes it: its heights unscaled.
    fn from(c: &Crs) -> Self {
        MapFrame {
            crs: c.name.clone(),
            vertical: c.vertical_datum.clone(),
            origin: c.origin,
            x_axis: c.x_axis,
            factors: [c.factors[0], c.factors[1], 1.0],
        }
    }
}

/// A file's metres into the drawing's: x′ = m·x + t on the plane, z′ = z_scale·z + z_shift.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub m: [[f64; 2]; 2],
    pub t: [f64; 2],
    pub z_scale: f64,
    pub z_shift: f64,
}

impl Transform {
    pub const IDENTITY: Transform = Transform { m: [[1.0, 0.0], [0.0, 1.0]], t: [0.0, 0.0], z_scale: 1.0, z_shift: 0.0 };

    pub fn apply(&self, [x, y, z]: [f64; 3]) -> [f64; 3] {
        [
            self.m[0][0] * x + self.m[0][1] * y + self.t[0],
            self.m[1][0] * x + self.m[1][1] * y + self.t[1],
            self.z_scale * z + self.z_shift,
        ]
    }
}

/// How far a map conversion's scale may stand from 1 once the file's length unit is taken out: a projection's grid
/// scale and a local system's are within a few in ten thousand; a scale past this is a unit the conversion does not
/// explain.
pub const SCALE_TOLERANCE: f64 = 0.02;

/// The unit axis of `[e, n]`; none when it has no length.
fn axis([e, n]: [f64; 2]) -> Option<[f64; 2]> {
    let r = hypot(e, n);
    (r > 0.0 && r.is_finite()).then(|| [e / r, n / r])
}

/// Where the file in `file` stands in the project's drawing: `file` its map conversion (none: the file has none),
/// `unit` the metres of the file's length unit (its bodies are read in metres), `project` the project's system as its
/// IFC files say it (or why it has none), `asked` how the user asked it placed. Refused with the reason, in the
/// interface's words, when the conversion cannot place it.
pub fn place(file: Option<&MapFrame>, unit: f64, project: Result<&MapFrame, &str>, asked: Placement) -> Result<Transform, String> {
    if asked == Placement::Origin {
        return Ok(Transform::IDENTITY);
    }
    let Some(f) = file else {
        return Err("Dosyada harita dönüşümü (IfcMapConversion) yok: nerede durduğu bilinmiyor. Başlangıç noktasına \
                    yerleştirmeyi seçin ya da dosyayı harita dönüşümüyle yeniden dışa aktarın."
            .to_owned());
    };
    let p = match project {
        Ok(p) => p,
        Err(why) => {
            return Err(format!(
                "Projenin koordinat sistemi IFC'ye göre söylenemiyor ({why}); dosya {} sistemine göre konumlanmış, \
                 yerleştirilmedi. Başlangıç noktasına yerleştirmeyi seçin ya da projeye sistem verin.",
                f.crs
            ));
        }
    };
    if f.crs != p.crs {
        return Err(format!(
            "Dosyanın koordinat sistemi {}, projeninki {}: yeniden yansıtma yapılmaz, dosya yerleştirilmedi. Dosyayı \
             projenin sisteminde yeniden dışa aktarın.",
            f.crs, p.crs
        ));
    }
    if let (Some(a), Some(b)) = (&f.vertical, &p.vertical)
        && a != b
    {
        return Err(format!(
            "Dosyanın yükseklik datumu “{a}”, projeninki “{b}”: kotlar birbirine çevrilmez, dosya yerleştirilmedi."
        ));
    }
    if !(unit > 0.0 && unit.is_finite()) {
        return Err("Dosyanın uzunluk birimi okunamadı: ölçeği bilinmiyor, yerleştirilmedi.".to_owned());
    }
    // The file's metres in the system's: its conversion's factors over its unit.
    let k = [f.factors[0] / unit, f.factors[1] / unit, f.factors[2] / unit];
    if k.iter().any(|s| !s.is_finite() || (s - 1.0).abs() > SCALE_TOLERANCE) {
        return Err(format!(
            "Harita dönüşümünün ölçeği ({:.6}) dosyanın uzunluk biriminin ({unit} m) karşılığı değil: dönüşüm yanlış \
             yazılmış olabilir, yerleştirilmedi. Başlangıç noktasına yerleştirmeyi seçin ya da dosyayı yeniden dışa \
             aktarın.",
            k[0] * unit
        ));
    }
    let (Some([fc, fs]), Some([pc, ps])) = (axis(f.x_axis), axis(p.x_axis)) else {
        return Err("Harita dönüşümünün x ekseni uzunluksuz: dönüş açısı okunamadı, yerleştirilmedi.".to_owned());
    };
    let (px, py) = (p.factors[0], p.factors[1]);
    if !(px > 0.0 && py > 0.0) {
        return Err("Projenin dönüşümünün ölçeği sıfır: yerleştirilmedi.".to_owned());
    }
    // map = Of + Rf·diag(kx, ky)·x;  drawing = diag(1/px, 1/py)·Rp⁻¹·(map − Op).
    let rf = [[fc, -fs], [fs, fc]];
    let rp_inv = [[pc, ps], [-ps, pc]];
    let mul = |a: [[f64; 2]; 2], b: [[f64; 2]; 2]| {
        [
            [a[0][0] * b[0][0] + a[0][1] * b[1][0], a[0][0] * b[0][1] + a[0][1] * b[1][1]],
            [a[1][0] * b[0][0] + a[1][1] * b[1][0], a[1][0] * b[0][1] + a[1][1] * b[1][1]],
        ]
    };
    let back = mul([[1.0 / px, 0.0], [0.0, 1.0 / py]], rp_inv);
    let m = mul(mul(back, rf), [[k[0], 0.0], [0.0, k[1]]]);
    let d = [f.origin[0] - p.origin[0], f.origin[1] - p.origin[1]];
    let t = [back[0][0] * d[0] + back[0][1] * d[1], back[1][0] * d[0] + back[1][1] * d[1]];
    Ok(Transform { m, t, z_scale: k[2], z_shift: f.origin[2] - p.origin[2] })
}

/// One body to plan: its owner (the object's index in the file read), its corners in the drawing's metres, its
/// triangles.
#[derive(Clone, Copy, Debug)]
pub struct Mesh<'a> {
    pub owner: u32,
    pub positions: &'a [[f64; 3]],
    pub triangles: &'a [[u32; 3]],
}

/// A line of the plan: its ends on the plane, its owner, and whether the cut plane made it (else an edge under it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanLine {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub owner: u32,
    pub cut: bool,
}

/// Two faces meeting at a sharper angle than this (the cosine of 20° between their normals) make an edge the plan
/// shows; flatter ones are one surface broken into triangles.
const CREASE: f64 = 0.94;
/// The grid corners are matched on to find which triangles share an edge, metres: a picture's join, not a measure.
const JOIN: f64 = 1e-5;

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn unit3(v: [f64; 3]) -> Option<[f64; 3]> {
    let r = hypot(hypot(v[0], v[1]), v[2]);
    (r > 0.0 && r.is_finite()).then(|| [v[0] / r, v[1] / r, v[2] / r])
}

/// A corner on the join grid. Display only: a corner within a hundredth of a millimetre of another is the same corner.
#[allow(clippy::cast_possible_truncation)]
fn key(p: [f64; 3]) -> [i64; 3] {
    p.map(|v| (v / JOIN).round() as i64)
}

/// Where segment a–b meets height `z`, `a` and `b` on its two sides.
fn at_height(a: [f64; 3], b: [f64; 3], z: f64) -> [f64; 2] {
    let t = (z - a[2]) / (b[2] - a[2]);
    [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]
}

/// The plan of `meshes` at a level: cut at height `cut`, the edges under it down to `bottom` (both in the drawing's
/// heights). A corner exactly at the cut counts under it.
pub fn plan(meshes: &[Mesh<'_>], cut: f64, bottom: f64) -> Vec<PlanLine> {
    let mut out = Vec::new();
    for mesh in meshes {
        let at = |i: u32| mesh.positions.get(i as usize).copied();
        // The cut: each triangle crossing the plane gives the segment between its two crossing edges.
        for t in mesh.triangles {
            let (Some(p0), Some(p1), Some(p2)) = (at(t[0]), at(t[1]), at(t[2])) else {
                continue;
            };
            let ps = [p0, p1, p2];
            let above = ps.map(|p| p[2] > cut);
            let ups = above.iter().filter(|a| **a).count();
            if ups == 0 || ups == 3 {
                continue;
            }
            let mut ends = Vec::with_capacity(2);
            for (i, j) in [(0, 1), (1, 2), (2, 0)] {
                if above[i] != above[j] {
                    ends.push(at_height(ps[i], ps[j], cut));
                }
            }
            if let [a, b] = ends[..]
                && (a[0] - b[0]).abs() + (a[1] - b[1]).abs() > 0.0
            {
                out.push(PlanLine { a, b, owner: mesh.owner, cut: true });
            }
        }
        // Under the cut: every edge with its triangles' normals, by its corners on the join grid.
        /// An edge's count, its first triangle's normal, whether it is cut, and its two corners.
        type Edge = (usize, Option<[f64; 3]>, bool, [f64; 3], [f64; 3]);
        let mut edges: std::collections::HashMap<([i64; 3], [i64; 3]), Edge> =
            std::collections::HashMap::new();
        for t in mesh.triangles {
            let (Some(p0), Some(p1), Some(p2)) = (at(t[0]), at(t[1]), at(t[2])) else {
                continue;
            };
            let Some(n) = unit3(cross(sub(p1, p0), sub(p2, p0))) else {
                continue;
            };
            for (a, b) in [(p0, p1), (p1, p2), (p2, p0)] {
                let (ka, kb) = (key(a), key(b));
                if ka == kb {
                    continue;
                }
                let (k, ends) = if ka < kb { ((ka, kb), (a, b)) } else { ((kb, ka), (b, a)) };
                let e = edges.entry(k).or_insert((0, None, false, ends.0, ends.1));
                e.0 += 1;
                match e.1 {
                    None => e.1 = Some(n),
                    Some(m) if (m[0] * n[0] + m[1] * n[1] + m[2] * n[2]).abs() < CREASE => e.2 = true,
                    // Two faces folded back on each other (a sliver) are a crease too.
                    Some(m) if m[0] * n[0] + m[1] * n[1] + m[2] * n[2] < 0.0 => e.2 = true,
                    Some(_) => {}
                }
            }
        }
        let mut shown: Vec<([f64; 3], [f64; 3])> = edges
            .into_values()
            .filter(|(count, _, crease, _, _)| *count != 2 || *crease)
            .map(|(_, _, _, a, b)| (a, b))
            .collect();
        // The same order whatever the hash map's.
        shown.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal).then(
            x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal),
        ));
        for (mut a, mut b) in shown {
            if a[2] > b[2] {
                std::mem::swap(&mut a, &mut b);
            }
            // Low a, high b: the part between the bottom and the cut.
            if a[2] > cut || b[2] < bottom {
                continue;
            }
            let lo = if a[2] < bottom { at_height(a, b, bottom) } else { [a[0], a[1]] };
            let hi = if b[2] > cut { at_height(a, b, cut) } else { [b[0], b[1]] };
            // An upright edge seen from above is a point.
            if (lo[0] - hi[0]).abs() + (lo[1] - hi[1]).abs() <= JOIN {
                continue;
            }
            out.push(PlanLine { a: lo, b: hi, owner: mesh.owner, cut: false });
        }
    }
    out
}

/// The project's levels and the file's storeys paired by elevation (metres, the drawing's heights): the nearest pair
/// first, each taken once. Each pair, then each level no storey took, then each storey no level took: (level, storey).
pub fn pair_levels(levels: &[f64], storeys: &[f64]) -> Vec<(Option<usize>, Option<usize>)> {
    let mut pairs: Vec<(f64, usize, usize)> = levels
        .iter()
        .enumerate()
        .flat_map(|(i, l)| storeys.iter().enumerate().map(move |(j, s)| ((s - l).abs(), i, j)))
        .collect();
    pairs.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
    let (mut level_taken, mut storey_taken) = (vec![false; levels.len()], vec![false; storeys.len()]);
    let mut out = Vec::new();
    for (_, i, j) in pairs {
        if !level_taken[i] && !storey_taken[j] {
            level_taken[i] = true;
            storey_taken[j] = true;
            out.push((Some(i), Some(j)));
        }
    }
    out.sort_by_key(|p| p.0);
    out.extend(level_taken.iter().enumerate().filter(|(_, t)| !**t).map(|(i, _)| (Some(i), None)));
    out.extend(storey_taken.iter().enumerate().filter(|(_, t)| !**t).map(|(j, _)| (None, Some(j))));
    out
}

/// How far the file's axis `b` stands from the project's axis `a`, both as two points on the plane: the distance of
/// `b`'s middle from `a`'s line (metres) and the turn between them (degrees, 0 to 90).
pub fn axis_offset(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> (f64, f64) {
    let da = [a[1][0] - a[0][0], a[1][1] - a[0][1]];
    let db = [b[1][0] - b[0][0], b[1][1] - b[0][1]];
    let (la, lb) = (hypot(da[0], da[1]), hypot(db[0], db[1]));
    if la == 0.0 || lb == 0.0 {
        return (f64::NAN, f64::NAN);
    }
    let mid = [(b[0][0] + b[1][0]) / 2.0, (b[0][1] + b[1][1]) / 2.0];
    let offset = ((mid[0] - a[0][0]) * da[1] - (mid[1] - a[0][1]) * da[0]).abs() / la;
    let sin = ((da[0] * db[1] - da[1] * db[0]) / (la * lb)).abs().min(1.0);
    (offset, crate::math::atan(sin / (1.0 - sin * sin).max(0.0).sqrt().max(f64::MIN_POSITIVE)) * (180.0 / std::f64::consts::PI))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{cos, sin};

    /// A box from (x0, y0, z0) to (x1, y1, z1): eight shared corners, twelve triangles.
    fn boxed(lo: [f64; 3], hi: [f64; 3]) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let p = |i: u32| {
            [
                if i & 1 == 0 { lo[0] } else { hi[0] },
                if i & 2 == 0 { lo[1] } else { hi[1] },
                if i & 4 == 0 { lo[2] } else { hi[2] },
            ]
        };
        let positions = (0..8).map(p).collect();
        let quads = [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]];
        let triangles = quads.iter().flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect();
        (positions, triangles)
    }

    /// The same box with each face's own corners (flat shading): 24 corners.
    fn flat((positions, triangles): (Vec<[f64; 3]>, Vec<[u32; 3]>)) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let mut out = (Vec::new(), Vec::new());
        for t in triangles {
            let base = out.0.len() as u32;
            out.0.extend(t.map(|i| positions[i as usize]));
            out.1.push([base, base + 1, base + 2]);
        }
        out
    }

    fn length(lines: &[PlanLine], cut: bool) -> f64 {
        lines.iter().filter(|l| l.cut == cut).map(|l| hypot(l.b[0] - l.a[0], l.b[1] - l.a[1])).sum()
    }

    fn on_square(p: [f64; 2], lo: f64, hi: f64) -> bool {
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        let within = |v: f64| v > lo - 1e-9 && v < hi + 1e-9;
        ((near(p[0], lo) || near(p[0], hi)) && within(p[1])) || ((near(p[1], lo) || near(p[1], hi)) && within(p[0]))
    }

    #[test]
    fn a_box_is_cut_along_its_sides_and_its_floor_edges_seen_under_the_cut() {
        for (positions, triangles) in [boxed([2.0, 2.0, 0.0], [5.0, 5.0, 3.0]), flat(boxed([2.0, 2.0, 0.0], [5.0, 5.0, 3.0]))] {
            let mesh = Mesh { owner: 7, positions: &positions, triangles: &triangles };
            let lines = plan(&[mesh], 1.2, 0.0);
            assert!((length(&lines, true) - 12.0).abs() < 1e-9, "the cut goes round the 3 × 3 box");
            assert!((length(&lines, false) - 12.0).abs() < 1e-9, "the floor's four edges, the uprights points");
            assert!(lines.iter().all(|l| l.owner == 7 && on_square(l.a, 2.0, 5.0) && on_square(l.b, 2.0, 5.0)));
        }
        // Over the box, nothing is cut and its top edges are under the cut: seen whole.
        let (positions, triangles) = boxed([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let lines = plan(&[Mesh { owner: 0, positions: &positions, triangles: &triangles }], 1.2, 0.0);
        assert_eq!(length(&lines, true), 0.0);
        assert!((length(&lines, false) - 8.0).abs() < 1e-9, "floor and top edges, both 4 m");
        // A box below the level's floor is not this level's.
        let (positions, triangles) = boxed([0.0, 0.0, -4.0], [1.0, 1.0, -3.0]);
        assert!(plan(&[Mesh { owner: 0, positions: &positions, triangles: &triangles }], 1.2, 0.0).is_empty());
    }

    #[test]
    fn a_slanted_edge_is_clipped_to_the_level_and_a_corner_on_the_cut_counts_under_it() {
        // A wedge: its sloping top from 0 m to 2 m over 4 m; the cut at 1 m takes the slope at x = 2.
        let positions = [[0.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 0.0, 2.0], [0.0, 1.0, 0.0], [4.0, 1.0, 0.0], [4.0, 1.0, 2.0]];
        let triangles = [[0, 1, 2], [3, 5, 4], [0, 3, 4], [0, 4, 1], [1, 4, 5], [1, 5, 2], [0, 2, 5], [0, 5, 3]];
        let lines = plan(&[Mesh { owner: 1, positions: &positions, triangles: &triangles }], 1.0, 0.0);
        let cut: Vec<_> = lines.iter().filter(|l| l.cut).collect();
        assert!(!cut.is_empty() && cut.iter().all(|l| (l.a[0] - 2.0).abs() < 1e-9 || (l.a[0] - 4.0).abs() < 1e-9));
        // The two sloping edges are seen from x = 0 to x = 2 only, where they reach the cut.
        let slopes = lines
            .iter()
            .filter(|l| !l.cut && (l.a[1] - l.b[1]).abs() < 1e-12 && l.a[0].min(l.b[0]) < 1e-9)
            .filter(|l| (l.a[0].max(l.b[0]) - 2.0).abs() < 1e-9)
            .count();
        assert_eq!(slopes, 2, "{lines:?}");
    }

    fn frame(crs: &str, origin: [f64; 3], turn: f64, scale: f64) -> MapFrame {
        let th = turn * (std::f64::consts::PI / 180.0);
        MapFrame { crs: crs.into(), vertical: None, origin, x_axis: [cos(th), sin(th)], factors: [scale, scale, 1.0] }
    }

    fn near(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-7)
    }

    #[test]
    fn a_file_on_the_project_s_own_conversion_lands_where_it_was_drawn() {
        let p = frame("EPSG:5256", [587_312.345, 4_400_612.789, 1285.4], 23.5, 0.99989);
        let t = place(Some(&p), 1.0, Ok(&p), Placement::Map).expect("placed");
        for q in [[0.0, 0.0, 0.0], [60.0, 5.0, 3.2], [-10.0, 45.0, -2.5]] {
            assert!(near(t.apply(q), q), "{q:?} → {:?}", t.apply(q));
        }
    }

    #[test]
    fn a_file_on_the_map_comes_into_a_local_project_through_the_project_s_conversion() {
        // The file's metres are the map's (no turn, no shift), its zero 1285.4 m high; the project's local plane is
        // turned 23.5°, scaled 0.99989 and its zero 1288.2 m high.
        let file = frame("EPSG:5256", [0.0, 0.0, 1285.4], 0.0, 1.0);
        let project = frame("EPSG:5256", [587_312.345, 4_400_612.789, 1288.2], 23.5, 0.99989);
        let t = place(Some(&file), 1.0, Ok(&project), Placement::Map).expect("placed");
        // A drawing point of the project on the map, by the project's conversion, comes back to itself.
        let th = 23.5 * (std::f64::consts::PI / 180.0);
        for [x, y] in [[0.0, 0.0], [60.0, 5.0], [10.0, 45.0]] {
            let (fx, fy) = (0.99989 * x, 0.99989 * y);
            let e = 587_312.345 + cos(th) * fx - sin(th) * fy;
            let n = 4_400_612.789 + sin(th) * fx + cos(th) * fy;
            let got = t.apply([e, n, 3.0]);
            assert!(near(got, [x, y, 3.0 + 1285.4 - 1288.2]), "{got:?}");
        }
        // A millimetre file says its scale 0.001: the same place; one that says 1 is refused.
        let mut mm = file.clone();
        mm.factors = [0.001, 0.001, 0.001];
        let m = place(Some(&mm), 0.001, Ok(&project), Placement::Map).expect("a millimetre file");
        assert!(near(m.apply([587_312.345, 4_400_612.789, 0.0]), t.apply([587_312.345, 4_400_612.789, 0.0])));
        let why = place(Some(&file), 0.001, Ok(&project), Placement::Map).expect_err("a scale its unit does not explain");
        assert!(why.contains("uzunluk biriminin"), "{why}");
    }

    #[test]
    fn levels_pair_by_the_nearest_elevation_and_axes_say_their_offset_and_turn() {
        // Zemin 0, 1. kat 3.00; the file's storeys 0 and 3.15, and a roof storey 6.00 the project has no level for.
        assert_eq!(
            pair_levels(&[0.0, 3.0], &[3.15, 0.0, 6.0]),
            vec![(Some(0), Some(1)), (Some(1), Some(0)), (None, Some(2))]
        );
        assert_eq!(pair_levels(&[0.0, 3.0, 6.0], &[0.0]), vec![(Some(0), Some(0)), (Some(1), None), (Some(2), None)]);
        // Axis A on x = 0; the file's on x = 0.02, turned 1° about its middle.
        let (off, turn) = axis_offset([[0.0, -2.0], [0.0, 11.0]], [[0.02, -2.0], [0.02, 11.0]]);
        assert!((off - 0.02).abs() < 1e-12 && turn.abs() < 1e-9, "{off} {turn}");
        let th = 1.0 * (std::f64::consts::PI / 180.0);
        let (s, c) = (sin(th), cos(th));
        let (off, turn) = axis_offset([[0.0, -2.0], [0.0, 11.0]], [[-6.5 * s, 4.5 - 6.5 * c], [6.5 * s, 4.5 + 6.5 * c]]);
        assert!(off.abs() < 1e-12 && (turn - 1.0).abs() < 1e-9, "{off} {turn}");
    }

    #[test]
    fn another_system_no_conversion_and_no_project_system_are_refused_unless_placed_at_the_origin() {
        let file = frame("EPSG:5256", [0.0, 0.0, 0.0], 0.0, 1.0);
        let project = frame("EPSG:5254", [0.0, 0.0, 0.0], 0.0, 1.0);
        let why = place(Some(&file), 1.0, Ok(&project), Placement::Map).expect_err("another system");
        assert!(why.contains("EPSG:5256") && why.contains("EPSG:5254") && why.contains("yeniden yansıtma"), "{why}");
        let why = place(None, 1.0, Ok(&project), Placement::Map).expect_err("no conversion");
        assert!(why.contains("harita dönüşümü"), "{why}");
        let why = place(Some(&file), 1.0, Err("projenin koordinat sistemi yok"), Placement::Map).expect_err("no system");
        assert!(why.contains("projenin koordinat sistemi yok"), "{why}");
        let mut other = project.clone();
        other.crs = file.crs.clone();
        other.vertical = Some("Antalya height (EPSG:5775)".into());
        let mut low = file.clone();
        low.vertical = Some("ITRF96".into());
        assert!(place(Some(&low), 1.0, Ok(&other), Placement::Map).expect_err("heights").contains("yükseklik datumu"));
        for f in [Some(&file), None] {
            assert_eq!(place(f, 1.0, Ok(&project), Placement::Origin), Ok(Transform::IDENTITY));
        }
    }
}
