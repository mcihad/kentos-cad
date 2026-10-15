//! A layer's renderer reads geometry in the project's drawing unit (docs/adr/0165 §2,
//! the 13 Ekim addendum): the classes Katman stili makes from `$alan` and `$uzunluk` are
//! in mm² and mm in a project in millimetres, and the layer draws each object by the
//! same numbers; a symbol's own expressions stay in metres, and a project in metres is
//! as it was. The shared cases (`fixtures/style/v1/units.json`, written by
//! `scripts/fixtures/style_units_cases.py` from the ADR) are the window's values, classes
//! and counts in each unit; the rest here is the drawn layer.
//!
//! The geometry is always the drawing's own: metres. Only the unit the renderer reads it
//! in changes, so a renderer written in a unit draws exactly what the same renderer
//! written in metres draws, once its numbers are in the unit.

use kentos_contracts::{Entity, LayerStyle};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::ReadingUnit;
use kentos_native_style::classify::{
    ExprScope, Method, categories_of, classes_present, graduated_of, numbers_per_object,
    unique_values, values_of,
};
use kentos_native_style::library::StyleLibrary;
use kentos_native_style::program::{BuildOptions, build_layer};
use kentos_native_style::renderer::Rule;
use kentos_native_style::tally::{RuleCount, category_tally, class_tally, drawable, rule_counts};
use serde_json::{Value, json};
use std::path::PathBuf;

/// The units a project is drawn in, each with the factors of a length and of an area
/// (metres to the unit, and its square).
const UNITS: [(&str, f64, f64, f64); 3] = [
    ("m", 1.0, 1.0, 1.0),
    ("cm", 100.0, 100.0, 10_000.0),
    ("mm", 1000.0, 1000.0, 1_000_000.0),
];

fn unit(per_metre: f64) -> ReadingUnit {
    ReadingUnit::per_metre(per_metre)
}

/// A rectangle `w` × `h` m: `w * h` m² and `2 (w + h)` m round.
fn rect(id: u32, w: f64, h: f64) -> Entity {
    let pts: Vec<Value> = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]
        .iter()
        .map(|(x, y)| json!({ "x": x + f64::from(id) * 10.0, "y": y }))
        .collect();
    serde_json::from_value(
        json!({ "kind": "polygon", "id": id, "layerId": "k", "attrs": {}, "pts": pts }),
    )
    .expect("an area")
}

/// A path `length` m long.
fn path(id: u32, length: f64) -> Entity {
    let x = f64::from(id) * 10.0;
    serde_json::from_value(
        json!({ "kind": "polyline", "id": id, "layerId": "k", "attrs": {},
        "pts": [{ "x": x, "y": 0.0 }, { "x": x + length, "y": 0.0 }] }),
    )
    .expect("a path")
}

fn point(id: u32, x: f64, y: f64) -> Entity {
    serde_json::from_value(
        json!({ "kind": "point", "id": id, "layerId": "k", "attrs": {}, "p": { "x": x, "y": y } }),
    )
    .expect("a point")
}

/// The drawing's objects: three areas of 3, 1 and 0.5 m² (7, 4 and 3 m round), a path of
/// 2 m and a point.
fn objects() -> Vec<Entity> {
    vec![
        rect(1, 2.0, 1.5),
        rect(2, 1.0, 1.0),
        rect(3, 0.5, 1.0),
        path(4, 2.0),
        point(5, 3.0, 4.0),
    ]
}

fn store_of(list: &[&Entity]) -> Store {
    let mut store = Store::new();
    store.put_many(list.iter().map(|e| {
        let b = e.base();
        (f64::from(b.id), b.layer_id.as_str(), false, shape(e))
    }));
    store
}

const RED: &str = "#FF0000";
const BLUE: &str = "#0000FF";

/// A fill symbol of one colour (an area's face; its edges are the layer's).
fn fill(color: &str) -> Value {
    json!({ "type": "fill", "layers": [{ "id": "0", "type": "simpleFill", "color": color }] })
}

fn line(color: &str) -> Value {
    json!({ "type": "line", "layers": [{ "id": "0", "type": "simpleLine", "color": color, "width": 0.3 }] })
}

/// What a build gave: the batches' JSON, their numbers and the pictures it made.
#[derive(Debug, PartialEq)]
struct Built {
    json: String,
    data: Vec<f32>,
    pictures: Vec<(String, u32, u32, Vec<u8>)>,
}

/// One layer's build as the desktop makes it: the objects' symbols from its `renderer`,
/// which reads geometry in `reading`; `view`: a view-dependent renderer's box and px per metre.
fn build(renderer: &Value, list: &[&Entity], reading: ReadingUnit, view: bool) -> Built {
    let store = store_of(list);
    let style: LayerStyle = serde_json::from_value(json!({
        "color": "#4E79A7", "lineType": "continuous", "lineWeight": 0.25, "renderer": renderer,
    }))
    .expect("a style");
    let library = StyleLibrary::default();
    let names = |_: &str| "Katman".to_owned();
    let opts = BuildOptions {
        line_types: None,
        layer_styles: None,
        origin: Vec2::new(0.0, 0.0),
        plot_scale: 1000.0,
        screen: false,
        hairlines: false,
        clip: view.then_some(Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 60.0,
            max_y: 20.0,
        }),
        library: &library,
        layer_name: &names,
        view: Default::default(),
        frame: view.then(|| (4.0, "heat:k:1".to_owned())),
        reading_unit: reading,
    };
    let (_, batches) = build_layer(&store, &style, list, &opts).expect("the core builds");
    Built {
        json: batches.json,
        data: batches.data,
        pictures: batches
            .pictures
            .into_iter()
            .map(|p| (p.key, p.width, p.height, p.rgba))
            .collect(),
    }
}

/// The batches' JSON of a build.
fn batches(b: &Built) -> Vec<Value> {
    serde_json::from_str::<Value>(&b.json)
        .expect("JSON")
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// The colours of a build's solid fills, in draw order.
fn fills(b: &Built) -> Vec<String> {
    batches(b)
        .iter()
        .filter(|x| x["kind"] == "fill" && x["style"]["kind"] == "solid")
        .filter_map(|x| x["style"]["color"].as_str().map(str::to_owned))
        .collect()
}

/// The colours of a build's strokes, in draw order.
fn strokes(b: &Built) -> Vec<String> {
    batches(b)
        .iter()
        .filter(|x| x["kind"] == "stroke")
        .filter_map(|x| x["style"]["color"].as_str().map(str::to_owned))
        .collect()
}

/// What Katman stili reads of the drawing, in `reading` (the window's `Source::scope`).
fn scope_in<T>(list: &[&Entity], reading: ReadingUnit, f: impl FnOnce(&ExprScope) -> T) -> T {
    let store = store_of(list);
    let name = |_: &str| "Katman".to_owned();
    let measures = |l: &[&Entity]| {
        let ids: Vec<f64> = l.iter().map(|e| f64::from(e.base().id)).collect();
        store.measures(&ids)
    };
    f(&ExprScope {
        layer_name: &name,
        measures: &measures,
        unit: reading,
    })
}

/// Katman stili's “Değerlerden sınıfla” and “Sınıfla” on `$alan` and `$uzunluk` list the
/// values in the project's unit, and what they make the layer draws by.
#[test]
fn the_values_the_window_lists_are_in_the_drawing_unit() {
    let all = objects();
    let list: Vec<&Entity> = all.iter().collect();
    for (name, per_metre, length, area) in UNITS {
        let r = unit(per_metre);
        let (alan, uzunluk) = scope_in(&list, r, |s| {
            (
                values_of(&list, "$alan", s).values,
                values_of(&list, "$uzunluk", s).values,
            )
        });
        let text = |x: f64| Some(format!("{x}"));
        assert_eq!(
            alan,
            [
                text(3.0 * area),
                text(1.0 * area),
                text(0.5 * area),
                None,
                None
            ],
            "$alan in {name}"
        );
        assert_eq!(
            uzunluk,
            [
                text(7.0 * length),
                text(4.0 * length),
                text(3.0 * length),
                text(2.0 * length),
                None
            ],
            "$uzunluk in {name}"
        );
    }
    // The owner's case, spelled out: 3 m² is 3 000 000 mm², 7 m round is 7 000 mm.
    let mm = unit(1000.0);
    let (alan, uzunluk) = scope_in(&list, mm, |s| {
        (
            values_of(&list, "$alan", s).values,
            values_of(&list, "$uzunluk", s).values,
        )
    });
    assert_eq!(alan[0].as_deref(), Some("3000000"));
    assert_eq!(uzunluk[0].as_deref(), Some("7000"));
}

/// The classes “Sınıfla” makes in a project in mm², their counts and what the layer draws:
/// every object in the class whose bounds hold its value, the same in every unit.
#[test]
fn graduated_classes_made_from_the_values_draw_the_objects_they_hold() {
    let all = objects();
    let list: Vec<&Entity> = all.iter().collect();
    for (name, per_metre, _, area) in UNITS {
        let r = unit(per_metre);
        let numbers = scope_in(&list, r, |s| numbers_per_object(&list, "$alan", s).values);
        let flat: Vec<f64> = numbers.iter().flatten().copied().collect();
        let present = classes_present(&list);
        let classes = graduated_of(&flat, Method::Interval, 2, "sariKirmizi", &present);
        // Equal intervals between the smallest (0.5) and the largest (3) of the areas.
        let bounds: Vec<f64> = vec![classes[0].min, classes[0].max, classes[1].max];
        assert_eq!(
            bounds,
            [0.5 * area, 1.75 * area, 3.0 * area],
            "bounds in {name}"
        );
        // The window's Nesne column: 0.5 and 1 in the first class, 3 in the second; the path
        // and the point have no area and are the rest.
        let (counts, rest) = class_tally(&numbers, &drawable(&list), &classes);
        assert_eq!((counts, rest), (vec![2, 1], 2), "counts in {name}");
        // The layer built with them, read in the same unit.
        let renderer = json!({ "type": "graduated", "expr": "$alan", "classes": serde_json::to_value(&classes).expect("classes") });
        let first = classes[0].symbols.fill.as_ref().expect("a fill symbol")["layers"][0]["color"]
            .as_str()
            .expect("a colour")
            .to_owned();
        let second = classes[1].symbols.fill.as_ref().expect("a fill symbol")["layers"][0]["color"]
            .as_str()
            .expect("a colour")
            .to_owned();
        assert_ne!(first, second);
        let drawn: Vec<Vec<String>> = list
            .iter()
            .map(|e| fills(&build(&renderer, &[*e], r, false)))
            .collect();
        assert_eq!(
            drawn,
            [
                vec![second.clone()],
                vec![first.clone()],
                vec![first.clone()],
                vec![],
                vec![]
            ],
            "the drawn layer in {name}"
        );
    }
}

/// Classes made in millimetres and drawn by a layer read in metres would meet no value:
/// the unit is what joins the window to the drawing.
#[test]
fn classes_in_the_drawing_unit_meet_no_value_read_in_another() {
    let all = objects();
    let list: Vec<&Entity> = all.iter().collect();
    let renderer = json!({ "type": "graduated", "expr": "$alan", "classes": [
        { "min": 500_000.0, "max": 1_750_000.0, "label": "a", "symbols": { "fill": fill(RED) } },
        { "min": 1_750_000.0, "max": 3_000_000.0, "label": "b", "symbols": { "fill": fill(BLUE) } },
    ] });
    let in_mm: Vec<Vec<String>> = list
        .iter()
        .map(|e| fills(&build(&renderer, &[*e], unit(1000.0), false)))
        .collect();
    assert_eq!(
        in_mm,
        [
            vec![BLUE.to_owned()],
            vec![RED.to_owned()],
            vec![RED.to_owned()],
            vec![],
            vec![]
        ]
    );
    // Read in metres the same numbers are 3, 1 and 0.5 (m²): below every class.
    for e in &list {
        assert!(fills(&build(&renderer, &[*e], ReadingUnit::METRE, false)).is_empty());
    }
}

/// Kategorili: the values “Değerlerden sınıfla” lists are the texts the layer compares.
#[test]
fn categories_made_from_the_values_draw_the_objects_with_that_value() {
    let all = objects();
    let list: Vec<&Entity> = all.iter().collect();
    for (name, per_metre, _, area) in UNITS {
        let r = unit(per_metre);
        let (values, found) = scope_in(&list, r, |s| {
            let v = values_of(&list, "$alan", s).values;
            let found = unique_values(&v);
            (v, found)
        });
        let present = classes_present(&list);
        let categories = categories_of(&found, &present, &[]);
        // Numbers by value: 0.5, 1 and 3 of the area's unit.
        let want: Vec<String> = [0.5, 1.0, 3.0].map(|a| format!("{}", a * area)).to_vec();
        assert_eq!(
            categories
                .iter()
                .map(|c| c.value.clone())
                .collect::<Vec<_>>(),
            want,
            "categories in {name}"
        );
        let (counts, rest) = category_tally(&values, &drawable(&list), &categories);
        // The path and the point have no value: it is the empty text, in no category.
        assert_eq!((counts, rest), (vec![1, 1, 1], 2), "counts in {name}");
        let renderer = json!({ "type": "categorized", "expr": "$alan",
            "categories": serde_json::to_value(&categories).expect("categories"),
            "other": { "fill": fill(BLUE), "line": line(BLUE), "marker": Value::Null } });
        let color_of = |i: usize| {
            categories[i].symbols.fill.as_ref().expect("a fill")["layers"][0]["color"]
                .as_str()
                .expect("a colour")
                .to_owned()
        };
        // 3 m² is the last of the three, 1 m² the middle one, 0.5 m² the first.
        let drawn: Vec<Vec<String>> = list[..3]
            .iter()
            .map(|e| fills(&build(&renderer, &[*e], r, false)))
            .collect();
        assert_eq!(
            drawn,
            [vec![color_of(2)], vec![color_of(1)], vec![color_of(0)]],
            "the drawn layer in {name}"
        );
    }
}

/// Kurallı: a rule's condition compares `$alan` and `$uzunluk` in the unit, in the window's
/// counts and in the drawing.
#[test]
fn rules_compare_the_values_in_the_drawing_unit() {
    let all = objects();
    let list: Vec<&Entity> = all.iter().collect();
    for (name, per_metre, length, area) in UNITS {
        let r = unit(per_metre);
        // Larger than 2 m² (3 m² only), or longer than 3.5 m round (7 and 4 m, not 3 m).
        let big = format!("$alan > {}", 2.0 * area);
        let long = format!("$uzunluk > {}", 3.5 * length);
        let mut a = Rule::new("a".into(), "Büyük", false);
        a.filter = Some(big.clone());
        let mut b = Rule::new("b".into(), "Uzun", false);
        b.filter = Some(long.clone());
        let rules = vec![a, b];
        let counts = scope_in(&list, r, |s| rule_counts(&rules, &list, s));
        assert_eq!(
            counts.get(&vec![0]),
            Some(&RuleCount::Count(1)),
            "{big} in {name}"
        );
        assert_eq!(
            counts.get(&vec![1]),
            Some(&RuleCount::Count(2)),
            "{long} in {name}"
        );
        // The same rules drawn, every match: the 3 m² area (7 m round) is in both, the 1 m²
        // one (4 m round) in the second, the 0.5 m² one (3 m round) in neither.
        let renderer = json!({ "type": "rules", "rules": [
            { "id": "a", "label": "Büyük", "filter": big, "symbols": { "fill": fill(RED) } },
            { "id": "b", "label": "Uzun", "filter": long, "symbols": { "fill": fill(BLUE) } },
        ] });
        let drawn: Vec<Vec<String>> = list[..3]
            .iter()
            .map(|e| fills(&build(&renderer, &[*e], r, false)))
            .collect();
        assert_eq!(
            drawn,
            [
                vec![RED.to_owned(), BLUE.to_owned()],
                vec![BLUE.to_owned()],
                vec![]
            ],
            "the drawn rules in {name}"
        );
    }
}

/// The renderers of ADR 0213 whose expressions are readings, each written in the unit
/// (`a`: its areas' factor, `l`: its lengths'), over the objects of `objects()`.
fn thematic(a: f64, l: f64) -> Vec<(&'static str, Value)> {
    let ramp = json!(["#FFFFFF", "#E15759", "#000000"]);
    let marker = json!({ "type": "marker", "layers": [
        { "id": "0", "type": "shape", "shape": "circle", "size": 3, "fill": "#59A14F" }] });
    vec![
        (
            "Sürekli renk",
            json!({ "type": "unclassed", "expr": "$alan", "min": 0.5 * a, "max": 3.0 * a,
                "ramp": ramp, "symbols": { "fill": fill("#888888"), "line": line("#888888") } }),
        ),
        (
            "Orantılı sembol",
            json!({ "type": "proportional", "expr": "$alan", "minValue": 0.5 * a, "maxValue": 3.0 * a,
                "minSize": 2, "maxSize": 10, "unit": "mm", "scaling": "area",
                "symbols": { "marker": marker, "fill": fill("#CCCCCC") } }),
        ),
        (
            "İki değişkenli renk",
            json!({ "type": "bivariate", "exprX": "$alan", "exprY": "$uzunluk",
                "breaksX": [2.0 * a], "breaksY": [3.5 * l],
                "colors": ["#EEEEEE", "#E15759", "#4E79A7", "#000000"],
                "symbols": { "fill": fill("#888888"), "line": line("#888888") } }),
        ),
        (
            "Nokta yoğunluğu",
            json!({ "type": "dotDensity", "fields": [{ "expr": "$alan", "color": "#E15759" }],
                "dotValue": 0.5 * a, "dotSize": 1, "unit": "mm", "seed": 3 }),
        ),
        (
            "Grafik",
            json!({ "type": "chart", "kind": "pie",
                "fields": [{ "expr": "$alan", "color": "#E15759" }, { "expr": "$alan / 2", "color": "#4E79A7" }],
                "size": 8, "unit": "mm",
                "sizeBy": { "minValue": 0.5 * a, "maxValue": 3.0 * a, "minSize": 4, "maxSize": 10 },
                "symbols": { "fill": fill("#CCCCCC") } }),
        ),
    ]
}

/// A renderer written in the unit and read in it draws what the same renderer in metres
/// draws; read in another unit it does not.
#[test]
fn the_thematic_renderers_read_their_values_in_the_drawing_unit() {
    let all = objects();
    let list: Vec<&Entity> = all.iter().collect();
    let in_metres = thematic(1.0, 1.0);
    for (name, per_metre, length, area) in UNITS {
        let in_unit = thematic(area, length);
        for ((label, metres), (_, written)) in in_metres.iter().zip(&in_unit) {
            let want = build(metres, &list, ReadingUnit::METRE, false);
            assert!(
                !want.json.is_empty() && want.json != "[]",
                "{label}: draws something"
            );
            let got = build(written, &list, unit(per_metre), false);
            assert_eq!(got, want, "{label} in {name}");
            if per_metre != 1.0 {
                // Its numbers are in the unit, read in metres they are another renderer.
                let wrong = build(written, &list, ReadingUnit::METRE, false);
                assert_ne!(wrong, want, "{label}: {name} numbers read in metres");
            }
        }
    }
}

/// Isı haritası's weight is a reading too: points weighted by their `$y` (east).
#[test]
fn a_heat_maps_weights_are_in_the_drawing_unit() {
    let points: Vec<Entity> = [
        (10.0, 5.0),
        (12.0, 6.0),
        (30.0, 9.0),
        (31.0, 10.0),
        (50.0, 15.0),
    ]
    .iter()
    .enumerate()
    .map(|(i, (x, y))| point(i as u32 + 1, *x, *y))
    .collect();
    let list: Vec<&Entity> = points.iter().collect();
    let heat = |l: f64| {
        json!({ "type": "heatmap", "radius": 6, "unit": "m", "weight": "$y", "max": 60.0 * l,
            "ramp": ["#FFFFFF", "#E15759", "#000000"], "quality": 2, "opacity": 1 })
    };
    let want = build(&heat(1.0), &list, ReadingUnit::METRE, true);
    assert_eq!(want.pictures.len(), 1, "the heat map's picture");
    assert!(
        want.pictures[0].3.iter().any(|b| *b != 0),
        "it paints something"
    );
    for (name, per_metre, length, _) in UNITS {
        let got = build(&heat(length), &list, unit(per_metre), true);
        assert_eq!(got, want, "weights in {name}");
        if per_metre != 1.0 {
            assert_ne!(build(&heat(length), &list, ReadingUnit::METRE, true), want);
        }
    }
}

/// Kümeleme and Yayma draw their single points with another renderer (Tek noktalar): its
/// expressions are readings in the unit too.
#[test]
fn a_clusters_and_a_spreads_single_points_read_in_the_drawing_unit() {
    // A point alone at 12 m east, and two close together at 30 m.
    let points = [
        point(1, 12.0, 1.0),
        point(2, 30.0, 5.0),
        point(3, 30.2, 5.1),
    ];
    let list: Vec<&Entity> = points.iter().collect();
    let dot = |color: &str| {
        json!({ "type": "marker", "layers": [
            { "id": "0", "type": "shape", "shape": "circle", "size": 3, "fill": color }] })
    };
    // The lone point's east coordinate is the category's value: 12, 1 200 or 12 000.
    let inner = |l: f64| {
        json!({ "type": "categorized", "expr": "$y",
            "categories": [{ "value": format!("{}", 12.0 * l), "label": "a",
                "symbols": { "marker": dot("#59A14F") } }],
            "other": { "marker": dot("#4E79A7") } })
    };
    let kinds = |l: f64| {
        vec![
            (
                "Kümeleme",
                json!({ "type": "cluster", "distance": 1, "unit": "m", "symbol": dot("#E15759"),
                    "count": true, "renderer": inner(l) }),
            ),
            (
                "Yayma",
                json!({ "type": "displacement", "tolerance": 1, "unit": "m", "placement": "ring",
                    "spacing": 0, "center": dot("#E15759"), "renderer": inner(l) }),
            ),
        ]
    };
    for (label, metres) in kinds(1.0) {
        let want = build(&metres, &list, ReadingUnit::METRE, false);
        for (name, per_metre, length, _) in UNITS {
            let written = kinds(length);
            let (_, renderer) = written
                .iter()
                .find(|(l, _)| *l == label)
                .expect("the same kind");
            assert_eq!(
                build(renderer, &list, unit(per_metre), false),
                want,
                "{label} in {name}"
            );
            if per_metre != 1.0 {
                assert_ne!(
                    build(renderer, &list, ReadingUnit::METRE, false),
                    want,
                    "{label}: {name} values read in metres"
                );
            }
        }
    }
}

/// A symbol is portable: its own expressions (a size, a width, a distance) stay in metres in
/// every project, even beside a renderer that reads the very same text in the unit.
#[test]
fn a_symbols_own_expressions_stay_in_metres() {
    let all = objects();
    // The 2 m path: its width is `$uzunluk` of the symbol (2 paper mm), its class the
    // renderer's `$uzunluk` in the unit (2 m, 200 cm, 2 000 mm).
    let wide = |color: &str| {
        json!({ "type": "line", "layers": [{ "id": "0", "type": "simpleLine", "color": color,
            "width": { "expr": "$uzunluk", "fallback": 9 } }] })
    };
    let renderer = |l: f64| {
        json!({ "type": "graduated", "expr": "$uzunluk", "classes": [
            { "min": 0.0, "max": 1.5 * l, "label": "kısa", "symbols": { "line": wide(BLUE) } },
            { "min": 1.5 * l, "max": 3.0 * l, "label": "orta", "symbols": { "line": wide(RED) } },
        ] })
    };
    let widths = |b: &Built| -> Vec<f64> {
        batches(b)
            .iter()
            .filter(|x| x["kind"] == "stroke")
            .filter_map(|x| x["style"]["width"].as_f64())
            .collect()
    };
    let list = [&all[3]];
    let metres = build(&renderer(1.0), &list, ReadingUnit::METRE, false);
    assert_eq!(strokes(&metres), [RED], "2 m is in the second class");
    let width = widths(&metres);
    assert_eq!(width.len(), 1);
    assert!(width[0] > 0.0);
    for (name, per_metre, length, _) in UNITS {
        let got = build(&renderer(length), &list, unit(per_metre), false);
        // The class by the unit's number (2 000 mm is in [1 500, 3 000)); the width by metres.
        assert_eq!(strokes(&got), [RED], "the class in {name}");
        assert_eq!(widths(&got), width, "the width in {name}");
    }
}

/// Geometri üretici's steps are the symbol's own: a buffer's distance from `$uzunluk` is a
/// distance in metres, whatever the project's unit (read in mm it would be a 350 m buffer).
#[test]
fn a_geometry_generators_distances_stay_in_metres() {
    let all = objects();
    // The salon: 7 m round, so a buffer of 0.35 m.
    let list = [&all[0]];
    let renderer = json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
        { "id": "g", "type": "geometryGenerator",
          "steps": [{ "kind": "buffer", "distance": { "expr": "$uzunluk / 20", "fallback": 0.1 },
              "side": "both" }],
          "symbol": fill(RED) }] } } });
    let metres = build(&renderer, &list, ReadingUnit::METRE, false);
    assert_eq!(fills(&metres), [RED], "the buffer is drawn");
    for (name, per_metre, _, _) in UNITS {
        assert_eq!(
            build(&renderer, &list, unit(per_metre), false),
            metres,
            "the generator in {name}"
        );
    }
}

/// A project in metres draws, lists and counts exactly as it did: the unit of metres is no
/// unit at all, and `Program::read` and a build in metres are the same program.
#[test]
fn a_project_in_metres_is_unchanged() {
    assert_eq!(ReadingUnit::per_metre(1.0), ReadingUnit::METRE);
    let all = objects();
    let list: Vec<&Entity> = all.iter().collect();
    for (label, renderer) in thematic(1.0, 1.0) {
        let a = build(&renderer, &list, ReadingUnit::METRE, false);
        let b = build(&renderer, &list, ReadingUnit::per_metre(1.0), false);
        assert_eq!(a, b, "{label}");
    }
    // The window's lists in metres: the values as the store measures them.
    let (alan, uzunluk) = scope_in(&list, ReadingUnit::METRE, |s| {
        (
            values_of(&list, "$alan", s).values,
            values_of(&list, "$uzunluk", s).values,
        )
    });
    assert_eq!(
        alan,
        [
            Some("3".to_owned()),
            Some("1".to_owned()),
            Some("0.5".to_owned()),
            None,
            None
        ]
    );
    assert_eq!(uzunluk[0].as_deref(), Some("7"));
}

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/style/v1/units.json");
    let text = std::fs::read_to_string(&path).expect("units.json");
    serde_json::from_str(&text).expect("units.json is JSON")
}

/// Numbers compared as numbers (1 and 1.0 alike), everything else as it is.
fn same(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, x)| b.get(k).is_some_and(|y| same(x, y)))
        }
        _ => got == want,
    }
}

/// The window's values, categories, classes, their counts and the rules' counts in each unit, as
/// `fixtures/style/v1/units.json` holds them (its reference: `scripts/fixtures/style_units_cases.py`).
#[test]
fn the_shared_unit_cases_as_the_window_makes_them() {
    let f = fixture();
    assert_eq!(f["format"], "kentos.style-units");
    assert_eq!(f["version"], 1);
    let entities: Vec<Entity> = f["entities"]
        .as_array()
        .expect("entities")
        .iter()
        .map(|e| serde_json::from_value(e.clone()).expect("an entity"))
        .collect();
    let list: Vec<&Entity> = entities.iter().collect();
    let present = classes_present(&list);
    let drawn = drawable(&list);
    let mut problems = Vec::new();
    let mut check = |got: Value, want: &Value, what: String| {
        if !same(&got, want) {
            problems.push(format!("{what}:\n  got  {got}\n  want {want}"));
        }
    };
    let mut seen = 0;
    for case in f["cases"].as_array().expect("cases") {
        let name = case["unit"].as_str().expect("a unit");
        let reading = unit(case["perMetre"].as_f64().expect("units to a metre"));
        for e in case["expressions"].as_array().expect("expressions") {
            let expr = e["expr"].as_str().expect("an expression");
            let what = |part: &str| format!("{expr} in {name}: {part}");
            let (values, numbers) = scope_in(&list, reading, |s| {
                (
                    values_of(&list, expr, s).values,
                    numbers_per_object(&list, expr, s).values,
                )
            });
            check(json!(values), &e["values"], what("values"));
            check(json!(numbers), &e["numbers"], what("numbers"));
            let found = unique_values(&values);
            let unique: Vec<Value> = found
                .iter()
                .map(|u| json!({ "value": u.value, "count": u.count }))
                .collect();
            check(json!(unique), &e["unique"], what("unique values"));
            // A category for each distinct value: what the Nesne column counts.
            let categories = categories_of(&found, &present, &[]);
            let (counts, rest) = category_tally(&values, &drawn, &categories);
            check(
                json!({ "counts": counts, "rest": rest }),
                &e["categories"],
                what("categories"),
            );
            let flat: Vec<f64> = numbers.iter().flatten().copied().collect();
            for g in e["graduated"].as_array().expect("graduated") {
                let n = g["n"].as_u64().expect("a class count") as usize;
                let classes = graduated_of(&flat, Method::Interval, n, "sariKirmizi", &present);
                let made: Vec<Value> = classes
                    .iter()
                    .map(|c| json!({ "min": c.min, "max": c.max, "label": c.label }))
                    .collect();
                check(json!(made), &g["classes"], what(&format!("{n} classes")));
                let (counts, rest) = class_tally(&numbers, &drawn, &classes);
                check(
                    json!({ "counts": counts, "rest": rest }),
                    &json!({ "counts": g["counts"], "rest": g["rest"] }),
                    what(&format!("{n} classes' counts")),
                );
            }
            seen += 1;
        }
        let rules: Vec<Rule> =
            serde_json::from_value(case["rules"]["rules"].clone()).expect("rules");
        let counts = scope_in(&list, reading, |s| rule_counts(&rules, &list, s));
        let got: Vec<Value> = case["rules"]["counts"]
            .as_array()
            .expect("counts")
            .iter()
            .map(|c| {
                let path: Vec<usize> = serde_json::from_value(c["path"].clone()).expect("a path");
                match counts.get(&path) {
                    Some(RuleCount::Count(n)) => json!({ "path": path, "count": n }),
                    Some(RuleCount::Error(m)) => json!({ "path": path, "error": m }),
                    None => json!({ "path": path }),
                }
            })
            .collect();
        check(
            json!(got),
            &case["rules"]["counts"],
            format!("rules in {name}"),
        );
    }
    assert_eq!(seen, 6, "two expressions in each of the three units");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
