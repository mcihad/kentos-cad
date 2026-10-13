//! Katman stili in a local project's drawing unit (docs/adr/0165 §2, the 13 Ekim
//! addendum): “Değerlerden sınıfla” and “Sınıfla” on `$alan` and `$uzunluk` list the
//! values in mm² and mm (cm, m: the project's), the Nesne column counts by them, and
//! the drawing area draws every object by the same numbers; the layer is built again
//! when the project's unit changes. The style crate's tests (`units.rs`) hold the
//! numbers; these drive the window and the drawing area end to end.

use kentos_contracts::{
    DrawingUnit, Entity, EntityBase, PathEntity, PointEntity, ProjectSettings, Vec2,
};
use kentos_domain::NewLayer;
use kentos_native_style::batches::{BatchKind, FillPaintBatch};
use kentos_native_style::color::parse_hex;
use kentos_native_style::renderer::GeometryClass;
use kentos_native_style::tally::{RuleCount, category_tally, class_tally};
use serde_json::{Value, json};

use super::thematic::{Edit, Of};
use super::{Event, Field, Kind, SetAt, Source};
use crate::app::{App, Message};
use crate::files_testing::app_with_drawing;

/// The project's units: how a length and an area of the drawing read in each.
const UNITS: [(DrawingUnit, f64, f64); 3] = [
    (DrawingUnit::Mm, 1000.0, 1_000_000.0),
    (DrawingUnit::Cm, 100.0, 10_000.0),
    (DrawingUnit::M, 1.0, 1.0),
];

fn ls(app: &mut App, e: Event) {
    let _ = app.update(Message::LayerStyle(e));
}

fn window(app: &App) -> &super::LayerStyleWindow {
    app.styles.layer_style.as_ref().expect("the window is open")
}

fn base(layer: &str) -> EntityBase {
    EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: layer.to_owned(),
        color: None,
        attrs: Default::default(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    }
}

/// A local project in `unit` whose only shown layer, Odalar, holds a salon of 3 m²
/// (2 × 1.5 m, 7 m round), a room of 1 m² (4 m round), a closet of 0.5 m² (3 m round),
/// a 2 m path and a point; the layer's id.
fn rooms(app: &mut App, unit: DrawingUnit) -> String {
    let doc = app.document.as_mut().expect("a drawing");
    let settings = ProjectSettings {
        srid: 0,
        drawing_unit: Some(unit),
        ..doc.model.settings().clone()
    };
    doc.model.set_settings(settings);
    let others: Vec<String> = doc
        .model
        .layers()
        .leaves()
        .into_iter()
        .map(|l| l.id.clone())
        .collect();
    let layer = doc
        .model
        .add_layer(NewLayer::layer("Odalar"), None, true)
        .expect("a layer");
    for id in others {
        doc.model.set_layer_visible(&id, false);
    }
    let p = |x: f64, y: f64| Vec2 { x, y };
    for (x, w, h) in [(0.0, 2.0, 1.5), (3.0, 1.0, 1.0), (5.0, 0.5, 1.0)] {
        doc.model
            .add(Entity::Polygon(PathEntity {
                base: base(&layer),
                pts: vec![p(x, 0.0), p(x + w, 0.0), p(x + w, h), p(x, h)],
                ms: None,
                bulges: None,
                holes: None,
                zs: None,
                parts: None,
                closed: false,
                widths: None,
            }))
            .expect("an area");
    }
    doc.model
        .add(Entity::Polyline(PathEntity {
            base: base(&layer),
            pts: vec![p(8.0, 0.0), p(10.0, 0.0)],
            ms: None,
            bulges: None,
            holes: None,
            zs: None,
            parts: None,
            closed: false,
            widths: None,
        }))
        .expect("a path");
    doc.model
        .add(Entity::Point(PointEntity {
            base: base(&layer),
            p: p(12.0, 1.0),
            z: None,
            parts: None,
        }))
        .expect("a point");
    layer
}

/// The solid fills the drawing area draws, as the viewport builds its scene: each colour with how
/// many objects it covers (a rectangle's triangles are twelve numbers), sorted by colour.
fn fills(app: &mut App) -> Vec<([f64; 4], usize)> {
    let doc = app.document.as_ref().expect("a drawing");
    app.spatial.sync(&doc.model);
    let palette = crate::viewport::palette(app.canvas());
    let styling = app.styles.styling(&app.spatial, &app.settings);
    let scene = app.viewport.styled_scene(doc, &palette, styling);
    let mut out = Vec::new();
    for part in &scene.layers {
        for b in &part.layer.batches {
            if let BatchKind::Fill {
                paint: FillPaintBatch::Solid { color },
            } = &b.kind
            {
                out.push((*color, b.range.len() / 12));
            }
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("colours compare"));
    out
}

/// A symbol set's fill colour as the drawing reads it.
fn color_of(symbols: &kentos_native_style::renderer::SymbolSet) -> [f64; 4] {
    let hex = symbols.fill.as_ref().expect("a fill symbol")["layers"][0]["color"]
        .as_str()
        .expect("a colour")
        .to_owned();
    parse_hex(&hex, 1.0)
}

fn sorted(mut v: Vec<([f64; 4], usize)>) -> Vec<([f64; 4], usize)> {
    v.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("colours compare"));
    v
}

/// The owner's report: Kategorili `$alan`, “Değerlerden sınıfla”; then Aralıklı `$alan`,
/// “Sınıfla”. In a project in millimetres the values are mm², and what the window made draws.
#[test]
fn the_window_classes_and_the_drawing_read_areas_in_the_project_unit() {
    for (unit, _, area) in UNITS {
        let mut app = app_with_drawing();
        let layer = rooms(&mut app, unit);
        ls(&mut app, Event::Open(Some(layer.clone())));
        ls(&mut app, Event::Kind(Kind::Categorized));
        ls(&mut app, Event::Expr(Field::Categories, "$alan".into()));
        ls(&mut app, Event::Classify);
        let w = window(&app);
        let values: Vec<String> = w
            .categorized
            .categories
            .iter()
            .map(|c| c.value.clone())
            .collect();
        let want: Vec<String> = [0.5, 1.0, 3.0].map(|a| format!("{}", a * area)).to_vec();
        assert_eq!(values, want, "the categories in {unit:?}");
        // The Nesne column: one object in each, the path and the point in none.
        let doc = &app.document.as_ref().expect("open").model;
        let src = Source {
            doc,
            store: app.spatial.store(),
        };
        let (counts, rest) = category_tally(
            &w.values(&src).values,
            &w.drawn(&src),
            &w.categorized.categories,
        );
        assert_eq!((counts, rest), (vec![1, 1, 1], 2), "the counts in {unit:?}");
        // The classes of equal interval.
        ls(&mut app, Event::Kind(Kind::Graduated));
        ls(&mut app, Event::Expr(Field::Classes, "$alan".into()));
        ls(&mut app, Event::Count("2".into()));
        ls(&mut app, Event::Graduate);
        let w = window(&app);
        let bounds: Vec<f64> = vec![
            w.graduated.classes[0].min,
            w.graduated.classes[0].max,
            w.graduated.classes[1].max,
        ];
        assert_eq!(
            bounds,
            [0.5 * area, 1.75 * area, 3.0 * area],
            "the bounds in {unit:?}"
        );
        let doc = &app.document.as_ref().expect("open").model;
        let src = Source {
            doc,
            store: app.spatial.store(),
        };
        let (counts, rest) = class_tally(
            &w.numbers(&src).values,
            &w.drawn(&src),
            &w.graduated.classes,
        );
        assert_eq!(
            (counts, rest),
            (vec![2, 1], 2),
            "the class counts in {unit:?}"
        );
        let (first, second) = (
            color_of(&w.graduated.classes[0].symbols),
            color_of(&w.graduated.classes[1].symbols),
        );
        // Uygula: the layer's renderer holds the numbers of the unit, and the drawing area
        // draws two objects in the first class and one in the second.
        ls(&mut app, Event::Apply);
        let renderer = app
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .and_then(|n| n.style.renderer.clone())
            .expect("a renderer");
        assert_eq!(renderer["type"], "graduated");
        assert_eq!(renderer["classes"][1]["max"], Value::from(3.0 * area));
        assert_eq!(
            fills(&mut app),
            sorted(vec![(first, 2), (second, 1)]),
            "the drawing in {unit:?}"
        );
    }
}

/// The unit is what meets the window's numbers and the drawing: a project whose unit changes
/// is drawn again, and classes made in mm² meet no area read in m².
#[test]
fn a_change_of_the_projects_unit_draws_the_layer_again() {
    let mut app = app_with_drawing();
    let layer = rooms(&mut app, DrawingUnit::Mm);
    ls(&mut app, Event::Open(Some(layer)));
    ls(&mut app, Event::Kind(Kind::Graduated));
    ls(&mut app, Event::Expr(Field::Classes, "$alan".into()));
    ls(&mut app, Event::Count("2".into()));
    ls(&mut app, Event::Graduate);
    ls(&mut app, Event::Apply);
    let in_mm = fills(&mut app);
    assert_eq!(
        in_mm.iter().map(|f| f.1).sum::<usize>(),
        3,
        "every area in a class: {in_mm:?}"
    );
    // The project's unit is metres now: the areas are 3, 1 and 0.5, below every class.
    let doc = app.document.as_mut().expect("open");
    let settings = ProjectSettings {
        drawing_unit: Some(DrawingUnit::M),
        ..doc.model.settings().clone()
    };
    doc.model.set_settings(settings);
    assert_eq!(fills(&mut app), vec![], "classes in mm² read in metres");
    // And back: the drawing is as it was.
    let doc = app.document.as_mut().expect("open");
    let settings = ProjectSettings {
        drawing_unit: Some(DrawingUnit::Mm),
        ..doc.model.settings().clone()
    };
    doc.model.set_settings(settings);
    assert_eq!(fills(&mut app), in_mm);
}

/// Kurallı: the conditions compare in the unit, in the window's counts and on the drawing.
#[test]
fn rules_count_and_draw_by_the_projects_unit() {
    for (unit, length, area) in UNITS {
        let mut app = app_with_drawing();
        let layer = rooms(&mut app, unit);
        ls(&mut app, Event::Open(Some(layer)));
        ls(&mut app, Event::Kind(Kind::Rules));
        // Larger than 2 m², longer than 3.5 m round.
        ls(
            &mut app,
            Event::Expr(Field::Rule(vec![0]), format!("$alan > {}", 2.0 * area)),
        );
        ls(&mut app, Event::AddRule);
        ls(
            &mut app,
            Event::Expr(Field::Rule(vec![1]), format!("$uzunluk > {}", 3.5 * length)),
        );
        let w = window(&app);
        let doc = &app.document.as_ref().expect("open").model;
        let counts = w.rule_counts(&Source {
            doc,
            store: app.spatial.store(),
        });
        assert_eq!(counts.get(&vec![0]), Some(&RuleCount::Count(1)), "{unit:?}");
        assert_eq!(counts.get(&vec![1]), Some(&RuleCount::Count(2)), "{unit:?}");
        // Each rule fills with a colour of its own: the salon is in both rules, the room in the
        // second, the closet in neither.
        let solid = |c: &str| json!({ "type": "fill", "layers": [{ "id": "0", "type": "simpleFill", "color": c }] });
        for (path, color) in [(vec![0], "#FF0000"), (vec![1], "#0000FF")] {
            ls(
                &mut app,
                Event::Symbol(SetAt::Rule(path), GeometryClass::Fill, Some(solid(color))),
            );
        }
        ls(&mut app, Event::Apply);
        let red = parse_hex("#FF0000", 1.0);
        let blue = parse_hex("#0000FF", 1.0);
        assert_eq!(
            fills(&mut app),
            sorted(vec![(red, 1), (blue, 2)]),
            "the drawing in {unit:?}"
        );
    }
}

/// Sürekli renk's “Verilerden al” takes the range of the values as the window reads them.
#[test]
fn a_continuous_colors_range_from_the_data_is_in_the_projects_unit() {
    for (unit, _, area) in UNITS {
        let mut app = app_with_drawing();
        let layer = rooms(&mut app, unit);
        ls(&mut app, Event::Open(Some(layer)));
        ls(&mut app, Event::Kind(Kind::Unclassed));
        ls(&mut app, Event::Expr(Field::Unclassed, "$alan".into()));
        ls(&mut app, Event::Thematic(Edit::FromData(Of::Unclassed)));
        let d = &window(&app).drafts.unclassed;
        assert_eq!((d.min, d.max), (0.5 * area, 3.0 * area), "{unit:?}");
    }
}
