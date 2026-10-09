//! Hazır altlıklar (docs/adr/0208 §1, §14): a ready basemap goes to the
//! bottom of the layer tree in place of the ready basemap there (as ArcGIS's
//! Basemap gallery does), else below everything, by `cad.layers.service` as
//! one undo step; Altlığı kaldır takes the bottom basemap away. A basemap
//! that needs a key this device does not have opens Bağlantılar on it. The
//! web's are `apps/web/src/app/services.ts`.

use iced::Task;
use kentos_contracts::{
    CommandResult, LayerNode, LayerNodeType, LayerServiceOperation, LayersService,
};
use kentos_native_application::{ExecutionContext, layers_service};
use kentos_services::presets::{self, Preset};

use crate::app::{App, Message};

/// The ready basemap at the bottom of the tree: the last top-level node, when a preset draws it.
pub fn bottom_basemap(nodes: &[LayerNode]) -> Option<&LayerNode> {
    nodes.last().filter(|n| {
        n.kind == LayerNodeType::Layer && n.service.as_ref().is_some_and(|s| s.preset.is_some())
    })
}

impl App {
    /// `basemap.<id>`: its preset as the basemap.
    pub(crate) fn basemap_command(&mut self, id: &str) -> Task<Message> {
        match presets::by_command(id) {
            Some(p) => self.add_basemap(p),
            None => self.error(format!("Hazır altlık bulunamadı: {id}")),
        }
        Task::none()
    }

    fn run_layers_service(&mut self, input: LayersService) -> bool {
        let Some(doc) = &mut self.document else {
            self.output("Açık çizim yok.");
            return false;
        };
        match layers_service::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { warnings, .. } => {
                for w in warnings {
                    self.warn(w.message);
                }
                true
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                false
            }
            _ => false,
        }
    }

    /// Shows `p` as the basemap: the bottom one changes to it, or it goes below everything.
    pub(crate) fn add_basemap(&mut self, p: &Preset) {
        let bottom = self
            .document
            .as_ref()
            .and_then(|d| bottom_basemap(d.model.layers().nodes()).map(|n| n.id.clone()));
        let input = LayersService {
            operation: if bottom.is_some() {
                LayerServiceOperation::Update
            } else {
                LayerServiceOperation::Add
            },
            layer: bottom.clone(),
            name: Some(p.name.clone()),
            parent: None,
            index: None,
            service: Some(presets::layer_of(p)),
            feed: None,
            fields: None,
            connections: p.connection.clone().map(|c| vec![c]),
            expected_revision: None,
        };
        if !self.run_layers_service(input) {
            return;
        }
        self.say(
            kentos_interaction::Level::Success,
            if bottom.is_some() {
                format!("Altlık “{}” oldu.", p.name)
            } else {
                format!("“{}” altlık olarak eklendi.", p.name)
            },
        );
        if let Some(c) = &p.connection
            && super::secrets::secrets().get(&c.origin, &c.id).is_none()
        {
            self.warn(format!(
                "“{}” bir anahtar ister: Harita › Altlık › Bağlantılar'da “{}” bağlantısının anahtarını girin.",
                p.name, c.name
            ));
            self.open_connections(Some(c.id.clone()));
        }
    }

    /// Altlığı kaldır: the bottom basemap goes, in one undo step.
    pub(crate) fn remove_basemap(&mut self) {
        let bottom = self.document.as_ref().and_then(|d| {
            bottom_basemap(d.model.layers().nodes()).map(|n| (n.id.clone(), n.name.clone()))
        });
        let Some((id, name)) = bottom else {
            self.output("Kaldırılacak hazır altlık yok.");
            return;
        };
        let input = LayersService {
            operation: LayerServiceOperation::Remove,
            layer: Some(id),
            name: None,
            parent: None,
            index: None,
            service: None,
            feed: None,
            fields: None,
            connections: None,
            expected_revision: None,
        };
        if self.run_layers_service(input) {
            self.say(
                kentos_interaction::Level::Success,
                format!("“{name}” altlığı kaldırıldı."),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::{App, Dialog, Message};
    use crate::files_testing::{app_with_drawing, last_said};

    /// The bottom basemap's name and preset, and the number of top-level nodes.
    fn bottom(app: &App) -> (Option<(String, String)>, usize) {
        let nodes = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .layers()
            .nodes();
        let named = super::bottom_basemap(nodes).map(|n| {
            let preset = n
                .service
                .as_ref()
                .and_then(|s| s.preset.clone())
                .unwrap_or_default();
            (n.name.clone(), preset)
        });
        (named, nodes.len())
    }

    fn named(name: &str, preset: &str) -> Option<(String, String)> {
        Some((name.to_owned(), preset.to_owned()))
    }

    #[test]
    fn a_basemap_goes_below_everything_and_the_next_takes_its_place_in_one_step() {
        let mut app = app_with_drawing();
        let (none, roots) = bottom(&app);
        assert_eq!(none, None);
        let _ = app.update(Message::Run("basemap.osmStandard"));
        assert_eq!(
            bottom(&app),
            (named("OSM Standart", "osm-standard"), roots + 1)
        );
        assert_eq!(last_said(&app), "“OSM Standart” altlık olarak eklendi.");
        let _ = app.update(Message::Run("basemap.osmTopo"));
        assert_eq!(
            bottom(&app),
            (named("OSM Topo", "osm-topo"), roots + 1),
            "in place of the one there was"
        );
        assert_eq!(last_said(&app), "Altlık “OSM Topo” oldu.");
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(
            bottom(&app),
            (named("OSM Standart", "osm-standard"), roots + 1)
        );
        let _ = app.update(Message::Run("basemap.remove"));
        assert_eq!(bottom(&app), (None, roots));
        assert_eq!(last_said(&app), "“OSM Standart” altlığı kaldırıldı.");
        app.remove_basemap();
        assert_eq!(last_said(&app), "Kaldırılacak hazır altlık yok.");
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(
            bottom(&app),
            (named("OSM Standart", "osm-standard"), roots + 1)
        );
    }

    #[test]
    fn a_basemap_that_needs_a_key_opens_baglantilar_on_its_connection() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("basemap.googleSatellite"));
        assert_eq!(bottom(&app).0, named("Google Uydu", "google-satellite"));
        let settings = app.document.as_ref().expect("a drawing").model.settings();
        let connection = settings
            .connections
            .first()
            .expect("the preset's connection")
            .clone();
        assert!(last_said(&app).contains(&format!(
            "“{}” bağlantısının anahtarını girin",
            connection.name
        )));
        assert!(matches!(app.dialog, Some(Dialog::Connections)));
        let window = app.connections.as_ref().expect("Bağlantılar");
        let chosen = window.chosen.map(|i| window.rows[i].id.clone());
        assert_eq!(chosen.as_deref(), Some(connection.id.as_str()));
    }
}
