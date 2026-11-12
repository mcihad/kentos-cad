//! The pictures of docs/adr/1061 (CAD-ADV-17): the exchange set ezdxf writes (`fixtures/exchange/v1/set.dxf`)
//! opened, its elevated line and point selected and 2B düzleme indir's preview window over them; and the same with
//! Kotları bir düzleme getir and Ölçüleri de kaldır. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=duzleme-indir,duzleme-indir-kot cargo test -p kentos-desktop --lib tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use crate::app::{App, Message};
use crate::flatten_window::Event;
use crate::tools_screens::Scene;

fn set() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exchange/v1/set.dxf")
}

/// The set opened as the desktop opens a DXF, its elevated objects selected, the window open.
fn opened_with_elevations(app: &mut App) {
    let bytes = std::fs::read(set()).unwrap_or_default();
    let Ok(opened) = kentos_native_application::cad_open::read(
        &set(),
        &bytes,
        kentos_native_application::cad_open::Format::Dxf,
        &mut kentos_formats::watch::Quiet,
    ) else {
        return;
    };
    crate::wall_scenes::opened(app, crate::tools_scenes::Objects::new(), 50.0);
    let Ok(doc) = crate::document::Document::from_v2(opened.document.to_snapshot_v2(), None) else {
        return;
    };
    app.show_document(doc);
    let picked: Vec<kentos_domain::Slot> = app
        .document
        .as_ref()
        .map(|d| {
            d.model
                .entities()
                .filter(|e| match e {
                    kentos_commands::Entity::Line(l) => l.za.is_some(),
                    kentos_commands::Entity::Point(p) => p.z.is_some(),
                    _ => false,
                })
                .map(|e| kentos_domain::Slot(e.base().id))
                .collect()
        })
        .unwrap_or_default();
    let ids: Vec<f64> = picked.iter().map(|s| f64::from(s.0)).collect();
    app.zoom_to_objects(&ids);
    app.selection.set(picked);
    let _ = app.run("cad.flatten");
}

fn preview(app: &mut App) {
    opened_with_elevations(app);
}

fn to_level(app: &mut App) {
    opened_with_elevations(app);
    let _ = app.update(Message::Flatten(Event::Level(true)));
    let _ = app.update(Message::Flatten(Event::Plane(2.5)));
    let _ = app.update(Message::Flatten(Event::Measures(true)));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("duzleme-indir", preview), ("duzleme-indir-kot", to_level)]
}
