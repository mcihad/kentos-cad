//! A network tool's network as it is (the CBS review's 16th item, docs/adr/9332), on the review's roads: A (500000,
//! 4320000), C (500030, 4320000), B (500030, 4320040); AC 30 m, CB 40 m, AB 50 m, one line layer and no network.
//! En kısa yol offers a network made of that layer and shows the network from the start; a network saved meanwhile
//! (Ağlar's Kaydet, a script) brings the tool to its first stop; A → B is the direct 50 m road.

use crate::app::{App, Message};
use crate::files_testing::drive;

pub(super) fn send(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

/// A CBS project in EPSG:5256 whose one line layer `qa_yollar` holds the three roads, without a network.
pub(super) fn roads() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = crate::tools_scenes::Objects::new();
    let (e, n) = (crate::tools_scenes::E, crate::tools_scenes::N);
    // The scenes' origin is E, N: the review's points moved onto it.
    let (a, c, b) = (
        [500_000.0 - e, 4_320_000.0 - n],
        [500_030.0 - e, 4_320_000.0 - n],
        [500_030.0 - e, 4_320_040.0 - n],
    );
    o.path("yol", &[a, c], false);
    o.path("yol", &[c, b], false);
    o.path("yol", &[a, b], false);
    crate::tools_scenes::open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = serde_json::json!("gis");
        d["layers"] = serde_json::json!([
            { "id": "yol", "name": "qa_yollar", "type": "layer", "visible": true, "locked": false, "expanded": true,
              "style": { "color": "#E5484D", "lineType": "continuous", "lineWeight": 0.5 }, "children": [] }
        ]);
        d["activeLayer"] = serde_json::json!("yol");
    });
    let _ = app.update(Message::Viewport(crate::viewport::Event::Resized(
        iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(1000.0, 700.0)),
    )));
    send(&mut app, Message::Run("view.zoomExtents"));
    app
}

pub(super) fn prompt(app: &App) -> String {
    app.session.prompt().text()
}

pub(super) fn settings(app: &App) -> Vec<(String, String)> {
    app.session
        .settings(&app.format())
        .into_iter()
        .map(|s| (s.label.into_owned(), s.value))
        .collect()
}

/// A click at a place of the world (the drawing's own system).
pub(super) fn click(app: &mut App, x: f64, y: f64) {
    let [sx, sy] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(x, y));
    let at = iced::Point::new(sx as f32, sy as f32);
    for e in [
        crate::viewport::Event::Moved(at),
        crate::viewport::Event::Pressed(at),
        crate::viewport::Event::Released(at),
    ] {
        send(app, Message::Viewport(e));
    }
}

#[test]
fn en_kisa_yol_offers_the_layer_and_shows_its_network_from_the_start() {
    let mut app = roads();
    send(&mut app, Message::Run("tool.netRoute"));
    assert_eq!(
        prompt(&app),
        "En kısa yol: projede ağ tanımlı değil; Ağlar ile tanımlayın ya da “qa_yollar” katmanından ağ oluşturun \
         [Ağlar (G) / Katmandan ağ oluştur (O)]"
    );
    let rows = settings(&app);
    assert!(
        rows[0]
            .1
            .contains("“qa_yollar” katmanından ağ oluşturun (O)"),
        "{rows:?}"
    );
    // Katmandan ağ oluştur: a road network of the layer, the tool on its first stop.
    send(&mut app, Message::PromptOption("O"));
    let networks = app
        .document
        .as_ref()
        .expect("open")
        .model
        .settings()
        .networks
        .clone();
    assert_eq!(networks.len(), 1);
    assert_eq!(networks[0].name, "qa_yollar ağı");
    assert!(
        prompt(&app)
            .starts_with("En kısa yol: ilk durağa tıklayın ya da Y,X yazın [Ağ (A): qa_yollar ağı"),
        "{}",
        prompt(&app)
    );
    let rows = settings(&app);
    assert_eq!(rows[0], ("Ağ".to_owned(), "qa_yollar ağı".to_owned()));
    assert_eq!(rows[1], ("Maliyet".to_owned(), "Uzunluk, metre".to_owned()));
    assert_eq!(rows[2], ("Tolerans".to_owned(), app.format().length(0.01)));
    assert_eq!(rows[3], ("Bağlanma".to_owned(), "uçlarda".to_owned()));
    // A → B: the direct road, 50 m.
    for (x, y) in [(500_000.0, 4_320_000.0), (500_030.0, 4_320_040.0)] {
        click(&mut app, x, y);
    }
    let log: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        prompt(&app).contains("Uzunluk 50.000 m"),
        "{} / {:?}",
        prompt(&app),
        &log[log.len().saturating_sub(4)..]
    );
}

#[test]
fn a_network_saved_while_the_tool_runs_brings_it_to_its_first_stop() {
    let mut app = roads();
    send(&mut app, Message::Run("tool.netRoute"));
    assert!(prompt(&app).contains("projede ağ tanımlı değil"));
    // Ağlar's Kaydet or a script: the network written by its command, the tool still running.
    let def = {
        let form = kentos_interaction::network::form::new_form(
            "ag-1",
            "Yollar",
            kentos_commands::NetworkKind::Road,
            "yol",
            |m| m,
        );
        kentos_interaction::network::form::def_of(&form, |v| v).expect("a network")
    };
    crate::files_testing::command(
        &mut app,
        "cad.network.define",
        serde_json::json!({ "operation": "set", "network": serde_json::to_value(def).expect("json") }),
    );
    let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(
        iced::Point::new(10.0, 10.0),
    )));
    assert_eq!(app.session.tool_id(), "netRoute", "the tool runs on");
    assert!(
        prompt(&app)
            .starts_with("En kısa yol: ilk durağa tıklayın ya da Y,X yazın [Ağ (A): Yollar"),
        "{}",
        prompt(&app)
    );
}

/// The pictures (`.run/gui-evidence/kullanim-9332/`): En kısa yol without a network, Katmandan ağ oluştur offered and
/// said in Öznitelikler; the network made, its name, cost and joining at the top of Öznitelikler.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop-cbs prerequisite_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9332", "01-agsiz-rota", || {
        let mut app = roads();
        send(&mut app, Message::Run("tool.netRoute"));
        app
    });
    crate::files_testing::usage_shots("9332", "02-katmandan-ag", || {
        let mut app = roads();
        send(&mut app, Message::Run("tool.netRoute"));
        send(&mut app, Message::PromptOption("O"));
        app
    });
}
