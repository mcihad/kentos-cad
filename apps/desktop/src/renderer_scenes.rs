//! Ek işleyiciler of docs/adr/0213 in a GIS project (`renderers.kcad`,
//! written by `scripts/fixtures/renderer_scene.py`): districts coloured by
//! a number (Sürekli renk) and by two (İki değişkenli renk), dotted by their
//! people (Nokta yoğunluğu) and charted (Grafik), incidents as a heat map,
//! trees clustered, bus stops spread round their place, schools and roads
//! sized by a number, the work area's outside covered (Ters alan, hidden
//! until shown). The web's are `shots.mjs renderers`. `tools_screens` takes
//! them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=isleyici-sahne,isleyici-isi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Each renderer's region closer in (`isleyici-*`), Ters alan shown, the
//! Katman stili window of each (`isleyici-pencere-*`) and its list, the legend.
//!
//! Test code only.

use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_screens::{Pointed, press_caption};

pub(crate) const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/renderers.kcad");
const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;

/// The regions of the scene, relative to its origin (the web's `REGIONS`).
const SAHNE: [f64; 4] = [-40.0, -140.0, 3700.0, 1300.0];
const SUREKLI: [f64; 4] = [-30.0, 570.0, 930.0, 1270.0];
const IKI: [f64; 4] = [970.0, 570.0, 1930.0, 1270.0];
const NOKTA: [f64; 4] = [-30.0, -130.0, 930.0, 570.0];
const GRAFIK: [f64; 4] = [970.0, -130.0, 1930.0, 570.0];
const ISI: [f64; 4] = [1970.0, 570.0, 2930.0, 1270.0];
const KUME: [f64; 4] = [1970.0, -130.0, 2930.0, 570.0];
const YAYMA: [f64; 4] = [2970.0, 620.0, 3640.0, 1260.0];
const ORANTILI: [f64; 4] = [2960.0, -120.0, 3660.0, 560.0];

/// The drawing open, the CBS ribbon's Harita on.
fn opened(app: &mut App) {
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "map";
    app.command_expanded = false;
}

/// Ters alan's layer shown too.
fn inverted_shown(app: &mut App) {
    opened(app);
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_layer_visible("calisma", true);
    }
}

/// Katman stili over a layer.
fn window(app: &mut App, layer: &str) {
    opened(app);
    app.open_layer_style(Some(layer.to_owned()));
}

/// The view on the box (relative to the scene's origin), once the drawing area has its size.
fn fit(s: &mut Snapshot, app: &mut App, b: [f64; 4]) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    s.settle(app, App::view, &mut update);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: X0 + b[0],
            min_y: Y0 + b[1],
            max_x: X0 + b[2],
            max_y: Y0 + b[3],
        },
        24.0,
    );
    s.settle(app, App::view, &mut update);
    // A heat map, a cluster and a spread are built for the view's scale once it settles.
    s.settle(app, App::view, &mut update);
}

/// The legend over the whole scene.
fn legend(s: &mut Snapshot, app: &mut App) {
    fit(s, app, SAHNE);
    let _ = app.update(Message::Run("style.legend"));
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    s.settle(app, App::view, &mut update);
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The whole scene: every renderer at work.
        ("isleyici-sahne", opened, |s, app| fit(s, app, SAHNE)),
        ("isleyici-surekli", opened, |s, app| fit(s, app, SUREKLI)),
        ("isleyici-iki", opened, |s, app| fit(s, app, IKI)),
        ("isleyici-nokta", opened, |s, app| fit(s, app, NOKTA)),
        ("isleyici-grafik", opened, |s, app| fit(s, app, GRAFIK)),
        ("isleyici-isi", opened, |s, app| fit(s, app, ISI)),
        ("isleyici-kume", opened, |s, app| fit(s, app, KUME)),
        ("isleyici-yayma", opened, |s, app| fit(s, app, YAYMA)),
        ("isleyici-orantili", opened, |s, app| fit(s, app, ORANTILI)),
        // Ters alan: the work area's outside covered.
        ("isleyici-ters", inverted_shown, |s, app| fit(s, app, SAHNE)),
        // Katman stili: the list of kinds by group, and each kind's form.
        (
            "isleyici-pencere-liste",
            |app| window(app, "surekli"),
            |s, app| {
                fit(s, app, SUREKLI);
                press_caption(s, app, "Sürekli renk", false, false);
            },
        ),
        (
            "isleyici-pencere-surekli",
            |app| window(app, "surekli"),
            |s, app| fit(s, app, SUREKLI),
        ),
        (
            "isleyici-pencere-orantili",
            |app| window(app, "okul"),
            |s, app| fit(s, app, ORANTILI),
        ),
        (
            "isleyici-pencere-iki",
            |app| window(app, "iki"),
            |s, app| fit(s, app, IKI),
        ),
        (
            "isleyici-pencere-nokta",
            |app| window(app, "nokta"),
            |s, app| fit(s, app, NOKTA),
        ),
        (
            "isleyici-pencere-grafik",
            |app| window(app, "grafik"),
            |s, app| fit(s, app, GRAFIK),
        ),
        (
            "isleyici-pencere-isi",
            |app| window(app, "olay"),
            |s, app| fit(s, app, ISI),
        ),
        (
            "isleyici-pencere-kume",
            |app| window(app, "agac"),
            |s, app| fit(s, app, KUME),
        ),
        (
            "isleyici-pencere-yayma",
            |app| window(app, "durak"),
            |s, app| fit(s, app, YAYMA),
        ),
        (
            "isleyici-pencere-ters",
            |app| window(app, "calisma"),
            |s, app| fit(s, app, SAHNE),
        ),
        // Lejant: each renderer's rows.
        ("isleyici-lejant", opened, legend),
    ]
}
