//! Kutupsal alım with each shot at its own line's height (calc/polar.rs, plan/map-cad.md MAP-02, docs/adr/2900): in the
//! sample's TM36 project at a mean ellipsoidal height of 1100 m, its heights orthometric with the TG-20 geoid (the
//! shared cases' synthetic grid), a station with a height and shots with zenith angles are reduced at their ends' mean
//! ellipsoidal height; the window's factors are `cad.crs.factors`' for the same line from the drawing's heights, and
//! Çizime ekle writes the points the window shows. A shot without a zenith angle stays at the mean; a project that does
//! not say what its heights are reduces at the mean and the window says why. docs/adr/3300: Aplikasyon takes each line
//! to a target with a height at its own height the same way. And the pictures.

use kentos_commands::{Entity, GeoidModel, HeightKind, HeightReference, SurveySettings};
use kentos_geometry_core::survey::polar::PolarPoint;
use serde_json::json;

use crate::app::{App, Message};
use crate::calc::polar::{DISTANCE, NAME, READING, ZENITH};
use crate::calc::{Event, Field};
use crate::files_testing::app_with_drawing;

fn calc(app: &mut App, e: Event) {
    let _ = app.update(Message::Calc(e));
}

/// The shared cases' TG-20 grid in this process's library, under its SHA-256.
fn tg() -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geoid/v1/tg.gtx");
    let bytes = std::fs::read(path).expect("the grid");
    let id = kentos_native_application::crs_models::sha256(&bytes);
    let model = kentos_native_application::crs_models::read(&bytes).expect("reads");
    kentos_geometry_core::crs::geoid::register(&id, model.grid);
    id
}

fn orthometric(id: &str) -> HeightReference {
    HeightReference {
        kind: HeightKind::Orthometric,
        vertical_srid: Some(5775),
        geoid: Some(GeoidModel { name: "TG-20 (sentetik)".into(), sha256: Some(id.into()) }),
    }
}

/// The sample at 1100 m with Uzunlukları projeksiyona indir, its heights as given.
fn project(app: &mut App, heights: Option<HeightReference>) {
    let doc = app.document.as_mut().expect("open");
    let mut settings = doc.settings().clone();
    settings.survey =
        Some(SurveySettings { ground_height: Some(1100.0), reduce_to_grid: Some(true), ..Default::default() });
    settings.heights = heights;
    doc.model.set_settings(settings);
}

/// Kutupsal alım from 486 500, 4 420 200 (height 1100) oriented north: K1 up the hill and K2 down it, slope lengths
/// with zenith angles, K3 level without one.
fn shots(app: &mut App) {
    let _ = app.run("calc.polar");
    calc(app, Event::Known(Field::Station, "486500,4420200".into()));
    calc(app, Event::Known(Field::Back, "486500,4420700".into()));
    calc(app, Event::StationZ("1100".into()));
    for (row, (name, reading, distance, zenith)) in
        [("K1", "100", "1250.4", "88.7"), ("K2", "250", "930.25", "104.2"), ("K3", "340", "610", "")]
            .into_iter()
            .enumerate()
    {
        calc(app, Event::Cell(row, NAME, name.into()));
        calc(app, Event::Cell(row, READING, reading.into()));
        calc(app, Event::Cell(row, DISTANCE, distance.into()));
        calc(app, Event::Cell(row, ZENITH, zenith.into()));
    }
}

fn computed(app: &App) -> (Vec<PolarPoint>, Option<Result<Option<String>, String>>) {
    let doc = app.document.as_ref().expect("open");
    let read = app.calc.polar.compute(&doc.model);
    (read.points.expect("computed"), read.heights)
}

fn factors(app: &mut App, input: serde_json::Value) -> serde_json::Value {
    let doc = app.document.as_mut().expect("open");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.crs.factors",
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    answer["output"].clone()
}

#[test]
fn each_shot_is_reduced_at_its_own_line_s_height() {
    let id = tg();
    let mut app = app_with_drawing();
    project(&mut app, Some(orthometric(&id)));
    shots(&mut app);
    let (points, heights) = computed(&app);
    assert_eq!(heights, Some(Ok(Some("TG-20 (sentetik)".to_owned()))));
    // Each line's factors are cad.crs.factors' for the same line from the drawing's heights, through the same geoid.
    for p in &points[..2] {
        let z = p.z.expect("a height from the zenith angle");
        let out = factors(
            &mut app,
            json!({"points": [[486500.0, 4420200.0], [p.p.x, p.p.y]], "elevations": [1100.0, z], "lines": [[0, 1]]}),
        );
        let line = &out["lines"][0];
        let near = |a: f64, b: &serde_json::Value, t: f64| assert!((a - b.as_f64().expect("a number")).abs() < t, "{a} ≠ {b}");
        near(p.scale.expect("a scale"), &line["scale"], 1e-12);
        near(p.height_factor.expect("a factor"), &line["heightFactor"], 1e-12);
        near(p.height.expect("its height"), &line["height"], 1e-6);
        assert_eq!(out["geoid"], "TG-20 (sentetik)");
        // Its height is its ends' ellipsoidal heights' mean: the heights plus N, some 30 m here.
        let n = out["points"][0]["undulation"].as_f64().expect("N at the station");
        assert!((25.0..40.0).contains(&n), "{n}");
        assert!((p.height.expect("its height") - (1100.0 + z) / 2.0 - n).abs() < 0.5, "{p:?}");
    }
    assert!(points[0].height > points[1].height, "uphill above downhill");
    assert_eq!(points[2].height, Some(1100.0), "a level shot at the mean");
    // Çizime ekle: cad.survey.points writes the points the window shows.
    calc(&mut app, Event::AddPoints);
    let doc = app.document.as_ref().expect("open");
    let mut written: Vec<(f64, f64)> = app
        .selection
        .ids()
        .iter()
        .filter_map(|&s| match doc.model.get(s) {
            Some(Entity::Point(p)) => Some((p.p.x, p.p.y)),
            _ => None,
        })
        .collect();
    written.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut shown: Vec<(f64, f64)> = points.iter().map(|p| (p.p.x, p.p.y)).collect();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(written.len(), 3);
    for (w, s) in written.iter().zip(&shown) {
        assert!((w.0 - s.0).abs() < 1e-9 && (w.1 - s.1).abs() < 1e-9, "{w:?} ≠ {s:?}");
    }
}

#[test]
fn without_the_project_s_heights_the_window_says_why_it_reduces_at_the_mean() {
    let mut app = app_with_drawing();
    project(&mut app, None);
    shots(&mut app);
    let (points, heights) = computed(&app);
    let why = heights.expect("asked").expect_err("not made");
    assert!(why.contains("söylenmemiş"), "{why}");
    assert!(points.iter().all(|p| p.height == Some(1100.0)), "{points:?}");
    let doc = app.document.as_ref().expect("open");
    let format = kentos_interaction::Format::of(doc.settings());
    let report = app.calc.polar.report(&doc.model, &format).expect("a report");
    assert!(!report[1].contains(&"Elipsoit yüksekliği".to_owned()), "no height column at the mean");
    // With them, the report has the column.
    let id = tg();
    project(&mut app, Some(orthometric(&id)));
    let doc = app.document.as_ref().expect("open");
    let report = app.calc.polar.report(&doc.model, &format).expect("a report");
    assert_eq!(report[1].last().map(String::as_str), Some("Elipsoit yüksekliği"));
}

/// Kutupsal alım with its h column and the note, light wide and dark narrow, `.run/gui-evidence/kendi-yuksekligi-2900/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib survey_heights_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kendi-yuksekligi-2900");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let id = tg();
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        project(&mut app, Some(orthometric(&id)));
        shots(&mut app);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("kutupsal-alim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// docs/adr/3300: Aplikasyon from the drawing's station point (height 1100) to its target points: a target with a
/// height is taken at its line's own height, the window's factors are `cad.crs.factors`' for the same line; a target
/// without a height, and one typed as coordinates, stay at the mean. The report has the height column.
#[test]
fn each_stakeout_line_is_taken_at_its_own_height() {
    let id = tg();
    let mut app = app_with_drawing();
    project(&mut app, Some(orthometric(&id)));
    let doc = app.document.as_mut().expect("open");
    let layer = doc.model.layers().leaves().first().map(|l| l.id.clone()).expect("a layer");
    let made = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.entities.create",
        kentos_commands::CallOp::Execute,
        json!({"layerId": layer, "objects": [
            {"geometry": {"kind": "point", "p": {"x": 486500.0, "y": 4420200.0}, "z": 1100.0}, "attrs": {"Ad": "S1"}},
            {"geometry": {"kind": "point", "p": {"x": 487700.0, "y": 4420550.0}, "z": 1180.5}, "attrs": {"Ad": "T1"}},
            {"geometry": {"kind": "point", "p": {"x": 486100.0, "y": 4419300.0}}, "attrs": {"Ad": "T2"}}
        ]}),
    )
    .expect("runs");
    assert_eq!(made["status"], "completed", "{made}");
    let _ = app.run("calc.stakeout");
    calc(&mut app, Event::Known(Field::Station, "S1".into()));
    calc(&mut app, Event::Known(Field::Back, "486500,4420700".into()));
    for (row, target) in ["T1", "T2", "486800,4419900"].into_iter().enumerate() {
        calc(&mut app, Event::Cell(row, 0, target.into()));
    }
    let doc = app.document.as_ref().expect("open");
    let read = app.calc.stakeout.compute(&doc.model);
    assert_eq!(read.heights, Some(Ok(Some("TG-20 (sentetik)".to_owned()))));
    let stakes = read.stakes.expect("computed");
    let out = factors(
        &mut app,
        json!({"points": [[486500.0, 4420200.0], [487700.0, 4420550.0]], "elevations": [1100.0, 1180.5], "lines": [[0, 1]]}),
    );
    let line = &out["lines"][0];
    let near = |a: f64, b: &serde_json::Value, t: f64| assert!((a - b.as_f64().expect("a number")).abs() < t, "{a} ≠ {b}");
    near(stakes[0].scale.expect("a scale"), &line["scale"], 1e-12);
    near(stakes[0].height_factor.expect("a factor"), &line["heightFactor"], 1e-12);
    near(stakes[0].height.expect("its height"), &line["height"], 1e-6);
    assert!(stakes[0].height.expect("its height") > 1100.0 + 25.0, "the heights plus N");
    assert_eq!((stakes[1].height, stakes[2].height), (Some(1100.0), Some(1100.0)), "no target height: the mean");
    let doc = app.document.as_ref().expect("open");
    let format = kentos_interaction::Format::of(doc.settings());
    let report = app.calc.stakeout.report(&doc.model, &format).expect("a report");
    assert_eq!(report[1].last().map(String::as_str), Some("Elipsoit yüksekliği"));
}

/// Aplikasyon with its h column and the note, `.run/gui-evidence/aplikasyon-yuksekligi-3300/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib survey_heights_tests::stakeout_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn stakeout_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/aplikasyon-yuksekligi-3300");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let id = tg();
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app_with_drawing();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        project(&mut app, Some(orthometric(&id)));
        let doc = app.document.as_mut().expect("open");
        let layer = doc.model.layers().leaves().first().map(|l| l.id.clone()).expect("a layer");
        let _ = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            "cad.entities.create",
            kentos_commands::CallOp::Execute,
            json!({"layerId": layer, "objects": [
                {"geometry": {"kind": "point", "p": {"x": 486500.0, "y": 4420200.0}, "z": 1100.0}, "attrs": {"Ad": "S1"}},
                {"geometry": {"kind": "point", "p": {"x": 487700.0, "y": 4420550.0}, "z": 1180.5}, "attrs": {"Ad": "T1"}},
                {"geometry": {"kind": "point", "p": {"x": 485800.0, "y": 4421300.0}, "z": 1312.0}, "attrs": {"Ad": "T2"}},
                {"geometry": {"kind": "point", "p": {"x": 486100.0, "y": 4419300.0}}, "attrs": {"Ad": "T3"}}
            ]}),
        );
        let _ = app.run("calc.stakeout");
        calc(&mut app, Event::Known(Field::Station, "S1".into()));
        calc(&mut app, Event::Known(Field::Back, "486500,4420700".into()));
        for (row, target) in ["T1", "T2", "T3"].into_iter().enumerate() {
            calc(&mut app, Event::Cell(row, 0, target.into()));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("aplikasyon-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
