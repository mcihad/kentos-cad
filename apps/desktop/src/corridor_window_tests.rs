//! Koridor (corridor_window.rs, plan/road.md Y7): the window builds the corridor as it is typed, writes through
//! `road.corridors.edit` and draws the slope plan (`road.corridors.draw`), each write one undo step; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::corridor_window::Event as W;

/// A road project with `road.corridors.draw`'s drawing: a surface, the main and side alignments, their profiles, two
/// templates, the superelevation schedules and two corridors.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.corridors.draw.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Corridors(e));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

fn draw(app: &mut App, corridor: &str, tick: f64) -> kentos_commands::CorridorsDrawn {
    let input = kentos_commands::CorridorsDraw {
        corridor: corridor.into(),
        layer_id: "0".into(),
        tick: Some(tick),
        height: None,
        expected_revision: None,
    };
    let mut cx = kentos_native_application::ExecutionContext::new(model(app));
    match kentos_native_application::road_corridors_draw::execute(&mut cx, input) {
        kentos_commands::CommandResult::Completed { output, .. } => output,
        other => panic!("drawn: {other:?}"),
    }
}

#[test]
fn the_slope_plan_is_drawn_in_one_step() {
    let mut app = app();
    let before = model(&mut app).entities().count();
    let drawn = draw(&mut app, "c-1", 5.0);
    assert!(drawn.lines > 10 && drawn.ticks > 50 && drawn.elevations > 20, "{drawn:?}");
    let doc = model(&mut app);
    assert_eq!(doc.entities().count(), before + (drawn.lines + drawn.ticks + drawn.elevations) as usize);
    assert_eq!(doc.undo().as_deref(), Some("Şevli plan"));
    assert_eq!(doc.entities().count(), before);
}

#[test]
fn the_window_makes_a_corridor_and_shows_it_built() {
    let mut app = app();
    let _ = app.update(Message::Run("corridor.new"));
    assert_eq!(app.dialog, Some(Dialog::Corridors));
    send(&mut app, W::Name("Kuzey koridoru".into()));
    for (k, v) in [(0, "25"), (1, "25"), (2, "20"), (3, "10")] {
        send(&mut app, W::Field(k, v.into()));
    }
    send(&mut app, W::Add);
    send(&mut app, W::Region(0, 1, "280".into()));
    let w = app.corridor_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("builds: {:?}", w.preview.why));
    assert!(rep.stations.len() > 15 && rep.triangles > 100, "{} km, {} triangles", rep.stations.len(), rep.triangles);
    send(&mut app, W::Save);
    assert!(app.corridor_window.is_none(), "Kaydet closes the window");
    let doc = model(&mut app);
    assert_eq!(doc.road_corridors().len(), 3);
    assert_eq!(doc.road_corridors()[2].regions[0].to, 280.0);
    assert_eq!(doc.undo().as_deref(), Some("Koridor"));
}

#[test]
fn a_saved_corridor_is_drawn_and_a_stale_one_rebound() {
    let mut app = app();
    let before = model(&mut app).entities().count();
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::DrawLayer("0".into()));
    send(&mut app, W::Draw);
    assert!(model(&mut app).entities().count() > before + 100);
    assert_eq!(model(&mut app).undo().as_deref(), Some("Şevli plan"));
    send(&mut app, W::Choose(1));
    send(&mut app, W::Rebind);
    assert_eq!(model(&mut app).road_corridors()[1].alignment_revision, 3);
    // A region left without its end is refused and the window stays.
    send(&mut app, W::Add);
    send(&mut app, W::Save);
    assert!(app.corridor_window.is_some());
}

/// The window and the slope plan over the drawing, wide and narrow, light and dark; `.run/gui-evidence/yol-y7-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib corridor_window_tests::screens -- --ignored --nocapture
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
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y7-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("corridor.editor"));
        shot(&mut app, "koridor");
        // The stale side corridor on its superelevation.
        send(&mut app, W::Choose(1));
        shot(&mut app, "koridor-yan");
        // A narrow swath: the open sides marked over the axis.
        send(&mut app, W::Choose(0));
        send(&mut app, W::Field(0, "6".into()));
        send(&mut app, W::Field(1, "6".into()));
        shot(&mut app, "koridor-dar");
        send(&mut app, W::Cancel);
    }
    let mut app = app();
    let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from("light"))]);
    app.apply_settings();
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let mut shot = |app: &mut App, file: &str| {
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("yol-y7-{file}-acik-genis.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    };
    draw(&mut app, "c-1", 5.0);
    draw(&mut app, "c-2", 5.0);
    let _ = app.update(Message::Run("view.zoomExtents"));
    shot(&mut app, "sevli-plan");
    // Closer: the curve, its ticks in cut and fill and the red elevations.
    app.zoom_to_bounds(&kentos_commands::Bounds { min_x: 200.0, min_y: 20.0, max_x: 360.0, max_y: 130.0 });
    shot(&mut app, "sevli-plan-yakin");
}

/// The case corridor checked as `road.corridors.query`'s shared case does it (docs/adr/0939): typed in the window, the
/// report's sampling is the reference's, and Kaydet writes the check.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("corridor.editor"));
    send(app, W::Choose(0));
    send(app, W::Line(1));
    send(app, W::RemoveRow);
    send(app, W::Region(0, 0, "100".into()));
    send(app, W::Region(0, 1, "260".into()));
    for (k, v) in [(6, "0,05"), (7, "0,05"), (8, "0,5")] {
        send(app, W::Field(k, v.into()));
    }
}

#[test]
fn the_window_checks_the_sampling_as_the_shared_case_does() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.corridors.query.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Denetim: plan ve kot birlikte, sık aralıkta").expect("the case");
    let want = &case["steps"][0]["result"]["output"]["corridors"][0]["sampling"];
    let mut app = app();
    checked(&mut app);
    let w = app.corridor_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("builds: {:?}", w.preview.why));
    let got = rep.sampling.as_ref().expect("checked");
    assert_eq!((got.measured, got.added, got.changes), (
        want["measured"].as_u64().unwrap() as u32,
        want["added"].as_u64().unwrap() as u32,
        want["changes"].as_u64().unwrap() as u32,
    ));
    let close = |a: f64, b: &serde_json::Value| (a - b.as_f64().unwrap()).abs() < 1e-6;
    assert!(close(got.before.plan, &want["before"]["plan"]) && close(got.after.height, &want["after"]["height"]), "{got:?}");
    assert!(rep.stations.iter().any(|s| s.why == "refine"));
    send(&mut app, W::Save);
    let r = model(&mut app).road_corridors()[0].refine.clone().expect("saved");
    assert_eq!((r.plan, r.height, r.least), (Some(0.05), Some(0.05), 0.5));
}

#[test]
fn a_tolerance_without_the_least_step_says_what_to_type() {
    let mut app = app();
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Field(7, "0,05".into()));
    let w = app.corridor_window.as_ref().expect("open");
    assert_eq!(w.preview.why.as_deref(), Some("Kesitleri denetlemek için en kısa adımı da yazın."));
}

/// Y33.3 (docs/adr/0939): the check typed in the window, its stations added in purple on the plan, wide and narrow,
/// light and dark, full screen; `.run/gui-evidence/yol-y33/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib corridor_window_tests::screens_sampling -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_sampling() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    use kentos_ui::widget::sizable::sizes;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y33");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name, full) in [
        ("light", Size::new(1440.0, 900.0), "acik", false),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar", false),
        ("light", Size::new(1920.0, 1080.0), "tam-ekran", true),
    ] {
        for step in ["once", "denetim"] {
            if full {
                sizes::set(crate::corridor_window::TITLE, sizes::Kept { width: 800.0, height: 600.0, maximized: true });
            } else {
                sizes::forget(crate::corridor_window::TITLE);
            }
            let mut app = app();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("corridor"));
            checked(&mut app);
            if step == "once" {
                send(&mut app, W::Field(8, String::new()));
                send(&mut app, W::Field(6, String::new()));
                send(&mut app, W::Field(7, String::new()));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("y33-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
    sizes::forget(crate::corridor_window::TITLE);
}

/// The templates' cut and fill slopes made `h`/1 through `road.templates.edit`: a slope this gentle meets the terrain
/// far out at a shallow angle (docs/adr/0951).
fn gentle(app: &mut App, h: f64) {
    let templates = model(app).road_templates().to_vec();
    for t in templates {
        let soft = |s: &kentos_commands::TemplateSideDef| kentos_commands::TemplateSideDef { cut: h, fill: h, ..s.clone() };
        let edit = kentos_commands::TemplatesEdit {
            operation: kentos_commands::RoadEditOperation::Set,
            template: Some(t.id.clone()),
            name: None,
            left: Some(soft(&t.left)),
            right: t.right.as_ref().map(soft),
            mirror: false,
            pavement: None,
            remove_pavement: false,
            expected_revision: None,
        };
        let r = kentos_native_application::road_templates_edit::execute(&mut kentos_native_application::ExecutionContext::new(model(app)), edit);
        assert!(matches!(r, kentos_commands::CommandResult::Completed { .. }), "{r:?}");
    }
}

/// Şev uçları reads the corridor as typed as `road.corridors.daylights` does: the shared case “Yatık şevler 12/1”.
#[test]
fn the_daylights_read_the_shared_case() {
    let mut app = app();
    gentle(&mut app, 12.0);
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(0));
    assert!(app.corridor_window.as_ref().unwrap().daylights.is_none(), "not read until shown");
    send(&mut app, W::Field(0, "60".into()));
    send(&mut app, W::Field(1, "60".into()));
    send(&mut app, W::Daylight(true));
    send(&mut app, W::Sensitive("10".into()));
    let (d, warnings) = app.corridor_window.as_ref().unwrap().daylights.clone().expect("shown").expect("reads");
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.corridors.daylights.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Yatık şevler 12/1: uzakta, yatık açıyla").expect("the case");
    let want = &case["steps"][0]["result"]["output"];
    let counts = serde_json::to_value(&d.counts).expect("serialises");
    assert_eq!(counts, want["counts"], "the counts are the command's");
    assert!((d.worst.as_ref().unwrap().sensitivity - want["worst"]["sensitivity"].as_f64().unwrap()).abs() < 1e-6);
    assert!(warnings.iter().any(|w| w.code == "corridor_daylight_crossings"));
    send(&mut app, W::Sensitive("sıfır".into()));
    let why = match app.corridor_window.as_ref().unwrap().daylights.clone() {
        Some(Err(why)) => why,
        other => panic!("a threshold that is no number: {other:?}"),
    };
    assert!(why.contains("Duyarlılık"), "{why}");
}

/// The daylights in Koridor, light and dark: the main corridor on 12/1 slopes and a 60 m swath, its sensitivity, its
/// bands and the sides worth a look; `.run/gui-evidence/yol-y45/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib corridor_window_tests::screens_daylights -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_daylights() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y45");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        for (h, file) in [(12.0, "yatik"), (0.0, "ortak")] {
            let mut app = app();
            if h > 0.0 {
                gentle(&mut app, h);
            }
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("corridor"));
            let _ = app.update(Message::Run("corridor.editor"));
            send(&mut app, W::Choose(0));
            send(&mut app, W::Field(0, "60".into()));
            send(&mut app, W::Field(1, "60".into()));
            send(&mut app, W::Daylight(true));
            send(&mut app, W::Sensitive(if h > 0.0 { "10" } else { "1,5" }.into()));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("y45-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// The drawing of `road.corridors.protected`'s cases: the corridors and five protected areas on two layers.
fn protected_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.corridors.protected.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Korunan alanlar reads the saved corridor as `road.corridors.protected` does: the shared case “Ana koridor ve
/// korunan alanlar”.
#[test]
fn the_protected_areas_read_the_shared_case() {
    let mut app = protected_app();
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Protected(true));
    let why = match app.corridor_window.as_ref().unwrap().protected.clone() {
        Some(Err(why)) => why,
        other => panic!("no layer yet: {other:?}"),
    };
    assert!(why.contains("katman"), "{why}");
    send(&mut app, W::ProtectedLayer("korunan".into()));
    let (z, warnings) = app.corridor_window.as_ref().unwrap().protected.clone().expect("shown").expect("reads");
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.corridors.protected.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Ana koridor ve korunan alanlar").expect("the case");
    let want = case["steps"][0]["result"]["output"]["areas"].as_array().unwrap().clone();
    assert_eq!(z.areas.len(), want.len());
    for (a, w) in z.areas.iter().zip(&want) {
        assert!((a.covered - w["covered"].as_f64().unwrap()).abs() < 1e-6, "#{} covered", a.entity);
        assert!((a.reach - w["reach"].as_f64().unwrap()).abs() < 1e-6, "#{} reach", a.entity);
    }
    assert!(warnings.iter().any(|w| w.code == "corridor_protected"));
    // Back to the plan: nothing read.
    send(&mut app, W::Daylight(false));
    assert!(app.corridor_window.as_ref().unwrap().protected.is_none());
}

/// The protected areas in Koridor, light and dark: the main corridor's footprint over five areas, the covered ones red;
/// `.run/gui-evidence/yol-y46/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib corridor_window_tests::screens_protected -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_protected() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y46");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = protected_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("corridor.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Protected(true));
        send(&mut app, W::ProtectedLayer("korunan".into()));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y46-korunan-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The preflight's shared cases' drawing for case `name` (its own setup, else the file's).
fn preflight_app(name: &str) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.corridors.preflight.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone();
    let setup = case.get("setup").cloned().unwrap_or_else(|| cases["setup"].clone());
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// ROAD-COR-13 (docs/adr/0956): the preflight's view reads `road.corridors.preflight` as its cases do: the tight
/// curve folds, its top surface lies over itself; the main corridor's two templates do not meet at their seam.
#[test]
fn the_preflight_reads_the_shared_cases() {
    let mut app = preflight_app("Dar kurp: katlanma ve üst üste binme");
    let _ = app.update(Message::Run("corridor.editor"));
    let at = app.corridor_window.as_ref().expect("open").rows.iter().position(|r| r.id == "c-3").expect("c-3");
    send(&mut app, W::Choose(at));
    send(&mut app, W::Check(true));
    let w = app.corridor_window.as_ref().expect("open");
    let Some(Ok((c, warnings))) = &w.preflight else { panic!("the check reads: {:?}", w.preflight) };
    assert!(!c.clear && !c.folds.is_empty() && !c.overlaps.is_empty());
    assert!(c.least.is_some_and(|m| m < 0.0));
    assert!(warnings.iter().any(|x| x.code == "corridor_overlap"));
    let mut app = preflight_app("Ana koridor: iki tip kesitin dikişi");
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Check(true));
    send(&mut app, W::Margin("0,95".into()));
    let w = app.corridor_window.as_ref().expect("open");
    let Some(Ok((c, _))) = &w.preflight else { panic!("the check reads") };
    assert!(c.seams.len() == 1 && !c.seams[0].meets && c.seams[0].step > 0.05);
    assert!(!c.tight.is_empty(), "a margin of 0.95 finds the curve tight");
}

/// The preflight over the plan: the tight curve light and wide, the main corridor's seam and tight curve dark and
/// narrow. `.run/gui-evidence/yol-y50/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib corridor_window_tests::screens_preflight -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_preflight() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y50");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (case, corridor, margin, theme, size, name) in [
        ("Dar kurp: katlanma ve üst üste binme", "c-3", "", "light", Size::new(1440.0, 900.0), "dar-kurp-acik-genis"),
        ("Ana koridor: iki tip kesitin dikişi", "c-1", "0,95", "light", Size::new(1440.0, 900.0), "dikis-acik-genis"),
        ("Ana koridor: iki tip kesitin dikişi", "c-1", "0,95", "dark", Size::new(1100.0, 650.0), "dikis-koyu-dar"),
        ("Yan koridor: düz aks", "c-2", "", "dark", Size::new(1440.0, 900.0), "temiz-koyu-genis"),
    ] {
        let mut app = preflight_app(case);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("corridor.editor"));
        let at = app.corridor_window.as_ref().expect("open").rows.iter().position(|r| r.id == corridor).expect("the corridor");
        send(&mut app, W::Choose(at));
        send(&mut app, W::Check(true));
        if !margin.is_empty() {
            send(&mut app, W::Margin(margin.into()));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y50-on-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Şev geçişi (Y54, docs/adr/0960; ROAD-GRADE-03): tk-1 gives way to its twin with flatter slopes at km 1+300; the
/// preflight finds the seam apart until 40 m is typed as the second region's slope transition, then the two meet;
/// Kaydet writes it with `road.corridors.edit`.
#[test]
fn a_slope_transition_closes_the_seam_and_is_written() {
    let mut app = preflight_app("Yatık şevli ikize geçişsiz dikiş");
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Check(true));
    let seam = |app: &App| {
        let w = app.corridor_window.as_ref().expect("open");
        let Some(Ok((c, _))) = &w.preflight else { panic!("the check reads: {:?}", w.preflight) };
        c.seams[0].clone()
    };
    let before = seam(&app);
    assert!(!before.meets && before.shift > 2.0, "{before:?}");
    send(&mut app, W::Region(1, 2, "40".into()));
    let after = seam(&app);
    assert!(after.meets && after.shift < 1e-9 && after.step < 1e-9, "{after:?}");
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_corridors()[0].regions[1].transition, Some(40.0));
    // A transition on the first region is refused and the window says why.
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Region(0, 2, "10".into()));
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_corridors()[0].regions[0].transition, None, "not written");
}

/// The slope transition typed and checked: the seam apart and met, the regions' table, light and wide, dark and
/// narrow. `.run/gui-evidence/yol-y54/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib corridor_window_tests::screens_transition -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_transition() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y54");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = preflight_app("Yatık şevli ikize geçişsiz dikiş");
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("corridor.editor"));
        send(&mut app, W::Choose(0));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y54-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        send(&mut app, W::Check(true));
        shot(&mut app, "dikis-gecissiz");
        send(&mut app, W::Region(1, 2, "40".into()));
        shot(&mut app, "dikis-gecisli");
        send(&mut app, W::Check(false));
        shot(&mut app, "bolgeler");
    }
}

/// A query case's own drawing (its setup), by its name.
fn app_of_case(name: &str) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.corridors.query.json")).expect("cases");
    let c = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(c["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

const PIVOT_CASE: &str = "İç kenar dönme ekseni yan değiştirir: km olur";
const WIDENING_CASE: &str = "Tek eğimli devere bağlı: genişlemenin başı km olur";

/// The superelevation's stations said in the window (docs/adr/0972): the pivot's two changes between rows on the side
/// corridor, the widening's start on the one-way crossfall.
#[test]
fn the_window_says_the_superelevation_s_stations() {
    let said = |app: &App| {
        let w = app.corridor_window.as_ref().expect("open");
        let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("builds: {:?}", w.preview.why));
        crate::corridor_window::superelevation_words(&rep.stations).unwrap_or_default()
    };
    let mut app = app_of_case(PIVOT_CASE);
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(1));
    assert_eq!(said(&app), "Dever km'leri: 2 dönme ekseni yan değiştirir");
    let mut app = app_of_case(WIDENING_CASE);
    let _ = app.update(Message::Run("corridor.editor"));
    send(&mut app, W::Choose(0));
    assert_eq!(said(&app), "Dever km'leri: 2 kritik · 1 genişleme", "the exit is past the region (km 300)");
}

/// `.run/gui-evidence/yol-y67/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib corridor_window_tests::screens_superelevation -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_superelevation() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y67");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        for (case, chosen, file) in [(PIVOT_CASE, 1, "donme-ekseni"), (WIDENING_CASE, 0, "genisleme")] {
            let mut app = app_of_case(case);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            let _ = app.update(Message::RibbonTab("corridor"));
            let _ = app.update(Message::Run("corridor.editor"));
            send(&mut app, W::Choose(chosen));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("y67-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
