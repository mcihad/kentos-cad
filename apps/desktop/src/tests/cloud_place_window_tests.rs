//! Bulutu konumla (cloud_place_window.rs, docs/adr/5100) on the shared cases' drawing of `cad.pointclouds.place`: the
//! selected cloud, Euler angles at first; a quarter turn about Z about a centre is the shared case's answer and
//! Konumla writes it (the cloud's file places, one undo step); Konumu kaldır takes a kept place away; a matrix that
//! scales is said, nothing written; the service hands the renderer the files' places. And its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::cloud_place_window::{By, Event as P};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.pointclouds.place.json")).expect("cases")
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

/// The first step's output of the shared case whose name starts so.
fn case_output(start: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(start))).expect(start)["steps"][0]
        ["result"]["output"]
        .clone()
}

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
        _ => assert_eq!(got, want, "{at}"),
    }
}

/// The cloud of the slot.
fn cloud(app: &App, slot: u32) -> kentos_commands::PointCloudFields {
    match app.document.as_ref().expect("a drawing").model.get(Slot(slot)) {
        Some(kentos_commands::Entity::PointCloud(c)) => c.cloud.clone(),
        other => panic!("not a cloud: {other:?}"),
    }
}

/// The cloud of the slot selected and the window opened.
fn open_on(slot: u32) -> App {
    let mut app = opened();
    app.selection.set(vec![Slot(slot)]);
    let _ = app.update(Message::Run("pointcloud.place"));
    assert_eq!(app.dialog, Some(Dialog::CloudPlace));
    app
}

#[test]
fn a_quarter_turn_by_euler_angles_is_the_shared_case_and_konumla_writes_it() {
    let mut app = open_on(1);
    assert_eq!(app.cloud_place.by, By::Euler);
    let _ = app.update(Message::CloudPlace(P::Yaw(90.0)));
    let _ = app.update(Message::CloudPlace(P::Center("500050 4400050 125".into())));
    let want = case_output("Euler: Z çevresinde çeyrek");
    let got = serde_json::to_value(app.cloud_place.preview.clone().expect("a preview").expect("placed")).expect("json");
    for k in ["matrix", "poses", "bounds", "shift", "angle"] {
        near(&got[k], &want[k], k);
    }
    let (_, rows) = app.cloud_place_lines();
    assert_eq!(rows, vec![vec!["bulut.xyz".to_owned(), "90,0000".to_owned(), "0,000".to_owned()]]);
    let _ = app.update(Message::CloudPlace(P::Place));
    near(&serde_json::to_value(cloud(&app, 1).sources[0].pose.expect("placed")).expect("json"), &want["poses"][0], "pose");
    near(&serde_json::to_value(cloud(&app, 1).bounds).expect("json"), &want["bounds"], "bounds");
    // The renderer is told the file's place.
    let files = crate::pointclouds::files_of(&cloud(&app, 1), &app.document.as_ref().expect("a drawing").model, None);
    assert!(files[0].4.is_some(), "the member's place");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(cloud(&app, 1).sources[0].pose, None, "one undo step");
}

#[test]
fn konumu_kaldir_takes_the_kept_place_away() {
    let mut app = open_on(3);
    assert!(cloud(&app, 3).placed());
    let _ = app.update(Message::CloudPlace(P::Remove));
    let c = cloud(&app, 3);
    assert_eq!(c.sources[0].pose, None);
    near(&serde_json::to_value(c.bounds).expect("json"), &serde_json::json!([500000.0, 4400000.0, 100.0, 500100.0, 4400100.0, 150.0]), "bounds");
}

#[test]
fn a_matrix_that_scales_is_said_and_nothing_is_written() {
    let mut app = open_on(1);
    let _ = app.update(Message::CloudPlace(P::By(By::Matrix)));
    let _ = app.update(Message::CloudPlace(P::Matrix("1.001 0 0 0; 0 1 0 0; 0 0 1 0; 0 0 0 1".into())));
    let why = app.cloud_place.preview.clone().expect("a preview").expect_err("refused");
    assert!(why.contains("katı dönüşüm"), "{why}");
    let _ = app.update(Message::CloudPlace(P::Place));
    assert!(!cloud(&app, 1).placed(), "nothing written");
}

/// `.run/gui-evidence/map-13-10/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib cloud_place_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/map-13-10");
    std::fs::create_dir_all(&out).expect("a folder");
    let shoot = |app: &mut App, size: Size, file: &str| {
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
    // 1: Euler angles, a quarter turn about Z about the cloud's middle, light and wide.
    let mut app = opened();
    set(&mut app, "light", wide);
    app.selection.set(vec![Slot(1)]);
    let _ = app.update(Message::Run("pointcloud.place"));
    let _ = app.update(Message::CloudPlace(P::Yaw(30.0)));
    let _ = app.update(Message::CloudPlace(P::Translation("150 0 0".into())));
    shoot(&mut app, wide, "1-euler-acik-genis.png");
    // 2: written, the window closed: the cloud drawn where it is placed (turned 30°, 150 m east), beside the two-file
    // cloud of the same file where the file says; its file not written.
    let _ = app.update(Message::CloudPlace(P::Place));
    let _ = app.update(Message::CloudPlace(P::Close));
    app.selection.set(vec![Slot(1), Slot(2)]);
    let _ = app.update(Message::Run("view.zoomSelection"));
    // The placed cloud is a new one to the service: its index is made after the first frame asks for it.
    shoot(&mut app, wide, "2-konumlandi-cizim-acik-genis.png");
    crate::pointclouds::service::service().settle(std::time::Duration::from_secs(20));
    shoot(&mut app, wide, "2-konumlandi-cizim-acik-genis.png");
    // 3: a matrix that scales, dark and narrow.
    let mut app = opened();
    set(&mut app, "dark", narrow);
    app.selection.set(vec![Slot(3)]);
    let _ = app.update(Message::Run("pointcloud.place"));
    let _ = app.update(Message::CloudPlace(P::By(By::Matrix)));
    let _ = app.update(Message::CloudPlace(P::Matrix("1.001 0 0 0; 0 1 0 0; 0 0 1 0; 0 0 0 1".into())));
    shoot(&mut app, narrow, "3-matris-ret-koyu-dar.png");
    // 4: its place in the ribbon: a cloud selected, the point cloud tab beside Tarama ağı.
    let mut app = opened();
    set(&mut app, "light", wide);
    app.selection.set(vec![Slot(1)]);
    shoot(&mut app, wide, "4-serit-nokta-bulutu-acik-genis.png");
    // 5: Tarama ağı's Nesnede tut: B's matrix kept on its object, the network adjusted again from there.
    let mut app = crate::scan_network_window_tests::opened();
    set(&mut app, "light", wide);
    app.selection.set(vec![Slot(1), Slot(2), Slot(3)]);
    let _ = app.update(Message::Run("pointcloud.network"));
    let _ = app.update(Message::ScanNetwork(crate::scan_network_window::Event::Keep));
    shoot(&mut app, wide, "5-tarama-agi-nesnede-tut-acik-genis.png");
    // 6: Tarama ağı's outer loop: five rounds at most, the notes say each round's move; dark and narrow.
    let mut app = crate::scan_network_window_tests::opened();
    set(&mut app, "dark", narrow);
    app.selection.set(vec![Slot(1), Slot(2), Slot(3)]);
    let _ = app.update(Message::Run("pointcloud.network"));
    let _ = app.update(Message::ScanNetwork(crate::scan_network_window::Event::Rounds(5.0)));
    shoot(&mut app, narrow, "6-tarama-agi-dis-dongu-koyu-dar.png");
}
