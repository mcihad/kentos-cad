//! The selection on the desktop (docs/adr/0029): the web's selection
//! commands (`app/commands.ts`: Tümünü seç, Seçimi kaldır, Seçimi ters
//! çevir, Seçime yakınlaştır) and what the properties panel says about a
//! selection (`ui/properties/PropertiesPanel.ts`): how many, of which kinds,
//! on which layer, in which colour, their total length and area, and for one
//! object its attributes. Editing them there is a later slice.

use std::borrow::Cow;

use kentos_contracts::Entity;
use kentos_domain::{LayerTree, Slot};
use kentos_interaction::{Format, ViewChange};

use crate::app::App;
use crate::document::Document;

/// A row of the properties panel: its name and its value.
pub type Row = (Cow<'static, str>, String);

/// Seçime yakınlaştır's margin, logical pixels (the web's `camera.fit(b, 96)`).
const SELECTION_PADDING: f64 = 96.0;

impl App {
    /// `edit.selectAll` (Ctrl+A): every object on a shown layer, and how many.
    pub(crate) fn select_all(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let layers = doc.model.layers();
        let ids: Vec<Slot> = doc
            .model
            .entities()
            .filter(|e| layers.is_visible(&e.base().layer_id))
            .map(|e| Slot(e.base().id))
            .collect();
        let n = ids.len();
        self.selection.set(ids);
        self.output(format!("{n} nesne seçildi."));
    }

    /// `edit.invertSelection`: the objects on shown layers not selected now.
    pub(crate) fn invert_selection(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let layers = doc.model.layers();
        let selection = &self.selection;
        let ids: Vec<Slot> = doc
            .model
            .entities()
            .filter(|e| layers.is_visible(&e.base().layer_id))
            .map(|e| Slot(e.base().id))
            .filter(|slot| !selection.contains(*slot))
            .collect();
        self.selection.set(ids);
    }

    /// `view.zoomSelection` (Ctrl+Shift+F): the selection's box as large as
    /// it fits, 96 px in from the edges (the web's `zoomToSelection`,
    /// docs/adr/0056); nothing with no selection (the web's command is off).
    pub(crate) fn zoom_selection(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        if self.selection.is_empty() {
            return;
        }
        self.spatial.sync(&doc.model);
        let ids: Vec<f64> = self
            .selection
            .ids()
            .iter()
            .map(|slot| f64::from(slot.0))
            .collect();
        if let Some(bounds) = self.spatial.store().extent(Some(&ids)) {
            self.viewport.change(ViewChange::Fit {
                bounds,
                padding: SELECTION_PADDING,
            });
        }
    }

    /// The properties panel's rows for the selection; none when nothing is selected.
    pub(crate) fn selection_rows(&self, doc: &Document) -> Option<Vec<Row>> {
        let objects: Vec<&Entity> = self
            .selection
            .ids()
            .iter()
            .filter_map(|slot| doc.model.get(*slot))
            .collect();
        let first = *objects.first()?;
        let layers = doc.model.layers();
        let format = Format::of(doc.settings());
        let ids: Vec<f64> = objects.iter().map(|e| f64::from(e.base().id)).collect();
        // Summed by the geometry store, as the web sums them: a selection can be large.
        let (length, area) = self.spatial.store().measure(&ids);
        let mut rows: Vec<Row> = Vec::new();
        let mut row = |key: &'static str, value: String| rows.push((Cow::Borrowed(key), value));
        if let [one] = objects.as_slice() {
            let base = one.base();
            let kind = match base.label.as_deref().filter(|l| !l.is_empty()) {
                Some(label) => format!("{} · {label}", kind_title(one.kind())),
                None => kind_title(one.kind()).to_owned(),
            };
            let layer = layer_path(layers, &base.layer_id);
            row("Nesne", format!("#{}", base.id));
            row("Tür", kind);
            row(
                "Katman",
                if layers.is_locked(&base.layer_id) {
                    format!("{layer} (kilitli)")
                } else {
                    layer
                },
            );
            row(
                "Renk",
                base.color.clone().unwrap_or_else(|| "Katmana göre".into()),
            );
        } else {
            // Kinds in the order first met, lower case (the web's summary).
            let mut kinds: Vec<(&'static str, usize)> = Vec::new();
            for e in &objects {
                match kinds.iter_mut().find(|(k, _)| *k == e.kind()) {
                    Some((_, n)) => *n += 1,
                    None => kinds.push((e.kind(), 1)),
                }
            }
            let kinds: Vec<String> = kinds
                .iter()
                .map(|(k, n)| format!("{n} {}", kind_title(k).to_lowercase()))
                .collect();
            let same_layer = objects
                .iter()
                .all(|e| e.base().layer_id == first.base().layer_id);
            let any_locked = objects.iter().any(|e| layers.is_locked(&e.base().layer_id));
            let same_color = objects.iter().all(|e| e.base().color == first.base().color);
            row("Seçim", format!("{} nesne seçili", objects.len()));
            row("Türler", kinds.join(", "));
            row(
                "Katman",
                if any_locked {
                    "Kilitli katman içeriyor".to_owned()
                } else if same_layer {
                    layer_path(layers, &first.base().layer_id)
                } else {
                    "Çeşitli".to_owned()
                },
            );
            row(
                "Renk",
                match (same_color, &first.base().color) {
                    (false, _) => "Çeşitli".to_owned(),
                    (true, Some(color)) => color.clone(),
                    (true, None) => "Katmana göre".to_owned(),
                },
            );
        }
        let total = objects.len() > 1;
        if length > 0.0 {
            row(
                if total { "Toplam uzunluk" } else { "Uzunluk" },
                format.length(length),
            );
        }
        if area > 0.0 {
            row(
                if total { "Toplam alan" } else { "Alan" },
                format.area(area),
            );
        }
        if let [one] = objects.as_slice() {
            // Its attributes, by name.
            for (key, value) in &one.base().attrs {
                rows.push((Cow::Owned(key.clone()), value.clone()));
            }
        }
        Some(rows)
    }
}

/// A layer's place in the tree, groups first: `Kadastro / Parsel` (the web's `path`).
fn layer_path(layers: &LayerTree, id: &str) -> String {
    let mut names = Vec::new();
    let mut at = layers.get(id);
    while let Some(node) = at {
        names.push(node.name.clone());
        at = layers.parent(&node.id);
    }
    if names.is_empty() {
        return id.to_owned();
    }
    names.reverse();
    names.join(" / ")
}

/// An object kind as the web titles it (`ENTITY_KIND_LABEL`).
pub fn kind_title(kind: &str) -> &'static str {
    match kind {
        "point" => "Nokta",
        "line" => "Çizgi",
        "polyline" => "Çoklu çizgi",
        "polygon" => "Kapalı alan",
        "circle" => "Daire",
        "arc" => "Yay",
        "ellipse" => "Elips",
        "spline" => "Eğri",
        "xline" => "Yardımcı çizgi",
        "ray" => "Işın",
        "text" => "Yazı",
        "dimension" => "Ölçü",
        "hatch" => "Tarama",
        _ => "Nesne",
    }
}

#[cfg(test)]
mod tests {
    use iced::keyboard::Modifiers;
    use iced::{Point, Rectangle, Size};
    use kentos_contracts::DocumentSnapshotV1;
    use kentos_domain::Slot;
    use serde_json::Value;

    use crate::app::{App, Message};
    use crate::document::Document;
    use crate::viewport::Event;

    /// The selection traces' drawing (fixtures/interaction/v1/objects.kcad),
    /// open in an area of 800 × 600 at 0.125 m per pixel around its centre.
    fn objects() -> App {
        let (mut app, _) = App::boot(None);
        let snapshot = DocumentSnapshotV1::from_json(include_str!(
            "../../../fixtures/interaction/v1/objects.kcad"
        ))
        .expect("the drawing reads");
        let doc = Document::new(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        let _ = app.update(Message::Viewport(Event::Resized(Rectangle::new(
            Point::ORIGIN,
            Size::new(800.0, 600.0),
        ))));
        app.viewport.camera.center = kentos_interaction::Vec2::new(487000.0, 4420000.0);
        app.viewport.camera.scale = 8.0;
        app
    }

    /// The area's pixel of a point given east and north of the centre.
    fn at(de: f32, dn: f32) -> Point {
        Point::new(400.0 + de * 8.0, 300.0 - dn * 8.0)
    }

    fn click(app: &mut App, de: f32, dn: f32) {
        let p = at(de, dn);
        let _ = app.update(Message::Viewport(Event::Moved(p)));
        let _ = app.update(Message::Viewport(Event::Pressed(p)));
        let _ = app.update(Message::Viewport(Event::Released(p)));
    }

    fn selected(app: &App) -> Vec<u32> {
        app.selection.ids().iter().map(|s| s.0).collect()
    }

    #[test]
    fn the_selection_commands_follow_the_web() {
        let mut app = objects();
        let _ = app.run("edit.selectAll");
        assert_eq!(
            selected(&app),
            [1, 2, 3, 4, 5, 6],
            "every object on a shown layer"
        );
        assert!(app.available("edit.deselect"));
        click(&mut app, -16.0, -12.25);
        assert_eq!(selected(&app), [1]);
        let _ = app.run("edit.invertSelection");
        assert_eq!(selected(&app), [2, 3, 4, 5, 6]);
        let _ = app.run("edit.deselect");
        assert!(selected(&app).is_empty());
        assert!(!app.available("edit.deselect"));
        // Esc clears a selection while no command runs, and ends a command before that.
        click(&mut app, -16.0, -12.25);
        let _ = app.run("tool.line");
        let _ = app.run("tool.cancel");
        assert_eq!(selected(&app), [1], "Esc left the command");
        let _ = app.run("tool.cancel");
        assert!(selected(&app).is_empty());
        // tool.select leaves a command and keeps the selection.
        click(&mut app, -16.0, -12.25);
        let _ = app.run("tool.polygon");
        let _ = app.run("tool.select");
        assert_eq!((app.session.tool_id(), selected(&app)), ("select", vec![1]));
    }

    #[test]
    fn the_properties_panel_says_what_is_selected() {
        let mut app = objects();
        let doc = app.document.clone().expect("open");
        assert!(app.selection_rows(&doc).is_none());
        click(&mut app, -18.0, 9.0);
        let rows = app.selection_rows(&doc).expect("one object");
        let get = |rows: &[super::Row], key: &str| {
            rows.iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        assert_eq!(get(&rows, "Tür"), "Kapalı alan");
        assert_eq!(get(&rows, "Katman"), "Parsel");
        assert_eq!(get(&rows, "Alan"), "120.00 m²");
        assert_eq!(get(&rows, "Parsel"), "7", "its attributes");
        let _ = app.update(Message::Modifiers(Modifiers::SHIFT));
        click(&mut app, -16.0, -12.25);
        click(&mut app, 10.0, -16.25);
        let _ = app.update(Message::Modifiers(Modifiers::empty()));
        let rows = app.selection_rows(&doc).expect("three objects");
        assert_eq!(get(&rows, "Seçim"), "3 nesne seçili");
        assert_eq!(get(&rows, "Türler"), "1 kapalı alan, 2 çizgi");
        assert_eq!(get(&rows, "Katman"), "Kilitli katman içeriyor");
        assert_eq!(get(&rows, "Toplam uzunluk"), "28.000 m");
        assert_eq!(get(&rows, "Toplam alan"), "120.00 m²");
    }

    #[test]
    fn f3_and_the_settings_reach_the_snap() {
        let mut app = objects();
        assert_eq!(app.shortcut("F3"), Some("draft.snap"));
        assert!(app.draft.snap);
        let _ = app.run("draft.snap");
        assert!(!app.draft.snap);
        assert!(matches!(
            app.history.last(),
            Some(kentos_ui::widget::command_line::Entry::Output(t)) if t == "Kenetleme kapalı"
        ));
        let _ = app.run("draft.snap");
        let _ = app.settings.choose(&[
            ("snap.endpoint", Value::Bool(false)),
            ("drafting.pickAperture", Value::from(9)),
        ]);
        app.apply_settings();
        let endpoint = kentos_interaction::SnapKind::Endpoint.bit();
        let quadrant = kentos_interaction::SnapKind::Quadrant.bit();
        assert_eq!(app.draft.snap_kinds & (endpoint | quadrant), 0);
        assert_eq!(app.draft.pick_aperture, 9.0);
        // A running tool snaps where the pointer is: no endpoint kind now, the middle still snaps.
        let _ = app.run("tool.polygon");
        let _ = app.update(Message::Viewport(Event::Moved(at(-23.5, -11.625))));
        assert_eq!(app.snap, None, "no endpoint snap");
        let _ = app.update(Message::Viewport(Event::Moved(at(-15.625, -11.5))));
        assert_eq!(
            app.snap.map(|s| s.kind),
            Some(kentos_interaction::SnapKind::Midpoint)
        );
        let _ = app.run("tool.cancel");
        assert_eq!(app.snap, None, "the marker goes with the command");
    }

    #[test]
    fn the_store_follows_what_the_app_does_to_the_drawing() {
        let mut app = objects();
        click(&mut app, -16.0, -12.25);
        let _ = app.run("tool.erase");
        assert_eq!(app.document.as_ref().map(Document::entity_count), Some(6));
        let _ = app.run("edit.undo");
        let _ = app.run("tool.line");
        click(&mut app, -2.0, -18.0);
        click(&mut app, 8.0, -18.0);
        let _ = app.run("tool.cancel");
        let _ = app.update(Message::LayerVisible("parsel".into()));
        let doc = app.document.as_ref().expect("open");
        let fresh = kentos_interaction::Spatial::of(&doc.model);
        assert_eq!(app.spatial.store().ids(), fresh.store().ids());
        assert_eq!(app.spatial.reloads(), 1, "read once, when opened");
        assert_eq!(app.spatial.len(), 8);
        // The hidden layer's area is no longer picked.
        click(&mut app, -18.0, 9.0);
        assert!(selected(&app).is_empty());
        assert!(app.selection.ids().iter().all(|s| *s != Slot(4)));
    }
}
