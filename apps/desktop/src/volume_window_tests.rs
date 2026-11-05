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

/// The corridors' templates given a pavement through `road.templates.edit` (docs/adr/0930 §2).
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
