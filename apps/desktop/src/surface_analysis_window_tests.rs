//! Yüzey analizi (surface_analysis_window.rs, plan/road.md Y18.5): the window reads a surface's slope ranges, two
//! surfaces' difference and the check points by class without writing; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::surface_analysis_window::{Event as W, Mode, Read};

/// `road.terrains.quality`'s drawing: the lattices (valley, pit, plane, terrace, ridge, cut, pad, sparse, spiky) and
/// the check points.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.quality.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// `road.terrains.repair`'s drawing: the same lattices and a plane with two spikes inside its squares.
fn repair_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.repair.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The spikes of the repair's plane asked.
fn spiky(app: &mut App) {
    let _ = app.update(Message::Run("terrain.analysis"));
    send(app, W::Mode(Mode::Quality));
    send(app, W::Surface("t-12".into()));
    send(app, W::Field(4, "2.5".into()));
    send(app, W::Field(5, "0.6".into()));
    send(app, W::Preview);
}

fn excluded(app: &App) -> usize {
    app.document.as_ref().and_then(|d| d.model.settings().road.as_ref()).map_or(0, |r| {
        r.surfaces.iter().find(|s| s.id == "t-12").map_or(0, |s| s.excluded.len())
    })
}

#[test]
fn the_spikes_are_left_out_after_their_difference_and_taken_back() {
    let mut app = repair_app();
    spiky(&mut app);
    let Read::Quality(q) = read(&app) else { panic!("quality") };
    assert_eq!(q.spike_count, Some(2), "{q:?}");
    send(&mut app, W::LeaveSpikes);
    let r = match &app.surface_analysis_window.as_ref().expect("open").repair {
        Some((_, Ok(r))) => r.clone(),
        other => panic!("a difference: {other:?}"),
    };
    assert!(r.cut > 0.0 && r.fill > 0.0 && r.corners_after + 2 == r.corners_before, "{r:?}");
    assert_eq!(excluded(&app), 0, "the difference writes nothing");
    send(&mut app, W::Apply);
    assert_eq!(excluded(&app), 2);
    let Read::Quality(q) = read(&app) else { panic!("quality") };
    assert_eq!(q.spike_count, Some(0), "read again without them: {q:?}");
    send(&mut app, W::RestoreAll);
    send(&mut app, W::Apply);
    assert_eq!(excluded(&app), 0);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(excluded(&app), 2);
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::SurfaceAnalysis(e));
}

fn read(app: &App) -> Read {
    match &app.surface_analysis_window.as_ref().expect("open").preview {
        Some(Ok(r)) => r.clone(),
        other => panic!("read: {other:?}"),
    }
}

#[test]
fn the_window_reads_ranges_a_difference_and_check_points() {
    let mut app = app();
    let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
    let _ = app.update(Message::Run("terrain.analysis"));
    assert_eq!(app.dialog, Some(Dialog::SurfaceAnalysis));
    // The breaks are the user's: none asked, none read.
    send(&mut app, W::Preview);
    assert!(matches!(&app.surface_analysis_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("Bir şey sorun")));
    send(&mut app, W::Surface("t-6".into()));
    send(&mut app, W::Field(0, "0, 1, 12, 35".into()));
    send(&mut app, W::Roughness(true));
    send(&mut app, W::Preview);
    let Read::Analysis(a) = read(&app) else { panic!("an analysis") };
    let slopes = a.slopes.expect("slopes");
    assert_eq!(slopes.bands.len(), 3);
    assert!((slopes.bands.iter().map(|b| b.area).sum::<f64>() + slopes.outside_area - a.area).abs() < 1e-6);
    assert!(slopes.bands.iter().any(|b| !b.parts.is_empty()) && a.roughness.is_some());
    // The difference of the pad over the terrace.
    send(&mut app, W::Mode(Mode::Compare));
    send(&mut app, W::Comparison("t-9".into()));
    send(&mut app, W::Field(2, "-0.1, 0, 0.1, 0.2".into()));
    send(&mut app, W::Preview);
    let Read::Compare(c) = read(&app) else { panic!("a difference") };
    assert!(c.fill > 0.0 && c.cut > 0.0 && c.bands.len() == 3, "{c:?}");
    // The check points by class.
    send(&mut app, W::Mode(Mode::Quality));
    send(&mut app, W::Checks(Some("kontrol".into())));
    send(&mut app, W::Field(7, "Sınıf".into()));
    send(&mut app, W::Preview);
    let Read::Quality(q) = read(&app) else { panic!("quality") };
    let checks = q.checkpoints.expect("check points");
    assert_eq!(checks.classes.iter().map(|c| c.class.as_str()).collect::<Vec<_>>(), ["", "bitkili", "sert"]);
    assert_eq!(checks.all.outside, 1);
    // Spikes need both values.
    send(&mut app, W::Field(4, "1".into()));
    send(&mut app, W::Preview);
    assert!(matches!(&app.surface_analysis_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("birlikte")));
    send(&mut app, W::Close);
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision().to_string(), before);
}

/// The window, wide and narrow, light and dark, each way; `.run/gui-evidence/yol-y18-yuzey-analizi-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib surface_analysis_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y18-yuzey-analizi-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Surface("t-3".into()));
        send(&mut app, W::Measure(0));
        send(&mut app, W::Field(0, "100, 125, 150, 175, 200, 225, 250".into()));
        send(&mut app, W::Roughness(true));
        send(&mut app, W::Field(1, "30".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "kot");
        send(&mut app, W::Surface("t-6".into()));
        send(&mut app, W::Measure(1));
        send(&mut app, W::Field(0, "0, 1, 5, 12, 25, 35".into()));
        send(&mut app, W::Field(1, String::new()));
        send(&mut app, W::Preview);
        shot(&mut app, "egim");
        send(&mut app, W::Mode(Mode::Compare));
        send(&mut app, W::Comparison("t-9".into()));
        send(&mut app, W::Field(2, "-0.1, -0.05, 0, 0.05, 0.1, 0.15, 0.2".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "fark");
        send(&mut app, W::Mode(Mode::Quality));
        send(&mut app, W::Checks(Some("kontrol".into())));
        send(&mut app, W::Field(7, "Sınıf".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "kontrol");
        let mut app = repair_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("terrain"));
        spiky(&mut app);
        send(&mut app, W::LeaveSpikes);
        shot(&mut app, "onarim");
    }
}

/// `road.terrains.coverage`'s drawing: the lattices and the boundaries over them.
fn coverage_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.coverage.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The coverage fixture's case input by its name.
fn coverage_case(name: &str) -> serde_json::Value {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.coverage.json")).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case")["steps"][0]["input"].clone()
}

#[test]
fn the_window_reads_a_surface_s_coverage_of_a_boundary() {
    let mut app = coverage_app();
    let _ = app.update(Message::Run("terrain.analysis"));
    send(&mut app, W::Mode(Mode::Coverage));
    send(&mut app, W::Surface("t-10".into()));
    send(&mut app, W::Preview);
    let Read::Coverage(c, pic) = read(&app) else { panic!("coverage") };
    assert_eq!((c.covered, c.estimated, pic.triangles.len()), (36.0, 0.0, 30));
    let input = coverage_case("Seyrek yüzey proje sınırında");
    send(&mut app, W::Boundary(input["boundary"].as_str().map(str::to_owned)));
    send(&mut app, W::Field(8, "3".into()));
    send(&mut app, W::Preview);
    let Read::Coverage(c, pic) = read(&app) else { panic!("coverage") };
    assert_eq!(c.estimated_triangles, 6);
    assert!((c.no_data.expect("a boundary") - (c.boundary_area.expect("a boundary") - c.covered)).abs() < 1e-9);
    assert_eq!((pic.rings.len(), pic.triangles.iter().filter(|t| t.1).count()), (1, 6));
    assert!(app.surface_analysis_window.as_ref().expect("open").warnings.iter().any(|w| w.contains("örtmüyor")));
    // The estimated area drawn: its outline, its area's text and the surface's outline, one undo step.
    let count = |app: &App| app.document.as_ref().expect("a drawing").model.entities().count();
    let n = count(&app);
    send(&mut app, W::Layer("kapsam".into()));
    send(&mut app, W::DrawLow);
    assert_eq!(count(&app), n + 3);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), n);
    let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
    send(&mut app, W::Close);
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision().to_string(), before);
}

/// Kapsam, the sparse surface in its boundary and the terrace under an L, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y19-kapsam-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib surface_analysis_window_tests::coverage_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn coverage_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = coverage_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y19-kapsam-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Mode(Mode::Coverage));
        send(&mut app, W::Surface("t-10".into()));
        let input = coverage_case("Seyrek yüzey proje sınırında");
        send(&mut app, W::Boundary(input["boundary"].as_str().map(str::to_owned)));
        send(&mut app, W::Field(8, "3".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "seyrek");
        send(&mut app, W::Surface("t-6".into()));
        let input = coverage_case("L biçimli sınır teras üstünde");
        send(&mut app, W::Boundary(input["boundary"].as_str().map(str::to_owned)));
        send(&mut app, W::Field(8, String::new()));
        send(&mut app, W::Preview);
        shot(&mut app, "teras");
        send(&mut app, W::Surface("t-12".into()));
        send(&mut app, W::Boundary(None));
        send(&mut app, W::Field(8, "3".into()));
        send(&mut app, W::Layer("kapsam".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "delikli");
    }
}

/// The density fixture's case by its name.
fn density_case(file: &str, name: &str) -> serde_json::Value {
    let text = match file {
        "density" => include_str!("../../../fixtures/commands/v1/road.terrains.density.json"),
        _ => include_str!("../../../fixtures/commands/v1/road.terrains.sparse.json"),
    };
    let cases: serde_json::Value = serde_json::from_str(text).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case")["steps"][0].clone()
}

#[test]
fn the_window_counts_a_surface_s_points_in_cells_and_draws_the_sparse_ones() {
    let mut app = coverage_app();
    let _ = app.update(Message::Run("terrain.analysis"));
    send(&mut app, W::Mode(Mode::Density));
    send(&mut app, W::Surface("t-10".into()));
    send(&mut app, W::Preview);
    assert!(matches!(&app.surface_analysis_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("hücre boyunu")));
    let case = density_case("density", "Proje sınırında");
    send(&mut app, W::Boundary(case["input"]["boundary"].as_str().map(str::to_owned)));
    send(&mut app, W::Field(9, "1.5".into()));
    send(&mut app, W::Field(10, "1".into()));
    send(&mut app, W::Preview);
    let Read::Density(d, pic) = read(&app) else { panic!("density") };
    let want = &case["result"]["output"];
    assert_eq!((d.cells, d.filled, d.sparse), (want["cells"].as_u64().unwrap() as u32, want["filled"].as_u64().unwrap() as u32, Some(want["sparse"].as_u64().unwrap() as u32)));
    assert_eq!((pic.counts.iter().flatten().count(), pic.rings.len()), (d.cells as usize, 1));
    // A share not met warns.
    send(&mut app, W::Field(11, "0.9".into()));
    send(&mut app, W::Preview);
    assert!(app.surface_analysis_window.as_ref().expect("open").warnings.iter().any(|w| w.contains("istenen %90.0")));
    // The sparse cells drawn: as many objects as the shared case writes, one undo step.
    let case = density_case("sparse", "Seyrek yüzeyin noktasız hücreleri");
    send(&mut app, W::Boundary(None));
    send(&mut app, W::Field(9, "1".into()));
    send(&mut app, W::Field(11, String::new()));
    let count = |app: &App| app.document.as_ref().expect("a drawing").model.entities().count();
    let n = count(&app);
    send(&mut app, W::Layer("kapsam".into()));
    send(&mut app, W::DrawSparse);
    // The window draws with the area labels on; the case draws without them.
    let rings = case["result"]["output"]["uids"].as_array().expect("uids").len();
    let areas = case["result"]["output"]["areas"].as_u64().expect("areas") as usize;
    assert_eq!(count(&app), n + rings + areas);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(count(&app), n);
}

/// Yoğunluk, the sparse surface in its boundary and the holed lattice, wide and narrow, light and dark, and the sparse
/// areas drawn; `.run/gui-evidence/yol-y19-yogunluk-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib surface_analysis_window_tests::density_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn density_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = coverage_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y19-yogunluk-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Mode(Mode::Density));
        send(&mut app, W::Surface("t-10".into()));
        let case = density_case("density", "Yüzeyi aşan sınır");
        send(&mut app, W::Boundary(case["input"]["boundary"].as_str().map(str::to_owned)));
        send(&mut app, W::Field(9, "1.5".into()));
        send(&mut app, W::Field(10, "1".into()));
        send(&mut app, W::Field(11, "0.9".into()));
        send(&mut app, W::Layer("kapsam".into()));
        send(&mut app, W::Preview);
        shot(&mut app, "seyrek");
        send(&mut app, W::Surface("t-6".into()));
        let case = density_case("density", "L biçimli sınır");
        send(&mut app, W::Boundary(case["input"]["boundary"].as_str().map(str::to_owned)));
        send(&mut app, W::Field(9, "2".into()));
        send(&mut app, W::Field(10, "4".into()));
        send(&mut app, W::Field(11, String::new()));
        send(&mut app, W::Preview);
        shot(&mut app, "teras");
        send(&mut app, W::DrawSparse);
        send(&mut app, W::Close);
        shot(&mut app, "cizildi");
    }
}

/// Y39.3 (docs/adr/0945): Kalite reads each class's robust measures as `road.terrains.quality`'s shared case does,
/// with the check points' own accuracy combined; the far point is flagged.
#[test]
fn the_check_points_are_read_beyond_one_rmse() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.quality.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Kontrol noktalarının kendi doğruluğu 1 cm");
    let want = &case.expect("the case")["steps"][0]["result"]["output"]["checkpoints"];
    let mut app = app();
    let _ = app.update(Message::Run("terrain.analysis"));
    send(&mut app, W::Mode(Mode::Quality));
    send(&mut app, W::Surface("Teras".into()));
    send(&mut app, W::Checks(Some("kontrol".into())));
    send(&mut app, W::Field(7, "Sınıf".into()));
    send(&mut app, W::Field(12, "0,01".into()));
    send(&mut app, W::Preview);
    let Read::Quality(q) = read(&app) else { panic!("quality") };
    let c = q.checkpoints.as_ref().expect("check points");
    let close = |a: f64, v: &serde_json::Value| (a - v.as_f64().unwrap()).abs() <= 1e-9 * v.as_f64().unwrap().abs().max(1.0);
    for (k, w) in c.classes.iter().zip(want["classes"].as_array().unwrap()) {
        // A class of one point has no robust measures, here as in the case.
        let Some(r) = k.robust.as_ref() else {
            assert!(w.get("robust").is_none(), "{}", k.class);
            continue;
        };
        let wr = &w["robust"];
        assert!(close(r.nmad, &wr["nmad"]) && close(r.q95, &wr["q95"]) && close(r.rmse_high, &wr["rmseHigh"]), "{}", k.class);
        assert!(close(r.combined.expect("combined"), &wr["combined"]));
    }
    assert!(c.points.iter().any(|p| p.far), "the far point is flagged");
}

/// Kalite with the robust measures, light and dark; `.run/gui-evidence/yol-y39/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib surface_analysis_window_tests::screens_robust -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_robust() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y39");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Mode(Mode::Quality));
        send(&mut app, W::Surface("t-6".into()));
        send(&mut app, W::Checks(Some("kontrol".into())));
        send(&mut app, W::Field(7, "Sınıf".into()));
        send(&mut app, W::Field(12, "0,01".into()));
        send(&mut app, W::Preview);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y39-kontrol-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// docs/adr/1600: Fark by a grid's cells beside the exact TIN, and the cut and fill factors apart from the geometric
/// volumes; a grid without a cell is said; in a KentOS Harita project as in Yol.
#[test]
fn fark_measures_by_a_grid_beside_the_exact_tin_and_adjusts_by_the_factors() {
    let mut app = app();
    let _ = app.update(Message::Run("terrain.analysis"));
    send(&mut app, W::Mode(Mode::Compare));
    send(&mut app, W::Surface("t-6".into()));
    send(&mut app, W::Comparison("t-9".into()));
    send(&mut app, W::Preview);
    let Read::Compare(tin) = read(&app) else { panic!("a difference") };
    assert!(tin.exact.is_none() && tin.cell.is_none() && tin.adjusted.is_none());
    send(&mut app, W::VolumeGrid(true));
    send(&mut app, W::Preview);
    assert!(matches!(&app.surface_analysis_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("hücre")));
    send(&mut app, W::Field(13, "0,25".into()));
    send(&mut app, W::Field(14, "1,25".into()));
    send(&mut app, W::Field(15, "1,1".into()));
    send(&mut app, W::Preview);
    let Read::Compare(g) = read(&app) else { panic!("a difference") };
    assert_eq!(g.cell, Some(0.25));
    let exact = g.exact.clone().expect("the exact TIN beside");
    assert!((exact.cut - tin.cut).abs() < 1e-9 && (exact.fill - tin.fill).abs() < 1e-9);
    let a = g.adjusted.clone().expect("adjusted");
    assert!((a.cut - g.cut * 1.25).abs() < 1e-9 && (a.fill - g.fill * 1.1).abs() < 1e-9);
}

/// Fark with a grid and the factors; `.run/gui-evidence/hacim-yontemi-1600/`.
#[test]
#[ignore = "writes pictures"]
fn screens_volume_method() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/hacim-yontemi-1600");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Mode(Mode::Compare));
        send(&mut app, W::Surface("t-5".into()));
        send(&mut app, W::Comparison("t-8".into()));
        send(&mut app, W::VolumeGrid(true));
        send(&mut app, W::Field(13, "2".into()));
        send(&mut app, W::Field(14, "1,25".into()));
        send(&mut app, W::Field(15, "1,1".into()));
        send(&mut app, W::Preview);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("fark-izgara-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// `road.terrains.zeroline`'s drawing (the compare lattices and a mound on the plane).
fn zero_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.zeroline.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// docs/adr/1620: Fark's Sıfır hattını çiz writes the mound's closed zero line over the plane in one undo step.
#[test]
fn fark_draws_the_zero_line_in_one_step() {
    let mut app = zero_app();
    let _ = app.update(Message::Run("terrain.analysis"));
    send(&mut app, W::Mode(Mode::Compare));
    send(&mut app, W::Surface("t-5".into()));
    send(&mut app, W::Comparison("t-10".into()));
    send(&mut app, W::Layer("sifir".into()));
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::DrawZero);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.entities().count(), before + 1, "{}", crate::files_testing::last_said(&app));
    let line = doc.entities().last().expect("the line");
    assert_eq!(line.base().layer_id, "sifir");
    assert!(matches!(line, kentos_commands::Entity::Polyline(p) if p.closed));
    assert!(crate::files_testing::last_said(&app).contains("1 çizgi (1 kapalı)"));
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Sıfır hattı"));
}

/// Fark with the zero line drawn; `.run/gui-evidence/sifir-hatti-1620/`.
#[test]
#[ignore = "writes pictures"]
fn screens_zero_line() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/sifir-hatti-1620");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = zero_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Mode(Mode::Compare));
        send(&mut app, W::Surface("t-5".into()));
        send(&mut app, W::Comparison("t-10".into()));
        send(&mut app, W::Layer("sifir".into()));
        send(&mut app, W::Preview);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-fark-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        send(&mut app, W::DrawZero);
        send(&mut app, W::Close);
        // The plane, the mound and the line: framed on them.
        let slots: Vec<kentos_domain::Slot> = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter(|e| ["sifir", "tumsek"].contains(&e.base().layer_id.as_str()))
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect();
        app.selection.set(slots);
        let _ = app.update(Message::Run("view.zoomSelection"));
        app.selection.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-sifir-hatti-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// `road.terrains.compare`'s drawing as a case of it sets it (docs/adr/1029: “Kazı” a cloud's ground).
fn compared_as(case: &str) -> (App, serde_json::Value) {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.compare.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == case).expect("the case").clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    (app, case)
}

/// ROAD-NUM-04 (docs/adr/1029): two surfaces whose heights are not shown to rest on one vertical system are not
/// compared, the window says why; Kotları aynı say › Var reads the shared case's draft, its warning shown.
#[test]
fn the_window_compares_no_heights_not_shown_one() {
    let (mut app, _) = compared_as("Bulutun düşey sistemi bilinmiyor: ret");
    let (_, draft) = compared_as("Varsayımla taslak");
    let want = &draft["steps"][0]["result"]["output"];
    let _ = app.update(Message::Run("terrain.analysis"));
    send(&mut app, W::Mode(Mode::Compare));
    send(&mut app, W::Surface("t-5".into()));
    send(&mut app, W::Comparison("t-8".into()));
    send(&mut app, W::Preview);
    assert!(matches!(&app.surface_analysis_window.as_ref().expect("open").preview, Some(Err(why)) if why.contains("gösterilemiyor")));
    let m = app
        .surface_analysis_control(crate::traces::Control::Pick(crate::surface_analysis_window::SAME_HEIGHTS, "Var"))
        .expect("a control")
        .expect("a message");
    let _ = app.update(m);
    send(&mut app, W::Preview);
    let Read::Compare(c) = read(&app) else { panic!("a difference") };
    assert!((c.cut - want["cut"].as_f64().unwrap()).abs() < 1e-6, "{c:?}");
    let w = app.surface_analysis_window.as_ref().expect("open");
    assert!(w.warnings.first().is_some_and(|m| m.contains("varsayıma dayanan bir taslak")), "{:?}", w.warnings);
}

/// The comparison refused and as a draft, light; `.run/gui-evidence/yol-num04/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib surface_analysis_window_tests::screens_heights -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_heights() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-num04");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (assume, step) in [(false, "ret"), (true, "taslak")] {
        let (mut app, _) = compared_as("Bulutun düşey sistemi bilinmiyor: ret");
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Mode(Mode::Compare));
        send(&mut app, W::Surface("t-5".into()));
        send(&mut app, W::Comparison("t-8".into()));
        send(&mut app, W::AssumeHeights(assume));
        send(&mut app, W::Preview);
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("num04-fark-{step}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// `road.terrains.cutfill`'s drawing (the zero line's with a bank and the plane and the mound at TM coordinates).
fn cutfill_app() -> (App, serde_json::Value) {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.terrains.cutfill.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    (app, cases)
}

/// docs/adr/1840: Fark's Kazı ve dolgu bölgelerini çiz writes the mound's zones over the plane in 0.2 m bands as the
/// shared case says, each a closed area with its kind, area and volume, the cut red; one undo step; the drawing kept.
#[test]
fn fark_draws_the_cut_and_fill_zones_in_one_step() {
    let (mut app, cases) = cutfill_app();
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Tümsek 0,2 m'lik bantlarla: on bir bölge").expect("the case");
    let want = case["steps"][0]["result"]["output"]["zones"].as_array().expect("zones").clone();
    let _ = app.update(Message::Run("terrain.analysis"));
    send(&mut app, W::Mode(Mode::Compare));
    send(&mut app, W::Surface("t-5".into()));
    send(&mut app, W::Comparison("t-10".into()));
    send(&mut app, W::Layer("kazi-dolgu".into()));
    send(&mut app, W::Field(16, "0,2".into()));
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::DrawZones);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.entities().count(), before + 2 * want.len(), "{}", crate::files_testing::last_said(&app));
    // Each zone its solid hatch, then its closed area.
    assert!(matches!(doc.entities().nth(before), Some(kentos_commands::Entity::Hatch(_))));
    let first = doc.entities().nth(before + 1).expect("the first zone");
    assert_eq!(first.base().layer_id, "kazi-dolgu");
    assert_eq!(first.base().attrs.get("Tür").map(String::as_str), Some("Kazı"));
    let area: f64 = first.base().attrs.get("Alan (m²)").and_then(|a| a.parse().ok()).expect("its area");
    assert!((area - want[0]["area"].as_f64().unwrap()).abs() < 1e-6);
    assert!(matches!(first, kentos_commands::Entity::Polygon(_)));
    assert!(crate::files_testing::last_said(&app).contains("9 kazı"), "{}", crate::files_testing::last_said(&app));
    let kept = doc.settings().road.as_ref().map(|r| r.drawings.iter().filter(|d| d.command == "road.terrains.cutfill").count());
    assert_eq!(kept, Some(1), "the drawing kept with its surfaces");
    let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("saves");
    assert!(bytes.windows(10).any(|w| w == b"\x67version\x18\xcd"), "schema 205");
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Kazı ve dolgu bölgeleri"));
}

/// docs/adr/2340: Bölgelere Yazı and Miktar tablosu ticked, Kazı ve dolgu bölgelerini çiz writes each zone's label
/// (the shared case's words and places) and the quantity table (the case's cells) with the zones, one undo step.
#[test]
fn fark_writes_the_zones_labels_and_quantity_table() {
    let (mut app, cases) = cutfill_app();
    let find = |name: &str| {
        cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).expect("the case")["steps"][0]["result"]
            ["output"]
            .clone()
    };
    let labels = find("Bölge yazıları: her bölgenin türü ve hacmi içinde");
    let table = find("Miktar tablosu: bölgeler, kazı, dolgu ve net");
    let _ = app.update(Message::Run("terrain.analysis"));
    send(&mut app, W::Mode(Mode::Compare));
    send(&mut app, W::Surface("t-5".into()));
    send(&mut app, W::Comparison("t-10".into()));
    send(&mut app, W::Layer("kazi-dolgu".into()));
    send(&mut app, W::ZoneLabels(true));
    send(&mut app, W::ZoneTable(true));
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    send(&mut app, W::DrawZones);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let zones = labels["zones"].as_array().map_or(0, Vec::len);
    let written: Vec<&kentos_commands::Entity> = doc.entities().skip(before).collect();
    assert_eq!(written.len(), 2 * zones + zones + 1, "{}", crate::files_testing::last_said(&app));
    let texts: Vec<(String, f64, f64)> = written
        .iter()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some((t.text.clone(), t.p.x, t.p.y)),
            _ => None,
        })
        .collect();
    for (got, want) in texts.iter().zip(labels["labels"].as_array().expect("labels")) {
        assert_eq!(got.0, want["text"].as_str().unwrap());
        assert!((got.1 - want["at"][0].as_f64().unwrap()).abs() < 1e-9 && (got.2 - want["at"][1].as_f64().unwrap()).abs() < 1e-9);
    }
    let Some(kentos_commands::Entity::Table(t)) = written.last() else { panic!("the table last") };
    assert_eq!(serde_json::to_value(&t.cells).expect("cells"), table["table"]);
    let undone = app.document.as_mut().and_then(|d| d.model.undo());
    assert_eq!(undone.as_deref(), Some("Kazı ve dolgu bölgeleri"));
}

/// The zones with their labels and quantity table at 1:100 (the fixture's 1:1000 would make 2.5 m texts on 10 m
/// zones); `.run/gui-evidence/bolge-yazisi-2340/`.
#[test]
#[ignore = "writes pictures"]
fn screens_zone_labels() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bolge-yazisi-2340");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let (mut app, _) = cutfill_app();
        {
            let model = &mut app.document.as_mut().expect("a drawing").model;
            let mut settings = model.settings().clone();
            settings.plot_scale = 100.0;
            model.set_settings(settings);
        }
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::Run("terrain.analysis"));
        send(&mut app, W::Mode(Mode::Compare));
        send(&mut app, W::Surface("t-5".into()));
        send(&mut app, W::Comparison("t-10".into()));
        send(&mut app, W::Layer("kazi-dolgu".into()));
        send(&mut app, W::ZoneLabels(true));
        send(&mut app, W::ZoneTable(true));
        snapshot.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("1-fark-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
        println!("{}", path.display());
        send(&mut app, W::DrawZones);
        send(&mut app, W::Close);
        snapshot.settle(&mut app, App::view, &mut update);
        let b = app.viewport.bounds;
        // The zones' 10 m square and the table to their right.
        app.viewport.camera.center.x = 6011.0;
        app.viewport.camera.center.y = 5.0;
        app.viewport.camera.scale = f64::from(b.width.min(b.height)) / 16.0;
        snapshot.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("2-bolgeler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
        println!("{}", path.display());
    }
}

/// Fark's cut and fill zones; `.run/gui-evidence/kazi-dolgu-1840/`.
#[test]
#[ignore = "writes pictures"]
fn screens_cut_fill() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kazi-dolgu-1840");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        for (interval, file) in [("", "iki"), ("0,2", "bantli")] {
            let (mut app, _) = cutfill_app();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            let _ = app.update(Message::RibbonTab("terrain"));
            let _ = app.update(Message::Run("terrain.analysis"));
            send(&mut app, W::Mode(Mode::Compare));
            send(&mut app, W::Surface("t-5".into()));
            send(&mut app, W::Comparison("t-10".into()));
            send(&mut app, W::Layer("kazi-dolgu".into()));
            send(&mut app, W::Field(16, interval.into()));
            send(&mut app, W::Preview);
            if interval.is_empty() {
                snapshot.settle(&mut app, App::view, &mut update);
                let path = out.join(format!("1-fark-{name}.png"));
                snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
                println!("{}", path.display());
            }
            send(&mut app, W::DrawZones);
            send(&mut app, W::Close);
            // Framed on the mound's 10 m square (the drawing also holds lattices at TM coordinates, far away).
            snapshot.settle(&mut app, App::view, &mut update);
            let b = app.viewport.bounds;
            app.viewport.camera.center.x = 6005.0;
            app.viewport.camera.center.y = 5.0;
            app.viewport.camera.scale = f64::from(b.width.min(b.height)) / 12.0;
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("2-bolgeler-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}
