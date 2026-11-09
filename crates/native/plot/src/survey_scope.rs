//! Each map's own survey quality (plan/map-cad.md MAP-18, docs/adr/2800): the accepted GNSS points a map's frame shows
//! on the ground, in words, for the texts that read `@konum_dogrulugu`, `@yukseklik_dogrulugu`, `@ag_dengelemesi` and
//! `@olcme_kalitesi` (docs/adr/2400). A point is where its object is drawn in the project's system (the archive keeps
//! the object's persistent id); a point whose object is not in the drawing is on no map. Inside is the map's clip shape
//! on the ground (docs/adr/3200): the content's four corners for the frame (`MapPrim::ground_corners`: a turned view
//! turns the frame on the ground), a rectangle's or a polygon's corners (the content frame's millionths) carried there
//! the same way, an atlas page's cut to its object too (the map's shape is the object's outline as drawn); its edges
//! included, decided with the exact orientation predicate. The words are `kentos_project::survey_facts`'.

use kentos_contracts::{Entity, SurveyPoint};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::predicates::orient2d;
use kentos_project::survey_facts::survey_facts_where;
use kentos_sheet::clip::Clip;
use kentos_sheet::display::{DisplayList, MapPrim, MapSurveyInput, Prim};
use kentos_sheet::projection::Transform;

/// Where an accepted point's object is drawn (east, north); none when the drawing has no point object of its id.
fn place(doc: &Document, p: &SurveyPoint) -> Option<Vec2> {
    let slot = doc.slot_of(Uuid::from_bytes(p.uid.0))?;
    match doc.get(slot)? {
        Entity::Point(e) => Some(Vec2::new(e.p.x, e.p.y)),
        _ => None,
    }
}

/// The ground a map shows (docs/adr/3200): its clip shape's ring, the frame's four corners or a rectangle's or a
/// polygon's corners in the content frame's millionths carried to the ground by the map's own transform (the frame's
/// turn on the paper leaves the ground as it is); none for a map without a centre.
fn shown_ring(m: &MapPrim) -> Option<Vec<[f64; 2]>> {
    let center = m.view.center?;
    let t = Transform::paper(&m.clip, center, m.view.scale, m.view.rotation);
    let f = &m.clip;
    let at = |u: i32, v: i32| {
        let paper = [
            f64::from(f.left) + f64::from(f.width) * f64::from(u) / 1e6,
            f64::from(f.top) + f64::from(f.height) * f64::from(v) / 1e6,
        ];
        t.inverse(paper)
    };
    Some(match &m.clip_shape {
        Clip::Frame => t.ground_corners(f).to_vec(),
        Clip::Rectangle { bounds: [l, top, r, b] } => vec![at(*l, *top), at(*r, *top), at(*r, *b), at(*l, *b)],
        Clip::Polygon { points } => points.iter().map(|p| at(p[0], p[1])).collect(),
    })
}

/// Whether `q` is inside the simple `ring` (either way round) or on an edge: on an edge by the exact orientation and
/// the edge's box, else the crossings of the ray to the right counted, each crossing's side decided exactly.
pub(crate) fn inside(ring: &[[f64; 2]], q: Vec2) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut odd = false;
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let turn = orient2d(Vec2::new(a[0], a[1]), Vec2::new(b[0], b[1]), q);
        if turn.is_nan() {
            return false;
        }
        if turn == 0.0 && q.x >= a[0].min(b[0]) && q.x <= a[0].max(b[0]) && q.y >= a[1].min(b[1]) && q.y <= a[1].max(b[1]) {
            return true;
        }
        if (a[1] > q.y) != (b[1] > q.y) {
            let left = if b[1] > a[1] { turn > 0.0 } else { turn < 0.0 };
            if left {
                odd = !odd;
            }
        }
    }
    odd
}

/// Each map of `list` with a place, its own survey quality; nothing when the project has no accepted point (every map
/// then reads the project's, which is empty).
pub fn map_surveys(doc: &Document, list: Option<&DisplayList>) -> Vec<MapSurveyInput> {
    let settings = doc.settings();
    if settings.map_survey.as_ref().is_none_or(|s| s.points.is_empty()) {
        return Vec::new();
    }
    let mut out: Vec<MapSurveyInput> = Vec::new();
    for p in list.into_iter().flat_map(|l| l.prims.iter()) {
        let Prim::Map(m) = p else { continue };
        let Some(ring) = shown_ring(m) else { continue };
        if out.iter().any(|s| s.item == m.item) {
            continue;
        }
        let f = survey_facts_where(settings, |point| place(doc, point).is_some_and(|q| inside(&ring, q)));
        out.push(MapSurveyInput { item: m.item.clone(), position: f.position, height: f.height, network: f.network, note: f.note });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_takes_the_edges_and_either_turn() {
        // A square turned 45° about (0, 0): a diamond of half-diagonal 10.
        let diamond = [[0.0, 10.0], [10.0, 0.0], [0.0, -10.0], [-10.0, 0.0]];
        assert!(inside(&diamond, Vec2::new(0.0, 0.0)));
        assert!(inside(&diamond, Vec2::new(5.0, 5.0)), "on an edge");
        assert!(!inside(&diamond, Vec2::new(6.0, 6.0)), "in the box, out of the diamond");
        let back = [diamond[3], diamond[2], diamond[1], diamond[0]];
        assert!(inside(&back, Vec2::new(-4.0, 3.0)) && !inside(&back, Vec2::new(-8.0, 8.0)));
        // Large coordinates: a point a micrometre outside an edge is outside.
        let big = [[487000.0, 4420010.0], [487010.0, 4420010.0], [487010.0, 4420000.0], [487000.0, 4420000.0]];
        assert!(inside(&big, Vec2::new(487010.0, 4420005.0)));
        assert!(!inside(&big, Vec2::new(487010.000001, 4420005.0)));
    }

    /// docs/adr/3200: a concave ring (an L leaving its top right quarter out), its notch's corner and edges inside.
    #[test]
    fn inside_takes_a_concave_ring() {
        let ell = [[0.0, 0.0], [10.0, 0.0], [10.0, 5.0], [5.0, 5.0], [5.0, 10.0], [0.0, 10.0]];
        assert!(inside(&ell, Vec2::new(2.0, 8.0)) && inside(&ell, Vec2::new(8.0, 2.0)));
        assert!(!inside(&ell, Vec2::new(8.0, 8.0)), "the notch");
        assert!(inside(&ell, Vec2::new(5.0, 5.0)) && inside(&ell, Vec2::new(7.0, 5.0)) && inside(&ell, Vec2::new(5.0, 7.0)));
        assert!(!inside(&ell, Vec2::new(5.000001, 7.0)));
        let back: Vec<[f64; 2]> = ell.iter().rev().copied().collect();
        assert!(!inside(&back, Vec2::new(8.0, 8.0)) && inside(&back, Vec2::new(1.0, 1.0)));
    }
}
