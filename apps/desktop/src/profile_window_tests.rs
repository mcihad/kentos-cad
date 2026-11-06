//! Boykesit (profile_window.rs, plan/road.md Y4): the window writes through `road.profiles`, each write one undo step;
//! and its pictures.

use crate::app::{App, Dialog, Message};
use crate::profile_window::Event as W;

/// A road project with `road.profiles.query`'s drawing: 120 points with heights, two surfaces, the main alignment, a
/// side one and a parallel axis; its profiles kept, or none.
fn app(profiles: bool) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.profiles.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    if !profiles {
        setup["settings"]["road"].as_object_mut().expect("road").remove("profiles");
    }
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Profiles(e));
}

fn profiles(app: &App) -> Vec<kentos_commands::ProfileDef> {
    app.document.as_ref().expect("a drawing").model.road_profiles().to_vec()
}

/// Boykesit oluştur: on the main alignment, its ends on the terrain, a parabola and a circle between.
fn made(app: &mut App) {
    let _ = app.update(Message::Run("profile.new"));
    assert_eq!(app.dialog, Some(Dialog::Profiles));
    send(app, W::FromGround);
    send(app, W::Add);
    send(app, W::Pvi(1, 1, "200".into()));
    send(app, W::Pvi(1, 2, "100".into()));
    send(app, W::Curve(1, 1));
    send(app, W::Pvi(1, 3, "120".into()));
    send(app, W::Add);
    send(app, W::Pvi(2, 1, "420".into()));
    send(app, W::Pvi(2, 2, "112".into()));
    send(app, W::Curve(2, 3));
    send(app, W::Pvi(2, 3, "1500".into()));
}

#[test]
fn the_window_makes_a_profile_on_the_terrain_and_rebinds_a_stale_one() {
    let mut app = app(false);
    made(&mut app);
    let w = app.profile_window.as_ref().expect("open");
    assert!(w.ground.length > 600.0, "the alignment's terrain is read");
    send(&mut app, W::Save);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(app.profile_window.is_none(), "Kaydet closes the window: {said:?}");
    let list = profiles(&app);
    assert_eq!(list.len(), 1);
    let p = &list[0];
    assert_eq!((p.id.as_str(), p.name.as_str(), p.alignment.as_str(), p.surface.as_deref()), ("b-1", "Proje kotu", "g-1", Some("t-1")));
    assert_eq!(p.pvis.len(), 4);
    assert_eq!((p.pvis[0].station, p.pvis[1].length, p.pvis[2].radius), (0.0, 120.0, 1500.0));
    assert!(p.pvis[3].station > 669.0, "the last PVI at the alignment's end");
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert_eq!(doc.undo().as_deref(), Some("Boykesit oluştur"));

    // A stale profile: Aksa yeniden bağla takes the alignment's present revision, one undo step.
    let mut app = self::app(true);
    let _ = app.update(Message::Run("profile.editor"));
    send(&mut app, W::Choose(1));
    send(&mut app, W::Rebind);
    assert_eq!(profiles(&app)[1].alignment_revision, 3);
    send(&mut app, W::Cancel);
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert_eq!(doc.undo().as_deref(), Some("Boykesiti düzenle"));

    // Çizime yaz: the chosen profile drawn as objects on the layer, one undo step.
    let mut app = self::app(true);
    let before = app.document.as_ref().expect("doc").model.entities().count();
    let _ = app.update(Message::Run("profile.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::DrawLayer("0".into()));
    send(&mut app, W::Place("0; -300".into()));
    send(&mut app, W::Draw);
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert!(doc.entities().count() > before + 50, "the frame, grid, lines and bands are drawn");
    // The grades' band as KGM's 2019 specification writes them (docs/adr/0978): the sign before %, three decimals.
    let grades: Vec<String> = doc
        .entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) if t.text.contains(" L=") && t.text.contains('%') => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    assert!(!grades.is_empty(), "the grades' band is written");
    for g in &grades {
        let head = g.split(" L=").next().unwrap_or_default();
        let ok = head == "%0.000"
            || ((head.starts_with("+%") || head.starts_with("-%"))
                && head[2..].split_once('.').is_some_and(|(_, d)| d.len() == 3 && d.chars().all(|c| c.is_ascii_digit())));
        assert!(ok, "{g}: KGM writes +%5.241, -%3.202");
    }
    assert_eq!(doc.undo().as_deref(), Some("Boykesit çiz"));
}

#[test]
fn curves_that_overlap_are_refused_and_the_window_stays() {
    let mut app = app(false);
    made(&mut app);
    send(&mut app, W::Pvi(1, 3, "400".into()));
    send(&mut app, W::Save);
    assert!(app.profile_window.is_some(), "a refused write leaves the window open");
    assert!(profiles(&app).is_empty());
}

/// The window over the drawing, wide and narrow, light and dark; `.run/gui-evidence/yol-y4-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib profile_window_tests::screens -- --ignored --nocapture
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
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("profile.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::PviChoose(2));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("yol-y4-boykesit-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // The stale profile on the side alignment.
        send(&mut app, W::Choose(1));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("yol-y4-boykesit-eski-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // Çizime yaz: the profile drawn under the alignments, the window closed over it.
        if name == "acik-genis" {
            send(&mut app, W::Choose(0));
            send(&mut app, W::DrawLayer("0".into()));
            send(&mut app, W::Draw);
            send(&mut app, W::Cancel);
            let _ = app.update(Message::Run("view.zoomExtents"));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join("yol-y4-boykesit-cizimde-acik-genis.png");
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
            // Closer: the first 230 m, its bands and the first curve.
            app.zoom_to_bounds(&kentos_commands::Bounds { min_x: 0.0, min_y: -320.0, max_x: 250.0, max_y: -120.0 });
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join("yol-y4-boykesit-bantlar-acik-genis.png");
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
            let _ = app.update(Message::Run("profile.editor"));
        }
        // A curve too long: the window says why before it is saved.
        send(&mut app, W::Choose(0));
        send(&mut app, W::Pvi(1, 3, "400".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("yol-y4-boykesit-binme-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// `road.profiles.impact`'s drawing: the profiles, corridors, section sets, the main volume and the main haul plan
/// approved (docs/adr/0962).
fn impact_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.profiles.impact.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Boykesit değişikliğinin etkisi (Y56, docs/adr/0962; ROAD-PROFILE-09): v1 typed 1 m higher, the window reads what
/// saving it would make stale as the shared case does; typed back, nothing; saved, nothing more to tell.
#[test]
fn typing_a_pvi_tells_what_saving_it_makes_stale() {
    let mut app = impact_app();
    let _ = app.update(Message::Run("profile.editor"));
    assert!(app.profile_window.as_ref().expect("open").impact.is_none(), "nothing typed yet");
    send(&mut app, W::Pvi(1, 2, "102,5".into()));
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.profiles.impact.json")).expect("cases");
    let want = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "v1 1 m yükselir").expect("the case")["steps"][0]
        ["result"]["output"]
        .clone();
    let Some(Ok((imp, warnings))) = app.profile_window.as_ref().expect("open").impact.clone() else { panic!("the impact reads") };
    assert_eq!(imp.changes.len(), 1);
    assert!((imp.changes[0].from - want["changes"][0]["from"].as_f64().expect("from")).abs() < 1e-6);
    assert!((imp.changes[0].to - want["changes"][0]["to"].as_f64().expect("to")).abs() < 1e-6);
    assert_eq!(imp.corridors.len(), 1);
    assert!(imp.volumes.iter().any(|v| v.approved) && imp.hauls.iter().any(|h| h.approved));
    assert!(warnings.iter().any(|w| w.code == "profile_impact_approved"));
    // Typed back to the saved elevation: the line does not change.
    send(&mut app, W::Pvi(1, 2, "101,5".into()));
    assert!(app.profile_window.as_ref().expect("open").impact.is_none());
    // Saved: the saved line is the typed one.
    send(&mut app, W::Pvi(1, 2, "102,5".into()));
    send(&mut app, W::Save);
    let _ = app.update(Message::Run("profile.editor"));
    assert!(app.profile_window.as_ref().expect("open").impact.is_none());
}

/// The impact over the profile and under it, wide and narrow, light and dark; `.run/gui-evidence/yol-y56/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib profile_window_tests::screens_impact -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_impact() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y56");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = impact_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y56-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("profile.editor"));
        send(&mut app, W::Pvi(1, 2, "102,5".into()));
        shot(&mut app, "v1-yukselir");
        send(&mut app, W::Pvi(1, 2, "101,5".into()));
        send(&mut app, W::Pvi(3, 3, "35".into()));
        send(&mut app, W::Pvi(3, 4, "50".into()));
        shot(&mut app, "asimetrik");
    }
}

/// The two lengths apart (docs/adr/0968): the status says the km's horizontal length and the red line's in space, the
/// latter `road.profiles.query`'s `length3d`.
#[test]
fn the_status_says_the_length_in_space_apart_from_the_km_s() {
    let mut app = app(true);
    app.open_profiles();
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let input = kentos_commands::ProfilesQuery { profile: Some("b-1".into()), stations: vec![], every: None, terrain: false };
    let kentos_commands::CommandResult::Completed { output, .. } =
        kentos_native_application::road_profiles_query::execute(&mut kentos_native_application::ExecutionContext::new(doc), input)
    else {
        panic!("the query reads")
    };
    let asked = output.profiles[0].length3d;
    let w = app.profile_window.as_ref().expect("open");
    let (said, bad) = crate::profile_window::status(&w.rows[0], false, &w.ground);
    assert!(!bad, "{said}");
    let want = format!("yatay 669,000 m · 3B {} m", format!("{asked:.3}").replace('.', ","));
    assert!(said.contains(&want), "{said} / {want}");
    assert!(asked > 669.0, "the red line in space is longer than its horizontal run");
}

/// `road.profiles.draw`'s KGM case drawing (docs/adr/0980): the main profile and a superelevation schedule by rows on
/// its alignment.
fn kgm_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.profiles.draw.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "KGM bantları, dever çizelgesiyle").expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Çizime yaz with KGM's bands (docs/adr/0980): the six bands' names in KGM's order, the right side's slope dashed, the
/// kept drawing following the schedule too; one undo step.
#[test]
fn the_profile_is_drawn_with_kgm_s_bands() {
    let mut app = kgm_app();
    let _ = app.update(Message::Run("profile.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::DrawLayer("0".into()));
    send(&mut app, W::Place("0; -200".into()));
    send(&mut app, W::Kgm(true));
    send(&mut app, W::Dever(Some("d-1".into())));
    send(&mut app, W::Draw);
    let doc = &app.document.as_ref().expect("doc").model;
    let texts: Vec<(String, f64)> = doc
        .entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some((t.text.clone(), t.p.y)),
            _ => None,
        })
        .collect();
    let y = |name: &str| texts.iter().find(|(t, _)| t == name).map(|(_, y)| *y).unwrap_or_else(|| panic!("{name} is written"));
    let order = ["Kırmızı Kot", "Siyah Kot", "Kilometre", "Yatay Geometri", "Düşey Geometri", "Dever"];
    assert!(order.windows(2).all(|w| y(w[0]) > y(w[1])), "KGM's order, top down");
    assert!(
        doc.entities().any(|e| e.base().look.line_type == Some(kentos_commands::layer::LineType::Dashed)),
        "the right side's slope dashed"
    );
    let kept = doc.road_drawings().last().expect("kept");
    assert!(kept.sources.iter().any(|s| s.kind == kentos_commands::RoadSourceKind::Superelevation && s.id == "d-1"));
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert_eq!(doc.undo().as_deref(), Some("Boykesit çiz"));
}

/// KGM's bands under the profile, in the window and drawn, light and dark; `.run/gui-evidence/yol-y73/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib profile_window_tests::screens_kgm -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_kgm() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y73");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = kgm_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("profile.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::DrawLayer("0".into()));
        send(&mut app, W::Place("0; -200".into()));
        send(&mut app, W::Kgm(true));
        send(&mut app, W::Dever(Some("d-1".into())));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y73-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, W::Draw);
        let _ = app.update(Message::Profiles(W::Cancel));
        // The bands under the frame: the frame's lower left is (0, −200); the bands go about 43 text heights down.
        app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -45.0, min_y: -320.0, max_x: 420.0, max_y: -150.0 }, 12.0);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y73-bantlar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: 100.0, min_y: -320.0, max_x: 420.0, max_y: -220.0 }, 12.0);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y73-bantlar-yakin-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Çizime yaz with a Kot aralığı (docs/adr/0990): 12 m splits the profile in four as the command's cases say (datums
/// 100, 104, 106, 108), so the last part's top height 120.00 is written, which the frame as high as the lines (top 119)
/// never writes.
#[test]
fn a_span_splits_the_drawn_profile() {
    let mut app = kgm_app();
    let _ = app.update(Message::Run("profile.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::DrawLayer("0".into()));
    send(&mut app, W::Place("0; -200".into()));
    send(&mut app, W::Span("12".into()));
    send(&mut app, W::Draw);
    let doc = &app.document.as_ref().expect("doc").model;
    let texts: Vec<String> = doc
        .entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect();
    // Beside its split and on the frame's right edge (docs/adr/0993).
    assert_eq!(texts.iter().filter(|t| *t == "120.00").count(), 2, "the last part's top");
}

/// Kot aralığı in the window and the split profile drawn, light and dark; `.run/gui-evidence/yol-y84/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib profile_window_tests::screens_span -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_span() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y84");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = kgm_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("profile.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::DrawLayer("0".into()));
        send(&mut app, W::Place("0; -200".into()));
        send(&mut app, W::Kgm(true));
        send(&mut app, W::Span("12".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y84-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, W::Draw);
        let _ = app.update(Message::Profiles(W::Cancel));
        // The frame: lower left (0, −200), 669 m long, 12 m × 10 high.
        app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -45.0, min_y: -215.0, max_x: 690.0, max_y: -70.0 }, 12.0);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y84-cizim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Yer left empty (docs/adr/0991): the profile goes under everything drawn, its frame's top 20 paper mm under the
/// drawing's box at 1:1000, so none of its objects reaches into that box.
#[test]
fn an_empty_place_draws_the_profile_under_the_drawing() {
    let mut app = kgm_app();
    let _ = app.update(Message::Run("profile.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::DrawLayer("0".into()));
    let before_ids: std::collections::HashSet<u32> =
        app.document.as_ref().expect("doc").model.entities().map(|e| e.base().id).collect();
    let doc = &app.document.as_ref().expect("doc").model;
    app.spatial.sync(doc);
    let before = app.spatial.extent().expect("a drawing");
    send(&mut app, W::Draw);
    let doc = &app.document.as_ref().expect("doc").model;
    let new: Vec<f64> = doc.entities().filter(|e| !before_ids.contains(&e.base().id)).map(|e| f64::from(e.base().id)).collect();
    assert!(!new.is_empty(), "the profile is drawn");
    app.spatial.sync(doc);
    let drawn = app.spatial.store().extent(Some(&new)).expect("its box");
    assert!(drawn.max_y < before.min_y, "the profile ({}) stays under the drawing ({})", drawn.max_y, before.min_y);
    assert!((drawn.min_x - before.min_x).abs() < 50.0, "from the drawing's left edge, the heights' texts left of it");
}

/// The corridor cases' drawing (fixtures/road/v1/corridor-bands.json): corridor c-1 on the profile's alignment.
fn corridor_app() -> App {
    let (mut app, _) = App::boot(None);
    let bands: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/road/v1/corridor-bands.json")).expect("json");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(bands["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Çizime yaz with KGM's bands and a corridor (docs/adr/0994): its slope and platform rows go over the frame, as
/// Yol paftaları draws them; with KentOS's bands the choice is not sent (the command would refuse it).
#[test]
fn the_window_draws_the_corridor_rows() {
    let mut app = corridor_app();
    let _ = app.update(Message::Run("profile.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::DrawLayer("0".into()));
    send(&mut app, W::Corridor(Some("c-1".into())));
    send(&mut app, W::Draw);
    let rows = |app: &App| {
        app.document.as_ref().expect("doc").model.entities().filter(|e| matches!(e, kentos_commands::Entity::Text(t) if t.text == "PLATFORM GEN.")).count()
    };
    assert_eq!(rows(&app), 0, "KentOS's bands: no rows");
    send(&mut app, W::Kgm(true));
    let before_ids: std::collections::HashSet<u32> =
        app.document.as_ref().expect("doc").model.entities().map(|e| e.base().id).collect();
    let doc = &app.document.as_ref().expect("doc").model;
    app.spatial.sync(doc);
    let before = app.spatial.extent().expect("a drawing");
    send(&mut app, W::Draw);
    assert_eq!(rows(&app), 1, "KGM's bands: the rows over the frame");
    // Yer left empty: the rows over the frame stay under the drawing too (docs/adr/0991; Y88's first picture).
    let doc = &app.document.as_ref().expect("doc").model;
    let new: Vec<f64> = doc.entities().filter(|e| !before_ids.contains(&e.base().id)).map(|e| f64::from(e.base().id)).collect();
    app.spatial.sync(doc);
    let drawn = app.spatial.store().extent(Some(&new)).expect("its box");
    assert!(drawn.max_y < before.min_y, "the rows ({}) stay under the drawing ({})", drawn.max_y, before.min_y);
}

/// The Boykesit window's KGM line (Dever bandı, Şev ve platform) and the profile drawn with the rows, light and dark;
/// `.run/gui-evidence/yol-y88/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib profile_window_tests::screens_corridor_choice -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_corridor_choice() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y88");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = corridor_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("profile.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::DrawLayer("0".into()));
        send(&mut app, W::Kgm(true));
        send(&mut app, W::Corridor(Some("c-1".into())));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y88-pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, W::Draw);
        let _ = app.update(Message::Profiles(W::Cancel));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y88-cizim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
