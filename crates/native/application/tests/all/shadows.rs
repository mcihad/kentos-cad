//! Gölge (docs/adr/8177 §3.2): the building's shadow on the ground at ±0.00 under the sun, each solid swept along the
//! rays down to the ground, their union less the building's footprint; the sun's place from the project or the ones
//! given; drawn filled for one hour and as outlines with their times for several, the old shadows taken away.

use kentos_commands::Entity;
use kentos_domain::Document;
use kentos_geometry_core::geom::space::area as area_of;
use kentos_native_application::shadows;
use serde_json::{Value, json};

/// Zemin kat 3 m high; `entities` on it.
fn drawing(entities: Vec<Value>) -> Document {
    let mut snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "Gölge", "origin": {"x": 0.0, "y": 0.0},
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 50.0, "workspace": "cad", "cadWork": "architecture",
            "arch": {"levels": [{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
                {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0}]}},
        "layers": [{"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
        "activeLayer": "0", "styles": {"items": [], "categories": []}, "uids": [], "entities": []
    });
    for (i, mut e) in entities.into_iter().enumerate() {
        e["id"] = json!(i + 1);
        e["layerId"] = json!("0");
        e["attrs"] = json!({});
        snapshot["entities"].as_array_mut().unwrap().push(e);
        snapshot["uids"]
            .as_array_mut()
            .unwrap()
            .push(json!(format!("0192f5a1-7242-7000-8000-{:012x}", i + 1)));
    }
    Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap()
}

/// A column 1 × 1 m, 3 m high, at (0, 0).
fn column() -> Value {
    json!({"kind": "column", "at": {"x": 0.0, "y": 0.0}, "width": 1.0, "depth": 1.0, "arch": {"level": "zemin"}})
}

#[test]
fn a_column_s_shadow_runs_as_far_as_it_is_high_under_a_sun_at_45_degrees() {
    let doc = drawing(vec![column()]);
    // The sun in the south, 45° up: the shadow runs north, 3 m from the column's foot.
    let s = 0.5_f64.sqrt();
    let shadow = shadows::shadow(&doc, [0.0, -s, s]);
    let area: f64 = area_of(&shadow);
    assert!(
        (area - 3.0).abs() < 1e-6,
        "1 × 3 m north of the column, its footprint left out: {area}"
    );
    let ys: Vec<f64> = shadow
        .iter()
        .flat_map(|a| a.outer.pts.iter().map(|p| p.y))
        .collect();
    let (lo, hi) = (
        ys.iter().copied().fold(f64::MAX, f64::min),
        ys.iter().copied().fold(f64::MIN, f64::max),
    );
    assert!(
        (lo - 0.5).abs() < 1e-6 && (hi - 3.5).abs() < 1e-6,
        "{lo} … {hi}"
    );
    // A low sun: longer; the sun down: none.
    let long: f64 = area_of(&shadows::shadow(
        &doc,
        [0.0, -0.9, kentos_num::sin(30f64.to_radians())],
    ));
    assert!(long > area, "{long}");
    assert!(
        shadows::shadow(&doc, [0.0, -1.0, 0.0]).is_empty(),
        "the sun down"
    );
}

/// A box of walls 25 cm thick, 3 m high, its outer faces (x0, 0) – (x0 + 6, 4).
fn box_walls(x0: f64) -> Value {
    let (a, b) = (x0 + 0.125, x0 + 5.875);
    json!({"kind": "wall", "pts": [{"x": a, "y": 0.125}, {"x": b, "y": 0.125}, {"x": b, "y": 3.875},
        {"x": a, "y": 3.875}, {"x": a, "y": 0.125}], "width": 0.25, "height": 3.0, "justify": "center",
        "arch": {"level": "zemin"}})
}

#[test]
fn the_rooms_are_the_building_not_the_ground_in_its_shadow() {
    let s = 0.5_f64.sqrt();
    let sun = [0.0, -s, s];
    // The building's outline is the ground it stands on: 6 × 3 m north of it, nothing in its room.
    let one = area_of(&shadows::shadow(&drawing(vec![box_walls(0.0)]), sun));
    assert!((one - 18.0).abs() < 1e-6, "{one}");
    // A second building, its room covered by 1. kat's slab: the room is the building too. Open to the sky, its south
    // wall's shadow falls into it (5.5 × 3 m).
    let slab = json!({"kind": "slab", "parts": [{"outer": {"pts": [{"x": 10.0, "y": 0.0}, {"x": 16.0, "y": 0.0},
        {"x": 16.0, "y": 4.0}, {"x": 10.0, "y": 4.0}]}}], "thickness": 0.15, "arch": {"level": "k1"}});
    let covered = area_of(&shadows::shadow(
        &drawing(vec![box_walls(0.0), box_walls(10.0), slab]),
        sun,
    ));
    let open = area_of(&shadows::shadow(
        &drawing(vec![box_walls(0.0), box_walls(10.0)]),
        sun,
    ));
    assert!((covered - 2.0 * one).abs() < 1.0, "{covered}");
    assert!((open - covered - 16.5).abs() < 1.0, "{open} {covered}");
}

#[test]
fn light_passes_through_a_slab_s_hole() {
    // A 10 m square slab on 1. kat (2.85–3.00 m) with a 4 m square hole in its middle; the sun in the south 45° up.
    let ring = |x0: f64, y0: f64, s: f64| json!({"pts": [{"x": x0, "y": y0}, {"x": x0 + s, "y": y0}, {"x": x0 + s, "y": y0 + s}, {"x": x0, "y": y0 + s}]});
    let slab = json!({"kind": "slab", "parts": [{"outer": ring(0.0, 0.0, 10.0), "holes": [ring(3.0, 3.0, 4.0)]}],
        "thickness": 0.15, "arch": {"level": "k1"}});
    let s = 0.5_f64.sqrt();
    let a = area_of(&shadows::shadow(&drawing(vec![slab]), [0.0, -s, s]));
    // Its sweep 10 × 10.15 less the light through the hole 4 × 3.85: the hole cast no shadow.
    assert!((a - (101.5 - 15.4)).abs() < 1e-6, "{a}");
}

#[test]
fn the_place_is_the_project_s_or_the_one_given_and_true_north_the_plan_s_without_an_arrow() {
    let wall = json!({"kind": "wall", "pts": [{"x": 0.0, "y": 0.0}, {"x": 6.0, "y": 0.0}], "width": 0.25, "height": 3.0,
                      "arch": {"level": "zemin"}});
    let doc = drawing(vec![wall]);
    assert!(!shadows::located(&doc), "a local project");
    let why = shadows::place(&doc, None, 3.0).expect_err("a local project, nothing given");
    assert!(why.contains("enlemini (E) ve boylamını (B)"), "{why}");
    let p = shadows::place(&doc, Some((39.75, 37.02)), 3.0).expect("given");
    assert_eq!((p.lat, p.lon, p.zone, p.north), (39.75, 37.02, 3.0, 0.0));
    assert!(
        p.notes
            .iter()
            .any(|n| n.contains("plan kuzeyi coğrafi kuzey sayıldı")),
        "{:?}",
        p.notes
    );
    // 21 December at 12:30 in Sivas: the sun due south, 26.8° up; towards it in the plan, south and up.
    let (sun, plan) = shadows::sun_at(&p, 355, 12.5);
    assert!((sun.elevation - 26.8).abs() < 0.2, "{sun:?}");
    assert!(
        plan[0].abs() < 0.01 && plan[1] < 0.0 && plan[2] > 0.0,
        "{plan:?}"
    );
    assert_eq!(shadows::clock(9.0), "09:00");
    assert_eq!(shadows::clock(15.5), "15:30");
}

#[test]
fn one_hour_is_filled_several_are_outlined_with_their_times_and_drawn_again_they_replace() {
    let mut doc = drawing(vec![column()]);
    let s = 0.5_f64.sqrt();
    let sun = [0.0, -s, s];
    let one = vec![("12:00".to_owned(), sun, shadows::shadow(&doc, sun))];
    assert_eq!(shadows::draw(&mut doc, "0", &one).expect("drawn"), 1);
    let kinds = |doc: &Document| {
        let mut k: Vec<&str> = doc
            .entities()
            .filter(|e| e.base().attrs.contains_key(shadows::MARK_ATTR))
            .map(|e| match e {
                Entity::Hatch(_) => "hatch",
                Entity::Polyline(_) => "outline",
                Entity::Text(_) => "time",
                _ => "other",
            })
            .collect();
        k.sort_unstable();
        k
    };
    assert_eq!(kinds(&doc), ["hatch", "outline", "time"]);
    let east = [0.5, -0.5, s];
    let two = vec![
        ("09:00".to_owned(), east, shadows::shadow(&doc, east)),
        ("12:00".to_owned(), sun, shadows::shadow(&doc, sun)),
    ];
    shadows::draw(&mut doc, "0", &two).expect("drawn again");
    assert_eq!(
        kinds(&doc),
        ["outline", "outline", "time", "time"],
        "the old one gone, no fill for several"
    );
    let times: Vec<String> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Text(t) if t.base.attrs.contains_key(shadows::MARK_ATTR) => {
                Some(t.text.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(times, ["09:00", "12:00"]);
}

/// Shadows on a locked layer are not taken away (docs/adr/9401 §10): drawn again on another layer, the old ones stay
/// as they are, no second set is drawn, and the refusal names the layer.
#[test]
fn shadows_on_a_locked_layer_are_not_replaced() {
    use kentos_domain::NewLayer;
    let mut doc = drawing(vec![column()]);
    let layer = doc
        .add_layer(NewLayer::layer("Gölge"), None, false)
        .unwrap();
    let s = 0.5_f64.sqrt();
    let sun = [0.0, -s, s];
    let one = vec![("12:00".to_owned(), sun, shadows::shadow(&doc, sun))];
    shadows::draw(&mut doc, &layer, &one).expect("drawn");
    let marks = |doc: &Document| -> Vec<(String, u32)> {
        doc.entities()
            .filter(|e| e.base().attrs.contains_key(shadows::MARK_ATTR))
            .map(|e| (e.base().layer_id.clone(), e.base().id))
            .collect()
    };
    let before = marks(&doc);
    assert!(!before.is_empty());
    assert!(doc.set_layer_locked(&layer, true));
    let why =
        shadows::draw(&mut doc, "0", &one).expect_err("the old shadows are on a locked layer");
    assert!(
        why.contains("“Gölge”") && why.contains("kilitli") && why.contains("Katmanlar"),
        "{why}"
    );
    assert_eq!(
        marks(&doc),
        before,
        "the old shadows as they were, no second set"
    );
}
