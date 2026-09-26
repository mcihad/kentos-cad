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

use iced::widget::{button, row, tooltip};
use iced::{Center, Element};
use kentos_domain::NewLayer;
use kentos_interaction::Level;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::{Tip, tip};
use kentos_ui::{label, style};

use crate::app::{App, Message};

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
        assert_eq!(model.layers().active(), active, "new objects still go there");
        assert!(model.layers().get(&group).expect("group").expanded, "opened to show it");
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
            for name in ["tek", "cok"] {
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
                    "tek" => slot_on("bina").into_iter().collect(),
                    _ => ["parsel", "bina", "cizim"]
                        .iter()
                        .filter_map(|layer| slot_on(layer))
                        .collect(),
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
