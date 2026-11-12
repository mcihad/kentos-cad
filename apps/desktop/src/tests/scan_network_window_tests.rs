//! Tarama ağı dengelemesi (scan_network_window.rs, docs/adr/4720) on the shared cases' drawing of
//! `cad.pointclouds.network`: three selected scans, the first held, Huber's weights at first — the shared case's
//! answer; Eşit weights — the unweighted case's; the links and loops tables; Dönüşüm uygula on the chosen scan with its
//! matrix; two scans are said and nothing opens. And its pictures.

use crate::gis;
use crate::map;
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::scan_network_window::{Event as N, Show, Weights};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pointclouds.network.json")).expect("cases")
}

pub(crate) fn opened() -> App {
    let mut text = cases()["setup"].to_string();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    text = text.replace("$fixture:", &format!("{}/", root.display()));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(&text).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The output of the shared case whose name starts so.
fn case_output(start: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start))).expect(start)["steps"][0]
        ["result"]["output"]
        .clone()
}

/// The same JSON, numbers within the cases' 10⁻⁶.
fn near(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().expect("a number"), b.as_f64().expect("a number"));
            assert!((a - b).abs() <= 1e-6, "{at}: {a} ≠ {b}");
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                near(x, y, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                near(a.get(k).unwrap_or(&Value::Null), b.get(k).unwrap_or(&Value::Null), &format!("{at}.{k}"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

/// The three scans A, B, C selected and the window opened.
fn three() -> App {
    let mut app = opened();
    app.selection.set(vec![Slot(1), Slot(2), Slot(3)]);
    let _ = app.update(Message::Run("pointcloud.network"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Map(map::MapDialogKind::ScanNetwork))
    );
    app
}

fn answer(app: &App) -> Value {
    serde_json::to_value(
        app.map
            .scan_network
            .answer
            .clone()
            .expect("an answer")
            .expect("adjusted"),
    )
    .expect("json")
}

#[test]
fn three_scans_with_hubers_weights_are_the_shared_case() {
    let app = three();
    assert_eq!(app.map.scan_network.weights, Weights::Huber);
    near(
        &answer(&app),
        &case_output("üç tarama, A sabit, Huber"),
        "Huber",
    );
    let (_, rows) = app.scan_network_lines();
    assert_eq!(rows.len(), 3, "a row a scan");
    assert_eq!(rows[0][1], "Sabit");
}

#[test]
fn equal_weights_and_the_tables() {
    let mut app = three();
    let _ = app.update(Message::Map(map::Event::ScanNetwork(N::Weights(
        Weights::Plain,
    ))));
    near(&answer(&app), &case_output("ağırlıksız"), "plain");
    let _ = app.update(Message::Map(map::Event::ScanNetwork(N::Show(Show::Links))));
    assert_eq!(app.scan_network_lines().1.len(), 3, "three links");
    let _ = app.update(Message::Map(map::Event::ScanNetwork(N::Show(Show::Loops))));
    let (_, rows) = app.scan_network_lines();
    assert_eq!(rows.len(), 1, "one loop");
    assert!(rows[0][0].contains(" → "), "{:?}", rows[0]);
}

#[test]
fn donusum_uygula_takes_the_chosen_scan_with_its_matrix() {
    let mut app = three();
    let r = app
        .map
        .scan_network
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    let chosen = app.map.scan_network.chosen.clone();
    let scan = r
        .scans
        .iter()
        .find(|s| s.uid == chosen)
        .expect("the chosen scan");
    let _ = app.update(Message::Map(map::Event::ScanNetwork(N::Apply)));
    assert_eq!(
        app.dialog,
        Some(Dialog::Gis(gis::GisDialogKind::Processing))
    );
    let window = app
        .gis
        .processing
        .dialog
        .as_ref()
        .expect("the tool's window");
    assert_eq!(window.tool.id, "pointcloud.transform");
    assert_eq!(
        window.values.get("matrix"),
        Some(&Value::from(crate::register_window::matrix_text(
            &scan.matrix
        )))
    );
    assert_eq!(
        app.selection.ids(),
        [Slot(2)],
        "B, the chosen scan, is the tool's input"
    );
}

/// docs/adr/5100: Nesnede tut keeps the chosen scan's matrix on its object (its file not written) and adjusts the
/// network again from there: that scan's correction is then what is left, a millimetre or so (a second round).
#[test]
fn nesnede_tut_keeps_the_chosen_scan_s_matrix_on_its_object() {
    let mut app = three();
    let r = app
        .map
        .scan_network
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    let chosen = app.map.scan_network.chosen.clone();
    let scan = r
        .scans
        .iter()
        .find(|s| s.uid == chosen)
        .expect("the chosen scan")
        .clone();
    let _ = app.update(Message::Map(map::Event::ScanNetwork(N::Keep)));
    let placed = match app.document.as_ref().expect("a drawing").model.get(Slot(2)) {
        Some(kentos_commands::Entity::PointCloud(c)) => c.cloud.sources[0].pose.expect("its place"),
        other => panic!("not a cloud: {other:?}"),
    };
    near(
        &serde_json::to_value(placed).expect("json"),
        &serde_json::to_value(scan.matrix).expect("json"),
        "pose",
    );
    let again = app
        .map
        .scan_network
        .answer
        .clone()
        .expect("an answer")
        .expect("adjusted");
    let left = again
        .scans
        .iter()
        .find(|s| s.uid == chosen)
        .expect("the chosen scan");
    let t = left.translation.iter().map(|v| v * v).sum::<f64>().sqrt();
    let w = left.rotation.iter().map(|v| v * v).sum::<f64>().sqrt();
    println!("left: {:.3} mm, {:.2e} rad (was {:.1} mm)", t * 1000.0, w, scan.translation.iter().map(|v| v * v).sum::<f64>().sqrt() * 1000.0);
    assert!(t < 0.005 && w < 1e-3, "what is left: {t} m, {w} rad");
}

/// docs/adr/5100: Dış döngü 5: the shared case's rounds (each round's largest move) and the scans' rotations and
/// translations about the centre.
#[test]
fn five_rounds_are_the_shared_case() {
    let mut app = three();
    let _ = app.update(Message::Map(map::Event::ScanNetwork(N::Rounds(5.0))));
    let got = answer(&app);
    let want = case_output("dış döngü: üç tarama");
    near(&got["rounds"], &want["rounds"], "rounds");
    for k in 0..3 {
        near(&got["scans"][k]["rotation"], &want["scans"][k]["rotation"], "rotation");
        near(&got["scans"][k]["translation"], &want["scans"][k]["translation"], "translation");
    }
    let (lines, _) = app.scan_network_lines();
    assert!(lines.len() >= 5, "the notes with the outer loop's");
}

#[test]
fn two_scans_are_said_and_nothing_opens() {
    let mut app = opened();
    app.selection.set(vec![Slot(1), Slot(2)]);
    let _ = app.update(Message::Run("pointcloud.network"));
    assert_eq!(app.dialog, None);
}

/// `.run/gui-evidence/tarama-agi-4720/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scan_network_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarama-agi-4720");
    std::fs::create_dir_all(&out).expect("a folder");
    let shoot = |app: &mut App, size: Size, file: &str| {
        // The clouds' indexes are made in the background; their cards would cover the window.
        let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !crate::pointclouds::index::progress().is_empty() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(file);
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    };
    let set = |app: &mut App, theme: &str, size: Size| {
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
    };
    let (wide, narrow) = (Size::new(1440.0, 900.0), Size::new(1100.0, 760.0));
    // 1: four scans, the scans' table, light and wide.
    let mut app = opened();
    set(&mut app, "light", wide);
    app.selection.set(vec![Slot(1), Slot(2), Slot(3), Slot(4)]);
    let _ = app.update(Message::Run("pointcloud.network"));
    shoot(&mut app, wide, "1-taramalar-acik-genis.png");
    // 2: its links.
    let _ = app.update(Message::Map(map::Event::ScanNetwork(N::Show(Show::Links))));
    shoot(&mut app, wide, "2-baglar-acik-genis.png");
    // 3: three scans' loop, dark and narrow.
    let mut app = opened();
    set(&mut app, "dark", narrow);
    app.selection.set(vec![Slot(1), Slot(2), Slot(3)]);
    let _ = app.update(Message::Run("pointcloud.network"));
    let _ = app.update(Message::Map(map::Event::ScanNetwork(N::Show(Show::Loops))));
    shoot(&mut app, narrow, "3-donguler-koyu-dar.png");
    // 4: its place in the ribbon: a cloud selected, the point cloud tab beside Tarama kaydı.
    let mut app = opened();
    set(&mut app, "light", wide);
    app.selection.set(vec![Slot(1)]);
    shoot(&mut app, wide, "4-serit-nokta-bulutu-acik-genis.png");
}
