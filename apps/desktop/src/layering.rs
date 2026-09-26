//! New layers and groups (the web's `layer.new` and `layer.newGroup`,
//! apps/web/src/app/commands.ts, and the two buttons of its Katmanlar panel).
//! A new layer goes last into the active layer's group and becomes the
//! active layer; a new group goes last at the top of the tree. Each is an
//! edit that is not undone: the web's layer creation is not an undo step
//! either (kentos-domain `add_layer`, fixtures/document-ops/v1/layers.json).

use iced::widget::{button, row, tooltip};
use iced::{Center, Element};
use kentos_domain::NewLayer;
use kentos_interaction::Level;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::{Tip, tip};
use kentos_ui::{label, style};

use crate::app::{App, Message};

impl App {
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
