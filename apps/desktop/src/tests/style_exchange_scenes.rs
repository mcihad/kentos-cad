//! The pictures of docs/adr/0571 (the rest of style exchange): QGIS 4.2's QML layer styles taken into layers, a
//! gradient with dots over it and an outline on two parcels, an SVG marker with a font marker on it on points (the
//! SVG joins the project's library), and the SLD of a point pattern on a third parcel. `tools_screens` takes them in
//! the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=stil-simgeler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_ui::snapshot::Snapshot;
use serde_json::json;

use crate::app::{App, Message};
use crate::layering::Event;
use crate::tools_scenes::{E, N, Objects, layer, open_edited};
use crate::tools_screens::Pointed;

const PARCEL: [[f64; 2]; 4] = [[0.0, 0.0], [36.0, 0.0], [36.0, 22.0], [0.0, 22.0]];

fn shifted(by: [f64; 2]) -> Vec<[f64; 2]> {
    PARCEL.iter().map(|p| [p[0] + by[0], p[1] + by[1]]).collect()
}

fn fixture(kind: &str, name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(kind)
        .join("v1")
        .join(name)
}

/// Three parcels on Parsel and four stops on Durak (a point layer of its own), each layer's look a file's: the QML's
/// gradient, dots and outline on the first two (the second with the SLD's dots instead), the markers on the stops.
fn imported(app: &mut App) {
    let mut o = Objects::new();
    o.path("parsel", &shifted([0.0, 0.0]), true);
    o.path("parsel", &shifted([40.0, 0.0]), true);
    o.path("sd", &shifted([0.0, 26.0]), true);
    for x in [8.0, 20.0, 32.0, 52.0] {
        o.point("durak", [x, -6.0], None);
    }
    open_edited(app, o, |d| {
        d["layers"] = json!([
            layer("parsel", "Parsel", "#3E63DD", 0.35),
            layer("sd", "Desenli parsel", "#2E7D32", 0.35),
            layer("durak", "Durak", "fg", 0.25),
        ]);
        d["activeLayer"] = json!("parsel");
    });
    for (id, kind, file) in [
        ("parsel", "qml", "qgis-simgeler-alan.qml"),
        ("durak", "qml", "qgis-simgeler-nokta.qml"),
    ] {
        let _ = app.update(Message::Layer(Event::QmlImportFrom(
            id.into(),
            Some(fixture(kind, file)),
        )));
    }
    let _ = app.update(Message::Layer(Event::SldImportFrom(
        "sd".into(),
        Some(fixture("sld", "qgis-desen.sld")),
    )));
    app.tab = "map";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 12.0,
            max_x: E + 80.0,
            max_y: N + 52.0,
        },
        24.0,
    );
}

fn settled(s: &mut Snapshot, app: &mut App) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    for _ in 0..3 {
        s.settle(app, App::view, &mut update);
    }
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("stil-simgeler", imported, settled),
        (
            "stil-simgeler-pencere",
            |app| {
                imported(app);
                app.open_layer_style(Some("durak".into()));
            },
            settled,
        ),
    ]
}
