//! Kübaj (volume_window.rs, plan/road.md Y8): the window computes the volume as it is typed, writes through
//! `road.volumes.edit` (one undo step) and writes the sheet as a file; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::volume_window::Event as W;

/// A road project with `road.volumes.query`'s drawing: two corridors and two earthwork volumes.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Volumes(e));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

#[test]
fn the_window_makes_a_volume_and_shows_it_computed() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.new"));
    assert_eq!(app.dialog, Some(Dialog::Volumes));
    send(&mut app, W::Name("Prizmoidal kübaj".into()));
    send(&mut app, W::Method(kentos_commands::VolumeMethod::Prismoidal));
    send(&mut app, W::Add);
    send(&mut app, W::Factor(0, 1, "600".into()));
    send(&mut app, W::Factor(0, 2, "1,25".into()));
    send(&mut app, W::Factor(0, 3, "0,9".into()));
    let w = app.volume_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("computes: {:?}", w.preview.why));
    assert!(rep.rows.len() > 20 && rep.cut > 1000.0, "{} rows, {} m³", rep.rows.len(), rep.cut);
    assert!((rep.factored_cut - 1.25 * rep.cut).abs() < 1e-6, "{rep:?}");
    send(&mut app, W::Save);
    assert!(app.volume_window.is_none(), "Kaydet closes the window");
    let doc = model(&mut app);
    assert_eq!(doc.road_volumes().len(), 3);
    assert_eq!(doc.road_volumes()[2].factors[0].cut, Some(1.25));
    assert_eq!(doc.undo().as_deref(), Some("Kübaj"));
    assert_eq!(doc.road_volumes().len(), 2);
}

#[test]
fn a_saved_volume_keeps_its_corridor_and_a_bad_factor_is_refused() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Corridor("c-2".into()));
    let w = app.volume_window.as_ref().expect("open");
    assert_eq!(w.preview.report.as_ref().map(|r| r.corridor.as_str()), Some("c-1"), "the corridor stays");
    send(&mut app, W::Add);
    send(&mut app, W::Factor(0, 2, "x".into()));
    assert!(app.volume_window.as_ref().is_some_and(|w| w.preview.why.is_some()));
    send(&mut app, W::Save);
    assert!(app.volume_window.is_some(), "the window stays");
    assert_eq!(model(&mut app).road_volumes()[0].factors.len(), 2);
}

#[test]
fn stripping_and_benching_are_typed_and_saved() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let before = app.volume_window.as_ref().and_then(|w| w.preview.report.as_ref()).map(|r| r.cut).unwrap_or_default();
    send(&mut app, W::AddStrip);
    send(&mut app, W::Strip(0, 1, "600".into()));
    send(&mut app, W::Strip(0, 2, "0,3".into()));
    send(&mut app, W::Bench(0, "3".into()));
    send(&mut app, W::Bench(1, "0,5".into()));
    let w = app.volume_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("computes: {:?}", w.preview.why));
    assert!(rep.strip.is_some_and(|v| v > 100.0) && rep.bench.is_some_and(|v| v > 10.0), "{:?} {:?}", rep.strip, rep.bench);
    assert!(rep.cut < before, "the stripped ground leaves less cut: {} → {}", before, rep.cut);
    send(&mut app, W::Save);
    let doc = model(&mut app);
    assert_eq!(doc.road_volumes()[0].strip[0].depth, 0.3);
    assert_eq!(doc.road_volumes()[0].bench.as_ref().map(|b| b.height), Some(0.5));
    assert_eq!(doc.undo().as_deref(), Some("Kübaj"));
}

/// The main corridor sampled more densely: the approved volume's sources change.
fn densify(app: &mut App) {
    let input = kentos_commands::CorridorsEdit {
        operation: kentos_commands::RoadEditOperation::Set,
        corridor: Some("c-1".into()),
        name: None,
        alignment: None,
        profile: None,
        surface: None,
        superelevation: None,
        left: None,
        right: None,
        tangent: Some(20.0),
        curve: None,
        sag: None,
        vertical_sag: None,
        refine: None,
        profile_points: None,
        critical: None,
        stations: None,
        regions: None,
        expected_revision: None,
    };
    let mut cx = kentos_native_application::ExecutionContext::new(model(app));
    assert!(matches!(
        kentos_native_application::road_corridors_edit::execute(&mut cx, input),
        kentos_commands::CommandResult::Completed { .. }
    ));
}

#[test]
fn an_approved_volume_stays_and_says_how_it_changed() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Approve(true));
    assert!(model(&mut app).road_volumes()[0].approved.is_some());
    assert_eq!(model(&mut app).undo().as_deref(), Some("Kübaj onayı"));
    let _ = model(&mut app).redo();
    // The corridor changes: the approval stays and the report says how today's figures differ.
    send(&mut app, W::Cancel);
    densify(&mut app);
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let w = app.volume_window.as_ref().expect("open");
    let approval = w.preview.report.as_ref().and_then(|r| r.approval.as_ref()).expect("an approval");
    assert!(approval.changed && !approval.rows.is_empty(), "{approval:?}");
    assert!(w.preview.warnings.iter().any(|x| x.code == "volume_changed"));
    // Docs/adr/1041: the approval kept what it was computed from; the corridor written again moved since.
    assert_eq!(approval.approved.algorithm.as_deref(), Some("kentos.end-area/1"));
    assert_eq!(approval.approved.sources.as_ref().map(|s| s.corridor.revision), Some(1));
    let moved: Vec<_> = approval.moved.iter().map(|m| (m.source, m.id.as_str(), m.then.as_deref(), m.now.as_deref())).collect();
    assert_eq!(moved, vec![(kentos_commands::VolumeSourceKind::Corridor, "c-1", Some("1"), Some("2"))]);
    assert_eq!(crate::volume_window::approval_sources_words(approval), " Onaydan beri değişen: koridor c-1 (1 → 2).");
    // Approved, its definition does not change.
    send(&mut app, W::Name("Başka".into()));
    send(&mut app, W::Save);
    assert!(app.volume_window.is_some(), "refused: the window stays");
    assert_eq!(model(&mut app).road_volumes()[0].name, "Ana kübaj");
}

#[test]
fn the_sheet_is_written_as_a_workbook_and_a_csv() {
    let mut app = app();
    let doc = model(&mut app);
    let def = doc.road_volumes()[0].clone();
    let (report, _) = kentos_native_application::road_volumes::preview(doc, &def).expect("computes");
    let sheet = kentos_native_application::road_volumes::sheet(&report);
    assert_eq!(sheet.rows.len(), report.rows.len() + 1, "a row a stretch and the totals");
    assert_eq!(sheet.rows.last().map(|r| r[0].as_str()), Some("Toplam"));
    let folder = std::env::temp_dir().join(format!("kentos-kubaj-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    for (format, name) in [(kentos_commands::WorkbookFormat::Xlsx, "kubaj.xlsx"), (kentos_commands::WorkbookFormat::Csv, "kubaj.csv")] {
        let path = folder.join(name);
        match crate::table_write::write(vec![sheet.clone()], format, &path) {
            crate::table_write::Outcome::Written { rows, .. } => assert_eq!(rows, report.rows.len() + 1),
            crate::table_write::Outcome::Failed(why) => panic!("{why}"),
        }
        assert!(std::fs::metadata(&path).is_ok_and(|m| m.len() > 200), "{name} written");
    }
    let csv = std::fs::read_to_string(folder.join("kubaj.csv")).expect("reads");
    // The writer's CSV opens with a byte order mark, for Excel (docs/adr/0223 §2).
    assert!(csv.trim_start_matches('\u{feff}').starts_with("Başlangıç km;Bitiş km;Uzunluk (m);Kazı (m³)"), "{csv}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The window, wide and narrow, light and dark; `.run/gui-evidence/yol-y8-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib volume_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y8-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        shot(&mut app, "kubaj");
        // Between the surfaces of the side corridor, the stale one.
        send(&mut app, W::Show(true));
        shot(&mut app, "kubaj-cetvel");
        // Stripped and benched: the topsoil and the benches in the sheet.
        send(&mut app, W::AddStrip);
        send(&mut app, W::Strip(0, 1, "600".into()));
        send(&mut app, W::Strip(0, 2, "0,3".into()));
        send(&mut app, W::Bench(0, "3".into()));
        send(&mut app, W::Bench(1, "0,5".into()));
        shot(&mut app, "kubaj-siyirma");
        send(&mut app, W::Choose(1));
        send(&mut app, W::Show(false));
        shot(&mut app, "kubaj-yuzey");
        send(&mut app, W::Cancel);
        // Approved, then its corridor sampled more densely: the window says how today's figures differ.
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Approve(true));
        send(&mut app, W::Cancel);
        densify(&mut app);
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        shot(&mut app, "kubaj-onay");
        send(&mut app, W::Cancel);
    }
}

/// The corridors' templates given a pavement through `road.templates.edit` (docs/adr/0975 §2).
fn paved(app: &mut App) {
    use kentos_commands::{PavementDef, PavementEdge, PavementLayerDef, RoadEditOperation, TemplatesEdit};
    let layer = |name: &str, thickness: f64| PavementLayerDef { name: name.into(), thickness, material: None };
    for (id, pavement) in [
        (
            "tk-1",
            PavementDef {
                layers: vec![layer("Aşınma", 0.05), layer("Binder", 0.08), layer("Plentmiks temel", 0.2), layer("Alttemel", 0.25)],
                edge: PavementEdge::Slope { h: 1.5 },
            },
        ),
        ("tk-2", PavementDef { layers: vec![layer("Beton", 0.25), layer("Alttemel", 0.3)], edge: PavementEdge::Vertical }),
    ] {
        let edit = TemplatesEdit {
            operation: RoadEditOperation::Set,
            template: Some(id.into()),
            name: None,
            left: None,
            right: None,
            mirror: false,
            pavement: Some(pavement),
            remove_pavement: false,
            expected_revision: None,
        };
        let r = kentos_native_application::road_templates_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(model(app)),
            edit,
        );
        assert!(matches!(r, kentos_commands::CommandResult::Completed { .. }), "{r:?}");
    }
}

#[test]
fn the_window_measures_by_a_rule_and_shows_the_layers() {
    let mut app = app();
    paved(&mut app);
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let road = app.volume_window.as_ref().and_then(|w| w.preview.report.clone()).expect("computes");
    assert_eq!(road.layers.len(), 5, "{:?}", road.layers);
    send(&mut app, W::Rule(Some(kentos_commands::VolumeRule::FormationGround)));
    let formation = app.volume_window.as_ref().and_then(|w| w.preview.report.clone()).expect("computes");
    // From the formation: more cut, less fill, the same layers.
    assert!(formation.cut > road.cut && formation.fill < road.fill, "{} {} / {} {}", formation.cut, formation.fill, road.cut, road.fill);
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_volumes()[0].rule, Some(kentos_commands::VolumeRule::FormationGround));
}

/// The Kübaj window on its tidy layout (DESIGN.md §7.19) with a rule and the pavement layers; `.run/gui-evidence/yol-y26/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib volume_window_tests::screens_rules -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_rules() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y26");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app();
        paved(&mut app);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y26-3-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        shot(&mut app, "kubaj-yol-zemin");
        send(&mut app, W::Rule(Some(kentos_commands::VolumeRule::FormationGround)));
        send(&mut app, W::View(crate::volume_window::Shown::Layers));
        shot(&mut app, "kubaj-formasyon-tabakalar");
        send(&mut app, W::Cancel);
    }
}

/// Y30.3 (docs/adr/0934 §3): Kurp on the main volume (end areas) changes its volumes by the sections' centroids, says
/// how much, and Kaydet writes it; on the prismoidal method the switch is off.
#[test]
fn the_curve_correction_is_switched_said_and_saved() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let plain = app.volume_window.as_ref().unwrap().preview.report.clone().expect("computes");
    assert!(plain.curve.is_none());
    send(&mut app, W::Curve(true));
    let curved = app.volume_window.as_ref().unwrap().preview.report.clone().expect("computes");
    let c = curved.curve.clone().expect("the correction's effect");
    assert!((curved.cut - plain.cut - c.cut).abs() < 1e-6 && (curved.fill - plain.fill - c.fill).abs() < 1e-6);
    assert!(c.fill.abs() > 1.0, "the case corridor's curve moves the fill: {c:?}");
    // Another method takes the correction off (it is the end areas'), back on end areas it is off.
    send(&mut app, W::Method(kentos_commands::VolumeMethod::Prismoidal));
    assert!(!app.volume_window.as_ref().unwrap().rows[0].curve);
    send(&mut app, W::Method(kentos_commands::VolumeMethod::EndArea));
    send(&mut app, W::Curve(true));
    send(&mut app, W::Save);
    assert!(model(&mut app).road_volumes()[0].curve);
}

/// The Y30.3 pictures, `.run/gui-evidence/yol-y26/y30-3-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib volume_window_tests::screens_curve -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_curve() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y26");
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
            let file = out.join(format!("y30-3-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        // Each state on a window opened afresh: a switch's slide does not run in a snapshot, and a state changed under
        // a drawn switch would show it half way.
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Curve(true));
        shot(&mut app, "kubaj-kurp");
        send(&mut app, W::Cancel);
        // A second app and renderer: the snapshot keeps its widget tree, so a switch drawn on keeps sliding.
        let mut other = self::app();
        let _ = other.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        other.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let _ = other.update(Message::RibbonTab("corridor"));
        let _ = other.update(Message::Run("volume.editor"));
        send(&mut other, W::Choose(0));
        send(&mut other, W::Method(kentos_commands::VolumeMethod::Prismoidal));
        snapshot.settle(&mut other, App::view, &mut |app: &mut App, message| {
            let _ = app.update(message);
        });
        let file = out.join(format!("y30-3-kubaj-kurp-prizmoidal-{name}.png"));
        snapshot.render(other.view(), &other.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The Kübaj window's tables on the kit's data table (DESIGN.md §7.6): the factors and strips typed in a grid, the sheet
/// scrolling under its head; `.run/gui-evidence/tablolar/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_tables -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_tables() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tablolar");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1440.0, 900.0), "koyu")] {
        let mut app = app();
        paved(&mut app);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("kubaj-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        for (j, row) in [["0", "120", "1,25", "0,90"], ["120", "600", "1,30", "0,95"]].iter().enumerate() {
            for (k, v) in row.iter().enumerate() {
                send(&mut app, W::Factor(j, k, (*v).into()));
            }
        }
        send(&mut app, W::Line(1));
        send(&mut app, W::AddStrip);
        for (k, v) in ["0", "600", "0,30"].iter().enumerate() {
            send(&mut app, W::Strip(0, k, (*v).into()));
        }
        send(&mut app, W::Show(true));
        shot(&mut app, "cetvel");
        send(&mut app, W::Cancel);
    }
}

/// Y35.3 (docs/adr/0941): σ, the shift and the share typed in Kübaj read the budget as `road.volumes.uncertainty`'s
/// shared case does; Belirsizlik lists it; without σ there is none.
#[test]
fn the_window_reads_the_uncertainty_as_the_shared_case_does() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.uncertainty.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Kabul %3: tutar").expect("the case");
    let want = &case["steps"][0]["result"]["output"];
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    assert!(app.volume_window.as_ref().unwrap().budget.is_none(), "no σ, no budget");
    for (k, v) in [(0, "0,05"), (1, "0,02"), (2, "3")] {
        send(&mut app, W::Uncertainty(k, v.into()));
    }
    let w = app.volume_window.as_ref().unwrap();
    let (b, _) = w.budget.clone().expect("asked").expect("reads");
    let close = |a: f64, v: &serde_json::Value| (a - v.as_f64().unwrap()).abs() <= 1e-6 * v.as_f64().unwrap().abs().max(1.0);
    assert!(close(b.cut.bound, &want["cut"]["bound"]) && close(b.fill.sigma, &want["fill"]["sigma"]), "{b:?}");
    assert_eq!(b.fill.accepted, want["fill"]["accepted"].as_bool());
    send(&mut app, W::View(crate::volume_window::Shown::Budget));
    assert_eq!(app.volume_window.as_ref().unwrap().shown, crate::volume_window::Shown::Budget);
    send(&mut app, W::Uncertainty(0, "x".into()));
    let why = app.volume_window.as_ref().unwrap().budget.clone().unwrap().expect_err("not a number");
    assert!(why.contains("Kot σ"), "{why}");
}

/// The budget in Kübaj, light and dark; `.run/gui-evidence/yol-y35/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_uncertainty -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_uncertainty() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y35");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1440.0, 900.0), "koyu")] {
        for (sigma, beta, accept, step) in [("0,05", "0,02", "3", "tutar"), ("2", "", "1", "yakin")] {
            let mut app = app();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("corridor"));
            let _ = app.update(Message::Run("volume.editor"));
            send(&mut app, W::Choose(0));
            for (k, v) in [(0, sigma), (1, beta), (2, accept)] {
                send(&mut app, W::Uncertainty(k, v.into()));
            }
            send(&mut app, W::View(crate::volume_window::Shown::Budget));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("y35-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// `road.volumes.progress`'s drawing: the volumes' and three surveys of the site (docs/adr/0942).
fn surveyed() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.progress.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Y36.3 (docs/adr/0942): a survey and the one before it picked in Kübaj read the progress as `road.volumes.progress`'s
/// shared case does; Dönem metrajı lists it by stretch; without a survey there is none.
#[test]
fn the_window_reads_the_progress_as_the_shared_case_does() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.progress.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Adlarıyla: Ekim ölçüsü, Eylül'den bu yana");
    let want = &case.expect("the case")["steps"][0]["result"]["output"];
    let mut app = surveyed();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    assert!(app.volume_window.as_ref().unwrap().progress.is_none(), "no survey, no progress");
    let pick = |app: &App, field, name| app.volume_control(crate::traces::Control::Pick(field, name)).expect("a control").expect("a message");
    let m = pick(&app, crate::volume_window::SURVEY, "Ekim ölçüsü");
    let _ = app.update(m);
    let m = pick(&app, crate::volume_window::PREVIOUS, "Eylül ölçüsü");
    let _ = app.update(m);
    let w = app.volume_window.as_ref().unwrap();
    assert_eq!(w.shown, crate::volume_window::Shown::Progress, "choosing a survey shows its list");
    let (p, _) = w.progress.clone().expect("asked").expect("reads");
    let close = |a: f64, v: &serde_json::Value| (a - v.as_f64().unwrap()).abs() <= 1e-6 * v.as_f64().unwrap().abs().max(1.0);
    assert!(close(p.now.cut_done, &want["now"]["cutDone"]) && close(p.now.fill_left, &want["now"]["fillLeft"]), "{p:?}");
    let period = p.period.expect("a period");
    assert!(close(period.cut_done, &want["period"]["cutDone"]) && close(period.fill_done, &want["period"]["fillDone"]));
    assert_eq!(p.stretches.len(), want["stretches"].as_array().unwrap().len());
    // The corridor's own surface is the original ground, not a survey: the window says why.
    let m = pick(&app, crate::volume_window::SURVEY, "Mevcut zemin");
    let _ = app.update(m);
    let why = app.volume_window.as_ref().unwrap().progress.clone().unwrap().expect_err("the original ground");
    assert!(why.contains("ilk arazisi"), "{why}");
    let m = pick(&app, crate::volume_window::SURVEY, "Yok");
    let _ = app.update(m);
    assert!(app.volume_window.as_ref().unwrap().progress.is_none(), "no survey, no progress");
}

/// The progress in Kübaj, light and dark: October's survey against September's, and a survey of the western half;
/// `.run/gui-evidence/yol-y36/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_progress -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_progress() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y36");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1440.0, 900.0), "koyu")] {
        for (survey, previous, step) in [("t-4", Some("t-3"), "ekim"), ("t-5", None, "bati")] {
            let mut app = surveyed();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("corridor"));
            let _ = app.update(Message::Run("volume.editor"));
            send(&mut app, W::Choose(0));
            send(&mut app, W::Survey(Some(survey.into())));
            send(&mut app, W::Previous(previous.map(Into::into)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("y36-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// `road.volumes.compare`'s drawing: the volumes' and two alternatives of the main road (docs/adr/0943).
fn alternatives() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.compare.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Y37.3 (docs/adr/0943): switching an alternative on in Kübaj reads the comparison as `road.volumes.compare`'s shared
/// case does, draws the alignments and lists it; switching it off takes it away.
#[test]
fn the_window_compares_alternatives_as_the_shared_case_does() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.compare.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == "Geniş kurp: aks kurpta ayrılıyor");
    let want = &case.expect("the case")["steps"][0]["result"]["output"]["alternatives"][1]["against"];
    let mut app = alternatives();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    assert!(app.volume_window.as_ref().unwrap().comparison.is_none(), "nothing picked, no comparison");
    send(&mut app, W::Alternative("v-5".into(), true));
    let w = app.volume_window.as_ref().unwrap();
    assert_eq!(w.shown, crate::volume_window::Shown::Compare);
    let (c, _) = w.comparison.clone().expect("asked").expect("reads");
    let got = c.alternatives[1].against.as_ref().expect("against the base");
    let plan = want["plan"].as_array().unwrap();
    assert_eq!(got.plan.len(), plan.len(), "{got:?}");
    for (g, p) in got.plan.iter().zip(plan) {
        assert!((g.from - p["from"].as_f64().unwrap()).abs() < 1e-6 && (g.to - p["to"].as_f64().unwrap()).abs() < 1e-6);
    }
    assert_eq!(w.traces.len(), 1, "the alternative's departures traced");
    let largest = w.traces[0].plan.iter().map(|p| p.1).fold(0.0, f64::max);
    assert!((largest - plan[0]["largest"].as_f64().unwrap()).abs() < 0.05, "the trace reaches the largest departure");
    send(&mut app, W::Alternative("v-5".into(), false));
    assert!(app.volume_window.as_ref().unwrap().comparison.is_none());
}

/// The comparison in Kübaj, light and dark: the main road against the wider curve and the higher crest;
/// `.run/gui-evidence/yol-y37/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_compare -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_compare() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y37");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        for (picked, step) in [(&["v-5", "v-6"][..], "iki"), (&["v-6"][..], "kot")] {
            let mut app = alternatives();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("corridor"));
            let _ = app.update(Message::Run("volume.editor"));
            send(&mut app, W::Choose(0));
            for id in picked {
                send(&mut app, W::Alternative((*id).into(), true));
            }
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("y37-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// The ledger reads the volume as `road.volumes.ledger` does: the shared cases “100 m'lik dilimler”, “Bölgelere göre”
/// and “Tam sayıya yuvarlama”.
#[test]
fn the_window_reads_the_ledger_as_the_shared_cases_do() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.ledger.json")).expect("cases");
    let want = |name: &str| {
        let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == name).expect("the case");
        case["steps"][0]["result"]["output"].clone()
    };
    let check = |app: &App, name: &str| {
        let w = app.volume_window.as_ref().unwrap();
        let (l, _) = w.ledger.clone().expect("asked").expect("reads");
        let o = want(name);
        let groups = o["groups"].as_array().unwrap();
        assert_eq!(l.groups.len(), groups.len(), "{name}");
        for (g, e) in l.groups.iter().zip(groups) {
            assert_eq!(g.label, e["label"].as_str().unwrap(), "{name}");
            for (a, b) in g.values.iter().zip(e["values"].as_array().unwrap()) {
                assert!((a - b.as_f64().unwrap()).abs() < 1e-6, "{name}: {a} vs {b}");
            }
        }
        for (c, e) in l.columns.iter().zip(o["columns"].as_array().unwrap()) {
            assert!((c.residue - e["residue"].as_f64().unwrap()).abs() < 1e-9, "{name}: residue");
        }
    };
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    assert!(app.volume_window.as_ref().unwrap().ledger.is_none(), "not read until shown");
    send(&mut app, W::View(crate::volume_window::Shown::Ledger));
    check(&app, "100 m'lik dilimler");
    send(&mut app, W::LedgerRegions(true));
    check(&app, "Bölgelere göre");
    send(&mut app, W::LedgerFactors);
    check(&app, "Katsayı satırlarına göre");
    send(&mut app, W::LedgerRegions(false));
    send(&mut app, W::LedgerDecimals(0));
    check(&app, "Tam sayıya yuvarlama");
    send(&mut app, W::LedgerStep("dilim".into()));
    let why = match app.volume_window.as_ref().unwrap().ledger.clone() {
        Some(Err(why)) => why,
        other => panic!("a step that is no number: {other:?}"),
    };
    assert!(why.contains("Dilim"), "{why}");
}

/// The ledger in Kübaj, light and dark: by 100 m with its bars and concentration, the largest stretches, by regions;
/// `.run/gui-evidence/yol-y43/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_ledger -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_ledger() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y43");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y43-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        send(&mut app, W::View(crate::volume_window::Shown::Ledger));
        shot(&mut app, "defter");
        send(&mut app, W::View(crate::volume_window::Shown::Largest));
        shot(&mut app, "en-buyuk");
        send(&mut app, W::View(crate::volume_window::Shown::Ledger));
        send(&mut app, W::LedgerDecimals(0));
        send(&mut app, W::LedgerRegions(true));
        shot(&mut app, "bolgeler");
        send(&mut app, W::LedgerDecimals(2));
        send(&mut app, W::LedgerFactors);
        shot(&mut app, "katsayilar");
    }
}

/// ROAD-VOL-16 (docs/adr/0949): the sheet's total is the volume's, rounded once, not its rounded rows summed. With
/// three decimals the main volume's rows add up to the total, but the ledger's whole-m³ case shows they need not.
#[test]
fn the_sheet_totals_are_rounded_once() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let report = app.volume_window.as_ref().unwrap().preview.report.clone().expect("computes");
    let sheet = kentos_native_application::road_volumes::sheet(&report);
    let total = sheet.rows.last().expect("the totals");
    assert_eq!(total[3], format!("{:.3}", report.cut), "the cut's total is the volume's");
    assert_eq!(total[4], format!("{:.3}", report.fill), "the fill's total is the volume's");
    // Rounded to whole m³ the rows' sum is off the total by one: summing them would have been a wrong total.
    send(&mut app, W::View(crate::volume_window::Shown::Ledger));
    send(&mut app, W::LedgerDecimals(0));
    let (l, _) = app.volume_window.as_ref().unwrap().ledger.clone().expect("asked").expect("reads");
    let rows: f64 = report.rows.iter().filter_map(|r| r.cut).map(f64::round).sum();
    assert_eq!(rows, l.columns[0].rows_rounded);
    assert_eq!(l.columns[0].total_rounded, report.cut.round());
    assert_eq!(l.columns[0].residue, 1.0);
}

/// ROAD-VOL-17 (docs/adr/0950): approving again and revoking keep every approval in the history, numbered, with the
/// day and why it ended; the history view reads them as `road.volumes.history` does; a volume with a history is not
/// removed.
#[test]
fn approvals_are_kept_in_the_history() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::ApprovalNote("1. hakediş".into()));
    send(&mut app, W::Approve(true));
    let v = model(&mut app).road_volumes()[0].clone();
    assert_eq!(v.approved.as_ref().map(|a| (a.number, a.note.clone())), Some((1, Some("1. hakediş".into()))));
    assert!(app.volume_window.as_ref().unwrap().approval_note.is_empty(), "the note is the approval's, then cleared");
    send(&mut app, W::Cancel);
    densify(&mut app);
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::ApprovalNote("2. hakediş".into()));
    send(&mut app, W::Approve(true));
    send(&mut app, W::ApprovalNote("Zemin raporu değişti".into()));
    send(&mut app, W::Approve(false));
    let v = model(&mut app).road_volumes()[0].clone();
    assert!(v.approved.is_none());
    assert_eq!(v.history.iter().map(|a| a.number).collect::<Vec<_>>(), vec![1, 2]);
    let ends: Vec<(bool, Option<String>)> = v.history.iter().map(|a| a.ended.as_ref().map(|e| (e.revoked, e.note.clone())).unwrap()).collect();
    assert_eq!(ends, vec![(false, None), (true, Some("Zemin raporu değişti".into()))]);
    assert!(v.history.iter().all(|a| a.ended.as_ref().is_some_and(|e| e.at.is_some())), "each ended on its day");
    send(&mut app, W::View(crate::volume_window::Shown::History));
    let h = app.volume_window.as_ref().unwrap().history.clone().expect("shown");
    let read = {
        let doc = &app.document.as_ref().unwrap().model;
        let input = kentos_commands::VolumesHistory { volume: "v-1".into() };
        let mut copy = doc.reading_copy();
        match kentos_native_application::road_volumes_history::plan(&kentos_native_application::ExecutionContext::new(&mut copy), &input) {
            kentos_commands::CommandResult::Completed { output, .. } => output,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(h, read, "the window reads the command's history");
    assert!(h.approvals[1].against.as_ref().is_some_and(|c| c.changed), "the corridor was densified between the two");
    send(&mut app, W::Remove);
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_volumes().len(), 2, "a volume with a history stays");
}

/// The approvals' history in Kübaj, light and dark: two hakediş, the second after the corridor was sampled more densely,
/// then revoked; `.run/gui-evidence/yol-y44/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_history -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_history() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y44");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::ApprovalNote("1. hakediş".into()));
        send(&mut app, W::Approve(true));
        send(&mut app, W::Cancel);
        densify(&mut app);
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::ApprovalNote("2. hakediş".into()));
        send(&mut app, W::Approve(true));
        send(&mut app, W::View(crate::volume_window::Shown::History));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y44-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        shot(&mut app, "iki-hakedis");
        send(&mut app, W::ApprovalNote("Zemin raporu değişti".into()));
        send(&mut app, W::Approve(false));
        send(&mut app, W::View(crate::volume_window::Shown::History));
        shot(&mut app, "kaldirildi");
    }
}

/// The states' shared cases' drawing for case `name` (its own setup, else the file's).
fn states_app(name: &str) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.states.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone();
    let setup = case.get("setup").cloned().unwrap_or_else(|| cases["setup"].clone());
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// ROAD-VOL-10 (docs/adr/0957): the states' view reads `road.volumes.states` as its cases do; a fill factor below 1 is
/// told; densities typed in the window make the factors and the masses.
#[test]
fn the_states_read_the_shared_cases_and_densities_are_typed() {
    const NAME: &str = "Yoğunluklu iki satır: kütleler";
    let mut app = states_app("Ana kübaj: sıkışma 0,9 zeminin yapamayacağını söylüyor");
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::View(crate::volume_window::Shown::States));
    let w = app.volume_window.as_ref().expect("open");
    let Some(Ok((st, warnings))) = &w.states else { panic!("the states read: {:?}", w.states) };
    assert!(st.fill_loose < st.fill, "a fill factor of 0.9 asks less than the fill");
    assert!(warnings.iter().any(|x| x.code == "volume_factor_physics"));
    // The densities typed: the first row's in place, loose and compacted and its source.
    for (k, v) in [(2, ""), (3, ""), (4, "1,8"), (5, "1,45"), (6, "1,95"), (7, "Zemin etüdü raporu, SK-3 sondajı")] {
        send(&mut app, W::Factor(0, k, v.into()));
    }
    for (k, v) in [(2, ""), (3, ""), (4, "1,75"), (5, "1,3"), (6, "1,85"), (7, "Zemin etüdü raporu, SK-7 sondajı")] {
        send(&mut app, W::Factor(1, k, v.into()));
    }
    let w = app.volume_window.as_ref().expect("open");
    let Some(Ok((st, _))) = &w.states else { panic!("the states read: {:?}", w.states) };
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.states.json")).expect("cases");
    let want = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == NAME).expect("the case")["steps"][0]["result"]["output"].clone();
    assert!((st.balance - want["balance"].as_f64().expect("a balance")).abs() < 1e-6);
    assert!((st.cut_mass.expect("weighed") - want["cutMass"].as_f64().expect("a mass")).abs() < 1e-6);
}

/// The states: the sample's fill factor told, light and wide; densities with masses, dark and narrow.
/// `.run/gui-evidence/yol-y51/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_states -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_states() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y51");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (case, theme, size, name) in [
        ("Ana kübaj: sıkışma 0,9 zeminin yapamayacağını söylüyor", "light", Size::new(1440.0, 900.0), "katsayi-acik-genis"),
        ("Yoğunluklu iki satır: kütleler", "light", Size::new(1440.0, 900.0), "yogunluk-acik-genis"),
        ("Yoğunluk ve katsayı karışık", "dark", Size::new(1100.0, 650.0), "karisik-koyu-dar"),
        ("Kaya: sıkışık yerindekinden hafif", "dark", Size::new(1440.0, 900.0), "kaya-koyu-genis"),
    ] {
        let mut app = states_app(case);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::View(crate::volume_window::Shown::States));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y51-hacim-halleri-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The grid method in the window (docs/adr/0974): without a cell it says so; with 2 m it computes on the grid and says
/// the method and the cell; saved, the cell is kept.
/// ROAD-VOL-01 (docs/adr/1040): the window says what the volume is computed from and by which algorithm, as
/// `road.volumes.query` does; the ground's acceptance follows the drawing: accepted, then a ground point deleted.
#[test]
fn the_window_says_what_the_volume_is_computed_from() {
    use kentos_native_application::ExecutionContext;
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let said = |app: &App| {
        let rep = app.volume_window.as_ref().and_then(|w| w.preview.report.clone()).expect("computes");
        (crate::volume_window::method_words(&rep), crate::road_dialog::volume_sources_words(rep.sources.as_ref().expect("sources")))
    };
    let (method, sources) = said(&app);
    assert!(method.starts_with("Yöntem: Ortalama alan (kentos.end-area/1), "), "{method}");
    assert!(sources.starts_with("Kaynaklar: koridor c-1 (1), aks g-1 (1), boykesit b-1 (1), tip kesit tk-1 (1)"), "{sources}");
    assert!(sources.ends_with("…, kabul edilmedi)"), "{sources}");
    let accepted = kentos_native_application::road_terrains_edit::execute(
        &mut ExecutionContext::new(model(&mut app)),
        kentos_commands::TerrainsEdit {
            operation: kentos_commands::TerrainsEditOperation::Accept,
            surface: None,
            id: Some("t-1".into()),
            expected_revision: None,
        },
    );
    assert!(matches!(accepted, kentos_commands::CommandResult::Completed { .. }), "{accepted:?}");
    send(&mut app, W::Choose(0));
    assert!(said(&app).1.ends_with("…, kabul edildi)"), "{}", said(&app).1);
    let first = model(&mut app).entities_with_uids().next().map(|(uid, _)| uid.to_string()).expect("a point");
    let gone = kentos_native_application::delete::execute(
        &mut ExecutionContext::new(model(&mut app)),
        serde_json::from_value(serde_json::json!({ "uids": [first] })).expect("the input"),
    );
    assert!(matches!(gone, kentos_commands::CommandResult::Completed { .. }), "{gone:?}");
    send(&mut app, W::Choose(0));
    assert!(said(&app).1.ends_with("…, kabulden sonra değişti)"), "{}", said(&app).1);
}

#[test]
fn the_window_computes_on_a_grid() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.new"));
    send(&mut app, W::Name("Izgara kübajı".into()));
    send(&mut app, W::Method(kentos_commands::VolumeMethod::Grid));
    let why = app.volume_window.as_ref().and_then(|w| w.preview.why.clone()).expect("no cell yet");
    assert!(why.contains("hücre boyunu"), "{why}");
    send(&mut app, W::Cell("2".into()));
    let w = app.volume_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("computes: {:?}", w.preview.why));
    assert_eq!(rep.step, Some(2.0));
    assert!(rep.cut > 1000.0 && rep.cut_area.is_some(), "{rep:?}");
    assert_eq!(crate::volume_window::method_words(rep), "Yöntem: Izgara (kentos.grid-midpoint/1), hücre 2 m.");
    send(&mut app, W::Save);
    assert_eq!(model(&mut app).road_volumes()[2].cell, Some(2.0));
}

/// `.run/gui-evidence/yol-y69/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib volume_window_tests::screens_grid -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_grid() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y69");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.new"));
        send(&mut app, W::Name("Izgara kübajı".into()));
        send(&mut app, W::Method(kentos_commands::VolumeMethod::Grid));
        send(&mut app, W::Cell("2".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y69-izgara-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// `road.volumes.progress`'s drawing as a case of it sets it (docs/adr/1029: the September survey a cloud's ground).
fn surveyed_as(case: &str) -> (App, serde_json::Value) {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.progress.json")).expect("cases");
    let case = cases["cases"].as_array().unwrap().iter().find(|c| c["name"] == case).expect("the case").clone();
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    (app, case)
}

/// ROAD-NUM-04 (docs/adr/1029): a survey whose heights are not shown to rest on the project's vertical system gives no
/// progress in Kübaj, it says why; Kotlar › Aynı say reads it as the shared case's draft does, with its warning.
#[test]
fn the_window_reads_no_progress_on_heights_not_shown_one() {
    let (mut app, _) = surveyed_as("Ölçünün düşey sistemi bilinmiyor: ret");
    let (_, draft) = surveyed_as("Varsayımla taslak dönem metrajı");
    let want = &draft["steps"][0]["result"]["output"];
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let pick = |app: &App, field, name| app.volume_control(crate::traces::Control::Pick(field, name)).expect("a control").expect("a message");
    let m = pick(&app, crate::volume_window::SURVEY, "Eylül ölçüsü");
    let _ = app.update(m);
    let why = app.volume_window.as_ref().unwrap().progress.clone().expect("asked").expect_err("not shown one system");
    assert!(why.contains("gösterilemiyor") && why.contains("bulut noktaları"), "{why}");
    let m = pick(&app, crate::volume_window::HEIGHTS, "Aynı say (taslak)");
    let _ = app.update(m);
    let (p, warnings) = app.volume_window.as_ref().unwrap().progress.clone().expect("asked").expect("a draft");
    assert!((p.now.cut_done - want["now"]["cutDone"].as_f64().unwrap()).abs() < 1e-6, "{p:?}");
    assert_eq!(warnings.first().map(|w| w.code.as_str()), Some("heights_assumed"));
}

/// The progress refused and as a draft, light and dark; `.run/gui-evidence/yol-num04/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_heights -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_heights() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-num04");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 800.0), "koyu")] {
        for (assume, step) in [(false, "ret"), (true, "taslak")] {
            let (mut app, _) = surveyed_as("Ölçünün düşey sistemi bilinmiyor: ret");
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("corridor"));
            let _ = app.update(Message::Run("volume.editor"));
            send(&mut app, W::Choose(0));
            send(&mut app, W::Survey(Some("t-3".into())));
            send(&mut app, W::AssumeHeights(assume));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("num04-donem-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// ROAD-VOL-19 (docs/adr/1037): Kübaj says what a volume measures as the query's basis does: the rule's lines and sides
/// under the diagram, the cut in place and the fill compacted on their figures.
#[test]
fn the_window_says_what_a_volume_measures() {
    let mut app = surveyed();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let rep = app.volume_window.as_ref().unwrap().preview.report.clone().expect("computes");
    let basis = kentos_native_application::road_volumes::basis_of(&rep).expect("a basis");
    assert_eq!((basis.name.as_str(), basis.given), ("Yol–Zemin", false));
    assert_eq!(basis.sides, kentos_commands::VolumeSides::Daylights);
    send(&mut app, W::Choose(1));
    let rep = app.volume_window.as_ref().unwrap().preview.report.clone().expect("computes");
    assert_eq!(kentos_native_application::road_volumes::basis_of(&rep).expect("a basis").sides, kentos_commands::VolumeSides::Corridor);
}

/// Kübaj on a section method's volume and a surfaces method's, light; `.run/gui-evidence/yol-y102/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_basis -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_basis() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y102");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (k, step) in [(0, "kesitler"), (1, "yuzeyler")] {
        let mut app = surveyed();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(k));
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y102-kubaj-{step}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Kübaj saying what it is computed from (docs/adr/1040): the ground never accepted, light at 1440 px; accepted,
/// light; a ground point deleted after it was accepted, dark at 1100 px; `.run/gui-evidence/yol-y105/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_sources -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_sources() {
    use iced::Size;
    use kentos_native_application::ExecutionContext;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y105");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut app = app();
    let _ = app.update(Message::RibbonTab("corridor"));
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    for (step, theme, size) in [
        ("kabul-edilmedi", "light", Size::new(1440.0, 900.0)),
        ("kabul-edildi", "light", Size::new(1440.0, 900.0)),
        ("kabulden-sonra-degisti-koyu", "dark", Size::new(1100.0, 760.0)),
    ] {
        match step {
            "kabul-edildi" => {
                let _ = kentos_native_application::road_terrains_edit::execute(
                    &mut ExecutionContext::new(model(&mut app)),
                    kentos_commands::TerrainsEdit {
                        operation: kentos_commands::TerrainsEditOperation::Accept,
                        surface: None,
                        id: Some("t-1".into()),
                        expected_revision: None,
                    },
                );
            }
            "kabulden-sonra-degisti-koyu" => {
                let first = model(&mut app).entities_with_uids().next().map(|(uid, _)| uid.to_string()).expect("a point");
                let _ = kentos_native_application::delete::execute(
                    &mut ExecutionContext::new(model(&mut app)),
                    serde_json::from_value(serde_json::json!({ "uids": [first] })).expect("the input"),
                );
            }
            _ => {}
        }
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        send(&mut app, W::Choose(0));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y105-kubaj-kaynaklar-{step}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Docs/adr/1041: an approval made before approvals kept their sources says so; one that keeps them says what moved,
/// the ground by its digest's first digits and a template the corridor no longer names as gone.
#[test]
fn the_approval_note_says_what_moved_since() {
    use kentos_commands::{VolumeApprovalDef, VolumeApprovalReport, VolumeSourceKind as K, VolumeSourceMove};
    let approved = VolumeApprovalDef {
        revision: 1,
        at: "2026-11-03".into(),
        note: None,
        cut: 1.0,
        fill: 1.0,
        factored_cut: 1.0,
        factored_fill: 1.0,
        strip: None,
        bench: None,
        rows: vec![],
        number: 1,
        ended: None,
        sources: None,
        algorithm: None,
    };
    let mut report = VolumeApprovalReport {
        approved,
        changed: false,
        moved: vec![],
        cut_change: 0.0,
        fill_change: 0.0,
        factored_cut_change: 0.0,
        factored_fill_change: 0.0,
        rows: vec![],
    };
    let words = crate::volume_window::approval_sources_words;
    assert_eq!(words(&report), " Bu onay hesaplandığı kaynakları saklamıyor (eski onay).");
    let source = |id: &str| kentos_commands::RoadSource { id: id.into(), revision: 1 };
    report.approved.sources = Some(kentos_commands::VolumeSources {
        corridor: source("c-1"),
        alignment: source("g-1"),
        profile: source("b-1"),
        templates: vec![source("tk-1"), source("tk-2")],
        superelevation: None,
        surface: kentos_commands::RoadSurfaceSource { id: "t-1".into(), digest: Some("d".repeat(64)), built: None },
    });
    assert_eq!(words(&report), "");
    let step = |source, id: &str, then: Option<&str>, now: Option<&str>| VolumeSourceMove {
        source,
        id: id.into(),
        then: then.map(str::to_owned),
        now: now.map(str::to_owned),
    };
    report.moved = vec![
        step(K::Template, "tk-2", Some("1"), None),
        step(K::Surface, "t-1", Some(&"d".repeat(64)), Some(&"e".repeat(64))),
    ];
    assert_eq!(words(&report), " Onaydan beri değişen: tip kesit tk-2 (1 → yok), arazi t-1 (dddddddd… → eeeeeeee…).");
}

/// Kübaj approved, then its corridor written again (sampled more densely): the approval's note says how today's figures
/// differ and which source moved since (docs/adr/1041); light at 1440 px, dark at 1100 px; `.run/gui-evidence/yol-y106/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib volume_window_tests::screens_approval_sources -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_approval_sources() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y106");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (name, theme, size) in [("acik", "light", Size::new(1440.0, 900.0)), ("koyu", "dark", Size::new(1100.0, 760.0))] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::ApprovalNote("1. hakediş".into()));
        send(&mut app, W::Approve(true));
        send(&mut app, W::Cancel);
        densify(&mut app);
        let _ = app.update(Message::Run("volume.editor"));
        send(&mut app, W::Choose(0));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y106-kubaj-onaydan-beri-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
