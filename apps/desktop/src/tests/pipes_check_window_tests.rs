//! Boru hattı denetimi (pipes_check_window.rs, docs/adr/1640) on the shared cases' drawing of `cad.pipes.check`: the
//! window opens on the first utility network with the attributes the pipes have and the project's surface, no limit;
//! limits typed in the form find more, Bulgulu shows only the pipes with findings, Bulgulu boruları seç selects them,
//! CSV kaydet writes the table, nothing is written to the drawing. Harita, Yol and GIS offer it, Mimari does not. Diğer
//! ağ measures the clearances to the water network as the shared cases do (docs/adr/2360); with it, the profile draws
//! the water pipes crossing the run (docs/adr/2760). And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::pipes_check_window::{Above, Event as P, Ground, Show};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.check.json")).expect("cases")
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
    let profile: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.profile.json")).expect("cases");
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

fn profile_output(name: &str) -> Value {
    let profile: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pipes.profile.json")).expect("cases");
    profile["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect(name)["steps"][0]["result"]
        ["output"]
        .clone()
}

/// docs/adr/2760: with Diğer ağ the water network, Hat profilini çiz draws its pipes crossing the run as the shared
/// case of `cad.pipes.profile` says (its objects' count): an ellipse for each crossing pipe with an invert there, as
/// wide as its diameter and the exaggeration times as high, its name and invert above; W7 has no heights, so it is
/// told, not drawn. Kesişen borular off: the profile without them, the sewer's main case.
#[test]
fn the_profile_draws_the_water_pipes_crossing_the_run() {
    use kentos_commands::Entity;
    let want = profile_output("Kesişen borular: içme suyu boruları atıksu hattını kesiyor");
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::Other("su".into())));
    assert!(app.pipes_check.profile_crossings, "on when the window opens");
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::PipesCheck(P::DrawProfile));
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains(", 4 kesişen boru (1 borunun orada kotu yok, çizilmedi);"), "{said}");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let drawn: Vec<&Entity> = doc.entities().skip(before).collect();
    assert_eq!(drawn.len(), want["uids"].as_array().map(Vec::len).unwrap_or(0), "{said}");
    let exaggeration = want["exaggeration"].as_f64().expect("the exaggeration");
    let known: Vec<&Value> =
        want["crossings"].as_array().expect("crossings").iter().filter(|c| c["invert"].is_number()).collect();
    let ellipses: Vec<(f64, f64)> = drawn
        .iter()
        .filter_map(|e| match e {
            Entity::Ellipse(e) => Some((e.major.x.hypot(e.major.y), e.ratio)),
            _ => None,
        })
        .collect();
    assert_eq!(ellipses.len(), known.len(), "W3, W1 and W2");
    for ((major, ratio), c) in ellipses.iter().zip(&known) {
        let d = c["diameter"].as_f64().expect("a diameter");
        assert!((major - d / 2.0 * exaggeration).abs() < 1e-9, "{major} {d}");
        assert!((ratio - 1.0 / exaggeration).abs() < 1e-12, "{ratio}");
    }
    let texts: Vec<&str> = drawn
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    for c in &known {
        let words = format!("{} {:.2}", c["name"].as_str().unwrap_or(""), c["invert"].as_f64().unwrap_or(0.0));
        assert!(texts.contains(&words.as_str()), "{words}");
    }
    assert!(!texts.iter().any(|t| t.starts_with("W7")), "W7 has no invert there");
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Hat profili"));
    // Kesişen borular off: the sewer's main case.
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::Other("su".into())));
    let _ = app.update(Message::PipesCheck(P::ProfileCrossings(false)));
    let _ = app.update(Message::PipesCheck(P::DrawProfile));
    let main = profile_output("Atıksu ana hattı P1'den çıkışa: altı boru");
    let count = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    assert_eq!(count, before + main["uids"].as_array().map(Vec::len).unwrap_or(0));
    assert!(!crate::files_testing::last_said(&app).contains("kesişen"), "{}", crate::files_testing::last_said(&app));
}

/// The window with Diğer ağ (kesişen borular in the profile's row) and the profile with the water pipes crossing the
/// run, then close on W1 crossing P2's body; `.run/gui-evidence/kesisen-borular-2760/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib pipes_check_window_tests::crossing_profile_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn crossing_profile_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kesisen-borular-2760");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pipes.check"));
        let _ = app.update(Message::PipesCheck(P::Other("su".into())));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
        let _ = app.update(Message::PipesCheck(P::DrawProfile));
        let centres: Vec<[f64; 2]> = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .skip(before)
            .filter_map(|e| match e {
                kentos_commands::Entity::Ellipse(e) => Some([e.c.x, e.c.y]),
                _ => None,
            })
            .collect();
        let (x0, x1) = centres.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), c| (a.min(c[0]), b.max(c[0])));
        let (y0, y1) = centres.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), c| (a.min(c[1]), b.max(c[1])));
        // The three crossings with the run's lines around them, then W1 (the second) on P2's body.
        let w1 = centres.get(1).copied().expect("W1");
        for (bounds, file) in [
            (kentos_render_wgpu::Bounds { min_x: x0 - 15.0, min_y: y0 - 45.0, max_x: x1 + 15.0, max_y: y1 + 45.0 }, "2-profil"),
            (kentos_render_wgpu::Bounds { min_x: w1[0] - 15.0, min_y: w1[1] - 8.0, max_x: w1[0] + 15.0, max_y: w1[1] + 12.0 }, "3-w1-yakin"),
        ] {
            app.viewport.camera.fit(&bounds, 12.0);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
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
            doc.entities().skip(n.saturating_sub(400)).filter(|e| e.base().id > 264).map(|e| kentos_domain::Slot(e.base().id)).collect();
        app.selection.set(slots);
        let _ = app.update(Message::Run("view.zoomSelection"));
        app.selection.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-profil-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

fn case_output(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect(name)["steps"][0]["result"]
        ["output"]
        .clone()
}

/// The window's limits set as the shared cases' (the sewer's full question).
fn with_case_limits(app: &mut App) {
    for e in [P::MinCover("0,8".into()), P::MaxCover("3".into()), P::MinSlope("9".into()), P::MaxSlope("50".into())] {
        let _ = app.update(Message::PipesCheck(e));
    }
    let _ = app.update(Message::PipesCheck(P::Catalog("200 300 400 500".into())));
}

fn same_list(got: &Value, want: &Value, what: &str) {
    let (g, w) = (got.as_array().cloned().unwrap_or_default(), want.as_array().cloned().unwrap_or_default());
    assert_eq!(g.len(), w.len(), "{what}: {got}");
    for (g, w) in g.iter().zip(&w) {
        for key in ["pipe", "other", "above"] {
            assert_eq!(g[key], w[key], "{what} {key}");
        }
        for key in ["clearance", "distance", "invert", "otherInvert"] {
            match (g[key].as_f64(), w[key].as_f64()) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{what} {key}: {a} {b}"),
                (a, b) => assert_eq!(a.is_some(), b.is_some(), "{what} {key}"),
            }
        }
    }
}

/// docs/adr/2360: Diğer ağ picks the water network (its heights and Çap guessed from its pipes); the least vertical
/// clearance and the water above give the shared case's crossings and findings, the least horizontal one its near
/// pass; Kesişim lists the four crossings; nothing is written.
#[test]
fn the_clearances_to_the_water_network_are_the_shared_cases() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("pipes.check"));
    with_case_limits(&mut app);
    let _ = app.update(Message::PipesCheck(P::Other("su".into())));
    assert_eq!(app.pipes_check.other_from, crate::pipes_check_window::From::Heights, "the water pipes have no Akar baş");
    assert_eq!(app.pipes_check.other_diameter, "Çap");
    let _ = app.update(Message::PipesCheck(P::Vertical("0,5".into())));
    let _ = app.update(Message::PipesCheck(P::Above(Above::Other)));
    let r = serde_json::to_value(app.pipes_check.answer.clone().expect("an answer").expect("checked")).expect("json");
    let want = case_output("açıklık: en az düşey açıklık ve suyun üstte olması");
    same_list(&r["crossings"], &want["crossings"], "crossings");
    assert_eq!(r["counts"], want["counts"]);
    assert_eq!(r["findings"].as_array().map(Vec::len), want["findings"].as_array().map(Vec::len));
    let _ = app.update(Message::PipesCheck(P::Show(Show::Crossings)));
    assert_eq!(app.pipes_check.show, Show::Crossings);
    // The least horizontal clearance alone: the near pass of the case.
    for e in [P::Vertical(String::new()), P::Above(Above::Either), P::Horizontal("3".into())] {
        let _ = app.update(Message::PipesCheck(e));
    }
    let r = serde_json::to_value(app.pipes_check.answer.clone().expect("an answer").expect("checked")).expect("json");
    let want = case_output("açıklık: en az yatay açıklık; kesişmeden yakın geçen boru");
    same_list(&r["near"], &want["near"], "near");
    assert_eq!(r["counts"], want["counts"]);
    // No other network: no clearance, the table back to the pipes.
    let _ = app.update(Message::PipesCheck(P::Show(Show::Near)));
    let _ = app.update(Message::PipesCheck(P::Other(String::new())));
    assert_eq!(app.pipes_check.show, Show::All);
    let r = app.pipes_check.answer.clone().expect("an answer").expect("checked");
    assert!(r.crossings.is_empty() && r.near.is_empty());
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

/// docs/adr/3160: Kesişimleri göster with the case's clearances (0.5 m vertical with the water above, 3 m horizontal)
/// marks every crossing with the command's verdict at its place and the near pass; the window closes; a water pipe
/// moved away changes the marks (the same input, the drawing as it is); İşareti kaldır takes them away.
#[test]
fn the_crossings_are_marked_on_the_drawing() {
    use crate::pipes_marks::Verdict;
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    with_case_limits(&mut app);
    for e in [P::Other("su".into()), P::Vertical("0,5".into()), P::Horizontal("3".into()), P::Above(Above::Other)] {
        let _ = app.update(Message::PipesCheck(e));
    }
    let r = app.pipes_check.answer.clone().expect("an answer").expect("checked");
    let _ = app.update(Message::PipesCheck(P::Mark));
    assert_eq!(app.dialog, None, "the window closes");
    let m = app.pipes_marks().expect("marked");
    assert_eq!(m.crossings.len(), r.crossings.len());
    assert_eq!(m.near.len(), r.near.len());
    for (mark, c) in m.crossings.iter().zip(&r.crossings) {
        assert_eq!(mark.at, c.at);
        let fails = r.findings.iter().any(|f| ["clash", "closeCrossing", "wrongSide"].contains(&f.kind.as_str()) && f.at == c.at);
        let want = if fails { Verdict::Fails } else if c.clearance.is_none() { Verdict::Unknown } else { Verdict::Holds };
        assert_eq!(mark.verdict, want, "{c:?}");
    }
    assert!(m.crossings.iter().any(|c| c.verdict == Verdict::Fails), "the case has a failing crossing");
    assert!(app.offers("data.unmark"));
    // The water network's pipes moved far away: no crossing, no mark.
    let doc = &mut app.document.as_mut().expect("open").model;
    let moved: Vec<(kentos_domain::Slot, kentos_commands::Entity)> = doc
        .entities_with_uids()
        .filter(|(_, e)| e.base().layer_id == "su")
        .filter_map(|(u, e)| {
            let mut e = e.clone();
            match &mut e {
                kentos_commands::Entity::Polyline(p) => p.pts.iter_mut().for_each(|v| v.y += 10_000.0),
                kentos_commands::Entity::Line(l) => {
                    l.a.y += 10_000.0;
                    l.b.y += 10_000.0;
                }
                _ => return None,
            }
            doc.slot_of(u).map(|s| (s, e))
        })
        .collect();
    assert!(!moved.is_empty());
    doc.update_many(moved, "Taşı");
    assert!(app.pipes_marks().is_none(), "nothing crosses any more");
    let _ = app.update(Message::Run("data.unmark"));
    assert!(app.pipes_shown_marks.is_none());
}

#[test]
fn a_clearance_that_is_no_number_is_said() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::Other("su".into())));
    let _ = app.update(Message::PipesCheck(P::Horizontal("üç".into())));
    assert_eq!(app.pipes_check.answer, Some(Err("En az yatay açıklık “üç” bir sayı değil.".into())));
}

/// `.run/gui-evidence/kesisimler-3160/`: the window with Kesişimleri göster, and the crossings and the near pass marked
/// on the drawing, light and wide, dark and narrow.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib pipes_check_window_tests::marks_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn marks_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kesisimler-3160");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pipes.check"));
        with_case_limits(&mut app);
        for e in [P::Other("su".into()), P::Vertical("0,5".into()), P::Horizontal("3".into()), P::Above(Above::Other), P::Show(Show::Crossings)] {
            let _ = app.update(Message::PipesCheck(e));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::PipesCheck(P::Mark));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-cizimde-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// `.run/gui-evidence/boru-acikligi-2360/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib pipes_check_window_tests::clearance_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn clearance_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/boru-acikligi-2360");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pipes.check"));
        with_case_limits(&mut app);
        for e in [
            P::Other("su".into()),
            P::Vertical("0,5".into()),
            P::Horizontal("3".into()),
            P::Above(Above::Other),
            P::Show(Show::Crossings),
        ] {
            let _ = app.update(Message::PipesCheck(e));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        for (show, file) in [(Show::Crossings, "1-kesisim"), (Show::Near, "2-yakin-gecis"), (Show::Found, "3-bulgulu")] {
            let _ = app.update(Message::PipesCheck(P::Show(show)));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

/// The shared cases' walls drawing (docs/adr/3960): the pipes carry `Et`, the window takes it.
fn walled(program: Option<Module>) -> App {
    kentos_processing::command::register_as("desktop");
    let case = cases()["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("et kalınlığı: örtü")))
        .expect("the walls case")
        .clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// docs/adr/3960: the window guesses `Et` as the walls' attribute (the pipes have it, the water pipes too); its cover is
/// to the top outside and its clearances outside to outside, the shared cases' answers; P7 without a wall is said.
#[test]
fn the_walls_are_read_from_the_pipes_and_the_answers_are_the_shared_cases() {
    let mut app = walled(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    assert_eq!(app.pipes_check.wall, "Et");
    with_case_limits(&mut app);
    let r = serde_json::to_value(app.pipes_check.answer.clone().expect("an answer").expect("checked")).expect("json");
    let want = case_output("et kalınlığı: örtü borunun dış tepesinden; et kalınlığı olmayan boru söylenir");
    assert_eq!(r["coverTo"], "outside");
    assert_eq!(r["counts"], want["counts"]);
    for (g, w) in r["pipes"].as_array().expect("pipes").iter().zip(want["pipes"].as_array().expect("pipes")) {
        for key in ["wall", "minCover", "maxCover"] {
            match (g[key].as_f64(), w[key].as_f64()) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-9, "{key}: {a} {b}"),
                (a, b) => assert_eq!(a, b, "{key}"),
            }
        }
    }
    let _ = app.update(Message::PipesCheck(P::Other("su".into())));
    assert_eq!(app.pipes_check.other_wall, "Et", "the water pipes' wall guessed too");
    let _ = app.update(Message::PipesCheck(P::Vertical("0,5".into())));
    let _ = app.update(Message::PipesCheck(P::Horizontal("3".into())));
    let r = serde_json::to_value(app.pipes_check.answer.clone().expect("an answer").expect("checked")).expect("json");
    let want = case_output("et kalınlığı: açıklıklar dıştan dışa");
    same_list(&r["crossings"], &want["crossings"], "crossings");
    same_list(&r["near"], &want["near"], "near");
    // Without the attribute: the crown again.
    let _ = app.update(Message::PipesCheck(P::Wall(String::new())));
    let r = app.pipes_check.answer.clone().expect("an answer").expect("checked");
    assert_eq!(r.cover_to, "crown");
}

/// `.run/gui-evidence/et-kalinligi-3960/`: the window with the walls, light and wide, dark and narrow.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib pipes_check_window_tests::wall_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn wall_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/et-kalinligi-3960");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = walled(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pipes.check"));
        with_case_limits(&mut app);
        let _ = app.update(Message::PipesCheck(P::Other("su".into())));
        let _ = app.update(Message::PipesCheck(P::Vertical("0,5".into())));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, stem: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        shot(&mut app, "1-boru-hatti");
        let _ = app.update(Message::PipesCheck(P::Show(Show::Crossings)));
        shot(&mut app, "2-kesisimler");
    }
}

/// Docs/adr/5160: Kesişen borunun taraması › Bütün boru on pipes without a wall: the shared case's objects, one filled
/// hatch per crossing pipe with an invert, each bounded by its one ellipse.
#[test]
fn the_crossing_pipes_fill_is_chosen() {
    use kentos_commands::{CrossingHatch, Entity};
    let want = profile_output("Etsiz kesişen boru, bütün boru taranır: iç elips");
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pipes.check"));
    let _ = app.update(Message::PipesCheck(P::Other("su".into())));
    let _ = app.update(Message::PipesCheck(P::ProfileHatch(CrossingHatch::Whole)));
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::PipesCheck(P::DrawProfile));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let drawn: Vec<&Entity> = doc.entities().skip(before).collect();
    assert_eq!(drawn.len(), want["uids"].as_array().map(Vec::len).unwrap_or(0));
    let loops: Vec<usize> = drawn
        .iter()
        .filter_map(|e| match e {
            Entity::Hatch(h) => h.boundary.as_ref().map(Vec::len),
            _ => None,
        })
        .collect();
    let known = want["crossings"].as_array().expect("crossings").iter().filter(|c| c["invert"].is_number()).count();
    assert_eq!(loops, vec![1; known]);
}

/// Docs/adr/5160, `.run/gui-evidence/kesisen-boru-taramasi-5160/`: the window with Kesişen borunun taraması on Bütün
/// boru, then W1 close: filled whole; light wide and dark narrow.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib pipes_check_window_tests::fill_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn fill_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kesisen-boru-taramasi-5160");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pipes.check"));
        let _ = app.update(Message::PipesCheck(P::Other("su".into())));
        let _ = app.update(Message::PipesCheck(P::ProfileHatch(kentos_commands::CrossingHatch::Whole)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
        let _ = app.update(Message::PipesCheck(P::DrawProfile));
        let w1 = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .skip(before)
            .filter_map(|e| match e {
                kentos_commands::Entity::Ellipse(e) => Some([e.c.x, e.c.y]),
                _ => None,
            })
            .nth(1)
            .expect("W1");
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: w1[0] - 1.2, min_y: w1[1] - 0.8, max_x: w1[0] + 1.2, max_y: w1[1] + 1.0 },
            12.0,
        );
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-w1-yakin-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
