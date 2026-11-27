//! The site's natural ground in a section (docs/adr/8174; EK-4 4.30; PAİY 57/2-a): the section building on a slope
//! whose survey points make a terrain surface, named the site's natural ground with ±0.00's elevation. The section draws
//! the surface's section along its line, dashed, its heights over ±0.00, and lists no missing natural ground; without
//! the site's ground it draws the graded ground flat at ±0.00 and lists the natural ground as missing, as before.

use kentos_commands::{DocumentSnapshotV2, Entity, EntityId};
use kentos_domain::Document;
use kentos_native_application::sections::{self, Left};
use serde_json::json;

/// ±0.00's absolute elevation.
const DATUM: f64 = 1000.5;

/// The slope: its height at plan point (x, y), absolute.
fn slope(x: f64, _y: f64) -> f64 {
    1000.0 + 0.1 * x
}

/// The section building, with a grid of survey points on a slope over and round it; with `ground`, a terrain surface
/// of them named the site's natural ground, ±0.00 at [`DATUM`].
fn building(ground: bool) -> Document {
    let mut snap: DocumentSnapshotV2 =
        kentos_kcad::decode(include_bytes!("../../../../../fixtures/arch/v1/section-building.kcad")).unwrap();
    let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
    for e in &snap.entities {
        if let Entity::Wall(w) = e {
            for p in &w.wall.pts {
                lo = [lo[0].min(p.x), lo[1].min(p.y)];
                hi = [hi[0].max(p.x), hi[1].max(p.y)];
            }
        }
    }
    let layer = snap.active_layer.clone();
    let mut next = snap.entities.iter().map(|e| e.base().id).max().unwrap_or(0) + 1;
    let mut uids = Vec::new();
    let mut x = lo[0] - 10.0;
    while x <= hi[0] + 10.0 {
        let mut y = lo[1] - 10.0;
        while y <= hi[1] + 10.0 {
            let e: Entity = serde_json::from_value(json!({"kind": "point", "id": next, "layerId": layer, "attrs": {},
                "p": {"x": x, "y": y}, "z": slope(x, y)}))
            .unwrap();
            let uid = format!("0192f5a1-7242-7000-8000-{next:012x}");
            snap.entities.push(e);
            snap.uids.push(EntityId::parse(&uid).unwrap());
            uids.push(uid);
            next += 1;
            y += 4.0;
        }
        x += 4.0;
    }
    if ground {
        snap.settings.road = Some(
            serde_json::from_value(json!({"surfaces": [{"id": "tabii", "name": "Tabii zemin", "points": {"uids": uids}}]}))
                .unwrap(),
        );
        let arch = snap.settings.arch.get_or_insert_with(Default::default);
        let site = arch.site.get_or_insert_with(Default::default);
        site.datum = Some(DATUM);
        site.natural_ground = Some("tabii".into());
    }
    Document::from_snapshot_v2(snap).unwrap()
}

/// The section B-B's drawing.
fn b_b(doc: &Document) -> sections::Drawn {
    let mark = doc
        .entities()
        .find_map(|e| match e {
            Entity::ViewMark(v) if v.mark.mark == "B" => Some(v.mark.clone()),
            _ => None,
        })
        .expect("B-B");
    sections::draw_mark(doc, &mark).expect("drawn")
}

#[test]
fn the_natural_ground_is_drawn_along_the_line_over_plus_minus_zero() {
    let doc = building(true);
    let drawn = b_b(&doc);
    assert!(!drawn.left.iter().any(|(why, _)| *why == Left::NaturalGround), "{:?}", drawn.left);
    let mark = doc
        .entities()
        .find_map(|e| match e {
            Entity::ViewMark(v) if v.mark.mark == "B" => Some(v.mark.clone()),
            _ => None,
        })
        .unwrap();
    let frame = sections::frame_of(&mark).unwrap();
    let lines: Vec<&kentos_commands::PathEntity> = drawn
        .entities
        .iter()
        .filter(|e| e.base().layer_id == "MM-CVRE-TABII")
        .filter_map(|e| match e {
            Entity::Polyline(p) => Some(p),
            _ => None,
        })
        .collect();
    assert!(!lines.is_empty(), "the natural ground is drawn");
    for l in &lines {
        assert_eq!(serde_json::to_value(l.base.look.line_type).unwrap(), json!("dashed"));
        // Each place's height is the slope's there, over ±0.00.
        for p in &l.pts {
            let at = frame.at(p.x, 0.0);
            let want = slope(at.x, at.y) - DATUM;
            assert!((p.y - want).abs() < 1e-6, "u {}: {} against {want}", p.x, p.y);
        }
    }
    assert!(drawn.layers.contains("MM-CVRE-TABII"));
}

#[test]
fn without_the_site_s_ground_the_graded_ground_is_flat_and_the_natural_ground_left_out() {
    let drawn = b_b(&building(false));
    assert!(drawn.left.iter().any(|(why, _)| *why == Left::NaturalGround), "{:?}", drawn.left);
    assert!(!drawn.layers.contains("MM-CVRE-TABII"));
    let flat = drawn.entities.iter().filter(|e| e.base().layer_id == "MM-CVRE-TESVIYE").all(|e| match e {
        Entity::Line(l) => l.a.y.abs() < 1e-12 && l.b.y.abs() < 1e-12,
        _ => false,
    });
    assert!(flat, "the graded ground at ±0.00");
}
