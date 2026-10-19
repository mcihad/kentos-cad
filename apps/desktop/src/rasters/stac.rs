//! STAC'ta ara (docs/adr/0528 §7; `raster.stac`): a STAC API's items that cover the view in a time, their assets,
//! and the preparation of the ones chosen. The window only asks and shows: the search is `cad.stac.search`'s (run on
//! a thread of its own, the host's fetcher answering), the plan under the choices is `cad.stac.prepare`'s (the grid,
//! the system, the scale and offset, the warnings, as the choices change), and Hazırla runs that command: a group
//! with a layer per asset, one undo step “STAC verisini hazırla”. Nothing is reckoned here.

use std::collections::BTreeSet;

use iced::widget::{Column, button, container, row, text_input};
use iced::{Element, Fill, Length, Task};
use kentos_contracts::{
    CommandResult, CommandWarning, StacItemInfo, StacNext, StacPrepare, StacPrepared, StacQa,
    StacSearch, StacSearchRecord, StacSearched,
};
use kentos_interaction::{Level, Vec2};
use kentos_native_application::{ExecutionContext, stac_prepare, stac_search};
use kentos_ui::label;
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{Dialog, overlay};

use super::{Event as Rasters, off_thread};
use crate::app::{App, Dialog as Asking, Message};
use crate::exchange::words::{self, Kind as Line};

pub const TITLE: &str = "STAC'ta ara";

/// The catalog the window opens on: Earth Search by Element 84, public.
pub const ENDPOINT: &str = "https://earth-search.aws.element84.com/v1";

/// The plan's answer: the record and its warnings, or why not.
type Planned = Result<(StacPrepared, Vec<CommandWarning>), String>;

#[derive(Debug, Clone)]
pub struct State {
    pub endpoint: String,
    pub collection: String,
    /// Start and end of the time asked, a date or an instant; either may be empty.
    pub from: String,
    pub to: String,
    pub cloud: String,
    pub limit: String,
    /// The view's box in degrees (west, south, east, north), none: the whole catalog.
    pub bbox: Option<[f64; 4]>,
    pub items: Vec<StacItemInfo>,
    pub matched: Option<u64>,
    pub dropped: u32,
    pub next: Option<StacNext>,
    /// What the last search asked, kept in the record.
    pub searched: Option<StacSearchRecord>,
    pub chosen: Option<usize>,
    /// The data assets and the quality asset chosen.
    pub data: BTreeSet<String>,
    pub qa: Option<String>,
    pub group: String,
    pub planned: Option<Planned>,
    /// A search is out (its ticket).
    pub waiting: Option<u64>,
    pub(crate) ticket: u64,
    pub said: Option<(Line, String)>,
}

impl Default for State {
    fn default() -> State {
        State {
            endpoint: ENDPOINT.to_owned(),
            collection: "sentinel-2-l2a".to_owned(),
            from: String::new(),
            to: String::new(),
            cloud: "20".to_owned(),
            limit: "10".to_owned(),
            bbox: None,
            items: Vec::new(),
            matched: None,
            dropped: 0,
            next: None,
            searched: None,
            chosen: None,
            data: BTreeSet::new(),
            qa: None,
            group: String::new(),
            planned: None,
            waiting: None,
            ticket: 0,
            said: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Endpoint(String),
    Collection(String),
    From(String),
    To(String),
    Cloud(String),
    Limit(String),
    /// Sentinel-2 L2A or Landsat Collection 2 Level-2 on Earth Search.
    Preset(&'static str),
    UseView,
    NoBox,
    Search,
    More,
    Found(u64, Box<Result<StacSearched, String>>),
    Choose(usize),
    Data(String),
    Quality(String),
    Group(String),
    Prepare,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Rasters(Rasters::Stac(e))
}

/// A number typed with a dot or a comma; none when empty, an error when not a number.
fn typed(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(Some)
        .ok_or_else(|| format!("{what} “{}” bir sayı değil.", text.trim()))
}

/// The time asked as a search takes it: `from/to`, an open end `..`; none when both are empty.
fn datetime_of(s: &State) -> Option<String> {
    match (s.from.trim(), s.to.trim()) {
        ("", "") => None,
        (a, "") => Some(format!("{a}/..")),
        ("", b) => Some(format!("../{b}")),
        (a, b) => Some(format!("{a}/{b}")),
    }
}

fn collections_of(text: &str) -> Vec<String> {
    text.split([',', ' ', ';'])
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The search the window's fields make.
fn search_of(s: &State) -> Result<StacSearch, String> {
    let cloud = typed(&s.cloud, "En çok bulut")?;
    let limit = typed(&s.limit, "Sayfa boyu")?;
    Ok(StacSearch {
        endpoint: s.endpoint.trim().to_owned(),
        bbox: s.bbox,
        datetime: datetime_of(s),
        collections: collections_of(&s.collection),
        ids: Vec::new(),
        max_cloud_cover: cloud,
        limit: limit.map(|v| v.max(0.0) as u32),
        next: None,
    })
}

fn record_of(input: &StacSearch) -> StacSearchRecord {
    StacSearchRecord {
        endpoint: input.endpoint.clone(),
        bbox: input.bbox,
        datetime: input.datetime.clone(),
        collections: input.collections.clone(),
        max_cloud_cover: input.max_cloud_cover,
    }
}

/// The data assets and the quality asset an item starts with: the red and near infrared bands, and the quality band
/// of the platforms the command knows (the keys in any case).
fn defaults(item: &StacItemInfo) -> (BTreeSet<String>, Option<String>) {
    let has = |key: &str| {
        item.assets
            .iter()
            .find(|a| a.key.eq_ignore_ascii_case(key))
            .map(|a| a.key.clone())
    };
    let mut data = BTreeSet::new();
    // Red, and one near infrared band (Sentinel-2 has `nir` and `nir08`; Landsat only `nir08`).
    if let Some(k) = has("red") {
        data.insert(k);
    }
    if let Some(k) = has("nir").or_else(|| has("nir08")) {
        data.insert(k);
    }
    let qa = ["scl", "qa_pixel"].iter().find_map(|k| has(k));
    (data, qa)
}

/// What an asset's row says beside its bands.
fn note_of(a: &kentos_contracts::StacAssetInfo) -> String {
    let mut notes = Vec::new();
    if !kentos_services::stac::href::is_http(&a.href) {
        notes.push("http değil");
    }
    if a.signed {
        notes.push("imzalı");
    }
    if a.raster_bands.len() > 1 || a.bands.len() > 1 {
        notes.push("çok bantlı");
    }
    if a.shape.is_none() || a.transform.is_none() || a.epsg.is_none() {
        notes.push("ızgara yok");
    }
    notes.join(", ")
}

fn scale_of(a: &kentos_contracts::StacAssetInfo) -> String {
    match a.raster_bands.first() {
        Some(b) if b.scale.is_some() || b.offset.is_some() => format!(
            "{} · {}",
            crate::crs::js_number(b.scale.unwrap_or(1.0)),
            crate::crs::js_number(b.offset.unwrap_or(0.0))
        ),
        _ => "—".to_owned(),
    }
}

/// The rows of an item worth listing: the bands and the data (not thumbnails and metadata).
fn listed(item: &StacItemInfo) -> impl Iterator<Item = &kentos_contracts::StacAssetInfo> {
    item.assets.iter().filter(|a| {
        !a.raster_bands.is_empty()
            || !a.bands.is_empty()
            || a.roles.iter().any(|r| r == "data" || r == "reflectance")
    })
}

impl App {
    /// `raster.stac`: the window, its time and box from nothing, the view's box when the project has a system.
    pub(crate) fn raster_stac_command(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        let s = State {
            bbox: self.stac_view_box(),
            ..State::default()
        };
        self.rasters.stac = Some(s);
        self.dialog = Some(Asking::RasterStac);
        Task::none()
    }

    /// The view's box in degrees, WGS 84; none without a system.
    fn stac_view_box(&self) -> Option<[f64; 4]> {
        let plane = kentos_processing::address::Plane::of(&self.document.as_ref()?.model)?;
        let b = self.viewport.camera.visible_bounds();
        let corners = [
            Vec2::new(b.min_x, b.min_y),
            Vec2::new(b.max_x, b.min_y),
            Vec2::new(b.max_x, b.max_y),
            Vec2::new(b.min_x, b.max_y),
        ];
        let mut out = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for c in corners {
            let (lon, lat) = plane.to_wgs84(c)?;
            out = [
                out[0].min(lon),
                out[1].min(lat),
                out[2].max(lon),
                out[3].max(lat),
            ];
        }
        // Whole numbers of nothing: four decimals are metres.
        Some(out.map(|v| (v * 10_000.0).round() / 10_000.0))
    }

    /// The window opened with its fields as a test or a picture wants them.
    #[cfg(test)]
    pub(crate) fn raster_stac_with(&mut self, s: State) {
        self.rasters.stac = Some(s);
        self.dialog = Some(Asking::RasterStac);
    }

    /// The prepare command's input from the window's choices; why not when nothing is chosen.
    fn stac_input(&self, s: &State) -> Result<StacPrepare, String> {
        let k = s
            .chosen
            .filter(|&k| k < s.items.len())
            .ok_or_else(|| "Bir öğe seçin.".to_owned())?;
        if s.data.is_empty() {
            return Err("En az bir veri bandı seçin (Veri sütunu).".to_owned());
        }
        let item = s.items[k].clone();
        let group = s.group.trim();
        Ok(StacPrepare {
            assets: item
                .assets
                .iter()
                .filter(|a| s.data.contains(&a.key))
                .map(|a| a.key.clone())
                .collect(),
            item,
            qa: s.qa.clone().map(|asset| StacQa { asset, mask: None }),
            search: s.searched.clone(),
            group: (!group.is_empty()).then(|| group.to_owned()),
            expected_revision: None,
        })
    }

    /// The plan again, after a choice changed.
    fn stac_replan(&mut self) {
        let Some(s) = &self.rasters.stac else {
            return;
        };
        let planned = if s.chosen.is_none() {
            None
        } else {
            Some(match (self.stac_input(s), &self.document) {
                (Err(why), _) => Err(why),
                (Ok(_), None) => Err("Açık çizim yok.".into()),
                (Ok(input), Some(doc)) => {
                    match stac_prepare::plan(&ExecutionContext::new(&mut doc.model.clone()), &input)
                    {
                        CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                        CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                            Err(error.message)
                        }
                        other => Err(format!("{other:?}")),
                    }
                }
            })
        };
        if let Some(s) = &mut self.rasters.stac {
            s.planned = planned;
        }
    }

    /// Ara and Sonraki sayfa: the search on a thread of its own.
    fn stac_search_task(&mut self, more: bool) -> Task<Message> {
        let Some(s) = &mut self.rasters.stac else {
            return Task::none();
        };
        let mut input = match search_of(s) {
            Ok(i) => i,
            Err(why) => {
                s.said = Some((Line::Error, why));
                return Task::none();
            }
        };
        if more {
            match s.next.clone() {
                Some(n) => input.next = Some(n),
                None => return Task::none(),
            }
        } else {
            s.searched = Some(record_of(&input));
            s.items.clear();
            s.chosen = None;
            s.planned = None;
        }
        s.ticket += 1;
        let ticket = s.ticket;
        s.waiting = Some(ticket);
        s.said = None;
        off_thread(move || {
            let found = stac_search::search(&input).map_err(|e| e.message);
            msg(Event::Found(ticket, Box::new(found)))
        })
    }

    pub(crate) fn raster_stac_event(&mut self, e: Event) -> Task<Message> {
        let Some(s) = &mut self.rasters.stac else {
            return Task::none();
        };
        match e {
            Event::Endpoint(t) => s.endpoint = t,
            Event::Collection(t) => s.collection = t,
            Event::From(t) => s.from = t,
            Event::To(t) => s.to = t,
            Event::Cloud(t) => s.cloud = t,
            Event::Limit(t) => s.limit = t,
            Event::Preset(id) => s.collection = id.to_owned(),
            Event::UseView => {
                let found = self.stac_view_box();
                if let Some(s) = &mut self.rasters.stac {
                    match found {
                        Some(b) => s.bbox = Some(b),
                        None => s.said = Some((
                            Line::Warn,
                            "Projenin koordinat sistemi yok: görünümün kutusu dereceye çevrilemez."
                                .to_owned(),
                        )),
                    }
                }
            }
            Event::NoBox => s.bbox = None,
            Event::Search => return self.stac_search_task(false),
            Event::More => return self.stac_search_task(true),
            Event::Found(ticket, found) => {
                if s.waiting != Some(ticket) {
                    return Task::none();
                }
                s.waiting = None;
                match *found {
                    Err(why) => s.said = Some((Line::Error, why)),
                    Ok(page) => {
                        for item in page.items {
                            if !s
                                .items
                                .iter()
                                .any(|i| i.id == item.id && i.collection == item.collection)
                            {
                                s.items.push(item);
                            }
                        }
                        s.matched = page.matched;
                        s.dropped = page.dropped;
                        s.next = page.next;
                    }
                }
                self.stac_replan();
                return Task::none();
            }
            Event::Choose(k) => {
                if k < s.items.len() {
                    s.chosen = Some(k);
                    let (data, qa) = defaults(&s.items[k]);
                    s.data = data;
                    s.qa = qa;
                    s.group.clear();
                }
            }
            Event::Data(key) => {
                if !s.data.remove(&key) {
                    if s.qa.as_deref() == Some(key.as_str()) {
                        s.qa = None;
                    }
                    s.data.insert(key);
                }
            }
            Event::Quality(key) => {
                if s.qa.as_deref() == Some(key.as_str()) {
                    s.qa = None;
                } else {
                    s.data.remove(&key);
                    s.qa = Some(key);
                }
            }
            Event::Group(t) => {
                s.group = t;
                return Task::none();
            }
            Event::Prepare => {
                self.stac_run();
                return Task::none();
            }
            Event::Close => {
                self.rasters.stac = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.stac_replan();
        Task::none()
    }

    fn stac_status(&mut self, why: String) {
        if let Some(s) = &mut self.rasters.stac {
            s.said = Some((Line::Error, why));
        }
    }

    /// Hazırla: the command run, one undo step.
    fn stac_run(&mut self) {
        let Some(s) = self.rasters.stac.clone() else {
            return;
        };
        let input = match self.stac_input(&s) {
            Ok(i) => i,
            Err(why) => {
                self.stac_status(why);
                return;
            }
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let model = &mut doc.model;
        match stac_prepare::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed { output, warnings } => {
                let first = output
                    .preparation
                    .bands
                    .first()
                    .and_then(|b| b.uid.as_deref())
                    .and_then(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .and_then(|u| model.slot_of(u));
                self.rasters.stac = None;
                self.dialog = None;
                if let Some(slot) = first {
                    self.zoom_to(&[slot]);
                }
                let p = &output.preparation;
                self.say(
                    Level::Success,
                    format!(
                        "STAC verisi hazırlandı: {} ({} bant{}). Ctrl+Z grubu, katmanları ve rasterleri birlikte geri alır.",
                        p.item,
                        p.bands.len(),
                        p.qa.as_ref().map_or_else(String::new, |q| format!(", kalite bandı {}", q.asset))
                    ),
                );
                if let Some(r) = p.reflectance {
                    self.say(
                        Level::Info,
                        format!(
                            "Spektral indis için Ölçek {} ve Öteleme {} (yansıma = sayı × ölçek + öteleme).",
                            crate::crs::js_number(r.scale),
                            crate::crs::js_number(r.offset)
                        ),
                    );
                }
                for w in warnings {
                    self.warn(w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                self.stac_status(error.message)
            }
            other => self.stac_status(format!("{other:?}")),
        }
    }

    pub(crate) fn raster_stac_view(&self) -> Element<'_, Message> {
        let Some(s) = &self.rasters.stac else {
            return Column::new().into();
        };
        let mut body = Column::new().spacing(12);
        let input = |placeholder: &'static str, value: &str, on: fn(String) -> Event| {
            kentos_ui::widget::focus_ring(
                text_input(placeholder, value)
                    .on_input(move |t| msg(on(t)))
                    .padding([5, 8])
                    .size(kentos_ui::theme::typography::body())
                    .style(kentos_ui::style::field::input),
            )
        };
        let small = |caption: &'static str, on: Option<Message>| {
            button(label::body(caption))
                .on_press_maybe(on)
                .padding([4, 10])
                .style(kentos_ui::style::button::secondary)
        };
        body = body.push(words::field(
            "STAC adresi",
            input("https://…/v1", &s.endpoint, Event::Endpoint),
            Some(
                "Herkese açık bir STAC API'sinin kökü; giriş bilgisi ve sorgu yazılmaz.".to_owned(),
            ),
        ));
        body = body.push(
            row![
                container(words::field(
                    "Koleksiyon",
                    row![
                        container(input("sentinel-2-l2a", &s.collection, Event::Collection))
                            .width(Fill),
                        small("Sentinel-2 L2A", Some(msg(Event::Preset("sentinel-2-l2a")))),
                        small("Landsat C2 L2", Some(msg(Event::Preset("landsat-c2-l2")))),
                    ]
                    .spacing(6),
                    None
                ))
                .width(Length::FillPortion(3)),
                container(words::field(
                    "En çok bulut (%)",
                    input("20", &s.cloud, Event::Cloud),
                    None
                ))
                .width(Length::FillPortion(1)),
                container(words::field(
                    "Sayfa boyu",
                    input("10", &s.limit, Event::Limit),
                    None
                ))
                .width(Length::FillPortion(1)),
            ]
            .spacing(16),
        );
        let boxed = match s.bbox {
            Some(b) => format!(
                "{}, {}, {}, {} (batı, güney, doğu, kuzey; derece)",
                crate::crs::js_number(b[0]),
                crate::crs::js_number(b[1]),
                crate::crs::js_number(b[2]),
                crate::crs::js_number(b[3])
            ),
            None => "Kutu yok: bütün katalog aranır.".to_owned(),
        };
        body = body.push(
            row![
                container(words::field(
                    "Başlangıç",
                    input("2024-07-01", &s.from, Event::From),
                    None
                ))
                .width(Length::FillPortion(1)),
                container(words::field(
                    "Bitiş",
                    input("2024-07-31", &s.to, Event::To),
                    None
                ))
                .width(Length::FillPortion(1)),
                container(words::field(
                    "Alan",
                    row![
                        container(label::caption(boxed)).width(Fill),
                        small("Görünümden al", Some(msg(Event::UseView))),
                        small("Kutuyu kaldır", s.bbox.map(|_| msg(Event::NoBox))),
                    ]
                    .spacing(6)
                    .align_y(iced::Center),
                    None
                ))
                .width(Length::FillPortion(3)),
            ]
            .spacing(16),
        );
        let searching = s.waiting.is_some();
        body = body.push(
            row![
                words::primary(
                    if searching { "Aranıyor…" } else { "Ara" },
                    (!searching).then(|| msg(Event::Search))
                ),
                words::secondary(
                    "Sonraki sayfa",
                    (!searching && s.next.is_some()).then(|| msg(Event::More))
                ),
                container(label::caption(match (s.items.is_empty(), s.matched) {
                    (true, _) => String::new(),
                    (false, Some(m)) => format!(
                        "{} öğe gösteriliyor, sunucuda {m} eşleşme{}",
                        s.items.len(),
                        if s.dropped > 0 {
                            format!("; {} öğe bulut oranı için süzüldü", s.dropped)
                        } else {
                            String::new()
                        }
                    ),
                    (false, None) => format!("{} öğe gösteriliyor", s.items.len()),
                }))
                .width(Fill),
            ]
            .spacing(8)
            .align_y(iced::Center),
        );
        // The items found.
        let rows: Vec<table::Row<'_, Message>> = s
            .items
            .iter()
            .enumerate()
            .map(|(k, i)| {
                table::Row::new([
                    label::caption(
                        i.datetime
                            .clone()
                            .map_or_else(|| "—".to_owned(), |d| seconds_of(&d)),
                    )
                    .into(),
                    label::caption(i.cloud_cover.map_or_else(
                        || "—".to_owned(),
                        |c| format!("{}", (c * 10.0).round() / 10.0),
                    ))
                    .into(),
                    label::caption(i.platform.clone().unwrap_or_default()).into(),
                    label::caption(i.epsg.map_or_else(String::new, |e| format!("EPSG:{e}"))).into(),
                    label::body(i.id.clone()).into(),
                ])
                .selected(s.chosen == Some(k))
                .on_press(msg(Event::Choose(k)))
            })
            .collect();
        let n = rows.len();
        let items = Table::new([
            table::Column::new("Çekim zamanı").width(168),
            table::Column::new("Bulut %").width(70).align_right(),
            table::Column::new("Platform").width(110),
            table::Column::new("Sistem").width(90),
            table::Column::new("Öğe").width(Length::Fill),
        ])
        .extend(rows)
        .empty("Öğe yok: Ara ile arayın.");
        body = body.push(words::field(
            "Bulunan öğeler",
            container(words::fitted(items, n)).width(Fill),
            None,
        ));
        // The chosen item's assets.
        if let Some(item) = s.chosen.and_then(|k| s.items.get(k)) {
            let rows: Vec<table::Row<'_, Message>> = listed(item)
                .map(|a| {
                    let key = a.key.clone();
                    table::Row::new([
                        words::check(
                            s.data.contains(&a.key),
                            "",
                            Some(msg(Event::Data(key.clone()))),
                        ),
                        words::check(
                            s.qa.as_deref() == Some(a.key.as_str()),
                            "",
                            Some(msg(Event::Quality(key))),
                        ),
                        label::body(a.key.clone()).into(),
                        label::caption(
                            a.bands
                                .first()
                                .and_then(|b| b.common_name.clone())
                                .unwrap_or_default(),
                        )
                        .into(),
                        label::caption(scale_of(a)).into(),
                        label::caption(
                            a.raster_bands
                                .first()
                                .and_then(|b| b.spatial_resolution)
                                .map_or_else(String::new, crate::crs::js_number),
                        )
                        .into(),
                        label::caption(note_of(a)).into(),
                    ])
                })
                .collect();
            let n = rows.len();
            let assets = Table::new([
                table::Column::new("Veri").width(46),
                table::Column::new("Kalite").width(56),
                table::Column::new("Varlık").width(110),
                table::Column::new("Bant").width(80),
                table::Column::new("Ölçek · öteleme")
                    .width(130)
                    .align_right(),
                table::Column::new("Piksel (m)").width(80).align_right(),
                table::Column::new("Not").width(Length::Fill),
            ])
            .extend(rows)
            .empty("Bu öğenin raster varlığı yok.");
            body = body.push(words::field(
                "Varlıklar",
                container(words::fitted(assets, n)).width(Fill),
                Some(
                    "Her varlık tek bantlı bir COG olarak, adresiyle bağlı eklenir; dosya açılmaz."
                        .to_owned(),
                ),
            ));
            body = body.push(words::field(
                "Grubun adı",
                input("STAC: öğenin kimliği", &s.group, Event::Group),
                Some(format!("Boşsa “STAC: {}” olur.", item.id)),
            ));
        }
        // What the plan says.
        let mut lines = Vec::new();
        match &s.planned {
            None => lines.push(words::text_line(
                Line::Info,
                "Bir öğe seçin: kırmızı ve yakın kızılötesi bantlarla kalite bandı kendiliğinden işaretlenir.",
            )),
            Some(Err(why)) => lines.push(words::text_line(Line::Error, why.clone())),
            Some(Ok((p, warnings))) => {
                let r = &p.preparation;
                lines.push(words::text_line(
                    Line::Ok,
                    format!(
                        "{} bant{}{}.",
                        r.bands.len(),
                        r.qa.as_ref().map_or_else(String::new, |q| format!(", kalite bandı {} (maske: {})", q.asset, mask_words(&q.mask))),
                        r.reflectance.map_or_else(String::new, |x| format!(
                            "; yansıma = sayı × {} + {}",
                            crate::crs::js_number(x.scale),
                            crate::crs::js_number(x.offset)
                        ))
                    ),
                ));
                for w in warnings {
                    lines.push(words::text_line(Line::Warn, w.message.clone()));
                }
            }
        }
        if let Some((kind, why)) = &s.said {
            lines.push(words::text_line(*kind, why.clone()));
        }
        body = body.push(words::summary(lines));
        let can = matches!(&s.planned, Some(Ok(_)));
        overlay::blocking(
            Dialog::new(TITLE)
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary("Hazırla", can.then(|| msg(Event::Prepare))))
                .width(860.0)
                .max_height(820.0),
        )
    }
}

/// An instant to the second: `2024-07-13 08:29:15` (the zone and the fraction are the catalog's, kept in the record).
fn seconds_of(d: &str) -> String {
    d.replace('T', " ").chars().take(19).collect()
}

/// A mask rule in words.
fn mask_words(m: &kentos_contracts::StacMask) -> String {
    let join = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    match m {
        kentos_contracts::StacMask::Values { values } => format!("sınıf {}", join(values)),
        kentos_contracts::StacMask::Bits { bits } => format!("bit {}", join(bits)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::{app_with_drawing, drive};
    use kentos_native_application::stac_host;

    fn recorded() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/services/v1/stac")
    }

    fn send(app: &mut App, e: Event) {
        let task = app.update(Message::Rasters(Rasters::Stac(e)));
        drive(app, task);
    }

    /// A drawing in the items' system (UTM 37N).
    fn app() -> App {
        stac_host::register(Box::new(stac_host::Recorded::open(recorded())));
        let mut app = app_with_drawing();
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.srid = 32637;
        doc.model.set_settings(settings);
        app
    }

    fn rasters(app: &App) -> Vec<kentos_contracts::RasterEntity> {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter_map(|e| match e {
                kentos_contracts::Entity::Raster(r) => Some(r.clone()),
                _ => None,
            })
            .collect()
    }

    /// The fields of the recorded search of Sentinel-2 over Sivas (the shared cases' first).
    fn searching() -> State {
        State {
            bbox: Some([36.9, 39.7, 37.1, 39.8]),
            from: "2024-07-01".into(),
            to: "2024-07-20".into(),
            limit: "2".into(),
            ..State::default()
        }
    }

    #[test]
    fn the_window_searches_plans_and_prepares_as_one_step() {
        let mut app = app();
        app.raster_stac_with(searching());
        send(&mut app, Event::Search);
        let s = app.rasters.stac.as_ref().expect("the window");
        assert!(s.waiting.is_none(), "the answer came");
        assert_eq!(s.items.len(), 2, "{:?}", s.said);
        assert_eq!(s.matched, Some(4));
        assert!(s.next.is_some());
        send(&mut app, Event::Choose(0));
        let s = app.rasters.stac.as_ref().expect("the window");
        assert_eq!(s.data.iter().cloned().collect::<Vec<_>>(), ["nir", "red"]);
        assert_eq!(s.qa.as_deref(), Some("scl"));
        let (p, warnings) = s.planned.clone().expect("a plan").expect("the plan");
        assert_eq!(p.preparation.bands.len(), 2);
        assert!(warnings.iter().any(|w| w.code == "stac_qa_grid_differs"));
        send(&mut app, Event::Prepare);
        assert!(app.rasters.stac.is_none(), "the window closes");
        let added = rasters(&app);
        assert_eq!(added.len(), 3);
        assert!(added.iter().all(|r| {
            r.raster
                .url
                .as_deref()
                .is_some_and(|u| u.starts_with("https://"))
        }));
        assert_eq!(added[0].base.attrs["stac.item"], "S2A_37SCD_20240713_0_L2A");
        assert!(app.document.as_ref().unwrap().model.can_undo());
        // One step takes back the group, the layers and the rasters.
        app.document.as_mut().unwrap().model.undo();
        assert!(rasters(&app).is_empty());
    }

    #[test]
    fn a_next_page_adds_its_items_and_a_refusal_is_said() {
        let mut app = app();
        app.raster_stac_with(searching());
        send(&mut app, Event::Search);
        send(&mut app, Event::More);
        let s = app.rasters.stac.as_ref().expect("the window");
        assert_eq!(s.items.len(), 4, "{:?}", s.said);
        // A project in another system: the plan says why, and Hazırla is not offered.
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.srid = 5254;
        doc.model.set_settings(settings);
        send(&mut app, Event::Choose(0));
        let s = app.rasters.stac.as_ref().expect("the window");
        let why = s.planned.clone().expect("a plan").expect_err("refused");
        assert!(
            why.contains("EPSG:32637") && why.contains("EPSG:5254"),
            "{why}"
        );
    }

    #[test]
    fn a_bad_field_is_said_before_any_search() {
        let mut app = app();
        app.raster_stac_with(State {
            cloud: "çok".into(),
            ..searching()
        });
        send(&mut app, Event::Search);
        let s = app.rasters.stac.as_ref().expect("the window");
        assert!(s.waiting.is_none());
        assert!(
            s.said
                .as_ref()
                .is_some_and(|(k, w)| *k == Line::Error && w.contains("çok"))
        );
    }
}
