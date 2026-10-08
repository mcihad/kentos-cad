//! Yatay ağ dengelemesi and Kot ağı dengelemesi (docs/adr/0203 §7–§8; the
//! web's `ui/calc/NetworkDialog.ts`): two Hesap windows with two tables
//! each, the known points and the observations, adjusted by the geometry
//! core (`survey::adjust`) at every change. The known points are fixed, or
//! weighted with their σ; a known point typed by its name alone is the
//! drawing's. New points the drawing has start where it has them. The
//! summary says what does not hold, or the adjustment's counts, m0 and
//! model test, then the tables of the points and of the observations.
//! Çizime yaz moves the drawing's points of the same name and adds the
//! others ([`write`]); Raporu kopyala copies everything as tab-separated
//! lines. What is typed stays while the app runs.

#[cfg(test)]
mod tests;
mod view;
mod write;

use iced::Task;
use kentos_contracts::{AngleUnit, Entity, ProjectSettings};
use kentos_domain::Document as Model;
use kentos_geometry_core::survey::adjust::horizontal::{
    self, ApproxPoint, KnownPoint, NetRow, NetworkInput, NetworkResult,
};
use kentos_geometry_core::survey::adjust::levelling::{
    self, KnownHeight, LevelInput, LevelResult, LevelRow,
};
use kentos_geometry_core::survey::adjust::{ObservationResult, Sigmas};
use kentos_interaction::ground::survey_grid;
use kentos_interaction::{fixed, js_trim, upper_tr};

use super::grid::{self, Owner, Table};
use super::read::read_number;
use super::{Event as Calc, event};
use crate::app::Message;
use crate::exchange::words::Kind as Line;

pub const NETWORK_TITLE: &str = "Yatay ağ dengelemesi";
pub const LEVEL_TITLE: &str = "Kot ağı dengelemesi";
/// The kind of the points Çizime yaz adds (the `Tür` attribute).
pub const POINT_KIND: &str = "Ağ noktası";

/// The tables of the two windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    NetworkKnown,
    NetworkRows,
    LevelKnown,
    LevelRows,
}

impl Sheet {
    /// The window's own events.
    fn msg(e: Event) -> Message {
        event(Calc::Network(e))
    }
}

impl Owner for Sheet {
    fn cell_id(self, row: usize, col: usize) -> iced::widget::Id {
        iced::widget::Id::from(format!("calc-net-{self:?}-{row}-{col}"))
    }
    fn cell(self, row: usize, col: usize, text: String) -> Message {
        Sheet::msg(Event::Cell(self, row, col, text))
    }
    fn paste(self, row: usize, col: usize, text: String) -> Message {
        Sheet::msg(Event::Paste(self, row, col, text))
    }
    fn submit(self, row: usize, col: usize) -> Message {
        Sheet::msg(Event::Submit(self, row, col))
    }
    fn remove_row(self, row: usize) -> Message {
        Sheet::msg(Event::RemoveRow(self, row))
    }
    fn add_row(self) -> Message {
        Sheet::msg(Event::AddRow(self))
    }
}

/// A table of text cells: one empty row at least, as the Hesap tables keep.
#[derive(Clone, Debug, PartialEq)]
pub struct Cells {
    cols: usize,
    pub rows: Vec<Vec<String>>,
}

impl Cells {
    pub fn new(cols: usize) -> Cells {
        Cells {
            cols,
            rows: vec![vec![String::new(); cols]],
        }
    }

    /// The table filled with these rows, then one empty row.
    pub fn of(cols: usize, rows: Vec<Vec<String>>) -> Cells {
        let mut rows: Vec<Vec<String>> = rows
            .into_iter()
            .map(|mut r| {
                r.resize(cols, String::new());
                r
            })
            .collect();
        rows.push(vec![String::new(); cols]);
        Cells { cols, rows }
    }

    /// The rows something is typed in, with their places.
    pub fn filled(&self) -> impl Iterator<Item = (usize, &Vec<String>)> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.iter().any(|c| !js_trim(c).is_empty()))
    }
}

impl Table for Cells {
    fn columns(&self) -> usize {
        self.cols
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
        while self.rows.len() <= row {
            self.rows.push(vec![String::new(); self.cols]);
        }
        if let Some(cell) = self.rows[row].get_mut(col) {
            *cell = text;
        }
    }
    fn insert_after(&mut self, row: usize) {
        let at = (row + 1).min(self.rows.len());
        self.rows.insert(at, vec![String::new(); self.cols]);
    }
    fn can_remove(&self, _row: usize) -> bool {
        self.rows.len() > 1
    }
    fn remove(&mut self, row: usize) {
        if row < self.rows.len() && self.rows.len() > 1 {
            self.rows.remove(row);
        }
    }
}

/// Kot ağı's kind of height differences (docs/adr/0203 §5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LevelKind {
    #[default]
    Geometric,
    Trigonometric,
}

impl std::fmt::Display for LevelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            LevelKind::Geometric => "Geometrik nivelman",
            LevelKind::Trigonometric => "Trigonometrik",
        })
    }
}

/// What the windows ask for of their own.
#[derive(Clone, Debug)]
pub enum Event {
    Cell(Sheet, usize, usize, String),
    Paste(Sheet, usize, usize, String),
    Pasted(Sheet, usize, usize, Option<String>),
    Submit(Sheet, usize, usize),
    AddRow(Sheet),
    RemoveRow(Sheet, usize),
    Kind(LevelKind),
}

/// The columns: the known points (Ad, Y, X, σ), the observations
/// (Durulan, Bakılan, Doğrultu, Kenar); Kot ağı's known heights (Ad, Kot,
/// σ) and height differences (Başlangıç, Bitiş, Kot farkı, Uzunluk).
pub const KNOWN_COLS: usize = 4;
pub const ROW_COLS: usize = 4;
pub const HEIGHT_COLS: usize = 3;
pub const LEVEL_COLS: usize = 4;

/// An adjustment's outcome: what does not hold in the tables (nothing is
/// computed then), or the core's answer; each observation row's table line.
#[derive(Clone, Debug, PartialEq)]
pub struct Solved<R> {
    pub problems: Vec<String>,
    pub result: Option<Result<R, String>>,
    /// The table row of each row given to the core.
    pub lines: Vec<usize>,
}

impl<R> Default for Solved<R> {
    fn default() -> Self {
        Solved {
            problems: Vec::new(),
            result: None,
            lines: Vec::new(),
        }
    }
}

/// Yatay ağ dengelemesi's form and its last solution.
#[derive(Clone, Debug)]
pub struct NetworkForm {
    pub known: Cells,
    pub rows: Cells,
    pub layer: Option<String>,
    pub solved: Solved<NetworkResult>,
}

impl Default for NetworkForm {
    fn default() -> Self {
        NetworkForm {
            known: Cells::new(KNOWN_COLS),
            rows: Cells::new(ROW_COLS),
            layer: None,
            solved: Solved::default(),
        }
    }
}

/// Kot ağı dengelemesi's form and its last solution.
#[derive(Clone, Debug)]
pub struct LevelForm {
    pub kind: LevelKind,
    pub known: Cells,
    pub rows: Cells,
    pub solved: Solved<LevelResult>,
}

impl Default for LevelForm {
    fn default() -> Self {
        LevelForm {
            kind: LevelKind::default(),
            known: Cells::new(HEIGHT_COLS),
            rows: Cells::new(LEVEL_COLS),
            solved: Solved::default(),
        }
    }
}

/// The core's a priori standard deviations from the project's (docs/adr/0203 §1).
pub fn sigmas(settings: &ProjectSettings) -> Sigmas {
    let s = settings.sigmas();
    Sigmas {
        direction: s.direction,
        distance: s.distance,
        ppm: s.ppm,
        centering: s.centering,
        zenith: s.zenith,
        levelling: s.levelling,
    }
}

/// A cell's text, trimmed.
fn cell(row: &[String], col: usize) -> &str {
    row.get(col).map_or("", |c| js_trim(c))
}

/// The drawing's points named `name` (its label or its `Ad`, compared the
/// Turkish way, as the Hesap windows find a known point).
pub fn named_points<'a>(model: &'a Model, name: &str) -> Vec<&'a kentos_contracts::PointEntity> {
    let key = upper_tr(js_trim(name));
    model
        .entities()
        .filter_map(|e| match e {
            Entity::Point(p) => {
                let n = p
                    .base
                    .label
                    .as_deref()
                    .or_else(|| p.base.attrs.get("Ad").map(String::as_str))
                    .unwrap_or("");
                (!n.is_empty() && upper_tr(js_trim(n)) == key).then_some(p)
            }
            _ => None,
        })
        .collect()
}

/// A number typed: none when empty, NaN when no number.
fn number(text: &str) -> Option<f64> {
    read_number(text)
}

impl NetworkForm {
    /// The core's input from the tables and the drawing, or what does not
    /// hold (docs/adr/0203 §7).
    pub fn input(&self, model: &Model) -> (Option<NetworkInput>, Vec<String>, Vec<usize>) {
        let settings = model.settings();
        let mut problems = Vec::new();
        let mut known = Vec::new();
        for (i, r) in self.known.filled() {
            let name = cell(r, 0);
            if name.is_empty() {
                problems.push(format!("{}. bilinen noktanın adı yok.", i + 1));
                continue;
            }
            let (y, x) = (number(cell(r, 1)), number(cell(r, 2)));
            let p = match (y, x) {
                (None, None) => match named_points(model, name).first() {
                    Some(p) => Some((p.p.x, p.p.y)),
                    None => {
                        problems.push(format!("{name}: çizimde bu adla nokta yok; Y ve X yazın."));
                        continue;
                    }
                },
                (Some(y), Some(x)) if y.is_finite() && x.is_finite() => Some((y, x)),
                (Some(_), Some(_)) => {
                    problems.push(format!("{name}: Y ya da X bir sayı değil."));
                    continue;
                }
                _ => {
                    problems.push(format!("{name}: Y ve X birlikte yazılır."));
                    continue;
                }
            };
            let sigma = match number(cell(r, 3)) {
                None => None,
                Some(s) if s.is_finite() && s > 0.0 => Some(s / 1000.0),
                Some(_) => {
                    problems.push(format!("{name}: σ sıfırdan büyük bir sayı olmalı (mm)."));
                    continue;
                }
            };
            if let Some((y, x)) = p {
                known.push(KnownPoint {
                    name: name.to_owned(),
                    y,
                    x,
                    sigma,
                });
            }
        }
        let mut rows = Vec::new();
        let mut lines = Vec::new();
        for (i, r) in self.rows.filled() {
            let (station, target) = (cell(r, 0), cell(r, 1));
            if station.is_empty() || target.is_empty() {
                problems.push(format!("{}. gözlemde durulan ya da bakılan yok.", i + 1));
                continue;
            }
            let (direction, distance) = (number(cell(r, 2)), number(cell(r, 3)));
            if direction.is_none() && distance.is_none() {
                problems.push(format!("{}. gözlemde doğrultu ya da kenar yok.", i + 1));
                continue;
            }
            if direction.is_some_and(|v| !v.is_finite()) {
                problems.push(format!("{}. gözlemde doğrultu bir sayı değil.", i + 1));
                continue;
            }
            if distance.is_some_and(|v| !v.is_finite()) {
                problems.push(format!("{}. gözlemde kenar bir sayı değil.", i + 1));
                continue;
            }
            rows.push(NetRow {
                station: station.to_owned(),
                target: target.to_owned(),
                direction,
                distance,
                line: Some(i + 1),
            });
            lines.push(i);
        }
        if !problems.is_empty() {
            return (None, problems, lines);
        }
        // The new points' places in the drawing, where it has them.
        let mut approx: Vec<ApproxPoint> = Vec::new();
        for r in &rows {
            for name in [&r.station, &r.target] {
                if approx.iter().any(|a| a.name == *name) {
                    continue;
                }
                if let Some(p) = named_points(model, name).first() {
                    approx.push(ApproxPoint {
                        name: name.clone(),
                        y: p.p.x,
                        x: p.p.y,
                    });
                }
            }
        }
        let unit = match settings.angle_unit {
            AngleUnit::Grad => "grad",
            AngleUnit::Deg => "deg",
        };
        let input = NetworkInput {
            unit: unit.to_owned(),
            sigma: sigmas(settings),
            grid: survey_grid(settings),
            known,
            approx,
            rows,
        };
        (Some(input), problems, lines)
    }

    /// The tables adjusted again (at every change).
    pub fn solve(&mut self, model: &Model) {
        let (input, problems, lines) = self.input(model);
        self.solved = Solved {
            problems,
            result: input.map(|i| horizontal::adjust(&i)),
            lines,
        };
    }

    /// The names of the known points the table types (their rows' names).
    pub fn known_names(&self) -> Vec<String> {
        self.known
            .filled()
            .map(|(_, r)| cell(r, 0).to_owned())
            .filter(|n| !n.is_empty())
            .collect()
    }
}

impl LevelForm {
    /// The core's input from the tables and the drawing, or what does not hold.
    pub fn input(&self, model: &Model) -> (Option<LevelInput>, Vec<String>, Vec<usize>) {
        let mut problems = Vec::new();
        let mut known = Vec::new();
        for (i, r) in self.known.filled() {
            let name = cell(r, 0);
            if name.is_empty() {
                problems.push(format!("{}. bilinen noktanın adı yok.", i + 1));
                continue;
            }
            let h = match number(cell(r, 1)) {
                None => match named_points(model, name).iter().find_map(|p| p.z) {
                    Some(z) => z,
                    None => {
                        problems.push(format!(
                            "{name}: çizimde bu adla kotlu nokta yok; kotu yazın."
                        ));
                        continue;
                    }
                },
                Some(h) if h.is_finite() => h,
                Some(_) => {
                    problems.push(format!("{name}: kot bir sayı değil."));
                    continue;
                }
            };
            let sigma = match number(cell(r, 2)) {
                None => None,
                Some(s) if s.is_finite() && s > 0.0 => Some(s / 1000.0),
                Some(_) => {
                    problems.push(format!("{name}: σ sıfırdan büyük bir sayı olmalı (mm)."));
                    continue;
                }
            };
            known.push(KnownHeight {
                name: name.to_owned(),
                h,
                sigma,
            });
        }
        let mut rows = Vec::new();
        let mut lines = Vec::new();
        for (i, r) in self.rows.filled() {
            let (from, to) = (cell(r, 0), cell(r, 1));
            if from.is_empty() || to.is_empty() {
                problems.push(format!("{}. gözlemde başlangıç ya da bitiş yok.", i + 1));
                continue;
            }
            let (Some(dh), Some(length)) = (number(cell(r, 2)), number(cell(r, 3))) else {
                problems.push(format!("{}. gözlemde kot farkı ya da uzunluk yok.", i + 1));
                continue;
            };
            if !(dh.is_finite() && length.is_finite()) {
                problems.push(format!(
                    "{}. gözlemde kot farkı ya da uzunluk bir sayı değil.",
                    i + 1
                ));
                continue;
            }
            rows.push(LevelRow {
                from: from.to_owned(),
                to: to.to_owned(),
                dh,
                length,
                line: Some(i + 1),
            });
            lines.push(i);
        }
        if !problems.is_empty() {
            return (None, problems, lines);
        }
        let kind = match self.kind {
            LevelKind::Geometric => "geometric",
            LevelKind::Trigonometric => "trigonometric",
        };
        let input = LevelInput {
            kind: kind.to_owned(),
            sigma: sigmas(model.settings()),
            known,
            rows,
        };
        (Some(input), problems, lines)
    }

    pub fn solve(&mut self, model: &Model) {
        let (input, problems, lines) = self.input(model);
        self.solved = Solved {
            problems,
            result: input.map(|i| levelling::adjust(&i)),
            lines,
        };
    }

    pub fn known_names(&self) -> Vec<String> {
        self.known
            .filled()
            .map(|(_, r)| cell(r, 0).to_owned())
            .filter(|n| !n.is_empty())
            .collect()
    }
}

/// A small angle (radians) in the project's fine unit: cc in a gon project, ″ in a degree one.
pub fn fine_angle(v: f64, unit: AngleUnit) -> String {
    match unit {
        AngleUnit::Grad => format!("{} cc", fixed(v * 2_000_000.0 / std::f64::consts::PI, 1)),
        AngleUnit::Deg => format!("{}″", fixed(v * 648_000.0 / std::f64::consts::PI, 1)),
    }
}

/// A small length (m) in millimetres.
pub fn mm(v: f64) -> String {
    format!("{} mm", fixed(v * 1000.0, 1))
}

/// What the summary says of an adjustment: its counts, vᵀPv, m0 and the model test.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Counts {
    pub n: usize,
    pub u: usize,
    pub f: usize,
    pub iterations: usize,
    pub omega: f64,
    pub m0: Option<f64>,
    pub chi2: Option<f64>,
    pub passed: Option<bool>,
}

impl From<&NetworkResult> for Counts {
    fn from(r: &NetworkResult) -> Counts {
        Counts {
            n: r.n,
            u: r.u,
            f: r.f,
            iterations: r.iterations,
            omega: r.omega,
            m0: r.m0,
            chi2: r.chi2,
            passed: r.passed,
        }
    }
}

impl From<&LevelResult> for Counts {
    fn from(r: &LevelResult) -> Counts {
        Counts {
            n: r.n,
            u: r.u,
            f: r.f,
            iterations: r.iterations,
            omega: r.omega,
            m0: r.m0,
            chi2: r.chi2,
            passed: r.passed,
        }
    }
}

/// The summary's lines of an adjustment: its counts, m0 and the model test,
/// the observation with the greatest test value when one is flagged.
pub fn summary_lines(c: Counts, worst: Option<String>) -> Vec<(Line, String)> {
    let Counts {
        n,
        u,
        f,
        iterations,
        omega,
        m0,
        chi2,
        passed,
    } = c;
    let mut out = vec![(
        Line::Ok,
        format!("{n} gözlem, {u} bilinmeyen, serbestlik derecesi {f}; {iterations} yineleme."),
    )];
    match (m0, chi2, passed) {
        (Some(m0), Some(chi2), Some(true)) => out.push((
            Line::Ok,
            format!(
                "m₀ = {} (önsel 1); model testi geçti: vᵀPv {} ≤ χ²₀,₉₅({f}) {}.",
                fixed(m0, 3),
                fixed(omega, 3),
                fixed(chi2, 3)
            ),
        )),
        (Some(m0), Some(chi2), _) => out.push((
            Line::Warn,
            format!(
                "m₀ = {} (önsel 1); model testi kaldı: vᵀPv {} > χ²₀,₉₅({f}) {}. Önsel doğrulukları ya da ölçüleri denetleyin.",
                fixed(m0, 3),
                fixed(omega, 3),
                fixed(chi2, 3)
            ),
        )),
        _ => out.push((
            Line::Warn,
            "Serbestlik derecesi 0: ölçüler denetlenemez; doğruluklar önsel ağırlıklarla.".to_owned(),
        )),
    }
    if let Some(w) = worst {
        out.push((Line::Warn, w));
    }
    out
}

/// The flagged observation with the greatest test value, said.
pub fn worst_line(o: &ObservationResult, what: &str) -> String {
    format!(
        "Uyuşumsuz ölçü olabilir: {what} (w {}); önce bu ölçüyü denetleyin.",
        fixed(o.w.unwrap_or(0.0), 2)
    )
}

impl crate::app::App {
    /// A window's own event; the tables solved again after it.
    pub(crate) fn network_event(&mut self, e: Event) -> Task<Message> {
        let task = match e {
            Event::Kind(kind) => {
                self.calc.level.kind = kind;
                Task::none()
            }
            Event::Cell(sheet, row, col, text) => {
                self.network_table(sheet).set(row, col, text);
                Task::none()
            }
            Event::Paste(sheet, row, col, contents) => {
                self.network_table(sheet).set(row, col, contents);
                iced::clipboard::read().map(move |t| Sheet::msg(Event::Pasted(sheet, row, col, t)))
            }
            Event::Pasted(sheet, row, col, raw) => match raw {
                Some(raw) => match grid::paste(self.network_table(sheet), row, col, &raw) {
                    Some(at) => iced::widget::operation::focus(sheet.cell_id(at, col)),
                    None => Task::none(),
                },
                None => Task::none(),
            },
            Event::Submit(sheet, row, col) => {
                grid::submit(self.network_table(sheet), sheet, row, col)
            }
            Event::AddRow(sheet) => grid::add(self.network_table(sheet), sheet),
            Event::RemoveRow(sheet, row) => {
                let table = self.network_table(sheet);
                if table.can_remove(row) {
                    table.remove(row);
                }
                Task::none()
            }
        };
        self.network_solve();
        task
    }

    /// A table of the windows.
    fn network_table(&mut self, sheet: Sheet) -> &mut Cells {
        match sheet {
            Sheet::NetworkKnown => &mut self.calc.network.known,
            Sheet::NetworkRows => &mut self.calc.network.rows,
            Sheet::LevelKnown => &mut self.calc.level.known,
            Sheet::LevelRows => &mut self.calc.level.rows,
        }
    }

    /// Both windows solved again on the open drawing.
    pub(crate) fn network_solve(&mut self) {
        if let Some(doc) = &self.document {
            match self.calc.open {
                Some(super::Window::Network) => self.calc.network.solve(&doc.model),
                Some(super::Window::Level) => self.calc.level.solve(&doc.model),
                _ => {}
            }
        }
    }

    /// ↑ or ↓ in a field of the windows: the table the field is in.
    pub(crate) fn network_arrow(&mut self, up: bool, from: &iced::widget::Id) -> Task<Message> {
        let sheets = match self.calc.open {
            Some(super::Window::Level) => [Sheet::LevelKnown, Sheet::LevelRows],
            _ => [Sheet::NetworkKnown, Sheet::NetworkRows],
        };
        for sheet in sheets {
            let table = self.network_table(sheet);
            let owns = (0..table.rows())
                .any(|r| (0..table.columns()).any(|c| sheet.cell_id(r, c) == *from));
            if owns {
                return grid::arrow(self.network_table(sheet), sheet, from, up);
            }
        }
        Task::none()
    }
}

/// The report's head of the points and of the observations.
const NETWORK_POINTS: [&str; 10] = [
    "Nokta",
    "Y (sağa)",
    "X (yukarı)",
    "σY (mm)",
    "σX (mm)",
    "σP (mm)",
    "a (mm)",
    "b (mm)",
    "θ",
    "Kayma (mm)",
];
const NETWORK_OBSERVATIONS: [&str; 8] = ["Durulan", "Bakılan", "Tür", "Ölçü", "v", "σ", "r", "w"];
const LEVEL_POINTS: [&str; 4] = ["Nokta", "Kot (m)", "σH (mm)", "Çizimdeki kottan (mm)"];
const LEVEL_OBSERVATIONS: [&str; 7] = ["Başlangıç", "Bitiş", "Kot farkı (m)", "v", "σ", "r", "w"];

fn head(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

impl NetworkForm {
    /// Raporu kopyala's lines: the title, the summary, the points, the
    /// observations and the stations' orientations.
    pub fn report(
        &self,
        model: &Model,
        format: &kentos_interaction::Format,
    ) -> Option<Vec<Vec<String>>> {
        let Some(Ok(r)) = &self.solved.result else {
            return None;
        };
        let unit = model.settings().angle_unit;
        let mut lines = vec![vec![NETWORK_TITLE.to_owned()]];
        let worst = r.worst.map(|i| {
            worst_line(
                &r.observations[i],
                &self.observation_name(&r.observations[i]),
            )
        });
        lines.extend(
            summary_lines(Counts::from(r), worst)
                .into_iter()
                .map(|(_, t)| vec![t]),
        );
        lines.push(Vec::new());
        lines.push(head(&NETWORK_POINTS));
        lines.extend(self.point_rows(r, model, format));
        lines.push(Vec::new());
        lines.push(head(&NETWORK_OBSERVATIONS));
        lines.extend(self.observation_rows(r, unit).0);
        lines.push(Vec::new());
        lines.push(vec!["İstasyon".to_owned(), "Yöneltme".to_owned()]);
        let full = match unit {
            AngleUnit::Grad => 400.0,
            AngleUnit::Deg => 360.0,
        };
        lines.extend(r.orientations.iter().map(|o| {
            vec![
                o.station.clone(),
                fixed(o.z * full / std::f64::consts::TAU, 5),
            ]
        }));
        Some(lines)
    }
}

impl LevelForm {
    /// Raporu kopyala's lines: the title, the summary, the points, the observations.
    pub fn report(
        &self,
        model: &Model,
        format: &kentos_interaction::Format,
    ) -> Option<Vec<Vec<String>>> {
        let Some(Ok(r)) = &self.solved.result else {
            return None;
        };
        let mut lines = vec![vec![LEVEL_TITLE.to_owned(), self.kind.to_string()]];
        let worst = r.worst.map(|i| {
            worst_line(
                &r.observations[i],
                &self.observation_name(&r.observations[i]),
            )
        });
        lines.extend(
            summary_lines(Counts::from(r), worst)
                .into_iter()
                .map(|(_, t)| vec![t]),
        );
        lines.push(Vec::new());
        lines.push(head(&LEVEL_POINTS));
        lines.extend(self.point_rows(r, model, format));
        lines.push(Vec::new());
        lines.push(head(&LEVEL_OBSERVATIONS));
        lines.extend(self.observation_rows(r).0);
        Some(lines)
    }
}
