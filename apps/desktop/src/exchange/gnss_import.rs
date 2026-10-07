//! GNSS içe aktar (docs/adr/0169 §6; the web's `ui/io/GnssImportDialog.ts`):
//! a receiver's GPX or NMEA file, read off the UI thread by the shared reader
//! (`kentos_formats::gnss`), its positions moved from WGS 84 into the
//! project's system the way Koordinat dönüştür moves them, with the accuracy
//! and what it rests on said (docs/adr/0167; the rules are
//! `kentos_interaction::gnss`). The kinds of points to take and how the
//! unnamed are named are chosen; the points show as they will be written. A
//! project without a system takes none: GNSS positions have one, and they are
//! never placed without it (CLAUDE.md §5). The points go in as one undo step,
//! onto a new layer or one of the drawing's.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, Row, column, container, row, text_input};
use iced::{Element, Fill, Length, Task};
use kentos_contracts::{Bounds, GnssRead, LineError};
use kentos_geometry_core::crs::format_dd;
use kentos_interaction::gnss::{self, Options, Placed, Plan};
use kentos_interaction::{Level, fixed};
use kentos_project::systems::{self, Named};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::{Dialog, overlay};

use super::apply::{self, ImportPlan, LayerTarget, layer_named};
use super::coord_import::point_layer_style;
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
    prefix: String,
    /// The first number as typed; the last good one stands while it is not one.
    start_text: String,
    start: u32,
    /// The target layer's id; `None`: a new layer named `name`.
    target: Option<String>,
    name: String,
    status: Option<(bool, String)>,
    /// The points as they will be written, with the current choices; none
    /// while the file is read or the project has no system to take them.
    plan: Option<Arc<Plan>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Read { id: u64, read: Arc<GnssRead> },
    Kind(&'static str),
    Prefix(String),
    Start(String),
    Target(Option<String>),
    Name(String),
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
        let state = State {
            name: words::base_name(&file.name),
            file,
            read: None,
            failed: None,
            reading_id: id,
            reading: true,
            off: BTreeSet::new(),
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
                };
                Some(Arc::new(gnss::place(
                    &read.points,
                    &options,
                    &to,
                    &name,
                    &choices,
                )))
            }
            _ => None,
        };
    }

    /// Where the points go: an existing layer, or a new one (merged into a
    /// layer of the same name if there is one).
    fn gnss_target(&self, s: &State) -> Option<LayerTarget> {
        if let Some(id) = &s.target {
            return Some(LayerTarget::Existing(id.clone()));
        }
        let name = s.name.trim();
        if name.is_empty() {
            return None;
        }
        let doc = self.document.as_ref()?;
        Some(match layer_named(&doc.model, name) {
            Some(same) => LayerTarget::Existing(same.id.clone()),
            None => LayerTarget::New {
                name: name.to_owned(),
                style: Box::new(point_layer_style()),
                visible: true,
                locked: false,
                fields: Vec::new(),
            },
        })
    }

    /// Why the import cannot run now, if a reason must be said (a locked layer).
    fn gnss_locked(&self, s: &State) -> Option<String> {
        let Some(LayerTarget::Existing(id)) = self.gnss_target(s) else {
            return None;
        };
        let doc = self.document.as_ref()?;
        doc.model.layers().is_locked(&id).then(|| {
            format!(
                "“{}” katmanı kilitli; kilidini Katmanlar panelinden açın ya da başka bir katman seçin.",
                self.layer_name(&id)
            )
        })
    }

    fn gnss_can_import(&self, s: &State) -> bool {
        !s.reading
            && s.plan.as_ref().is_some_and(|p| !p.placed.is_empty())
            && self.gnss_target(s).is_some()
            && self.gnss_locked(s).is_none()
    }

    fn gnss_import_run(&mut self) {
        let Some(Window::GnssImport(s)) = &self.exchange else {
            return;
        };
        if !self.gnss_can_import(s) {
            return;
        }
        let (Some(target), Some(plan)) = (self.gnss_target(s), s.plan.clone()) else {
            return;
        };
        let name = s.file.name.clone();
        let problems = s.read.as_ref().map_or(0, |r| r.problems.len());
        let layer_name = match &target {
            LayerTarget::New { name, .. } => name.clone(),
            LayerTarget::Existing(id) => self.layer_name(id),
        };
        let import = ImportPlan {
            label: format!("GNSS: {name}"),
            layers: vec![(String::new(), target)],
            group: None,
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        match apply::apply_import(
            &mut doc.model,
            gnss::entities(&plan.placed),
            Vec::new(),
            &import,
        ) {
            Err(error) => {
                if let Some(Window::GnssImport(s)) = &mut self.exchange {
                    s.status = Some((true, error));
                }
            }
            Ok(applied) => {
                self.zoom_to(&applied.slots);
                self.say(
                    Level::Success,
                    format!(
                        "“{name}”: {} GNSS noktası “{layer_name}” katmanına alındı{}. Tek adımda geri alınabilir.",
                        applied.slots.len(),
                        if applied.created.is_empty() { "" } else { " (yeni katman)" }
                    ),
                );
                let left = plan.skipped.len() + problems;
                if left > 0 {
                    self.warn(format!(
                        "“{name}”: {left} kayıt alınmadı (okunamayan ya da projenin sistemine çevrilemeyen); nedenleri içe aktarma penceresinde yazılıydı."
                    ));
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
                .max_height(820.0),
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
        }
        let prefix = kentos_ui::widget::focus_ring(
            text_input("Ön ek", &s.prefix)
                .on_input(|t| event(Event::Prefix(t)))
                .padding([5, 8])
                .width(96)
                .size(kentos_ui::theme::typography::body())
                .style(style::field::input),
        );
        let start = kentos_ui::widget::focus_ring(
            text_input("İlk numara", &s.start_text)
                .on_input(|t| event(Event::Start(t)))
                .padding([5, 8])
                .width(80)
                .size(kentos_ui::theme::typography::body())
                .style(style::field::input),
        );
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
                lines.push(if n > 0 {
                    words::text_line(Line::Ok, format!("{n} nokta alınacak."))
                } else if r.points.is_empty() {
                    words::text_line(Line::Warn, "Dosyada alınacak konum yok.")
                } else {
                    words::text_line(
                        Line::Warn,
                        "Alınacak nokta yok; türlerden en az birini seçin.",
                    )
                });
                if let Some(way) = plan.placed.first().and_then(|p| p.attrs.get("Dönüşüm")) {
                    let kind = if way.ends_with("resmî dönüşüm değil") {
                        Line::Warn
                    } else {
                        Line::Info
                    };
                    lines.push(words::text_line(kind, format!("Dönüşüm: {way}.")));
                }
                if n > 0 {
                    let bare = plan.placed.iter().filter(|p| p.z.is_none()).count();
                    let heights = "Kot elipsoit yüksekliğidir (dosyadaki yükseklik ile geoit ayrımının toplamı); ortometrik yüksekliğe çevrilmez.";
                    lines.push(words::text_line(
                        Line::Info,
                        if bare > 0 {
                            format!(
                                "{heights} {bare} noktanın elipsoit yüksekliği bilinmiyor: kotu boş, dosyadaki yüksekliği öznitelikte."
                            )
                        } else {
                            heights.to_owned()
                        },
                    ));
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
                if n > 0 && !named.geographic() {
                    lines.push(words::text_line(
                        Line::Info,
                        words::extent_text(&self.number_format(), &bounds(&plan.placed)),
                    ));
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
        let mut choices = vec![Choice::new("Yeni katman")];
        choices.extend(leaves.iter().map(|id| {
            let mark = if layers.is_locked(id) {
                " (kilitli)"
            } else if !layers.is_visible(id) {
                " (gizli)"
            } else {
                ""
            };
            Choice::new(format!("{}{mark}", self.layer_path(id)))
        }));
        let selected = match &s.target {
            None => Some(0),
            Some(id) => leaves.iter().position(|l| l == id).map(|i| i + 1),
        };
        let pick = Select::new(choices, selected, move |i| {
            event(Event::Target(
                i.checked_sub(1).and_then(|k| leaves.get(k).cloned()),
            ))
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
            let name = text_input("Yeni katmanın adı", &s.name)
                .on_input(|t| event(Event::Name(t)))
                .on_submit(event(Event::Run))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(style::field::input);
            let name = kentos_ui::widget::focus_ring(name);
            parts = parts.push(
                container(words::field(
                    "Yeni katmanın adı",
                    name,
                    Some("Bu adda bir katman varsa noktalar ona eklenir.".to_owned()),
                ))
                .width(Length::FillPortion(2)),
            );
        }
        parts.into()
    }
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
}
