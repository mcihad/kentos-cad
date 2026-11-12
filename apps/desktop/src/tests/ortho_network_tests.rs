//! Mozaik birleşme çizgisi's Ağ and Ayak izi (ortho_network_section.rs, docs/adr/5260) on the shared cases' drawings of
//! `cad.orthophoto.network` and `cad.orthophoto.footprint`: Ağ with every raster ticked plans the case's elements and
//! its junction; unticking one plans the two others; the button writes the elements in one undo step and selects them.
//! Ayak izi plans the case's outline, then its hull. And their pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::ortho_compare_window_tests::{open, with_fixtures};
use crate::ortho_network_section::{Mode, NetEvent as N};
use crate::ortho_seamlines_window::Event as E;

fn case(file: &str, name: &str) -> Value {
    let all: Value = serde_json::from_str(&std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/commands/v1").join(file),
    )
    .expect("cases"))
    .expect("JSON");
    all["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(name))).cloned().expect(name)
}

/// Numbers within 10⁻⁶; the revision left out.
fn near(got: &Value, want: &Value, at: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            assert!((a.as_f64().expect("n") - b.as_f64().expect("n")).abs() <= 1e-6, "{at}: {a} ≠ {b}");
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{at}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                near(x, y, &format!("{at}[{i}]"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).filter(|k| *k != "revision").collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                near(a.get(k).unwrap_or(&Value::Null), b.get(k).unwrap_or(&Value::Null), &format!("{at}.{k}"));
            }
        }
        _ => assert_eq!(got, want, "{at}"),
    }
}

fn opened(c: &Value) -> App {
    let mut app = open(&with_fixtures(&c["setup"]), Some(Module::Map));
    let _ = app.update(Message::Run("raster.seamlines"));
    assert_eq!(app.dialog, Some(Dialog::OrthoSeamlines));
    app
}

#[test]
fn the_network_plans_the_case_s_elements_and_junction_and_writes_them() {
    let c = case("cad.orthophoto.network.json", "üç görüntü");
    let mut app = opened(&c);
    let _ = app.update(Message::OrthoSeamlines(E::Net(N::Mode(Mode::Network))));
    let got = serde_json::to_value(app.ortho_seamlines.net.network.clone().expect("an answer").expect("planned")).expect("json");
    near(&got, &c["steps"][0]["result"]["output"], "network");
    // Unticking the third: two elements, no junction.
    let third = app.ortho_seamlines.rasters[2].0.clone();
    let _ = app.update(Message::OrthoSeamlines(E::Net(N::Tick(third))));
    let two = app.ortho_seamlines.net.network.clone().expect("an answer").expect("planned");
    assert_eq!((two.elements.len(), two.junctions.len()), (2, 0));
    let back = app.ortho_seamlines.rasters[2].0.clone();
    let _ = app.update(Message::OrthoSeamlines(E::Net(N::Tick(back))));
    let before = app.document.as_ref().expect("a drawing").model.entities().count();
    let _ = app.update(Message::OrthoSeamlines(E::Net(N::Write)));
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(doc.entities().count(), before + 3);
    assert_eq!(app.selection.ids().len(), 3);
    assert_eq!(app.dialog, None);
}

#[test]
fn the_footprint_plans_the_case_s_outline_and_hull() {
    let c = case("cad.orthophoto.footprint.json", "NoData'dan ayak izi");
    let mut app = opened(&c);
    assert_eq!(app.ortho_seamlines.net.mode, Mode::Footprint, "one raster: its footprint");
    let got = serde_json::to_value(app.ortho_seamlines.net.footprint.clone().expect("an answer").expect("planned")).expect("json");
    near(&got, &c["steps"][0]["result"]["output"], "outline");
    let _ = app.update(Message::OrthoSeamlines(E::Net(N::Hull(true))));
    let hull = case("cad.orthophoto.footprint.json", "dışbükey zarf: tek parça");
    let got = serde_json::to_value(app.ortho_seamlines.net.footprint.clone().expect("an answer").expect("planned")).expect("json");
    near(&got, &hull["steps"][0]["result"]["output"], "hull");
}

/// `.run/gui-evidence/mozaik-agi-5260/`: Ağ with three images and their junction, and Ayak izi's outline.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ortho_network_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/mozaik-agi-5260");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (file, prefix, what, mode) in [
            ("cad.orthophoto.network.json", "dört görüntü 2 × 2", "ag", Mode::Network),
            ("cad.orthophoto.network.json", "üç görüntü", "ag-uc", Mode::Network),
            ("cad.orthophoto.footprint.json", "NoData'dan ayak izi", "ayak-izi", Mode::Footprint),
        ] {
            let c = case(file, prefix);
            let mut app = opened(&c);
            let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::OrthoSeamlines(E::Net(N::Mode(mode))));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let shot = out.join(format!("{what}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&shot).expect("writes");
            println!("{}", shot.display());
        }
    }
}
