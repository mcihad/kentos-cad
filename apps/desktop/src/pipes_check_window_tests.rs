//! Boru hattı denetimi (pipes_check_window.rs, docs/adr/1640) on the shared cases' drawing of `cad.pipes.check`: the
//! window opens on the first utility network with the attributes the pipes have and the project's surface, no limit;
//! limits typed in the form find more, Bulgulu shows only the pipes with findings, Bulgulu boruları seç selects them,
//! CSV kaydet writes the table, nothing is written to the drawing. Harita, Yol and GIS offer it, Mimari does not. And
//! its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::pipes_check_window::{Event as P, Ground, Show};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.pipes.check.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    // The network is built by İşlemler's tools, as the app registers them at its start.
    kentos_processing::command::register_as("desktop");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn kinds(app: &App) -> Vec<(String, u32)> {
    let r = app.pipes_check.answer.clone().expect("an answer").expect("checked");
    r.counts.into_iter().map(|c| (c.kind, c.count)).collect()
}

#[test]
fn the_sewer_is_checked_and_limits_find_more() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("pipes.check"));
    assert_eq!(app.dialog, Some(Dialog::PipesCheck));
    assert_eq!(app.pipes_check.network, "atiksu");
    assert_eq!(app.pipes_check.ground, Ground::Surface);
    // No limit: only what is wrong in any project.
    let none = kinds(&app);
    assert!(none.iter().all(|(k, _)| !["shallow", "deep", "lowSlope", "steepSlope", "invertUnit"].contains(&k.as_str())), "{none:?}");
    assert!(none.contains(&("reverseSlope".into(), 2)) && none.contains(&("detached".into(), 1)), "{none:?}");
    // The cases' limits: the window's answer is the shared case's.
    for e in [P::MinCover("0,8".into()), P::MaxCover("3".into()), P::MinSlope("9".into()), P::MaxSlope("50".into())] {
        let _ = app.update(Message::PipesCheck(e));
    }
    let _ = app.update(Message::PipesCheck(P::Catalog("200 300 400 500".into())));
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"]["counts"];
    let got: Vec<Value> =
        kinds(&app).into_iter().map(|(k, n)| serde_json::json!({"kind": k, "count": n})).collect();
    assert_eq!(&Value::from(got), want);
    let _ = app.update(Message::PipesCheck(P::Show(Show::Found)));
    let _ = app.update(Message::PipesCheck(P::Select));
    assert_eq!(app.dialog, None);
    assert_eq!(app.selection.ids().len(), 8, "every pipe but P1");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn a_limit_that_is_no_number_is_said() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::MinCover("bir".into())));
    assert_eq!(app.pipes_check.answer, Some(Err("En az örtü “bir” bir sayı değil.".into())));
}

#[test]
fn the_storm_network_reads_the_structures() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::Network("yagmur".into())));
    let _ = app.update(Message::PipesCheck(P::From(crate::pipes_check_window::From::Heights)));
    let _ = app.update(Message::PipesCheck(P::Ground(Ground::Structures)));
    let _ = app.update(Message::PipesCheck(P::Unit("m".into())));
    let r = app.pipes_check.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.pipes.len(), 3);
    assert!(r.pipes[1].reversed, "Akış = ters");
    assert!(r.pipes.iter().all(|p| !p.findings.contains(&"reverseSlope".to_owned())));
    assert_eq!(r.pipes[2].findings, ["noGround", "curved"].map(String::from), "the last structure has no level; an arc");
}

#[test]
fn csv_kaydet_writes_the_table() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    let dir = std::env::temp_dir().join(format!("kentos-boru-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("denetim.csv");
    app.pipes_check_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 10, "a head and nine pipes");
    assert!(lines[2].starts_with("P2;") && lines[2].ends_with(";Ters eğim"), "{}", lines[2]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn harita_yol_and_gis_offer_it_mimari_does_not() {
    for (module, yes) in [(Module::Map, true), (Module::Road, true), (Module::Gis, true), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("pipes.check"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/boru-denetimi-1640/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib pipes_check_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/boru-denetimi-1640");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pipes.check"));
        for e in [P::MinCover("0,8".into()), P::MaxCover("3".into()), P::MinSlope("9".into()), P::MaxSlope("50".into())] {
            let _ = app.update(Message::PipesCheck(e));
        }
        let _ = app.update(Message::PipesCheck(P::Catalog("200 300 400 500".into())));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // The ribbon's place: Arazi › Drenaj.
        let _ = app.update(Message::PipesCheck(P::Close));
        let _ = app.update(Message::RibbonTab("terrain"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-serit-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/1860: Hat profilini çiz draws the sewer's run from P1 to its outlet as the shared case of
/// `cad.pipes.profile` says (its objects' count), under the drawing, in one undo step “Hat profili”.
#[test]
fn the_profile_of_the_run_is_drawn_in_one_step() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    let profile: Value = serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.pipes.profile.json")).expect("cases");
    let want = &profile["cases"][0]["steps"][0]["result"]["output"];
    let p1 = want["pipes"][0]["uid"].as_str().expect("P1").to_owned();
    assert_eq!(app.pipes_check.profile_start, p1, "the first pipe checked");
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::PipesCheck(P::DrawProfile));
    let doc = app.document.as_ref().expect("a drawing");
    let drawn = want["uids"].as_array().map(Vec::len).unwrap_or(0);
    assert_eq!(doc.model.entities().count(), before + drawn, "{}", crate::files_testing::last_said(&app));
    assert!(crate::files_testing::last_said(&app).contains("6 boru"), "{}", crate::files_testing::last_said(&app));
    assert_eq!(app.dialog, None);
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Hat profili"));
}

/// The profile drawn under the drawing; `.run/gui-evidence/hat-profili-1860/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn profile_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/hat-profili-1860");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pipes.check"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::PipesCheck(P::DrawProfile));
        // Framed on the profile: the objects written last.
        let doc = &app.document.as_ref().expect("a drawing").model;
        let n = doc.entities().count();
        let slots: Vec<kentos_domain::Slot> =
            doc.entities().skip(n.saturating_sub(400)).filter(|e| e.base().id > 257).map(|e| kentos_domain::Slot(e.base().id)).collect();
        app.selection.set(slots);
        let _ = app.update(Message::Run("view.zoomSelection"));
        app.selection.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-profil-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
