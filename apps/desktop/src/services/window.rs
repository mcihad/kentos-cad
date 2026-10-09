//! Harita servisi (docs/adr/0208 §14; the web's `ui/services/ServiceDialog.ts`):
//! the kinds on the left (Hazır altlıklar, XYZ / TMS, WMS, WMTS, OGC API
//! Tiles, Vektör karo, ArcGIS REST, Google), on the right the address, the
//! connection and Bağlan, which reads what the service has
//! (`kentos_services::connect`, every request with the connection's proof,
//! off the window's thread), then the searchable list of its layers and
//! what to ask for: style, format, system (the project's when offered),
//! clear ground, one picture of the view, the levels; the layer's name and
//! opacity. Ekle writes it by `cad.layers.service` as one undo step: an
//! opaque service goes to the bottom, a clear one over the basemaps. Opened
//! from a service layer's menu (Servis ayarları) it changes that layer.

use iced::widget::{Column, button, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    LayerNodeType, LayerServiceOperation, LayersService, ServiceKind, ServiceLayer,
};
use kentos_services::connect::{Choice as Picked, Connecting, Item, suggested_srid};
use kentos_services::presets;
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;

pub const TITLE: &str = "Harita servisi";

/// The kinds on the left: none is Hazır altlıklar.
pub const KINDS: [(Option<ServiceKind>, &str, &str, &str); 8] = [
    (None, "Hazır altlıklar", "basemap", ""),
    (
        Some(ServiceKind::Xyz),
        "XYZ / TMS",
        "basemapOsm",
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
    ),
    (
        Some(ServiceKind::Wms),
        "WMS",
        "serviceAdd",
        "https://cbs.ornek.gov.tr/geoserver/wms",
    ),
    (
        Some(ServiceKind::Wmts),
        "WMTS",
        "serviceSettings",
        "https://atlas.harita.gov.tr/wmts/1.0.0/WMTSCapabilities.xml",
    ),
    (
        Some(ServiceKind::OgcTiles),
        "OGC API Tiles",
        "serviceExtent",
        "https://maps.ornek.org/ogcapi/collections/orto/map/tiles",
    ),
    (
        Some(ServiceKind::Vector),
        "Vektör karo",
        "basemapVector",
        "https://tiles.openfreemap.org/styles/liberty",
    ),
    (
        Some(ServiceKind::Arcgis),
        "ArcGIS REST",
        "serviceInfo",
        "https://services.arcgisonline.com/arcgis/rest/services/World_Imagery/MapServer",
    ),
    (Some(ServiceKind::Google), "Google", "basemapGoogleRoad", ""),
];

/// The opacities offered.
const OPACITIES: [f64; 7] = [1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4];

/// The window's state.
#[derive(Clone, Debug)]
pub struct Window {
    /// Its place in [`KINDS`].
    pub kind: usize,
    pub url: String,
    pub connection: Option<String>,
    /// What Bağlan read.
    pub connecting: Option<Connecting>,
    pub busy: bool,
    pub said: Option<(Kind, String)>,
    pub search: String,
    pub picked: Vec<String>,
    pub style: Option<String>,
    pub format: Option<String>,
    pub srid: Option<u32>,
    pub opacity: f64,
    pub transparent: bool,
    pub dynamic: bool,
    pub name: String,
    /// The name as the service named it, until the user types one.
    pub named: bool,
    pub tile_size: u32,
    pub subdomains: String,
    pub y_flip: bool,
    pub max_zoom: String,
    pub attribution: String,
    /// Servis ayarları: the layer it changes.
    pub layer: Option<String>,
    /// A changed layer's service as it was (its fields kept when Bağlan is not pressed).
    pub before: Option<ServiceLayer>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Kind(usize),
    Url(String),
    Connection(usize),
    Connect,
    Connected(Result<Box<Connecting>, String>),
    Search(String),
    Pick(String),
    Style(usize),
    Format(usize),
    Srid(usize),
    Opacity(usize),
    Transparent(bool),
    Dynamic(bool),
    Name(String),
    TileSize(usize),
    Subdomains(String),
    YFlip(bool),
    MaxZoom(String),
    Attribution(String),
    Preset(String),
    Add,
    Cancel,
}

fn msg(e: Event) -> Message {
    Message::Services(super::app::Event::Window(e))
}

fn kind_at(i: usize) -> Option<ServiceKind> {
    KINDS.get(i).and_then(|k| k.0)
}

impl Window {
    fn new() -> Window {
        Window {
            kind: 0,
            url: String::new(),
            connection: None,
            connecting: None,
            busy: false,
            said: None,
            search: String::new(),
            picked: Vec::new(),
            style: None,
            format: None,
            srid: None,
            opacity: 1.0,
            transparent: true,
            dynamic: false,
            name: String::new(),
            named: false,
            tile_size: 256,
            subdomains: String::new(),
            y_flip: false,
            max_zoom: String::new(),
            attribution: String::new(),
            layer: None,
            before: None,
        }
    }

    /// Bağlan's offer, once read.
    fn offer(&self) -> Option<&kentos_services::connect::Offer> {
        self.connecting.as_ref().and_then(Connecting::offer)
    }

    /// The items picked, as the offer has them.
    fn picked_items(&self) -> Vec<&Item> {
        let Some(o) = self.offer() else {
            return Vec::new();
        };
        self.picked
            .iter()
            .filter_map(|id| o.items.iter().find(|i| &i.id == id))
            .collect()
    }

    /// The systems the picked items are all offered in.
    fn srids(&self) -> Vec<u32> {
        let items = self.picked_items();
        let Some(first) = items.first() else {
            return Vec::new();
        };
        let mut out = first.srids.clone();
        for i in &items[1..] {
            out.retain(|s| i.srids.contains(s));
        }
        out.retain(|s| kentos_services::crs::known(*s));
        out
    }

    fn choice(&self) -> Picked {
        Picked {
            items: self.picked.clone(),
            style: self.style.clone(),
            format: self.format.clone(),
            srid: self.srid,
            transparent: self.transparent,
            dynamic: self.dynamic,
            opacity: Some(self.opacity),
            connection: self.connection.clone(),
            map_type: None,
            min_zoom: None,
            max_zoom: self.max_zoom.trim().parse().ok(),
            tile_size: Some(self.tile_size),
            subdomains: self
                .subdomains
                .split([',', ' '])
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
            y_flip: self.y_flip,
            attribution: Some(self.attribution.clone()).filter(|a| !a.trim().is_empty()),
        }
    }
}

/// Where a new service goes in the top of the tree: an opaque one under
/// everything, a clear one over the services at the bottom (the basemaps).
fn place_for(nodes: &[kentos_contracts::LayerNode], clear: bool) -> Option<u32> {
    if !clear {
        return None;
    }
    let basemaps = nodes
        .iter()
        .rev()
        .take_while(|n| n.kind == LayerNodeType::Layer && n.service.is_some())
        .count();
    Some((nodes.len() - basemaps) as u32)
}

impl App {
    /// Opens Harita servisi; on a layer drawn from a service (Servis ayarları), to change it.
    pub(crate) fn open_service_window(&mut self, layer: Option<String>) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let mut w = Window::new();
        if let Some(id) = &layer
            && let Some(n) = doc.model.layers().get(id)
            && let Some(s) = &n.service
        {
            w.kind = KINDS.iter().position(|k| k.0 == Some(s.kind)).unwrap_or(1);
            w.url = s.url.clone();
            w.connection = s.connection.clone();
            w.opacity = s.opacity.unwrap_or(1.0);
            w.transparent = s.transparent;
            w.dynamic = s.dynamic;
            w.name = n.name.clone();
            w.named = true;
            w.tile_size = s.tile_size.unwrap_or(256);
            w.subdomains = s.subdomains.join(",");
            w.y_flip = s.y_flip;
            w.max_zoom = s.max_zoom.map(|z| z.to_string()).unwrap_or_default();
            w.attribution = s.attribution.clone().unwrap_or_default();
            w.before = Some(s.clone());
            w.layer = layer.clone();
        }
        self.service_window = Some(w);
        self.dialog = Some(Dialog::ServiceAdd);
    }

    pub(crate) fn service_window_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = &mut self.service_window else {
            return Task::none();
        };
        match event {
            Event::Kind(i) => {
                if i < KINDS.len() && i != w.kind {
                    w.kind = i;
                    w.connecting = None;
                    w.picked.clear();
                    w.said = None;
                    w.url = String::new();
                    w.style = None;
                    w.format = None;
                    w.srid = None;
                    if !w.named || w.layer.is_none() {
                        w.name = String::new();
                        w.named = false;
                    }
                    if kind_at(i) == Some(ServiceKind::Google) {
                        return self.service_connect();
                    }
                }
            }
            Event::Url(t) => {
                w.url = t;
                w.connecting = None;
                w.picked.clear();
            }
            Event::Connection(i) => {
                let ids: Vec<String> = self
                    .document
                    .as_ref()
                    .map(|d| {
                        d.model
                            .settings()
                            .connections
                            .iter()
                            .map(|c| c.id.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                let Some(w) = &mut self.service_window else {
                    return Task::none();
                };
                if i == 0 {
                    w.connection = None;
                } else if let Some(id) = ids.get(i - 1) {
                    w.connection = Some(id.clone());
                } else {
                    // Yeni bağlantı…: Bağlantılar over this window, a new connection in it.
                    self.open_connections(None);
                    return self.connections_event(super::connections::Event::Add);
                }
            }
            Event::Connect => return self.service_connect(),
            Event::Connected(answer) => {
                w.busy = false;
                match answer {
                    Ok(c) => {
                        let offer = c.offer().cloned();
                        w.connecting = Some(*c);
                        if let Some(o) = offer {
                            w.said = Some((
                                Kind::Ok,
                                format!(
                                    "{}: {} katman.",
                                    if o.title.is_empty() {
                                        "Servis okundu".to_owned()
                                    } else {
                                        o.title.clone()
                                    },
                                    o.pickable,
                                ),
                            ));
                            if !w.named || w.name.is_empty() {
                                w.name = o.title.clone();
                            }
                            // One to pick: picked.
                            let pickable: Vec<&Item> =
                                o.items.iter().filter(|i| i.pickable).collect();
                            if pickable.len() == 1 {
                                w.picked = vec![pickable[0].id.clone()];
                            }
                        }
                    }
                    Err(why) => w.said = Some((Kind::Error, why)),
                }
            }
            Event::Search(t) => w.search = t,
            Event::Pick(id) => {
                let many = kind_at(w.kind) == Some(ServiceKind::Wms)
                    || (kind_at(w.kind) == Some(ServiceKind::Arcgis) && !id.is_empty());
                if many {
                    if let Some(at) = w.picked.iter().position(|p| *p == id) {
                        w.picked.remove(at);
                    } else {
                        // The whole service and its layers do not go together.
                        w.picked.retain(|p| !p.is_empty());
                        w.picked.push(id.clone());
                    }
                } else {
                    w.picked = vec![id.clone()];
                }
                w.style = None;
                w.format = None;
                if !w.named
                    && let Some(item) = w.offer().and_then(|o| o.items.iter().find(|i| i.id == id))
                {
                    w.name = item.title.clone();
                }
            }
            Event::Style(i) => {
                let styles: Vec<String> = w
                    .picked_items()
                    .first()
                    .map(|it| it.styles.iter().map(|s| s.id.clone()).collect())
                    .unwrap_or_default();
                w.style = styles.get(i).cloned();
            }
            Event::Format(i) => {
                let formats = formats_of(w);
                w.format = formats.get(i).cloned();
            }
            Event::Srid(i) => {
                let srids = w.srids();
                w.srid = srids.get(i).copied();
            }
            Event::Opacity(i) => w.opacity = OPACITIES.get(i).copied().unwrap_or(1.0),
            Event::Transparent(on) => w.transparent = on,
            Event::Dynamic(on) => w.dynamic = on,
            Event::Name(t) => {
                w.name = t;
                w.named = true;
            }
            Event::TileSize(i) => w.tile_size = if i == 1 { 512 } else { 256 },
            Event::Subdomains(t) => w.subdomains = t,
            Event::YFlip(on) => w.y_flip = on,
            Event::MaxZoom(t) => w.max_zoom = t,
            Event::Attribution(t) => w.attribution = t,
            Event::Preset(id) => {
                if let Some(p) = presets::preset(&id) {
                    self.service_window = None;
                    self.dialog = None;
                    self.add_basemap(p);
                }
            }
            Event::Add => self.service_add(),
            Event::Cancel => {
                self.service_window = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Bağlan: the service's capabilities (and what they lead to) read off the window's thread.
    fn service_connect(&mut self) -> Task<Message> {
        let conn = self.service_window.as_ref().and_then(|w| {
            let id = w.connection.as_ref()?;
            let doc = self.document.as_ref()?;
            let c = doc
                .model
                .settings()
                .connections
                .iter()
                .find(|c| &c.id == id)?
                .clone();
            let secret = super::secrets::secrets().get(&c.origin, &c.id);
            Some((c, secret))
        });
        let Some(w) = &mut self.service_window else {
            return Task::none();
        };
        let Some(kind) = kind_at(w.kind) else {
            return Task::none();
        };
        if kind == ServiceKind::Google && conn.is_none() {
            // Google's offer needs nothing read; its key is asked for at Ekle.
            match Connecting::start(kind, "") {
                Ok((c, _)) => {
                    w.connecting = Some(c);
                    w.said = Some((
                        Kind::Info,
                        "Bir harita türü seçin; anahtar bir Google bağlantısında olmalı.".into(),
                    ));
                }
                Err(e) => w.said = Some((Kind::Error, e)),
            }
            return Task::none();
        }
        let url = w.url.trim().to_owned();
        w.busy = true;
        w.said = Some((Kind::Info, "Servis okunuyor…".into()));
        w.connecting = None;
        w.picked.clear();
        super::app::off_thread(
            move || connect_now(kind, &url, conn.as_ref().map(|(c, s)| (c, s.as_ref()))),
            |answer| super::app::Event::Window(Event::Connected(answer)),
        )
    }

    /// Ekle (or Kaydet on a service layer): the service layer the choice makes, as one undo step.
    fn service_add(&mut self) {
        let project = self
            .document
            .as_ref()
            .map_or(0, |d| d.model.settings().srid);
        let Some(w) = &mut self.service_window else {
            return;
        };
        let name = w.name.trim().to_owned();
        if name.is_empty() {
            w.said = Some((Kind::Error, "Katmana bir ad verin.".into()));
            return;
        }
        let service = match (&w.connecting, &w.before) {
            (Some(c), _) => match c.layer(&w.choice(), project) {
                Ok(s) => s,
                Err(e) => {
                    w.said = Some((Kind::Error, e));
                    return;
                }
            },
            // Servis ayarları without Bağlan: the general fields only.
            (None, Some(b)) => ServiceLayer {
                opacity: (w.opacity < 1.0).then_some(w.opacity),
                attribution: Some(w.attribution.clone()).filter(|a| !a.trim().is_empty()),
                max_zoom: w.max_zoom.trim().parse().ok().or(b.max_zoom),
                connection: w.connection.clone(),
                ..b.clone()
            },
            (None, None) => {
                w.said = Some((Kind::Error, "Önce Bağlan ile servisi okuyun.".into()));
                return;
            }
        };
        let layer = w.layer.clone();
        let clear = service.transparent || service.opacity.is_some();
        let index = self
            .document
            .as_ref()
            .and_then(|d| place_for(d.model.layers().nodes(), clear));
        let input = LayersService {
            operation: if layer.is_some() {
                LayerServiceOperation::Update
            } else {
                LayerServiceOperation::Add
            },
            layer: layer.clone(),
            name: Some(name.clone()),
            parent: None,
            index: if layer.is_some() { None } else { index },
            service: Some(service.clone()),
            feed: None,
            fields: None,
            connections: None,
            expected_revision: None,
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        match kentos_native_application::layers_service::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            input,
        ) {
            kentos_contracts::CommandResult::Completed { .. } => {
                self.service_window = None;
                self.dialog = None;
                self.say(
                    kentos_interaction::Level::Success,
                    if layer.is_some() {
                        format!("“{name}” servis katmanı değişti.")
                    } else {
                        format!("“{name}” servis katmanı eklendi.")
                    },
                );
                if let Some(id) = &service.connection
                    && let Some(doc) = &self.document
                    && let Some(c) = doc
                        .model
                        .settings()
                        .connections
                        .iter()
                        .find(|c| &c.id == id)
                    && let Some(why) = kentos_services::auth::missing(
                        c,
                        super::secrets::secrets().get(&c.origin, &c.id).as_ref(),
                    )
                {
                    self.warn(why);
                }
            }
            kentos_contracts::CommandResult::Failed { error }
            | kentos_contracts::CommandResult::Conflict { error }
            | kentos_contracts::CommandResult::NeedsInput { error } => {
                if let Some(w) = &mut self.service_window {
                    w.said = Some((Kind::Error, error.message));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn service_window_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.service_window else {
            return iced::widget::text("").into();
        };
        let mut kinds = Column::new().spacing(2);
        for (i, (_, name, glyph, _)) in KINDS.iter().enumerate() {
            kinds = kinds.push(
                button(
                    row![icon(from_web(Some(glyph))).size(16.0), label::body(*name)]
                        .spacing(8)
                        .align_y(Center),
                )
                .width(Fill)
                .padding([6, 10])
                .on_press_maybe((w.layer.is_none() || i == w.kind).then(|| msg(Event::Kind(i))))
                .style(style::button::table_row(w.kind == i, false)),
            );
        }
        let left = container(kinds)
            .style(style::container::field_box)
            .padding(4)
            .width(Length::Fixed(typography::scaled(180.0)));
        let right: Element<'_, Message> = if KINDS[w.kind].0.is_none() {
            self.presets_view()
        } else {
            self.service_form(w)
        };
        let line = match &w.said {
            Some((kind, t)) => words::text_line(*kind, t.clone()),
            None => words::text_line(
                Kind::Info,
                match KINDS[w.kind].0 {
                    None => "Bir altlığa tıklayın: en alttaki hazır altlığın yerini alır ya da en alta eklenir.".to_owned(),
                    Some(ServiceKind::Google) => {
                        "Google'ın karoları kullanıcının API anahtarıyla gelir (Map Tiles API); anahtarı bir Google bağlantısına girin.".to_owned()
                    }
                    Some(_) => "Adresi yazıp Bağlan'a basın: servisin katmanları listelenir.".to_owned(),
                },
            ),
        };
        let ready = w.connecting.is_some() && !w.picked.is_empty()
            || (w.connecting.is_none() && w.before.is_some());
        let mut frame = Frame::new(if w.layer.is_some() {
            "Servis ayarları"
        } else {
            TITLE
        })
        .push(column![row![left, right].spacing(16), words::summary(vec![line])].spacing(12))
        .action(words::secondary("Vazgeç", Some(msg(Event::Cancel))))
        .width(900.0);
        if KINDS[w.kind].0.is_some() {
            frame = frame.action(words::primary(
                if w.layer.is_some() { "Kaydet" } else { "Ekle" },
                (ready && !w.busy).then(|| msg(Event::Add)),
            ));
        }
        overlay::modal(frame, msg(Event::Cancel))
    }

    /// The ready basemaps by group, each a button with its icon.
    fn presets_view(&self) -> Element<'_, Message> {
        let c = presets::catalog();
        let mut body = Column::new().spacing(10);
        for g in &c.groups {
            let mut line = iced::widget::Row::new().spacing(8);
            for p in c.presets.iter().filter(|p| p.group == g.id) {
                line = line.push(
                    button(
                        column![
                            icon(from_web(Some(p.icon.as_str()))).size(28.0),
                            label::body(p.name.clone()).center(),
                        ]
                        .spacing(6)
                        .align_x(Center),
                    )
                    .width(Length::Fixed(typography::scaled(124.0)))
                    .padding([10, 6])
                    .on_press(msg(Event::Preset(p.id.clone())))
                    .style(style::button::library_tile(false)),
                );
            }
            body = body.push(column![label::strong(g.name.clone()), line.wrap()].spacing(6));
        }
        scrollable(body)
            .height(Length::Fixed(typography::scaled(430.0)))
            .into()
    }

    fn service_form<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let kind = kind_at(w.kind).unwrap_or(ServiceKind::Xyz);
        let labelled = |name: &'a str, body: Element<'a, Message>| -> Element<'a, Message> {
            row![
                container(label::body(name)).width(Length::Fixed(typography::scaled(110.0))),
                body
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let connections: Vec<(String, String)> = self
            .document
            .as_ref()
            .map(|d| {
                d.model
                    .settings()
                    .connections
                    .iter()
                    .map(|c| (c.id.clone(), format!("{} ({})", c.name, c.auth.label())))
                    .collect()
            })
            .unwrap_or_default();
        let mut choices = vec![Choice::new("Yok (kimlik istemez)")];
        choices.extend(connections.iter().map(|(_, n)| Choice::new(n.clone())));
        choices.push(Choice::new("Yeni bağlantı…").icon(from_web(Some("serviceConnections"))));
        let chosen = match &w.connection {
            None => Some(0),
            Some(id) => connections.iter().position(|(c, _)| c == id).map(|i| i + 1),
        };
        let conn = Select::new(choices, chosen, |i| msg(Event::Connection(i)));
        let mut form = Column::new().spacing(8);
        if kind != ServiceKind::Google {
            form = form.push(labelled(
                "Adres",
                row![
                    text_input(KINDS[w.kind].3, &w.url)
                        .size(typography::body())
                        .padding([4, 8])
                        .on_input(|t| msg(Event::Url(t)))
                        .on_submit(msg(Event::Connect))
                        .style(style::field::input),
                    button(label::body("Bağlan"))
                        .style(style::button::primary)
                        .padding([4, 14])
                        .on_press_maybe(
                            (!w.busy && !w.url.trim().is_empty()).then(|| msg(Event::Connect))
                        ),
                ]
                .spacing(8)
                .align_y(Center)
                .into(),
            ));
        }
        form = form.push(labelled("Bağlantı", conn.into()));
        if kind == ServiceKind::Google {
            form = form.push(
                button(label::body("Harita türlerini göster"))
                    .style(style::button::secondary)
                    .padding([4, 12])
                    .on_press_maybe((!w.busy).then(|| msg(Event::Connect))),
            );
        }
        // The service's layers, searchable.
        if let Some(o) = w.offer() {
            let many = kind == ServiceKind::Wms || kind == ServiceKind::Arcgis;
            let needle = kentos_contracts::fields::fold(w.search.trim());
            let mut list = Column::new().spacing(1);
            let mut shown = 0;
            for it in &o.items {
                if !needle.is_empty()
                    && !kentos_contracts::fields::fold(&it.title).contains(&needle)
                    && !kentos_contracts::fields::fold(&it.id).contains(&needle)
                {
                    continue;
                }
                shown += 1;
                if shown > 400 {
                    break;
                }
                let on = w.picked.contains(&it.id);
                let mark: Element<'_, Message> = if many {
                    check_box(
                        if on { Check::Checked } else { Check::Unchecked },
                        it.pickable.then(|| msg(Event::Pick(it.id.clone()))),
                    )
                } else {
                    icon(from_web(Some(if it.vector {
                        "basemapVector"
                    } else {
                        "basemap"
                    })))
                    .size(14.0)
                    .into()
                };
                let mut words_line = row![mark, label::body(it.title.clone())]
                    .spacing(8)
                    .align_y(Center);
                if !it.id.is_empty() && it.id != it.title && kind != ServiceKind::OgcTiles {
                    words_line = words_line.push(label::caption(it.id.clone()));
                }
                if it.queryable && kind == ServiceKind::Wms {
                    words_line = words_line.push(label::caption("sorgulanır"));
                }
                list = list.push(
                    button(container(words_line).padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: 14.0 * it.depth as f32,
                    }))
                    .width(Fill)
                    .padding([3, 6])
                    .on_press_maybe(it.pickable.then(|| msg(Event::Pick(it.id.clone()))))
                    .style(style::button::table_row(on, false)),
                );
            }
            form = form
                .push(
                    text_input("Katman ara", &w.search)
                        .size(typography::body())
                        .padding([4, 8])
                        .on_input(|t| msg(Event::Search(t)))
                        .style(style::field::input),
                )
                .push(
                    container(scrollable(list).height(Length::Fixed(typography::scaled(170.0))))
                        .style(style::container::field_box)
                        .width(Fill),
                );
            form = form.push(self.service_options(w, kind));
        } else if kind == ServiceKind::Xyz && w.before.is_none() {
            form = form.push(label::caption(
                "Şablonun yer tutucuları: {z}, {x}, {y} ya da {-y} (TMS), {s} (alt alanlar), {r} (yüksek çözünürlük), {quadkey}.",
            ));
        }
        let opacity = Select::new(
            OPACITIES
                .iter()
                .map(|o| Choice::new(format!("% {}", (o * 100.0).round()))),
            OPACITIES.iter().position(|o| (o - w.opacity).abs() < 1e-9),
            |i| msg(Event::Opacity(i)),
        );
        form = form
            .push(labelled(
                "Katmanın adı",
                text_input("Katman ağacındaki adı", &w.name)
                    .size(typography::body())
                    .padding([4, 8])
                    .on_input(|t| msg(Event::Name(t)))
                    .style(style::field::input)
                    .into(),
            ))
            .push(labelled("Saydamlık", opacity.into()));
        container(form).width(Fill).into()
    }

    /// What to ask for, once a layer is picked: per kind.
    fn service_options<'a>(&'a self, w: &'a Window, kind: ServiceKind) -> Element<'a, Message> {
        let labelled = |name: &'a str, body: Element<'a, Message>| -> Element<'a, Message> {
            row![
                container(label::body(name)).width(Length::Fixed(typography::scaled(110.0))),
                body
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let check = |on: bool, words: &'a str, e: fn(bool) -> Event| -> Element<'a, Message> {
            row![
                check_box(
                    if on { Check::Checked } else { Check::Unchecked },
                    Some(msg(e(!on)))
                ),
                label::body(words)
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let project = self
            .document
            .as_ref()
            .map_or(0, |d| d.model.settings().srid);
        let mut opts = Column::new().spacing(8);
        let items = w.picked_items();
        if matches!(kind, ServiceKind::Wms | ServiceKind::Wmts)
            && items.len() == 1
            && !items[0].styles.is_empty()
        {
            let styles = &items[0].styles;
            let chosen = w
                .style
                .as_ref()
                .and_then(|s| styles.iter().position(|x| &x.id == s))
                .or(Some(0));
            opts = opts.push(labelled(
                "Stil",
                Select::new(
                    styles.iter().map(|s| Choice::new(s.title.clone())),
                    chosen,
                    |i| msg(Event::Style(i)),
                )
                .into(),
            ));
        }
        let formats = formats_of(w);
        if matches!(kind, ServiceKind::Wms | ServiceKind::Wmts) && !formats.is_empty() {
            let chosen = w
                .format
                .as_ref()
                .and_then(|f| formats.iter().position(|x| x == f));
            opts = opts.push(labelled(
                "Biçim",
                Select::new(
                    formats.iter().map(|f| Choice::new(f.clone())),
                    chosen,
                    |i| msg(Event::Format(i)),
                )
                .placeholder("Önerilen")
                .into(),
            ));
        }
        let srids = w.srids();
        if matches!(kind, ServiceKind::Wms | ServiceKind::Wmts) && !srids.is_empty() {
            let suggested = suggested_srid(&srids, project);
            let chosen = w
                .srid
                .or(suggested)
                .and_then(|s| srids.iter().position(|x| *x == s));
            opts = opts.push(labelled(
                "Sistem",
                Select::new(
                    srids.iter().map(|s| {
                        let name = kentos_project::crs::system(*s).map_or_else(
                            || format!("EPSG:{s}"),
                            |c| format!("{} (EPSG:{s})", c.name),
                        );
                        let c = Choice::new(name);
                        if Some(*s) == suggested {
                            c.detail(if *s == project {
                                "projenin sistemi"
                            } else {
                                "önerilen"
                            })
                        } else {
                            c
                        }
                    }),
                    chosen,
                    |i| msg(Event::Srid(i)),
                )
                .into(),
            ));
        }
        match kind {
            ServiceKind::Wms => {
                opts = opts
                    .push(check(
                        w.transparent,
                        "Saydam zemin (altındaki katmanlar görünür)",
                        Event::Transparent,
                    ))
                    .push(check(
                        w.dynamic,
                        "Görünüm başına tek resim (karosuz)",
                        Event::Dynamic,
                    ));
            }
            ServiceKind::Arcgis => {
                opts = opts.push(check(w.transparent, "Saydam zemin", Event::Transparent));
            }
            ServiceKind::Xyz => {
                opts = opts
                    .push(labelled(
                        "Karo boyu",
                        Select::new(
                            [Choice::new("256 piksel"), Choice::new("512 piksel")],
                            Some(usize::from(w.tile_size == 512)),
                            |i| msg(Event::TileSize(i)),
                        )
                        .into(),
                    ))
                    .push(labelled(
                        "Alt alanlar",
                        text_input("a,b,c ({s} için)", &w.subdomains)
                            .size(typography::body())
                            .padding([4, 8])
                            .on_input(|t| msg(Event::Subdomains(t)))
                            .style(style::field::input)
                            .into(),
                    ))
                    .push(check(
                        w.y_flip,
                        "TMS: satırlar alttan sayılır",
                        Event::YFlip,
                    ))
                    .push(labelled(
                        "En büyük kat",
                        text_input("19", &w.max_zoom)
                            .size(typography::body())
                            .padding([4, 8])
                            .on_input(|t| msg(Event::MaxZoom(t)))
                            .style(style::field::input)
                            .into(),
                    ))
                    .push(labelled(
                        "Atıf",
                        text_input("© Kaynağın adı", &w.attribution)
                            .size(typography::body())
                            .padding([4, 8])
                            .on_input(|t| msg(Event::Attribution(t)))
                            .style(style::field::input)
                            .into(),
                    ));
            }
            _ => {}
        }
        opts.into()
    }
}

/// Bağlan's requests in turn, each with the connection's proof, waited for
/// (on a thread of its own; the pictures call it directly).
pub(crate) fn connect_now(
    kind: ServiceKind,
    url: &str,
    conn: Option<(
        &kentos_contracts::ServiceConnection,
        Option<&kentos_contracts::ConnectionSecret>,
    )>,
) -> Result<Box<Connecting>, String> {
    let (mut c, mut next) = Connecting::start(kind, url)?;
    let mut steps = 0;
    while let Some(req) = next {
        steps += 1;
        if steps > 16 {
            return Err("Servis okunurken çok fazla istek gerekti; adresi denetleyin.".to_owned());
        }
        let body = super::net::send_text(req, conn, url, 6)?;
        next = c.answer(&body)?;
    }
    Ok(Box::new(c))
}

/// The formats the picked item offers (WMTS) or the service (WMS).
fn formats_of(w: &Window) -> Vec<String> {
    match kind_at(w.kind) {
        Some(ServiceKind::Wmts) => w
            .picked_items()
            .first()
            .map(|i| i.formats.clone())
            .unwrap_or_default(),
        Some(ServiceKind::Wms) => w.offer().map(|o| o.formats.clone()).unwrap_or_default(),
        _ => Vec::new(),
    }
}
