//! Atlamalı nivelman in Kot ağı dengelemesi (plan/map-cad.md MAP-02, docs/adr/5300): the leap-frog table shows with
//! Trigonometrik, each setup's height difference beside its row is the command's (the shared case's), Çizime yaz gives
//! N1 and N2 the case's heights; with Geometrik the setups are left out. And the area of use map of Proje ayarları ›
//! Koordinat sistemi (TUREF / TM30 with ED50 / TM30 as the second system). And the pictures.

use serde_json::Value;

use super::{Cells, HEIGHT_COLS, LEAPFROG_COLS, LevelKind};
use crate::app::{App, Message};
use crate::calc::{Event as CalcEvent, Window};

/// The shared case's leap-frog adjustment (fixtures/commands/v1/cad.survey.adjust.json).
fn shared() -> Value {
    let all: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.survey.adjust.json")).expect("reads");
    all["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Atlamalı nivelman: iki reper")))
        .cloned()
        .expect("the case")
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Kot ağı with the case's known points by name and its setups typed.
fn leapfrog_with(app: &mut App, kind: LevelKind) -> Value {
    let c = shared();
    let _ = app.update(Message::Run("calc.levelNetwork"));
    assert_eq!(app.calc.open, Some(Window::Level));
    app.calc.level.kind = kind;
    app.calc.level.known = Cells::of(HEIGHT_COLS, vec![vec!["R1".into()], vec!["R2".into()]]);
    let work = &c["steps"][0]["input"]["work"];
    let rows = work["leapfrog"]
        .as_array()
        .expect("setups")
        .iter()
        .map(|s| {
            let mut r = vec![text(&s["from"]), text(&s["to"])];
            for side in ["back", "fore"] {
                for f in ["distance", "zenith", "target"] {
                    r.push(text(&s[side][f]));
                }
            }
            r
        })
        .collect();
    app.calc.level.leapfrog = Cells::of(LEAPFROG_COLS, rows);
    app.network_solve();
    c
}

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
        "../../../../../fixtures/interaction/v1/network-adjust.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

#[test]
fn the_setups_adjust_as_the_command_and_write_their_heights() {
    let mut app = app();
    let c = leapfrog_with(&mut app, LevelKind::Trigonometric);
    let out = &c["steps"][0]["result"]["output"];
    let Some(Ok(r)) = app.calc.level.solved.result.clone() else {
        panic!("adjusted: {:?}", app.calc.level.solved);
    };
    assert_eq!(r.f, 1);
    for (got, want) in app.calc.level.leapfrog_dh.iter().zip(out["leapfrog"].as_array().expect("differences")) {
        assert!((got - want["dh"].as_f64().unwrap_or(f64::NAN)).abs() < 1e-9, "{got} {want}");
    }
    let _ = app.update(Message::Calc(CalcEvent::AddPoints));
    let doc = app.document.as_ref().expect("a drawing");
    for p in out["points"].as_array().expect("points") {
        let name = text(&p["name"]);
        let z = super::named_points(&doc.model, &name).first().and_then(|q| q.z).expect("written");
        assert!((z - p["h"].as_f64().unwrap_or(f64::NAN)).abs() < 1e-6, "{name}");
    }
}

#[test]
fn geometric_levelling_leaves_the_setups_out() {
    let mut app = app();
    let _ = leapfrog_with(&mut app, LevelKind::Geometric);
    // Only the setups tie N1 and N2: without them nothing ties a new point.
    assert!(app.calc.level.leapfrog_dh.iter().all(|d| d.is_nan()));
}

#[test]
fn the_area_map_reads_the_system_s_and_the_operation_s_polygons() {
    let map = crate::project::area_map_of(5254, Some(2320));
    assert_eq!(map.0, Some(1525), "TM30's land");
    assert_eq!(map.1, Some(1237), "the datum operations' Türkiye");
    assert_eq!(crate::project::area_map_of(5254, Some(5255)).1, None, "the same datum");
}

/// `.run/gui-evidence/atlamali-nivelman-5300/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib calc::network::leapfrog_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/atlamali-nivelman-5300");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, w, h, name) in [("light", 1440.0, 900.0, "acik-genis"), ("dark", 1100.0, 760.0, "koyu-dar")] {
        for shot in ["kot-agi", "kullanim-alani"] {
            let mut app = if shot == "kot-agi" { app() } else { crate::files_testing::app_with_drawing() };
            let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
            app.apply_settings();
            if shot == "kot-agi" {
                let _ = leapfrog_with(&mut app, LevelKind::Trigonometric);
            } else {
                let doc = app.document.as_mut().expect("a drawing");
                let mut settings = doc.settings().clone();
                settings.srid = 5254;
                settings.second_srid = Some(2320);
                doc.model.set_settings(settings);
                let _ = app.update(Message::Run("file.settings"));
                let _ = app.update(crate::project::settings_message(crate::project::SettingsEvent::Section(
                    crate::project::settings::Section::Crs,
                )));
            }
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{shot}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
