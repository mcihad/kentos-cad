//! A local project's drawing unit (docs/adr/0165 §2) through the session:
//! what is typed is in the unit, what the drawing keeps is metres. The web's
//! are `apps/web/src/tools/drawingUnit.test.ts`.

use crate::common;

use common::Bench;
use kentos_contracts::{AngleUnit, DrawingUnit, Entity, ProjectSettings, Workspace};

/// `tool` running on the empty drawing made a local CAD project in `unit`,
/// its angles in degrees, as a new one is (docs/adr/0165 §2, §4).
fn local(tool: &str, unit: DrawingUnit) -> Bench {
    let mut b = Bench::new(tool);
    let settings = ProjectSettings {
        srid: 0,
        drawing_unit: Some(unit),
        workspace: Some(Workspace::Cad),
        angle_unit: AngleUnit::Deg,
        ..b.doc.settings().clone()
    };
    b.doc.set_settings(settings);
    b
}

fn ends(e: &Entity) -> [[f64; 2]; 2] {
    let Entity::Line(line) = e else {
        panic!("a line, not {e:?}");
    };
    [[line.a.x, line.a.y], [line.b.x, line.b.y]]
}

#[test]
fn typed_points_are_in_millimetres() {
    let mut b = local("line", DrawingUnit::Mm);
    for text in ["100,250", "@500,0", "@100<90"] {
        assert!(b.type_text(text), "{text}");
    }
    let mut lines: Vec<(u32, [[f64; 2]; 2])> =
        b.doc.entities().map(|e| (e.base().id, ends(e))).collect();
    lines.sort_by_key(|(id, _)| *id);
    let lines: Vec<[[f64; 2]; 2]> = lines.into_iter().map(|(_, l)| l).collect();
    assert_eq!(lines[0], [[0.1, 0.25], [0.6, 0.25]]);
    let [a, end] = lines[1];
    assert_eq!(a, [0.6, 0.25]);
    assert!(
        (end[0] - 0.6).abs() < 1e-12 && (end[1] - 0.35).abs() < 1e-12,
        "{end:?}"
    );
}

#[test]
fn a_typed_radius_is_in_millimetres() {
    let mut b = local("circle", DrawingUnit::Mm);
    assert!(b.type_text("0,0"));
    assert!(b.type_text("12.5"));
    let Entity::Circle(circle) = b.newest() else {
        panic!("a circle, not {:?}", b.newest());
    };
    assert_eq!(circle.r, 0.0125);
}

#[test]
fn a_project_with_a_coordinate_system_is_in_metres() {
    let mut b = local("line", DrawingUnit::Mm);
    let settings = ProjectSettings {
        srid: 5254,
        ..b.doc.settings().clone()
    };
    b.doc.set_settings(settings);
    for text in ["100,250", "@5,0"] {
        assert!(b.type_text(text), "{text}");
    }
    assert_eq!(ends(b.newest()), [[100.0, 250.0], [105.0, 250.0]]);
}

/// A 2 × 1.5 m area and a 1 × 1 m one on the drawing's layer: 3 and 1 m².
fn two_areas(b: &mut Bench) -> [kentos_domain::Slot; 2] {
    let square = |b: &mut Bench, x: f64, w: f64, h: f64| {
        let p = |x: f64, y: f64| kentos_contracts::Vec2 { x, y };
        b.doc
            .add(Entity::Polygon(kentos_contracts::PathEntity {
                ms: None,
                base: common::base("cizim"),
                pts: vec![p(x, 0.0), p(x + w, 0.0), p(x + w, h), p(x, h)],
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
            }))
            .expect("a slot")
    };
    [square(b, 0.0, 2.0, 1.5), square(b, 5.0, 1.0, 1.0)]
}

/// The labels the engine places over the drawing (every one, a place or not).
fn label_texts(b: &mut Bench) -> Vec<String> {
    b.spatial.sync(&b.doc);
    let window = kentos_geometry_core::geometry::Bounds {
        min_x: -10.0,
        min_y: -10.0,
        max_x: 20.0,
        max_y: 20.0,
    };
    let options = kentos_geometry_core::store::placing::PlaceOptions {
        unplaced: true,
        hidden: true,
    };
    let mut texts = b
        .spatial
        .store()
        .place_labels(&window, 40.0, &[], options)
        .texts;
    texts.sort();
    texts
}

/// The drawing's unit set again: `None` drops the project's own (metres).
fn in_unit(b: &mut Bench, unit: Option<DrawingUnit>) {
    let settings = ProjectSettings {
        drawing_unit: unit,
        ..b.doc.settings().clone()
    };
    b.doc.set_settings(settings);
}

/// A label's expression and a layer's filter read lengths and areas in the
/// drawing unit, as Öznitelikler shows them (docs/adr/0165 §2, the 12 Ekim
/// addendum); another unit makes them again. The web's are in
/// `apps/web/src/viewport/picking.test.ts`.
#[test]
fn labels_and_filters_read_lengths_and_areas_in_the_drawing_unit() {
    let mut b = local("line", DrawingUnit::Mm);
    let slots = two_areas(&mut b);
    let mut style = b
        .doc
        .layers()
        .get("cizim")
        .expect("the layer")
        .style
        .clone();
    style.label = Some(
        serde_json::from_str(
            r#"{"placement":"center","size":10,"text":"$alan || ' / ' || $uzunluk"}"#,
        )
        .expect("a label style"),
    );
    assert!(b.doc.set_layer_style("cizim", style, "Etiketler"));
    assert_eq!(label_texts(&mut b), ["1000000 / 4000", "3000000 / 7000"]);
    // In metres the same label reads m² and m.
    in_unit(&mut b, None);
    assert_eq!(label_texts(&mut b), ["1 / 4", "3 / 7"]);
    in_unit(&mut b, Some(DrawingUnit::Mm));
    let filter = serde_json::from_str(r#"{"expression":"$alan > 2000000"}"#).expect("a filter");
    b.doc
        .set_layer_filter("cizim", Some(filter), "Katman süzgeci")
        .expect("the filter is set");
    b.spatial.sync(&b.doc);
    assert!(
        b.spatial.filter_shown(slots[0]),
        "3 000 000 mm² > 2 000 000 mm²"
    );
    assert!(
        !b.spatial.filter_shown(slots[1]),
        "1 000 000 mm² ≤ 2 000 000 mm²"
    );
    assert_eq!(label_texts(&mut b), ["3000000 / 7000"]);
    // In metres the same filter leaves out both: neither is labelled.
    in_unit(&mut b, None);
    b.spatial.sync(&b.doc);
    assert!(!b.spatial.filter_shown(slots[0]));
    assert!(!b.spatial.filter_shown(slots[1]));
    assert!(label_texts(&mut b).is_empty());
}
