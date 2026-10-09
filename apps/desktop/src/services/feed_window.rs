//! Servisten veri al (docs/adr/0208 §10, §14; the web's
//! `ui/services/FeedDialog.ts`): the kinds on the left (WFS, OGC API
//! Features, ArcGIS REST, GeoJSON adresi), on the right the address, the
//! connection and Bağlan (`kentos_services::feed`), the searchable list of
//! types, the system asked, the area (the view, the drawing's extent, the
//! selection's, everything), a filter, the most objects, the key that
//! matches objects on Yenile and the target layer. Al takes the objects
//! page by page off the window's thread (Durdur stops it), moves them into
//! the project's system and writes them as one undo step (“Servisten veri
//! al”): a new layer remembers its feed (`cad.layers.service`'s `addFeed`,
//! its fields from the values) and Yenile takes it again.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::widget::{Column, button, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    CommandResult, Entity, FeatureFeed, FeedKind, LayerServiceOperation, LayersService,
};
use kentos_services::feed::{DEFAULT_MOST, FeedChoice, FeedConnecting, Taking};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;

pub const TITLE: &str = "Servisten veri al";

pub const KINDS: [(FeedKind, &str, &str, &str); 4] = [
    (
        FeedKind::Wfs,
        "WFS",
        "serviceFeed",
        "https://cbs.ornek.gov.tr/geoserver/wfs",
    ),
    (
        FeedKind::OgcFeatures,
        "OGC API Features",
        "serviceExtent",
        "https://demo.pygeoapi.io/master",
    ),
    (
        FeedKind::Arcgis,
        "ArcGIS REST",
        "serviceInfo",
        "https://services.arcgis.com/…/FeatureServer",
    ),
    (
        FeedKind::Geojson,
        "GeoJSON adresi",
        "basemapVector",
        "https://ornek.org/veri.geojson",
    ),
];

/// Where the objects are asked from.
pub const AREAS: [&str; 4] = ["Görünüm", "Çizimin kapsamı", "Seçimin kapsamı", "Hepsi"];

#[derive(Clone, Debug)]
pub struct Window {
    pub kind: usize,
    pub url: String,
    pub connection: Option<String>,
    pub connecting: Option<FeedConnecting>,
    pub busy: bool,
    pub said: Option<(Kind, String)>,
    pub search: String,
    pub item: Option<String>,
    pub srid: Option<u32>,
    pub area: usize,
    pub filter: String,
    pub most: String,
    pub key: String,
    /// The target: none, a new layer named `name`; else an existing layer's id.
    pub target: Option<String>,
    pub name: String,
    /// The take on its way: its stop flag and what it has taken.
    pub taking: Option<(Arc<AtomicBool>, u64)>,
    /// Yenile: the layer whose feed is taken again.
    pub refresh: Option<String>,
}

/// What a take brought.
#[derive(Debug, Clone)]
pub struct Taken {
    pub feed: FeatureFeed,
    pub entities: Vec<Entity>,
    /// Left out: no place in the project's system.
    pub dropped: usize,
    pub matched: Option<u64>,
    /// The take stopped at the most asked.
    pub capped: bool,
    /// What the pages' readers left out, summed.
    pub skipped: Vec<kentos_contracts::ReportItem>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Kind(usize),
    Url(String),
    Connection(usize),
    Connect,
    Connected(Result<Box<FeedConnecting>, String>),
    Search(String),
    Item(String),
    Srid(usize),
    Area(usize),
    Filter(String),
    Most(String),
    Key(String),
    Target(usize),
    Name(String),
    Take,
    Progress(u64),
    Taken(Result<Box<Taken>, String>),
    Stop,
    Cancel,
}

fn msg(e: Event) -> Message {
    Message::Services(super::app::Event::Feed(e))
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
            item: None,
            srid: None,
            area: 0,
            filter: String::new(),
            most: DEFAULT_MOST.to_string(),
            key: String::new(),
            target: None,
            name: String::new(),
            taking: None,
            refresh: None,
        }
    }

    fn items(&self) -> Vec<&kentos_services::feed::FeedItem> {
        self.connecting
            .as_ref()
            .and_then(FeedConnecting::offer)
            .map(|o| o.items.iter().collect())
            .unwrap_or_default()
    }

    /// The systems listed for the item chosen (`FeedConnecting::systems`), those KentOS knows.
    fn srids(&self, project: u32) -> Vec<u32> {
        match (&self.connecting, &self.item) {
            (Some(c), Some(item)) => c
                .systems(item, project)
                .into_iter()
                .filter(|s| kentos_services::crs::known(*s))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The system the objects will be asked in: what the list shows chosen.
    fn asked(&self, project: u32) -> Option<u32> {
        let item = self.item.as_ref()?;
        self.connecting
            .as_ref()?
            .asked_srid(item, self.srid, project)
    }
}

/// A box of the project's system in another's: its edges, five points each, moved and boxed.
fn box_in(b: [f64; 4], f: &dyn Fn(f64, f64) -> Option<(f64, f64)>) -> Option<[f64; 4]> {
    let mut out = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for i in 0..=4 {
        let t = f64::from(i) / 4.0;
        for (x, y) in [
            (b[0] + (b[2] - b[0]) * t, b[1]),
            (b[0] + (b[2] - b[0]) * t, b[3]),
            (b[0], b[1] + (b[3] - b[1]) * t),
            (b[2], b[1] + (b[3] - b[1]) * t),
        ] {
            let (px, py) = f(x, y)?;
            out = [
                out[0].min(px),
                out[1].min(py),
                out[2].max(px),
                out[3].max(py),
            ];
        }
    }
    out.iter().all(|v| v.is_finite()).then_some(out)
}

impl App {
    pub(crate) fn open_feed_window(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.feed_window = Some(Window::new());
        self.dialog = Some(Dialog::Feed);
    }

    /// Yenile: a feed layer's objects taken again, by the window's take.
    pub(crate) fn refresh_feed(&mut self, layer: &str) -> Task<Message> {
        let Some(feed) = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(layer))
            .and_then(|n| n.feed.clone())
        else {
            return Task::none();
        };
        let mut w = Window::new();
        w.refresh = Some(layer.to_owned());
        w.target = Some(layer.to_owned());
        w.connection = feed.connection.clone();
        self.feed_window = Some(w);
        self.dialog = Some(Dialog::Feed);
        self.start_take(feed)
    }

    pub(crate) fn feed_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = &mut self.feed_window else {
            return Task::none();
        };
        match event {
            Event::Kind(i) => {
                if i < KINDS.len() && i != w.kind {
                    w.kind = i;
                    w.connecting = None;
                    w.item = None;
                    w.said = None;
                    w.url = String::new();
                }
            }
            Event::Url(t) => {
                w.url = t;
                w.connecting = None;
                w.item = None;
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
                let Some(w) = &mut self.feed_window else {
                    return Task::none();
                };
                w.connection = if i == 0 {
                    None
                } else {
                    ids.get(i - 1).cloned()
                };
            }
            Event::Connect => return self.feed_connect(),
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
                                    "{}: {} tür.",
                                    if o.title.is_empty() {
                                        "Servis okundu"
                                    } else {
                                        &o.title
                                    },
                                    o.items.len()
                                ),
                            ));
                            if o.items.len() == 1 {
                                w.item = Some(o.items[0].id.clone());
                                if w.name.is_empty() {
                                    w.name = o.items[0].title.clone();
                                }
                            }
                        }
                    }
                    Err(why) => w.said = Some((Kind::Error, why)),
                }
            }
            Event::Search(t) => w.search = t,
            Event::Item(id) => {
                let title = w
                    .items()
                    .into_iter()
                    .find(|i| i.id == id)
                    .map(|i| i.title.clone());
                w.item = Some(id);
                w.srid = None;
                if let Some(t) = title {
                    w.name = t;
                }
            }
            Event::Srid(i) => {
                let project = self
                    .document
                    .as_ref()
                    .map_or(0, |d| d.model.settings().srid);
                let Some(w) = &mut self.feed_window else {
                    return Task::none();
                };
                w.srid = w.srids(project).get(i).copied();
            }
            Event::Area(i) => w.area = i.min(AREAS.len() - 1),
            Event::Filter(t) => w.filter = t,
            Event::Most(t) => w.most = t,
            Event::Key(t) => w.key = t,
            Event::Target(i) => {
                let ids: Vec<String> = self
                    .document
                    .as_ref()
                    .map(|d| {
                        d.model
                            .layers()
                            .leaves()
                            .iter()
                            .filter(|n| n.service.is_none())
                            .map(|n| n.id.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                let Some(w) = &mut self.feed_window else {
                    return Task::none();
                };
                w.target = if i == 0 {
                    None
                } else {
                    ids.get(i - 1).cloned()
                };
            }
            Event::Name(t) => w.name = t,
            Event::Take => return self.feed_take(),
            Event::Progress(n) => {
                if let Some((_, taken)) = &mut w.taking {
                    *taken = n;
                }
            }
            Event::Taken(answer) => {
                w.taking = None;
                match answer {
                    Ok(t) => self.feed_write(*t),
                    Err(why) => {
                        if let Some(w) = &mut self.feed_window {
                            w.said = Some((Kind::Error, why));
                        }
                    }
                }
            }
            Event::Stop => {
                if let Some((stop, _)) = &w.taking {
                    stop.store(true, Ordering::Relaxed);
                }
            }
            Event::Cancel => {
                if let Some((stop, _)) = &w.taking {
                    stop.store(true, Ordering::Relaxed);
                }
                self.feed_window = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    fn feed_connection(
        &self,
        id: Option<&String>,
    ) -> Option<(
        kentos_contracts::ServiceConnection,
        Option<kentos_contracts::ConnectionSecret>,
    )> {
        let id = id?;
        let c = self
            .document
            .as_ref()?
            .model
            .settings()
            .connections
            .iter()
            .find(|c| &c.id == id)?
            .clone();
        let secret = super::secrets::secrets().get(&c.origin, &c.id);
        Some((c, secret))
    }

    pub(crate) fn feed_connect(&mut self) -> Task<Message> {
        let conn = self.feed_connection(
            self.feed_window
                .as_ref()
                .and_then(|w| w.connection.as_ref()),
        );
        let Some(w) = &mut self.feed_window else {
            return Task::none();
        };
        let kind = KINDS[w.kind].0;
        let url = w.url.trim().to_owned();
        w.busy = true;
        w.said = Some((Kind::Info, "Servis okunuyor…".into()));
        w.connecting = None;
        w.item = None;
        super::app::off_thread(
            move || {
                let (mut c, mut next) = FeedConnecting::start(kind, &url)?;
                let mut steps = 0;
                while let Some(req) = next {
                    steps += 1;
                    if steps > 8 {
                        return Err(
                            "Servis okunurken çok fazla istek gerekti; adresi denetleyin."
                                .to_owned(),
                        );
                    }
                    let body = super::net::send_text(
                        req,
                        conn.as_ref().map(|(c, s)| (c, s.as_ref())),
                        &url,
                        6,
                    )?;
                    next = c.answer(&body)?;
                }
                Ok(Box::new(c))
            },
            |answer| super::app::Event::Feed(Event::Connected(answer)),
        )
    }

    /// The area chosen, in the project's system.
    fn feed_area(&self, area: usize) -> Option<[f64; 4]> {
        match area {
            0 => {
                let b = self.viewport.camera.visible_bounds();
                Some([b.min_x, b.min_y, b.max_x, b.max_y])
            }
            1 => self
                .spatial
                .extent()
                .map(|b| [b.min_x, b.min_y, b.max_x, b.max_y]),
            2 => {
                let ids: Vec<f64> = self
                    .selection
                    .ids()
                    .iter()
                    .map(|s| f64::from(s.0))
                    .collect();
                if ids.is_empty() {
                    return None;
                }
                self.spatial
                    .store()
                    .extent(Some(&ids))
                    .map(|b| [b.min_x, b.min_y, b.max_x, b.max_y])
            }
            _ => None,
        }
    }

    /// Al: the feed the choice makes, taken.
    fn feed_take(&mut self) -> Task<Message> {
        let project = self
            .document
            .as_ref()
            .map_or(0, |d| d.model.settings().srid);
        let Some(w) = &self.feed_window else {
            return Task::none();
        };
        if w.area == 2 && self.selection.is_empty() {
            if let Some(w) = &mut self.feed_window {
                w.said = Some((
                    Kind::Error,
                    "Seçim yok: önce çizimde bir alan seçin ya da başka bir alan kullanın.".into(),
                ));
            }
            return Task::none();
        }
        let choice = FeedChoice {
            item: w.item.clone().unwrap_or_default(),
            srid: w.srid,
            filter: Some(w.filter.clone()).filter(|f| !f.trim().is_empty()),
            bbox: self.feed_area(w.area),
            limit: w.most.trim().parse::<u64>().ok().filter(|n| *n > 0),
            key: Some(w.key.clone()).filter(|k| !k.trim().is_empty()),
            connection: w.connection.clone(),
        };
        let feed = match w.connecting.as_ref().map(|c| c.feed(&choice, project)) {
            Some(Ok(f)) => f,
            Some(Err(e)) => {
                if let Some(w) = &mut self.feed_window {
                    w.said = Some((Kind::Error, e));
                }
                return Task::none();
            }
            None => {
                if let Some(w) = &mut self.feed_window {
                    w.said = Some((Kind::Error, "Önce Bağlan ile servisi okuyun.".into()));
                }
                return Task::none();
            }
        };
        if w.target.is_none() && w.name.trim().is_empty() {
            if let Some(w) = &mut self.feed_window {
                w.said = Some((Kind::Error, "Yeni katmana bir ad verin.".into()));
            }
            return Task::none();
        }
        self.start_take(feed)
    }

    /// Takes `feed`'s objects page by page off the window's thread.
    fn start_take(&mut self, feed: FeatureFeed) -> Task<Message> {
        let conn = self.feed_connection(feed.connection.as_ref());
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let project = Arc::new(super::systems::ProjectSystem::of(doc.model.settings()));
        let geojson = self
            .feed_window
            .as_ref()
            .and_then(|w| w.connecting.as_ref())
            .is_some_and(FeedConnecting::geojson);
        let most = self
            .feed_window
            .as_ref()
            .and_then(|w| w.most.trim().parse::<u32>().ok())
            .unwrap_or(DEFAULT_MOST);
        let stop = Arc::new(AtomicBool::new(false));
        if let Some(w) = &mut self.feed_window {
            w.taking = Some((stop.clone(), 0));
            w.said = Some((Kind::Info, "Nesneler alınıyor…".into()));
        }
        iced_runtime::task::blocking(
            move |mut out: iced::futures::channel::mpsc::Sender<Message>| {
                let send = |out: &mut iced::futures::channel::mpsc::Sender<Message>, e: Event| {
                    let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                        out,
                        msg(e),
                    ));
                };
                let answer = take_all(&feed, &project, conn.as_ref(), most, geojson, &stop, |n| {
                    send(&mut out, Event::Progress(n));
                });
                send(&mut out, Event::Taken(answer.map(Box::new)));
            },
        )
    }

    /// The objects taken written as one undo step.
    fn feed_write(&mut self, t: Taken) {
        let Some(w) = self.feed_window.clone() else {
            return;
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let n = t.entities.len();
        let label = if w.refresh.is_some() {
            "Servisi yenile"
        } else {
            "Servisten veri al"
        };
        let mut feed = t.feed.clone();
        feed.fetched = Some(crate::settings::rfc3339(std::time::SystemTime::now()));
        let refresh = w.refresh.clone();
        let target = w.target.clone();
        let name = w.name.trim().to_owned();
        let result = doc.model.transact(label, |m| -> Result<String, String> {
            let layer = match (&refresh, &target) {
                (Some(id), _) => {
                    // Yenile: the layer's objects replaced (by key when given), its feed's time renewed.
                    let input = LayersService {
                        operation: LayerServiceOperation::Update,
                        layer: Some(id.clone()),
                        name: None,
                        parent: None,
                        index: None,
                        service: None,
                        feed: Some(feed.clone()),
                        fields: None,
                        connections: None,
                        expected_revision: None,
                    };
                    refusal(kentos_native_application::layers_service::execute(
                        &mut kentos_native_application::ExecutionContext::new(m),
                        input,
                    ))?;
                    replace_objects(m, id, &feed, t.entities.clone())?;
                    return Ok(id.clone());
                }
                (None, Some(id)) => id.clone(),
                (None, None) => {
                    let rows: Vec<Vec<(String, String)>> = t
                        .entities
                        .iter()
                        .map(|e| {
                            e.base()
                                .attrs
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect()
                        })
                        .collect();
                    let fields = kentos_contracts::fields::infer_fields(
                        rows.iter().map(Vec::as_slice),
                        |a, b| kentos_geometry_core::text::natural::natural_cmp(a, b),
                    );
                    let input = LayersService {
                        operation: LayerServiceOperation::AddFeed,
                        layer: None,
                        name: Some(name.clone()),
                        parent: None,
                        index: Some(0),
                        service: None,
                        feed: Some(feed.clone()),
                        fields: Some(fields),
                        connections: None,
                        expected_revision: None,
                    };
                    let id = match kentos_native_application::layers_service::execute(
                        &mut kentos_native_application::ExecutionContext::new(m),
                        input,
                    ) {
                        CommandResult::Completed { output, .. } => output.layer,
                        other => return Err(refusal(other).err().unwrap_or_default()),
                    };
                    // A colour of its own, seen on any basemap.
                    if let Some(node) = m.layers().get(&id) {
                        let (line, fill) =
                            kentos_services::feed::layer_colors(m.layers().leaves().len());
                        let mut style = node.style.clone();
                        style.color = line.into();
                        style.fill = Some(fill.into());
                        m.set_layer_style(&id, style, label);
                    }
                    id
                }
            };
            let mut list = t.entities.clone();
            for e in &mut list {
                e.base_mut().layer_id = layer.clone();
            }
            m.add_many(list, label)
                .map_err(|e| format!("{e}. Hiçbir nesne eklenmedi."))?;
            Ok(layer)
        });
        match result {
            Ok(_) => {
                self.feed_window = None;
                self.dialog = None;
                let said = kentos_services::feed::taken_words(
                    n,
                    t.matched,
                    &t.skipped,
                    t.dropped,
                    t.capped,
                    t.feed.srid.unwrap_or(4326),
                );
                // What was left out is a warning: the user may want to take it again otherwise.
                let level = if t.skipped.is_empty() && t.dropped == 0 {
                    kentos_interaction::Level::Success
                } else {
                    kentos_interaction::Level::Warn
                };
                self.say(level, said);
            }
            Err(e) => {
                if let Some(w) = &mut self.feed_window {
                    w.said = Some((Kind::Error, e));
                }
            }
        }
    }

    pub(crate) fn feed_window_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.feed_window else {
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
                .on_press_maybe(w.refresh.is_none().then(|| msg(Event::Kind(i))))
                .style(style::button::table_row(w.kind == i, false)),
            );
        }
        let left = container(kinds)
            .style(style::container::field_box)
            .padding(4)
            .width(Length::Fixed(typography::scaled(180.0)));
        let right = self.feed_form(w);
        let line = match (&w.taking, &w.said) {
            (Some((_, n)), _) => {
                words::text_line(Kind::Info, format!("Nesneler alınıyor… {n} nesne."))
            }
            (None, Some((kind, t))) => words::text_line(*kind, t.clone()),
            (None, None) => words::text_line(
                Kind::Info,
                "Adresi yazıp Bağlan'a basın: servisin türleri listelenir.",
            ),
        };
        let taking = w.taking.is_some();
        let ready = w.connecting.is_some() && w.item.is_some() && !taking;
        let mut frame = Frame::new(if w.refresh.is_some() {
            "Servisi yenile"
        } else {
            TITLE
        })
        .push(column![row![left, right].spacing(16), words::summary(vec![line])].spacing(12))
        .action(words::secondary("Vazgeç", Some(msg(Event::Cancel))))
        .width(900.0);
        frame = if taking {
            frame.action(words::primary("Durdur", Some(msg(Event::Stop))))
        } else {
            frame.action(words::primary("Al", ready.then(|| msg(Event::Take))))
        };
        overlay::modal(frame, msg(Event::Cancel))
    }

    fn feed_form<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let labelled = |name: &'a str, body: Element<'a, Message>| -> Element<'a, Message> {
            row![
                container(label::body(name)).width(Length::Fixed(typography::scaled(120.0))),
                body
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let input =
            |hint: &'a str, value: &'a str, on: fn(String) -> Event| -> Element<'a, Message> {
                text_input(hint, value)
                    .size(typography::body())
                    .padding([4, 8])
                    .on_input(move |t| msg(on(t)))
                    .style(style::field::input)
                    .into()
            };
        let connections: Vec<String> = self
            .document
            .as_ref()
            .map(|d| {
                d.model
                    .settings()
                    .connections
                    .iter()
                    .map(|c| format!("{} ({})", c.name, c.auth.label()))
                    .collect()
            })
            .unwrap_or_default();
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
        let mut choices = vec![Choice::new("Yok (kimlik istemez)")];
        choices.extend(connections.into_iter().map(Choice::new));
        let chosen = match &w.connection {
            None => Some(0),
            Some(id) => ids.iter().position(|c| c == id).map(|i| i + 1),
        };
        let mut form = Column::new().spacing(8);
        if w.refresh.is_none() {
            form = form
                .push(labelled(
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
                ))
                .push(labelled(
                    "Bağlantı",
                    Select::new(choices, chosen, |i| msg(Event::Connection(i))).into(),
                ));
        }
        let items = w.items();
        if !items.is_empty() {
            let needle = kentos_contracts::fields::fold(w.search.trim());
            let mut list = Column::new().spacing(1);
            for it in items
                .iter()
                .filter(|i| {
                    needle.is_empty()
                        || kentos_contracts::fields::fold(&i.title).contains(&needle)
                        || kentos_contracts::fields::fold(&i.id).contains(&needle)
                })
                .take(400)
            {
                let mut line = row![label::body(it.title.clone())]
                    .spacing(8)
                    .align_y(Center);
                if !it.id.is_empty() && it.id != it.title {
                    line = line.push(label::caption(it.id.clone()));
                }
                if let Some(g) = &it.geometry {
                    line = line.push(label::caption(
                        g.trim_start_matches("esriGeometry").to_owned(),
                    ));
                }
                list = list.push(
                    button(line)
                        .width(Fill)
                        .padding([3, 6])
                        .on_press(msg(Event::Item(it.id.clone())))
                        .style(style::button::table_row(
                            w.item.as_ref() == Some(&it.id),
                            false,
                        )),
                );
            }
            form = form.push(input("Tür ara", &w.search, Event::Search)).push(
                container(scrollable(list).height(Length::Fixed(typography::scaled(150.0))))
                    .style(style::container::field_box)
                    .width(Fill),
            );
            let project = self
                .document
                .as_ref()
                .map_or(0, |d| d.model.settings().srid);
            let srids = w.srids(project);
            if !srids.is_empty() {
                let chosen = w
                    .asked(project)
                    .and_then(|s| srids.iter().position(|x| *x == s));
                form = form.push(labelled(
                    "İstenen sistem",
                    Select::new(
                        srids.iter().map(|s| {
                            let c = Choice::new(kentos_project::crs::system(*s).map_or_else(
                                || format!("EPSG:{s}"),
                                |c| format!("{} (EPSG:{s})", c.name),
                            ));
                            if *s == project {
                                c.detail("projenin sistemi")
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
            form = form
                .push(labelled(
                    "Alan",
                    Select::new(AREAS.iter().map(|a| Choice::new(*a)), Some(w.area), |i| {
                        msg(Event::Area(i))
                    })
                    .into(),
                ))
                .push(labelled(
                    "Süzgeç",
                    input(
                        if KINDS[w.kind].0 == FeedKind::Arcgis {
                            "ArcGIS where: ADA = 104"
                        } else {
                            "CQL: ada = '104'"
                        },
                        &w.filter,
                        Event::Filter,
                    ),
                ))
                .push(labelled(
                    "En çok nesne",
                    input("50000", &w.most, Event::Most),
                ))
                .push(labelled(
                    "Anahtar alan",
                    input(
                        "Yenile'de nesneleri eşler (isteğe bağlı)",
                        &w.key,
                        Event::Key,
                    ),
                ));
            let layers: Vec<(String, String)> = self
                .document
                .as_ref()
                .map(|d| {
                    d.model
                        .layers()
                        .leaves()
                        .iter()
                        .filter(|n| n.service.is_none())
                        .map(|n| (n.id.clone(), d.model.layers().path(&n.id)))
                        .collect()
                })
                .unwrap_or_default();
            let mut targets = vec![Choice::new("Yeni katman (kaynağını hatırlar)")];
            targets.extend(layers.iter().map(|(_, p)| Choice::new(p.clone())));
            let chosen = match &w.target {
                None => Some(0),
                Some(id) => layers.iter().position(|(l, _)| l == id).map(|i| i + 1),
            };
            form = form.push(labelled(
                "Hedef katman",
                Select::new(targets, chosen, |i| msg(Event::Target(i))).into(),
            ));
            if w.target.is_none() {
                form = form.push(labelled(
                    "Katmanın adı",
                    input("Yeni katmanın adı", &w.name, Event::Name),
                ));
            }
        }
        container(form).width(Fill).into()
    }
}

fn refusal<T>(r: CommandResult<T>) -> Result<T, String> {
    match r {
        CommandResult::Completed { output, .. } => Ok(output),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("Komut tamamlanmadı.".into()),
    }
}

/// Yenile's objects: with a key, the old object of the same key keeps its
/// identity and takes the new one's geometry and attributes, the new ones
/// go in and the ones the service no longer gives go; without a key the
/// layer's objects are replaced.
fn replace_objects(
    m: &mut kentos_domain::Document,
    layer: &str,
    feed: &FeatureFeed,
    fresh: Vec<Entity>,
) -> Result<(), String> {
    let old: Vec<(kentos_domain::Slot, Entity)> = m
        .entities()
        .filter(|e| e.base().layer_id == layer)
        .map(|e| (kentos_domain::Slot(e.base().id), e.clone()))
        .collect();
    let mut fresh = fresh;
    for e in &mut fresh {
        e.base_mut().layer_id = layer.to_owned();
    }
    let Some(key) = &feed.key else {
        m.remove(&old.iter().map(|(s, _)| *s).collect::<Vec<_>>());
        m.add_many(fresh, "Servisi yenile")
            .map_err(|e| e.to_string())?;
        return Ok(());
    };
    let value = |e: &Entity| e.base().attrs.get(key).cloned();
    let mut gone = Vec::new();
    let mut kept = std::collections::HashSet::new();
    for (slot, e) in &old {
        match value(e).and_then(|v| {
            fresh
                .iter()
                .position(|f| value(f).as_deref() == Some(v.as_str()))
        }) {
            Some(i) => {
                let mut f = fresh[i].clone();
                *f.base_mut() = kentos_contracts::EntityBase {
                    attrs: f.base().attrs.clone(),
                    ..e.base().clone()
                };
                m.update(*slot, f);
                kept.insert(i);
            }
            None => gone.push(*slot),
        }
    }
    m.remove(&gone);
    let new: Vec<Entity> = fresh
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !kept.contains(i))
        .map(|(_, e)| e)
        .collect();
    m.add_many(new, "Servisi yenile")
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Every page of `feed` taken (each with the connection's proof), moved into
/// the project's system; `progress` hears the count after each page.
pub(crate) fn take_all(
    feed: &FeatureFeed,
    project: &super::systems::ProjectSystem,
    conn: Option<&(
        kentos_contracts::ServiceConnection,
        Option<kentos_contracts::ConnectionSecret>,
    )>,
    most: u32,
    geojson: bool,
    stop: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> Result<Taken, String> {
    let srid = feed.srid.unwrap_or(4326);
    let pair = super::systems::pair(project, srid)?;
    let area = match feed.bbox {
        Some(b) => Some(
            box_in(b, &|x, y| (pair.to_grid)(x, y))
                .ok_or("İstenen alan servisin sisteminde gösterilemiyor.")?,
        ),
        None => None,
    };
    let (mut taking, first) = Taking::start(feed, area, most, geojson);
    let mut next = Some(first);
    let mut entities = Vec::new();
    let mut dropped = 0;
    let mut pages = 0;
    while let Some(req) = next {
        if stop.load(Ordering::Relaxed) {
            return Err("Alma durduruldu; çizim değişmedi.".into());
        }
        pages += 1;
        if pages > 1000 {
            break;
        }
        let body = super::net::send_text(req, conn.map(|(c, s)| (c, s.as_ref())), &feed.url, 4)?;
        let (mut result, more) = taking.answer(&body, "")?;
        dropped += kentos_services::features::transform(&mut result, |p| {
            (pair.to_project)(p.x, p.y).map(|(x, y)| kentos_contracts::Vec2 { x, y })
        });
        entities.extend(result.entities);
        progress(entities.len() as u64);
        next = more;
    }
    let capped = taking.taken >= u64::from(most);
    Ok(Taken {
        feed: feed.clone(),
        entities,
        dropped,
        matched: taking.matched,
        capped,
        skipped: taking.skipped.clone(),
    })
}
