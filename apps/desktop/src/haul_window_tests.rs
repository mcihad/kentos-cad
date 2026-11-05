//! Brükner (haul_window.rs, plan/road.md Y9): the window reads the diagram as it is typed and writes through
//! `road.hauls.edit` (one undo step); and its pictures.

use crate::app::{App, Dialog, Message};
use crate::haul_window::Event as W;

/// A road project with `road.hauls.query`'s drawing: two corridors, two volumes and two diagrams.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.hauls.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Hauls(e));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

#[test]
fn the_window_makes_a_diagram_with_a_site_and_reads_it() {
    let mut app = app();
    let _ = app.update(Message::Run("haul.new"));
    assert_eq!(app.dialog, Some(Dialog::Hauls));
    send(&mut app, W::Name("Döküm Brükneri".into()));
    send(&mut app, W::Field(1, "200".into()));
    send(&mut app, W::Add);
    send(&mut app, W::Site(0, 2, "350".into()));
    send(&mut app, W::Site(0, 3, "9000".into()));
    let w = app.haul_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    assert!(rep.points.windows(2).any(|p| p[0][0] == 350.0 && p[1][0] == 350.0), "the site's step");
    assert!(rep.hauled > 1000.0 && rep.moment > 0.0, "{rep:?}");
    send(&mut app, W::Save);
    assert!(app.haul_window.is_none(), "Kaydet closes the window");
    let doc = model(&mut app);
    assert_eq!(doc.road_hauls().len(), 3);
    assert_eq!(doc.road_hauls()[2].free, Some(200.0));
    assert_eq!(doc.undo().as_deref(), Some("Brükner"));
}

#[test]
fn prices_and_a_payload_give_the_cost_and_are_written() {
    let mut app = app();
    let _ = app.update(Message::Run("haul.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Field(1, "100".into()));
    send(&mut app, W::Field(3, "12,5".into()));
    assert!(app.haul_window.as_ref().is_some_and(|w| w.preview.why.is_some()), "a price wants a number in the dot");
    send(&mut app, W::Field(3, "12.5".into()));
    assert!(
        app.haul_window.as_ref().and_then(|w| w.preview.why.as_deref()).is_some_and(|e| e.contains("para birimi")),
        "a price wants a currency"
    );
    send(&mut app, W::Field(2, "TL".into()));
    send(&mut app, W::Field(6, "15".into()));
    let w = app.haul_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("reads: {:?}", w.preview.why));
    let cost = rep.cost.as_ref().expect("priced");
    assert_eq!(cost.currency, "TL");
    assert!((cost.total - rep.over_moment / 1000.0 * 12.5).abs() < 1e-6, "{cost:?} {rep:?}");
    assert!(cost.trips.is_some_and(|t| t >= 1.0));
    send(&mut app, W::Save);
    assert!(app.haul_window.is_none());
    let saved = model(&mut app).road_hauls()[0].cost.clone().expect("written");
    assert_eq!((saved.overhaul, saved.payload), (Some(12.5), Some(15.0)));
}

#[test]
fn a_bad_site_is_refused_and_the_window_stays() {
    let mut app = app();
    let _ = app.update(Message::Run("haul.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Add);
    send(&mut app, W::Site(0, 2, "9000".into()));
    send(&mut app, W::Site(0, 3, "10".into()));
    assert!(app.haul_window.as_ref().is_some_and(|w| w.preview.why.is_some()), "a site off the stations");
    send(&mut app, W::Save);
    assert!(app.haul_window.is_some());
    assert!(model(&mut app).road_hauls()[0].sites.is_empty());
}

/// The window, wide and narrow, light and dark; `.run/gui-evidence/yol-y9-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib haul_window_tests::screens -- --ignored --nocapture
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
            let file = out.join(format!("yol-y9-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("haul.editor"));
        // The main diagram with a borrow and a waste site, its balance line lowered: loops both ways.
        send(&mut app, W::Choose(0));
        send(&mut app, W::Field(0, "9000".into()));
        send(&mut app, W::Add);
        send(&mut app, W::Site(0, 2, "120".into()));
        send(&mut app, W::Site(0, 3, "2000".into()));
        send(&mut app, W::Kind(0, kentos_commands::HaulSiteKind::Borrow));
        send(&mut app, W::Field(1, "150".into()));
        send(&mut app, W::Field(2, "TL".into()));
        for (k, v) in [(3, "12.5"), (4, "40"), (5, "85"), (6, "15")] {
            send(&mut app, W::Field(k, v.into()));
        }
        send(&mut app, W::Add);
        send(&mut app, W::Site(1, 2, "560".into()));
        send(&mut app, W::Site(1, 3, "8000".into()));
        shot(&mut app, "bruckner");
        send(&mut app, W::Choose(1));
        shot(&mut app, "bruckner-yan");
        send(&mut app, W::Cancel);
    }
}

/// Y38.3 (docs/adr/0944): the unsuitable cut and the barriers typed in Brükner are written through `road.hauls.edit`
/// (a closed barrier without a detour) and read back.
#[test]
fn unsuitable_cut_and_barriers_are_typed_and_saved() {
    let mut app = app();
    let _ = app.update(Message::Run("haul.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::AddUnsuitable);
    for (k, v) in [(0, "0"), (1, "250"), (2, "35")] {
        send(&mut app, W::Unsuitable(0, k, v.into()));
    }
    send(&mut app, W::AddBarrier);
    send(&mut app, W::Barrier(0, 0, "Dere".into()));
    send(&mut app, W::Barrier(0, 1, "300".into()));
    send(&mut app, W::Barrier(0, 2, "400".into()));
    send(&mut app, W::AddBarrier);
    send(&mut app, W::Barrier(1, 1, "450".into()));
    send(&mut app, W::Save);
    let h = &app.document.as_ref().expect("a drawing").model.road_hauls()[0];
    assert_eq!(h.unsuitable, vec![kentos_commands::HaulUnsuitableDef { from: 0.0, to: 250.0, share: 35.0 }]);
    assert_eq!(h.barriers.len(), 2);
    assert_eq!(h.barriers[0].detour, Some(400.0));
    assert_eq!((h.barriers[1].name.as_str(), h.barriers[1].detour), ("Engel 2", None), "no detour: closed");
}

/// The unsuitable cut and the barriers in Brükner; `.run/gui-evidence/yol-y38/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib haul_window_tests::screens_sorting -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_sorting() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y38");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("haul.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::AddUnsuitable);
        for (k, v) in [(0, "100"), (1, "250"), (2, "30")] {
            send(&mut app, W::Unsuitable(0, k, v.into()));
        }
        send(&mut app, W::AddBarrier);
        for (k, v) in [(0, "Dere"), (1, "420"), (2, "150")] {
            send(&mut app, W::Barrier(0, k, v.into()));
        }
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("y38-bruckner-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
        println!("{}", path.display());
    }
}

/// ROAD-HAUL-06 (docs/adr/0955): the network's layer and tolerances and a site's place are written through
/// `road.hauls.edit`; the diagram then reads the site where its access meets the alignment.
#[test]
fn the_window_writes_the_haul_road_network() {
    let mut app = network_app();
    let _ = app.update(Message::Run("haul.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::NetLayer(Some("tasima".into())));
    send(&mut app, W::NetField(1, "10".into()));
    send(&mut app, W::Add);
    // The waste site at the end of the first road (the network cases' “Dere döküm”).
    let site = network_site();
    send(&mut app, W::Site(0, 0, "Dere döküm".into()));
    send(&mut app, W::Site(0, 2, "100".into()));
    send(&mut app, W::Site(0, 3, "30000".into()));
    send(&mut app, W::Site(0, 4, site.0.to_string()));
    send(&mut app, W::Site(0, 5, site.1.to_string()));
    send(&mut app, W::Save);
    let h = model(&mut app).road_hauls()[0].clone();
    let n = h.network.expect("a network");
    assert_eq!((n.layers.as_slice(), n.connect, n.reach), (["tasima".to_owned()].as_slice(), 0.5, Some(10.0)));
    assert_eq!(h.sites[0].at.map(|p| (p.x, p.y)), Some(site));
}

fn network_case() -> serde_json::Value {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.hauls.network.json")).expect("cases");
    cases["setup"].clone()
}

/// The first network case's waste site's place.
fn network_site() -> (f64, f64) {
    let s = &network_case()["settings"]["road"]["hauls"][0]["sites"][0]["at"];
    (s["x"].as_f64().expect("x"), s["y"].as_f64().expect("y"))
}

/// The network cases' drawing, its first diagram without its network (the window writes it).
fn network_app() -> App {
    let (mut app, _) = App::boot(None);
    let mut setup = network_case();
    let hauls = setup.pointer_mut("/settings/road/hauls").and_then(|h| h.as_array_mut()).expect("hauls");
    hauls[0].as_object_mut().expect("a diagram").remove("network");
    hauls[0]["sites"] = serde_json::json!([]);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The Brükner window with the network's fields and sites off the road; `.run/gui-evidence/yol-y49/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib haul_window_tests::screens_network -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_network() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y49");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let (mut app, _) = App::boot(None);
        let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(network_case()).expect("the drawing reads");
        let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("haul.editor"));
        send(&mut app, W::Choose(0));
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("y49-bruckner-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
        println!("{}", path.display());
    }
}
