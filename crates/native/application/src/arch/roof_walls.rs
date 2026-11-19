//! Çatı örtüsünü aşan duvarlar (docs/adr/8165; Planlı Alanlar İmar Yönetmeliği 40/5): the walls standing in a sloping
//! roof's outline whose tops rise over its covering, by how much and what they are, for the standards package's roof
//! rule (`cati_olculeri`). 40/5 lets a stair housing (merdiven evi), a light well (ışıklık), an air shaft, an end or a
//! gable wall (alın ve kalkan duvarı) pass the covering by at most 0.60 m, an elevator tower by what the elevator
//! needs.
//!
//! - **Its top:** its foot and its height (`Document::wall_foot`, `Document::wall_height`).
//! - **The covering over a point:** the roof's eaves height and its planes' rise there (`core::height_at`).
//! - **Where:** the corners of its two faces and where a face crosses the eaves line (1 cm inside), the points within
//!   the roof's outline; the largest of top less covering. The covering is the lowest of the planes (concave), so over
//!   the wall's footprint the most is at those points.
//! - **What it is:** by the rooms beside its middle (30 cm off each face) on its storey: an elevator's (Asansör) first,
//!   then a stair's (Merdiven …), then a light well's (Işıklık, Aydınlık); none of them: another wall.
//! - A flat roof has no planes over its eaves: its parapets are 40/6's, not read here.
//!
//! The measures are the drawing's; the rules decide.

use kentos_commands::Entity;
use kentos_domain::Document;
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region::inside_area;
use kentos_geometry_core::geom::roof as core;

/// What a wall over the covering is, by the rooms beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WallOverKind {
    /// An elevator shaft's (asansör kulesi).
    Elevator,
    /// A stair's room's (merdiven evi).
    Stair,
    /// A light well's (ışıklık).
    LightWell,
    /// Another: an end or a gable wall (alın ve kalkan duvarı), a fire wall.
    Other,
}

impl WallOverKind {
    /// Its name in the audit facts.
    pub fn name(self) -> &'static str {
        match self {
            WallOverKind::Elevator => "elevator",
            WallOverKind::Stair => "stair",
            WallOverKind::LightWell => "lightWell",
            WallOverKind::Other => "other",
        }
    }
}

/// A wall rising over a roof's covering: its persistent id, what it is, by how much it passes the covering (m) and
/// where (the point it passes most, in plan).
#[derive(Clone, Debug, PartialEq)]
pub struct WallOver {
    pub uid: String,
    pub kind: WallOverKind,
    pub over: f64,
    pub at: V,
}

/// How far a wall must pass the covering to be read, m: a wall built to the covering is not over it.
const LEAST: f64 = 1e-3;

/// The walls in the outline `pts` of the roof `spec` whose tops rise over its covering, in the drawing's order; none
/// for a flat roof.
pub fn walls_over(doc: &Document, pts: &[V], spec: &core::RoofSpec) -> Vec<WallOver> {
    if !spec.slopes.iter().any(|s| s.is_some_and(|s| s > 0.0)) || pts.len() < 3 {
        return Vec::new();
    }
    let roof = Area { outer: Ring { pts: pts.to_vec(), bulges: None }, holes: Vec::new() };
    let mut out = Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        let Entity::Wall(w) = e else { continue };
        let Some(height) = doc.wall_height(&w.wall) else { continue };
        let top = doc.wall_foot(&w.wall) + height;
        if top <= spec.base + LEAST {
            continue;
        }
        let Shape::Wall { pts: path, bulges, wall } = crate::geometry::shape(e) else { continue };
        let Ok(lines) = kentos_geometry_core::geom::wall::lines(&path, bulges.as_deref(), &wall) else { continue };
        let (Some(left), Some(right)) = (lines.first(), lines.last()) else { continue };
        let mut most: Option<(f64, V)> = None;
        for face in [&left.pts, &right.pts] {
            for q in samples(face, pts) {
                if !inside_area(&roof, q) {
                    continue;
                }
                let cover = spec.base + core::height_at(pts, spec, q).unwrap_or(0.0);
                let over = top - cover;
                if most.is_none_or(|(m, _)| over > m) {
                    most = Some((over, q));
                }
            }
        }
        let Some((over, at)) = most.filter(|(o, _)| *o > LEAST) else { continue };
        out.push(WallOver { uid: uid.to_string(), kind: kind_of(doc, &w.wall, &path), over, at });
    }
    out
}

/// A face's corners and where its stretches cross the closed outline `ring`, 1 cm on either side along the stretch.
fn samples(face: &[V], ring: &[V]) -> Vec<V> {
    let mut out: Vec<V> = face.to_vec();
    let n = ring.len();
    for s in face.windows(2) {
        let (a, b) = (s[0], s[1]);
        let len = (b.x - a.x).hypot(b.y - a.y);
        if len <= 0.0 {
            continue;
        }
        for i in 0..n {
            let (c, d) = (ring[i], ring[(i + 1) % n]);
            let (ex, ey) = (b.x - a.x, b.y - a.y);
            let (fx, fy) = (d.x - c.x, d.y - c.y);
            let den = ex * fy - ey * fx;
            if den.abs() < 1e-12 {
                continue;
            }
            let (wx, wy) = (c.x - a.x, c.y - a.y);
            let t = (wx * fy - wy * fx) / den;
            let u = (wx * ey - wy * ex) / den;
            if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                for k in [-0.01, 0.01] {
                    let t = (t + k / len).clamp(0.0, 1.0);
                    out.push(V::new(a.x + ex * t, a.y + ey * t));
                }
            }
        }
    }
    out
}

/// What a wall is by the rooms beside its path's middle on its storey, 30 cm off each face.
fn kind_of(doc: &Document, w: &kentos_commands::WallFields, path: &[V]) -> WallOverKind {
    use kentos_domain::room_names::{has, stair_name, words};
    let Some((mid, dir)) = middle(path) else { return WallOverKind::Other };
    let off = w.width / 2.0 + 0.3;
    let normal = V::new(-dir.y, dir.x);
    let level = w.arch.level.as_deref();
    let mut kinds = Vec::new();
    for side in [off, -off] {
        let q = V::new(mid.x + normal.x * side, mid.y + normal.y * side);
        for e in doc.entities() {
            let Entity::Space(s) = e else { continue };
            let x = &s.space;
            if x.arch.level.as_deref() != level {
                continue;
            }
            if !kentos_domain::arch_values::core_areas(&x.parts).iter().any(|a| inside_area(a, q)) {
                continue;
            }
            let (name, usage) = (x.name.as_deref(), x.usage.as_deref());
            let ws = words(name, usage);
            kinds.push(if has(&ws, "ASANSOR") {
                WallOverKind::Elevator
            } else if stair_name(name, usage) {
                WallOverKind::Stair
            } else if has(&ws, "ISIKLIK") || has(&ws, "AYDINLIK") {
                WallOverKind::LightWell
            } else {
                WallOverKind::Other
            });
        }
    }
    [WallOverKind::Elevator, WallOverKind::Stair, WallOverKind::LightWell]
        .into_iter()
        .find(|k| kinds.contains(k))
        .unwrap_or(WallOverKind::Other)
}

/// The middle of a path by its length and the unit direction of the stretch it is on.
fn middle(path: &[V]) -> Option<(V, V)> {
    let total: f64 = path.windows(2).map(|s| (s[1].x - s[0].x).hypot(s[1].y - s[0].y)).sum();
    let mut left = total / 2.0;
    for s in path.windows(2) {
        let (a, b) = (s[0], s[1]);
        let len = (b.x - a.x).hypot(b.y - a.y);
        if len <= 0.0 {
            continue;
        }
        if left <= len {
            let k = left / len;
            let at = V::new(a.x + (b.x - a.x) * k, a.y + (b.y - a.y) * k);
            return Some((at, V::new((b.x - a.x) / len, (b.y - a.y) / len)));
        }
        left -= len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_face_crossing_the_eaves_is_read_just_inside() {
        let ring = [V::new(0.0, 0.0), V::new(10.0, 0.0), V::new(10.0, 8.0), V::new(0.0, 8.0)];
        // A face from outside the roof (x −2) to x 4 along y 3: its corners and 1 cm either side of x 0.
        let pts = samples(&[V::new(-2.0, 3.0), V::new(4.0, 3.0)], &ring);
        assert_eq!(pts.len(), 4, "{pts:?}");
        assert!(pts.iter().any(|p| (p.x - 0.01).abs() < 1e-9 && (p.y - 3.0).abs() < 1e-9), "{pts:?}");
        assert!(pts.iter().any(|p| (p.x + 0.01).abs() < 1e-9), "{pts:?}");
    }

    #[test]
    fn the_middle_of_a_path() {
        let (m, d) = middle(&[V::new(0.0, 0.0), V::new(4.0, 0.0), V::new(4.0, 2.0)]).expect("a middle");
        assert!((m.x - 3.0).abs() < 1e-12 && m.y.abs() < 1e-12, "{m:?}");
        assert!((d.x - 1.0).abs() < 1e-12 && d.y.abs() < 1e-12, "{d:?}");
    }
}
