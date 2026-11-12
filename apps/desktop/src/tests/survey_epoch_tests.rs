//! GNSS ağ dengelemesi at the measurement epoch (survey_network_window.rs, plan/map-cad.md MAP-03,
//! docs/adr/2120): on the shared case whose baselines carry their observation times and whose campaign has a velocity
//! grid, the window says the measurement epoch (the baselines' mid-times), the known points moved to it from 2005.0 and
//! how far (Bilinen noktalar's Taşınma); a typed epoch wins; a box that is no number says so; Kabul et keeps the
//! measurement epoch in the adjustment's record. And the pictures.

use kentos_commands::NetworkEpochFrom;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::survey_network_window::{Event as N, View};

const CASE: &str = "Ölçme epoku bazların gözlem zamanlarından (2 Kasım 2026); 2005,0'daki bilinen noktalar hız \
                    ızgarasıyla oraya taşınır, yeni noktalar karara (2005,0) döner";

/// The shared cases' velocity grid and geoid models in the core, as a device's grid library gives them.
pub(crate) fn grids() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geoid/v1");
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let Ok(bytes) = std::fs::read(entry.path()) else { continue };
        if let Ok(model) = kentos_native_application::crs_models::read(&bytes) {
            kentos_geometry_core::crs::geoid::register(&kentos_native_application::crs_models::sha256(&bytes), model.grid);
        }
    }
}

/// The case's drawing: the network with its baselines' times, the campaign with the velocity grid.
fn opened() -> App {
    grids();
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.network.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == CASE).expect("the case").clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    assert_eq!(app.dialog, Some(Dialog::SurveyNetwork));
    app
}

fn answer(app: &App) -> kentos_commands::MapSurveyNetworked {
    app.survey_network.answer.clone().expect("an answer").expect("adjusted")
}

#[test]
fn the_known_points_go_to_the_measurement_epoch_and_the_record_keeps_it() {
    let mut app = opened();
    let a = answer(&app);
    assert_eq!(a.epoch_from, NetworkEpochFrom::Baselines);
    let t = a.epoch.expect("the measurement epoch");
    assert!((t - 2026.837).abs() < 1e-3, "{t}");
    let moved: Vec<&str> = a.moved.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(moved, ["bn-1", "bn-2"]);
    // ~0.54 m in 21.8 years on the synthetic grid: west and north, a little down.
    let m = &a.moved[0];
    assert!((m.years - 21.837).abs() < 1e-3 && m.shift[0] < -0.5 && m.shift[1] > 0.15 && m.shift[2] < 0.0, "{m:?}");
    assert!(a.acceptable, "{:?}", a.why);
    // A typed epoch wins over the baselines' times.
    let _ = app.update(Message::SurveyNetwork(N::Epoch("2026.5".into())));
    let a = answer(&app);
    assert_eq!((a.epoch_from, a.epoch), (NetworkEpochFrom::Given, Some(2026.5)));
    assert!((a.moved[0].years - 21.5).abs() < 1e-9);
    // A box that is no number is said, nothing adjusted.
    let _ = app.update(Message::SurveyNetwork(N::Epoch("abc".into())));
    let why = app.survey_network.answer.clone().expect("an answer").expect_err("refused");
    assert!(why.contains("Ölçme epoku “abc” bir sayı değil"), "{why}");
    // Back to the baselines' times; Kabul et keeps the measurement epoch in the record.
    let _ = app.update(Message::SurveyNetwork(N::Epoch(String::new())));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    let survey = app.document.as_ref().and_then(|d| d.model.map_survey()).cloned().expect("the archive");
    assert_eq!(survey.adjustments.len(), 1);
    assert_eq!(survey.adjustments[0].epoch, Some(t));
    let new = survey.points.iter().filter(|p| p.result.adjustment.as_deref() == Some("da-1")).count();
    assert_eq!(new, 3, "N1, N4, N5");
    assert!(survey.points.iter().all(|p| p.result.target_epoch == Some(2005.0)), "back to the decision");
}

/// The window at the measurement epoch: the summary's epoch line and Bilinen noktalar with Taşınma; light wide and
/// dark narrow, `.run/gui-evidence/ag-epoku-2120/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib survey_epoch_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ag-epoku-2120");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        for (view, file) in [(View::Known, "1-bilinen-noktalar"), (View::Marks, "2-noktalar")] {
            let _ = app.update(Message::SurveyNetwork(N::View(view)));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

const SIMILAR: &str = "Üç bilinen nokta: 15(1)(c) benzerlik dönüşümü; ölçek C1'in ±3 ppm'i içinde, istatistikle uyumlu";
const STRETCHED: &str = "Bilinen noktalar 5 ppm ölçekli: benzerlik dönüşümünün ölçeği C1'in sınırını aşar";

/// The window on one of the shared cases' drawings (Madde 15(1)(c), docs/adr/2160), with class C1.
fn opened_case(name: &str) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.survey.network.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::Class(crate::survey_network_window::Class::C1)));
    app
}

#[test]
fn three_known_points_give_article_15_1_c_s_similarity() {
    let app = opened_case(SIMILAR);
    let a = answer(&app);
    let s = a.similarity.clone().expect("three known points: the similarity");
    assert_eq!((s.known.len(), s.dof), (3, 2));
    assert!(s.scale.abs() < 3.0 && s.agrees, "{s:?}");
    assert_eq!(a.rules.as_ref().and_then(|r| r.similarity), Some(true));
    // Stretched by 5 ppm: the scale passes C1's limit, said in the rules.
    let app = opened_case(STRETCHED);
    let a = answer(&app);
    let s = a.similarity.clone().expect("the similarity");
    assert!(s.scale > 3.0 && !s.agrees, "{s:?}");
    assert_eq!(a.rules.as_ref().map(|r| (r.similarity, r.passed)), Some((Some(false), false)));
}

/// The window with three known points: the summary's similarity line and Bilinen noktalar's residuals, light wide,
/// `.run/gui-evidence/ag-benzerlik-2160/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib survey_epoch_tests::similarity_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn similarity_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ag-benzerlik-2160");
    std::fs::create_dir_all(&out).expect("a folder");
    for (case, file) in [(SIMILAR, "1-uc-bilinen-nokta"), (STRETCHED, "2-olcek-asar")] {
        let size = Size::new(1440.0, 900.0);
        let mut app = opened_case(case);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from("light"))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::SurveyNetwork(N::View(View::Known)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("{file}-acik-genis.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
