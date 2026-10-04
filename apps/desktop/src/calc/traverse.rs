//! Poligon hesabı (the web's `ui/calc/TraverseDialog.ts`, docs/adr/0071):
//! from a known point oriented on a known back point, through the measured
//! angles (kırılma açısı: at each station clockwise from the previous point
//! to the next) and horizontal leg lengths, to the new points. Bağlı: closed
//! on a known end point, oriented on a known fore point when the angle there
//! was measured; Kapalı: back on the start; Açık: no closure. The
//! misclosures are shown and taken off (angles equally, coordinates by the
//! compass rule); no tolerance is applied: the surveyor judges them. The
//! computation is the shared core's (`survey::traverse::traverse`).

use std::fmt;

use iced::widget::{column, row};
use iced::{Element, Fill};
use kentos_domain::Document as Model;
use kentos_interaction::survey::traverse::{TraverseInput, TraverseResult, traverse};
use kentos_interaction::{Format, Vec2, fixed, js_trim};
use kentos_ui::label;
use kentos_ui::widget::Dialog;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::switch::Switch;

use super::grid::{self, Col, Table};
use super::read::{Known, read_number, resolve_point, unit_name};
use super::{
    Event, Field, NewPoint, Window, angle_text, event, footer, known_field, knowns, result_table,
    summary,
};
use crate::app::Message;
use crate::exchange::words::{self, Kind as Line};

pub const TITLE: &str = "Poligon hesabı";

/// The table's columns: Nokta, Kırılma açısı, Sonraki noktaya kenar.
pub const NAME: usize = 0;
pub const ANGLE: usize = 1;
pub const DISTANCE: usize = 2;

/// Which traverse.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    #[default]
    Connected,
    Closed,
    Open,
}

impl Kind {
    fn hint(self) -> &'static str {
        match self {
            Kind::Connected => "Bilinen bir noktadan başlar, bilinen başka bir noktada biter.",
            Kind::Closed => "Başladığı noktaya döner.",
            Kind::Open => "Bilinen bir noktada bitmez: kapanma denetimi ve dengeleme yok.",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Kind::Connected => "Bağlı",
            Kind::Closed => "Kapalı",
            Kind::Open => "Açık",
        })
    }
}

/// What is typed, kept while the app runs: the known points, the start
/// station's row, the new points (two empty at first) and the end
/// station's row.
#[derive(Clone, Debug)]
pub struct Form {
    pub kind: Kind,
    pub end_oriented: bool,
    pub start: String,
    pub back: String,
    pub end: String,
    pub fore: String,
    pub first: [String; 3],
    pub rows: Vec<[String; 3]>,
    pub last: [String; 3],
    pub layer: Option<String>,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            kind: Kind::Connected,
            end_oriented: true,
            start: String::new(),
            back: String::new(),
            end: String::new(),
            fore: String::new(),
            first: Default::default(),
            rows: vec![Default::default(); 2],
            last: Default::default(),
            layer: None,
        }
    }
}

/// What the fields give: the result and the new points' names, or why not.
pub struct Computed {
    pub errors: Vec<String>,
    pub result: Option<TraverseResult>,
    /// The new points' names, empty rows at the end left out.
    pub names: Vec<String>,
    /// The end station's name, when the traverse ends on a known point.
    pub end_name: Option<String>,
}

impl Computed {
    /// Each leg's name: the point it reaches.
    fn leg_name(&self, i: usize) -> String {
        self.names
            .get(i)
            .cloned()
            .unwrap_or_else(|| self.end_name.clone().unwrap_or_default())
    }

    /// Each leg's point: a new one, or none for the known end.
    fn leg_point(&self, r: &TraverseResult, i: usize) -> Option<Vec2> {
        r.points.get(i).copied()
    }
}

/// A known point's name for the table's station rows: the name it was
/// found by, else `fallback` (the web's `nameOf`).
fn name_of(model: &Model, text: &str, fallback: &str) -> String {
    match resolve_point(model, text) {
        Known::Point { name, .. } if !name.is_empty() => name,
        _ => fallback.to_owned(),
    }
}

impl Form {
    fn has_end_row(&self) -> bool {
        self.kind != Kind::Open
    }

    /// Whether the end station has an angle (to the fore point, or to the
    /// back point on a closed traverse).
    fn end_angle(&self) -> bool {
        self.kind == Kind::Closed || (self.kind == Kind::Connected && self.end_oriented)
    }

    /// The station rows named after the known points (the web's `viewRows`):
    /// the start's, and the end's (the start's again on a closed traverse);
    /// the end station has no leg after it.
    pub fn sync(&mut self, model: &Model) {
        self.first[NAME] = name_of(model, &self.start, "A");
        if self.has_end_row() {
            self.last[NAME] = if self.kind == Kind::Closed {
                self.first[NAME].clone()
            } else {
                name_of(model, &self.end, "B")
            };
            self.last[DISTANCE].clear();
        }
    }

    /// The fields read and computed (the web's `recompute`).
    pub fn compute(&self, model: &Model) -> Computed {
        let mut errors = Vec::new();
        let mut known = |text: &str, label: &str| match resolve_point(model, text) {
            Known::Empty => {
                errors.push(format!("{label} verilmedi."));
                None
            }
            Known::Error(e) => {
                errors.push(format!("{label}: {e}"));
                None
            }
            Known::Point { p, .. } => Some(p),
        };
        let start = known(&self.start, "Başlangıç noktası");
        let back = known(&self.back, "Başlangıçta bakılan nokta");
        let (end, fore) = match self.kind {
            Kind::Connected => {
                let end = known(&self.end, "Bitiş noktası");
                let fore = if self.end_oriented {
                    known(&self.fore, "Bitişte bakılan nokta")
                } else {
                    None
                };
                (end, fore)
            }
            Kind::Closed => (start, back),
            Kind::Open => (None, None),
        };
        // New points with nothing typed at the end of the table are left out.
        let mut rows: &[[String; 3]] = &self.rows;
        while let [head @ .., last] = rows
            && last.iter().all(|v| js_trim(v).is_empty())
        {
            rows = head;
        }
        // The stations with a leg after them: the start, the new points
        // (on an open traverse, all but the last).
        let legs = match (self.kind, rows) {
            (Kind::Open, [head @ .., _]) => head,
            _ => rows,
        };
        let mut angles = Vec::new();
        let mut distances = Vec::new();
        for (i, row) in std::iter::once(&self.first).chain(legs).enumerate() {
            let a = read_number(&row[ANGLE]);
            let d = read_number(&row[DISTANCE]);
            let place = if i == 0 {
                "Başlangıç satırında".to_owned()
            } else {
                format!("{}. satırda", i + 1)
            };
            match a {
                None => errors.push(format!("{place} kırılma açısı yok.")),
                Some(v) if v.is_nan() => {
                    errors.push(format!("{place} kırılma açısı bir sayı değil."))
                }
                Some(_) => {}
            }
            match d {
                None => errors.push(format!("{place} kenar uzunluğu yok.")),
                Some(v) if v.is_nan() => {
                    errors.push(format!("{place} kenar uzunluğu bir sayı değil."))
                }
                Some(_) => {}
            }
            angles.push(a.unwrap_or(f64::NAN));
            distances.push(d.unwrap_or(f64::NAN));
        }
        if fore.is_some() {
            let a = read_number(&self.last[ANGLE]);
            if a.is_none_or(f64::is_nan) {
                errors.push(if self.kind == Kind::Closed {
                    "Son satırda (başlangıca dönüşte) kırılma açısı yok.".to_owned()
                } else {
                    "Bitiş satırında kırılma açısı yok.".to_owned()
                });
            }
            angles.push(a.unwrap_or(f64::NAN));
        }
        if self.kind == Kind::Open && rows.is_empty() {
            errors.push("Açık poligonda en az bir yeni nokta olmalı.".to_owned());
        }
        let names: Vec<String> = rows
            .iter()
            .enumerate()
            .map(|(i, r)| match js_trim(&r[NAME]) {
                "" => format!("P{}", i + 1),
                name => name.to_owned(),
            })
            .collect();
        let mut result = None;
        let mut end_name = None;
        if let (true, Some(start), Some(back)) = (errors.is_empty(), start, back) {
            match traverse(&TraverseInput {
                unit: unit_name(model.settings().angle_unit).to_owned(),
                start,
                back,
                end,
                fore,
                angles,
                distances,
            }) {
                Ok(r) => {
                    result = Some(r);
                    end_name = self.has_end_row().then(|| {
                        if self.kind == Kind::Closed {
                            name_of(model, &self.start, "A")
                        } else {
                            name_of(model, &self.end, "B")
                        }
                    });
                }
                Err(e) => errors.push(e),
            }
        }
        Computed {
            errors,
            result,
            names,
            end_name,
        }
    }

    /// The new points to add, with their names.
    pub fn points(&self, model: &Model) -> Vec<NewPoint> {
        let c = self.compute(model);
        let Some(r) = &c.result else {
            return Vec::new();
        };
        r.points
            .iter()
            .zip(&c.names)
            .map(|(&p, name)| NewPoint {
                name: name.clone(),
                p,
                z: None,
            })
            .collect()
    }

    pub fn report(&self, model: &Model, format: &Format) -> Option<Vec<Vec<String>>> {
        let c = self.compute(model);
        let r = c.result.as_ref()?;
        let mut lines = vec![
            vec![TITLE.to_owned()],
            ["Nokta", "Semt", "Kenar", "ΔY", "ΔX", "vY", "vX", "Y", "X"]
                .map(str::to_owned)
                .to_vec(),
        ];
        for (i, leg) in r.legs.iter().enumerate() {
            let p = c.leg_point(r, i);
            lines.push(vec![
                c.leg_name(i),
                fixed(leg.bearing, 4),
                format.length_bare(leg.distance),
                format.length_bare(leg.dy),
                format.length_bare(leg.dx),
                fixed(leg.vy, 4),
                fixed(leg.vx, 4),
                p.map(|p| format.coord(p.x)).unwrap_or_default(),
                p.map(|p| format.coord(p.y)).unwrap_or_default(),
            ]);
        }
        // Each misclosure's row ends with its verdict against the project's
        // tolerance (docs/adr/0169 §3).
        let checked = checked(r, model.settings());
        if let Some(f) = r.angle_misclosure {
            let mut row = vec![
                "Açı kapanma hatası".to_owned(),
                small_angle_text(format, f),
                "Düzeltme".to_owned(),
                small_angle_text(format, r.angle_correction.unwrap_or(0.0)),
            ];
            row.extend(verdict(&checked.angle));
            lines.push(row);
        }
        if let Some(fs) = r.linear_misclosure {
            let mut row = vec![
                "fy".to_owned(),
                mm_text(r.fy.unwrap_or(0.0)),
                "fx".to_owned(),
                mm_text(r.fx.unwrap_or(0.0)),
                "fs".to_owned(),
                mm_text(fs),
                "Toplam".to_owned(),
                format.length(r.length),
            ];
            row.extend(verdict(&checked.coord));
            lines.push(row);
        }
        Some(lines)
    }

    /// What the summary says of a result (the web's `show`); the
    /// misclosures against the project's tolerances (docs/adr/0169 §3).
    fn summary_lines(
        &self,
        r: &TraverseResult,
        format: &Format,
        settings: &kentos_contracts::ProjectSettings,
    ) -> Vec<(Line, String)> {
        let mut lines = Vec::new();
        match r.angle_misclosure {
            Some(f) => lines.push((
                Line::Info,
                format!(
                    "Açı kapanma hatası fβ = {}; her açıya {} düzeltme verildi.",
                    small_angle_text(format, f),
                    small_angle_text(format, r.angle_correction.unwrap_or(0.0))
                ),
            )),
            None if self.kind == Kind::Connected => lines.push((
                Line::Info,
                "Bitişte yöneltme yok: açı kapanması denetlenmedi.".to_owned(),
            )),
            None => {}
        }
        lines.push((
            Line::Info,
            match r.linear_misclosure {
                Some(fs) => {
                    // Math.round, and left out when it comes to nothing (fs = 0).
                    let ratio = if fs != 0.0 {
                        (r.length / fs + 0.5).floor()
                    } else {
                        0.0
                    };
                    let ratio = if ratio.is_finite() && ratio != 0.0 {
                        format!(", 1/{ratio}")
                    } else {
                        String::new()
                    };
                    format!(
                        "Koordinat kapanma hatası fy = {}, fx = {}, fs = {}; kenarlara uzunluklarıyla orantılı dağıtıldı (toplam {}{ratio}).",
                        mm_text(r.fy.unwrap_or(0.0)),
                        mm_text(r.fx.unwrap_or(0.0)),
                        mm_text(fs),
                        format.length(r.length)
                    )
                }
                None => format!(
                    "Açık poligon: kapanma denetimi ve dengeleme yok (toplam {}).",
                    format.length(r.length)
                ),
            },
        ));
        lines.extend(closure_lines(r, settings));
        lines
    }

    pub fn view<'a>(&'a self, model: &Model, format: &Format) -> Element<'a, Message> {
        let c = self.compute(model);
        let kind = Segmented::new(
            [Kind::Connected, Kind::Closed, Kind::Open],
            self.kind,
            |k| event(Event::TraverseKind(k)),
        )
        .hints([Kind::Connected, Kind::Closed, Kind::Open].map(Kind::hint));
        let mut top = row![words::field("Poligon türü", kind, None)].spacing(18);
        if self.kind == Kind::Connected {
            top = top.push(words::field(
                "Bitiş",
                Switch::new(self.end_oriented, |on| event(Event::EndOriented(on)))
                    .label("Bitişte yöneltme açısı ölçüldü"),
                None,
            ));
        }
        let mut fields = vec![
            known_field(
                model,
                format,
                "Başlangıç noktası (A)",
                Field::Start,
                &self.start,
                None,
            ),
            known_field(
                model,
                format,
                "Başlangıçta bakılan nokta",
                Field::Back,
                &self.back,
                Some("İlk açı bu noktadan ölçülür"),
            ),
        ];
        if self.kind == Kind::Connected {
            fields.push(known_field(
                model,
                format,
                "Bitiş noktası (B)",
                Field::End,
                &self.end,
                None,
            ));
            if self.end_oriented {
                fields.push(known_field(
                    model,
                    format,
                    "Bitişte bakılan nokta",
                    Field::Fore,
                    &self.fore,
                    Some("Son açı bu noktaya ölçülür"),
                ));
            }
        }
        let columns: &'static [Col] = if format.angle_unit_label() == "°" {
            &COLUMNS_DEG
        } else {
            &COLUMNS_GRAD
        };
        let table = grid::view(Window::Traverse, columns, self, |r, col| {
            if col == NAME {
                format!("P{r}")
            } else {
                String::new()
            }
        });
        let mut body = column![
            column![top, knowns(fields)].spacing(12),
            column![
                label::caption(
                    "Kırılma açısı her istasyonda önceki noktadan sonraki noktaya saat yönünde ölçülür (başlangıçta bakılan noktadan). Kenarlar yataydır. Elektronik tablodan satırları yapıştırabilirsiniz."
                )
                .width(Fill),
                table,
            ]
            .spacing(8),
        ]
        .spacing(16);
        let lines = match &c.result {
            None => c
                .errors
                .iter()
                .take(6)
                .map(|e| (Line::Warn, e.clone()))
                .collect(),
            Some(r) => self.summary_lines(r, format, model.settings()),
        };
        if let Some(summary) = summary(lines) {
            body = body.push(summary);
        }
        if let Some(r) = &c.result {
            let rows = r
                .legs
                .iter()
                .enumerate()
                .map(|(i, leg)| {
                    let p = c.leg_point(r, i);
                    vec![
                        c.leg_name(i),
                        angle_text(format, leg.bearing),
                        format.length_bare(leg.distance),
                        format.length_bare(leg.dy),
                        format.length_bare(leg.dx),
                        p.map_or_else(|| "bilinen".to_owned(), |p| format.coord(p.x)),
                        p.map_or_else(|| "bilinen".to_owned(), |p| format.coord(p.y)),
                    ]
                })
                .collect();
            body = body.push(
                column![
                    label::strong("Sonuç"),
                    result_table(
                        &[
                            "Nokta",
                            "Semt",
                            "Kenar (m)",
                            "ΔY (m)",
                            "ΔX (m)",
                            "Y (sağa)",
                            "X (yukarı)"
                        ],
                        rows,
                        &[false, true, true, true, true, true, true],
                    ),
                ]
                .spacing(6),
            );
        }
        let new_points = c.result.as_ref().is_some_and(|r| !r.points.is_empty());
        footer(
            Dialog::new(TITLE).scroll(body),
            model,
            self.layer.as_deref(),
            c.result.is_some(),
            new_points,
        )
        .width(940.0)
        .into()
    }
}

/// The table as it shows: the start station, the new points, the end station.
impl Table for Form {
    fn columns(&self) -> usize {
        3
    }

    fn rows(&self) -> usize {
        1 + self.rows.len() + usize::from(self.has_end_row())
    }

    fn get(&self, row: usize, col: usize) -> &str {
        if row == 0 {
            &self.first[col]
        } else if self.has_end_row() && row == self.rows() - 1 {
            &self.last[col]
        } else {
            self.rows.get(row - 1).map_or("", |r| r[col].as_str())
        }
    }

    fn set(&mut self, row: usize, col: usize, text: String) {
        if row == 0 {
            self.first[col] = text;
        } else if self.has_end_row() && row == self.rows() - 1 {
            self.last[col] = text;
        } else if let Some(r) = self.rows.get_mut(row - 1) {
            r[col] = text;
        }
    }

    /// The stations' names are the known points'; the end station has no
    /// leg, and an angle only when it looks at a known point; an open
    /// traverse's last new point has neither an angle nor a leg after it.
    fn readonly(&self, row: usize, col: usize) -> bool {
        let rows = self.rows();
        let end_row = self.has_end_row() && row == rows - 1;
        if col == NAME {
            return row == 0 || end_row;
        }
        if end_row {
            return col == DISTANCE || !self.end_angle();
        }
        self.kind == Kind::Open && row == rows - 1 && row > 0
    }

    fn can_insert_after(&self, row: usize) -> bool {
        !(self.has_end_row() && row == self.rows() - 1)
    }

    /// A new point after the table's row `row` (the web's `splice(r, 0, {})`
    /// on the new points, which start at the table's second row).
    fn insert_after(&mut self, row: usize) {
        let at = row.min(self.rows.len());
        self.rows.insert(at, Default::default());
    }

    fn can_remove(&self, row: usize) -> bool {
        row > 0 && row <= self.rows.len()
    }

    fn remove(&mut self, row: usize) {
        if self.can_remove(row) {
            self.rows.remove(row - 1);
        }
    }
}

/// A small angle (a misclosure) with its fine unit: grad with cc (10⁻⁴ g),
/// degrees with seconds (the web's `smallAngleText`).
pub fn small_angle_text(format: &Format, v: f64) -> String {
    if format.angle_unit_label() == "°" {
        format!(
            "{}° ({}″)",
            unsigned(fixed(v, 5)),
            unsigned(fixed(v * 3600.0, 1))
        )
    } else {
        format!(
            "{} g ({} cc)",
            unsigned(fixed(v, 5)),
            unsigned(fixed(v * 10_000.0, 1))
        )
    }
}

/// A small length (a misclosure) in millimetres (the web's `mmText`).
pub fn mm_text(m: f64) -> String {
    format!("{} mm", unsigned(fixed(m * 1000.0, 1)))
}

/// A rounded value that came out as −0 reads as 0 (the web's `unsigned`,
/// `/^-0(\.0*)?$/`).
fn unsigned(text: String) -> String {
    let zero = text.strip_prefix("-0").is_some_and(|rest| {
        rest.is_empty()
            || rest
                .strip_prefix('.')
                .is_some_and(|d| d.bytes().all(|b| b == b'0'))
    });
    if zero { text[1..].to_owned() } else { text }
}

const COLUMNS_GRAD: [Col; 3] = columns("g");
const COLUMNS_DEG: [Col; 3] = columns("°");

const fn columns(unit: &'static str) -> [Col; 3] {
    [
        Col {
            label: "Nokta",
            unit: None,
            numeric: false,
        },
        Col {
            label: "Kırılma açısı",
            unit: Some(unit),
            numeric: true,
        },
        Col {
            label: "Sonraki noktaya kenar",
            unit: Some("m"),
            numeric: true,
        },
    ]
}

/// The project's traverse tolerances as Ölçme writes them and the core's
/// verdicts (docs/adr/0169 §3): each given one with its mark and whether the
/// misclosure is above it; none without that misclosure.
struct Checked {
    given: bool,
    angle: Option<(String, bool)>,
    coord: Option<(String, bool)>,
}

fn checked(r: &TraverseResult, settings: &kentos_contracts::ProjectSettings) -> Checked {
    use kentos_interaction::survey::Unit;
    use kentos_interaction::survey::traverse::closure;
    let unit = settings.angle_unit;
    let texts = kentos_project::survey_form::texts(settings.survey.as_ref(), unit);
    let survey = settings.survey.clone().unwrap_or_default();
    let core = match unit {
        kentos_contracts::AngleUnit::Grad => Unit::GRAD,
        kentos_contracts::AngleUnit::Deg => Unit::DEG,
    };
    let c = closure(
        core,
        r.angle_misclosure,
        r.linear_misclosure,
        survey.traverse_angle,
        survey.traverse_coord,
    );
    let mark = kentos_project::survey_form::angle_mark(unit);
    Checked {
        given: !texts[5].is_empty() || !texts[6].is_empty(),
        angle: c
            .angle_over
            .map(|over| (format!("{} {mark}", texts[5]), over)),
        coord: c.coord_over.map(|over| (format!("{} mm", texts[6]), over)),
    }
}

/// A report row's cells for a verdict: Tolerans, the tolerance, aşıyor or içinde.
fn verdict(c: &Option<(String, bool)>) -> Vec<String> {
    c.as_ref().map_or_else(Vec::new, |(t, over)| {
        vec![
            "Tolerans".to_owned(),
            t.clone(),
            if *over { "aşıyor" } else { "içinde" }.to_owned(),
        ]
    })
}

/// The misclosures against the project's tolerances (Proje ayarları ›
/// Ölçme, docs/adr/0169 §3); without one, the surveyor judges them.
pub(super) fn closure_lines(
    r: &TraverseResult,
    settings: &kentos_contracts::ProjectSettings,
) -> Vec<(Line, String)> {
    let c = checked(r, settings);
    if !c.given {
        return vec![(
            Line::Info,
            "Hata sınırı verilmedi (Proje ayarları › Ölçme): kapanma hatalarını ölçü sınıfınızın sınırlarıyla karşılaştırın."
                .to_owned(),
        )];
    }
    let mut lines = Vec::new();
    if let Some((t, over)) = c.angle {
        lines.push(if over {
            (
                Line::Warn,
                format!("Açı kapanma hatası toleransı ({t}) aşıyor."),
            )
        } else {
            (
                Line::Info,
                format!("Açı kapanma hatası toleransın ({t}) içinde."),
            )
        });
    }
    if let Some((t, over)) = c.coord {
        lines.push(if over {
            (
                Line::Warn,
                format!("Koordinat kapanma hatası fs toleransı ({t}) aşıyor."),
            )
        } else {
            (
                Line::Info,
                format!("Koordinat kapanma hatası fs toleransın ({t}) içinde."),
            )
        });
    }
    lines
}
