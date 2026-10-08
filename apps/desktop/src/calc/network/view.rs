//! The two windows' views (docs/adr/0203 §7; the web's `NetworkDialog.ts`):
//! the known points and the observations side by side, the a priori
//! standard deviations, the summary, the points' and the observations'
//! results, the footer.

use iced::widget::{Column, column, container, row};
use iced::{Center, Element, Fill, FillPortion};
use kentos_contracts::AngleUnit;
use kentos_domain::Document as Model;
use kentos_interaction::ground::grid_note;
use kentos_interaction::{Format, fixed};
use kentos_ui::label;
use kentos_ui::widget::Dialog;
use kentos_ui::widget::segmented::Segmented;

use super::super::grid::{self, Col};
use super::super::parts::{footer_button, layer_select, result_table, result_table_marked};
use super::super::{Event as Calc, MAX_HEIGHT, event, summary};
use super::{
    Counts, LEVEL_TITLE, LevelForm, LevelKind, NETWORK_TITLE, NetworkForm, Sheet, fine_angle, mm,
    named_points, summary_lines, worst_line,
};
use crate::app::Message;
use crate::exchange::words::{self, Kind as Line};

const fn col(label: &'static str, unit: Option<&'static str>, numeric: bool) -> Col {
    Col {
        label,
        unit,
        numeric,
    }
}

const KNOWN: [Col; 4] = [
    col("Ad", None, false),
    col("Y", Some("m"), true),
    col("X", Some("m"), true),
    col("σ", Some("mm"), true),
];
const ROWS_GRAD: [Col; 4] = [
    col("Durulan", None, false),
    col("Bakılan", None, false),
    col("Doğrultu", Some("g"), true),
    col("Kenar", Some("m"), true),
];
const ROWS_DEG: [Col; 4] = [
    col("Durulan", None, false),
    col("Bakılan", None, false),
    col("Doğrultu", Some("°"), true),
    col("Kenar", Some("m"), true),
];
const HEIGHTS: [Col; 3] = [
    col("Ad", None, false),
    col("Kot", Some("m"), true),
    col("σ", Some("mm"), true),
];
const LEVELS: [Col; 4] = [
    col("Başlangıç", None, false),
    col("Bitiş", None, false),
    col("Kot farkı", Some("m"), true),
    col("Uzunluk", Some("m"), true),
];

/// The a priori standard deviations the adjustment weighs with (the project's, docs/adr/0203 §1).
fn sigma_line(model: &Model, level: Option<LevelKind>) -> String {
    let settings = model.settings();
    let s = settings.sigmas();
    let unit = settings.angle_unit;
    let named = settings.survey.as_ref().is_some_and(|s| s.has_sigmas());
    let whose = if named { "projenin" } else { "varsayılan" };
    let what = match level {
        None => format!(
            "doğrultu {}, kenar {} + {} ppm, merkezleme {}",
            fine_angle(s.direction, unit),
            mm(s.distance),
            fixed(s.ppm, 1),
            mm(s.centering)
        ),
        Some(LevelKind::Geometric) => format!("nivelman {} /√km", mm(s.levelling)),
        Some(LevelKind::Trigonometric) => format!(
            "başucu açısı {}, kenar {} + {} ppm",
            fine_angle(s.zenith, unit),
            mm(s.distance),
            fixed(s.ppm, 1)
        ),
    };
    format!("Önsel doğruluklar ({whose}): {what}. Proje ayarları › Ölçme'de değiştirilir.")
}

/// The two tables side by side under their titles.
fn tables<'a>(left: Element<'a, Message>, right: Element<'a, Message>) -> Element<'a, Message> {
    row![
        container(left).width(FillPortion(2)),
        container(right).width(FillPortion(3)),
    ]
    .spacing(18)
    .into()
}

fn titled<'a>(title: &'a str, note: &'a str, table: Element<'a, Message>) -> Element<'a, Message> {
    column![
        label::strong(title),
        label::caption(note).width(Fill),
        table
    ]
    .spacing(6)
    .into()
}

/// The footer: Raporu kopyala, the layer (Yatay ağ), Çizime yaz, Kapat.
fn footer<'a>(
    dialog: Dialog<'a, Message>,
    layer: Option<Element<'a, Message>>,
    done: bool,
) -> Dialog<'a, Message> {
    let mut dialog = dialog.action(footer_button(
        "Raporu kopyala",
        done.then(|| event(Calc::CopyReport)),
        false,
    ));
    if let Some(layer) = layer {
        dialog = dialog.action(
            row![label::caption("Katman"), layer]
                .spacing(8)
                .align_y(Center),
        );
    }
    dialog
        .action(footer_button(
            "Çizime yaz",
            done.then(|| event(Calc::AddPoints)),
            true,
        ))
        .action(footer_button("Kapat", Some(event(Calc::Close)), false))
        .max_height(MAX_HEIGHT)
}

/// An observation's kind as the table names it.
fn kind_word(kind: &str) -> &'static str {
    match kind {
        "direction" => "Doğrultu",
        "distance" => "Kenar",
        "y" => "Y (ağırlıklı)",
        "x" => "X (ağırlıklı)",
        "dh" => "Kot farkı",
        _ => "Kot (ağırlıklı)",
    }
}

impl NetworkForm {
    pub fn view<'a>(&'a self, model: &Model, format: &Format) -> Element<'a, Message> {
        let unit = model.settings().angle_unit;
        let rows_cols: &'static [Col] = match unit {
            AngleUnit::Grad => &ROWS_GRAD,
            AngleUnit::Deg => &ROWS_DEG,
        };
        let known = grid::view_with(
            Sheet::NetworkKnown,
            &KNOWN,
            &self.known,
            |_, c| match c {
                1 | 2 => "çizimdeki".to_owned(),
                3 => "sabit".to_owned(),
                _ => String::new(),
            },
            0,
            |_| Vec::new(),
        );
        let rows = grid::view_with(
            Sheet::NetworkRows,
            rows_cols,
            &self.rows,
            |_, _| String::new(),
            0,
            |_| Vec::new(),
        );
        let mut body = Column::new().spacing(14).push(tables(
            titled(
                "Bilinen noktalar",
                "Y ve X boşsa çizimdeki aynı adlı nokta; σ boşsa sabit, yazılırsa ağırlıklı.",
                known,
            ),
            titled(
                "Gözlemler",
                "Bir istasyonun bütün doğrultuları bir seridir; kenar yataydır. Elektronik tablodan satırlar yapıştırılabilir.",
                rows,
            ),
        ));
        let mut notes = vec![sigma_line(model, None)];
        if let Some(h) = kentos_interaction::ground::survey_grid(model.settings()).map(|g| g.height)
        {
            notes.push(grid_note(h));
        }
        body = body.push(label::caption(notes.join(" ")).width(Fill));
        let s = &self.solved;
        let lines: Vec<(Line, String)> = match &s.result {
            None => s
                .problems
                .iter()
                .take(6)
                .map(|p| (Line::Warn, p.clone()))
                .collect(),
            Some(Err(e)) => vec![(Line::Error, e.clone())],
            Some(Ok(r)) => summary_lines(
                Counts::from(r),
                r.worst.map(|i| {
                    worst_line(
                        &r.observations[i],
                        &self.observation_name(&r.observations[i]),
                    )
                }),
            ),
        };
        if let Some(summary) = summary(lines) {
            body = body.push(summary);
        }
        if let Some(Ok(r)) = &s.result {
            let points = self.point_rows(r, model, format);
            let theta = match unit {
                AngleUnit::Grad => "θ (g)",
                AngleUnit::Deg => "θ (°)",
            };
            body = body.push(
                column![
                    label::strong("Noktalar"),
                    result_table(
                        &[
                            "Nokta",
                            "Y (sağa)",
                            "X (yukarı)",
                            "σY (mm)",
                            "σX (mm)",
                            "σP (mm)",
                            "a (mm)",
                            "b (mm)",
                            theta,
                            "Kayma (mm)",
                        ],
                        points,
                        &[false, true, true, true, true, true, true, true, true, true],
                    )
                ]
                .spacing(6),
            );
            let (rows, marked) = self.observation_rows(r, unit);
            body = body.push(
                column![
                    label::strong("Gözlemler"),
                    result_table_marked(
                        &["Durulan", "Bakılan", "Tür", "Ölçü", "v", "σ", "r", "w"],
                        rows,
                        &[false, false, false, true, true, true, true, true],
                        &marked,
                    )
                ]
                .spacing(6),
            );
        }
        let done = matches!(&s.result, Some(Ok(_)));
        footer(
            Dialog::new(NETWORK_TITLE).scroll(body),
            Some(layer_select(model, self.layer.as_deref())),
            done,
        )
        .width(1040.0)
        .into()
    }

    /// The points' result rows: name, place, standard deviations, error
    /// ellipse, and how far from the drawing's point of the same name (mm).
    pub fn point_rows(
        &self,
        r: &kentos_geometry_core::survey::adjust::horizontal::NetworkResult,
        model: &Model,
        format: &Format,
    ) -> Vec<Vec<String>> {
        let unit = model.settings().angle_unit;
        r.points
            .iter()
            .map(|p| {
                let shift = named_points(model, &p.name).first().map_or_else(
                    || "yeni".to_owned(),
                    |d| {
                        let (dy, dx) = (d.p.x - p.y, d.p.y - p.x);
                        fixed((dy * dy + dx * dx).sqrt() * 1000.0, 1)
                    },
                );
                vec![
                    p.name.clone(),
                    format.coord(p.y),
                    format.coord(p.x),
                    fixed(p.sy * 1000.0, 1),
                    fixed(p.sx * 1000.0, 1),
                    fixed(p.sp * 1000.0, 1),
                    fixed(p.a * 1000.0, 1),
                    fixed(p.b * 1000.0, 1),
                    fixed(p.theta * unit_full(unit) / std::f64::consts::TAU, 2),
                    shift,
                ]
            })
            .collect()
    }

    /// An observation as the summary names it: “K2 → Y1 kenarı”.
    pub fn observation_name(
        &self,
        o: &kentos_geometry_core::survey::adjust::ObservationResult,
    ) -> String {
        match o.kind {
            "y" | "x" => {
                let name = self.known_names().get(o.row).cloned().unwrap_or_default();
                format!(
                    "{name} noktasının {}'i",
                    if o.kind == "y" { "Y" } else { "X" }
                )
            }
            kind => {
                let (station, target) = self.row_names(o.row);
                let what = if kind == "direction" {
                    "doğrultusu"
                } else {
                    "kenarı"
                };
                format!("{station} → {target} {what}")
            }
        }
    }

    /// A given row's station and target as typed.
    fn row_names(&self, row: usize) -> (String, String) {
        let line = self.solved.lines.get(row).copied().unwrap_or(0);
        let r = self.rows.rows.get(line);
        let get = |c: usize| {
            r.and_then(|r| r.get(c))
                .map(|t| kentos_interaction::js_trim(t).to_owned())
                .unwrap_or_default()
        };
        (get(0), get(1))
    }

    /// The observations' result rows and which are flagged.
    pub fn observation_rows(
        &self,
        r: &kentos_geometry_core::survey::adjust::horizontal::NetworkResult,
        unit: AngleUnit,
    ) -> (Vec<Vec<String>>, Vec<bool>) {
        let names = self.known_names();
        let mut rows = Vec::new();
        let mut marked = Vec::new();
        for o in &r.observations {
            let (a, b) = match o.kind {
                "y" | "x" => (
                    names.get(o.row).cloned().unwrap_or_default(),
                    "—".to_owned(),
                ),
                _ => self.row_names(o.row),
            };
            let line = self.solved.lines.get(o.row).copied();
            let typed = |c: usize| {
                line.and_then(|l| self.rows.rows.get(l))
                    .and_then(|r| r.get(c))
                    .map(|t| kentos_interaction::js_trim(t).to_owned())
                    .unwrap_or_default()
            };
            let (observed, v, sigma) = match o.kind {
                "direction" => (typed(2), fine_angle(o.v, unit), fine_angle(o.sigma, unit)),
                "distance" => (typed(3), mm(o.v), mm(o.sigma)),
                _ => (String::new(), mm(o.v), mm(o.sigma)),
            };
            rows.push(vec![
                a,
                b,
                kind_word(o.kind).to_owned(),
                observed,
                v,
                sigma,
                fixed(o.r, 2),
                o.w.map_or_else(|| "—".to_owned(), |w| fixed(w, 2)),
            ]);
            marked.push(o.flag == "blunder");
        }
        (rows, marked)
    }
}

impl LevelForm {
    pub fn view<'a>(&'a self, model: &Model, format: &Format) -> Element<'a, Message> {
        let kind = Segmented::new(
            [LevelKind::Geometric, LevelKind::Trigonometric],
            self.kind,
            |k| super::Sheet::msg(super::Event::Kind(k)),
        );
        let known = grid::view_with(
            Sheet::LevelKnown,
            &HEIGHTS,
            &self.known,
            |_, c| match c {
                1 => "çizimdeki".to_owned(),
                2 => "sabit".to_owned(),
                _ => String::new(),
            },
            0,
            |_| Vec::new(),
        );
        let rows = grid::view_with(
            Sheet::LevelRows,
            &LEVELS,
            &self.rows,
            |_, _| String::new(),
            0,
            |_| Vec::new(),
        );
        let length_note = match self.kind {
            LevelKind::Geometric => "Uzunluk nivelman hattınındır.",
            LevelKind::Trigonometric => "Uzunluk yatay uzunluktur.",
        };
        let mut body = Column::new()
            .spacing(14)
            .push(words::field("Ölçü türü", kind, None))
            .push(tables(
                titled(
                    "Bilinen kotlar",
                    "Kot boşsa çizimdeki aynı adlı noktanın kotu; σ boşsa sabit, yazılırsa ağırlıklı.",
                    known,
                ),
                titled("Kot farkları", length_note, rows),
            ))
            .push(label::caption(sigma_line(model, Some(self.kind))).width(Fill));
        let s = &self.solved;
        let lines: Vec<(Line, String)> = match &s.result {
            None => s
                .problems
                .iter()
                .take(6)
                .map(|p| (Line::Warn, p.clone()))
                .collect(),
            Some(Err(e)) => vec![(Line::Error, e.clone())],
            Some(Ok(r)) => summary_lines(
                Counts::from(r),
                r.worst.map(|i| {
                    worst_line(
                        &r.observations[i],
                        &self.observation_name(&r.observations[i]),
                    )
                }),
            ),
        };
        if let Some(summary) = summary(lines) {
            body = body.push(summary);
        }
        if let Some(Ok(r)) = &s.result {
            let points = self.point_rows(r, model, format);
            body = body.push(
                column![
                    label::strong("Noktalar"),
                    result_table(
                        &["Nokta", "Kot (m)", "σH (mm)", "Çizimdeki kottan (mm)"],
                        points,
                        &[false, true, true, true],
                    )
                ]
                .spacing(6),
            );
            let (rows, marked) = self.observation_rows(r);
            body = body.push(
                column![
                    label::strong("Gözlemler"),
                    result_table_marked(
                        &["Başlangıç", "Bitiş", "Kot farkı (m)", "v", "σ", "r", "w"],
                        rows,
                        &[false, false, true, true, true, true, true],
                        &marked,
                    )
                ]
                .spacing(6),
            );
        }
        let done = matches!(&s.result, Some(Ok(_)));
        footer(Dialog::new(LEVEL_TITLE).scroll(body), None, done)
            .width(1040.0)
            .into()
    }

    /// The points' result rows: name, height, its standard deviation, and
    /// how far from the drawing's height of the same name (mm).
    pub fn point_rows(
        &self,
        r: &kentos_geometry_core::survey::adjust::levelling::LevelResult,
        model: &Model,
        format: &Format,
    ) -> Vec<Vec<String>> {
        r.points
            .iter()
            .map(|p| {
                let shift = named_points(model, &p.name)
                    .first()
                    .and_then(|d| d.z)
                    .map_or_else(|| "—".to_owned(), |z| fixed((p.h - z) * 1000.0, 1));
                vec![
                    p.name.clone(),
                    format.length_bare(p.h),
                    fixed(p.sh * 1000.0, 1),
                    shift,
                ]
            })
            .collect()
    }

    /// The observations' result rows and which are flagged.
    pub fn observation_rows(
        &self,
        r: &kentos_geometry_core::survey::adjust::levelling::LevelResult,
    ) -> (Vec<Vec<String>>, Vec<bool>) {
        let names = self.known_names();
        let mut rows = Vec::new();
        let mut marked = Vec::new();
        for o in &r.observations {
            let (a, b, observed) = if o.kind == "h" {
                (
                    names.get(o.row).cloned().unwrap_or_default(),
                    "—".to_owned(),
                    String::new(),
                )
            } else {
                let (a, b) = self.row_names(o.row);
                let line = self.solved.lines.get(o.row).copied();
                let dh = line
                    .and_then(|l| self.rows.rows.get(l))
                    .and_then(|r| r.get(2))
                    .map(|t| kentos_interaction::js_trim(t).to_owned())
                    .unwrap_or_default();
                (a, b, dh)
            };
            rows.push(vec![
                a,
                b,
                observed,
                mm(o.v),
                mm(o.sigma),
                fixed(o.r, 2),
                o.w.map_or_else(|| "—".to_owned(), |w| fixed(w, 2)),
            ]);
            marked.push(o.flag == "blunder");
        }
        (rows, marked)
    }

    /// An observation as the summary names it: “N2 → R2 kot farkı”.
    pub fn observation_name(
        &self,
        o: &kentos_geometry_core::survey::adjust::ObservationResult,
    ) -> String {
        if o.kind == "h" {
            let name = self.known_names().get(o.row).cloned().unwrap_or_default();
            return format!("{name} noktasının kotu");
        }
        let (a, b) = self.row_names(o.row);
        format!("{a} → {b} kot farkı")
    }

    fn row_names(&self, row: usize) -> (String, String) {
        let line = self.solved.lines.get(row).copied().unwrap_or(0);
        let r = self.rows.rows.get(line);
        let get = |c: usize| {
            r.and_then(|r| r.get(c))
                .map(|t| kentos_interaction::js_trim(t).to_owned())
                .unwrap_or_default()
        };
        (get(0), get(1))
    }
}

/// A full turn in the project's angle unit.
fn unit_full(unit: AngleUnit) -> f64 {
    match unit {
        AngleUnit::Grad => 400.0,
        AngleUnit::Deg => 360.0,
    }
}
