//! The style engine's own rules: units, marker places, waves, the inside
//! point of an area, renderers and the layer build (batches, levels,
//! dimension hairlines). The frozen answers of random symbols and layers
//! are held in `tests/style.rs` (fixtures/style/v1/cases.json).

use std::collections::HashMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::jsmath::{PI, cos, js_max, js_min, sin};
use kentos_geometry_core::store::Store;

use super::build::{
    LayerObjects, MODE_DIMENSION, MODE_RENDERER, MODE_SET, Program, build_layer, compile_one,
};
use super::compile::{Env, to_drawn, to_world};
use super::model::Unit;
use super::place::{
    MAX_MARKERS_PER_PATH, PlaceGroup, WaveSpec, interior_point, place_along, wave_paths,
};
use super::prim::PrimUnit;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn square(s: f64) -> Vec<Vec2> {
    vec![v(0.0, 0.0), v(s, 0.0), v(s, s), v(0.0, s)]
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn units_turn_paper_mm_into_metres_and_keep_px_for_drawn_sizes() {
    let aspects = HashMap::new();
    let env = |plot_scale| Env {
        plot_scale,
        aspects: &aspects,
        screen: false,
    };
    assert_eq!(to_world(1.0, Unit::Mm, &env(1000.0)), 1.0);
    assert_eq!(to_world(2.0, Unit::Mm, &env(500.0)), 1.0);
    assert_eq!(to_world(3.0, Unit::M, &env(1000.0)), 3.0);
    assert!(close(to_world(96.0, Unit::Px, &env(1000.0)), 25.4));
    assert_eq!(to_drawn(4.0, Unit::Px, &env(1000.0)), (4.0, PrimUnit::Px));
    assert_eq!(
        to_drawn(0.5, Unit::Mm, &env(2000.0)),
        (1.0, PrimUnit::World)
    );
}

#[test]
fn screen_sized_symbols_draw_paper_mm_as_px_and_keep_placement_in_metres() {
    let aspects = HashMap::new();
    let env = Env {
        plot_scale: 2000.0,
        aspects: &aspects,
        screen: true,
    };
    // Drawn sizes: 25.4 mm is 96 px whatever the scale, so zooming does not change them.
    let (w, u) = to_drawn(25.4, Unit::Mm, &env);
    assert!(close(w, 96.0) && u == PrimUnit::Px);
    assert_eq!(to_drawn(4.0, Unit::Px, &env), (4.0, PrimUnit::Px));
    // Map units stay metres; placing lengths follow the view's scale.
    assert_eq!(to_drawn(3.0, Unit::M, &env), (3.0, PrimUnit::World));
    assert_eq!(to_world(1.0, Unit::Mm, &env), 2.0);
}

#[test]
fn places_markers_at_intervals_vertices_ends_and_centres() {
    let path = [v(0.0, 0.0), v(10.0, 0.0), v(10.0, 10.0)];
    let at = |p: Vec<super::place::Placed>| p.iter().map(|q| (q.at.x, q.at.y)).collect::<Vec<_>>();
    assert_eq!(
        at(place_along(&path, false, "interval", 5.0, 0.0, None, 0.0)),
        [
            (0.0, 0.0),
            (5.0, 0.0),
            (10.0, 0.0),
            (10.0, 5.0),
            (10.0, 10.0)
        ]
    );
    let shifted: Vec<f64> = place_along(&path, false, "interval", 5.0, 2.5, None, 0.0)
        .iter()
        .map(|p| p.at.x + p.at.y)
        .collect();
    assert_eq!(shifted, [2.5, 7.5, 12.5, 17.5]);
    // A closed square does not repeat its start.
    assert_eq!(
        place_along(&square(10.0), true, "interval", 10.0, 0.0, None, 0.0).len(),
        4
    );
    let corners = place_along(&path, false, "vertex", 0.0, 0.0, None, 0.0);
    assert_eq!(corners.len(), 3);
    assert!(close(corners[1].angle, PI / 4.0));
    assert_eq!(
        at(place_along(
            &path,
            false,
            "innerVertex",
            0.0,
            0.0,
            None,
            0.0
        )),
        [(10.0, 0.0)]
    );
    assert_eq!(
        at(place_along(&path, false, "center", 0.0, 0.0, None, 0.0)),
        [(10.0, 0.0)]
    );
    assert_eq!(
        at(place_along(
            &path,
            false,
            "segmentCenter",
            0.0,
            0.0,
            None,
            0.0
        )),
        [(5.0, 0.0), (10.0, 5.0)]
    );
    assert!(close(
        place_along(&path, false, "last", 0.0, 0.0, None, 0.0)[0].angle,
        PI / 2.0
    ));
    // No interval, or NaN: nothing; a tiny one stops at the cap.
    assert!(place_along(&path, false, "interval", 0.0, 0.0, None, 0.0).is_empty());
    assert!(place_along(&path, false, "interval", f64::NAN, 0.0, None, 0.0).is_empty());
    assert_eq!(
        place_along(&path, false, "interval", 1e-9, 0.0, None, 0.0).len(),
        MAX_MARKERS_PER_PATH
    );
    // A single point is a place facing east; an empty path none.
    assert_eq!(
        at(place_along(
            &[v(3.0, 4.0)],
            false,
            "interval",
            5.0,
            0.0,
            None,
            0.0
        )),
        [(3.0, 4.0)]
    );
    assert!(place_along(&[], false, "first", 0.0, 0.0, None, 0.0).is_empty());
}

#[test]
fn groups_wrap_on_closed_paths_and_are_cut_at_open_ends() {
    let group = Some(PlaceGroup {
        count: 3.0,
        spacing: 1.0,
    });
    let closed: Vec<(f64, f64)> = place_along(&square(10.0), true, "first", 0.0, 0.0, group, 0.0)
        .iter()
        .map(|p| (p.at.x, p.at.y))
        .collect();
    assert_eq!(closed, [(0.0, 1.0), (0.0, 0.0), (1.0, 0.0)]);
    let open: Vec<f64> = place_along(
        &[v(0.0, 0.0), v(10.0, 0.0)],
        false,
        "first",
        0.0,
        0.0,
        group,
        0.0,
    )
    .iter()
    .map(|p| p.at.x)
    .collect();
    assert_eq!(open, [0.0, 1.0]);
}

#[test]
fn keeps_codes_clear_of_sharp_corners_only() {
    // 16 places round a 20 m square, 4 on its corners; a place exactly `clear` from a corner stays.
    let sq = square(20.0);
    assert_eq!(
        place_along(&sq, true, "interval", 5.0, 0.0, None, 0.0).len(),
        16
    );
    assert_eq!(
        place_along(&sq, true, "interval", 5.0, 0.0, None, 1.0).len(),
        12
    );
    assert_eq!(
        place_along(&sq, true, "interval", 5.0, 0.0, None, 5.0).len(),
        12
    );
    assert_eq!(
        place_along(&sq, true, "interval", 5.0, 0.0, None, 5.5).len(),
        4
    );
    // Gentle 15° turns are not corners.
    let ring: Vec<Vec2> = (0..24)
        .map(|i| {
            let a = f64::from(i) * PI / 12.0;
            v(20.0 * cos(a), 20.0 * sin(a))
        })
        .collect();
    let plain = place_along(&ring, true, "interval", 5.0, 0.0, None, 0.0).len();
    assert_eq!(
        place_along(&ring, true, "interval", 5.0, 0.0, None, 5.0).len(),
        plain
    );
}

fn wave(
    shape: &str,
    length: f64,
    spacing: f64,
    connect: bool,
    offset_along: Option<f64>,
) -> WaveSpec {
    WaveSpec {
        shape: shape.into(),
        length,
        amplitude: 1.0,
        spacing,
        connect,
        offset_along,
    }
}

#[test]
fn lays_sine_waves_along_a_path_connected_or_as_dashes() {
    let path = [v(0.0, 0.0), v(20.0, 0.0)];
    let joined = wave_paths(&path, false, &wave("sine", 5.0, 5.0, true, None));
    assert_eq!(joined.len(), 1);
    let pts = &joined[0];
    assert_eq!(pts[0], v(0.0, 0.0));
    assert_eq!(pts[pts.len() - 1], v(20.0, 0.0));
    // A quarter of the first wave is its crest, one amplitude to the left.
    let top = pts.iter().map(|p| p.y).fold(f64::MIN, js_max);
    let bottom = pts.iter().map(|p| p.y).fold(f64::MAX, js_min);
    assert!(close(top, 1.0) && close(bottom, -1.0));
    // Two 7-unit repeats fit in 20; each wave is its own piece, centred on the path.
    let dashed = wave_paths(&path, false, &wave("sine", 5.0, 7.0, false, None));
    assert_eq!(dashed.len(), 2);
    assert!(close(dashed[0][0].x, 4.0));
    assert!(close(dashed[1][dashed[1].len() - 1].x, 16.0));
}

#[test]
fn anchors_waves_at_the_path_start_so_markers_stay_in_step() {
    let path = [v(0.0, 0.0), v(20.0, 0.0)];
    let waves = wave_paths(&path, false, &wave("sine", 4.0, 5.0, false, Some(1.0)));
    let starts: Vec<f64> = waves.iter().map(|w| w[0].x).collect();
    assert_eq!(starts.len(), 4);
    for (s, want) in starts.iter().zip([1.0, 6.0, 11.0, 16.0]) {
        assert!(close(*s, want));
    }
    assert!(close(waves[3][waves[3].len() - 1].x, 20.0));
    // A closed square wraps the anchor into the first repeat.
    let ring = wave_paths(
        &square(10.0),
        true,
        &wave("sine", 4.0, 5.0, false, Some(11.0)),
    );
    assert_eq!(ring.len(), 8);
    assert!(close(ring[0][0].x, 1.0));
    // No wave length: the path as it is.
    assert_eq!(
        wave_paths(&path, false, &wave("sine", 0.0, 5.0, true, None)),
        vec![path.to_vec()]
    );
}

#[test]
fn finds_a_point_inside_any_area_never_in_a_hole() {
    assert_eq!(interior_point(&[square(10.0)]), Some(v(5.0, 5.0)));
    let u = vec![
        v(0.0, 0.0),
        v(30.0, 0.0),
        v(30.0, 30.0),
        v(20.0, 30.0),
        v(20.0, 10.0),
        v(10.0, 10.0),
        v(10.0, 30.0),
        v(0.0, 30.0),
    ];
    let p = interior_point(&[u]).expect("inside point");
    let in_u = (p.y < 10.0 && p.x > 0.0 && p.x < 30.0)
        || (p.x < 10.0 && p.x > 0.0)
        || (p.x > 20.0 && p.x < 30.0);
    assert!(in_u);
    let donut = interior_point(&[
        square(10.0),
        vec![v(3.0, 3.0), v(3.0, 7.0), v(7.0, 7.0), v(7.0, 3.0)],
    ])
    .expect("inside point");
    assert!(!(donut.x > 3.0 && donut.x < 7.0 && donut.y > 3.0 && donut.y < 7.0));
    assert_eq!(interior_point(&[]), None);
}

// ── Layers ─────────────────────────────────────────────────────────────

/// One object of a layer build: its geometry, attributes and how it is drawn (mode, set or symbol, simple set, colour).
struct Obj {
    shape: Shape,
    attrs: &'static [(&'static str, &'static str)],
    how: [i32; 4],
}

fn line(
    a: (f64, f64),
    b: (f64, f64),
    attrs: &'static [(&'static str, &'static str)],
    how: [i32; 4],
) -> Obj {
    Obj {
        shape: Shape::Line {
            a: v(a.0, a.1),
            b: v(b.0, b.1),
        },
        attrs,
        how,
    }
}

/// Builds a layer as the page does: the store, the program and the table of the fields its expressions read.
fn build(program: &str, objects: &[Obj], origin: Vec2) -> (Vec<Json>, Vec<f32>) {
    let program = Program::read(program).expect("program");
    let mut store = Store::new();
    let mut ids = Vec::new();
    let mut how = Vec::new();
    let mut texts = String::new();
    let mut lens = Vec::new();
    let mut numbers = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        let id = (i + 1) as f64;
        store.put(id, "k", false, o.shape.clone());
        ids.push(id);
        how.extend_from_slice(&o.how);
        for f in &program.fields {
            match o.attrs.iter().find(|(k, _)| k == f) {
                Some((_, value)) => {
                    texts.push_str(value);
                    lens.push(value.encode_utf16().count() as i32);
                }
                None => lens.push(-1),
            }
        }
        assert!(
            !program.needs.label
                && !program.needs.layer
                && !program.needs.kind
                && !program.needs.vertices
        );
        if program.needs.id {
            numbers.push(id);
        }
    }
    let objects = LayerObjects {
        ids: &ids,
        objects: &how,
        texts: &texts,
        text_lens: &lens,
        numbers: &numbers,
    };
    let out = build_layer(&store, &program, &objects, None, origin, 1000.0, false).expect("build");
    let Json::Arr(batches) = Json::parse(&out.json).expect("json") else {
        panic!("not an array: {}", out.json);
    };
    (batches, out.data)
}

fn text(v: &Json) -> &str {
    match v {
        Json::Str(s) => s,
        _ => "",
    }
}

fn number(v: &Json) -> Option<f64> {
    match v {
        Json::Num(x) => Some(*x),
        _ => None,
    }
}

const RED: &str =
    r##"{"type":"line","layers":[{"id":"r","type":"simpleLine","color":"#FF0000","width":0.3}]}"##;
const BLUE: &str =
    r##"{"type":"line","layers":[{"id":"b","type":"simpleLine","color":"#0000FF","width":0.3}]}"##;

/// The stroke batches' colours and scale ranges, in draw order.
fn strokes(batches: &[Json]) -> Vec<(String, Option<f64>, Option<f64>)> {
    batches
        .iter()
        .filter(|b| text(b.get("kind")) == "stroke")
        .map(|b| {
            (
                text(b.get("style").get("color")).to_string(),
                number(b.get("minScale")),
                number(b.get("maxScale")),
            )
        })
        .collect()
}

fn program(renderer: &str) -> String {
    format!(
        r##"{{"symbols":{{}},"renderer":{renderer},"sets":[],"refs":[],"colors":["#000000"],"assets":{{}}}}"##
    )
}

#[test]
fn categorized_graduated_and_single_renderers() {
    let cat = program(&format!(
        r#"{{"type":"categorized","expr":"Tur","categories":[{{"value":"yol","label":"Yol","symbols":{{"line":{RED}}}}}],"other":{{"line":{BLUE}}}}}"#
    ));
    let draw = |p: &str, attrs: &'static [(&'static str, &'static str)]| {
        strokes(
            &build(
                p,
                &[line(
                    (0.0, 0.0),
                    (1.0, 0.0),
                    attrs,
                    [MODE_RENDERER, 0, -1, 0],
                )],
                v(0.0, 0.0),
            )
            .0,
        )
    };
    assert_eq!(
        draw(&cat, &[("Tur", "yol")]),
        [("#FF0000".into(), None, None)]
    );
    assert_eq!(
        draw(&cat, &[("Tur", "dere")]),
        [("#0000FF".into(), None, None)]
    );
    // 10 is in the last class: its max is included.
    let grad = program(&format!(
        r#"{{"type":"graduated","expr":"$uzunluk * 10","classes":[{{"min":0,"max":5,"label":"kısa","symbols":{{"line":{RED}}}}},{{"min":5,"max":10,"label":"uzun","symbols":{{"line":{BLUE}}}}}]}}"#
    ));
    assert_eq!(draw(&grad, &[]), [("#0000FF".into(), None, None)]);
    let single = program(&format!(
        r#"{{"type":"single","symbols":{{"line":{RED}}}}}"#
    ));
    assert_eq!(draw(&single, &[]).len(), 1);
}

#[test]
fn rules_draw_every_match_narrow_scales_and_catch_the_rest() {
    let rules = program(&format!(
        r#"{{"type":"rules","rules":[
            {{"id":"1","label":"Anayol","filter":"Tur = 'ana'","symbols":{{"line":{RED}}},"maxScale":5000,"children":[{{"id":"1a","label":"Yakın","minScale":100,"maxScale":2000,"symbols":{{"line":{BLUE}}}}}]}},
            {{"id":"2","label":"Hepsi","symbols":{{"line":{BLUE}}}}},
            {{"id":"3","label":"Diğer","isElse":true,"symbols":{{"line":{RED}}}}}]}}"#
    ));
    let (batches, _) = build(
        &rules,
        &[line(
            (0.0, 0.0),
            (1.0, 0.0),
            &[("Tur", "ana")],
            [MODE_RENDERER, 0, -1, 0],
        )],
        v(0.0, 0.0),
    );
    assert_eq!(
        strokes(&batches),
        [
            ("#FF0000".into(), None, Some(5000.0)),
            ("#0000FF".into(), Some(100.0), Some(2000.0)),
            ("#0000FF".into(), None, None)
        ]
    );
    let else_only = program(&format!(
        r#"{{"type":"rules","rules":[{{"id":"x","label":"x","filter":"yanlış","symbols":{{"line":{BLUE}}}}},{{"id":"y","label":"y","isElse":true,"symbols":{{"line":{RED}}}}}]}}"#
    ));
    let (batches, _) = build(
        &else_only,
        &[line((0.0, 0.0), (1.0, 0.0), &[], [MODE_RENDERER, 0, -1, 0])],
        v(0.0, 0.0),
    );
    assert_eq!(strokes(&batches), [("#FF0000".into(), None, None)]);
}

#[test]
fn equal_styles_share_a_batch_of_origin_relative_segments() {
    let p = format!(
        r##"{{"symbols":{{}},"renderer":null,"sets":[{{"line":{RED}}}],"refs":[],"colors":["#FF0000"],"assets":{{}}}}"##
    );
    let objects = [
        line(
            (486010.0, 4420010.0),
            (486020.0, 4420010.0),
            &[],
            [MODE_SET, 0, 0, 0],
        ),
        line(
            (486020.0, 4420010.0),
            (486020.0, 4420030.0),
            &[],
            [MODE_SET, 0, 0, 0],
        ),
    ];
    let (batches, data) = build(&p, &objects, v(486000.0, 4420000.0));
    assert_eq!(batches.len(), 1);
    let b = &batches[0];
    // Six numbers a segment: its ends from the origin, the distance so far and which ends are the path's.
    assert_eq!(number(b.get("len")), Some(12.0));
    assert_eq!(&data[..6], &[10.0, 10.0, 20.0, 10.0, 0.0, 3.0]);
    assert_eq!(&data[6..], &[20.0, 10.0, 20.0, 30.0, 0.0, 3.0]);
    let Json::Arr(bounds) = b.get("bounds") else {
        panic!("bounds")
    };
    assert_eq!(
        bounds.iter().filter_map(number).collect::<Vec<_>>(),
        [10.0, 10.0, 20.0, 30.0]
    );
}

#[test]
fn dimensions_keep_their_hairlines_at_every_scale() {
    // A rule with a scale range comes first; the dimension after it is not limited by that range.
    let rules = program(&format!(
        r#"{{"type":"rules","rules":[{{"id":"1","label":"Yakın","maxScale":500,"symbols":{{"line":{RED}}}}}]}}"#
    ));
    let dim = Obj {
        shape: Shape::Dimension {
            a: v(0.0, 0.0),
            b: v(10.0, 0.0),
            offset: 3.0,
            height: 1.0,
            text: None,
            style: None,
            angle: None,
            c: None,
        },
        attrs: &[],
        how: [MODE_DIMENSION, 0, -1, 0],
    };
    let (batches, _) = build(
        &rules,
        &[
            line((0.0, 0.0), (1.0, 0.0), &[], [MODE_RENDERER, 0, -1, 0]),
            dim,
        ],
        v(0.0, 0.0),
    );
    let s = strokes(&batches);
    assert_eq!(s.len(), 2);
    assert_eq!(s[0], ("#FF0000".into(), None, Some(500.0)));
    assert_eq!(s[1], ("#000000".into(), None, None));
}

#[test]
fn one_symbol_draws_nothing_on_text_and_dimensions() {
    let call = |entity: &str| {
        let input = format!(
            r#"{{"symbol":{RED},"entity":{entity},"layerName":"","kindLabel":"","vertices":null,"plotScale":1000,"assets":{{}}}}"#
        );
        compile_one(&Json::parse(&input).expect("input")).expect("compile")
    };
    let empty = r#"{"strokes":[],"fills":[],"markers":[]}"#;
    assert_eq!(
        call(
            r#"{"id":1,"layerId":"a","attrs":{},"kind":"dimension","a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":3,"height":1}"#
        ),
        empty
    );
    assert_eq!(
        call(
            r#"{"id":1,"layerId":"a","attrs":{},"kind":"text","p":{"x":0,"y":0},"text":"A","height":1,"rotation":0}"#
        ),
        empty
    );
    assert_ne!(
        call(
            r#"{"id":1,"layerId":"a","attrs":{},"kind":"line","a":{"x":0,"y":0},"b":{"x":10,"y":0}}"#
        ),
        empty
    );
}
