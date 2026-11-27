//! GNSS içe aktar (docs/adr/0169 §6; the web's `ui/io/GnssImportDialog.ts`):
//! a receiver's GPX or NMEA file, read off the UI thread by the shared reader
//! (`kentos_formats::gnss`), its positions moved from WGS 84 into the
//! project's system the way Koordinat dönüştür moves them, with the accuracy
//! and what it rests on said (docs/adr/0167; the rules are
//! `kentos_interaction::gnss`). The kinds of points to take and how the
//! unnamed are named are chosen; the points show as they will be written. The
//! file's routes and tracks (an NMEA log's fixes) come in as polylines too
//! unless the user unticks them (docs/adr/0222 §6). A project without a system
//! takes none: GNSS positions have one, and they are never placed without it
//! (CLAUDE.md §5). The points and lines go in as one undo step, onto a new
//! layer or one of the drawing's (docs/adr/9329): a new layer's name is one
//! no layer has, and the lines go onto a layer of their own unless the user
//! puts them with the points. The new layers keep the file they came from;
//! the log keeps what the preview said (the transformation, the heights, the
//! records not taken).

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, Row, column, container, row};
use iced::{Element, Fill, Length, Task};
use kentos_commands::{Bounds, GnssRead, LineError};
use kentos_geometry_core::crs::format_dd;
use kentos_interaction::gnss::{self, Options, Placed, Plan};
use kentos_interaction::{Level, fixed};
use kentos_project::systems::{self, Named};
use kentos_ui::label;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::{Dialog, overlay};

use super::apply::{self, ImportPlan, LayerTarget};
use super::coord_import::point_layer_style;
use super::new_layer;
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Kind, Picked, Window, message, off_thread};
use crate::app::{App, Message};

/// Rows of the preview table.
const PREVIEW_ROWS: usize = 12;

/// Problems shown in the summary; the rest are counted.
const SHOWN_PROBLEMS: usize = 5;

static READS: AtomicU64 = AtomicU64::new(0);

/// The kinds of points in the order of the list, as the window names them.
const KINDS: [(&str, &str); 4] = [
    ("wpt", "Yol noktaları"),
    ("rtept", "Rota noktaları"),
    ("trkpt", "İz noktaları"),
    ("gga", "GGA konumları"),
];

/// The format as the file line names it.
fn format_name(format: &str) -> &str {
    match format {
        "gpx" => "GPX 1.1",
        "nmea" => "NMEA 0183",
        other => other,
    }
}

#[derive(Debug, Clone)]
pub struct State {
    file: Picked,
    read: Option<Arc<GnssRead>>,
    failed: Option<String>,
    reading_id: u64,
    reading: bool,
    /// The kinds the user left out; every other kind in the file is taken.
    off: BTreeSet<String>,
    /// The routes and tracks as lines (docs/adr/0222 §6).
    lines: bool,
    prefix: String,
    /// The first number as typed; the last good one stands while it is not one.
    start_text: String,
    start: u32,
    /// The target layer's id; `None`: a new layer named `name`.
    target: Option<String>,
    name: String,
    /// Where the routes and tracks go (docs/adr/9329).
    lines_to: LinesTo,
    /// Their new layer's name.
    lines_name: String,
    status: Option<(bool, String)>,
    /// The points as they will be written, with the current choices; none
    /// while the file is read or the project has no system to take them.
    plan: Option<Arc<Plan>>,
}

/// Where a file's routes and tracks go: a new layer of their own, the points' layer, or one of the drawing's.
#[derive(Debug, Clone, PartialEq)]
pub enum LinesTo {
    New,
    Points,
    Layer(String),
}

/// The key the lines go to their layer by in the import's plan; the points' is the empty one.
const LINES_KEY: &str = "cizgiler";

#[derive(Debug, Clone)]
pub enum Event {
    Read {
        id: u64,
        read: Arc<GnssRead>,
    },
    Kind(&'static str),
    /// The routes and tracks as lines, or not.
    Lines,
    Prefix(String),
    Start(String),
    Target(Option<String>),
    Name(String),
    LinesTo(LinesTo),
    LinesName(String),
    Another,
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::GnssImport(e))
}

/// Reads the file off the UI thread.
fn read(id: u64, bytes: Arc<[u8]>) -> Task<Message> {
    off_thread(
        move || kentos_formats::gnss::read(&bytes),
        move |read| {
            Exchange::GnssImport(Event::Read {
                id,
                read: Arc::new(read),
            })
        },
    )
}

/// The kinds in the file, with how many of each, in the list's order.
fn counts(read: &GnssRead) -> Vec<(&'static str, &'static str, usize)> {
    KINDS
        .iter()
        .map(|(kind, name)| {
            let n = read.points.iter().filter(|p| p.kind == *kind).count();
            (*kind, *name, n)
        })
        .filter(|(_, _, n)| *n > 0)
        .collect()
}

/// The extent of the placed points.
fn bounds(placed: &[Placed]) -> Bounds {
    let mut b = Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    };
    for p in placed {
        b.min_x = b.min_x.min(p.p.x);
        b.min_y = b.min_y.min(p.p.y);
        b.max_x = b.max_x.max(p.p.x);
        b.max_y = b.max_y.max(p.p.y);
    }
    b
}

impl App {
    /// The project's own system, when it has one (docs/adr/0168 §1).
    fn gnss_system(&self) -> Option<Named> {
        systems::own(self.document.as_ref()?.settings())
    }

    pub(super) fn gnss_import_picked(&mut self, file: Picked) -> Task<Message> {
        let target = match &self.exchange {
            Some(Window::GnssImport(s)) => s.target.clone(),
            _ => None,
        };
        let id = READS.fetch_add(1, Ordering::Relaxed) + 1;
        // Names no layer has (docs/adr/9329): the file's, its lines' after it.
        let base = words::base_name(&file.name);
        let (name, lines_name) = match &self.document {
            Some(d) => {
                let name = apply::free_layer_name(&d.model, &base);
                let lines = apply::free_layer_name(&d.model, &format!("{name}_iz"));
                (name, lines)
            }
            None => (base.clone(), format!("{base}_iz")),
        };
        let state = State {
            name,
            lines_to: LinesTo::New,
            lines_name,
            file,
            read: None,
            failed: None,
            reading_id: id,
            reading: true,
            off: BTreeSet::new(),
            lines: true,
            prefix: "G".to_owned(),
            start_text: "1".to_owned(),
            start: 1,
            target,
            status: None,
            plan: None,
        };
        let task = read(id, state.file.bytes.clone());
        self.open_window(Window::GnssImport(Box::new(state)));
        task
    }

    pub(super) fn gnss_import_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Another => return self.pick(Kind::Gnss),
            Event::Run => {
                self.gnss_import_run();
                return Task::none();
            }
            _ => {}
        }
        let Some(Window::GnssImport(s)) = &mut self.exchange else {
            return Task::none();
        };
        let mut again = true;
        match e {
            Event::Read { id, read } => {
                if id != s.reading_id {
                    return Task::none();
                }
                s.reading = false;
                s.failed = None;
                s.read = Some(read);
                s.off.clear();
            }
            Event::Kind(kind) => {
                if !s.off.remove(kind) {
                    s.off.insert(kind.to_owned());
                }
            }
            Event::Lines => s.lines = !s.lines,
            Event::Prefix(p) => s.prefix = p.trim().to_owned(),
            Event::Start(t) => {
                if let Ok(n) = t.trim().parse::<u32>() {
                    s.start = n;
                }
                s.start_text = t;
            }
            Event::Target(t) => {
                s.target = t;
                again = false;
            }
            Event::Name(name) => {
                s.name = name;
                again = false;
            }
            Event::LinesTo(to) => {
                s.lines_to = to;
                again = false;
            }
            Event::LinesName(name) => {
                s.lines_name = name;
                again = false;
            }
            Event::Another | Event::Run => {}
        }
        if again {
            self.gnss_place();
        }
        Task::none()
    }

    /// The points again with the current choices.
    fn gnss_place(&mut self) {
        let system = self.gnss_system();
        let choices = self
            .document
            .as_ref()
            .map(|d| systems::choices(d.settings()))
            .unwrap_or_default();
        // What the project says its heights are (docs/adr/0519 §6).
        let heights = self
            .document
            .as_ref()
            .and_then(|d| d.settings().heights.as_ref().map(|h| h.kind));
        // Its geoid model by name and SHA-256, when it names one (docs/adr/0631 §5).
        let doc_geoid = self
            .document
            .as_ref()
            .and_then(|d| d.settings().heights.as_ref())
            .and_then(|h| h.geoid.as_ref())
            .and_then(|g| g.sha256.clone().map(|id| (g.name.clone(), id)));
        let Some(Window::GnssImport(s)) = &mut self.exchange else {
            return;
        };
        s.plan = match (&s.read, system) {
            (
                Some(read),
                Some(Named {
                    name,
                    system: Some(to),
                    ..
                }),
            ) => {
                let kinds = counts(read)
                    .into_iter()
                    .map(|(k, _, _)| k.to_owned())
                    .filter(|k| !s.off.contains(k))
                    .collect();
                let options = Options {
                    kinds,
                    prefix: s.prefix.clone(),
                    start: s.start,
                    lines: s.lines,
                };
                // The project's geoid model when this device keeps its grid (docs/adr/0631 §5).
                let model = doc_geoid.as_ref().and_then(|(name, id)| {
                    kentos_native_application::crs_models::load(
                        id,
                        kentos_native_application::crs_models::ModelKind::Geoid,
                        None,
                    )
                    .ok()
                    .and_then(|()| kentos_geometry_core::crs::geoid::get(id))
                    .map(|g| (name.clone(), g))
                });
                Some(Arc::new(gnss::place_all_in(
                    &read.points,
                    &read.lines,
                    &options,
                    &to,
                    &name,
                    &choices,
                    heights,
                    model.as_ref().map(|(n, g)| (n.as_str(), g.as_ref())),
                )))
            }
            _ => None,
        };
    }

    /// Where the points go: one of the drawing's layers, or a new one, or why there is none (docs/adr/9329).
    fn gnss_target(&self, s: &State) -> Result<LayerTarget, String> {
        if let Some(id) = &s.target {
            return Ok(LayerTarget::Existing(id.clone()));
        }
        let Some(doc) = self.document.as_ref() else {
            return Err(String::new());
        };
        new_layer::target(&doc.model, &s.name, point_layer_style(), "Hedef katman'dan")
    }

    /// Where the lines go when they go apart from the points: their new layer or one of the drawing's, or why there is
    /// none; none when there are no lines to take or they go with the points.
    fn gnss_lines_target(&self, s: &State) -> Option<Result<LayerTarget, String>> {
        if s.plan.as_ref().is_none_or(|p| p.lines.is_empty()) {
            return None;
        }
        match &s.lines_to {
            LinesTo::Points => None,
            LinesTo::Layer(id) => Some(Ok(LayerTarget::Existing(id.clone()))),
            LinesTo::New => {
                let doc = self.document.as_ref()?;
                if s.target.is_none() && apply::same_layer_name(&s.lines_name, &s.name) {
                    return Some(Err(
                        "Çizgilerin katmanının adı noktalarınkiyle aynı; başka bir ad yazın ya da Noktaların katmanı'nı seçin."
                            .to_owned(),
                    ));
                }
                let style = kentos_commands::LayerStyle { color: "ink".into(), ..kentos_domain::default_style() };
                Some(new_layer::target(&doc.model, &s.lines_name, style, "Çizgilerin katmanı'ndan"))
            }
        }
    }

    /// Why the import cannot run now, if a reason must be said (a locked layer).
    fn gnss_locked(&self, s: &State) -> Option<String> {
        let doc = self.document.as_ref()?;
        let lines = self.gnss_lines_target(s).and_then(Result::ok);
        [self.gnss_target(s).ok(), lines].into_iter().flatten().find_map(|t| match t {
            LayerTarget::Existing(id) if doc.model.layers().is_locked(&id) => Some(format!(
                "“{}” katmanı kilitli; kilidini Katmanlar panelinden açın ya da başka bir katman seçin.",
                self.layer_name(&id)
            )),
            _ => None,
        })
    }

    fn gnss_can_import(&self, s: &State) -> bool {
        !s.reading
            && s.plan
                .as_ref()
                .is_some_and(|p| !p.placed.is_empty() || !p.lines.is_empty())
            && self.gnss_target(s).is_ok()
            && self.gnss_lines_target(s).is_none_or(|t| t.is_ok())
            && self.gnss_locked(s).is_none()
    }

    fn gnss_import_run(&mut self) {
        let Some(Window::GnssImport(s)) = &self.exchange else {
            return;
        };
        if !self.gnss_can_import(s) {
            return;
        }
        let (Ok(target), Some(plan)) = (self.gnss_target(s), s.plan.clone()) else {
            return;
        };
        let lines_target = match self.gnss_lines_target(s) {
            Some(Ok(t)) => Some(t),
            Some(Err(_)) => return,
            None => None,
        };
        let name = s.file.name.clone();
        let file = s.file.clone();
        let report = self.gnss_report(s);
        let mut made = gnss::entities(&plan.placed);
        let mut lines = gnss::line_entities(&plan.lines);
        if lines_target.is_some() {
            for e in &mut lines {
                e.base_mut().layer_id = LINES_KEY.to_owned();
            }
        }
        made.extend(lines);
        let named = |app: &App, t: &LayerTarget| match t {
            LayerTarget::New { name, .. } => name.clone(),
            LayerTarget::Existing(id) => app.layer_name(id),
        };
        let layer_name = named(self, &target);
        let lines_name = lines_target.as_ref().map(|t| named(self, t));
        let mut layers = vec![(String::new(), target)];
        layers.extend(lines_target.map(|t| (LINES_KEY.to_owned(), t)));
        let import = ImportPlan {
            source_srid: None,
            label: format!("GNSS: {name}"),
            layers,
            group: None,
        };
        let driver = new_layer::format_word(&file.name);
        let origin = match (&file.path, &driver) {
            (Some(path), Some(driver)) => {
                Some(apply::FileOrigin { driver, path, bytes: &file.bytes })
            }
            _ => None,
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        match apply::apply_file_import(&mut doc.model, made, &import, origin) {
            Err(error) => {
                if let Some(Window::GnssImport(s)) = &mut self.exchange {
                    s.status = Some((true, error));
                }
            }
            Ok(applied) => {
                self.zoom_to(&applied.slots);
                let (n, l) = (plan.placed.len(), plan.lines.len());
                let what = match (n, l, &lines_name) {
                    (n, 0, _) => format!("{n} GNSS noktası “{layer_name}” katmanına"),
                    (0, l, Some(to)) => format!("{l} GNSS çizgisi “{to}” katmanına"),
                    (0, l, None) => format!("{l} GNSS çizgisi “{layer_name}” katmanına"),
                    (n, l, Some(to)) => format!("{n} GNSS noktası “{layer_name}”, {l} çizgi “{to}” katmanına"),
                    (n, l, None) => format!("{n} GNSS noktası ve {l} çizgi “{layer_name}” katmanına"),
                };
                let made = match applied.created.len() {
                    0 => String::new(),
                    1 => " (yeni katman)".to_owned(),
                    k => format!(" ({k} yeni katman)"),
                };
                self.say(Level::Success, format!("“{name}”: {what} alındı{made}. Tek adımda geri alınabilir."));
                // What the preview said stays in the log (docs/adr/9329): the transformation, the heights, the records
                // not taken with why.
                for (warn, line) in report {
                    if warn {
                        self.warn(line);
                    } else {
                        self.say(Level::Info, line);
                    }
                }
                self.close_exchange();
            }
        }
    }

    pub(super) fn gnss_import_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let meta = match (&s.read, &s.failed) {
            (Some(r), _) => format!(
                "{}, {} konum, {}",
                format_name(&r.format),
                r.points.len(),
                r.encoding
            ),
            (None, Some(_)) => "okunamadı".to_owned(),
            (None, None) => "okunuyor…".to_owned(),
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&s.file.name, meta))
            .push(self.gnss_options(s))
            .push(self.gnss_table(s))
            .push(self.gnss_summary(s))
            .push(self.gnss_layer(s));
        let status = self
            .gnss_locked(s)
            .map(|why| (true, why))
            .or_else(|| s.status.clone());
        if let Some((error, text)) = status {
            body = body.push(words::text_line(
                if error { Line::Error } else { Line::Info },
                text,
            ));
        }
        let can = self.gnss_can_import(s);
        overlay::blocking(
            Dialog::new("GNSS içe aktar")
                // The body scrolls; the buttons stay in view whatever the window's height.
                .scroll(body)
                .action(words::ghost("Başka dosya…", Some(event(Event::Another))))
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("İçe aktar", can.then(|| event(Event::Run))))
                .width(960.0)
                .max_height(820.0)
                .on_close(Some(message(Exchange::Close))),
        )
    }

    fn gnss_options<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let mut parts = Row::new().spacing(24);
        if let Some(r) = &s.read {
            let kinds = counts(r);
            if !kinds.is_empty() {
                let mut boxes = Row::new().spacing(18);
                for (kind, name, n) in kinds {
                    boxes = boxes.push(words::check(
                        !s.off.contains(kind),
                        format!("{name} ({n})"),
                        Some(event(Event::Kind(kind))),
                    ));
                }
                parts = parts.push(words::field("Alınacak noktalar", boxes, None));
            }
            // The routes and tracks as lines (docs/adr/0222 §6).
            if !r.lines.is_empty() {
                let what = match r.format.as_str() {
                    "nmea" => "Kaydın izi".to_owned(),
                    _ => format!("Rota ve izler ({})", r.lines.len()),
                };
                parts = parts.push(words::field(
                    "Çizgiler",
                    words::check(s.lines, what, Some(event(Event::Lines))),
                    None,
                ));
            }
        }
        let prefix = kentos_ui::widget::text_field("Ön ek", &s.prefix)
            .on_input(|t| event(Event::Prefix(t)))
            .width(96);
        let start = kentos_ui::widget::text_field("İlk numara", &s.start_text)
            .on_input(|t| event(Event::Start(t)))
            .width(80);
        parts = parts.push(
            container(words::field(
                "Adsız noktalar",
                row![prefix, start].spacing(6),
                Some(
                    "Dosyada adı olmayan noktalar ön ek ve sırayla artan numarayla adlanır."
                        .to_owned(),
                ),
            ))
            .width(Fill),
        );
        parts.into()
    }

    fn gnss_table<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = &s.read else {
            return words::empty(if s.failed.is_some() {
                "Önizleme yok."
            } else {
                "Dosya okunuyor…"
            });
        };
        let Some(plan) = &s.plan else {
            return words::empty("Önizleme yok: konumlar projenin sistemine çevrilemiyor.");
        };
        if plan.placed.is_empty() {
            return words::empty(if r.points.is_empty() {
                "Dosyada konum yok."
            } else {
                "Seçili türde nokta yok."
            });
        }
        let format = self.number_format();
        let geographic = self.gnss_system().is_some_and(|n| n.geographic());
        let (first, second) = if geographic {
            ("Enlem".to_owned(), "Boylam".to_owned())
        } else {
            (format.axes_text("Y (sağa)"), format.axes_text("X (yukarı)"))
        };
        let place = |p: &Placed| -> (String, String) {
            if geographic {
                (format_dd(p.p.y, true, 9), format_dd(p.p.x, false, 9))
            } else {
                (format.coord(p.p.x), format.coord(p.p.y))
            }
        };
        let attr = |p: &Placed, key: &str| p.attrs.get(key).cloned().unwrap_or_default();
        let rows = plan.placed.iter().take(PREVIEW_ROWS).map(|p| {
            let (a, b) = place(p);
            TableRow::new([
                label::caption(p.line.to_string()).into(),
                label::body(p.name.clone()).into(),
                label::caption(attr(p, "Kaynak")).into(),
                label::mono(a).into(),
                label::mono(b).into(),
                label::mono(p.z.map_or_else(|| "—".to_owned(), |z| fixed(z, 3))).into(),
                label::caption(attr(p, "Çözüm")).into(),
                label::mono(attr(p, "Uydu")).into(),
                label::mono(attr(p, "HDOP")).into(),
                label::caption(attr(p, "Zaman")).into(),
            ])
        });
        // Fixed widths but the time's: a name, a coordinate and the receiver's
        // time whole at the smallest window (1100×650).
        Table::new([
            TableColumn::new("Satır").width(40).align_right(),
            TableColumn::new("Ad").width(88),
            TableColumn::new("Kaynak").width(104),
            TableColumn::new(first).width(96).align_right(),
            TableColumn::new(second).width(104).align_right(),
            TableColumn::new("Kot (m)").width(76).align_right(),
            TableColumn::new("Çözüm").width(72),
            TableColumn::new("Uydu").width(40).align_right(),
            TableColumn::new("HDOP").width(44).align_right(),
            TableColumn::new("Zaman").width(Length::Fill),
        ])
        .extend(rows)
        .into()
    }

    /// Whether the file's positions lie in the project system's area of use (GIS-034, docs/adr/1089): how many do
    /// not, the system's zone and their longitudes; for a system whose area is not known, that it was not looked at.
    pub(super) fn gnss_zone_line<'a>(
        &self,
        points: &[kentos_commands::GnssPoint],
    ) -> Option<Element<'a, Message>> {
        self.gnss_zone_words(points).map(|(kind, words)| words::text_line(kind, words))
    }

    /// [`Self::gnss_zone_line`]'s words and kind.
    fn gnss_zone_words(&self, points: &[kentos_commands::GnssPoint]) -> Option<(Line, String)> {
        if points.is_empty() {
            return None;
        }
        let srid = self.document.as_ref()?.settings().srid;
        let Some(area) = kentos_project::crs::area(srid) else {
            return Some((
                Line::Info,
                "Projenin sisteminin kullanım alanı bilinmiyor; konumların dilime uygunluğu denetlenmedi.".to_owned(),
            ));
        };
        let out: Vec<&kentos_commands::GnssPoint> =
            points.iter().filter(|p| !area.holds_near(p.lon, p.lat)).collect();
        if out.is_empty() {
            return None;
        }
        let (least, most) = out
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(p.lon), b.max(p.lon)));
        let deg = |v: f64| format!("{v:.3}°");
        // The registry's own words for the zone (“28.5°–31.5° D (3° dilim)”) before EPSG's English name.
        let zone = kentos_project::crs::system(srid)
            .and_then(|s| s.area.clone())
            .unwrap_or_else(|| area.name.clone());
        Some((
            Line::Warn,
            format!(
                "{} konum sistemin kullanım alanının dışında: boylamları {}–{}, sistemin alanı {zone}. Sayısal dönüşüm yapılır ama bu dilim o konumlar için seçilmemiş olabilir; projenin sistemini denetleyin.",
                out.len(),
                deg(least),
                deg(most),
            ),
        ))
    }

    /// What the preview said, for the log once the points are in (docs/adr/9329): the transformation and its accuracy,
    /// what the elevations are, the positions outside the system's zone and the records not taken with why; a warning
    /// is marked.
    fn gnss_report(&self, s: &State) -> Vec<(bool, String)> {
        let (Some(read), Some(plan)) = (&s.read, &s.plan) else {
            return Vec::new();
        };
        let name = &s.file.name;
        let mut out = Vec::new();
        if let Some(way) = way_of(plan) {
            out.push((way.ends_with("resmî dönüşüm değil"), format!("“{name}”: dönüşüm {way}.")));
        }
        if let Some(note) = heights_note(plan) {
            out.push((false, format!("“{name}”: {note}")));
        }
        if let Some((kind, words)) = self.gnss_zone_words(&read.points) {
            out.push((kind == Line::Warn, format!("“{name}”: {words}")));
        }
        let left: Vec<&LineError> = read.problems.iter().chain(&plan.skipped).chain(&plan.lines_skipped).collect();
        if !left.is_empty() {
            let mut reasons: Vec<String> =
                left.iter().take(SHOWN_PROBLEMS).map(|e| e.message.trim_end_matches('.').to_owned()).collect();
            if left.len() > SHOWN_PROBLEMS {
                reasons.push(format!("… ve {} kayıt daha", left.len() - SHOWN_PROBLEMS));
            }
            out.push((true, format!("“{name}”: {} kayıt alınmadı: {}.", left.len(), reasons.join("; "))));
        }
        out
    }

    fn gnss_summary<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = &s.read else {
            return words::summary(vec![match &s.failed {
                Some(e) => words::text_line(Line::Error, e.clone()),
                None => words::text_line(Line::Info, "Dosya okunuyor…"),
            }]);
        };
        let mut lines = Vec::new();
        match (self.gnss_system(), &s.plan) {
            (None, _) => lines.push(words::text_line(
                Line::Error,
                "Projenin koordinat sistemi yok: GNSS konumları WGS 84'tedir ve yerel sisteme çevrilemez. Proje ayarlarından bir koordinat sistemi seçin ya da özel sistem tanımlayın.",
            )),
            (Some(Named { name, system: None, .. }), _) => lines.push(words::text_line(
                Line::Error,
                format!(
                    "“{name}” sisteminin tanımı dönüşümlerde kullanılamıyor (yerel sistemin tabanı kayıttaki izdüşümlü bir sistem değil); GNSS konumları ona çevrilemez."
                ),
            )),
            (Some(named), Some(plan)) => {
                let n = plan.placed.len();
                let l = plan.lines.len();
                lines.push(match (n, l) {
                    (0, 0) if r.points.is_empty() => {
                        words::text_line(Line::Warn, "Dosyada alınacak konum yok.")
                    }
                    (0, 0) => words::text_line(
                        Line::Warn,
                        "Alınacak nokta ya da çizgi yok; türlerden en az birini ya da çizgileri seçin.",
                    ),
                    (n, 0) => words::text_line(Line::Ok, format!("{n} nokta alınacak.")),
                    (0, l) => words::text_line(Line::Ok, format!("{l} çizgi (rota ya da iz) alınacak.")),
                    (n, l) => words::text_line(
                        Line::Ok,
                        format!("{n} nokta ve {l} çizgi (rota ya da iz) alınacak."),
                    ),
                });
                if let Some(way) = way_of(plan) {
                    let kind = if way.ends_with("resmî dönüşüm değil") {
                        Line::Warn
                    } else {
                        Line::Info
                    };
                    lines.push(words::text_line(kind, format!("Dönüşüm: {way}.")));
                }
                if let Some(note) = heights_note(plan) {
                    lines.push(words::text_line(Line::Info, note));
                }
                if !plan.skipped.is_empty() {
                    lines.push(problems(
                        format!(
                            "{} nokta projenin sistemine çevrilemedi; alınmayacak:",
                            plan.skipped.len()
                        ),
                        &plan.skipped,
                    ));
                }
                if !plan.lines_skipped.is_empty() {
                    lines.push(problems(
                        format!("{} çizgi alınmayacak:", plan.lines_skipped.len()),
                        &plan.lines_skipped,
                    ));
                }
                if n > 0 && !named.geographic() {
                    lines.push(words::text_line(
                        Line::Info,
                        words::extent_text(&self.number_format(), &bounds(&plan.placed)),
                    ));
                }
                // The target zone (GIS-034): points outside the project system's EPSG area of use are said with the
                // zone and the points' longitudes, apart from the transformation's accuracy.
                if let Some(line) = self.gnss_zone_line(&r.points) {
                    lines.push(line);
                }
            }
            (Some(_), None) => {}
        }
        if !r.problems.is_empty() {
            lines.push(problems(
                format!("Dosyanın {} kaydı okunmadı:", r.problems.len()),
                &r.problems,
            ));
        }
        words::summary(lines)
    }

    fn gnss_layer<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(doc) = &self.document else {
            return label::caption("").into();
        };
        let layers = doc.model.layers();
        let leaves: Vec<String> = layers.leaves().iter().map(|l| l.id.clone()).collect();
        let named = |id: &String| {
            let mark = if layers.is_locked(id) {
                " (kilitli)"
            } else if !layers.is_visible(id) {
                " (gizli)"
            } else {
                ""
            };
            Choice::new(format!("{}{mark}", self.layer_path(id)))
        };
        let mut choices = vec![Choice::new("Yeni katman")];
        choices.extend(leaves.iter().map(named));
        let selected = match &s.target {
            None => Some(0),
            Some(id) => leaves.iter().position(|l| l == id).map(|i| i + 1),
        };
        let points_leaves = leaves.clone();
        let pick = Select::new(choices, selected, move |i| {
            event(Event::Target(i.checked_sub(1).and_then(|k| points_leaves.get(k).cloned())))
        });
        let hidden = s
            .target
            .as_ref()
            .is_some_and(|id| !layers.is_visible(id))
            .then(|| "Katman gizli: noktalar alınır ama görünmez.".to_owned());
        let mut parts = row![
            container(words::field("Hedef katman", pick, hidden)).width(Length::FillPortion(3))
        ]
        .spacing(16);
        if s.target.is_none() {
            let name = kentos_ui::widget::text_field("Yeni katmanın adı", &s.name)
                .on_input(|t| event(Event::Name(t)))
                .on_submit(event(Event::Run));
            // A name a layer has is said with the way to it (docs/adr/9329).
            let help = self.gnss_target(s).err().filter(|why| !why.is_empty()).unwrap_or_else(|| new_layer::help("Hedef katman'dan"));
            parts = parts.push(
                container(words::field("Yeni katmanın adı", name, Some(help))).width(Length::FillPortion(2)),
            );
        }
        let has_lines = s.plan.as_ref().is_some_and(|p| !p.lines.is_empty());
        if !has_lines {
            return parts.into();
        }
        // The routes and tracks: a layer of their own unless they go with the points (docs/adr/9329).
        let mut choices = vec![Choice::new("Yeni katman"), Choice::new("Noktaların katmanı")];
        choices.extend(leaves.iter().map(named));
        let selected = match &s.lines_to {
            LinesTo::New => Some(0),
            LinesTo::Points => Some(1),
            LinesTo::Layer(id) => leaves.iter().position(|l| l == id).map(|i| i + 2),
        };
        let pick = Select::new(choices, selected, move |i| {
            event(Event::LinesTo(match i {
                0 => LinesTo::New,
                1 => LinesTo::Points,
                k => leaves.get(k - 2).cloned().map_or(LinesTo::New, LinesTo::Layer),
            }))
        });
        let mut lines = row![
            container(words::field(
                "Çizgilerin katmanı",
                pick,
                Some("Rotalar ve izler noktalardan ayrı bir katmanda durur.".to_owned()),
            ))
            .width(Length::FillPortion(3))
        ]
        .spacing(16);
        if s.lines_to == LinesTo::New {
            let name = kentos_ui::widget::text_field("Çizgilerin yeni katmanının adı", &s.lines_name)
                .on_input(|t| event(Event::LinesName(t)))
                .on_submit(event(Event::Run));
            let help = match self.gnss_lines_target(s) {
                Some(Err(why)) if !why.is_empty() => why,
                _ => new_layer::help("Çizgilerin katmanı'ndan"),
            };
            lines = lines.push(
                container(words::field("Çizgilerin yeni katmanının adı", name, Some(help)))
                    .width(Length::FillPortion(2)),
            );
        }
        column![parts, lines].spacing(10).into()
    }
}

/// The way the positions were moved into the project's system, as the plan's first object records it.
fn way_of(plan: &Plan) -> Option<String> {
    plan.placed
        .first()
        .map(|p| &p.attrs)
        .or_else(|| plan.lines.first().map(|l| &l.attrs))
        .and_then(|a| a.get("Dönüşüm"))
        .cloned()
}

/// What the points' elevations are (docs/adr/0519 §6): the plan's words when the project says what its heights are,
/// else that they are ellipsoidal heights; none without points.
fn heights_note(plan: &Plan) -> Option<String> {
    if plan.placed.is_empty() {
        return None;
    }
    if let Some(note) = &plan.heights {
        return Some(note.clone());
    }
    let bare = plan.placed.iter().filter(|p| p.z.is_none()).count();
    let heights = "Kot elipsoit yüksekliğidir (dosyadaki yükseklik ile geoit ayrımının toplamı); ortometrik yüksekliğe çevrilmez.";
    Some(if bare > 0 {
        format!("{heights} {bare} noktanın elipsoit yüksekliği bilinmiyor: kotu boş, dosyadaki yüksekliği öznitelikte.")
    } else {
        heights.to_owned()
    })
}

/// A warning line with the first problems and how many more.
fn problems<'a>(title: String, items: &[LineError]) -> Element<'a, Message> {
    let shown: Vec<Element<'a, Message>> = items
        .iter()
        .take(SHOWN_PROBLEMS)
        .map(|e| label::caption(format!("• {}", e.message)).into())
        .collect();
    let mut list = Column::with_children(shown).spacing(2);
    if items.len() > SHOWN_PROBLEMS {
        list = list.push(label::caption(format!(
            "• … ve {} kayıt daha.",
            items.len() - SHOWN_PROBLEMS
        )));
    }
    words::line(Line::Warn, column![label::body(title), list].spacing(4))
}

#[cfg(test)]
impl State {
    /// The points as they will be written, with the current choices.
    pub(super) fn plan(&self) -> Option<&Plan> {
        self.plan.as_deref()
    }

    /// How many of the file's records were not read.
    pub(super) fn problems(&self) -> usize {
        self.read.as_ref().map_or(0, |r| r.problems.len())
    }

    /// The new layers' names offered: the points', the lines'.
    pub(super) fn name_for_test(&self) -> &str {
        &self.name
    }

    pub(super) fn lines_name_for_test(&self) -> &str {
        &self.lines_name
    }

    /// The file's positions as read (WGS 84).
    #[cfg(test)]
    pub(super) fn positions(&self) -> Vec<kentos_commands::GnssPoint> {
        self.read.as_ref().map_or_else(Vec::new, |r| r.points.clone())
    }
}
