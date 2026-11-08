//! Arşivden ağ dengelemesi (survey_network_window.rs, docs/adr/1740) on the shared cases' drawing of
//! `map.survey.network`: the window opens with the network adjusted as the shared case says; Kabul et writes it in one
//! undo step and the drawing saves as `.kcad` schema 199; Dışla leaves a baseline out and Geri al takes it back; a GVX
//! file and the known point form fill an empty archive; Harita offers it, GIS and Mimari do not. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::survey_network_window::{Class, Event as N, Field, View};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/map.survey.network.json")).expect("cases")
}

fn opened_with(setup: Value, program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn opened(program: Option<Module>) -> App {
    opened_with(cases()["setup"].clone(), program)
}

fn survey(app: &App) -> kentos_commands::MapSurvey {
    app.document.as_ref().and_then(|d| d.model.map_survey()).cloned().unwrap_or_default()
}

#[test]
fn the_window_shows_the_shared_case_s_network_and_kabul_et_writes_it_in_one_step() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    assert_eq!(app.dialog, Some(Dialog::SurveyNetwork));
    let a = app.survey_network.answer.clone().expect("an answer").expect("adjusted");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    assert_eq!(Value::from(a.acceptable), want["acceptable"]);
    assert_eq!(Value::from(a.free.dof), want["free"]["dof"]);
    assert!((a.constrained.m0.expect("m0") - want["constrained"]["m0"].as_f64().expect("m0")).abs() < 1e-6, "the shared cases' tolerance");
    let marks: Vec<&str> = a.stations.iter().map(|s| s.mark.as_str()).collect();
    assert_eq!(marks, ["BN1", "N1", "BN2", "N4", "N5"]);
    let _ = app.update(Message::SurveyNetwork(N::Class(Class::C2)));
    let r = app.survey_network.answer.clone().expect("an answer").expect("adjusted");
    assert_eq!(r.rules.as_ref().map(|r| (r.class.as_str(), r.passed)), Some(("C2", true)));
    let _ = app.update(Message::SurveyNetwork(N::Accept));
    let s = survey(&app);
    assert_eq!(s.adjustments.len(), 1);
    assert_eq!(s.adjustments[0].class.as_deref(), Some("C2"));
    let points: Vec<(&str, u32)> = s.points.iter().map(|p| (p.mark.as_str(), p.result.revision)).collect();
    assert_eq!(points, [("N1", 2), ("N4", 1), ("N5", 1)], "N1 made again, N4 and N5 accepted");
    assert_eq!(s.points[0].history.len(), 1, "N1's older result kept");
    // The drawing saves as schema 199 and reads back the same archive.
    let doc = app.document.as_ref().expect("a drawing");
    let bytes = kentos_kcad::encode(&doc.model.to_snapshot_v2()).expect("saves");
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(back.settings.map_survey.as_ref(), Some(&s));
    // One undo step takes it all away.
    let _ = app.update(Message::Run("edit.undo"));
    let s = survey(&app);
    assert!(s.adjustments.is_empty());
    assert_eq!(s.points.len(), 1);
    assert_eq!(s.points[0].result.revision, 1);
}

#[test]
fn disla_leaves_a_baseline_out_and_geri_al_takes_it_back() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    let _ = app.update(Message::SurveyNetwork(N::View(View::Baselines)));
    let _ = app.update(Message::SurveyNetwork(N::Exclude("bz-8".into())));
    let s = survey(&app);
    let b = s.baselines.iter().find(|b| b.id == "bz-8").expect("kept");
    assert!(b.excluded.as_deref().is_some_and(|r| r.contains("w = ")), "{:?}", b.excluded);
    let a = app.survey_network.answer.clone().expect("an answer").expect("adjusted");
    assert_eq!(a.baselines.len(), 7, "seven go in");
    let _ = app.update(Message::SurveyNetwork(N::Include("bz-8".into())));
    let a = app.survey_network.answer.clone().expect("an answer").expect("adjusted");
    assert_eq!(a.baselines.len(), 8);
}

#[test]
fn a_gvx_file_and_the_known_point_form_fill_an_empty_archive() {
    let query: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/map.survey.query.json")).expect("cases");
    let mut app = opened_with(query["setup"].clone(), Some(Module::Map));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    assert!(matches!(&app.survey_network.answer, Some(Err(_))), "no baselines yet");
    let gvx = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/map-survey/v1/ag.gvx");
    app.survey_network_file(gvx);
    assert_eq!(survey(&app).baselines.len(), 8);
    let why = app.survey_network.answer.clone().expect("an answer").expect_err("no known point");
    assert!(why.starts_with("Ağın bilinen noktası yok"), "{why}");
    let known = &cases()["setup"]["settings"]["mapSurvey"]["known"][0];
    for (field, value) in [
        (Field::Mark, known["mark"].as_str().expect("mark").to_owned()),
        (Field::Lat, known["lat"].to_string()),
        (Field::Lon, known["lon"].to_string()),
        (Field::Height, known["height"].to_string()),
        (Field::Epoch, known["epoch"].to_string()),
        (Field::Source, "TUSAGA-Aktif (örnek)".to_owned()),
    ] {
        let _ = app.update(Message::SurveyNetwork(N::Known(field, value)));
    }
    let _ = app.update(Message::SurveyNetwork(N::AddKnown));
    assert_eq!(survey(&app).known.len(), 1);
    // The GVX file's vectors carry their GPS times: the measurement epoch is 2026's, the station's coordinate 2005.0's;
    // BÖHHBÜY Madde 15(1)(a) wants it moved there, so the campaign needs its velocity grid (docs/adr/2120).
    let why = app.survey_network.answer.clone().expect("an answer").expect_err("the station is not moved");
    assert!(why.contains("Madde 15(1)(a) uyarınca ölçme epokuna taşınmalı"), "{why}");
    crate::survey_epoch_tests::grids();
    let bytes = std::fs::read(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/geoid/v1/hiz.tif"))
        .expect("the velocity grid");
    let velocity = kentos_commands::SurveyGrid {
        name: "Hız alanı (sentetik)".into(),
        sha256: kentos_native_application::crs_models::sha256(&bytes),
    };
    let campaign = survey(&app).campaigns[0].id.clone();
    let _ = app.map_survey_edit(kentos_commands::MapSurveyOperation::SetCampaign {
        campaign,
        name: None,
        work: None,
        operator: None,
        epoch: None,
        velocity: Some(velocity),
    });
    app.survey_network_run();
    let a = app.survey_network.answer.clone().expect("an answer").expect("adjusted");
    assert!(a.acceptable, "{:?}", a.why);
    assert_eq!(a.free.held, ["BN1"]);
    assert_eq!(a.moved.len(), 1, "the station moved to the measurement epoch");
}

#[test]
fn a_known_point_s_sigma_is_three_numbers_or_none() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.survey.adjustment"));
    for (field, value) in [(Field::Mark, "N9"), (Field::Lat, "39.8"), (Field::Lon, "32.7"), (Field::Height, "900"), (Field::Sigma(0), "0.01")] {
        let _ = app.update(Message::SurveyNetwork(N::Known(field, value.into())));
    }
    let before = survey(&app).known.len();
    let _ = app.update(Message::SurveyNetwork(N::AddKnown));
    assert_eq!(survey(&app).known.len(), before, "refused, nothing written");
}

#[test]
fn harita_offers_it_gis_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Gis, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("map.survey.adjustment"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/ag-dengelemesi-1740/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib survey_network_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ag-dengelemesi-1740");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("map.survey.adjustment"));
        let _ = app.update(Message::SurveyNetwork(N::Class(Class::C2)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        for (view, file) in [(View::Marks, "1-noktalar"), (View::Baselines, "2-bazlar"), (View::Known, "3-bilinen")] {
            let _ = app.update(Message::SurveyNetwork(N::View(view)));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
        // Kabul et, then the archive panel and the drawing.
        let _ = app.update(Message::SurveyNetwork(N::View(View::Marks)));
        let _ = app.update(Message::SurveyNetwork(N::Accept));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("4-kabul-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::SurveyNetwork(N::Close));
        let _ = app.update(Message::Run("map.survey"));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("5-arsiv-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // The ribbon's place: Ölçme › GNSS arşivi.
        let _ = app.update(Message::RibbonTab("survey"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("6-serit-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
