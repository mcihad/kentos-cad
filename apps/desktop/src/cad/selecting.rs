//! The selection on the desktop (docs/adr/0029): the web's selection
//! commands (`app/commands.ts`: Tümünü seç, Seçimi kaldır, Seçimi ters
//! çevir, Seçime yakınlaştır). What the Öznitelikler panel shows of it and
//! edits there is `properties/` (docs/adr/0063).

use kentos_commands::SelectRegion;
use kentos_domain::Slot;
use kentos_interaction::ViewChange;
use kentos_interaction::selectable;
use kentos_native_application::entities_select::Selected;

use crate::app::App;

/// Seçime yakınlaştır's margin, logical pixels (the web's `camera.fit(b, 96)`).
const SELECTION_PADDING: f64 = 96.0;

impl App {
    /// The selection query over the drawing (docs/adr/0275) with the filter's kinds, the store first brought up to
    /// the drawing; none without one.
    pub(crate) fn query_selection(&mut self, region: SelectRegion) -> Option<Selected> {
        let doc = self.document.as_ref()?;
        self.spatial.sync(&doc.model);
        selectable::query_in(&doc.model, &self.spatial, self.draft.select_kinds, region)
    }

    /// `edit.selectAll` (Ctrl+A): every object shown, and how many.
    pub(crate) fn select_all(&mut self) {
        let Some(found) = self.query_selection(SelectRegion::All) else {
            return;
        };
        let n = found.slots.len();
        self.selection.set(found.slots);
        self.say_filtered(found.left_out.kinds as usize);
        self.say_layers_left(found.left_out.layers as usize);
        self.output(format!("{n} nesne seçildi."));
    }

    /// What Otomatik seçim, off, left out of a selection: the other layers' objects (docs/adr/0622 §5).
    fn say_layers_left(&mut self, n: usize) {
        let words = self
            .document
            .as_ref()
            .and_then(|d| selectable::layer_left_words(&d.model, n));
        if let Some(words) = words {
            self.output(words);
        }
    }

    /// What the selection filter left out of a selection, said (the web's `sayFiltered`).
    fn say_filtered(&mut self, n: usize) {
        if n > 0 {
            self.output(format!("Seçim süzgeci {n} nesneyi dışarıda bıraktı."));
        }
    }

    /// The persistent ids of these objects, as the query takes them.
    pub(crate) fn uids_of(&self, slots: &[Slot]) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        slots
            .iter()
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect()
    }

    /// `edit.invertSelection`: the objects shown not selected now.
    pub(crate) fn invert_selection(&mut self) {
        let uids = self.uids_of(self.selection.ids());
        let Some(found) = self.query_selection(SelectRegion::Invert { uids }) else {
            return;
        };
        self.selection.set(found.slots);
        self.say_filtered(found.left_out.kinds as usize);
        self.say_layers_left(found.left_out.layers as usize);
    }

    /// `view.zoomSelection` (Ctrl+Shift+F): the selection's box as large as
    /// it fits, 96 px in from the edges (the web's `zoomToSelection`,
    /// docs/adr/0056); nothing with no selection (the web's command is off).
    pub fn zoom_selection(&mut self) {
        // Nothing of the drawing selected: a data layer's selected rows framed, when there are (docs/adr/9133).
        if self.selection.is_empty() {
            self.zoom_to_data_selection();
            return;
        }
        let ids: Vec<f64> = self
            .selection
            .ids()
            .iter()
            .map(|slot| f64::from(slot.0))
            .collect();
        self.zoom_to_objects(&ids);
    }

    /// The whole drawing in view (Tümünü göster, the middle button's double
    /// click): [`extents_box`]. The objects it leaves out, far from the rest,
    /// are said: Kapsam denetimi selects them (docs/adr/9203).
    pub(crate) fn zoom_extents(&mut self) {
        if self.document.is_none() {
            return;
        }
        // The data layers' boxes as their sources say them now (docs/adr/9327); one still read stands in with the box
        // the layer kept, and the view is fitted again once the sources have answered.
        let shown = self.shown_data_boxes();
        let reading = !shown.reading.is_empty();
        let shown_any = self.extents_shown(&shown, !reading);
        if reading {
            self.fit_when_boxes_read(!shown_any);
            if !shown_any {
                self.say(
                    kentos_interaction::Level::Info,
                    format!(
                        "{} kapsamı kaynağından okunuyor; okununca görünüm sığdırılır.",
                        named_layers(&shown.reading)
                    ),
                );
            }
        }
    }

    /// The view on the box of the drawing's objects (Tümünü göster's, less those far from the rest) and `data`, the
    /// shown data layers' boxes: whether there was a box to show, and how many objects were left out.
    pub(crate) fn show_extents(&mut self, data: &[[f64; 4]]) -> (bool, usize) {
        let Some(doc) = &self.document else {
            return (false, 0);
        };
        let (objects, far) = objects_box(doc, &mut self.spatial);
        let mut boxes = data.to_vec();
        if let Some(b) = &objects {
            boxes.push([b.min_x, b.min_y, b.max_x, b.max_y]);
        }
        let Some(b) = crate::layering::union(&boxes) else {
            return (false, far);
        };
        self.viewport.show(&kentos_render_wgpu::Bounds { min_x: b[0], min_y: b[1], max_x: b[2], max_y: b[3] });
        (true, far)
    }

    /// The view on the box of these objects (by slot), as large as it fits,
    /// 96 px in from the edges: Seçime yakınlaştır's and Katmana yakınlaştır's.
    pub fn zoom_to_objects(&mut self, ids: &[f64]) {
        let Some(doc) = &self.document else {
            return;
        };
        self.spatial.sync(&doc.model);
        match self.spatial.store().extent(Some(ids)) {
            Some(bounds) => self.viewport.change(ViewChange::Fit {
                bounds,
                padding: SELECTION_PADDING,
            }),
            // Records without a geometry are in no place (docs/adr/0410 §4).
            None => self.say(
                kentos_interaction::Level::Info,
                "Seçilenlerin çizimde yeri yok: geometrisiz kayıtlar yalnız tabloda ve formda görünür.",
            ),
        }
    }
}

/// What Tümünü göster shows, and an opened drawing that keeps no view of its own: the box of the objects not on a
/// frozen layer (docs/adr/0311 §1; a hidden one's count), a block's insert by its pieces and a leader by its note as the
/// store has them, with the shown data layers' tables (docs/adr/9101 §5); less the objects lying far from the rest of
/// them by Kapsam denetimi's rule (docs/adr/0141, 9203), whose number comes with it. An object fallen to zero or brought
/// in another system made the whole drawing a dot, and the drawing seemed to have lost its data: the 453 534 objects
/// of a city plan were shown over 8 800 km.
pub(crate) fn extents_box(
    doc: &crate::document::Document,
    spatial: &mut kentos_interaction::Spatial,
) -> (Option<kentos_render_wgpu::Bounds>, usize) {
    let (bounds, far) = objects_box(doc, spatial);
    let settings = doc.settings();
    let mut boxes: Vec<[f64; 4]> = kentos_plot::layers::shown_layers(doc.model.layers().nodes())
        .into_iter()
        .filter_map(|n| crate::data_layers::extent_in_project(n, settings))
        .collect();
    if let Some(b) = &bounds {
        boxes.push([b.min_x, b.min_y, b.max_x, b.max_y]);
    }
    let bounds = crate::layering::union(&boxes).map(|b| kentos_render_wgpu::Bounds {
        min_x: b[0],
        min_y: b[1],
        max_x: b[2],
        max_y: b[3],
    });
    (bounds, far)
}

/// The layers named as a message says them: “Katman” or “3 katmanın”.
fn named_layers(names: &[String]) -> String {
    match names {
        [one] => format!("“{one}” katmanının"),
        many => format!("{} katmanın", many.len()),
    }
}

/// The box of the drawing's objects Tümünü göster shows (see [`extents_box`]), without the data layers' tables, and how
/// many objects lie far from the rest.
fn objects_box(
    doc: &crate::document::Document,
    spatial: &mut kentos_interaction::Spatial,
) -> (Option<kentos_render_wgpu::Bounds>, usize) {
    spatial.sync(&doc.model);
    let tree = doc.model.layers();
    let counted: Option<Vec<f64>> = tree.leaves().iter().any(|l| tree.is_frozen(&l.id)).then(|| {
        doc.model
            .entities()
            .filter(|e| !tree.is_frozen(&e.base().layer_id))
            .map(|e| f64::from(e.base().id))
            .collect()
    });
    let store = spatial.store();
    let far = store.extent_outliers_of(counted.as_deref());
    let bounds = if far.is_empty() {
        store.extent(counted.as_deref())
    } else {
        let left: std::collections::HashSet<u64> = far.iter().map(|id| id.to_bits()).collect();
        let near: Vec<f64> = counted
            .unwrap_or_else(|| doc.model.entities().map(|e| f64::from(e.base().id)).collect())
            .into_iter()
            .filter(|id| !left.contains(&id.to_bits()))
            .collect();
        store.extent(Some(&near))
    };
    (bounds, far.len())
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
        "insert" => "Blok",
        "leader" => "Kılavuz",
        "table" => "Tablo",
        "image" => "Resim",
        "wall" => "Duvar",
        "opening" => "Açıklık",
        "space" => "Mahal",
        "grid" => "Aks",
        "column" => "Kolon",
        "beam" => "Kiriş",
        "roof" => "Çatı",
        "railing" => "Korkuluk",
        "stair" => "Merdiven",
        "ramp" => "Rampa",
        "tag" => "Etiket",
        "viewmark" => "Görünüm işareti",
        "slab" => "Döşeme",
        "nurbs" => "NURBS eğrisi",
        "raster" => "Raster",
        "pointcloud" => "Nokta bulutu",
        "record" => "Kayıt",
        _ => "Nesne",
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use iced::{Point, Rectangle, Size};
    use kentos_commands::DocumentSnapshotV1;
    use kentos_domain::Slot;
    use serde_json::Value;

    use crate::app::{App, Message};
    use crate::document::Document;
    use crate::viewport::Event;

    /// The selection traces' drawing (fixtures/interaction/v1/objects.kcad),
    /// open in an area of 800 × 600 at 0.125 m per pixel around its centre.
    pub(crate) fn objects() -> App {
        let (mut app, _) = App::boot(None);
        let snapshot = DocumentSnapshotV1::from_json(include_str!(
            "../../../../fixtures/interaction/v1/objects.kcad"
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
    pub(crate) fn at(de: f32, dn: f32) -> Point {
        Point::new(400.0 + de * 8.0, 300.0 - dn * 8.0)
    }

    pub(crate) fn click(app: &mut App, de: f32, dn: f32) {
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

    /// Izgara (F7; docs/adr/0080): a session setting, on by default, its
    /// command's check and the status bar's lamp follow it.
    #[test]
    fn f7_turns_the_grid_on_and_off() {
        let mut app = objects();
        assert_eq!(app.shortcut("F7"), Some("draft.grid"));
        assert_eq!(app.checked("draft.grid"), Some(true));
        let _ = app.run("draft.grid");
        assert_eq!(app.checked("draft.grid"), Some(false));
        assert!(!app.viewport.grid_shown, "the drawing area follows");
        assert_eq!(
            app.log.last().map(|l| l.text.as_str()),
            Some("Izgara kapalı")
        );
        let _ = app.run("draft.grid");
        assert!(app.settings.bool("drafting.grid"));
    }

    #[test]
    fn f3_and_the_settings_reach_the_snap() {
        let mut app = objects();
        assert_eq!(app.shortcut("F3"), Some("draft.snap"));
        assert!(app.draft.snap);
        let _ = app.run("draft.snap");
        assert!(!app.draft.snap);
        assert_eq!(
            app.log.last().map(|l| l.text.as_str()),
            Some("Kenetleme kapalı")
        );
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

    /// Otomatik seçim turned off (docs/adr/0622 §5), perhaps by a stray click on the status bar: a notice says what is
    /// left to select and turns it back on (Geri aç). A click on another layer's object then selects nothing and a
    /// notice offers Otomatik seçimi aç, whose click selects again (the owner, 19 November 2026: once it went wrong
    /// nothing could be selected, and only a cut status line said why).
    #[test]
    fn otomatik_secim_off_says_why_and_its_notice_turns_it_on() {
        let mut app = objects();
        app.toasts.clear();
        let _ = app.update(Message::Run("edit.autoSelect"));
        assert!(!app.auto_select_on());
        let (title, body, action) = {
            let (_, toast, _) = app.toasts.iter().last().expect("a notice");
            let action = toast.action_of().map(|(label, m)| (label.to_owned(), m.clone()));
            (toast.title().to_owned(), toast.description().map(str::to_owned), action)
        };
        assert_eq!(title, "Otomatik seçim kapandı");
        assert_eq!(
            body.as_deref(),
            Some(
                "Yalnız etkin katmanın (“Çizim”) nesneleri seçilir. Bu katmanda 3 nesne var; başka katmanlardaki 4 \
                 nesne seçilemez."
            )
        );
        let (label, back) = action.expect("Geri aç");
        assert_eq!(label, "Geri aç");
        let _ = app.update(back);
        assert!(app.auto_select_on(), "Geri aç turns it on");

        let _ = app.update(Message::Run("edit.autoSelect"));
        app.toasts.clear();
        // The parcel is on Parsel, Çizim is active.
        click(&mut app, -18.0, 9.0);
        assert!(selected(&app).is_empty());
        let (title, action) = {
            let (_, toast, _) = app.toasts.iter().last().expect("a notice");
            let action = toast.action_of().map(|(label, m)| (label.to_owned(), m.clone()));
            (toast.title().to_owned(), action)
        };
        assert_eq!(title, "Başka katmandaki nesne seçilmedi");
        let (label, on) = action.expect("Otomatik seçimi aç");
        assert_eq!(label, "Otomatik seçimi aç");
        let _ = app.update(on);
        assert!(app.auto_select_on());
        click(&mut app, -18.0, 9.0);
        assert_eq!(selected(&app), [4]);
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
