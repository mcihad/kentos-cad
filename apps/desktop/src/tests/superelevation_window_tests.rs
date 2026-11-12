//! Dever (superelevation_window.rs, plan/road.md Y6, Y27): the window builds a schedule as it is typed, says why it does
//! not build, and writes through `road.superelevations.edit`, each write one undo step; its axis of rotation and
//! Denetim (docs/adr/0931); and its pictures.

use crate::app::{App, Dialog, Message};
use crate::road;
use crate::superelevation_window::Event as W;

/// A road project with `road.superelevations.query`'s drawing: a surface, the main alignment with one curve (p1), the
/// side alignment, two sample line sets and two schedules (d-1 from the curve, d-2 from rows, stale); or without the
/// schedules.
fn app(schedules: bool) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/road.superelevations.query.json"
    ))
    .expect("cases");
    let mut setup = cases["setup"].clone();
    if !schedules {
        setup["settings"]["road"].as_object_mut().expect("road").remove("superelevations");
    }
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Superelevations(e)));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

/// Dever oluştur on the main alignment: normal −2,5 both sides, the curve's rate, runoff, share and widening.
fn made(app: &mut App) {
    let _ = app.update(Message::Run("superelevation.new"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Superelevations))
    );
    send(app, W::Name("Ana aks deveri".into()));
    send(app, W::Normal(0, "-2,5".into()));
    send(app, W::Normal(1, "-2,5".into()));
    send(app, W::AllCurves);
    send(app, W::Cell(0, 1, "6".into()));
    send(app, W::Cell(0, 2, "40".into()));
    send(app, W::Cell(0, 3, "0,3".into()));
    send(app, W::Cell(0, 4, "0,5".into()));
}

#[test]
fn the_window_makes_a_schedule_and_shows_its_diagram() {
    let mut app = app(false);
    made(&mut app);
    let w = app.road.superelevation_window.as_ref().expect("open");
    let report = w.report.as_ref().expect("built").as_ref().expect("builds");
    assert_eq!(report.critical.len(), 8, "NC, LC, RC and FS in and out");
    assert!(
        report.right.iter().any(|p| (p[1] - 0.06).abs() < 1e-9),
        "the left turn puts the right side outside"
    );
    assert!(report.left.iter().any(|p| (p[1] + 0.06).abs() < 1e-9));
    assert!(!report.left_widening.is_empty(), "the inside widens");
    send(&mut app, W::Save);
    assert!(
        app.road.superelevation_window.is_none(),
        "Kaydet closes the window"
    );
    let doc = model(&mut app);
    assert_eq!(doc.road_superelevations().len(), 1);
    assert_eq!(doc.road_superelevations()[0].id, "d-1");
    assert_eq!(doc.undo().as_deref(), Some("Dever"));
    assert!(doc.road_superelevations().is_empty());
}

#[test]
fn a_rate_below_the_normal_slope_says_why_and_a_blank_is_refused() {
    let mut app = app(false);
    made(&mut app);
    send(&mut app, W::Cell(0, 1, "2".into()));
    let w = app.road.superelevation_window.as_ref().expect("open");
    let why = w
        .report
        .as_ref()
        .expect("built")
        .as_ref()
        .expect_err("does not build");
    assert!(why.contains("p1"), "names the curve: {why}");
    // A blank rate is not a number; the window stays and nothing is written.
    send(&mut app, W::Cell(0, 1, String::new()));
    send(&mut app, W::Save);
    assert!(app.road.superelevation_window.is_some());
    assert!(model(&mut app).road_superelevations().is_empty());
}

#[test]
fn a_stale_schedule_is_rebound_and_a_used_one_stays() {
    let mut app = app(true);
    let _ = app.update(Message::Run("superelevation.editor"));
    send(&mut app, W::Choose(1));
    send(&mut app, W::Rebind);
    let doc = model(&mut app);
    assert_eq!(doc.road_superelevations()[1].alignment_revision, 3);
    assert_eq!(doc.undo().as_deref(), Some("Dever"));
    // A set on the main alignment takes d-1 in Enkesit; then d-1 cannot be removed.
    let _ = app.update(Message::Road(road::Event::Superelevations(W::Cancel)));
    let _ = app.update(Message::Run("section.editor"));
    let _ = app.update(Message::Road(road::Event::Sections(
        crate::section_window::Event::Choose(crate::section_window::Chosen::Set(0)),
    )));
    let _ = app.update(Message::Road(road::Event::Sections(
        crate::section_window::Event::Superelevation(Some("d-1".into())),
    )));
    let _ = app.update(Message::Road(road::Event::Sections(
        crate::section_window::Event::Save,
    )));
    assert_eq!(
        model(&mut app).road_sections()[0].superelevation.as_deref(),
        Some("d-1")
    );
    let _ = app.update(Message::Run("superelevation.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Remove);
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_superelevations().len(), 2, "the set's schedule stays");
}

#[test]
fn the_axis_is_written_and_denetim_reads_the_schedule_as_typed() {
    use kentos_commands::SuperelevationPivot as P;
    let mut app = app(false);
    made(&mut app);
    let w = app.road.superelevation_window.as_ref().expect("open");
    assert_eq!(
        w.check_template.as_deref(),
        Some("tk-1"),
        "the first template is the one read"
    );
    let c = w
        .checked
        .as_ref()
        .expect("checked")
        .as_ref()
        .expect("reads");
    assert!(c.curves.is_empty(), "no speed, no friction");
    assert!(c.gradients.iter().any(|g| g.left > 0.0 || g.right > 0.0), "the slopes change on the runoff");
    // 80 km/h, friction at most 0,05 and 1:400: the limits break, before anything is saved.
    send(&mut app, W::Check(0, "80".into()));
    send(&mut app, W::Check(1, "0,05".into()));
    send(&mut app, W::Check(2, "400".into()));
    let c = app
        .road
        .superelevation_window
        .as_ref()
        .unwrap()
        .checked
        .clone()
        .expect("checked")
        .expect("reads");
    assert_eq!(c.curves.len(), 1);
    assert!(c.findings.len() >= 2, "{:?}", c.findings);
    assert!(
        model(&mut app).road_superelevations().is_empty(),
        "Denetim writes nothing"
    );
    // About the inside edge the edge's gradient doubles over the centreline's.
    let before = c
        .gradients
        .iter()
        .map(|g| g.left.max(g.right))
        .fold(0.0, f64::max);
    send(&mut app, W::Pivot(Some(P::InsideEdge)));
    let c = app
        .road
        .superelevation_window
        .as_ref()
        .unwrap()
        .checked
        .clone()
        .expect("checked")
        .expect("reads");
    let after = c
        .gradients
        .iter()
        .map(|g| g.left.max(g.right))
        .fold(0.0, f64::max);
    assert!(after > 1.5 * before, "{before} → {after}");
    // n of 1:n at most 1 says why.
    send(&mut app, W::Check(2, "1".into()));
    let why = app
        .road
        .superelevation_window
        .as_ref()
        .unwrap()
        .checked
        .clone()
        .expect("checked")
        .expect_err("refused");
    assert!(why.contains("1:200"), "{why}");
    send(&mut app, W::Check(2, "400".into()));
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_superelevations()[0].pivot, Some(P::InsideEdge));
    // The centreline is stored as no field.
    let _ = app.update(Message::Run("superelevation.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Pivot(None));
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_superelevations()[0].pivot, None);
}

/// The window over the drawing, wide and narrow, light and dark; `.run/gui-evidence/yol-y6-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib superelevation_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app(true);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = if let Some(file) = file.strip_prefix("../") {
                out.join(format!("{file}-{name}.png"))
            } else {
                out.join(format!("yol-y6-{file}-{name}.png"))
            };
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("superelevation.editor"));
        shot(&mut app, "dever");
        // The stale side schedule from rows.
        send(&mut app, W::Choose(1));
        shot(&mut app, "dever-satirlar");
        // A rate below the normal slope: why it does not build.
        send(&mut app, W::Choose(0));
        send(&mut app, W::Cell(0, 1, "2".into()));
        shot(&mut app, "dever-red");
        send(&mut app, W::Cancel);
        // Y27.3: the axis of rotation and Denetim on the main schedule, 50 km/h with AASHTO's limits there (side
        // friction 0,16, relative gradient 1:154 written 150) as an example; the 40 m runoff is too steep about the inside edge.
        let _ = app.update(Message::Run("superelevation.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Pivot(Some(kentos_commands::SuperelevationPivot::InsideEdge)));
        send(&mut app, W::Check(0, "50".into()));
        send(&mut app, W::Check(1, "0,16".into()));
        send(&mut app, W::Check(2, "150".into()));
        let y27 = |f: &str| format!("../yol-y26/y27-3-{f}");
        shot(&mut app, &y27("dever-eksen"));
        send(&mut app, W::List(crate::superelevation_window::Listed::Gradients));
        shot(&mut app, &y27("dever-goreli"));
        send(&mut app, W::List(crate::superelevation_window::Listed::Findings));
        shot(&mut app, &y27("dever-bulgular"));
        // A longer runoff: the gradient falls under the limit, the findings go.
        send(&mut app, W::Cell(0, 2, "80".into()));
        send(&mut app, W::List(crate::superelevation_window::Listed::Friction));
        shot(&mut app, &y27("dever-surtunme"));
        send(&mut app, W::Cancel);
        if name == "acik-genis" {
            // Enkesit's set takes the schedule; the lanes turn with it on the curve.
            let _ = app.update(Message::Run("section.editor"));
            let _ = app.update(Message::Road(road::Event::Sections(
                crate::section_window::Event::Choose(crate::section_window::Chosen::Set(0)),
            )));
            let _ = app.update(Message::Road(road::Event::Sections(
                crate::section_window::Event::Superelevation(Some("d-1".into())),
            )));
            let _ = app.update(Message::Road(road::Event::Sections(
                crate::section_window::Event::Save,
            )));
            let _ = app.update(Message::Run("section.editor"));
            let _ = app.update(Message::Road(road::Event::Sections(
                crate::section_window::Event::Choose(crate::section_window::Chosen::Set(0)),
            )));
            for _ in 0..6 {
                let _ = app.update(Message::Road(road::Event::Sections(
                    crate::section_window::Event::Next,
                )));
            }
            shot(&mut app, "enkesit-dever");
        }
    }
}

const EQUATION_CASE: &str = "Km eşitliği geçişin içinde: kritik km'ler iki yanında";

/// The query case's own drawing: the main alignment with a km equation inside the transition.
fn equation_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.superelevations.query.json")).expect("cases");
    let c = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == EQUATION_CASE).expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(c["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The critical stations' table writes the km as written (docs/adr/0973): before the equation (internal 200, back
/// 1+200, ahead 1+500) the start's base, after it the ahead one; the internal station is not the km.
#[test]
fn the_critical_table_writes_the_km_across_an_equation() {
    let mut app = equation_app();
    let input = kentos_commands::SuperelevationsQuery { superelevation: Some("d-1".into()), stations: vec![] };
    let kentos_commands::CommandResult::Completed { output, .. } =
        kentos_native_application::road_superelevations_query::execute(&mut kentos_native_application::ExecutionContext::new(model(&mut app)), input)
    else {
        panic!("the query reads")
    };
    let rows = crate::superelevation_window::critical_rows(&output.superelevations[0].critical);
    let kms: Vec<&str> = rows.iter().map(|r| r[2].as_str()).collect();
    assert_eq!(&kms[..4], &["1+178,046", "1+194,713", "1+511,379", "1+534,713"]);
}

/// `.run/gui-evidence/yol-y68/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib superelevation_window_tests::screens_equation -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_equation() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y68");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = equation_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("superelevation.editor"));
        send(&mut app, W::Choose(0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y68-km-esitligi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Denetim with the speed left empty (docs/adr/0986): the main alignment at 70 km/h, each curve's friction at its own
/// speed; `.run/gui-evidence/yol-y80/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib superelevation_window_tests::screens_own_speed -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_own_speed() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y80");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app(true);
        let al = model(&mut app).road_superelevations()[0].alignment.clone();
        let edit = kentos_commands::AlignmentsEdit {
            alignment: al,
            operation: kentos_commands::AlignmentsEditOperation::Set,
            name: None,
            pis: None,
            stationing: None,
            offset: None,
            design_speed: Some(70.0),
            start_place: None,
            end_place: None,
            expected_revision: None,
        };
        let done = kentos_native_application::road_alignments_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(model(&mut app)),
            edit,
        );
        assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("superelevation.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Check(1, "0,10".into()));
        send(&mut app, W::List(crate::superelevation_window::Listed::Friction));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y80-surtunme-kendi-hizi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, W::List(crate::superelevation_window::Listed::Findings));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y80-bulgular-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
