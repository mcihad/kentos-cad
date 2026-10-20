//! The pictures of docs/adr/0225 (Nesne ekleri) on the shared trace's drawing
//! (`attachments.kcad`, scripts/fixtures/attachments_scene.py): a block's
//! parcels with photos, a document and an address, the layer's card and
//! actions, the field photos' points. Öznitelikler's Ekler and Eylemler, the
//! hover card's photo, Bilgi's card on a parcel and on a photo's point, the
//! photo viewer, Adres ekle, Bilgi kartı ve eylemler, Konumlu fotoğraflar
//! over the EXIF fixtures, the drawing's menu with Eylemler ▸. The web's are
//! `shots.mjs attachments`. The photos are drawn by the shell's own wgpu
//! pipeline (photo_view.rs), so the pictures want the wgpu snapshot:
//!
//! ```text
//! KENTOS_SNAPSHOT_BACKEND=wgpu KENTOS_SHOTS_ONLY=ekler-oznitelikler,ekler-uzerine,ekler-kart,ekler-kart-foto,ekler-goruntuleyici,ekler-adres,ekler-bilgi-karti,ekler-konumlu,ekler-eylemler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;
use std::sync::Arc;

use kentos_domain::Slot;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::attachments::Event;
use crate::document::Document;
use crate::tools_scenes::{E, N, click, hover, run};
use crate::tools_screens::{Pointed, Scene};

const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/attachments.kcad");

/// The drawing beside its linked deed, the view around the block.
pub(crate) fn opened(app: &mut App) {
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/attachments.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "data";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 34.0,
            min_y: N - 20.0,
            max_x: E + 26.0,
            max_y: N + 22.0,
        },
        24.0,
    );
}

/// The layers folded away, so Öznitelikler has the dock.
fn fold_layers(app: &mut App) {
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// 101/5 selected: its four attachments (two photos, the deed, the address) and its layer's two actions.
fn properties(app: &mut App) {
    opened(app);
    app.selection.set([Slot(1)]);
    fold_layers(app);
    for id in ["general", "geometry"] {
        let _ = app.update(Message::Properties(crate::properties::Event::Toggle(id)));
    }
    app.photos_now();
    hover(app, [-28.0, -18.0]);
}

/// The pointer resting on 101/5: the hover card's rows end with Ekler, its first photo under them.
fn rollover(app: &mut App) {
    opened(app);
    hover(app, [-6.0, 2.0]);
    app.hover_card = app.selection.hover();
    app.photos_now();
}

/// Bilgi on 101/5: the layer's card, its photos, the deed and the address, the actions.
fn card(app: &mut App) {
    opened(app);
    app.tab = "map";
    run(app, "tool.identify");
    click(app, [-6.0, 2.0]);
    app.photos_now();
    hover(app, [-28.0, -18.0]);
}

/// Bilgi on a photo's point: the Fotoğraflar layer's card.
fn photo_card(app: &mut App) {
    opened(app);
    app.tab = "map";
    run(app, "tool.identify");
    click(app, [8.0, 17.5]);
    app.photos_now();
    hover(app, [-28.0, -18.0]);
}

/// The viewer on 101/5's façade, the first of its two photos.
fn viewer(app: &mut App) {
    opened(app);
    app.selection.set([Slot(1)]);
    let _ = app.attachment_event(Event::Open(Slot(1), 0));
    app.photos_now();
}

/// Adres ekle on 101/8, an address and its note typed.
fn link(app: &mut App) {
    opened(app);
    app.selection.set([Slot(4)]);
    let _ = app.attachment_event(Event::AddLink(Slot(4)));
    let _ = app.attachment_event(Event::FormUrl(
        "https://imar.example.org/ada/101/parsel/8".to_owned(),
    ));
    let _ = app.attachment_event(Event::FormName("İmar durumu".to_owned()));
    let _ = app.attachment_event(Event::FormNote(iced::widget::text_editor::Action::Edit(
        iced::widget::text_editor::Edit::Paste(Arc::new(
            "Belediyenin imar durumu sayfası".to_owned(),
        )),
    )));
}

/// Bilgi kartı ve eylemler on Parsel, 101/5 selected for the preview.
fn layer_card(app: &mut App) {
    opened(app);
    app.selection.set([Slot(1)]);
    let _ = app.open_layer_info(Some("parsel".to_owned()));
    app.photos_now();
}

/// Konumlu fotoğraflar over four EXIF fixtures: two placed (one dated by its
/// GPS clock), one without GPS, one with a broken GPS.
fn photo_import(app: &mut App) {
    opened(app);
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exif/v1");
    let files = ["konum.jpg", "gps-saati.jpg", "gpssiz.jpg", "yarim-gps.jpg"]
        .into_iter()
        .map(|name| {
            let path = folder.join(name);
            let bytes = std::fs::read(&path).expect("the fixture reads");
            crate::exchange::photo_import::PhotoFile {
                name: name.to_owned(),
                path,
                bytes: bytes.into(),
            }
        })
        .collect();
    app.photo_import_now(files);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("ekler-oznitelikler", properties),
        ("ekler-uzerine", rollover),
        ("ekler-kart", card),
        ("ekler-kart-foto", photo_card),
        ("ekler-goruntuleyici", viewer),
        ("ekler-adres", link),
        ("ekler-bilgi-karti", layer_card),
        ("ekler-konumlu", photo_import),
    ]
}

/// The drawing's menu over 101/5, Eylemler ▸ open.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![(
        "ekler-eylemler",
        |app| {
            opened(app);
            hover(app, [-6.0, 2.0]);
        },
        |s, app| {
            let area = app.viewport.bounds;
            let p = crate::tools_scenes::at(app, [-6.0, 2.0]);
            let at = iced::Point::new(area.x + p.x, area.y + p.y);
            let mut update = |app: &mut App, message: Message| {
                let _ = app.update(message);
            };
            s.input(
                app,
                App::view,
                &mut update,
                kentos_ui::snapshot::Input::RightClick(at),
            );
            s.settle(app, App::view, &mut update);
            // Eylemler is the menu's last entry: up from its foot, then into it.
            use iced::keyboard::key::Named;
            crate::tools_screens::press_keys(s, app, &[Named::ArrowUp, Named::ArrowRight]);
        },
    )]
}
