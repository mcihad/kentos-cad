//! Koordinat dönüştür (`crs.transform`, docs/adr/0167 §4; the web's
//! `ui/calc/ConvertDialog.ts` and `convert.ts`): a point typed or shown on
//! the drawing, or a list pasted, from one coordinate system to another; the
//! source is the project's and the target its second system at first. Each
//! answer says how sure it is (EPSG's operations, ±m). Nothing is written to
//! the drawing; the values are copied, the list also saved as CSV. What is
//! typed stays while the app runs.
//!
//! A projected system's point is two numbers, east first, named as the
//! project's type names its axes, read as the Hesap windows read numbers; a
//! geographic one is the latitude, then the longitude, in decimal degrees,
//! degrees and minutes, or degrees minutes seconds. Values are written with
//! the project's length digits, or in the user's notation with fixed digits.
//! The readings and writings pass the shared cases
//! (`fixtures/crs/v1/convert.json`, `scripts/fixtures/crs_convert_cases.py`:
//! PROJ).

use std::path::PathBuf;

use iced::widget::{button, column, container, row, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::ProjectSettings;
use kentos_geometry_core::crs::{format_dd, format_dms, parse_angle, transform};
use kentos_interaction::second::{Notation, accuracy_text};
use kentos_interaction::{Format, Level, Vec2, fixed};
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Tip, tip};

use super::grid::{self, Col, Table};
use super::read::read_number;
use super::{Event as CalcEvent, Window, footer_button, result_table, summary};
use crate::app::{App, Message};
use crate::crs::{self, System};
use crate::exchange::words::Kind as Line;

pub const TITLE: &str = "Koordinat dönüştür";

/// The source when the project has no system of its own (the web's `DEFAULT_SRID`).
const DEFAULT_SRID: u32 = 5256;
/// WGS 84's latitudes and longitudes: the target when the project has no second system.
const WGS84: u32 = 4326;

/// Tek nokta or Liste.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Point,
    List,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Mode::Point => "Tek nokta",
            Mode::List => "Liste",
        })
    }
}

/// What the window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    From(u32),
    To(u32),
    /// The source and the target change places.
    Swap,
    Mode(Mode),
    /// The point's two values.
    A(String),
    B(String),
    /// Çizimden: the window closes, the point is shown on the drawing.
    Pick,
    /// The values to the clipboard: the point's two, or the list.
    Copy,
    /// The list as a CSV file: where, then written.
    SaveCsv,
    CsvSaved(Option<PathBuf>),
}

fn message(e: Event) -> Message {
    Message::Calc(CalcEvent::Convert(e))
}

/// Why a point was not converted: a field unread, a latitude or longitude
/// out of range, a point the target does not reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertError {
    A,
    B,
    Range,
    Unreachable,
}

#[cfg(test)]
impl ConvertError {
    /// As the shared cases write it.
    pub fn as_str(self) -> &'static str {
        match self {
            ConvertError::A => "a",
            ConvertError::B => "b",
            ConvertError::Range => "range",
            ConvertError::Unreachable => "unreachable",
        }
    }
}

/// How the window reads and writes: the project's axes' names and length
/// digits, the user's notation.
#[derive(Clone, Copy, Debug)]
pub struct ConvertFormat {
    pub east: &'static str,
    pub north: &'static str,
    pub decimals: usize,
    pub notation: Notation,
}

impl ConvertFormat {
    pub fn of(settings: &ProjectSettings, notation: Notation) -> Self {
        let f = Format::of(settings);
        Self {
            east: f.east_label(),
            north: f.north_label(),
            decimals: (settings.length_decimals as usize).min(12),
            notation,
        }
    }
}

/// A converted point: its values with their names, how sure they are.
#[derive(Clone, Debug, PartialEq)]
pub struct Converted {
    pub point: Vec2,
    pub values: [(&'static str, String); 2],
    pub accuracy: String,
}

fn geographic(sys: &System) -> bool {
    sys.kind == "geographic"
}

/// The two fields' names in a system: east and north as the project names
/// them, or latitude and longitude.
pub fn field_names(sys: &System, f: &ConvertFormat) -> [&'static str; 2] {
    if geographic(sys) {
        ["Enlem", "Boylam"]
    } else {
        [f.east, f.north]
    }
}

/// A typed point in `sys`'s own order (x east or longitude, y north or
/// latitude), or why there is none.
pub fn read_point(sys: &System, a: &str, b: &str) -> Result<Vec2, ConvertError> {
    if geographic(sys) {
        let lat = parse_angle(a).ok_or(ConvertError::A)?;
        let lon = parse_angle(b).ok_or(ConvertError::B)?;
        return if lat.abs() > 90.0 || lon.abs() > 180.0 {
            Err(ConvertError::Range)
        } else {
            Ok(Vec2::new(lon, lat))
        };
    }
    let east = read_number(a)
        .filter(|v| v.is_finite())
        .ok_or(ConvertError::A)?;
    let north = read_number(b)
        .filter(|v| v.is_finite())
        .ok_or(ConvertError::B)?;
    Ok(Vec2::new(east, north))
}

/// A point of `sys` (its own order) as the window writes it.
pub fn write_point(sys: &System, p: Vec2, f: &ConvertFormat) -> [(&'static str, String); 2] {
    if geographic(sys) {
        let write = |deg: f64, latitude: bool| match f.notation {
            Notation::Dd => format_dd(deg, latitude, 7),
            Notation::Dms => format_dms(deg, latitude, 4),
        };
        return [("Enlem", write(p.y, true)), ("Boylam", write(p.x, false))];
    }
    [
        (f.east, fixed(p.x, f.decimals)),
        (f.north, fixed(p.y, f.decimals)),
    ]
}

/// `a` and `b` typed in `from`, taken to `to` and written; or why not.
pub fn convert_point(
    from: &System,
    to: &System,
    a: &str,
    b: &str,
    f: &ConvertFormat,
) -> Result<Converted, ConvertError> {
    let p = read_point(from, a, b)?;
    let moved = match (from.transform_system(), to.transform_system()) {
        (Some(s), Some(t)) => transform(&s, &t, p),
        _ => None,
    }
    .ok_or(ConvertError::Unreachable)?;
    Ok(Converted {
        point: moved.point,
        values: write_point(to, moved.point, f),
        accuracy: accuracy_text(&moved),
    })
}

/// What went wrong with a point, in a sentence that says how to put it right.
pub fn error_text(e: ConvertError, from: &System, f: &ConvertFormat) -> String {
    let [a, b] = field_names(from, f);
    let how = if geographic(from) {
        "40 45 12.3456, 40°45′12.3456″K ya da 40.7534293 biçiminde yazın"
    } else {
        "bir sayı yazın, ör. 414120.512"
    };
    match e {
        ConvertError::A => format!("{a} okunamadı: {how}."),
        ConvertError::B => format!("{b} okunamadı: {how}."),
        ConvertError::Range => {
            "Enlem −90° ile 90°, boylam −180° ile 180° arasında olmalı.".to_owned()
        }
        ConvertError::Unreachable => {
            "Nokta hedef sistemin ulaştığı yerin dışında; değer yazılmadı.".to_owned()
        }
    }
}

/// A list converted: each filled row's name, number (1 first) and result.
pub fn convert_rows(
    from: &System,
    to: &System,
    rows: &[[String; 3]],
    f: &ConvertFormat,
) -> Vec<(String, usize, Result<Converted, ConvertError>)> {
    rows.iter()
        .enumerate()
        .filter(|(_, [name, a, b])| {
            !(name.trim().is_empty() && a.trim().is_empty() && b.trim().is_empty())
        })
        .map(|(i, [name, a, b])| {
            let name = match name.trim() {
                "" => (i + 1).to_string(),
                n => n.to_owned(),
            };
            (name, i + 1, convert_point(from, to, a, b, f))
        })
        .collect()
}

/// A CSV cell: quoted when it holds a comma, a quote or a line break.
fn csv_cell(v: &str) -> String {
    if v.contains([',', '"', '\n']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_owned()
    }
}

/// Rows as CSV, a heading line first.
pub fn csv_text(heading: &[&str], rows: &[Vec<String>]) -> String {
    let line = |cells: Vec<String>| cells.join(",");
    let mut out = vec![line(heading.iter().map(|c| csv_cell(c)).collect())];
    out.extend(
        rows.iter()
            .map(|r| line(r.iter().map(|c| csv_cell(c)).collect())),
    );
    out.join("\n") + "\n"
}

/// What is typed, kept while the app runs.
#[derive(Clone, Debug)]
pub struct Form {
    /// The source and the target; none: the project's, and its second system.
    pub from: Option<u32>,
    pub to: Option<u32>,
    pub mode: Mode,
    pub a: String,
    pub b: String,
    /// Liste's rows: name and the two values.
    pub rows: Vec<[String; 3]>,
    /// Çizimden is waiting for the point.
    pub picking: bool,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            from: None,
            to: None,
            mode: Mode::Point,
            a: String::new(),
            b: String::new(),
            rows: vec![Default::default(); 3],
            picking: false,
        }
    }
}

/// Liste's columns: name and the two values.
impl Table for Form {
    fn columns(&self) -> usize {
        3
    }

    fn rows(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, row: usize, col: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map_or("", String::as_str)
    }

    fn set(&mut self, row: usize, col: usize, text: String) {
        if let Some(cell) = self.rows.get_mut(row).and_then(|r| r.get_mut(col)) {
            *cell = text;
        }
    }

    fn insert_after(&mut self, row: usize) {
        let at = (row + 1).min(self.rows.len());
        self.rows.insert(at, Default::default());
    }

    fn can_remove(&self, _row: usize) -> bool {
        self.rows.len() > 1
    }

    fn remove(&mut self, row: usize) {
        if self.rows.len() > 1 && row < self.rows.len() {
            self.rows.remove(row);
        }
    }
}

/// A value's field across its half of the row (the web's `calc-convert-point`).
fn wide_field<'a>(
    title: &'static str,
    value: &'a str,
    placeholder: &'static str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .padding([5, 8])
        .width(Fill)
        .font(typography::mono())
        .size(typography::body())
        .style(style::field::input);
    column![label::caption(title), kentos_ui::widget::focus_ring(input)]
        .spacing(4)
        .into()
}

/// Liste's columns by the source's fields' names.
const COLUMNS_GIS: [Col; 3] = [col("Ad", false), col("Y", true), col("X", true)];
const COLUMNS_CAD: [Col; 3] = [col("Ad", false), col("X", true), col("Y", true)];
const COLUMNS_GEOGRAPHIC: [Col; 3] = [col("Ad", false), col("Enlem", false), col("Boylam", false)];

const fn col(label: &'static str, numeric: bool) -> Col {
    Col {
        label,
        unit: None,
        numeric,
    }
}

/// The systems one may convert between, a heading for each datum, as the
/// registry lists them (no local system); each choice's system.
fn system_choices() -> (Vec<Choice>, Vec<Option<u32>>) {
    let mut choices = Vec::new();
    let mut srids = Vec::new();
    let mut datum = "";
    for s in crs::systems() {
        if s.is_local() {
            continue;
        }
        let label = crs::datum_label(&s.datum);
        if label != datum {
            datum = label;
            choices.push(Choice::header(label));
            srids.push(None);
        }
        choices.push(
            Choice::new(s.name.clone())
                .detail(format!("EPSG:{}", s.srid))
                .shown(crs::title(s)),
        );
        srids.push(Some(s.srid));
    }
    (choices, srids)
}

impl Form {
    /// The source and the target in use: as chosen, or the project's and its
    /// second system (WGS 84 when it has none; a geographic source goes to
    /// the default grid).
    pub fn systems(&self, settings: &ProjectSettings) -> (&'static System, &'static System) {
        let fallback = |srid: u32| crs::system(srid).or_else(|| crs::system(DEFAULT_SRID));
        let own = (settings.srid != crs::LOCAL_SRID).then_some(settings.srid);
        let from = self
            .from
            .or(own)
            .and_then(crs::system)
            .or_else(|| fallback(DEFAULT_SRID));
        let to_default = settings.second().unwrap_or(match from {
            Some(s) if geographic(s) => DEFAULT_SRID,
            _ => WGS84,
        });
        let to = self.to.or(Some(to_default)).and_then(crs::system);
        match (from, to.or_else(|| fallback(WGS84))) {
            (Some(from), Some(to)) => (from, to),
            // The registry always has both defaults; the first two systems otherwise.
            _ => (&crs::systems()[1], &crs::systems()[2]),
        }
    }

    /// The point converted, when both values are typed.
    pub fn point(
        &self,
        settings: &ProjectSettings,
        f: &ConvertFormat,
    ) -> Option<Result<Converted, ConvertError>> {
        if self.a.trim().is_empty() && self.b.trim().is_empty() {
            return None;
        }
        let (from, to) = self.systems(settings);
        Some(convert_point(from, to, &self.a, &self.b, f))
    }

    /// The list's converted rows as the results table and the CSV give them:
    /// the heading, then name and values.
    pub fn listed(
        &self,
        settings: &ProjectSettings,
        f: &ConvertFormat,
    ) -> Option<(Vec<&'static str>, Vec<Vec<String>>)> {
        let (from, to) = self.systems(settings);
        let [a, b] = field_names(to, f);
        let rows: Vec<Vec<String>> = convert_rows(from, to, &self.rows, f)
            .into_iter()
            .filter_map(|(name, _, r)| {
                r.ok()
                    .map(|c| vec![name, c.values[0].1.clone(), c.values[1].1.clone()])
            })
            .collect();
        (!rows.is_empty()).then(|| (vec!["Ad", a, b], rows))
    }

    pub fn view<'a>(
        &'a self,
        settings: &ProjectSettings,
        f: &ConvertFormat,
    ) -> Element<'a, Message> {
        let (from, to) = self.systems(settings);
        let (choices, srids) = system_choices();
        let at = |srid: u32| srids.iter().position(|s| *s == Some(srid));
        let pick = |which: fn(u32) -> Event, selected: u32| {
            let srids = srids.clone();
            container(Select::new(choices.clone(), at(selected), move |i| {
                message(which(srids.get(i).copied().flatten().unwrap_or(selected)))
            }))
            .width(300)
        };
        let swap = tip(
            button(icon(crate::icons::from_web(Some("reverse"))).size(16.0))
                .on_press(message(Event::Swap))
                .padding([6, 10])
                .style(style::button::secondary),
            Tip::new("Kaynakla hedefi değiştir"),
            iced::widget::tooltip::Position::Top,
        );
        let systems = row![
            column![
                label::caption("Kaynak sistem"),
                pick(Event::From, from.srid)
            ]
            .spacing(4),
            swap,
            column![label::caption("Hedef sistem"), pick(Event::To, to.srid)].spacing(4),
        ]
        .spacing(10)
        .align_y(iced::Bottom);
        let mode = column![
            label::caption("Dönüştürülecek"),
            Segmented::new([Mode::Point, Mode::List], self.mode, |m| {
                message(Event::Mode(m))
            }),
        ]
        .spacing(4);
        let mut body = column![systems, mode].spacing(16);
        let [a, b] = field_names(from, f);
        let mut lines: Vec<(Line, String)> = Vec::new();
        match self.mode {
            Mode::Point => {
                let own = from.srid == settings.srid;
                let show = button(
                    row![
                        icon(crate::icons::from_web(Some("snap"))).size(14.0),
                        label::body("Çizimden")
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .on_press_maybe(own.then(|| message(Event::Pick)))
                .padding([5, 10])
                .style(style::button::secondary);
                let show = tip(
                    show,
                    Tip::new("Çizimden seç").body(if own {
                        "Noktayı çizimde gösterin; kenetlenir."
                    } else {
                        "Çizimin koordinatları projenin sistemindedir: çizimden seçmek için kaynak sistem projeninki olmalı."
                    }),
                    iced::widget::tooltip::Position::Top,
                );
                let geographic = geographic(from);
                body = body.push(
                    row![
                        container(wide_field(
                            a,
                            &self.a,
                            if geographic { "40 45 12.3456" } else { "" },
                            |t| message(Event::A(t))
                        ))
                        .width(Fill),
                        container(wide_field(
                            b,
                            &self.b,
                            if geographic { "29 55 01.2345" } else { "" },
                            |t| message(Event::B(t))
                        ))
                        .width(Fill),
                        show,
                    ]
                    .spacing(18)
                    .align_y(iced::Bottom),
                );
                match self.point(settings, f) {
                    None => lines.push((
                        Line::Info,
                        "Noktanın iki değerini yazın ya da çizimden seçin.".to_owned(),
                    )),
                    Some(Err(e)) => lines.push((Line::Warn, error_text(e, from, f))),
                    Some(Ok(c)) => {
                        let values =
                            iced::widget::Row::with_children(c.values.iter().map(|(name, v)| {
                                row![label::muted(*name), label::mono(v.clone())]
                                    .spacing(8)
                                    .into()
                            }))
                            .spacing(28);
                        body = body.push(
                            container(values)
                                .padding([10, 12])
                                .width(Fill)
                                .style(style::container::bordered),
                        );
                        lines.push((
                            if from.datum == to.datum {
                                Line::Ok
                            } else {
                                Line::Info
                            },
                            format!("{}: {}.", to.name, c.accuracy),
                        ));
                    }
                }
            }
            Mode::List => {
                let columns: &'static [Col] = if geographic(from) {
                    &COLUMNS_GEOGRAPHIC
                } else if f.east == "X" {
                    &COLUMNS_CAD
                } else {
                    &COLUMNS_GIS
                };
                body = body.push(
                    column![
                        label::caption(
                            "Satır satır ad ve iki değer yazın ya da elektronik tablodan yapıştırın (sekme, noktalı virgül ya da boşlukla ayrılmış)."
                        ),
                        grid::view(Window::Convert, columns, self, |_, _| String::new()),
                    ]
                    .spacing(8),
                );
                let rows = convert_rows(from, to, &self.rows, f);
                let good: Vec<&(String, usize, Result<Converted, ConvertError>)> =
                    rows.iter().filter(|r| r.2.is_ok()).collect();
                let bad: Vec<&(String, usize, Result<Converted, ConvertError>)> =
                    rows.iter().filter(|r| r.2.is_err()).collect();
                if rows.is_empty() {
                    lines.push((
                        Line::Info,
                        "Dönüştürülecek satır yok: tabloya yazın ya da yapıştırın.".to_owned(),
                    ));
                }
                if let Some((_, _, Ok(first))) = good.first() {
                    lines.push((
                        Line::Ok,
                        format!(
                            "{} nokta dönüştürüldü. {}: {}.",
                            good.len(),
                            to.name,
                            first.accuracy
                        ),
                    ));
                }
                for (_, n, r) in bad.iter().take(6) {
                    if let Err(e) = r {
                        lines.push((
                            Line::Warn,
                            format!("Satır {n}: {}", error_text(*e, from, f)),
                        ));
                    }
                }
                if bad.len() > 6 {
                    lines.push((
                        Line::Warn,
                        format!("{} satır daha okunamadı.", bad.len() - 6),
                    ));
                }
                if let Some((heading, rows)) = self.listed(settings, f) {
                    body = body.push(result_table(&heading, rows, &[false, true, true]));
                }
            }
        }
        if let Some(summary) = summary(lines) {
            body = body.push(summary);
        }
        let copyable = match self.mode {
            Mode::Point => matches!(self.point(settings, f), Some(Ok(_))),
            Mode::List => self.listed(settings, f).is_some(),
        };
        let mut dialog = Dialog::new(TITLE).scroll(body);
        if self.mode == Mode::List {
            dialog = dialog.action(footer_button(
                "CSV olarak kaydet",
                copyable.then(|| message(Event::SaveCsv)),
                false,
            ));
        }
        dialog
            .action(footer_button(
                "Panoya kopyala",
                copyable.then(|| message(Event::Copy)),
                true,
            ))
            .action(footer_button(
                "Kapat",
                Some(Message::Calc(CalcEvent::Close)),
                false,
            ))
            .width(820.0)
            .max_height(super::MAX_HEIGHT)
            .into()
    }
}

impl App {
    /// The project's settings and the window's format: the axes' names, the
    /// length digits, the user's notation.
    fn convert_format(&self) -> Option<(ProjectSettings, ConvertFormat)> {
        let doc = self.document.as_ref()?;
        let settings = doc.settings().clone();
        let f = ConvertFormat::of(&settings, self.draft.geographic);
        Some((settings, f))
    }

    pub(crate) fn convert_event(&mut self, e: Event) -> Task<Message> {
        let form = &mut self.calc.convert;
        match e {
            Event::From(srid) => form.from = Some(srid),
            Event::To(srid) => form.to = Some(srid),
            Event::Swap => {
                if let Some((settings, _)) = self.convert_format() {
                    let (from, to) = self.calc.convert.systems(&settings);
                    let form = &mut self.calc.convert;
                    form.from = Some(to.srid);
                    form.to = Some(from.srid);
                }
            }
            Event::Mode(mode) => form.mode = mode,
            Event::A(t) => form.a = t,
            Event::B(t) => form.b = t,
            Event::Pick => {
                form.picking = true;
                self.calc_pick(TITLE, "nokta".to_owned());
            }
            Event::Copy => return self.convert_copy(),
            Event::SaveCsv => {
                let name = match self.convert_format() {
                    Some((settings, _)) => {
                        format!(
                            "koordinatlar-{}.csv",
                            self.calc.convert.systems(&settings).1.srid
                        )
                    }
                    None => "koordinatlar.csv".to_owned(),
                };
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Dönüştürülen koordinatları kaydet")
                            .add_filter("CSV (.csv)", &["csv"])
                            .set_file_name(name)
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| message(Event::CsvSaved(path)),
                );
            }
            Event::CsvSaved(None) => {}
            Event::CsvSaved(Some(path)) => self.convert_write_csv(&path),
        }
        Task::none()
    }

    /// Çizimden's point: its exact coordinates in the two fields.
    pub(crate) fn convert_picked(&mut self, p: Option<Vec2>) {
        let form = &mut self.calc.convert;
        form.picking = false;
        if let Some(p) = p {
            form.a = format!("{}", p.x);
            form.b = format!("{}", p.y);
        }
    }

    /// The values to the system clipboard, tab-separated: the point's two,
    /// or the list's rows with their heading.
    fn convert_copy(&mut self) -> Task<Message> {
        let Some((settings, f)) = self.convert_format() else {
            return Task::none();
        };
        let form = &self.calc.convert;
        let (text, said) = match form.mode {
            Mode::Point => match form.point(&settings, &f) {
                Some(Ok(c)) => (
                    format!("{}\t{}", c.values[0].1, c.values[1].1),
                    "Dönüştürülen nokta panoya kopyalandı.".to_owned(),
                ),
                _ => return Task::none(),
            },
            Mode::List => match form.listed(&settings, &f) {
                Some((heading, rows)) => {
                    let mut lines = vec![heading.join("\t")];
                    lines.extend(rows.iter().map(|r| r.join("\t")));
                    (
                        lines.join("\n"),
                        format!(
                            "{} nokta panoya kopyalandı (elektronik tabloya yapıştırılabilir).",
                            rows.len()
                        ),
                    )
                }
                None => return Task::none(),
            },
        };
        self.say(Level::Success, said);
        iced::clipboard::write(text)
    }

    /// The list written to `path` as CSV, and said.
    fn convert_write_csv(&mut self, path: &std::path::Path) {
        let Some((settings, f)) = self.convert_format() else {
            return;
        };
        let Some((heading, rows)) = self.calc.convert.listed(&settings, &f) else {
            return;
        };
        match std::fs::write(path, csv_text(&heading, &rows)) {
            Ok(()) => self.say(
                Level::Success,
                format!(
                    "{} nokta CSV olarak kaydedildi: {}.",
                    rows.len(),
                    path.display()
                ),
            ),
            Err(e) => self.say(
                Level::Error,
                format!(
                    "{} yazılamadı: {e}. Başka bir klasör seçip yeniden deneyin.",
                    path.display()
                ),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        let path = format!(
            "{}/../../fixtures/crs/v1/convert.json",
            env!("CARGO_MANIFEST_DIR")
        );
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
    }

    /// The readings and writings as the shared cases say (PROJ), exactly.
    #[test]
    fn reads_and_writes_as_the_shared_cases_say() {
        let file = fixture();
        assert_eq!(file["format"], "kentos.crs-convert");
        let cases = file["cases"].as_array().expect("cases");
        assert!(cases.len() >= 15);
        for case in cases {
            let name = case["name"].as_str().expect("name");
            let cad = case["axes"] == "cad";
            let f = ConvertFormat {
                east: if cad { "X" } else { "Y" },
                north: if cad { "Y" } else { "X" },
                decimals: case["decimals"].as_u64().expect("decimals") as usize,
                notation: Notation::parse(case["notation"].as_str().expect("notation")),
            };
            let system = |key: &str| {
                crs::system(case[key].as_u64().expect("srid") as u32).expect("a system")
            };
            let input = |i: usize| case["input"][i].as_str().expect("input");
            let got = convert_point(system("fromSrid"), system("toSrid"), input(0), input(1), &f);
            match case["error"].as_str() {
                Some(error) => assert_eq!(got.map_err(ConvertError::as_str), Err(error), "{name}"),
                None => {
                    let c = got.unwrap_or_else(|e| panic!("{name}: {e:?}"));
                    let want = &case["expect"];
                    let values: Vec<(String, String)> = c
                        .values
                        .iter()
                        .map(|(n, v)| ((*n).to_owned(), v.clone()))
                        .collect();
                    let expected: Vec<(String, String)> = want["values"]
                        .as_array()
                        .expect("values")
                        .iter()
                        .map(|v| {
                            (
                                v[0].as_str().expect("name").to_owned(),
                                v[1].as_str().expect("value").to_owned(),
                            )
                        })
                        .collect();
                    assert_eq!(values, expected, "{name}");
                    assert_eq!(
                        c.accuracy,
                        want["accuracy"].as_str().expect("accuracy"),
                        "{name}"
                    );
                }
            }
        }
    }

    /// The words the web writes too (`convert.test.ts`), a list with a row it
    /// cannot read, and CSV.
    #[test]
    fn says_what_is_wrong_lists_and_writes_csv() {
        let f = ConvertFormat {
            east: "Y",
            north: "X",
            decimals: 3,
            notation: Notation::Dms,
        };
        let system = |srid| crs::system(srid).expect("a system");
        assert_eq!(
            error_text(ConvertError::A, system(5254), &f),
            "Y okunamadı: bir sayı yazın, ör. 414120.512."
        );
        assert_eq!(
            error_text(ConvertError::B, system(4326), &f),
            "Boylam okunamadı: 40 45 12.3456, 40°45′12.3456″K ya da 40.7534293 biçiminde yazın."
        );
        let rows = [
            [
                "R1".to_owned(),
                "414120.512".to_owned(),
                "4540398.207".to_owned(),
            ],
            Default::default(),
            [String::new(), "yok".to_owned(), "1".to_owned()],
        ];
        // A row's name, its number and its values or why there are none.
        type Row = (String, usize, Result<Vec<String>, &'static str>);
        let got: Vec<Row> = convert_rows(system(5254), system(2320), &rows, &f)
            .into_iter()
            .map(|(n, i, r)| {
                (
                    n,
                    i,
                    r.map(|c| c.values.iter().map(|v| v.1.clone()).collect())
                        .map_err(ConvertError::as_str),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (
                    "R1".to_owned(),
                    1,
                    Ok(vec!["414154.869".to_owned(), "4540584.350".to_owned()])
                ),
                ("3".to_owned(), 3, Err("a")),
            ]
        );
        assert_eq!(
            csv_text(
                &["Ad", "Y", "X"],
                &[
                    vec!["R1".into(), "414154.869".into(), "4540584.350".into()],
                    vec!["a,b".into(), "1".into(), "2".into()],
                ]
            ),
            "Ad,Y,X\nR1,414154.869,4540584.350\n\"a,b\",1,2\n"
        );
    }
}

/// Koordinat dönüştür's pictures over the sample drawing (TUREF TM36), its
/// second system ED50 TM36: a point to ED50 TM36 and to WGS 84 in degrees,
/// minutes and seconds, a list with a row it cannot read, at 1440 × 900 and
/// 1100 × 650 in both themes; `.run/shots/donustur-*` (the web's:
/// `(cd apps/web && node scripts/e2e/shots.mjs convert)`):
///
/// ```text
/// cargo test -p kentos-desktop calc::convert::screens -- --ignored --nocapture
/// ```
#[cfg(test)]
mod screens {
    use super::*;
    use crate::app::App;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    fn send(app: &mut App, e: Event) {
        let _ = app.update(message(e));
    }

    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (theme, suffix) in [("dark", ""), ("light", "-acik")] {
            for (w, h) in [(1440.0, 900.0), (1100.0, 650.0)] {
                for name in ["nokta", "wgs84", "liste"] {
                    let mut app = crate::files_testing::app_with_drawing();
                    let _ = app
                        .settings
                        .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                    app.apply_settings();
                    let _ = app.update(Message::SecondCrs(crate::second_crs::Event::Choose(Some(
                        2322,
                    ))));
                    app.follow.flash = None;
                    let _ = app.update(Message::Run("crs.transform"));
                    send(&mut app, Event::A("486512.34".into()));
                    send(&mut app, Event::B("4420187.52".into()));
                    match name {
                        "wgs84" => send(&mut app, Event::To(4326)),
                        "liste" => {
                            send(&mut app, Event::Mode(Mode::List));
                            app.calc.convert.rows = vec![
                                ["P1".into(), "486512.34".into(), "4420187.52".into()],
                                ["P2".into(), "486530.25".into(), "4420150.80".into()],
                                ["P3".into(), "48650x".into(), "4420120.00".into()],
                            ];
                        }
                        _ => {}
                    }
                    let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.settle(&mut app, App::view, &mut update);
                    let file = out.join(format!("donustur-{name}-{w}{suffix}.png"));
                    snapshot
                        .render(app.view(), &app.theme())
                        .save(&file)
                        .expect("writes the picture");
                    println!("{}", file.display());
                }
            }
        }
    }
}
