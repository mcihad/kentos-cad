//! Each map's own survey quality (plan/map-cad.md MAP-18, docs/adr/2800): the accepted GNSS points a map's frame shows
//! on the ground, in words, for the texts that read `@konum_dogrulugu`, `@yukseklik_dogrulugu`, `@ag_dengelemesi` and
//! `@olcme_kalitesi` (docs/adr/2400). A point is where its object is drawn in the project's system (the archive keeps
//! the object's persistent id); a point whose object is not in the drawing is on no map. Inside is the content's four
//! corners on the ground (`MapPrim::ground_corners`: a turned view turns the frame on the ground), its edges included,
//! decided with the exact orientation predicate. The words are `kentos_project::survey_facts`'.

use kentos_contracts::{Entity, SurveyPoint};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::predicates::orient2d;
use kentos_project::survey_facts::survey_facts_where;
use kentos_sheet::display::{DisplayList, MapSurveyInput, Prim};

/// Where an accepted point's object is drawn (east, north); none when the drawing has no point object of its id.
fn place(doc: &Document, p: &SurveyPoint) -> Option<Vec2> {
    let slot = doc.slot_of(Uuid::from_bytes(p.uid.0))?;
    match doc.get(slot)? {
        Entity::Point(e) => Some(Vec2::new(e.p.x, e.p.y)),
        _ => None,
    }
}

/// Whether `q` is inside the four corners (in order, either way round) or on an edge.
pub(crate) fn inside(corners: &[[f64; 2]; 4], q: Vec2) -> bool {
    let mut side = 0.0_f64;
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let turn = orient2d(Vec2::new(a[0], a[1]), Vec2::new(b[0], b[1]), q);
        if turn.is_nan() {
            return false;
        }
        if turn != 0.0 {
            if side != 0.0 && turn.signum() != side {
                return false;
            }
            side = turn.signum();
        }
    }
    true
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
        let Some(corners) = m.ground_corners() else { continue };
        if out.iter().any(|s| s.item == m.item) {
            continue;
        }
        let f = survey_facts_where(settings, |point| place(doc, point).is_some_and(|q| inside(&corners, q)));
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
}
