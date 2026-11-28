//! Kütle (docs/adr/8178): a massing study's prism. In the section model it is a prism from its bottom to its top on
//! MM-XXXX-KUTLE, its cut light and unfilled, its courtyard a hole and its arcs round; it casts the sun's shadow. The
//! IFC does not write it, and the frame's joins do not cut the elements inside it.

use kentos_domain::Document;
use kentos_geometry_core::geom::section::Height;
use kentos_geometry_core::geom::space::area as area_of;
use kentos_native_application::{arch_ifc, sections, shadows};
use serde_json::{Value, json};

/// Zemin kat and 1. kat, 3 m each; `entities` on them, their uids …001, …002 in order.
fn drawing(entities: Vec<Value>) -> Document {
    let mut snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "Kütle", "origin": {"x": 0.0, "y": 0.0},
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
        snapshot["uids"].as_array_mut().unwrap().push(json!(uid(i + 1)));
    }
    Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap()
}

fn uid(n: usize) -> String {
    format!("0192f5a1-7242-7000-8000-{n:012x}")
}

fn ring(pts: &[(f64, f64)]) -> Value {
    json!({"pts": pts.iter().map(|(x, y)| json!({"x": x, "y": y})).collect::<Vec<_>>()})
}

/// A mass of `parts` on `level`, `height` high, its bottom `offset` over the storey's floor.
fn mass(parts: Value, level: &str, height: f64, offset: f64) -> Value {
    json!({"kind": "mass", "parts": parts, "height": height, "offset": offset, "name": "Blok A", "arch": {"level": level}})
}

fn square(x0: f64, y0: f64, side: f64) -> Value {
    ring(&[(x0, y0), (x0 + side, y0), (x0 + side, y0 + side), (x0, y0 + side)])
}

#[test]
fn a_mass_is_a_prism_from_its_bottom_to_its_top_its_courtyard_a_hole_light_on_its_layer() {
    // 1. kat's floor at 3 m, its bottom 1.5 m under it, 9 m high; a 4 × 4 m courtyard.
    let doc = drawing(vec![mass(json!([{"outer": square(0.0, 0.0, 10.0), "holes": [square(3.0, 3.0, 4.0)]}]), "k1", 9.0, -1.5)]);
    let m = sections::model(&doc);
    assert_eq!(m.solids.len(), 1, "one prism");
    let s = &m.solids[0];
    assert!(
        matches!(s.height, Height::Prism { bottom, top } if (bottom - 1.5).abs() < 1e-9 && (top - 10.5).abs() < 1e-9),
        "{:?}",
        s.height
    );
    assert_eq!(s.rings.len(), 2, "its outer ring and its courtyard");
    let look = &m.looks[0];
    assert_eq!((look.layer.as_str(), look.key.as_str(), look.heavy, look.fill.is_none()), ("MM-XXXX-KUTLE", "KUTLE", false, true));
    assert_eq!(m.owners[0], Some(uid(1)));
}

#[test]
fn a_round_mass_is_round_and_a_mass_without_its_storey_stands_on_zero() {
    // Two half circles of radius 3: the prism's ring follows the arc.
    let circle = json!([{"outer": {"pts": [{"x": 3.0, "y": 0.0}, {"x": -3.0, "y": 0.0}], "bulges": [1.0, 1.0]}}]);
    let doc = drawing(vec![json!({"kind": "mass", "parts": circle, "height": 6.0})]);
    let m = sections::model(&doc);
    let s = &m.solids[0];
    assert!(matches!(s.height, Height::Prism { bottom, top } if bottom.abs() < 1e-9 && (top - 6.0).abs() < 1e-9));
    let r = &s.rings[0];
    assert!(r.len() > 16, "the arc in many points: {}", r.len());
    for p in r {
        assert!((p.x.hypot(p.y) - 3.0).abs() < 1e-6, "on the circle: {p:?}");
    }
}

#[test]
fn a_mass_casts_its_shadow_as_far_as_it_is_high_under_a_sun_at_45_degrees() {
    let doc = drawing(vec![mass(json!([{"outer": square(-0.5, -0.5, 1.0)}]), "zemin", 3.0, 0.0)]);
    let s = 0.5_f64.sqrt();
    let area = area_of(&shadows::shadow(&doc, [0.0, -s, s]));
    assert!((area - 3.0).abs() < 1e-6, "1 × 3 m north of it, as a column's: {area}");
}

#[test]
fn the_ifc_writes_no_mass_and_the_frame_does_not_join_the_elements_inside_it() {
    let wall = json!({"kind": "wall", "pts": [{"x": 1.0, "y": 1.0}, {"x": 7.0, "y": 1.0}], "width": 0.25, "height": 3.0,
        "arch": {"level": "zemin"}});
    let column = json!({"kind": "column", "at": {"x": 4.0, "y": 4.0}, "width": 0.4, "depth": 0.4, "arch": {"level": "zemin"}});
    let alone = drawing(vec![wall.clone(), column.clone()]);
    let massed = drawing(vec![wall, column, mass(json!([{"outer": square(0.0, 0.0, 10.0)}]), "zemin", 6.0, 0.0)]);
    let (b0, ..) = arch_ifc::model(&alone);
    let (b1, ..) = arch_ifc::model(&massed);
    let classes = |b: &kentos_ifc::building::Building| b.elements.iter().map(|e| format!("{:?}", e.class)).collect::<Vec<_>>();
    assert_eq!(classes(&b1), classes(&b0), "the mass is no element");
    // The wall's and the column's solids as they were: nothing of them is cut by the mass round them.
    let (m0, m1) = (sections::model(&alone), sections::model(&massed));
    for owner in [uid(1), uid(2)] {
        let of = |m: &sections::Model| {
            m.solids.iter().zip(&m.owners).filter(|(_, o)| o.as_deref() == Some(owner.as_str())).map(|(s, _)| format!("{:?}", (&s.rings, &s.height))).collect::<Vec<_>>()
        };
        assert_eq!(of(&m1), of(&m0), "{owner}");
    }
    assert_eq!(m1.solids.len(), m0.solids.len() + 1);
}
