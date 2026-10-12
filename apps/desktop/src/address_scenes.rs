//! Adres arama's pictures (docs/adr/0229 §13) over the address scene
//! (fixtures/interaction/v1/addresses.kcad, scripts/fixtures/address_scene.py):
//! CBS's Harita tab with its Adres panel and CAD's Ekle; the Adres tab with
//! an address found and its door marked, a tie, Kaynak's list, Konumdan
//! adres's answer; Adres verisi with Denetle's summary; Adres sağlayıcıları
//! on Nominatim's card; Adresleri eşle's window and its points and report.
//! The web is paused (docs/adr/0228): the desktop's alone. `tools_screens`
//! takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=adres-serit,adres-serit-cad,adres-ara,adres-belirsiz,adres-kaynak,adres-konumdan,adres-verisi,adres-saglayicilar,adres-islem,adres-eslendi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use kentos_contracts::DocumentSnapshotV1;
use kentos_interaction::Vec2;
use kentos_ui::snapshot::Snapshot;
use serde_json::json;

use crate::address::Event;
use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::drive;
use crate::processing::Event as Processing;
use crate::tools_screens::Pointed;

pub(crate) const ADDRESSES: &str = include_str!("../../../fixtures/interaction/v1/addresses.kcad");

/// The scene's origin.
const E: f64 = 500_000.0;
const N: f64 = 4_400_000.0;

/// The address scene open in `app`, the view on the three neighbourhoods.
pub(crate) fn open_addresses(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(ADDRESSES).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/addresses.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 20.0,
            min_y: N - 20.0,
            max_x: E + 420.0,
            max_y: N + 420.0,
        },
        24.0,
    );
}

fn send(app: &mut App, message: Message) {
    let task = app.update(message);
    drive(app, task);
}

fn ev(app: &mut App, e: Event) {
    send(app, Message::Address(e));
}

/// The Adres tab with `text` looked for.
fn found(app: &mut App, text: &str) {
    open_addresses(app);
    send(app, Message::Run("address.search"));
    ev(app, Event::Text(text.to_owned()));
    ev(app, Event::Find);
}

fn nothing(_s: &mut Snapshot, _app: &mut App) {}

fn processing(app: &mut App, e: Processing) {
    send(app, Message::Processing(e));
}

/// Adresleri eşle's window with the address list of the processing cases (fixtures/processing/v1/adresler.csv).
fn geocode_window(app: &mut App) {
    open_addresses(app);
    send(app, Message::Run("processing.run.address.geocode"));
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/processing/v1/adresler.csv");
    let bytes = std::fs::read(&path).expect("the list reads");
    processing(
        app,
        Processing::FileChosen("file".into(), Some(("adresler.csv".into(), path, bytes))),
    );
    processing(app, Processing::Value("address".into(), json!("Adres")));
    processing(
        app,
        Processing::Value("via".into(), json!("locator:adres-1")),
    );
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // CBS's Harita: the Adres panel after Altlık (Adres ara large, the five others small).
        (
            "adres-serit",
            |app| {
                open_addresses(app);
                app.tab = "map";
            },
            nothing,
        ),
        // CAD's Ekle: the same panel last.
        (
            "adres-serit-cad",
            |app| {
                open_addresses(app);
                crate::files_testing::make_cad(app);
                app.tab = "insert";
            },
            nothing,
        ),
        // An address in one line with its abbreviations: door 4 matched, the row's click marks it.
        (
            "adres-ara",
            |app| {
                found(app, "Kızılay Mah. Atatürk Blv. No:4 Çankaya/Ankara");
                ev(app, Event::Press(0));
            },
            nothing,
        ),
        // Without its neighbourhood: door 2 in two neighbourhoods, Belirsiz; the second chosen and marked.
        (
            "adres-belirsiz",
            |app| {
                found(app, "Atatürk Bulvarı No:2");
                ev(app, Event::Press(1));
            },
            nothing,
        ),
        // Kaynak's list: the project's data, the device's providers dimmed with why, the two windows below.
        (
            "adres-kaynak",
            |app| found(app, "Kızılay Mah. Atatürk Blv. No:4 Çankaya/Ankara"),
            |s, app| {
                // The list's own box, at the bar's left (the table's Kaynak cells say Numarataj too).
                let at = crate::files_testing::find_texts(s, app, "Numarataj")
                    .into_iter()
                    .min_by(|a, b| a.x.total_cmp(&b.x))
                    .expect("Kaynak's box is on screen");
                let mut update = |app: &mut App, message: Message| {
                    let _ = app.update(message);
                };
                s.input(
                    app,
                    App::view,
                    &mut update,
                    kentos_ui::snapshot::Input::Click(at.center()),
                );
            },
        ),
        // Konumdan adres in Bahçelievler: no door near, the street's nearest point and its distance.
        (
            "adres-konumdan",
            |app| {
                open_addresses(app);
                let _ = app.update(Message::Run("address.reverse"));
                let task = app.address_at(Vec2::new(E + 150.0, N + 250.0));
                drive(app, task);
            },
            nothing,
        ),
        // Adres verisi: Numarataj's three parts and Denetle's summary (two doors left out, door 14 twice).
        (
            "adres-verisi",
            |app| {
                open_addresses(app);
                send(app, Message::Run("address.data"));
                ev(app, Event::Data(crate::address::data::Event::Check));
            },
            nothing,
        ),
        // Adres sağlayıcıları on Nominatim's card: what goes where, the limits, the terms to accept.
        (
            "adres-saglayicilar",
            |app| {
                open_addresses(app);
                send(app, Message::Run("address.providers"));
            },
            nothing,
        ),
        // Adresleri eşle: the address list, the project's data as the source.
        ("adres-islem", geocode_window, nothing),
        // Its run: the points at their places, the rows not written in the report.
        (
            "adres-eslendi",
            |app| {
                geocode_window(app);
                processing(app, Processing::Run);
            },
            nothing,
        ),
    ]
}
