//! Tarama kaydı (register_window.rs, docs/adr/1800) on the shared cases' drawing of `cad.pointclouds.register`: the
//! drawing's first two scans, the GNSS layer guessed; the moved scan fitted as the shared case says, its control
//! points' KOH_Z down from 5 cm; the flat pair's plan shift loose; the CSV with the matrix; nothing written; Harita
//! and GIS offer it; the weights row's Huber and trimmed fits as the shared cases give them (docs/adr/2300); the
//! rejections row's angle and edges as the shared cases give them (docs/adr/2700); Buluta uygula's Dönüşüm uygula on
//! the moving scan with the answer's matrix, after which nothing is left to fit (docs/adr/3100). And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::register_window::{Event as R, View, Weights};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pointclouds.register.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let mut text = cases()["setup"].to_string();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    text = text.replace("$fixture:", &format!("{}/", root.display()));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(&text).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The output of the shared case named `name`.
fn case_output(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case")["steps"][0]["result"]
        ["output"]
        .clone()
}

/// docs/adr/2300: the scan with the raised patch fitted plainly, with Huber's weights and trimmed, as the shared cases
/// give them; the weights' report in the answer.
#[test]
fn the_weights_hold_the_fit_against_gross_mismatches() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pointcloud.register"));
    let (a, e) = (app.register.clouds[0].0.clone(), app.register.clouds[4].0.clone());
    let _ = app.update(Message::Register(R::Fixed(a)));
    let _ = app.update(Message::Register(R::Moving(e)));
    let same = |app: &App, name: &str| {
        let r = app.register.answer.clone().expect("an answer").expect("registered");
        let want = case_output(name);
        for (i, t) in r.translation.iter().enumerate() {
            assert!((t - want["translation"][i].as_f64().expect("a number")).abs() < 1e-7, "{name} {i}: {t}");
        }
        match (&r.robust, want.get("robust")) {
            (Some(w), Some(x)) => {
                assert_eq!(Value::from(w.lighter), x["lighter"], "{name}");
                if let Some(s) = w.scale {
                    assert!((s - x["scale"].as_f64().expect("a scale")).abs() < 1e-9, "{name}");
                }
            }
            (None, None) => {}
            other => panic!("{name}: {other:?}"),
        }
    };
    same(&app, "kaba hatalı eşleşmeler: 0,5 m yüksekteki yama düz ICP'yi aşağı çeker");
    let _ = app.update(Message::Register(R::Weights(Weights::Huber)));
    same(&app, "Huber ağırlıkları (k 1,345) kaba hatayı bastırır");
    let _ = app.update(Message::Register(R::Weights(Weights::Trimmed)));
    let _ = app.update(Message::Register(R::Ratio(0.9)));
    same(&app, "kırpılmış ICP: eşleşmelerin en iyi %90'ı");
}

/// docs/adr/2700: the wall only the second scan saw, fitted plainly and with the normals' angle (30° at first); the
/// curb's shadow, plainly and with the pairs at the fixed scan's edge left out: the shared cases' transforms and counts.
#[test]
fn the_rejections_leave_out_what_one_scan_did_not_see() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pointcloud.register"));
    let clouds: Vec<String> = app.register.clouds.iter().map(|c| c.0.clone()).collect();
    assert_eq!(clouds.len(), 8, "a, b, c, d, e, f, h, i");
    let same = |app: &App, name: &str| {
        let r = app.register.answer.clone().expect("an answer").expect("registered");
        let want = case_output(name);
        for (i, t) in r.translation.iter().enumerate() {
            assert!((t - want["translation"][i].as_f64().expect("a number")).abs() < 1e-7, "{name} {i}: {t}");
        }
        assert_eq!(Value::from(r.pairs), want["pairs"], "{name}");
        let got = r.rejected.as_ref().map(|x| serde_json::json!({"edges": x.edges, "angle": x.angle}));
        assert_eq!(got.unwrap_or(Value::Null), want.get("rejected").cloned().unwrap_or(Value::Null), "{name}");
    };
    let _ = app.update(Message::Register(R::Fixed(clouds[0].clone())));
    let _ = app.update(Message::Register(R::Moving(clouds[5].clone())));
    same(&app, "yalnız ikinci taramada duvar: ayağı zeminle eşleşir, düz ICP kayar");
    let _ = app.update(Message::Register(R::ByAngle(true)));
    same(&app, "normal açısı 30°: duvarın eşleşmeleri ayıklanır");
    let _ = app.update(Message::Register(R::Angle(60.0)));
    same(&app, "normal açısı 60°");
    let _ = app.update(Message::Register(R::ByAngle(false)));
    let _ = app.update(Message::Register(R::Fixed(clouds[6].clone())));
    let _ = app.update(Message::Register(R::Moving(clouds[7].clone())));
    same(&app, "bordürün arkası sabit taramada gölge: kaldırım yolun kenarıyla eşleşir, düz ICP kayar");
    let _ = app.update(Message::Register(R::Edges(true)));
    same(&app, "kenardaki eşleşmeler ayıklanınca bordür kaydırmaz");
}

/// `.run/gui-evidence/tarama-ayiklama-2700/`: the wall with the normals' angle, and the curb with the edges left out.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib register_window_tests::rejection_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn rejection_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarama-ayiklama-2700");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (pair, stem) in [((0, 5), "1-duvar-normal-acisi"), ((6, 7), "2-bordur-kenar")] {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("pointcloud.register"));
            let (a, b) = (app.register.clouds[pair.0].0.clone(), app.register.clouds[pair.1].0.clone());
            let _ = app.update(Message::Register(R::Fixed(a)));
            let _ = app.update(Message::Register(R::Moving(b)));
            let _ = app.update(Message::Register(if pair.0 == 0 { R::ByAngle(true) } else { R::Edges(true) }));
            if pair.0 != 0 {
                // The synthetic sidewalk stands 15 cm over the control points set on the plain ground.
                let _ = app.update(Message::Register(R::Control(String::new())));
            }
            // The clouds' indexes are made in the background; their cards would cover the window.
            let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
            while !crate::pointclouds::index::progress().is_empty() && std::time::Instant::now() < until {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

/// `.run/gui-evidence/tarama-agirlik-2300/`: the scan with the raised patch, plain and with Huber's weights.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib register_window_tests::weights_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn weights_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarama-agirlik-2300");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (weights, stem) in [(Weights::Plain, "1-esit"), (Weights::Huber, "2-huber")] {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("pointcloud.register"));
            let (a, e) = (app.register.clouds[0].0.clone(), app.register.clouds[4].0.clone());
            let _ = app.update(Message::Register(R::Fixed(a)));
            let _ = app.update(Message::Register(R::Moving(e)));
            let _ = app.update(Message::Register(R::Weights(weights)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

#[test]
fn the_moved_scan_is_fitted_as_the_shared_case_says() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("pointcloud.register"));
    assert_eq!(app.dialog, Some(Dialog::Register));
    assert_eq!(app.register.control, "gnss");
    let r = app.register.answer.clone().expect("an answer").expect("registered");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    assert_eq!((Value::from(r.iterations), Value::from(r.converged), Value::from(r.pairs)), (want["iterations"].clone(), want["converged"].clone(), want["pairs"].clone()));
    for (i, t) in r.translation.iter().enumerate() {
        assert!((t - want["translation"][i].as_f64().expect("a number")).abs() < 1e-7, "{i}: {t}");
    }
    let (before, after) = (r.control_before.clone().expect("before"), r.control_after.clone().expect("after"));
    assert!(before.koh_z > 0.04 && after.koh_z < 0.01, "{} → {}", before.koh_z, after.koh_z);
    // The flat pair: the height found, the plan shift loose.
    let (c, d) = (app.register.clouds[2].0.clone(), app.register.clouds[3].0.clone());
    let _ = app.update(Message::Register(R::Fixed(c)));
    let _ = app.update(Message::Register(R::Moving(d)));
    let flat = app.register.answer.clone().expect("an answer").expect("registered");
    assert!(flat.horizontal < 0.1 && (flat.translation[2] + 0.05).abs() < 0.01, "{} {:?}", flat.horizontal, flat.translation);
    let dir = std::env::temp_dir().join(format!("kentos-kayit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("kayit.csv");
    app.register_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    assert!(text.starts_with("Kural paketi;lidar-uyum-ornek;") && text.contains("\nMatris (hareketli → sabit)\n"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

/// docs/adr/3100: Buluta uygula selects the moving scan and opens Dönüşüm uygula with the answer's matrix and a layer
/// of its own; run, the moved scan is a new cloud there (the source untouched), and Tarama kaydı between the fixed scan
/// and it finds nothing left to move.
#[test]
fn the_fit_is_applied_to_the_moving_scan() {
    use crate::processing::{Event as P, RunStatus};
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pointcloud.register"));
    let r = app.register.answer.clone().expect("an answer").expect("registered");
    let (fixed, moving) = (app.register.fixed.clone(), app.register.moving.clone());
    let _ = app.update(Message::Register(R::Apply));
    assert_eq!(app.dialog, Some(Dialog::Processing));
    let window = app.processing.dialog.as_ref().expect("the tool's window");
    assert_eq!(window.tool.id, "pointcloud.transform");
    assert_eq!(window.values.get("matrix"), Some(&Value::from(crate::register_window::matrix_text(&r.matrix))));
    let slot = |app: &App, uid: &str| {
        app.document.as_ref().and_then(|d| d.model.slot_of(kentos_domain::Uuid::parse_str(uid).expect("a uid"))).expect("on the drawing")
    };
    assert_eq!(app.selection.ids(), [slot(&app, &moving)], "the moving scan is the tool's input");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/pointcloud/v1/files/kayit-b.xyz");
    let before = std::fs::read(&source).expect("the source");
    let dir = crate::files_testing::scratch("buluta-uygula");
    let out = dir.join("kayitli.laz");
    let _ = app.update(Message::Processing(P::Value("output".into(), Value::from(out.to_string_lossy().into_owned()))));
    let task = app.update(Message::Processing(P::Run));
    crate::files_testing::drive(&mut app, task);
    let status = app.processing.dialog.as_ref().map(|w| w.status.clone());
    let Some(RunStatus::Ok { text, .. }) = status else { panic!("{status:?}") };
    assert!(text.contains("4.320 nokta taşındı") && text.contains("dönüklük"), "{text}");
    assert_eq!(std::fs::read(&source).expect("the source"), before, "the source untouched");
    let doc = &app.document.as_ref().expect("open").model;
    let layer = doc.layers().leaves().into_iter().find(|l| l.name == "Kaydedilmiş bulut").expect("its layer").id.clone();
    let (moved, _) = doc
        .entities_with_uids()
        .find(|(_, e)| e.base().layer_id == layer && matches!(e, kentos_commands::Entity::PointCloud(_)))
        .expect("the moved scan");
    let moved = moved.to_string();
    // Fitted again against the fixed scan: nothing left to move.
    let _ = app.update(Message::Processing(P::Close));
    app.selection.set([slot(&app, &fixed), slot(&app, &moved)]);
    let _ = app.update(Message::Run("pointcloud.register"));
    assert_eq!((app.register.fixed.as_str(), app.register.moving.as_str()), (fixed.as_str(), moved.as_str()));
    let again = app.register.answer.clone().expect("an answer").expect("registered");
    // The moved scan starts where the first fit left it (its RMS, rounded to the file's grid), the height is held and
    // the control points' KOH_Z is the fitted one; what a second fit still turns in the plan (a flat ground, strength
    // about 0.13) lowers the RMS by less than a tenth of a millimetre.
    assert!((again.rms_before - r.rms_after).abs() < 0.0005, "{} vs {}", again.rms_before, r.rms_after);
    assert!(again.rms_before - again.rms_after < 0.0001, "{} -> {}", again.rms_before, again.rms_after);
    assert!(again.translation[2].abs() < 0.0005, "{:?}", again.translation);
    let (fitted, moved_koh) = (r.control_after.clone().expect("after"), again.control_before.clone().expect("before"));
    assert!((fitted.koh_z - moved_koh.koh_z).abs() < 0.001, "{} vs {}", fitted.koh_z, moved_koh.koh_z);
}

/// docs/adr/3500: the fit applied, then the same matrix with Tersini uygula on the moved scan: the scan is back where
/// it was (Tarama kaydı's RMS before is the first fit's before), and the third cloud's history keeps the first file and
/// both matrices, the inverse last.
#[test]
fn tersini_uygula_takes_the_scan_back_and_the_history_grows() {
    use crate::processing::{Event as P, RunStatus};
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pointcloud.register"));
    let r = app.register.answer.clone().expect("an answer").expect("registered");
    let (fixed, moving) = (app.register.fixed.clone(), app.register.moving.clone());
    let _ = app.update(Message::Register(R::Apply));
    let dir = crate::files_testing::scratch("tersini-uygula");
    let run = |app: &mut App, file: &str| {
        let out = dir.join(file);
        let _ = app.update(Message::Processing(P::Value("output".into(), Value::from(out.to_string_lossy().into_owned()))));
        let task = app.update(Message::Processing(P::Run));
        crate::files_testing::drive(app, task);
        let status = app.processing.dialog.as_ref().map(|w| w.status.clone());
        assert!(matches!(status, Some(RunStatus::Ok { .. })), "{status:?}");
    };
    run(&mut app, "kayitli.laz");
    let clouds_on = |app: &App, name: &str| -> Vec<(String, std::collections::BTreeMap<String, String>)> {
        let doc = &app.document.as_ref().expect("open").model;
        let layer = doc.layers().leaves().into_iter().find(|l| l.name == name).expect("the layer").id.clone();
        doc.entities_with_uids()
            .filter(|(_, e)| e.base().layer_id == layer && matches!(e, kentos_commands::Entity::PointCloud(_)))
            .map(|(u, e)| (u.to_string(), e.base().attrs.clone()))
            .collect()
    };
    let moved = clouds_on(&app, "Kaydedilmiş bulut");
    let (moved_uid, moved_attrs) = moved.last().cloned().expect("the moved scan");
    assert_eq!(moved_attrs.get("Dönüşümler").map(|h| h.split(" | ").count()), Some(1), "{moved_attrs:?}");
    // Back: the same matrix, inverted, on the moved scan.
    let _ = app.update(Message::Processing(P::Close));
    let slot = |app: &App, uid: &str| {
        app.document.as_ref().and_then(|d| d.model.slot_of(kentos_domain::Uuid::parse_str(uid).expect("a uid"))).expect("on the drawing")
    };
    app.selection.set([slot(&app, &moved_uid)]);
    let _ = app.update(Message::Run("processing.run.pointcloud.transform"));
    assert_eq!(app.dialog, Some(Dialog::Processing));
    let matrix = crate::register_window::matrix_text(&r.matrix);
    let _ = app.update(Message::Processing(P::Value("matrix".into(), Value::from(matrix))));
    let _ = app.update(Message::Processing(P::Value("invert".into(), Value::Bool(true))));
    let _ = app.update(Message::Processing(P::Value("layer".into(), serde_json::json!({"newName": "Kaydedilmiş bulut"}))));
    run(&mut app, "geri.laz");
    let back = clouds_on(&app, "Kaydedilmiş bulut");
    let (back_uid, back_attrs) = back.last().cloned().expect("the scan moved back");
    let history: Vec<&str> = back_attrs["Dönüşümler"].split(" | ").collect();
    assert_eq!(history.len(), 2, "{back_attrs:?}");
    assert_eq!(history[0], moved_attrs["Dönüşümler"]);
    assert_eq!(back_attrs["Kaynak"], moved_attrs["Kaynak"], "the first file kept");
    // Fitted again against the fixed scan from where it is back: the first fit's RMS before.
    let _ = app.update(Message::Processing(P::Close));
    app.selection.set([slot(&app, &fixed), slot(&app, &back_uid)]);
    let _ = app.update(Message::Run("pointcloud.register"));
    let again = app.register.answer.clone().expect("an answer").expect("registered");
    assert!((again.rms_before - r.rms_before).abs() < 0.001, "{} vs {}", again.rms_before, r.rms_before);
    let _ = moving;
}

/// docs/adr/3900: the fit applied, then Son dönüşümü geri al on the moved scan: no matrix typed, the history's last
/// step taken back; the new cloud keeps the first file and no history, and Tarama kaydı finds the first fit's RMS before
/// again.
#[test]
fn son_donusumu_geri_al_takes_the_last_step_back() {
    use crate::processing::{Event as P, RunStatus};
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("pointcloud.register"));
    let r = app.register.answer.clone().expect("an answer").expect("registered");
    let fixed = app.register.fixed.clone();
    let _ = app.update(Message::Register(R::Apply));
    let dir = crate::files_testing::scratch("son-donusum");
    let run = |app: &mut App, file: &str| {
        let out = dir.join(file);
        let _ = app.update(Message::Processing(P::Value("output".into(), Value::from(out.to_string_lossy().into_owned()))));
        let task = app.update(Message::Processing(P::Run));
        crate::files_testing::drive(app, task);
        let status = app.processing.dialog.as_ref().map(|w| w.status.clone());
        assert!(matches!(status, Some(RunStatus::Ok { .. })), "{status:?}");
    };
    run(&mut app, "kayitli.laz");
    let last_on = |app: &App, name: &str| -> (String, std::collections::BTreeMap<String, String>) {
        let doc = &app.document.as_ref().expect("open").model;
        let layer = doc.layers().leaves().into_iter().find(|l| l.name == name).expect("the layer").id.clone();
        doc.entities_with_uids()
            .filter(|(_, e)| e.base().layer_id == layer && matches!(e, kentos_commands::Entity::PointCloud(_)))
            .map(|(u, e)| (u.to_string(), e.base().attrs.clone()))
            .last()
            .expect("a cloud")
    };
    let (moved_uid, moved_attrs) = last_on(&app, "Kaydedilmiş bulut");
    let _ = app.update(Message::Processing(P::Close));
    let slot = |app: &App, uid: &str| {
        app.document.as_ref().and_then(|d| d.model.slot_of(kentos_domain::Uuid::parse_str(uid).expect("a uid"))).expect("on the drawing")
    };
    app.selection.set([slot(&app, &moved_uid)]);
    let _ = app.update(Message::Run("processing.run.pointcloud.transform"));
    let _ = app.update(Message::Processing(P::Value("undoLast".into(), Value::Bool(true))));
    let _ = app.update(Message::Processing(P::Value("layer".into(), serde_json::json!({"newName": "Kaydedilmiş bulut"}))));
    run(&mut app, "geri-alinan.laz");
    let (back_uid, back_attrs) = last_on(&app, "Kaydedilmiş bulut");
    assert_eq!(back_attrs.get("Kaynak"), moved_attrs.get("Kaynak"), "the first file kept");
    assert!(!back_attrs.contains_key("Dönüşümler"), "no step left: {back_attrs:?}");
    let _ = app.update(Message::Processing(P::Close));
    app.selection.set([slot(&app, &fixed), slot(&app, &back_uid)]);
    let _ = app.update(Message::Run("pointcloud.register"));
    let again = app.register.answer.clone().expect("an answer").expect("registered");
    assert!((again.rms_before - r.rms_before).abs() < 0.001, "{} vs {}", again.rms_before, r.rms_before);
}

/// docs/adr/3900: Dönüşüm uygula on the moved scan with Son dönüşümü geri al ticked (the matrix and Tersini uygula
/// hidden), then after its run; `.run/gui-evidence/son-donusum-geri-3900/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib register_window_tests::undo_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn undo_screens() {
    use crate::processing::Event as P;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/son-donusum-geri-3900");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("pointcloud.register"));
    let _ = app.update(Message::Register(R::Apply));
    let dir = crate::files_testing::scratch("son-donusum-resim");
    let run = |app: &mut App, file: &str| {
        let out = dir.join(file);
        let _ = app.update(Message::Processing(P::Value("output".into(), Value::from(out.to_string_lossy().into_owned()))));
        let task = app.update(Message::Processing(P::Run));
        crate::files_testing::drive(app, task);
    };
    run(&mut app, "kayitli.laz");
    let _ = app.update(Message::Processing(P::Close));
    let doc = &app.document.as_ref().expect("open").model;
    let layer = doc.layers().leaves().into_iter().find(|l| l.name == "Kaydedilmiş bulut").expect("the layer").id.clone();
    let moved = doc
        .entities()
        .filter(|e| e.base().layer_id == layer)
        .map(|e| kentos_domain::Slot(e.base().id))
        .last()
        .expect("the moved scan");
    app.selection.set([moved]);
    let _ = app.update(Message::Run("processing.run.pointcloud.transform"));
    let _ = app.update(Message::Processing(P::Value("undoLast".into(), Value::Bool(true))));
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("1-pencere-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
    run(&mut app, "geri-alinan.laz");
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("2-sonuc-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}

#[test]
fn harita_and_gis_offer_it() {
    for module in [Module::Map, Module::Gis] {
        assert!(opened(Some(module)).offers("pointcloud.register"), "{module:?}");
    }
}

/// `.run/gui-evidence/tarama-kaydi-1800/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib register_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarama-kaydi-1800");
    std::fs::create_dir_all(&out).expect("a folder");
    for (pair, view, stem) in [((0usize, 1usize), View::Control, "1-kaydirilmis-kontrol"), ((2, 3), View::Worst, "2-duz-zemin")] {
        for (theme, size, name) in
            [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")]
        {
            let mut app = opened(Some(Module::Map));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("pointcloud.register"));
            let (a, b) = (app.register.clouds[pair.0].0.clone(), app.register.clouds[pair.1].0.clone());
            let _ = app.update(Message::Register(R::Fixed(a)));
            let _ = app.update(Message::Register(R::Moving(b)));
            let _ = app.update(Message::Register(R::View(view)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
    // docs/adr/3100: Buluta uygula, Dönüşüm uygula's window with the matrix, and the moved scan on the drawing.
    let out3100 = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/donusum-uygula-3100");
    std::fs::create_dir_all(&out3100).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pointcloud.register"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, stem: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out3100.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        shot(&mut app, "1-tarama-kaydi");
        let _ = app.update(Message::Register(R::Apply));
        shot(&mut app, "2-donusum-uygula");
        let dir = crate::files_testing::scratch(&format!("buluta-uygula-resim-{name}"));
        let out = dir.join("kayitli.laz");
        let _ = app.update(Message::Processing(crate::processing::Event::Value(
            "output".into(),
            Value::from(out.to_string_lossy().into_owned()),
        )));
        let task = app.update(Message::Processing(crate::processing::Event::Run));
        crate::files_testing::drive(&mut app, task);
        shot(&mut app, "3-sonuc");
        let _ = app.update(Message::Processing(crate::processing::Event::Close));
        let _ = app.update(Message::Run("view.zoomExtents"));
        shot(&mut app, "4-cizimde");
    }
    // Where it sits: the point cloud's context tab, Dönüşüm uygula in İşlemler.
    let mut app = opened(Some(Module::Map));
    let size = Size::new(1920.0, 1000.0);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    app.selection.set([Slot(2)]);
    app.follow_selection_tab();
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out3100.join("5-menu-bulut-sekmesi.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
    // Where it sits: the point cloud's context tab, beside Tarama uyumu.
    let mut app = opened(Some(Module::Map));
    let size = Size::new(1920.0, 1000.0);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    app.selection.set([Slot(1), Slot(2)]);
    app.follow_selection_tab();
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("3-menu-bulut-sekmesi.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}

/// docs/adr/3500: Dönüşüm uygula with Tersini uygula on the moved scan, and the cloud moved back selected (its
/// Kaynak and Dönüşümler in Öznitelikler); light wide and dark narrow, `.run/gui-evidence/tersini-uygula-3500/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib register_window_tests::inverse_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn inverse_screens() {
    use crate::processing::Event as P;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tersini-uygula-3500");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("pointcloud.register"));
        let r = app.register.answer.clone().expect("an answer").expect("registered");
        let _ = app.update(Message::Register(R::Apply));
        let dir = crate::files_testing::scratch(&format!("tersini-uygula-resim-{name}"));
        let run = |app: &mut App, file: &str| {
            let _ = app.update(Message::Processing(P::Value("output".into(), Value::from(dir.join(file).to_string_lossy().into_owned()))));
            let task = app.update(Message::Processing(P::Run));
            crate::files_testing::drive(app, task);
        };
        run(&mut app, "kayitli.laz");
        let _ = app.update(Message::Processing(P::Close));
        let doc = &app.document.as_ref().expect("open").model;
        let layer = doc.layers().leaves().into_iter().find(|l| l.name == "Kaydedilmiş bulut").expect("its layer").id.clone();
        let moved = doc.entities_with_uids().filter(|(_, e)| e.base().layer_id == layer).map(|(u, _)| u).last().expect("moved");
        let moved = doc.slot_of(moved).expect("its slot");
        app.selection.set([moved]);
        let _ = app.update(Message::Run("processing.run.pointcloud.transform"));
        let _ = app.update(Message::Processing(P::Value("matrix".into(), Value::from(crate::register_window::matrix_text(&r.matrix)))));
        let _ = app.update(Message::Processing(P::Value("invert".into(), Value::Bool(true))));
        let _ = app.update(Message::Processing(P::Value("layer".into(), serde_json::json!({"newName": "Kaydedilmiş bulut"}))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-tersini-uygula-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        run(&mut app, "geri.laz");
        let _ = app.update(Message::Processing(P::Close));
        let doc = &app.document.as_ref().expect("open").model;
        let back = doc.entities_with_uids().filter(|(_, e)| e.base().layer_id == layer).map(|(u, _)| u).last().expect("back");
        let back = doc.slot_of(back).expect("its slot");
        app.selection.set([back]);
        app.follow_selection_tab();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-gecmis-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/4300: Dönüşüm uygula from the İşlem pane with Eksen ve açı (Z, 0.5°, the cloud's middle, a move of 1, −2,
/// 0.5 m): the written cloud's history holds a matrix whose rotation is the test's own quaternion's and which takes the
/// cloud's middle to itself plus the move. And the pane's picture, `.run/gui-evidence/eksen-aci-4300/`.
#[test]
fn an_axis_and_an_angle_from_the_pane_move_the_scan() {
    use crate::processing::Event as P;
    let mut app = opened(Some(Module::Map));
    let (slot, bounds) = {
        let doc = &app.document.as_ref().expect("open").model;
        doc.entities()
            .find_map(|e| match e {
                kentos_commands::Entity::PointCloud(c) => Some((Slot(e.base().id), c.cloud.bounds)),
                _ => None,
            })
            .expect("a scan")
    };
    app.selection.set([slot]);
    let _ = app.update(Message::Run("processing.run.pointcloud.transform"));
    for (k, v) in [
        ("form", serde_json::json!("axisAngle")),
        ("axis", serde_json::json!("0 0 1")),
        ("angle", serde_json::json!(0.5)),
        ("translation", serde_json::json!("1 -2 0.5")),
        ("layer", serde_json::json!({"newName": "Dönen bulut"})),
    ] {
        let _ = app.update(Message::Processing(P::Value(k.into(), v)));
    }
    let dir = crate::files_testing::scratch("eksen-aci");
    let out = dir.join("donen.laz");
    let _ = app.update(Message::Processing(P::Value("output".into(), Value::from(out.to_string_lossy().into_owned()))));
    let task = app.update(Message::Processing(P::Run));
    crate::files_testing::drive(&mut app, task);
    let doc = &app.document.as_ref().expect("open").model;
    let layer = doc.layers().leaves().into_iter().find(|l| l.name == "Dönen bulut").expect("the layer").id.clone();
    let history = doc
        .entities()
        .filter(|e| e.base().layer_id == layer)
        .find_map(|e| e.base().attrs.get("Dönüşümler").cloned())
        .expect("a history");
    let m: Vec<f64> = history.split_whitespace().map(|w| w.parse().expect("a number")).collect();
    // The quaternion (cos θ/2, sin θ/2 · ẑ): a rotation of 0.5° about Z.
    let h = 0.5_f64.to_radians() / 2.0;
    let (w, z) = (h.cos(), h.sin());
    let want = [[1.0 - 2.0 * z * z, -2.0 * z * w, 0.0], [2.0 * z * w, 1.0 - 2.0 * z * z, 0.0], [0.0, 0.0, 1.0]];
    let middle = [0, 1, 2].map(|k| (bounds[k] + bounds[k + 3]) / 2.0);
    let moved = [1.0, -2.0, 0.5];
    for i in 0..3 {
        for j in 0..3 {
            assert!((m[4 * i + j] - want[i][j]).abs() < 1e-12, "R[{i}][{j}] {} vs {}", m[4 * i + j], want[i][j]);
        }
        let image = m[4 * i] * middle[0] + m[4 * i + 1] * middle[1] + m[4 * i + 2] * middle[2] + m[4 * i + 3];
        assert!((image - (middle[i] + moved[i])).abs() < 1e-6, "{i}: {image}");
    }
}

/// `.run/gui-evidence/eksen-aci-4300/`: Dönüşüm uygula with Eksen ve açı chosen, its fields filled:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib register_window_tests::axis_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn axis_screens() {
    use crate::processing::Event as P;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/eksen-aci-4300");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let slot = {
            let doc = &app.document.as_ref().expect("open").model;
            doc.entities().find(|e| matches!(e, kentos_commands::Entity::PointCloud(_))).map(|e| Slot(e.base().id)).expect("a scan")
        };
        app.selection.set([slot]);
        let _ = app.update(Message::Run("processing.run.pointcloud.transform"));
        for (k, v) in [
            ("form", serde_json::json!("axisAngle")),
            ("axis", serde_json::json!("0 0 1")),
            ("angle", serde_json::json!(0.5)),
            ("translation", serde_json::json!("1 -2 0.5")),
        ] {
            let _ = app.update(Message::Processing(P::Value(k.into(), v)));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("donusum-uygula-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
