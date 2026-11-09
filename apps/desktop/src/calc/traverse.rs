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
use kentos_commands::{SurveyWork, TraverseShape};
use kentos_domain::Document as Model;
use kentos_interaction::ground::{grid_note, survey_grid};
use kentos_interaction::survey::traverse::{TraverseInput, TraverseResult, traverse};
use kentos_interaction::{Format, Vec2, fixed, js_trim};
use kentos_ui::label;
use kentos_ui::widget::Dialog;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::switch::Switch;

use super::grid::{self, Col, Table};
use super::read::{Known, known_point, read_number, resolve_point, unit_name};
use super::{
    Event, Field, Window, angle_text, event, footer, known_field, knowns, result_table, summary,
};
use crate::app::Message;
use crate::exchange::words::{self, Kind as Line};

pub const TITLE: &str = "Poligon hesabı";

/// The table's columns: Nokta, Kırılma açısı, Sonraki noktaya kenar; with the heights (docs/adr/4100) the leg's zenith
/// angle, the instrument's height at its station and the target's at its next point.
pub const NAME: usize = 0;
pub const ANGLE: usize = 1;
pub const DISTANCE: usize = 2;
pub const ZENITH: usize = 3;
pub const INSTRUMENT: usize = 4;
pub const TARGET: usize = 5;

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
    pub first: [String; 6],
    pub rows: Vec<[String; 6]>,
    pub last: [String; 6],
    pub layer: Option<String>,
    /// The heights by trigonometric heighting (docs/adr/4100): asked, and the start's and the end's heights as typed
    /// (empty: the known point's own).
    pub heights: bool,
    pub start_z: String,
    pub end_z: String,
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
            heights: false,
            start_z: String::new(),
            end_z: String::new(),
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
    /// The angles and the legs' lengths as the core takes them.
    pub angles: Vec<f64>,
    pub distances: Vec<f64>,
    /// Docs/adr/4100: each leg's zenith angle, instrument and target heights; whether the legs were taken at their own
    /// heights, or why not.
    pub zeniths: Vec<f64>,
    pub instruments: Vec<f64>,
    pub targets: Vec<f64>,
    pub own_heights: Option<Result<Option<String>, String>>,
    /// The start's and the end's heights as taken, each with whether it is the known point's own.
    pub ends_z: Option<((f64, bool), Option<(f64, bool)>)>,
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

/// A known point's own height (docs/adr/4100): the drawing's point it names (or sits on) when it has one.
fn point_height(model: &Model, text: &str) -> Option<f64> {
    let Known::Point { p, .. } = resolve_point(model, text) else { return None };
    model.entities().find_map(|e| match e {
        kentos_commands::Entity::Point(pt) if pt.p.x == p.x && pt.p.y == p.y => pt.z,
        _ => None,
    })
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
        let mut rows: &[[String; 6]] = &self.rows;
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
        let (mut zeniths, mut instruments, mut targets) = (Vec::new(), Vec::new(), Vec::new());
        for (i, row) in std::iter::once(&self.first).chain(legs).enumerate() {
            let a = read_number(&row[ANGLE]);
            let d = read_number(&row[DISTANCE]);
            let place = if i == 0 {
                "Başlangıç satırında".to_owned()
            } else {
                format!("{}. satırda", i + 1)
            };
            // Docs/adr/4100: the leg's zenith angle; the instrument's and the target's heights (empty: 0).
            if self.heights {
                match read_number(&row[ZENITH]) {
                    None => errors.push(format!("{place} başucu açısı yok.")),
                    Some(v) if v.is_nan() => errors.push(format!("{place} başucu açısı bir sayı değil.")),
                    Some(_) => {}
                }
                zeniths.push(read_number(&row[ZENITH]).unwrap_or(f64::NAN));
                for (col, what, list) in
                    [(INSTRUMENT, "alet yüksekliği", &mut instruments), (TARGET, "işaret yüksekliği", &mut targets)]
                {
                    let v = read_number(&row[col]).unwrap_or(0.0);
                    if v.is_nan() {
                        errors.push(format!("{place} {what} bir sayı değil."));
                    }
                    list.push(v);
                }
            }
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
        // Docs/adr/4100: the start's height (typed, else the known point's own) and the end's.
        let mut heights = None;
        let mut ends_z = None;
        if self.heights {
            // Each height typed, else the known point's own (`true`).
            let typed_or_own = |typed: &str, point: &str| {
                read_number(typed).map(|z| (z, false)).or_else(|| point_height(model, point).map(|z| (z, true)))
            };
            match typed_or_own(&self.start_z, &self.start) {
                None => errors.push("Başlangıç noktasının kotu yok: kotunu yazın ya da kotu olan bir nokta seçin.".to_owned()),
                Some((v, _)) if v.is_nan() => errors.push("Başlangıç kotu bir sayı değil.".to_owned()),
                Some((z0, own)) => {
                    let end = match self.kind {
                        Kind::Closed => Some((z0, own)),
                        Kind::Open => None,
                        Kind::Connected => typed_or_own(&self.end_z, &self.end),
                    };
                    let z1 = end.map(|(z, _)| z);
                    if z1.is_some_and(f64::is_nan) {
                        errors.push("Bitiş kotu bir sayı değil.".to_owned());
                    }
                    ends_z = Some(((z0, own), end));
                    heights = Some(kentos_interaction::survey::traverse::TraverseHeights {
                        start: z0,
                        end: z1,
                        zeniths: zeniths.clone(),
                        instruments: instruments.clone(),
                        targets: targets.clone(),
                        refraction: Some(model.settings().refraction()),
                        undulations: None,
                    });
                }
            }
        }
        let mut result = None;
        let mut end_name = None;
        let mut own_heights = None;
        if let (true, Some(start), Some(back)) = (errors.is_empty(), start, back) {
            let mut input = TraverseInput {
                unit: unit_name(model.settings().angle_unit).to_owned(),
                start,
                back,
                end,
                fore,
                angles: angles.clone(),
                distances: distances.clone(),
                grid: survey_grid(model.settings()),
                heights,
            };
            // Each leg at its own height when the project's heights can be made ellipsoidal (docs/adr/2900, 4100).
            own_heights = kentos_native_application::survey_heights::traverse_undulations(model.settings(), &input)
                .map(|r| r.map(|(u, name)| {
                    if let Some(h) = input.heights.as_mut() {
                        h.undulations = Some(u);
                    }
                    name
                }));
            match traverse(&input) {
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
            angles,
            distances,
            zeniths,
            instruments,
            targets,
            own_heights,
            ends_z,
        }
    }

    /// What `cad.survey.points` is asked to compute and write (docs/adr/0622 §9): the known points as typed, the
    /// angles, the lengths and the names; none while the fields do not compute.
    pub fn work(&self, model: &Model) -> Option<SurveyWork> {
        let c = self.compute(model);
        if !c.errors.is_empty() || c.result.is_none() {
            return None;
        }
        let (shape, end, fore) = match self.kind {
            Kind::Connected => (
                TraverseShape::Connected,
                Some(known_point(model, &self.end)?),
                if self.end_oriented {
                    Some(known_point(model, &self.fore)?)
                } else {
                    None
                },
            ),
            Kind::Closed => (TraverseShape::Closed, None, None),
            Kind::Open => (TraverseShape::Open, None, None),
        };
        Some(SurveyWork::Traverse {
            shape,
            start: known_point(model, &self.start)?,
            back: known_point(model, &self.back)?,
            end,
            fore,
            angles: c.angles,
            distances: c.distances,
            names: c.names,
            // Docs/adr/4100: the typed heights (empty: the known points' own) and the legs' readings.
            heights: self.heights.then(|| kentos_commands::SurveyTraverseHeights {
                start: read_number(&self.start_z),
                end: read_number(&self.end_z),
                zeniths: c.zeniths,
                instruments: c.instruments,
                targets: c.targets,
            }),
        })
    }

    pub fn report(&self, model: &Model, format: &Format) -> Option<Vec<Vec<String>>> {
        let c = self.compute(model);
        let r = c.result.as_ref()?;
        // With the project's grid: the length on the ground as measured and the leg's factors (docs/adr/0171 §4).
        let reduced = r.legs.iter().any(|l| l.ground.is_some());
        let mut head = vec!["Nokta", "Semt"];
        if reduced {
            head.push("Zeminde");
        }
        head.extend(["Kenar", "ΔY", "ΔX", "vY", "vX", "Y", "X"]);
        if reduced {
            head.extend(["Ölçek", "Yükseklik çarpanı"]);
        }
        // Docs/adr/4100: the heights: each leg's height difference, its correction, the point's height.
        let heights = r.heights.as_ref();
        if heights.is_some() {
            head.extend(["Δh", "vh", "H"]);
        }
        let mut lines = vec![
            vec![TITLE.to_owned()],
            head.into_iter().map(str::to_owned).collect(),
        ];
        for (i, leg) in r.legs.iter().enumerate() {
            let p = c.leg_point(r, i);
            let mut row = vec![c.leg_name(i), fixed(leg.bearing, 4)];
            if reduced {
                row.push(
                    leg.ground
                        .map(|g| format.length_bare(g))
                        .unwrap_or_default(),
                );
            }
            row.extend([
                format.length_bare(leg.distance),
                format.length_bare(leg.dy),
                format.length_bare(leg.dx),
                fixed(leg.vy, 4),
                fixed(leg.vx, 4),
                p.map(|p| format.coord(p.x)).unwrap_or_default(),
                p.map(|p| format.coord(p.y)).unwrap_or_default(),
            ]);
            if reduced {
                row.push(leg.scale.map(|k| fixed(k, 8)).unwrap_or_default());
                row.push(leg.height_factor.map(|k| fixed(k, 8)).unwrap_or_default());
            }
            if let Some(h) = heights {
                row.push(leg.dh.map(|d| fixed(d, 4)).unwrap_or_default());
                row.push(leg.vh.map(|v| fixed(v, 4)).unwrap_or_default());
                row.push(h.get(i).map_or_else(|| "bilinen".to_owned(), |z| fixed(*z, 3)));
            }
            lines.push(row);
        }
        if let Some(f) = r.height_misclosure {
            lines.push(vec!["Kot kapanma hatası".to_owned(), mm_text(f)]);
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
        settings: &kentos_commands::ProjectSettings,
        c: &Computed,
    ) -> Vec<(Line, String)> {
        let own = &c.own_heights;
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
        // Docs/adr/4100: the heights' closure, and the height each leg was reduced at.
        if r.heights.is_some() {
            // Where the start's and the end's heights come from: typed, or the known point's own.
            let said = |(z, own): (f64, bool)| {
                format!("{} m{}", format.length_bare(z), if own { ", noktanın kendi kotu" } else { "" })
            };
            let ends = match c.ends_z {
                Some((start, Some(end))) if self.kind == Kind::Connected => {
                    format!(" (başlangıç {}; bitiş {})", said(start), said(end))
                }
                Some((start, _)) => format!(" (başlangıç {})", said(start)),
                None => String::new(),
            };
            lines.push((
                Line::Info,
                match r.height_misclosure {
                    Some(f) => format!(
                        "Kot kapanma hatası fh = {}{ends}; kenarlara uzunluklarıyla orantılı dağıtıldı. Yer eğriliği ve refraksiyon: (1 − k)·D²/2R, k = {} (Proje ayarları › Ölçme).",
                        mm_text(f),
                        settings.refraction()
                    ),
                    None => format!(
                        "Kotlar trigonometrik nivelmanla{ends}; bitişin kotu yok, kot kapanması denetlenmedi. Yer eğriliği ve refraksiyon: (1 − k)·D²/2R, k = {}.",
                        settings.refraction()
                    ),
                },
            ));
        }
        match own {
            Some(Ok(model)) => lines.push((
                Line::Info,
                match model {
                    Some(name) => format!("Her kenar kendi elipsoit yüksekliğinde indirildi (uçlarının H + N ortalaması; geoit {name})."),
                    None => "Her kenar kendi elipsoit yüksekliğinde indirildi (uçlarının kotlarının ortalaması).".to_owned(),
                },
            )),
            Some(Err(why)) => lines.push((Line::Warn, why.clone())),
            None => {}
        }
        if let Some(h) = settings
            .ground_height()
            .filter(|_| r.legs.iter().any(|l| l.ground.is_some()) && !matches!(own, Some(Ok(_))))
        {
            lines.push((Line::Info, grid_note(h)));
        }
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
        // Docs/adr/4100: the heights by trigonometric heighting.
        top = top.push(words::field(
            "Kotlar",
            Switch::new(self.heights, |on| event(Event::TraverseHeights(on))).label("Trigonometrik nivelmanla hesapla"),
            None,
        ));
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
        // With the heights' six columns the leg's header is short; the caption says it is to the next point.
        let all: &'static [Col] = match (format.angle_unit_label() == "°", self.heights) {
            (true, false) => &COLUMNS_DEG,
            (false, false) => &COLUMNS_GRAD,
            (true, true) => &HEIGHTS_DEG,
            (false, true) => &HEIGHTS_GRAD,
        };
        let columns = &all[..Table::columns(self)];
        let table = grid::view(Window::Traverse, columns, self, |r, col| match col {
            NAME => format!("P{r}"),
            INSTRUMENT | TARGET => "0".to_owned(),
            _ => String::new(),
        });
        // The start's and the end's heights: empty, the known point's own.
        let heights_row = self.heights.then(|| {
            let mut r = row![super::number_field("Başlangıç kotu (m)", &self.start_z, "noktanın kotu", |t| {
                event(Event::TraverseStartZ(t))
            })]
            .spacing(18);
            if self.kind == Kind::Connected {
                r = r.push(super::number_field("Bitiş kotu (m)", &self.end_z, "noktanın kotu", |t| {
                    event(Event::TraverseEndZ(t))
                }));
            }
            r
        });
        let mut head = column![top, knowns(fields)].spacing(12);
        if let Some(r) = heights_row {
            head = head.push(r);
        }
        let mut body = column![
            head,
            column![
                label::caption(if self.heights {
                    "Kırılma açısı her istasyonda önceki noktadan sonraki noktaya saat yönünde ölçülür (başlangıçta bakılan noktadan). Kenar (yatay), başucu açısı (0 tam yukarı), alet ve işaret yüksekliği o istasyondan sonraki noktaya okunur: Δh = S·cot z + i − t. Elektronik tablodan satırları yapıştırabilirsiniz."
                } else {
                    "Kırılma açısı her istasyonda önceki noktadan sonraki noktaya saat yönünde ölçülür (başlangıçta bakılan noktadan). Kenarlar yataydır. Elektronik tablodan satırları yapıştırabilirsiniz."
                })
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
            Some(r) => self.summary_lines(r, format, model.settings(), &c),
        };
        if let Some(summary) = summary(lines) {
            body = body.push(summary);
        }
        if let Some(r) = &c.result {
            // With the project's grid the measured length comes before the grid's (docs/adr/0171 §4).
            let reduced = r.legs.iter().any(|l| l.ground.is_some());
            let rows = r
                .legs
                .iter()
                .enumerate()
                .map(|(i, leg)| {
                    let p = c.leg_point(r, i);
                    let mut row = vec![c.leg_name(i), angle_text(format, leg.bearing)];
                    if reduced {
                        row.push(
                            leg.ground
                                .map(|g| format.length_bare(g))
                                .unwrap_or_default(),
                        );
                    }
                    row.extend([
                        format.length_bare(leg.distance),
                        format.length_bare(leg.dy),
                        format.length_bare(leg.dx),
                        p.map_or_else(|| "bilinen".to_owned(), |p| format.coord(p.x)),
                        p.map_or_else(|| "bilinen".to_owned(), |p| format.coord(p.y)),
                    ]);
                    // Docs/adr/4100: the point's height.
                    if let Some(h) = &r.heights {
                        row.push(h.get(i).map_or_else(|| "bilinen".to_owned(), |z| format.length_bare(*z)));
                    }
                    row
                })
                .collect();
            let table = if r.heights.is_some() {
                let mut head: Vec<&str> = vec!["Nokta", "Semt"];
                if reduced {
                    head.extend(["Zeminde (m)", "Düzlemde (m)"]);
                } else {
                    head.push("Kenar (m)");
                }
                head.extend(["ΔY (m)", "ΔX (m)", "Y (sağa)", "X (yukarı)", "H (m)"]);
                let numeric: Vec<bool> = (0..head.len()).map(|k| k > 0).collect();
                result_table(&head, rows, &numeric)
            } else if reduced {
                result_table(
                    &[
                        "Nokta",
                        "Semt",
                        "Zeminde (m)",
                        "Düzlemde (m)",
                        "ΔY (m)",
                        "ΔX (m)",
                        "Y (sağa)",
                        "X (yukarı)",
                    ],
                    rows,
                    &[false, true, true, true, true, true, true, true],
                )
            } else {
                result_table(
                    &[
                        "Nokta",
                        "Semt",
                        "Kenar (m)",
                        "ΔY (m)",
                        "ΔX (m)",
                        "Y (sağa)",
                        "X (yukarı)",
                    ],
                    rows,
                    &[false, true, true, true, true, true, true],
                )
            };
            body = body.push(column![label::strong("Sonuç"), table].spacing(6));
        }
        let new_points = c.result.as_ref().is_some_and(|r| !r.points.is_empty());
        footer(
            Dialog::new(TITLE)
                .on_close(crate::calc::event(crate::calc::Event::Close))
                .scroll(body),
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
    /// Docs/adr/4100: with the heights, the legs' zenith angles and heights too.
    fn columns(&self) -> usize {
        if self.heights { 6 } else { 3 }
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
            return col != ANGLE || !self.end_angle();
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

const COLUMNS_GRAD: [Col; 6] = columns("g", "Sonraki noktaya kenar");
const COLUMNS_DEG: [Col; 6] = columns("°", "Sonraki noktaya kenar");
const HEIGHTS_GRAD: [Col; 6] = columns("g", "Kenar");
const HEIGHTS_DEG: [Col; 6] = columns("°", "Kenar");

const fn columns(unit: &'static str, leg: &'static str) -> [Col; 6] {
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
            label: leg,
            unit: Some("m"),
            numeric: true,
        },
        // Docs/adr/4100: the heights' readings.
        Col {
            label: "Başucu açısı",
            unit: Some(unit),
            numeric: true,
        },
        Col {
            label: "Alet yüksekliği",
            unit: Some("m"),
            numeric: true,
        },
        Col {
            label: "İşaret yüksekliği",
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

fn checked(r: &TraverseResult, settings: &kentos_commands::ProjectSettings) -> Checked {
    use kentos_interaction::survey::Unit;
    use kentos_interaction::survey::traverse::closure;
    let unit = settings.angle_unit;
    let texts = kentos_project::survey_form::texts(settings.survey.as_ref(), unit);
    let survey = settings.survey.clone().unwrap_or_default();
    let core = match unit {
        kentos_commands::AngleUnit::Grad => Unit::GRAD,
        kentos_commands::AngleUnit::Deg => Unit::DEG,
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
    settings: &kentos_commands::ProjectSettings,
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
