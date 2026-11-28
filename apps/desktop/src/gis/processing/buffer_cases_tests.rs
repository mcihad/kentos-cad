//! The municipal report's 11th item, what was left of it (docs/adr/9179): Tampon on a road's axis, its cases one by
//! one. In a metric system 10 m makes the road's corridor; in a geographic one the run is refused before it starts and
//! nothing is made; Durdur ends a background run with the drawing as it was; an area crossing itself is buffered as
//! drawn and the run says it is invalid and where to look; a result name a layer has already writes into that layer.
//! The record keeps the run in words: its input as it resolved, the distance with its unit and the layer it wrote.

use serde_json::json;

use kentos_processing::{Status, Target};

use super::Event;
use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::gis;

/// A GIS drawing in `srid` with these layers (id, name, geometry; none for a layer of any kind), empty.
fn drawing(srid: u32, layers: &[(&str, &str, &str)]) -> App {
    let mut drawing: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ))
    .expect("the drawing");
    drawing["settings"]["srid"] = json!(srid);
    drawing["settings"]["workspace"] = json!("gis");
    let layer = |id: &str, name: &str, geometry: &str| {
        let mut layer = json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false,
                "expanded": true, "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 },
                "children": [] });
        if !geometry.is_empty() {
            layer["geometry"] = json!(geometry);
        }
        layer
    };
    drawing["layers"].as_array_mut().expect("layers").extend(
        layers
            .iter()
            .map(|(id, name, geometry)| layer(id, name, geometry)),
    );
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string()).expect("reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// An object on `layer`.
fn create(app: &mut App, layer: &str, geometry: serde_json::Value) {
    let input =
        serde_json::from_value(json!({ "layerId": layer, "objects": [{ "geometry": geometry }] }))
            .expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    let done = kentos_native_application::create::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        input,
    );
    assert!(
        matches!(done, kentos_commands::CommandResult::Completed { .. }),
        "{done:?}"
    );
}

/// A place `x`, `y` metres from the drawings' origin (500000, 4400000).
fn p(x: f64, y: f64) -> serde_json::Value {
    json!({ "x": 500_000.0 + x, "y": 4_400_000.0 + y })
}

/// A GIS drawing in `srid` with “Yol ekseni” (a 100 m road axis, east–west) and “Alanlar” (an area whose ring crosses
/// itself, a bow tie).
fn open(srid: u32) -> App {
    let mut app = drawing(
        srid,
        &[
            ("yol", "Yol ekseni", "line"),
            ("alanlar", "Alanlar", "area"),
        ],
    );
    create(
        &mut app,
        "yol",
        json!({ "kind": "line", "a": p(0.0, 0.0), "b": p(100.0, 0.0) }),
    );
    create(
        &mut app,
        "alanlar",
        json!({ "kind": "polygon", "pts": [p(0.0, 50.0), p(10.0, 60.0), p(10.0, 50.0), p(0.0, 60.0)] }),
    );
    app
}

/// The CBS review's points (docs/adr/9333): “qa_noktalar” with three points 30 m apart, and “Karışık” (a layer of any
/// kind) with the same three points and a 100 m line drawn west to east 50 m north of them.
fn points() -> App {
    let mut app = drawing(
        5256,
        &[
            ("noktalar", "qa_noktalar", "point"),
            ("karisik", "Karışık", ""),
        ],
    );
    for layer in ["noktalar", "karisik"] {
        for i in 0..3 {
            create(
                &mut app,
                layer,
                json!({ "kind": "point", "p": p(30.0 * f64::from(i), 0.0) }),
            );
        }
    }
    create(
        &mut app,
        "karisik",
        json!({ "kind": "line", "a": p(0.0, 50.0), "b": p(100.0, 50.0) }),
    );
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Gis(gis::Event::Processing(e)));
}

/// Tampon opened on `layer` with 10 m (or `distance`) and the result named `name`.
fn tampon(app: &mut App, layer: &str, distance: f64, name: &str) {
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    event(
        app,
        Event::Value(
            "input".into(),
            json!({ "scope": "layer", "layerId": layer }),
        ),
    );
    event(app, Event::Value("distance".into(), json!(distance)));
    event(
        app,
        Event::Value("layer".into(), json!({ "newName": name })),
    );
}

fn run(app: &mut App) {
    let task = app.update(Message::Gis(gis::Event::Processing(Event::Run)));
    drive(app, task);
}

/// The layers named `name` and their objects' boxes.
fn results(app: &App, name: &str) -> Vec<(String, Vec<[f64; 4]>)> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.layers()
        .leaves()
        .into_iter()
        .filter(|n| n.name == name)
        .map(|n| {
            let boxes = doc
                .by_layer(&n.id)
                .map(|e| {
                    let b = kentos_geometry_core::entity::entity_bounds(
                        &kentos_native_application::geometry::shape(e),
                    );
                    [b.min_x, b.min_y, b.max_x, b.max_y]
                })
                .collect();
            (n.id.clone(), boxes)
        })
        .collect()
}

fn last_record(app: &App) -> kentos_processing::RunRecord {
    app.gis.processing.runner.history()[0].clone()
}

#[test]
fn ten_metres_in_a_metric_system_make_the_roads_corridor_and_the_record_says_how() {
    let mut app = open(5256);
    tampon(&mut app, "yol", 10.0, "Tampon");
    run(&mut app);
    let made = results(&app, "Tampon");
    assert_eq!(made.len(), 1, "one result layer");
    let [b] = made[0].1[..] else {
        panic!("one corridor: {made:?}")
    };
    // The capsule around the 100 m axis: 10 m past each end and on each side.
    let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
    assert!(
        close(b[2] - b[0], 120.0) && close(b[3] - b[1], 20.0),
        "{b:?}"
    );
    let r = last_record(&app);
    assert_eq!(r.status, Status::Ok);
    assert_eq!(r.records_in, 1);
    assert!(
        r.words
            .iter()
            .any(|w| w.ends_with("“Yol ekseni” katmanında")),
        "{:?}",
        r.words
    );
    assert!(
        r.words.iter().any(|w| w == "Uzaklık: 10 m"),
        "{:?}",
        r.words
    );
    assert!(
        r.words
            .iter()
            .any(|w| w.ends_with("“Tampon” (yeni katman)")),
        "{:?}",
        r.words
    );
}

#[test]
fn ten_metres_in_a_geographic_system_are_refused_and_nothing_is_made() {
    let mut app = open(4326);
    let before = app.document.as_ref().map_or(0, |d| d.model.len());
    tampon(&mut app, "yol", 10.0, "Tampon");
    let w = app.gis.processing.dialog.as_ref().expect("the window");
    assert!(
        w.issues
            .iter()
            .any(|i| i.message.contains("Projenin sistemi coğrafi")),
        "{:?}",
        w.issues
    );
    run(&mut app);
    assert!(results(&app, "Tampon").is_empty(), "no result layer");
    assert_eq!(app.document.as_ref().map_or(0, |d| d.model.len()), before);
}

#[test]
fn durdur_ends_the_run_with_the_drawing_as_it_was_and_the_record_keeps_its_input() {
    let mut app = open(5256);
    let before = app.document.as_ref().map_or(0, |d| d.model.len());
    tampon(&mut app, "yol", 10.0, "Tampon");
    event(&mut app, Event::Target("worker".into()));
    let task = app.update(Message::Gis(gis::Event::Processing(Event::Run)));
    event(&mut app, Event::Stop);
    drive(&mut app, task);
    assert_eq!(
        app.document.as_ref().map_or(0, |d| d.model.len()),
        before,
        "the drawing as it was"
    );
    assert!(results(&app, "Tampon").is_empty());
    let r = last_record(&app);
    assert_eq!(
        (r.status, r.target),
        (Status::Canceled, Some(Target::Worker))
    );
    assert!(
        r.words.iter().any(|w| w == "Uzaklık: 10 m"),
        "{:?}",
        r.words
    );
}

#[test]
fn an_area_crossing_itself_is_buffered_as_drawn_and_said_invalid() {
    let mut app = open(5256);
    tampon(&mut app, "alanlar", 1.0, "Alan tamponu");
    run(&mut app);
    assert_eq!(
        results(&app, "Alan tamponu").first().map(|(_, b)| b.len()),
        Some(1),
        "buffered as drawn"
    );
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|t| t.contains("1 nesnenin geometrisi geçersiz (kendini kesen halka): tamponları çizildiği hâliyle alındı, onarılmadı")
            && t.contains("Geçerliliği denetle")),
        "{said:#?}"
    );
    // The road's line is valid: no such note.
    tampon(&mut app, "yol", 10.0, "Tampon");
    let lines = app.log.lines().count();
    run(&mut app);
    assert!(
        !app.log
            .lines()
            .skip(lines)
            .any(|l| l.text.contains("geçersiz"))
    );
}

#[test]
fn a_result_name_a_layer_has_writes_into_that_layer_and_the_record_says_so() {
    let mut app = open(5256);
    tampon(&mut app, "yol", 10.0, "Tampon");
    run(&mut app);
    tampon(&mut app, "yol", 20.0, "Tampon");
    run(&mut app);
    let made = results(&app, "Tampon");
    assert_eq!(
        made.len(),
        1,
        "the same layer, not a second one named alike: {made:?}"
    );
    assert_eq!(made[0].1.len(), 2, "both corridors in it");
    let r = last_record(&app);
    assert!(
        r.words
            .iter()
            .any(|w| w.ends_with("“Tampon” (var olan katmana yazıldı)")),
        "{:?}",
        r.words
    );
    assert!(
        r.words.iter().any(|w| w == "Uzaklık: 20 m"),
        "{:?}",
        r.words
    );
}

/// Tampon's Yan as it acts (the CBS review's 17th item, docs/adr/9333): the window's Yan with its input's count and what
/// it says.
fn yan(app: &App) -> (Option<(usize, usize)>, Option<usize>, Option<String>) {
    let w = app.gis.processing.dialog.as_ref().expect("the window");
    let side = w
        .tool
        .parameters
        .iter()
        .find(|p| p.name == "side")
        .expect("Yan");
    let kentos_processing::ParamKind::Choice { options } = &side.kind else {
        panic!("Yan is a choice");
    };
    (
        w.acting(side),
        super::fields::only_choice(side, options, w),
        w.acting_note(side),
    )
}

#[test]
fn yan_is_off_for_points_and_the_record_says_it_was_not_used() {
    let mut app = points();
    tampon(&mut app, "noktalar", 10.0, "Tampon");
    event(&mut app, Event::Value("side".into(), json!("left")));
    // Sol and Sağ off: only İki yan is left, and why is said.
    assert_eq!(
        yan(&app),
        (
            Some((0, 3)),
            Some(0),
            Some("Girdide çizgi yok; nesneler iki yandan alınır.".to_owned())
        )
    );
    run(&mut app);
    let made = results(&app, "Tampon");
    let [(_, boxes)] = &made[..] else {
        panic!("one result layer: {made:?}")
    };
    assert_eq!(boxes.len(), 3, "a record for each point: {boxes:?}");
    let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
    for (i, b) in boxes.iter().enumerate() {
        let x = 500_000.0 + 30.0 * i as f64;
        assert!(
            close(b[0], x - 10.0) && close(b[2], x + 10.0),
            "a disc of 10 m about its point: {b:?}"
        );
        assert!(
            close(b[1], 4_399_990.0) && close(b[3], 4_400_010.0),
            "{b:?}"
        );
    }
    let r = last_record(&app);
    assert_eq!((r.status, r.records_in, r.records_out), (Status::Ok, 3, 3));
    assert!(
        r.words
            .iter()
            .any(|w| w == "Yan: kullanılmadı (girdide çizgi yok; nesneler iki yandan alınır)"),
        "{:?}",
        r.words
    );
    assert!(
        !r.words.iter().any(|w| w.starts_with("Yan: Sol")),
        "{:?}",
        r.words
    );
}

#[test]
fn yan_on_points_and_a_line_says_what_it_reaches() {
    let mut app = points();
    tampon(&mut app, "karisik", 10.0, "Tampon");
    // İki yan says nothing more: every object is buffered from both sides.
    assert_eq!(yan(&app), (Some((1, 4)), None, None));
    event(&mut app, Event::Value("side".into(), json!("left")));
    assert_eq!(
        yan(&app),
        (
            Some((1, 4)),
            None,
            Some("Sol yalnız 1 çizgide; öbür 3 nesne iki yandan alınır.".to_owned())
        )
    );
    run(&mut app);
    let made = results(&app, "Tampon");
    let [(_, boxes)] = &made[..] else {
        panic!("one result layer: {made:?}")
    };
    assert_eq!(
        boxes.len(),
        4,
        "three discs and the line's strip: {boxes:?}"
    );
    // The line drawn west to east: its left is the north, its ends flat.
    let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
    assert!(
        boxes.iter().any(|b| close(b[0], 500_000.0)
            && close(b[1], 4_400_050.0)
            && close(b[2], 500_100.0)
            && close(b[3], 4_400_060.0)),
        "{boxes:?}"
    );
    let r = last_record(&app);
    assert!(
        r.words
            .iter()
            .any(|w| w == "Yan: Sol (1 çizgide; öbür 3 nesne iki yandan alınır)"),
        "{:?}",
        r.words
    );
}

/// The picture (`.run/gui-evidence/kullanim-9179/`): İşlemler's Geçmiş after the road's buffer was written twice into
/// “Tampon” and the bow tie's buffer was said invalid, each run with its input, distance and the layer it wrote.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9179", "01-is-kaydi", || {
        let mut app = open(5256);
        tampon(&mut app, "yol", 10.0, "Tampon");
        run(&mut app);
        tampon(&mut app, "yol", 20.0, "Tampon");
        run(&mut app);
        tampon(&mut app, "alanlar", 1.0, "Alan tamponu");
        run(&mut app);
        app.gis.processing.dialog = None;
        app.dialog = None;
        let _ = app.update(Message::Run("processing.history"));
        app
    });
}
