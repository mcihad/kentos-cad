//! Koordinat listesi içe aktar: Netcad NCN, TXT, CSV (the web's
//! `ui/io/CoordImportDialog.ts`). The shared reader reads the file off the
//! UI thread; the window shows what it found (delimiter, decimal mark,
//! header, what each column holds) with the first rows, and every choice
//! reads the file again. The coordinate system is asked, the project's by
//! default; any other blocks the import (no silent reprojection, CLAUDE.md
//! §5). The points go in as one undo step.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, Row, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    CoordColumn, CoordDelimiter, CoordRead, CoordReadOptions, DecimalMark, HeaderMode,
    ImportResult, LabelPlacement, LabelStyle, LayerStyle, PointStyle, PointSymbol,
};
use kentos_interaction::{Format, Level};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};
use kentos_ui::{label, style};

use super::apply::{self, ImportPlan, LayerTarget, layer_named};
use super::crs::CrsQuestion;
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Kind, Picked, Window, message, off_thread};
use crate::app::{App, Message};

/// Rows of the preview table.
const PREVIEW_ROWS: u32 = 12;

static READS: AtomicU64 = AtomicU64::new(0);

/// What each column may hold, in the web's order.
const ROLES: [CoordColumn; 6] = [
    CoordColumn::Name,
    CoordColumn::Y,
    CoordColumn::X,
    CoordColumn::Z,
    CoordColumn::Code,
    CoordColumn::Skip,
];

fn role_label(c: CoordColumn) -> &'static str {
    match c {
        CoordColumn::Name => "Ad",
        CoordColumn::Y => "Y (sağa)",
        CoordColumn::X => "X (yukarı)",
        CoordColumn::Z => "Z (kot)",
        CoordColumn::Code => "Kod",
        CoordColumn::Skip => "Alınmaz",
    }
}

fn delimiter_label(d: CoordDelimiter) -> &'static str {
    match d {
        CoordDelimiter::Auto => "Otomatik",
        CoordDelimiter::Space => "Boşluk",
        CoordDelimiter::Tab => "Sekme",
        CoordDelimiter::Semicolon => "Noktalı virgül (;)",
        CoordDelimiter::Comma => "Virgül (,)",
    }
}

const DELIMITERS: [CoordDelimiter; 5] = [
    CoordDelimiter::Auto,
    CoordDelimiter::Space,
    CoordDelimiter::Tab,
    CoordDelimiter::Semicolon,
    CoordDelimiter::Comma,
];

/// Column orders survey files use; Netcad's is first (the web's `COORD_ORDERS`).
pub const ORDERS: [(&str, &[CoordColumn]); 4] = [
    (
        "Ad Y X Z",
        &[
            CoordColumn::Name,
            CoordColumn::Y,
            CoordColumn::X,
            CoordColumn::Z,
        ],
    ),
    (
        "Ad X Y Z",
        &[
            CoordColumn::Name,
            CoordColumn::X,
            CoordColumn::Y,
            CoordColumn::Z,
        ],
    ),
    ("Y X Z", &[CoordColumn::Y, CoordColumn::X, CoordColumn::Z]),
    ("X Y Z", &[CoordColumn::X, CoordColumn::Y, CoordColumn::Z]),
];

/// An order laid over a file with `count` columns (the others are not read).
fn order_columns(order: &[CoordColumn], count: usize) -> Vec<CoordColumn> {
    (0..count.max(order.len()))
        .map(|i| order.get(i).copied().unwrap_or(CoordColumn::Skip))
        .collect()
}

/// The order `columns` follow, if they are one of the known ones.
fn order_of(columns: &[CoordColumn]) -> Option<usize> {
    let used = columns.iter().filter(|c| **c != CoordColumn::Skip).count();
    ORDERS.iter().position(|(_, order)| {
        order.len() == used
            && order
                .iter()
                .enumerate()
                .all(|(i, c)| columns.get(i) == Some(c))
    })
}

/// A segment of the column order control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Order(usize);

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(ORDERS.get(self.0).map_or("", |(name, _)| name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mark(DecimalMark);

impl fmt::Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            DecimalMark::Comma => "Virgül",
            _ => "Nokta",
        })
    }
}

/// The look of a layer made for imported points: a cross with the name
/// beside it (the web's `POINT_LAYER_STYLE`).
pub fn point_layer_style() -> LayerStyle {
    LayerStyle {
        color: "ink".into(),
        point: Some(PointStyle {
            symbol: PointSymbol::Cross,
            size: 8.0,
        }),
        label: Some(LabelStyle {
            placement: LabelPlacement::Beside,
            size: 10.5,
            grow: None,
            max_size: None,
            weight: None,
            template: None,
            min_feature_px: None,
            min_scale: Some(0.9),
            max_scale: None,
            ink: None,
        }),
        ..kentos_domain::default_style()
    }
}

#[derive(Debug, Clone)]
pub struct State {
    file: Picked,
    delimiter: CoordDelimiter,
    decimal: DecimalMark,
    header: HeaderMode,
    /// What each column holds once the user chose; empty lets the reader suggest.
    columns: Vec<CoordColumn>,
    read: Option<Arc<CoordRead>>,
    failed: Option<String>,
    reading_id: u64,
    reading: bool,
    importing: bool,
    /// The target layer's id; `None`: a new layer named `name`.
    target: Option<String>,
    name: String,
    crs: CrsQuestion,
    status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Read {
        id: u64,
        read: Arc<CoordRead>,
    },
    Delimiter(CoordDelimiter),
    Decimal(DecimalMark),
    Header(bool),
    Order(usize),
    Column(usize, CoordColumn),
    Target(Option<String>),
    Name(String),
    Crs(u32),
    Another,
    Run,
    /// The full read for the import: its objects, or why not.
    Imported {
        id: u64,
        result: Result<ImportResult, String>,
    },
}

fn event(e: Event) -> Message {
    message(Exchange::CoordImport(e))
}

impl State {
    fn options(&self) -> CoordReadOptions {
        CoordReadOptions {
            delimiter: self.delimiter,
            decimal: self.decimal,
            header: self.header,
            columns: self.columns.clone(),
            preview_rows: PREVIEW_ROWS,
            entities: false,
        }
    }
}

/// Reads the file off the UI thread with `options`.
fn read(id: u64, bytes: Arc<[u8]>, options: CoordReadOptions) -> Task<Message> {
    off_thread(
        move || kentos_formats::coords::read(&bytes, &options),
        move |read| {
            Exchange::CoordImport(Event::Read {
                id,
                read: Arc::new(read),
            })
        },
    )
}

impl App {
    pub(super) fn coord_import_picked(&mut self, file: Picked) -> Task<Message> {
        let srid = self.project_srid();
        let (crs, target) = match &self.exchange {
            Some(Window::CoordImport(s)) => (s.crs, s.target.clone()),
            _ => (CrsQuestion::new(srid), None),
        };
        let id = READS.fetch_add(1, Ordering::Relaxed) + 1;
        let state = State {
            name: words::base_name(&file.name),
            file,
            delimiter: CoordDelimiter::Auto,
            decimal: DecimalMark::Auto,
            header: HeaderMode::Auto,
            columns: Vec::new(),
            read: None,
            failed: None,
            reading_id: id,
            reading: true,
            importing: false,
            target,
            crs,
            status: None,
        };
        let task = read(id, state.file.bytes.clone(), state.options());
        self.open_window(Window::CoordImport(state));
        task
    }

    pub(super) fn coord_import_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Another => return self.pick(Kind::Coords),
            Event::Run => return self.coord_import_run(),
            Event::Imported { id, result } => {
                self.coord_import_apply(id, result);
                return Task::none();
            }
            _ => {}
        }
        let Some(Window::CoordImport(s)) = &mut self.exchange else {
            return Task::none();
        };
        // Every choice about the file reads it again; a late answer to an older read is dropped.
        let mut again = false;
        match e {
            Event::Read { id, read } => {
                if id == s.reading_id {
                    s.reading = false;
                    s.failed = None;
                    s.read = Some(read);
                }
            }
            Event::Delimiter(d) => {
                s.delimiter = d;
                // Other fields, other columns: let the reader suggest again.
                s.columns.clear();
                again = true;
            }
            Event::Decimal(d) => {
                s.decimal = d;
                again = true;
            }
            Event::Header(on) => {
                s.header = if on { HeaderMode::Yes } else { HeaderMode::No };
                again = true;
            }
            Event::Order(i) => {
                let count = s.read.as_ref().map_or(0, |r| r.columns.len());
                if let Some((_, order)) = ORDERS.get(i) {
                    s.columns = order_columns(order, count);
                    again = true;
                }
            }
            Event::Column(i, role) => {
                if let Some(r) = &s.read {
                    let mut next = r.columns.clone();
                    if let Some(slot) = next.get_mut(i) {
                        *slot = role;
                    }
                    s.columns = next;
                    again = true;
                }
            }
            Event::Target(t) => s.target = t,
            Event::Name(name) => s.name = name,
            Event::Crs(srid) => s.crs.srid = srid,
            Event::Another | Event::Run | Event::Imported { .. } => {}
        }
        if again {
            let id = READS.fetch_add(1, Ordering::Relaxed) + 1;
            s.reading_id = id;
            s.reading = true;
            return read(id, s.file.bytes.clone(), s.options());
        }
        Task::none()
    }

    /// Where the points go: an existing layer, or a new one (merged into a
    /// layer of the same name if there is one).
    fn coord_target(&self, s: &State) -> Option<LayerTarget> {
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
            },
        })
    }

    /// Why the import cannot run now, if a reason must be said (a locked layer).
    fn coord_locked(&self, s: &State) -> Option<String> {
        let Some(LayerTarget::Existing(id)) = self.coord_target(s) else {
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

    fn coord_can_import(&self, s: &State) -> bool {
        !s.importing
            && !s.reading
            && s.read.as_ref().is_some_and(|r| r.points > 0)
            && s.crs.matches(self.project_srid())
            && self.coord_target(s).is_some()
            && self.coord_locked(s).is_none()
    }

    fn coord_import_run(&mut self) -> Task<Message> {
        let Some(Window::CoordImport(s)) = &self.exchange else {
            return Task::none();
        };
        if !self.coord_can_import(s) {
            return Task::none();
        }
        let Some(r) = s.read.clone() else {
            return Task::none();
        };
        // Exactly the choices the preview showed.
        let options = CoordReadOptions {
            delimiter: r.delimiter,
            decimal: r.decimal,
            header: if r.header {
                HeaderMode::Yes
            } else {
                HeaderMode::No
            },
            columns: r.columns.clone(),
            preview_rows: 0,
            entities: true,
        };
        let bytes = s.file.bytes.clone();
        let id = s.reading_id;
        if let Some(Window::CoordImport(s)) = &mut self.exchange {
            s.importing = true;
            s.status = Some((false, format!("{} nokta okunuyor…", r.points)));
        }
        off_thread(
            move || {
                let full = kentos_formats::coords::read(&bytes, &options);
                match full.result {
                    None => Err("okuyucu nesne döndürmedi".to_owned()),
                    Some(result) => match apply::unusable(&result.entities) {
                        Some((place, kind)) => Err(apply::unusable_text(place, kind)),
                        None => Ok(result),
                    },
                }
            },
            move |result| Exchange::CoordImport(Event::Imported { id, result }),
        )
    }

    fn coord_import_apply(&mut self, id: u64, result: Result<ImportResult, String>) {
        let Some(Window::CoordImport(s)) = &self.exchange else {
            return;
        };
        if id != s.reading_id {
            return;
        }
        let (Some(target), Some(read)) = (self.coord_target(s), s.read.clone()) else {
            return;
        };
        let name = s.file.name.clone();
        let result = match result {
            Ok(result) => result,
            Err(e) => {
                if let Some(Window::CoordImport(s)) = &mut self.exchange {
                    s.importing = false;
                    s.status = Some((true, format!("Noktalar okunamadı: {e}")));
                }
                return;
            }
        };
        let layer_name = match &target {
            LayerTarget::New { name, .. } => name.clone(),
            LayerTarget::Existing(id) => self.layer_name(id),
        };
        let plan = ImportPlan {
            label: format!("Koordinat listesi: {name}"),
            layers: vec![(String::new(), target)],
            group: None,
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        match apply::apply_import(&mut doc.model, result.entities, &plan) {
            Err(error) => {
                if let Some(Window::CoordImport(s)) = &mut self.exchange {
                    s.importing = false;
                    s.status = Some((true, error));
                }
            }
            Ok(applied) => {
                self.zoom_to(&applied.slots);
                self.say(
                    Level::Success,
                    format!(
                        "“{name}”: {} nokta “{layer_name}” katmanına alındı{}. Tek adımda geri alınabilir.",
                        applied.slots.len(),
                        if applied.created.is_empty() { "" } else { " (yeni katman)" }
                    ),
                );
                if read.error_count > 0 {
                    let lines: Vec<String> = read
                        .errors
                        .iter()
                        .take(3)
                        .map(|e| format!("satır {}", e.line))
                        .collect();
                    self.warn(format!(
                        "“{name}”: {} satır nokta olmadığı için alınmadı ({}{}).",
                        read.error_count,
                        lines.join(", "),
                        if read.error_count > 3 { " …" } else { "" }
                    ));
                }
                self.close_exchange();
            }
        }
    }

    pub(super) fn coord_import_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let srid = self.project_srid();
        let r = s.read.as_deref();
        let meta = match r {
            Some(r) => format!(
                "{} veri satırı, {}{}",
                r.data_lines,
                r.encoding,
                if r.header {
                    ", ilk satır başlık"
                } else {
                    ""
                }
            ),
            None => "okunuyor…".to_owned(),
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&s.file.name, meta))
            .push(self.coord_options(s))
            .push(self.coord_table(s))
            .push(self.coord_summary(s))
            .push(s.crs.view(srid, |srid| event(Event::Crs(srid))))
            .push(self.coord_layer(s));
        let status = self
            .coord_locked(s)
            .filter(|_| !s.importing)
            .map(|why| (true, why))
            .or_else(|| s.status.clone());
        if let Some((error, text)) = status {
            body = body.push(words::text_line(
                if error { Line::Error } else { Line::Info },
                text,
            ));
        }
        let can = self.coord_can_import(s);
        overlay::blocking(
            Dialog::new("Koordinat listesi içe aktar")
                .push(scrollable(body).height(Length::Shrink))
                .action(words::ghost("Başka dosya…", Some(event(Event::Another))))
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("İçe aktar", can.then(|| event(Event::Run))))
                .width(860.0),
        )
    }

    fn coord_options<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let r = s.read.as_deref();
        let delimiters = DELIMITERS.iter().map(|d| match (d, r) {
            (CoordDelimiter::Auto, Some(r)) => Choice::new(format!(
                "Otomatik: {}",
                delimiter_label(r.delimiter).to_lowercase()
            )),
            (d, _) => Choice::new(delimiter_label(*d)),
        });
        let current = DELIMITERS.iter().position(|d| *d == s.delimiter);
        let delimiter = Select::new(delimiters, current, |i| {
            event(Event::Delimiter(DELIMITERS[i.min(DELIMITERS.len() - 1)]))
        })
        .searchable(false);
        let resolved = r.map_or(DecimalMark::Point, |r| r.decimal);
        let comma_delimited = r.is_some_and(|r| r.delimiter == CoordDelimiter::Comma);
        let decimal = Segmented::new_with(
            [Mark(DecimalMark::Point), Mark(DecimalMark::Comma)],
            Mark(resolved),
            |m| event(Event::Decimal(m.0)),
            move |m| !(m.0 == DecimalMark::Comma && comma_delimited),
        );
        let header = words::check(
            r.is_some_and(|r| r.header),
            "Başlık (sütun adları)",
            Some(event(Event::Header(!r.is_some_and(|r| r.header)))),
        );
        let count = r.map_or(0, |r| r.columns.len());
        let order = Segmented::new_with(
            (0..ORDERS.len()).map(Order),
            Order(r.and_then(|r| order_of(&r.columns)).unwrap_or(usize::MAX)),
            |o| event(Event::Order(o.0)),
            move |o| count == 0 || ORDERS[o.0].1.len() <= count,
        );
        row![
            words::field("Ayırıcı", container(delimiter).width(210), None),
            words::field("Ondalık ayırıcı", decimal, None),
            words::field("İlk satır", header, None),
            words::field(
                "Sütun sırası",
                order,
                Some("Y sağa (doğu), X yukarı (kuzey) değerdir. Her sütunu tablonun başlığından da seçebilirsiniz.".to_owned()),
            ),
        ]
        .spacing(16)
        .into()
    }

    fn coord_table<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = s.read.as_deref() else {
            return words::empty(if s.failed.is_some() {
                "Önizleme yok."
            } else {
                "Dosya okunuyor…"
            });
        };
        if r.preview.is_empty() {
            return words::empty("Dosyada nokta satırı yok.");
        }
        const LINE: f32 = 52.0;
        const CELL: f32 = 128.0;
        let roles: Vec<Choice> = ROLES.iter().map(|c| Choice::new(role_label(*c))).collect();
        let mut head = Row::new()
            .spacing(8)
            .align_y(Center)
            .push(container(label::caption("Satır")).width(LINE));
        for (i, role) in r.columns.iter().enumerate() {
            let pick = Select::new(
                roles.clone(),
                ROLES.iter().position(|c| c == role),
                move |k| event(Event::Column(i, ROLES[k.min(ROLES.len() - 1)])),
            )
            .searchable(false);
            let mut cell = Column::new().spacing(2);
            if r.header {
                cell = cell.push(label::caption(
                    r.header_fields.get(i).cloned().unwrap_or_default(),
                ));
            }
            head = head.push(container(cell.push(pick)).width(CELL));
        }
        head = head.push(container(label::caption("Durum")).width(Fill));
        let mut rows = Column::new().spacing(4).push(head);
        for p in &r.preview {
            let mut line = Row::new()
                .spacing(8)
                .align_y(Center)
                .push(container(label::caption(p.line.to_string())).width(LINE));
            for (i, role) in r.columns.iter().enumerate() {
                let value = p.fields.get(i).cloned().unwrap_or_default();
                let cell: Element<'a, Message> = if *role == CoordColumn::Skip {
                    label::muted(value).into()
                } else {
                    label::mono(value).into()
                };
                line = line.push(container(cell).width(CELL));
            }
            let status: Element<'a, Message> = match &p.error {
                Some(error) => row![
                    icon(Icon::Warning).size(14.0).tone(Tone::Warning),
                    label::caption(error.clone()),
                ]
                .spacing(6)
                .align_y(Center)
                .into(),
                None => icon(Icon::Success).size(14.0).tone(Tone::Success).into(),
            };
            rows = rows.push(line.push(container(status).width(Fill)));
        }
        container(
            scrollable(rows)
                .direction(style::field::thin_scrollbar())
                .height(Length::Fixed(260.0)),
        )
        .padding(8)
        .width(Fill)
        .style(style::container::bordered)
        .into()
    }

    fn coord_summary<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = s.read.as_deref() else {
            return words::summary(vec![match &s.failed {
                Some(e) => words::text_line(Line::Error, e.clone()),
                None => words::text_line(Line::Info, "Dosya okunuyor…"),
            }]);
        };
        let mut lines = vec![if r.points > 0 {
            words::text_line(Line::Ok, format!("{} nokta alınacak.", r.points))
        } else {
            words::text_line(
                Line::Warn,
                "Alınacak nokta yok; ayırıcıyı ve sütunları denetleyin.",
            )
        }];
        if r.error_count > 0 {
            let shown: Vec<Element<'a, Message>> = r
                .errors
                .iter()
                .take(5)
                .map(|e| label::caption(format!("• {}", e.message)).into())
                .collect();
            let more = r.error_count as usize > shown.len();
            let mut list = Column::with_children(shown).spacing(2);
            if more {
                list = list.push(label::caption(format!(
                    "• … ve {} satır daha.",
                    r.error_count as usize - r.errors.iter().take(5).count()
                )));
            }
            lines.push(words::line(
                Line::Warn,
                column![
                    label::body(format!("{} satır nokta değil; alınmayacak:", r.error_count)),
                    list
                ]
                .spacing(4),
            ));
        }
        if r.duplicate_names > 0 {
            lines.push(words::text_line(
                Line::Info,
                format!(
                    "{} nokta başka bir noktayla aynı adı taşıyor; hepsi alınır.",
                    r.duplicate_names
                ),
            ));
        }
        for hint in &r.hints {
            lines.push(words::text_line(Line::Warn, hint.clone()));
        }
        if let (Some(b), Some(doc)) = (&r.bounds, &self.document) {
            lines.push(words::text_line(
                Line::Info,
                words::extent_text(&Format::of(doc.settings()), b),
            ));
        }
        words::summary(lines)
    }

    fn coord_layer<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_lay_over_the_files_columns_as_on_the_web() {
        assert_eq!(
            order_columns(ORDERS[0].1, 5),
            [
                CoordColumn::Name,
                CoordColumn::Y,
                CoordColumn::X,
                CoordColumn::Z,
                CoordColumn::Skip
            ]
        );
        assert_eq!(order_of(&order_columns(ORDERS[1].1, 6)), Some(1));
        assert_eq!(
            order_of(&[CoordColumn::Y, CoordColumn::Name, CoordColumn::X]),
            None
        );
    }
}
