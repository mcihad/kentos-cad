//! New layers and groups (the web's `layer.new` and `layer.newGroup`,
//! apps/web/src/app/commands.ts, and the two buttons of its Katmanlar panel).
//! A new layer goes last into the active layer's group and becomes the
//! active layer; a new group goes last at the top of the tree. Each is an
//! edit that is not undone: the web's layer creation is not an undo step
//! either (kentos-domain `add_layer`, fixtures/document-ops/v1/layers.json).
//!
//! The tree follows the drawing's selection (the owner's request, 26
//! September): the rows of the selected objects' layers show selected, their
//! groups open, and one such layer is scrolled into view. The active layer,
//! where new objects go, does not change.

use std::time::{Duration, Instant};

use iced::widget::{button, row, tooltip};
use iced::{Center, Element, Task};
use kentos_contracts::{LayerNode, LayerNodeType, LineType};
use kentos_domain::{NewLayer, Slot};
use kentos_interaction::Level;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::tree_view::RENAME;
use kentos_ui::widget::{Menu, Tip, tip};
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// Two presses on one row closer than this are a double click (KentOS UI's sash's).
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// The colours of the layer's colour menu, as on the web (`DRAW_COLORS`,
/// apps/web/src/ui/toolbar/fields.ts), after the theme's two inks.
const COLORS: [(&str, &str); 8] = [
    ("Siyah", "ink"),
    ("Kırmızı", "#E5484D"),
    ("Sarı", "#F2C94C"),
    ("Yeşil", "#5FBF77"),
    ("Camgöbeği", "#4CC3D9"),
    ("Mavi", "#4F8EF7"),
    ("Eflatun", "#C86DD7"),
    ("Gri", "#8C9AAA"),
];

/// Line types with the web's names (`LINE_TYPE_LABEL`, model/layers.ts).
const LINE_TYPES: [(LineType, &str); 4] = [
    (LineType::Continuous, "Sürekli"),
    (LineType::Dashed, "Kesikli"),
    (LineType::Dashdot, "Noktalı kesik"),
    (LineType::Dotted, "Noktalı"),
];

/// Plot line weights in mm (the web's `LINE_WEIGHTS`).
const LINE_WEIGHTS: [f64; 6] = [0.13, 0.18, 0.25, 0.35, 0.5, 0.7];

/// What the layer tree's rows and their menu ask (the web's LayersPanel).
#[derive(Debug, Clone)]
pub enum Event {
    /// Etkin katman yap: new objects go there.
    Activate(String),
    /// Yalnızca bunu göster.
    Isolate(String),
    /// Nesnelerini seç: the objects of a layer, or of every layer of a group.
    SelectObjects(String),
    Color(String, String),
    LineType(String, LineType),
    LineWeight(String, f64),
    /// Yeniden adlandır: the row's name becomes a text box.
    Rename(String),
    RenameInput(String),
    RenameDone,
    RenameCancel,
    /// Yanına yeni katman (a layer) or İçine yeni katman (a group).
    AddBeside(String),
    /// Sil: the layer, or the group with everything under it, and their objects.
    Remove(String),
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
        // The pressed row is in view already.
        self.layer_reveal = None;
        if double {
            self.activate_row(&id);
        }
    }

    pub(crate) fn layer_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Remove(id) = event {
            self.ask_remove_layer(id);
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
        let Some(doc) = &mut self.document else {
            return Task::none();
        };
        let model = &mut doc.model;
        match event {
            Event::Activate(id) => {
                model.set_active_layer(&id);
            }
            Event::Isolate(id) => model.isolate_layer(&id),
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
            Event::Color(id, color) => restyle(model, &id, "Katman rengi", |s| s.color = color),
            Event::LineType(id, line_type) => {
                restyle(model, &id, "Çizgi tipi", |s| s.line_type = line_type);
            }
            Event::LineWeight(id, weight) => {
                restyle(model, &id, "Çizgi kalınlığı", |s| {
                    s.line_weight = weight
                });
            }
            Event::RenameInput(text) => {
                if let Some((_, name)) = &mut self.renaming {
                    *name = text;
                }
            }
            Event::RenameDone => {
                if let Some((id, name)) = self.renaming.take() {
                    model.rename_layer(&id, &name);
                }
            }
            Event::RenameCancel => self.renaming = None,
            // The web makes the new layer active; it says nothing.
            Event::AddBeside(id) => {
                let name = model.layers().unique_name("Yeni katman");
                let new = model.add_layer(NewLayer::layer(name), Some(&id));
                model.set_active_layer(&new);
            }
            Event::Rename(_) | Event::Remove(_) => {}
        }
        Task::none()
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
        match doc.model.remove_layer(id) {
            Err(refusal) => self.warn(refusal.to_string()),
            Ok(gone) => {
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

    /// A row's context menu, the web's `LayersPanel.menuFor` item by item.
    pub(crate) fn layer_menu(&self, node: &LayerNode, active: bool) -> Menu<Message> {
        let id = node.id.clone();
        let event = |e: Event| Message::Layer(e);
        let is_layer = node.kind == LayerNodeType::Layer;
        let mut menu = Menu::new();
        if is_layer {
            menu = menu
                .item(
                    "Etkin katman yap",
                    (!active).then(|| event(Event::Activate(id.clone()))),
                )
                .icon(Icon::Check);
        }
        menu = menu
            .item(
                if node.visible { "Gizle" } else { "Göster" },
                Message::LayerVisible(id.clone()),
            )
            .icon(if node.visible {
                Icon::EyeOff
            } else {
                Icon::Eye
            })
            .item(
                if node.locked { "Kilidi aç" } else { "Kilitle" },
                Message::LayerLocked(id.clone()),
            )
            .icon(if node.locked {
                Icon::Unlock
            } else {
                Icon::Lock
            })
            .item("Yalnızca bunu göster", event(Event::Isolate(id.clone())))
            .item("Tüm katmanları göster", Message::Run("layer.showAll"))
            .separator()
            .item("Nesnelerini seç", event(Event::SelectObjects(id.clone())))
            .separator();
        if is_layer {
            let style = &node.style;
            // One of each group, as the web's radio items.
            let types = LINE_TYPES.iter().fold(Menu::new(), |menu, (t, name)| {
                menu.radio(
                    *name,
                    style.line_type == *t,
                    event(Event::LineType(id.clone(), *t)),
                )
            });
            let weights = LINE_WEIGHTS.iter().fold(Menu::new(), |menu, w| {
                menu.radio(
                    format!("{w:.2} mm"),
                    (style.line_weight - w).abs() < 1e-9,
                    event(Event::LineWeight(id.clone(), *w)),
                )
            });
            menu = menu
                .submenu("Renk", self.layer_colors(node))
                .submenu("Çizgi tipi", types)
                .submenu("Kalınlık", weights)
                // The style dialog is not on the desktop yet: shown, dimmed.
                .item(
                    if style.renderer.is_some() {
                        "Katman stili… (özel)"
                    } else {
                        "Katman stili…"
                    },
                    None,
                )
                .separator();
        }
        menu.item("Yeniden adlandır", event(Event::Rename(id.clone())))
            .item(
                if is_layer {
                    "Yanına yeni katman"
                } else {
                    "İçine yeni katman"
                },
                event(Event::AddBeside(id.clone())),
            )
            .icon(Icon::Layers)
            .separator()
            // Always offered: what cannot go says why (the web's).
            .item("Sil", event(Event::Remove(id)))
            .icon(crate::icons::from_web(Some("trash")))
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
        let Some(doc) = &mut self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let name = doc.model.layers().unique_name("Yeni katman");
        let active = doc.model.layers().active().to_owned();
        let id = doc
            .model
            .add_layer(NewLayer::layer(name.clone()), Some(&active));
        doc.model.set_active_layer(&id);
        self.say(
            Level::Success,
            format!("“{name}” katmanı eklendi ve etkin yapıldı."),
        );
    }

    /// `layer.newGroup`: “Yeni grup” at the top of the tree.
    pub(crate) fn new_group(&mut self) {
        let Some(doc) = &mut self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let name = doc.model.layers().unique_name("Yeni grup");
        doc.model.add_layer(NewLayer::group(name.clone()), None);
        self.say(Level::Success, format!("“{name}” grubu eklendi."));
    }

    /// The Katmanlar panel's header: the layer count and the two buttons.
    pub(crate) fn layers_actions(&self, count: usize) -> Element<'_, Message> {
        let add = |glyph: Icon, title: &'static str, id: &'static str| {
            tip(
                button(icon(glyph).size(14.0))
                    .on_press(Message::Run(id))
                    .padding([2, 4])
                    .style(style::button::ghost),
                Tip::new(title),
                tooltip::Position::Bottom,
            )
        };
        row![
            label::caption(format!("{count} katman")),
            add(Icon::Layers, "Yeni katman", "layer.new"),
            add(Icon::Folder, "Yeni grup", "layer.newGroup"),
        ]
        .spacing(4)
        .align_y(Center)
        .into()
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

/// A layer's style changed by `change`, as one undo step named `label` (the
/// web's `setLayerStyle`).
fn restyle(
    model: &mut kentos_domain::Document,
    id: &str,
    label: &str,
    change: impl FnOnce(&mut kentos_contracts::LayerStyle),
) {
    if let Some(mut style) = model.layers().get(id).map(|n| n.style.clone()) {
        change(&mut style);
        model.set_layer_style(id, style, label);
    }
}

#[cfg(test)]
mod tests {
    use crate::app::Message;
    use crate::files_testing::{app_with_drawing, last_said};

    #[test]
    fn a_new_layer_goes_next_to_the_active_one_and_becomes_active() {
        let mut app = app_with_drawing();
        let model = &app.document.as_ref().expect("open").model;
        let active = model.layers().active().to_owned();
        let group = model.layers().parent(&active).map(|g| g.id.clone());
        let revision = model.revision();

        let _ = app.update(Message::Run("layer.new"));
        let model = &app.document.as_ref().expect("open").model;
        let id = model.layers().active().to_owned();
        let node = model.layers().get(&id).expect("added");
        assert_eq!(node.name, "Yeni katman");
        assert_eq!(model.layers().parent(&id).map(|g| g.id.clone()), group);
        assert!(model.is_dirty() && !model.can_undo());
        assert_ne!(model.revision(), revision);
        assert_eq!(
            last_said(&app),
            "“Yeni katman” katmanı eklendi ve etkin yapıldı."
        );

        let _ = app.update(Message::Run("layer.new"));
        let model = &app.document.as_ref().expect("open").model;
        let second = model.layers().active();
        assert_eq!(
            model.layers().get(second).map(|n| n.name.as_str()),
            Some("Yeni katman 2")
        );
    }

    /// Katmanlar → Sil (task 12, fixtures/document-ops/v1/layer-remove.json
    /// for the document's side): the active layer and a group holding it are
    /// refused in their words; a layer with objects is asked about, then goes
    /// with them in one step that undo brings back; an empty layer goes at once.
    #[test]
    fn a_layer_goes_with_its_objects_after_the_question() {
        use super::Event;
        use crate::app::Dialog;
        let mut app = app_with_drawing();
        let _ = app.update(Message::Layer(Event::Remove("parsel".into())));
        assert_eq!(
            last_said(&app),
            "“Parsel” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin."
        );
        let _ = app.update(Message::Layer(Event::Remove("layer-g".into())));
        assert_eq!(
            last_said(&app),
            "“Kadastro” grubu etkin katmanı (“Parsel”) içeriyor; silinemez. Önce grubun dışındaki bir katmanı etkinleştirin."
        );
        let objects = app.document.as_ref().expect("open").model.count("cizim");
        assert!(objects > 0);
        let _ = app.update(Message::Layer(Event::Remove("cizim".into())));
        assert_eq!(app.dialog, Some(Dialog::RemoveLayer));
        assert_eq!(app.removing_layer.as_deref(), Some("cizim"));
        // Vazgeç leaves it.
        let _ = app.update(Message::DialogClosed);
        let layers =
            |app: &crate::app::App| app.document.as_ref().expect("open").model.layers().clone();
        assert!(layers(&app).get("cizim").is_some());
        let _ = app.update(Message::Layer(Event::Remove("cizim".into())));
        let _ = app.update(Message::DialogConfirmed);
        assert_eq!(app.dialog, None);
        assert_eq!(
            last_said(&app),
            format!("“Çizim” katmanı ve üzerindeki {objects} nesne silindi.")
        );
        let model = &mut app.document.as_mut().expect("open").model;
        assert!(model.layers().get("cizim").is_none());
        assert_eq!(model.undo().as_deref(), Some("Katman sil"));
        assert_eq!(model.count("cizim"), objects);
        let empty = model.add_layer(kentos_domain::NewLayer::layer("Boş"), None);
        let _ = app.update(Message::Layer(Event::Remove(empty.clone())));
        assert_eq!(app.dialog, None, "nothing to ask");
        assert_eq!(last_said(&app), "“Boş” katmanı silindi.");
        assert!(layers(&app).get(&empty).is_none());
    }

    #[test]
    fn the_tree_shows_the_selected_objects_layer_and_keeps_the_active_one() {
        let mut app = app_with_drawing();
        let model = &app.document.as_ref().expect("open").model;
        let active = model.layers().active().to_owned();
        // An object on another layer than the active one, inside a group.
        let (slot, layer) = model
            .entities()
            .find_map(|e| {
                let layer = &e.base().layer_id;
                (layer != &active && model.layers().parent(layer).is_some())
                    .then(|| (kentos_domain::Slot(e.base().id), layer.clone()))
            })
            .expect("an object in a group, off the active layer");
        let group = model.layers().parent(&layer).expect("its group").id.clone();
        app.document
            .as_mut()
            .expect("open")
            .model
            .set_layer_expanded(&group, false);
        let revision = app.document.as_ref().expect("open").model.revision();

        app.selection.set(vec![slot]);
        let _ = app.update(Message::Run("view.zoomIn"));
        assert!(app.layer_row_selected(&layer));
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(
            model.layers().active(),
            active,
            "new objects still go there"
        );
        assert!(
            model.layers().get(&group).expect("group").expanded,
            "opened to show it"
        );
        assert_eq!(model.revision(), revision, "not an edit");

        // A click in the tree chooses the rows, until the selection changes.
        let _ = app.update(Message::LayerSelected(active.clone()));
        assert!(app.layer_row_selected(&active) && !app.layer_row_selected(&layer));
        let _ = app.update(Message::Run("edit.deselect"));
        assert!(app.layer_row_selected(&active));
        app.selection.set(vec![slot]);
        let _ = app.update(Message::Run("view.zoomIn"));
        assert!(app.layer_row_selected(&layer) && !app.layer_row_selected(&active));
        // An empty selection gives the rows back to the layer last clicked.
        let _ = app.update(Message::Run("edit.deselect"));
        assert!(app.layer_row_selected(&active) && !app.layer_row_selected(&layer));
    }

    #[test]
    fn a_double_click_makes_a_layer_active_and_opens_or_closes_a_group() {
        let mut app = app_with_drawing();
        let press = |app: &mut crate::app::App, id: &str| {
            let _ = app.update(Message::LayerSelected(id.to_owned()));
        };
        press(&mut app, "cizim");
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(
            model.layers().active(),
            "parsel",
            "one click only chooses the row"
        );
        press(&mut app, "cizim");
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(model.layers().active(), "cizim");
        assert!(!model.is_dirty(), "not an edit");
        let open = model.layers().get("layer-g").expect("group").expanded;
        press(&mut app, "layer-g");
        press(&mut app, "layer-g");
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(
            model.layers().get("layer-g").expect("group").expanded,
            !open
        );
        // A third press starts over: one click.
        press(&mut app, "layer-g");
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(
            model.layers().get("layer-g").expect("group").expanded,
            !open
        );
    }

    #[test]
    fn the_rows_menu_does_what_the_webs_does() {
        use super::Event;
        let mut app = app_with_drawing();
        let layer = |app: &mut crate::app::App, e: Event| {
            let _ = app.update(Message::Layer(e));
        };
        layer(&mut app, Event::Activate("bina".into()));
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(model.layers().active(), "bina");
        // Yalnızca bunu göster.
        layer(&mut app, Event::Isolate("bina".into()));
        let model = &app.document.as_ref().expect("open").model;
        assert!(model.layers().is_visible("bina") && !model.layers().is_visible("parsel"));
        let _ = app.update(Message::Run("layer.showAll"));
        // Nesnelerini seç: a group's are all its layers'.
        layer(&mut app, Event::SelectObjects("layer-g".into()));
        assert_eq!(app.selection.len(), 4);
        assert_eq!(last_said(&app), "Kadastro: 4 nesne seçildi.");
        // Colour, line type and weight: one undo step each.
        layer(&mut app, Event::Color("bina".into(), "#4F8EF7".into()));
        layer(
            &mut app,
            Event::LineType("bina".into(), kentos_contracts::LineType::Dashed),
        );
        layer(&mut app, Event::LineWeight("bina".into(), 0.35));
        let model = &app.document.as_ref().expect("open").model;
        let style = &model.layers().get("bina").expect("layer").style;
        assert_eq!(style.color, "#4F8EF7");
        assert_eq!(style.line_type, kentos_contracts::LineType::Dashed);
        assert_eq!(style.line_weight, 0.35);
        let _ = app.update(Message::Run("edit.undo"));
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(
            model.layers().get("bina").expect("layer").style.line_weight,
            0.25
        );
        // Yeniden adlandır: typed, then Enter; Esc leaves the name.
        layer(&mut app, Event::Rename("bina".into()));
        layer(&mut app, Event::RenameInput("Yapı".into()));
        layer(&mut app, Event::RenameDone);
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(model.layers().get("bina").expect("layer").name, "Yapı");
        layer(&mut app, Event::Rename("bina".into()));
        layer(&mut app, Event::RenameInput("Başka".into()));
        layer(&mut app, Event::RenameCancel);
        let model = &app.document.as_ref().expect("open").model;
        assert_eq!(model.layers().get("bina").expect("layer").name, "Yapı");
        // Yanına yeni katman: beside it, made active.
        layer(&mut app, Event::AddBeside("bina".into()));
        let model = &app.document.as_ref().expect("open").model;
        let new = model.layers().active().to_owned();
        assert_eq!(model.layers().get(&new).expect("new").name, "Yeni katman");
        assert_eq!(
            model.layers().parent(&new).map(|g| g.id.clone()),
            Some("layer-g".to_owned())
        );
    }

    #[test]
    fn warnings_said_while_their_tab_is_open_are_seen() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::Messages));
        app.warn("Görülen uyarı.");
        let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::History));
        app.warn("Görülmeyen uyarı.");
        assert_eq!(app.warnings_total - app.seen_warnings, 1);
    }

    #[test]
    fn a_new_group_goes_to_the_top_of_the_tree() {
        let mut app = app_with_drawing();
        let active = app
            .document
            .as_ref()
            .expect("open")
            .model
            .layers()
            .active()
            .to_owned();
        let _ = app.update(Message::Run("layer.newGroup"));
        let model = &app.document.as_ref().expect("open").model;
        let last = model.layers().nodes().last().expect("a node");
        assert_eq!(last.name, "Yeni grup");
        assert!(last.children.is_empty());
        assert_eq!(model.layers().active(), active, "the active layer stays");
        assert_eq!(last_said(&app), "“Yeni grup” grubu eklendi.");
    }
}

/// Pictures for the owner: the layer tree following the selection, one
/// layer (its group opened) and several; `.run/shots/katman-secimi-*`.
/// `cargo test -p kentos-desktop layering::screens -- --ignored --nocapture`
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["tek", "cok", "menu", "renk", "ad", "sil"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let model = &app.document.as_ref().expect("open").model;
                let slot_on = |layer: &str| {
                    model
                        .entities()
                        .find(|e| e.base().layer_id == layer)
                        .map(|e| kentos_domain::Slot(e.base().id))
                };
                let picked: Vec<_> = match name {
                    "cok" => ["parsel", "bina", "cizim"]
                        .iter()
                        .filter_map(|layer| slot_on(layer))
                        .collect(),
                    _ => slot_on("bina").into_iter().collect(),
                };
                // The group closed first: following the selection opens it.
                app.document
                    .as_mut()
                    .expect("open")
                    .model
                    .set_layer_expanded("layer-g", false);
                app.selection.set(picked);
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                // The Bina row: the panel's third row, its swatch after the caret and indent.
                let panel = width - app.docks.size(kentos_ui::widget::docking::Side::Right);
                let bina = iced::Point::new(panel + 120.0, 261.0);
                let swatch = iced::Point::new(panel + 21.0, 261.0);
                use kentos_ui::snapshot::Input;
                match name {
                    "menu" => {
                        snapshot.input(&mut app, App::view, &mut update, Input::RightClick(bina))
                    }
                    "renk" => {
                        snapshot.input(&mut app, App::view, &mut update, Input::Click(swatch))
                    }
                    "ad" => {
                        let _ = app.update(Message::Layer(Event::Rename("bina".into())));
                        let _ = app
                            .update(Message::Layer(Event::RenameInput("Bina ve yapılar".into())));
                    }
                    "sil" => {
                        let _ = app.update(Message::Layer(Event::Remove("cizim".into())));
                    }
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("katman-secimi-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
