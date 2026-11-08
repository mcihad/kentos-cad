//! A layer's source system on the desktop (docs/adr/0583 §1–§2): the layer's right-click menu Koordinat sistemi ▸
//! says where its objects came from, Tanımla ▸ writes or clears that record (`cad.layers.crs`), Dönüştür ▸
//! opens the preview of moving the objects from the system chosen into the project's (`cad.layers.reproject`,
//! reproject_window.rs, docs/adr/2500); its Uygula moves them.

use iced::Task;
use kentos_commands::{LayerNode, LayersCrs};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_native_application::layers_crs::assign;
use kentos_ui::widget::Menu;

use crate::app::{App, Message};

/// What Koordinat sistemi ▸ asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The layer's record: a system of the registry, none takes it away.
    Assign(String, Option<u32>),
    /// The layer's objects from this system into the project's, previewed first; none: from its record.
    Reproject(String, Option<u32>),
}

impl App {
    /// Koordinat sistemini tanımla… and Koordinatları dönüştür from the command line or a menu without a layer
    /// (docs/adr/0583 §2): the
    /// active layer's record said, the preview of moving its objects from the recorded system (docs/adr/2500); a
    /// system is chosen in the layer's menu.
    pub(crate) fn layer_crs_run(&mut self, moving: bool) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let layers = doc.model.layers();
        let Some(node) = layers.get(layers.active()).cloned() else {
            return Task::none();
        };
        let record = node.source_crs.as_ref().map(|c| (c.name.clone(), c.srid));
        match (moving, record) {
            (true, Some((_, Some(_)))) => {
                return self.layer_crs_event(Event::Reproject(node.id.clone(), None));
            }
            (true, _) => self.warn(format!(
                "“{}” katmanının kayıtlı kaynak sistemi yok: Katmanlar'da katmanın sağ tık menüsünde Koordinat sistemi ▸ \
                 Dönüştür ▸ ile nesnelerin bulunduğu sistemi seçin.",
                node.name
            )),
            (false, record) => self.say(
                Level::Info,
                format!(
                    "“{}” katmanının kaynak sistemi: {}. Değiştirmek için Katmanlar'da katmanın sağ tık menüsünde \
                     Koordinat sistemi ▸ Tanımla ▸.",
                    node.name,
                    record.map_or_else(|| "bilinmiyor".to_owned(), |(name, _)| name)
                ),
            ),
        }
        Task::none()
    }

    pub(crate) fn layer_crs_event(&mut self, event: Event) -> Task<Message> {
        let (layer, srid) = match event {
            Event::Reproject(layer, from) => return self.open_reproject(layer, from),
            Event::Assign(layer, srid) => (layer, srid),
        };
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let name = doc
            .model
            .layers()
            .get(&layer)
            .map_or_else(|| layer.clone(), |n| n.name.clone());
        let result = assign::execute(
            &mut ExecutionContext::new(&mut doc.model),
            LayersCrs {
                layer,
                srid,
                expected_revision: None,
            },
        );
        if let Some(out) = self.answered(result) {
            let said = match (&out.crs, out.changed) {
                (_, false) => format!("“{name}” katmanının kaynak sistemi zaten böyle."),
                (Some(c), true) => {
                    format!("“{name}” katmanının kaynak sistemi: {}. Nesneler değişmedi.", c.name)
                }
                (None, true) => format!("“{name}” katmanının kaynak sistemi kaydı silindi."),
            };
            self.say(Level::Success, said);
        }
        Task::none()
    }

    /// Koordinat sistemi ▸ of a layer: its record, Tanımla ▸ and Dönüştür ▸, the registry's systems by datum.
    pub(crate) fn layer_crs_menu(&self, node: &LayerNode) -> Menu<Message> {
        let Some(doc) = &self.document else {
            return Menu::new();
        };
        let settings = doc.settings();
        let id = node.id.clone();
        let record = node.source_crs.as_ref();
        let current = record.and_then(|c| c.srid);
        let icon = |name: &str| crate::icons::from_web(Some(name));
        let mut menu = Menu::new().header(match record {
            Some(c) => format!("Kaynak: {}", c.name),
            None => "Kaynak sistemi bilinmiyor".to_owned(),
        });
        if let Some(area) = record.and_then(|c| c.area.as_ref()) {
            menu = menu.item(format!("Kullanım alanı: {area}"), None);
        }
        // Every registry system but the local one, the project's own too: a layer may come from it.
        let groups = crate::second_crs::choices(0);
        let mut assign = Menu::new();
        for (datum, systems) in &groups {
            let list = systems.iter().fold(Menu::new(), |list, s| {
                list.radio(
                    s.name.clone(),
                    current == Some(s.srid),
                    Message::LayerCrs(Event::Assign(id.clone(), Some(s.srid))),
                )
                .hint(format!("EPSG:{}", s.srid))
            });
            assign = assign.submenu(*datum, list);
        }
        assign = assign.separator().item(
            "Kaydı sil",
            record.map(|_| Message::LayerCrs(Event::Assign(id.clone(), None))),
        );
        menu = menu
            .separator()
            .submenu("Tanımla", assign)
            .icon(icon("crs"));
        // Moving needs the project's own system and an unlocked layer (docs/adr/0583 §2).
        let can_move = settings.has_system() && !node.locked;
        let mut moving = Menu::new();
        if let Some(srid) = current.filter(|s| *s != settings.srid) {
            moving = moving
                .item(
                    format!("Kayıttaki sistemden (EPSG:{srid})"),
                    can_move.then(|| Message::LayerCrs(Event::Reproject(id.clone(), None))),
                )
                .separator();
        }
        for (datum, systems) in &groups {
            let list =
                systems
                    .iter()
                    .filter(|s| s.srid != settings.srid)
                    .fold(Menu::new(), |list, s| {
                        list.item(
                            s.name.clone(),
                            can_move.then(|| {
                                Message::LayerCrs(Event::Reproject(id.clone(), Some(s.srid)))
                            }),
                        )
                        .hint(format!("EPSG:{}", s.srid))
                    });
            moving = moving.submenu(*datum, list);
        }
        menu.submenu("Dönüştür", moving)
            .icon(icon("geoReproject"))
    }
}
