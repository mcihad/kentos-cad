//! The site's natural ground in a section (docs/adr/8174; EK-4 4.30; PAİY 57/2-a): the section building on a slope
//! whose survey points make a terrain surface, named the site's natural ground with ±0.00's elevation. The section draws
//! the surface's section along its line, dashed, its heights over ±0.00, and lists no missing natural ground; without
//! the site's ground it draws the graded ground flat at ±0.00 and lists the natural ground as missing, as before.

use kentos_commands::{DocumentSnapshotV2, Entity, EntityId, SiteGround};
use kentos_domain::Document;
use kentos_native_application::sections::{self, Left};
use serde_json::json;

/// ±0.00's absolute elevation.
const DATUM: f64 = 1000.5;

/// The slope: its height at plan point (x, y), absolute.
fn slope(x: f64, _y: f64) -> f64 {
    1000.0 + 0.1 * x
}

/// The section building, with a grid of survey points on a slope over and round it; with `ground`, their layer the
/// site's natural ground, ±0.00 at [`DATUM`].
fn building(ground: bool) -> Document {
    let mut snap: DocumentSnapshotV2 = kentos_kcad::decode(include_bytes!(
        "../../../../../fixtures/arch/v1/section-building.kcad"
    ))
    .unwrap();
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
    let mut x = lo[0] - 10.0;
    while x <= hi[0] + 10.0 {
        let mut y = lo[1] - 10.0;
        while y <= hi[1] + 10.0 {
            let e: Entity = serde_json::from_value(
                json!({"kind": "point", "id": next, "layerId": layer, "attrs": {},
                "p": {"x": x, "y": y}, "z": slope(x, y)}),
            )
            .unwrap();
            let uid = format!("0192f5a1-7242-7000-8000-{next:012x}");
            snap.entities.push(e);
            snap.uids.push(EntityId::parse(&uid).unwrap());
            next += 1;
            y += 4.0;
        }
        x += 4.0;
    }
    if ground {
        let arch = snap.settings.arch.get_or_insert_with(Default::default);
        let site = arch.site.get_or_insert_with(Default::default);
        site.datum = Some(DATUM);
        site.natural_ground = Some(SiteGround::of_layer(&layer));
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
    assert!(
        !drawn
            .left
            .iter()
            .any(|(why, _)| *why == Left::NaturalGround),
        "{:?}",
        drawn.left
    );
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
        assert_eq!(
            serde_json::to_value(l.base.look.line_type).unwrap(),
            json!("dashed")
        );
        // Each place's height is the slope's there, over ±0.00.
        for p in &l.pts {
            let at = frame.at(p.x, 0.0);
            let want = slope(at.x, at.y) - DATUM;
            assert!(
                (p.y - want).abs() < 1e-6,
                "u {}: {} against {want}",
                p.x,
                p.y
            );
        }
    }
    assert!(drawn.layers.contains("MM-CVRE-TABII"));
}

#[test]
fn without_the_site_s_ground_the_graded_ground_is_flat_and_the_natural_ground_left_out() {
    let drawn = b_b(&building(false));
    assert!(
        drawn
            .left
            .iter()
            .any(|(why, _)| *why == Left::NaturalGround),
        "{:?}",
        drawn.left
    );
    assert!(!drawn.layers.contains("MM-CVRE-TABII"));
    let flat = drawn
        .entities
        .iter()
        .filter(|e| e.base().layer_id == "MM-CVRE-TESVIYE")
        .all(|e| match e {
            Entity::Line(l) => l.a.y.abs() < 1e-12 && l.b.y.abs() < 1e-12,
            _ => false,
        });
    assert!(flat, "the graded ground at ±0.00");
}

/// An L-shaped building on Zemin kat and 1. kat (a smaller one), survey points on a plane over it (z = 1000 + 0.1·x +
/// 0.2·y) whose layer is the site's natural ground, no ±0.00.
fn l_building() -> Document {
    let wall = |pts: &[[f64; 2]], level: &str| {
        let pts: Vec<_> = pts.iter().map(|p| json!({"x": p[0], "y": p[1]})).collect();
        json!({"kind": "wall", "pts": pts, "width": 0.2, "height": 3.0, "justify": "center", "arch": {"level": level}})
    };
    let mut entities = vec![
        // Outer faces (0,0) (10,0) (10,4) (4,4) (4,10) (0,10): the centre line 10 cm in.
        wall(
            &[
                [0.1, 0.1],
                [9.9, 0.1],
                [9.9, 3.9],
                [3.9, 3.9],
                [3.9, 9.9],
                [0.1, 9.9],
                [0.1, 0.1],
            ],
            "zemin",
        ),
        wall(
            &[[0.1, 0.1], [3.9, 0.1], [3.9, 3.9], [0.1, 3.9], [0.1, 0.1]],
            "k1",
        ),
    ];
    for i in 0..7 {
        for j in 0..7 {
            let (x, y) = (-2.0 + 2.5 * f64::from(i), -2.0 + 2.5 * f64::from(j));
            entities.push(
                json!({"kind": "point", "p": {"x": x, "y": y}, "z": 1000.0 + 0.1 * x + 0.2 * y}),
            );
        }
    }
    let mut snap = json!({
        "format": "kentos.document", "version": 2, "name": "L", "origin": {"x": 0.0, "y": 0.0},
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 50.0, "workspace": "cad",
            "arch": {"levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
            ], "site": {"naturalGround": {"layers": ["0"]}}}},
        "layers": [{"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "0", "styles": {"items": [], "categories": []}, "uids": [], "entities": []
    });
    for (i, e) in entities.into_iter().enumerate() {
        let mut e = e;
        e["id"] = json!(i + 1);
        e["layerId"] = json!("0");
        e["attrs"] = json!({});
        let uid = format!("0192f5a1-7242-7000-8000-{:012x}", i + 1);
        snap["entities"].as_array_mut().unwrap().push(e);
        snap["uids"].as_array_mut().unwrap().push(json!(uid));
    }
    Document::from_snapshot_v2(serde_json::from_value(snap).unwrap()).unwrap()
}

#[test]
fn plus_minus_zero_is_the_mean_at_the_zemin_kat_s_corners_of_an_l() {
    use kentos_native_application::site_ground;
    let doc = l_building();
    // Its six corners, not 1. kat's four nor a vertex on a straight run.
    let mut corners: Vec<(i64, i64)> = site_ground::building_corners(&doc)
        .expect("the L's outline")
        .iter()
        .map(|p| ((p.x * 1000.0).round() as i64, (p.y * 1000.0).round() as i64))
        .collect();
    corners.sort_unstable();
    assert_eq!(
        corners,
        vec![
            (0, 0),
            (0, 10_000),
            (4000, 4000),
            (4000, 10_000),
            (10_000, 0),
            (10_000, 4000)
        ]
    );
    // (1000 + 1001 + 1001.8 + 1001.2 + 1002.4 + 1002) / 6.
    let datum = site_ground::datum_from_corners(&doc).expect("±0.00");
    assert!((datum - 1001.4).abs() < 1e-9, "{datum}");
}

/// The ground a click makes (docs/adr/8174 §3.1) is the site's own, so that an architecture project is saved: it keeps
/// no road settings, and the encoder refused a site ground among them ("yol ayarları yalnız yol projesinin olur").
#[test]
fn a_ground_made_is_the_site_s_and_the_architecture_project_is_saved() {
    use kentos_native_application::site_ground;
    let mut doc = building(false);
    assert_eq!(
        doc.settings().cad_work,
        Some(kentos_commands::CadWork::Architecture)
    );
    let layer = doc.to_snapshot_v2().active_layer;
    let (settings, made) = site_ground::with_ground(&doc, &layer, false).expect("a ground");
    assert!(made.triangles > 0 && made.low < made.high, "{made:?}");
    assert!(settings.road.is_none(), "no road settings");
    doc.set_settings(settings);
    let site = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.site.clone())
        .unwrap();
    assert_eq!(site.natural_ground, Some(SiteGround::of_layer(&layer)));
    let saved = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("saved");
    let back = kentos_kcad::decode(&saved).expect("read back");
    assert_eq!(
        back.settings
            .arch
            .and_then(|a| a.site)
            .and_then(|s| s.natural_ground),
        Some(SiteGround::of_layer(&layer))
    );
    // The graded ground of the same layer is refused before anything is set.
    let why = site_ground::with_ground(&doc, &layer, true).expect_err("the natural ground's layer");
    assert!(why.contains("tabii zeminin katmanı"), "{why}");
}

/// A ground level mark's lines (EK-4 4.30, Şekil 4.6): absolute, a point and two decimals, the difference from ±0.00
/// in brackets with its sign; ±0.00 itself as ±0.00; a ground not there gives no line, no ±0.00 no brackets.
#[test]
fn a_ground_level_mark_reads_as_the_regulation_s_figure() {
    use kentos_native_application::site_ground::level_lines;
    assert_eq!(
        level_lines(Some(849.85), Some(850.10), Some(855.5)),
        ["tzk 849.85 (-5.65)", "tsk 850.10 (-5.40)"]
    );
    assert_eq!(
        level_lines(Some(855.5), Some(855.75), Some(855.5)),
        ["tzk 855.50 (±0.00)", "tsk 855.75 (+0.25)"]
    );
    assert_eq!(
        level_lines(None, Some(850.1), Some(855.5)),
        ["tsk 850.10 (-5.40)"]
    );
    assert_eq!(level_lines(Some(849.85), None, None), ["tzk 849.85"]);
    assert!(level_lines(None, None, Some(1.0)).is_empty());
}
