//! Malzeme bölgeleri in Yüzey analizi › Fark (docs/adr/3540; plan/map-cad.md MAP-15): the zones are the closed objects
//! of a layer, named and factored by their attributes, and the window's answer is the shared case's; and its pictures.

use crate::app::{App, Message};
use crate::road;
use crate::surface_analysis_window::{Event as W, Mode, Read, Tab};

fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.compare.zones.json")).expect("cases")
}

/// The zones' cases' drawing: the compare lattices, the layer of rock, earth and clay, and the other boundaries.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::SurfaceAnalysis(e)));
}

fn read(app: &App) -> Read {
    match &app
        .road
        .surface_analysis_window
        .as_ref()
        .expect("open")
        .preview
    {
        Some(Ok(r)) => r.clone(),
        other => panic!("a reading: {other:?}"),
    }
}

/// Fark of the pad over the terrace with the layer of materials.
fn compared(app: &mut App) {
    let _ = app.update(Message::Run("terrain.analysis"));
    send(app, W::Mode(Mode::Compare));
    send(app, W::Surface("t-6".into()));
    send(app, W::Comparison("t-9".into()));
    send(app, W::Preview);
}

#[test]
fn fark_reads_the_material_zones_from_a_layer() {
    let mut app = app();
    compared(&mut app);
    assert_eq!(
        app.road
            .surface_analysis_window
            .as_ref()
            .expect("open")
            .materials
            .as_deref(),
        Some("malzeme"),
        "guessed from its name"
    );
    let Read::Compare(c) = read(&app) else {
        panic!("a difference")
    };
    let case = cases()["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| {
            c["name"]
                .as_str()
                .is_some_and(|n| n.starts_with("Üç bölge"))
        })
        .expect("the case")
        .clone();
    let want = &case["steps"][0]["result"]["output"];
    let names: Vec<&str> = c.zones.iter().map(|z| z.name.as_str()).collect();
    assert_eq!(names, ["Kaya", "Toprak", "Kil"]);
    for (z, w) in c.zones.iter().zip(want["zones"].as_array().expect("zones")) {
        assert_eq!(z.boundary, w["boundary"].as_str().expect("uid"));
        for (got, key) in [(z.covered, "covered"), (z.cut, "cut"), (z.fill, "fill")] {
            assert!((got - w[key].as_f64().expect("a number")).abs() < 1e-9, "{} {key}", z.name);
        }
        assert_eq!((z.adjusted.cut_factor, z.adjusted.fill_factor), (w["adjusted"]["cutFactor"].as_f64().expect("f"), w["adjusted"]["fillFactor"].as_f64().expect("f")));
    }
    let (rest, zoned) = (c.rest.clone().expect("the rest"), c.zoned.clone().expect("by material"));
    assert!((rest.cut - want["rest"]["cut"].as_f64().expect("cut")).abs() < 1e-9);
    assert!((zoned.net - want["zoned"]["net"].as_f64().expect("net")).abs() < 1e-9);
    // No layer, no zones; a grid measures without them.
    send(&mut app, W::Materials(None));
    send(&mut app, W::Preview);
    let Read::Compare(none) = read(&app) else { panic!("a difference") };
    assert!(none.zones.is_empty() && none.rest.is_none() && none.zoned.is_none());
    send(&mut app, W::Materials(Some("malzeme".into())));
    send(&mut app, W::VolumeMethod(1));
    send(&mut app, W::Field(13, "0,5".into()));
    send(&mut app, W::Preview);
    let Read::Compare(grid) = read(&app) else { panic!("a difference") };
    assert!(grid.zones.is_empty() && grid.cell == Some(0.5));
}

/// Fark with the materials' table; light wide and dark narrow, `.run/gui-evidence/malzeme-bolgeleri-3540/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib material_zones_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/malzeme-bolgeleri-3540");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::RibbonTab("terrain"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        compared(&mut app);
        send(&mut app, W::Field(14, "1,2".into()));
        send(&mut app, W::Field(15, "1,05".into()));
        // The chart, then the content (the owner's words: “grafik ve içerik ayrı sekmelerde olsun”).
        for (tab, list, which) in [(Tab::Chart, "Özet", "grafik"), (Tab::Content, "Özet", "icerik-ozet"), (Tab::Content, "Malzemeler", "icerik-malzemeler")] {
            send(&mut app, W::Tab(tab));
            send(&mut app, W::List(list));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("fark-malzeme-{which}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}
