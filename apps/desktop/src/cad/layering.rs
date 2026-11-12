//! New layers and groups (the web's `layer.new` and `layer.newGroup`,
//! apps/web/src/app/commands.ts, and the two buttons of its Katmanlar panel).
//! A new layer goes last into the active layer's group and becomes the
//! active layer; a new group goes last at the top of the tree. Each is one
//! undo step, “Katman ekle” or “Grup ekle”: undo takes it away and makes the
//! layer before active again (kentos-domain `add_layer`,
//! fixtures/document-ops/v1/layer-add.json, docs/adr/0076).
//!
//! In a cloud database project this account may not edit (a project
//! Editor: objects, not the tree), a new, renamed or removed layer could not
//! be saved, and an object drawn on a new one would never reach the server:
//! those changes are refused where they start, in the web's words
//! ([`TREE_LOCKED`], the web's b19ed6f, docs/adr/0078). The eye, the lock,
//! the fold, the active layer and the style stay the user's own.
//!
//! The tree follows the drawing's selection (the owner's request, 26
//! September): the rows of the selected objects' layers show selected, their
//! groups open, and one such layer is scrolled into view. The active layer,
//! where new objects go, does not change.

use std::time::{Duration, Instant};

use iced::widget::{button, container, row, tooltip};
use iced::{Center, Element, Task};
use kentos_commands::{
    LayerEditOperation, LayerNode, LayerNodeType, LayerSetOperation, LayersEdit, LayersSet,
    LineType,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::layers_edit;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::style;
use kentos_ui::widget::tree_view::RENAME;
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};

use crate::app::{App, Message};
use crate::module_surfaces::LayerEntry;

/// Why the layer tree may not change here (the web's `TREE_LOCKED`).
pub const TREE_LOCKED: &str = "Bu projede katman ağacını değiştirme yetkiniz yok (project.edit); proje sahibinden ya da yöneticisinden isteyin.";

/// Two presses on one row closer than this are a double click (KentOS UI's sash's).
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

// The colours of the layer's colour menu after the theme's two inks, the line
// types and weights: the web's `DRAW_COLORS`, `LINE_TYPE_LABEL` and
// `LINE_WEIGHTS`, kept once for the desktop (ribbon_panels.rs).
use crate::ribbon_panels::{DRAW_COLORS as COLORS, LINE_TYPES, LINE_WEIGHTS, weight_text};

/// What the layer tree's rows and their menu ask (the web's LayersPanel).
#[derive(Debug, Clone)]
pub enum Event {
    /// Etkin katman yap: new objects go there.
    Activate(String),
    /// Yalnızca bunu göster.
    Isolate(String),
    /// Nesnelerini seç: the objects of a layer, or of every layer of a group.
    SelectObjects(String),
    /// Katmana yakınlaştır, Gruba yakınlaştır: the view on those objects (docs/adr/0141).
    ZoomTo(String),
    Color(String, String),
    LineType(String, LineType),
    /// A project line type with a pattern by its name (docs/adr/0313 §7).
    LinePattern(String, String),
    LineWeight(String, f64),
    /// Yeniden adlandır: the row's name becomes a text box.
    Rename(String),
    RenameInput(String),
    RenameDone,
    RenameCancel,
    /// Yanına yeni katman (a layer) or İçine yeni katman (a group).
    AddBeside(String),
    /// Kopyasını oluştur (docs/adr/0177 §3): the layer and its objects copied.
    Duplicate(String),
    /// Başka katmanlarla birleştir…: Katmanları birleştir's window, this layer the target.
    MergeInto(String),
    /// Alanlar… (docs/adr/0199 §3): the layer's fields.
    Fields(String),
    /// Formlar… (forms/designer.rs, docs/adr/0401): the layer's form, or a new one.
    Forms(String),
    /// Etiketler… (labelling/, docs/adr/0212 §4): how its objects are labelled.
    Labels(String),
    /// Süzgeç…, Seçimden süzgeç and Süzgeci kaldır on this layer (layer_filters.rs, docs/adr/0211 §4).
    Filter(String),
    FilterFromSelection(String),
    FilterClear(String),
    /// Bilgi kartı ve eylemler… on this layer (layer_info.rs, docs/adr/0225 §5).
    Info(String),
    /// Veri kümesi bilgisi… on this layer (dataset_window.rs, docs/adr/0633).
    Dataset(String),
    /// Görünüşü SLD olarak kaydet… and SLD'den görünüş al… (layer_sld.rs, docs/adr/0516): the path asked, then the
    /// path given (none: the dialog was cancelled).
    SldExport(String),
    SldImport(String),
    SldExportTo(String, Option<std::path::PathBuf>),
    SldImportFrom(String, Option<std::path::PathBuf>),
    /// Görünüşü QML olarak kaydet… and QML'den görünüş al… (layer_qml.rs, docs/adr/0555), as the SLD's.
    QmlExport(String),
    QmlImport(String),
    QmlExportTo(String, Option<std::path::PathBuf>),
    QmlImportFrom(String, Option<std::path::PathBuf>),
    /// Görünüşü MapLibre stili olarak kaydet… and MapLibre stilinden görünüş al… (layer_maplibre.rs,
    /// docs/adr/0560); with no layer the whole map's (Dosya alışverişi).
    MaplibreExport(String),
    MaplibreImport(String),
    MaplibreExportTo(Option<String>, Option<std::path::PathBuf>),
    MaplibreImportFrom(Option<String>, Option<std::path::PathBuf>),
    /// Stil yükle… and Stili kaydet… (layer_style_file.rs, docs/adr/0635 §9): one file for the three formats, its
    /// extension choosing the command; the path asked, then the path given.
    StyleLoad(String),
    StyleSave(String),
    StyleLoadFrom(String, Option<std::path::PathBuf>),
    StyleSaveTo(String, Option<std::path::PathBuf>),
    /// Sil: the layer, or the group with everything under it, and their objects.
    Remove(String),
    /// A layer's own snapping, a group's on all its layers (the magnet, Kenet ▸;
    /// layer_snap.rs, docs/adr/0163 §4); none for the general kinds.
    Snap(String, Option<kentos_commands::LayerSnap>),
    /// A row dragged onto another (layer_order.rs): the rows' places as the tree shows them, and where.
    Move(usize, usize, kentos_ui::widget::tree_view::Place),
    /// Sıra ▸ (layer_order.rs).
    Order(String, crate::layer_order::Step),
    /// Katman türü ▸ (layer_types.rs, docs/adr/0622 §5): what the layer holds.
    Geometry(String, kentos_commands::LayerGeometryChoice),
}

impl App {
    /// Once the drawing's selection changes, the tree shows its objects'
    /// layers selected, and the groups around them open so their rows can be
    /// seen; with one layer the tree also scrolls to it (view.rs). An empty
    /// selection gives the rows back to the layer last clicked. Opening a
    /// group is not an edit (kentos-domain `set_layer_expanded`).
    pub(crate) fn follow_selection_layers(&mut self) {
        let version = self.selection.version();
        if version == self.followed_selection {
            return;
        }
        self.followed_selection = version;
        self.selection_layers.clear();
        let Some(doc) = &mut self.document else {
            self.layers_follow = false;
            return;
        };
        let mut seen = std::collections::HashSet::new();
        for slot in self.selection.ids() {
            if let Some(entity) = doc.model.get(*slot) {
                let layer = &entity.base().layer_id;
                if seen.insert(layer.as_str()) {
                    self.selection_layers.push(layer.clone());
                }
            }
        }
        self.layers_follow = !self.selection_layers.is_empty();
        for layer in &self.selection_layers {
            let mut id = layer.clone();
            while let Some(group) = doc.model.layers().parent(&id).map(|g| g.id.clone()) {
                if doc.model.layers().get(&group).is_some_and(|g| !g.expanded) {
                    doc.model.set_layer_expanded(&group, true);
                }
                id = group;
            }
        }
    }

    /// Why the layer tree may not change in the open drawing: a cloud
    /// database project where this account lacks `project.edit` (the web's
    /// `treeLocked`). A file project and a local drawing save the tree with
    /// the file.
    pub(crate) fn tree_locked(&self) -> Option<&'static str> {
        let doc = self.document.as_ref()?;
        let source = doc.cloud_source()?;
        let (_, may_edit) = crate::cloud::access_of(&source.info);
        (doc.is_database() && !may_edit).then_some(TREE_LOCKED)
    }

    /// A row pressed: it is chosen (the web's focused row), and the tree has
    /// the keyboard (layer_tree.rs). A second press soon after is a double
    /// click: a layer becomes the active one, a group opens or closes (the
    /// web's `onActivate`).
    pub(crate) fn layer_pressed(&mut self, id: String) {
        let now = Instant::now();
        let double = self
            .last_layer_press
            .as_ref()
            .is_some_and(|(last, at)| *last == id && now.duration_since(*at) <= DOUBLE_CLICK);
        self.last_layer_press = (!double).then(|| (id.clone(), now));
        self.selected_layer = Some(id.clone());
        // A click in the tree chooses its rows until the selection changes again.
        self.layers_follow = false;
        self.layers_keyboard = true;
        self.vertices.keyboard = false;
        self.blocks_panel.keyboard = false;
        self.arch.templates_panel.keyboard = false;
        // The pressed row is in view already.
        self.layer_reveal = None;
        if double {
            self.activate_row(&id);
        }
    }

    pub(crate) fn layer_event(&mut self, event: Event) -> Task<Message> {
        // The tree's own changes are refused where it may not change (b19ed6f).
        if matches!(
            event,
            Event::Remove(_) | Event::Rename(_) | Event::AddBeside(_)
        ) && let Some(why) = self.tree_locked()
        {
            self.warn(why);
            return Task::none();
        }
        if let Event::Remove(id) = event {
            self.ask_remove_layer(id);
            return Task::none();
        }
        if let Event::Duplicate(id) = event {
            self.duplicate_layer(Some(id));
            return Task::none();
        }
        if let Event::Move(from, to, place) = event {
            self.layer_dropped(from, to, place);
            return Task::none();
        }
        if let Event::Geometry(id, choice) = event {
            self.set_layer_geometry(id, choice);
            return Task::none();
        }
        if let Event::Order(id, step) = event {
            self.layer_step(id, step);
            return Task::none();
        }
        if let Event::MergeInto(id) = event {
            self.open_layer_merge(Some(id));
            return Task::none();
        }
        if let Event::Forms(id) = event {
            self.open_forms_for_layer(&id);
            return Task::none();
        }
        if let Event::Fields(id) = event {
            if let Some(why) = self.tree_locked() {
                self.warn(why);
                return Task::none();
            }
            return self.open_layer_fields(&id);
        }
        match event {
            Event::Labels(id) => {
                self.open_labelling(Some(id));
                return Task::none();
            }
            Event::Filter(id) => return self.open_layer_filter(Some(id)),
            Event::FilterFromSelection(id) => {
                self.filter_from_selection(Some(id));
                return Task::none();
            }
            Event::FilterClear(id) => {
                self.clear_layer_filter(Some(id));
                return Task::none();
            }
            Event::Info(id) => return self.open_layer_info(Some(id)),
            Event::Dataset(id) => return self.open_dataset_info(Some(id)),
            Event::SldExport(id) => return self.sld_export_ask(id),
            Event::SldImport(id) => return self.sld_import_ask(id),
            Event::SldExportTo(id, path) => {
                self.sld_export_to(id, path);
                return Task::none();
            }
            Event::SldImportFrom(id, path) => {
                self.sld_import_from(id, path);
                return Task::none();
            }
            Event::QmlExport(id) => return self.qml_export_ask(id),
            Event::QmlImport(id) => return self.qml_import_ask(id),
            Event::QmlExportTo(id, path) => {
                self.qml_export_to(id, path);
                return Task::none();
            }
            Event::QmlImportFrom(id, path) => {
                self.qml_import_from(id, path);
                return Task::none();
            }
            Event::MaplibreExport(id) => return self.maplibre_export_ask(Some(id)),
            Event::MaplibreImport(id) => return self.maplibre_import_ask(Some(id)),
            Event::MaplibreExportTo(id, path) => {
                self.maplibre_export_to(id, path);
                return Task::none();
            }
            Event::MaplibreImportFrom(id, path) => {
                self.maplibre_import_from(id, path);
                return Task::none();
            }
            Event::StyleLoad(id) => return self.style_load_ask(id),
            Event::StyleSave(id) => return self.style_save_ask(id),
            Event::StyleLoadFrom(id, path) => {
                self.style_load_from(id, path);
                return Task::none();
            }
            Event::StyleSaveTo(id, path) => {
                self.style_save_to(id, path);
                return Task::none();
            }
            _ => {}
        }
        if let Event::ZoomTo(id) = &event {
            self.zoom_to_layer(id);
            return Task::none();
        }
        if let Event::Rename(id) = &event {
            let name = self
                .document
                .as_ref()
                .and_then(|d| d.model.layers().get(id))
                .map(|n| n.name.clone());
            if let Some(name) = name {
                self.renaming = Some((id.clone(), name));
                return iced::widget::operation::focus(RENAME);
            }
            return Task::none();
        }
        // The eye, the lock, the name and the active layer are `cad.layers.set`'s,
        // a style `cad.layers.edit`'s (docs/adr/0248 §6).
        match event {
            Event::Activate(id) => {
                self.set_layers(LayersSet::new(LayerSetOperation::Active, Some(id)));
                return Task::none();
            }
            Event::Isolate(id) => {
                let mut input = LayersSet::new(LayerSetOperation::Isolate, None);
                input.layers = Some(vec![id]);
                self.set_layers(input);
                return Task::none();
            }
            Event::Color(id, color) => {
                let mut input = LayersEdit::new(LayerEditOperation::Style);
                input.layer = Some(id);
                input.color = Some(color);
                self.edit_layers(input);
                return Task::none();
            }
            Event::LineType(id, line_type) => {
                let mut input = LayersEdit::new(LayerEditOperation::Style);
                input.layer = Some(id);
                input.line_type = Some(line_type);
                self.edit_layers(input);
                return Task::none();
            }
            Event::LinePattern(id, name) => {
                let mut input = LayersEdit::new(LayerEditOperation::Style);
                input.layer = Some(id);
                input.line_pattern = Some(Some(name));
                self.edit_layers(input);
                return Task::none();
            }
            Event::LineWeight(id, weight) => {
                let mut input = LayersEdit::new(LayerEditOperation::Style);
                input.layer = Some(id);
                input.line_weight = Some(weight);
                self.edit_layers(input);
                return Task::none();
            }
            Event::RenameDone => {
                if let Some((id, name)) = self.renaming.take() {
                    // An empty name leaves the name as it was (the shared document's rule).
                    if !name.trim().is_empty() {
                        let mut input = LayersSet::new(LayerSetOperation::Rename, Some(id));
                        input.name = Some(name);
                        self.set_layers(input);
                    }
                }
                return Task::none();
            }
            // As Yeni katman: one step, the new layer made active (the web's since bdaed77).
            Event::AddBeside(id) => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let name = doc.model.layers().unique_name(layers_edit::NEW_LAYER);
                let mut input = LayersEdit::new(LayerEditOperation::Add);
                input.name = Some(name.clone());
                input.parent = Some(id);
                input.activate = Some(true);
                if self.edit_layers(input).is_some() {
                    self.say(
                        Level::Success,
                        format!("“{name}” katmanı eklendi ve etkin yapıldı."),
                    );
                }
                return Task::none();
            }
            _ => {}
        }
        let Some(doc) = &mut self.document else {
            return Task::none();
        };
        let model = &mut doc.model;
        match event {
            Event::Activate(_)
            | Event::Isolate(_)
            | Event::Color(..)
            | Event::LineType(..)
            | Event::LinePattern(..)
            | Event::LineWeight(..)
            | Event::RenameDone
            | Event::AddBeside(_) => {}
            // Handled above, before the drawing is borrowed.
            Event::Duplicate(_)
            | Event::Geometry(..)
            | Event::Move(..)
            | Event::Order(..)
            | Event::MergeInto(_)
            | Event::Fields(_)
            | Event::Forms(_)
            | Event::Labels(_)
            | Event::Filter(_)
            | Event::FilterFromSelection(_)
            | Event::FilterClear(_)
            | Event::Info(_)
            | Event::Dataset(_)
            | Event::SldExport(_)
            | Event::SldImport(_)
            | Event::SldExportTo(..)
            | Event::SldImportFrom(..)
            | Event::QmlExport(_)
            | Event::QmlImport(_)
            | Event::QmlExportTo(..)
            | Event::QmlImportFrom(..)
            | Event::MaplibreExport(_)
            | Event::MaplibreImport(_)
            | Event::MaplibreExportTo(..)
            | Event::MaplibreImportFrom(..)
            | Event::StyleLoad(_)
            | Event::StyleSave(_)
            | Event::StyleLoadFrom(..)
            | Event::StyleSaveTo(..) => {}
            Event::Snap(id, snap) => model.set_layer_snap(&id, snap),
            Event::SelectObjects(id) => {
                let Some(node) = model.layers().get(&id) else {
                    return Task::none();
                };
                let name = node.name.clone();
                let mut layers = Vec::new();
                leaves(node, &mut layers);
                let slots: Vec<Slot> = model
                    .entities()
                    .filter(|e| layers.contains(&e.base().layer_id.as_str()))
                    .map(|e| Slot(e.base().id))
                    .collect();
                let n = slots.len();
                self.selection.set(slots);
                self.output(format!("{name}: {n} nesne seçildi."));
            }
            Event::RenameInput(text) => {
                if let Some((_, name)) = &mut self.renaming {
                    *name = text;
                }
            }
            Event::RenameCancel => self.renaming = None,
            Event::Rename(_) | Event::Remove(_) | Event::ZoomTo(_) => {}
        }
        Task::none()
    }

    /// Katmana yakınlaştır, Gruba yakınlaştır: the view on the box of the
    /// layer's objects, or of those of every layer below a group; it goes into
    /// the view history like any navigation (docs/adr/0141). A layer with no
    /// objects has nothing to show: the menu offers it dimmed.
    pub(crate) fn zoom_to_layer(&mut self, id: &str) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(node) = doc.model.layers().get(id) else {
            return;
        };
        let mut layers = Vec::new();
        leaves(node, &mut layers);
        let ids: Vec<f64> = doc
            .model
            .entities()
            .filter(|e| layers.contains(&e.base().layer_id.as_str()))
            .map(|e| f64::from(e.base().id))
            .collect();
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
    }

    /// Sil (the web's `LayersPanel.remove`): what the drawing refuses (the
    /// last or the active layer, a lock) is said in its words; a layer or
    /// group with objects on it is asked about first (`Dialog::RemoveLayer`).
    fn ask_remove_layer(&mut self, id: String) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        if let Some(refused) = model.layer_removal_refused(&id) {
            self.warn(refused);
            return;
        }
        if removal_counts(model, &id).1 > 0 {
            self.removing_layer = Some(id);
            self.dialog = Some(crate::app::Dialog::RemoveLayer);
        } else {
            self.remove_layer(&id);
        }
    }

    /// The question before a layer or group goes with its objects (the web's `askRemove`).
    pub(crate) fn remove_layer_question(&self) -> Element<'_, Message> {
        let id = self.removing_layer.as_deref().unwrap_or_default();
        let (title, message) = match self.document.as_ref().and_then(|doc| {
            let node = doc.model.layers().get(id)?;
            Some((node, removal_counts(&doc.model, id)))
        }) {
            Some((node, (layers, objects))) if node.kind == LayerNodeType::Group => (
                "Grubu sil",
                format!(
                    "“{}” grubu, içindeki {layers} katman ve {objects} nesneyle birlikte silinsin mi?",
                    node.name
                ),
            ),
            Some((node, (_, objects))) => (
                "Katmanı sil",
                format!(
                    "“{}” katmanı üzerindeki {objects} nesneyle birlikte silinsin mi?",
                    node.name
                ),
            ),
            None => ("Katmanı sil", String::new()),
        };
        kentos_ui::widget::overlay::modal(
            kentos_ui::widget::Confirm::new(title, Message::DialogConfirmed, Message::DialogClosed)
                .message(message)
                .detail("Geri al (Ctrl+Z) katmanı nesneleriyle geri getirir.")
                .confirm("Sil")
                .destructive(),
            Message::DialogClosed,
        )
    }

    /// Removes the layer or group with its objects as one undo step and says
    /// what went. Answered a while after the question: the drawing may have
    /// changed meanwhile, so a node that is gone is left and a refusal is
    /// asked again (by the document).
    pub(crate) fn remove_layer(&mut self, id: &str) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let Some(node) = doc.model.layers().get(id) else {
            return;
        };
        let name = node.name.clone();
        let group = node.kind == LayerNodeType::Group;
        let layers = doc.model.layers().leaves_of(id).len();
        let mut input = LayersEdit::new(LayerEditOperation::Remove);
        input.layer = Some(id.to_owned());
        if let Some(done) = self.edit_layers(input) {
            let gone = done.objects as usize;
            {
                if self.selected_layer.as_deref() == Some(id) {
                    self.selected_layer = None;
                }
                let said = match (group, gone, layers) {
                    (false, 0, _) => format!("“{name}” katmanı silindi."),
                    (false, n, _) => format!("“{name}” katmanı ve üzerindeki {n} nesne silindi."),
                    (true, 0, 0) => format!("“{name}” grubu silindi."),
                    (true, 0, k) => format!("“{name}” grubu ve içindeki {k} katman silindi."),
                    (true, n, k) => {
                        format!("“{name}” grubu, içindeki {k} katman ve {n} nesne silindi.")
                    }
                };
                self.say(Level::Success, said);
            }
        }
    }

    /// Whether the node is the active layer or a group holding it: never frozen.
    fn holds_active(&self, node: &LayerNode) -> bool {
        self.document.as_ref().is_some_and(|d| {
            let tree = d.model.layers();
            tree.leaves_of(&node.id)
                .iter()
                .any(|l| l.id == tree.active())
        })
    }

    /// Whether a selected object is on the layer (Seçimden süzgeç's item).
    fn selected_on_layer(&self, layer: &str) -> bool {
        self.document.as_ref().is_some_and(|doc| {
            self.selection.ids().iter().any(|s| {
                doc.model
                    .get(*s)
                    .is_some_and(|e| e.base().layer_id == layer)
            })
        })
    }

    /// A node's count as the tree shows it (docs/adr/0211 §4): the objects that show and all of them, and
    /// whether a filter leaves some out (a group: one of its layers'); a group sums its layers.
    pub(crate) fn layer_counted(
        &self,
        doc: &crate::document::Document,
        node: &LayerNode,
    ) -> (usize, usize, bool) {
        match node.kind {
            LayerNodeType::Layer => {
                let total = doc.count(&node.id);
                match node
                    .filter
                    .as_ref()
                    .filter(|_| node.service.is_none())
                    .and_then(|_| self.spatial.filter_counts(&node.id))
                {
                    Some((passed, _)) => (passed, total, true),
                    None => (total, total, false),
                }
            }
            LayerNodeType::Group => node.children.iter().fold((0, 0, false), |(s, t, f), c| {
                let (cs, ct, cf) = self.layer_counted(doc, c);
                (s + cs, t + ct, f || cf)
            }),
        }
    }

    /// The tree's count column: its usual width, or the widest “geçen / bütün” a filter makes (docs/adr/0211 §4).
    pub(crate) fn layer_count_width(&self, doc: &crate::document::Document) -> iced::Length {
        // The badges a row puts before its count (funnel, snowflake, clock, link; `layer_node`), the most one row
        // has, and whether a count reads “geçen / bütün”: a bound layer's “50000” was cut at the edge once.
        fn walk(
            nodes: &[LayerNode],
            linked: &dyn Fn(&str) -> bool,
            badges: &mut usize,
            filtered: &mut bool,
        ) {
            for n in nodes {
                let layer = n.kind == LayerNodeType::Layer;
                let funnel = layer && n.filter.is_some() && n.service.is_none();
                *filtered |= funnel;
                let here = usize::from(funnel)
                    + usize::from(n.frozen == Some(true))
                    + usize::from(layer && n.time.is_some())
                    + usize::from(linked(&n.id));
                *badges = (*badges).max(here);
                walk(&n.children, linked, badges, filtered);
            }
        }
        let sources = &doc.model.settings().sources;
        let linked = |id: &str| sources.iter().any(|s| s.layer == id);
        let (mut badges, mut filtered) = (0, false);
        walk(
            doc.model.layers().nodes(),
            &linked,
            &mut badges,
            &mut filtered,
        );
        // The widest count the drawing's objects can make.
        let digits = doc.model.len().to_string().len();
        let chars = if filtered { 2 * digits + 3 } else { digits };
        iced::Length::Fixed(
            kentos_ui::theme::typography::scaled(4.0 + 16.0 * badges as f32 + 6.4 * chars as f32)
                .max(44.0),
        )
    }

    /// How many objects a layer, or every layer below a group, holds.
    fn objects_below(&self, node: &LayerNode) -> usize {
        self.document
            .as_ref()
            .map_or(0, |doc| doc.count_below(node))
    }

    /// The layer's colour menu, from its swatch (the web's `colorItems`): the ink colours,
    /// then the colours, each with its sample, the layer's chosen.
    pub(crate) fn layer_colors(&self, node: &LayerNode) -> Menu<Message> {
        let id = node.id.clone();
        let current = node.style.color.clone();
        COLORS.iter().fold(
            Menu::new()
                .radio(
                    "Ana mürekkep",
                    current == "fg",
                    Message::Layer(Event::Color(id.clone(), "fg".into())),
                )
                .swatch(self.drawing_color("fg"))
                .radio(
                    "İkincil mürekkep",
                    current == "fg-dim",
                    Message::Layer(Event::Color(id.clone(), "fg-dim".into())),
                )
                .swatch(self.drawing_color("fg-dim"))
                .separator(),
            |menu, (name, value)| {
                menu.radio(
                    *name,
                    current.eq_ignore_ascii_case(value),
                    Message::Layer(Event::Color(id.clone(), (*value).into())),
                )
                .swatch(self.drawing_color(value))
            },
        )
    }

    /// Whether the tree shows this row selected: the selection's layers
    /// after a selection, the row last clicked after a click.
    pub(crate) fn layer_row_selected(&self, id: &str) -> bool {
        if self.layers_follow {
            self.selection_layers.iter().any(|layer| layer == id)
        } else {
            self.selected_layer.as_deref() == Some(id)
        }
    }

    /// `layer.new`: “Yeni katman” (“Yeni katman 2” when taken) next to the
    /// active layer, made active.
    pub(crate) fn new_layer(&mut self) {
        if let Some(why) = self.tree_locked() {
            self.warn(why);
            return;
        }
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let name = doc.model.layers().unique_name(layers_edit::NEW_LAYER);
        let mut input = LayersEdit::new(LayerEditOperation::Add);
        input.name = Some(name.clone());
        input.parent = Some(doc.model.layers().active().to_owned());
        input.activate = Some(true);
        if self.edit_layers(input).is_some() {
            self.say(
                Level::Success,
                format!("“{name}” katmanı eklendi ve etkin yapıldı."),
            );
        }
    }

    /// `layer.newGroup`: “Yeni grup” at the top of the tree.
    pub(crate) fn new_group(&mut self) {
        if let Some(why) = self.tree_locked() {
            self.warn(why);
            return;
        }
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let name = doc.model.layers().unique_name(layers_edit::NEW_GROUP);
        let mut input = LayersEdit::new(LayerEditOperation::Add);
        input.kind = Some(LayerNodeType::Group);
        input.name = Some(name.clone());
        if self.edit_layers(input).is_some() {
            self.say(Level::Success, format!("“{name}” grubu eklendi."));
        }
    }

    /// The Katmanlar panel's header: the layer count and the two buttons.
    pub(crate) fn layers_actions(&self, count: usize) -> Element<'_, Message> {
        // Off, with the reason in their tips, where the tree may not change.
        let locked = self.tree_locked();
        let add = |glyph: Icon, title: &'static str, id: &'static str| {
            let tip_of = match locked {
                Some(why) => Tip::new(title).body(why),
                None => Tip::new(title),
            };
            tip(
                button(icon(glyph).size(14.0))
                    .on_press_maybe(locked.is_none().then_some(Message::Run(id)))
                    .padding(kentos_ui::theme::metrics::icon_small(14.0))
                    .style(style::button::ghost),
                tip_of,
                tooltip::Position::Bottom,
            )
        };
        // A CBS layer holds one kind of object (layer_types.rs, docs/adr/0622 §5): Yeni katman asks which.
        let gis = self.document.as_ref().is_some_and(|d| {
            d.model.settings().project_type() == Some(kentos_commands::Workspace::Gis)
        });
        let new_layer: Element<'_, Message> = if gis && locked.is_none() {
            let menu = MenuButton::new(
                container(icon(crate::icons::from_web(Some("layerAdd"))).size(14.0))
                    .padding([2, 4]),
                || {
                    crate::layer_types::NEW_TYPED
                        .iter()
                        .fold(Menu::new().header("Yeni katman"), |m, (id, g)| {
                            m.item(format!("{} katmanı", g.label()), Message::Run(id))
                                .icon(crate::layer_types::glyph(Some(*g)))
                        })
                        .separator()
                        .item("Karışık katman", Message::Run("layer.new"))
                        .icon(crate::layer_types::glyph(None))
                },
            );
            tip(
                menu,
                Tip::new("Yeni katman").body(
                    "Nokta, çizgi, alan ya da açıklama katmanı: türlü katman yalnız kendi türünden nesne alır. Karışık katman her türü alır.",
                ),
                tooltip::Position::Bottom,
            )
        } else {
            add(
                crate::icons::from_web(Some("layerAdd")),
                "Yeni katman",
                "layer.new",
            )
        };
        // The count is the ⋯ menu's heading: the header keeps its room for the tab's title (the web's meta gives way the same).
        row![
            // The commands' own icons, as the web's panel draws them.
            new_layer,
            add(
                crate::icons::from_web(Some("folderAdd")),
                "Yeni grup",
                "layer.newGroup"
            ),
            self.states_button(),
            self.more_button(count),
        ]
        .spacing(4)
        .align_y(Center)
        .into()
    }

    /// Katman durumları ▾ (docs/adr/0177 §4): the project's states, the one
    /// the layers are in now marked, each applied by a click; Yeni durum
    /// kaydet saves the layers at once as “Durum n”; Katman durumları… opens
    /// the window (the web's `statesMenu`).
    fn states_button(&self) -> Element<'_, Message> {
        let states: Vec<(String, String, bool, String)> = self
            .layer_states()
            .iter()
            .map(|s| {
                let parts = kentos_domain::layer_states::parts_of(s);
                (
                    s.id.clone(),
                    s.name.clone(),
                    self.state_matches(s),
                    crate::layer_states::parts_text(parts),
                )
            })
            .collect();
        let locked = self.states_locked().is_some();
        let menu = MenuButton::new(
            container(icon(crate::icons::from_web(Some("layerStates"))).size(14.0)).padding([2, 4]),
            move || {
                let mut m = Menu::new();
                if states.is_empty() {
                    m = m.item("Kayıtlı durum yok", None);
                }
                for (id, name, now, parts) in &states {
                    m = m
                        .radio(
                            name.clone(),
                            *now,
                            Message::LayerStates(crate::layer_states::Event::Apply(id.clone())),
                        )
                        .hint(parts.clone());
                }
                m.separator()
                    .item(
                        "Yeni durum kaydet",
                        (!locked).then_some(Message::Run("layer.stateSave")),
                    )
                    .icon(crate::icons::from_web(Some("layerStateSave")))
                    .item("Katman durumları…", Message::Run("layer.states"))
                    .icon(crate::icons::from_web(Some("layerStates")))
            },
        );
        tip(
            menu,
            Tip::new("Katman durumları").body(
                "Kayıtlı durumlar (işaretli olan şimdiki hâl), yeni durum kaydetme ve Katman durumları penceresi.",
            ),
            tooltip::Position::Bottom,
        )
    }

    /// The panel's ⋯ (docs/adr/0177 §7): the actions on the whole tree, under
    /// the tree's layer count.
    fn more_button(&self, count: usize) -> Element<'_, Message> {
        let locked = self.tree_locked().is_some();
        let menu = MenuButton::new(
            container(icon(crate::icons::from_web(Some("more"))).size(14.0)).padding([2, 4]),
            move || {
                Menu::new()
                    .header(format!("{count} katman"))
                    .item(
                        "Katmanları birleştir…",
                        (!locked).then_some(Message::Run("layer.merge")),
                    )
                    .icon(crate::icons::from_web(Some("layerMerge")))
                    .item(
                        "Kullanılmayanları temizle…",
                        (!locked).then_some(Message::Run("layer.purge")),
                    )
                    .icon(crate::icons::from_web(Some("layerPurge")))
                    .item("Katman listesi…", Message::Run("layer.list"))
                    .icon(crate::icons::from_web(Some("layerList")))
                    .separator()
                    .item(
                        "Katmanları çevir…",
                        (!locked).then_some(Message::Run("layer.translate")),
                    )
                    .icon(crate::icons::from_web(Some("layersTranslate")))
                    .item("Bütün katmanları çöz", Message::Run("layer.thawAll"))
                    .icon(crate::icons::from_web(Some("layersThawAll")))
            },
        );
        tip(
            menu,
            Tip::new("Katman işlemleri")
                .body("Katmanları birleştir, Kullanılmayanları temizle, Katman listesi, Katmanları çevir ve Bütün katmanları çöz."),
            tooltip::Position::Bottom,
        )
    }
}

/// How many layers a removal takes and how many objects are on them.
fn removal_counts(model: &kentos_domain::Document, id: &str) -> (usize, usize) {
    let layers = model.layers().leaves_of(id);
    let objects = layers.iter().map(|l| model.count(&l.id)).sum();
    (layers.len(), objects)
}

/// The layers of a node: itself, or every layer below a group.
fn leaves<'a>(node: &'a LayerNode, out: &mut Vec<&'a str>) {
    match node.kind {
        LayerNodeType::Layer => out.push(&node.id),
        LayerNodeType::Group => {
            for child in &node.children {
                leaves(child, out);
            }
        }
    }
}

#[cfg(test)]
#[path = "layering/tests.rs"]
mod tests;

#[path = "layering/menus.rs"]
mod menus;

#[cfg(test)]
#[path = "layering/screens.rs"]
mod screens;
