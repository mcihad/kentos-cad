//! Yüzey analizi (plan/road.md Y18.5, docs/adr/0916 §5, `terrain.analysis`): a terrain surface read three ways, nothing
//! written. Analiz: elevation, slope or aspect ranges (Civil 3D's Elevations, Slopes, Directions) coloured in plan with
//! their plan and 3D areas, the roughness and a raster's slope against the triangles' (`road.terrains.analysis`). Fark:
//! a second surface over the first, cut, fill and net and the difference's ranges (`road.terrains.compare`). Kalite: near
//! pairs, spikes, coverage gaps and check points by class (`road.terrains.quality`); the spikes found can be left out
//! of the surface, the difference said first and written in one undo step, or everything left out taken back
//! (`road.terrains.repair`, Y18.7). The breaks and limits are the user's; KentOS fills in none.
//!
//! ```text
//! ┌ Yüzey analizi ──────────────────────────────────────────────────────────────────────────────────┐
//! │ Kip [Analiz ▾] Yüzey [▾]  (Analiz) Ölçü [Eğim ▾] Sınırlar [0, 2, 5] Pürüzlülük [▾] Raster [ ]     │
//! │                           (Fark) Karşılaştırma [▾] Aralıklar [ ]                                  │
//! │                           (Kalite) Yakın [ ] Erim [ ] Sınır [ ] Kenar [ ] Kontrol [▾] Sınıf [ ]   │
//! │ ┌ the ranges in plan / the difference's ranges / the check points ┐  tables                 [Önizle]│
//! │ └──────────────────────────────────────────────────────────────────┘                       [Kapat] │
//! └──────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    TerrainBands, TerrainCheckpointsAsk, TerrainSpikesAsk, TerrainsAnalysed, TerrainsAnalysis, TerrainsCompare,
    TerrainsCompared, TerrainsQuality, TerrainsQualityRead, TerrainsRepair, TerrainsRepaired,
};
use kentos_interaction::Level;
use kentos_native_application::{
    ExecutionContext, road_terrains_analysis, road_terrains_compare, road_terrains_quality, road_terrains_repair,
};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Yüzey analizi";
const PREVIEW: &str = "Önizle";
const LEAVE: &str = "Sivrileri bırak";
const RESTORE: &str = "Bırakılanları geri al";
const APPLY: &str = "Onarımı uygula";
const CLOSE: &str = "Kapat";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 9] = [
    "Sınırlar",
    "Raster hücresi (m)",
    "Fark aralıkları (m)",
    "Yakın (m)",
    "Sivri erimi (m)",
    "Sivri sınırı (m)",
    "Boşluk kenarı (m)",
    "Sınıf alanı",
    "",
];
const MODES: [&str; 3] = ["Analiz", "Fark", "Kalite"];
const MEASURES: [&str; 3] = ["Kot (m)", "Eğim (%)", "Bakı (°)"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Analysis,
    Compare,
    Quality,
}

/// What was read last.
#[derive(Debug, Clone)]
pub(crate) enum Read {
    Analysis(TerrainsAnalysed),
    Compare(TerrainsCompared),
    Quality(TerrainsQualityRead),
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) mode: Mode,
    pub(crate) surface: Option<String>,
    pub(crate) comparison: Option<String>,
    pub(crate) measure: usize,
    pub(crate) roughness: bool,
    pub(crate) checks: Option<String>,
    pub(crate) fields: [String; 9],
    pub(crate) preview: Option<Result<Read, String>>,
    pub(crate) warnings: Vec<String>,
    /// A repair asked (Kalite: the spikes left out, or everything taken back) and its difference, before it is written.
    pub(crate) repair: Option<(TerrainsRepair, Result<TerrainsRepaired, String>)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Mode(Mode),
    Surface(String),
    Comparison(String),
    Measure(usize),
    Roughness(bool),
    Checks(Option<String>),
    Field(usize, String),
    Preview,
    LeaveSpikes,
    RestoreAll,
    Apply,
    Close,
}

fn msg(event: Event) -> Message {
    Message::SurfaceAnalysis(event)
}

fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

fn list(text: &str, what: &str) -> Result<Option<Vec<f64>>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.split([',', ';', ' ']).filter(|s| !s.is_empty()).map(|s| s.parse::<f64>().map_err(|_| format!("{what}: “{s}” bir sayı değil."))).collect::<Result<Vec<_>, _>>().map(Some)
}

impl Window {
    fn surface(&self) -> Result<String, String> {
        self.surface.clone().ok_or_else(|| "Bir arazi yüzeyi seçin.".to_owned())
    }

    pub(crate) fn analysis(&self) -> Result<TerrainsAnalysis, String> {
        let breaks = list(&self.fields[0], FIELDS[0])?;
        let mut q = TerrainsAnalysis {
            surface: self.surface()?,
            elevations: None,
            slopes: None,
            aspects: None,
            roughness: self.roughness,
            grid: number(&self.fields[1], FIELDS[1])?,
            pieces: true,
        };
        match self.measure {
            0 => q.elevations = breaks,
            1 => q.slopes = breaks,
            _ => q.aspects = breaks,
        }
        Ok(q)
    }

    pub(crate) fn compare(&self) -> Result<TerrainsCompare, String> {
        Ok(TerrainsCompare {
            base: self.surface()?,
            comparison: self.comparison.clone().ok_or("Karşılaştırılacak ikinci yüzeyi seçin.")?,
            ranges: list(&self.fields[2], FIELDS[2])?,
        })
    }

    pub(crate) fn quality(&self) -> Result<TerrainsQuality, String> {
        let (radius, limit) = (number(&self.fields[4], FIELDS[4])?, number(&self.fields[5], FIELDS[5])?);
        let spikes = match (radius, limit) {
            (Some(radius), Some(limit)) => Some(TerrainSpikesAsk { radius, limit }),
            (None, None) => None,
            _ => return Err("Sivri nokta için erim ile sınır birlikte yazılır.".to_owned()),
        };
        let field = self.fields[7].trim();
        Ok(TerrainsQuality {
            surface: self.surface()?,
            near: number(&self.fields[3], FIELDS[3])?,
            spikes,
            gaps: number(&self.fields[6], FIELDS[6])?,
            checkpoints: self.checks.clone().map(|layer| TerrainCheckpointsAsk {
                layer: Some(layer),
                uids: Vec::new(),
                class_field: (!field.is_empty()).then(|| field.to_owned()),
            }),
        })
    }
}

/// A range's colour: from green through amber to red by its place among `n`.
fn ramp(tk: &Tokens, k: usize, n: usize) -> Color {
    let t = if n > 1 { k as f32 / (n - 1) as f32 } else { 0.0 };
    let mix = |a: Color, b: Color, s: f32| Color::from_rgb(a.r + (b.r - a.r) * s, a.g + (b.g - a.g) * s, a.b + (b.b - a.b) * s);
    if t < 0.5 { mix(tk.success, tk.warning, t * 2.0) } else { mix(tk.warning, tk.danger, (t - 0.5) * 2.0) }
}

impl App {
    /// Yüzey analizi: the window on the road project's first terrain surface.
    pub(crate) fn open_surface_analysis(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Yüzey analizi yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let surfaces: Vec<String> = doc.model.settings().road.as_ref().map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| s.id.clone()).collect());
        let Some(first) = surfaces.first().cloned() else {
            self.warn("Yüzey analizi bir arazi yüzeyinde okunur: önce Arazi yüzeyleri'nde bir yüzey oluşturun.".to_owned());
            return;
        };
        self.surface_analysis_window = Some(Box::new(Window {
            mode: Mode::Analysis,
            surface: Some(first),
            comparison: surfaces.get(1).cloned(),
            measure: 1,
            roughness: false,
            checks: None,
            fields: Default::default(),
            preview: None,
            warnings: Vec::new(),
            repair: None,
        }));
        self.dialog = Some(Dialog::SurfaceAnalysis);
    }

    fn preview_surface_analysis(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.surface_analysis_window.as_mut() else { return };
        let read = match w.mode {
            Mode::Analysis => w.analysis().and_then(|q| road_terrains_analysis::preview(&doc.model, &q)).map(|(r, x)| (Read::Analysis(r), x)),
            Mode::Compare => w.compare().and_then(|q| road_terrains_compare::preview(&doc.model, &q)).map(|(r, x)| (Read::Compare(r), x)),
            Mode::Quality => w.quality().and_then(|q| road_terrains_quality::preview(&doc.model, &q)).map(|(r, x)| (Read::Quality(r), x)),
        };
        match read {
            Ok((r, warnings)) => {
                w.preview = Some(Ok(r));
                w.warnings = warnings.into_iter().map(|x| x.message).collect();
            }
            Err(why) => {
                w.preview = Some(Err(why));
                w.warnings.clear();
            }
        }
    }

    pub(crate) fn surface_analysis_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.surface_analysis_window.as_mut() else { return Task::none() };
        match event {
            Event::Mode(m) => w.mode = m,
            Event::Surface(s) => w.surface = Some(s),
            Event::Comparison(s) => w.comparison = Some(s),
            Event::Measure(m) => w.measure = m.min(2),
            Event::Roughness(on) => w.roughness = on,
            Event::Checks(l) => w.checks = l,
            Event::Field(k, t) => w.fields[k.min(8)] = t,
            Event::Preview => {
                w.repair = None;
                self.preview_surface_analysis();
                return Task::none();
            }
            Event::LeaveSpikes | Event::RestoreAll => {
                let Some(surface) = w.surface.clone() else { return Task::none() };
                let input = if matches!(event, Event::LeaveSpikes) {
                    let spikes = match &w.preview {
                        Some(Ok(Read::Quality(q))) => q.spikes.iter().map(|s| [s.at[0], s.at[1]]).collect(),
                        _ => Vec::new(),
                    };
                    TerrainsRepair { surface, exclude: spikes, restore: Vec::new(), expected_revision: None }
                } else {
                    let left = self.document.as_ref().and_then(|d| d.model.settings().road.as_ref()).and_then(|r| {
                        r.surfaces.iter().find(|s| Some(&s.id) == w.surface.as_ref()).map(|s| s.excluded.iter().map(|e| [e.x, e.y]).collect())
                    });
                    TerrainsRepair { surface, exclude: Vec::new(), restore: left.unwrap_or_default(), expected_revision: None }
                };
                let Some(doc) = &self.document else { return Task::none() };
                let read = road_terrains_repair::preview(&doc.model, &input);
                if let Some(w) = self.surface_analysis_window.as_mut() {
                    w.repair = Some((input, read));
                }
                return Task::none();
            }
            Event::Apply => {
                let Some((input, Ok(_))) = w.repair.clone() else { return Task::none() };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let result = road_terrains_repair::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if let Some(out) = self.answered(result) {
                    self.say(Level::Success, format!("Yüzey onarıldı; {} nokta kaydı dışarıda.", out.excluded.len()));
                    if let Some(w) = self.surface_analysis_window.as_mut() {
                        w.repair = None;
                    }
                    self.preview_surface_analysis();
                }
                return Task::none();
            }
            Event::Close => {
                self.surface_analysis_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        if let Some(w) = self.surface_analysis_window.as_mut() {
            w.preview = None;
            w.warnings.clear();
            w.repair = None;
        }
        Task::none()
    }

    pub(crate) fn surface_analysis_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.surface_analysis_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let model = self.document.as_ref().map(|d| &d.model);
        let surfaces: Vec<(String, String)> = model
            .and_then(|m| m.settings().road.as_ref())
            .map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect());
        let pick_surface = |at: &Option<String>, on: fn(String) -> Event| {
            let ids: Vec<String> = surfaces.iter().map(|s| s.0.clone()).collect();
            let chosen = at.as_ref().and_then(|s| ids.iter().position(|i| i == s));
            container(Select::new(surfaces.iter().map(|s| Choice::new(s.1.clone())), chosen, move |i| msg(on(ids[i].clone())))).width(170)
        };
        let field = |hint: &'static str, k: usize, width: f32| {
            words::field(
                FIELDS[k],
                kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width),
                None,
            )
        };
        let mode_at = match w.mode {
            Mode::Analysis => 0,
            Mode::Compare => 1,
            Mode::Quality => 2,
        };
        let mode = container(Select::new(MODES.iter().map(|m| Choice::new(*m)), Some(mode_at), |i| {
            msg(Event::Mode([Mode::Analysis, Mode::Compare, Mode::Quality][i]))
        }))
        .width(110);
        let mut top = row![words::field("Kip", mode, None), words::field("Yüzey", pick_surface(&w.surface, Event::Surface), None)]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom);
        match w.mode {
            Mode::Analysis => {
                let measure = container(Select::new(MEASURES.iter().map(|m| Choice::new(*m)), Some(w.measure), |i| msg(Event::Measure(i)))).width(110);
                let rough = container(Select::new(["Yok", "Var"].into_iter().map(Choice::new), Some(usize::from(w.roughness)), |i| {
                    msg(Event::Roughness(i == 1))
                }))
                .width(80);
                top = top
                    .push(words::field("Ölçü", measure, None))
                    .push(field("ör. 0, 2, 5, 10", 0, 200.0))
                    .push(words::field("Pürüzlülük", rough, None))
                    .push(field("yok", 1, 130.0));
            }
            Mode::Compare => {
                top = top.push(words::field("Karşılaştırma", pick_surface(&w.comparison, Event::Comparison), None)).push(field("ör. -1, 0, 1", 2, 220.0));
            }
            Mode::Quality => {
                let layers = self.relation_layers();
                let lids: Vec<Option<String>> = std::iter::once(None).chain(layers.iter().map(|(id, _)| Some(id.clone()))).collect();
                let names: Vec<String> = std::iter::once("Yok".to_owned()).chain(layers.iter().map(|(_, n)| n.clone())).collect();
                let at = lids.iter().position(|l| *l == w.checks);
                let checks = container(Select::new(names.into_iter().map(Choice::new), at, move |i| msg(Event::Checks(lids[i].clone())))).width(130);
                top = top
                    .push(field("yok", 3, 80.0))
                    .push(field("yok", 4, 100.0))
                    .push(field("yok", 5, 100.0))
                    .push(field("yok", 6, 110.0))
                    .push(words::field("Kontrol katmanı", checks, None))
                    .push(field("ör. Sınıf", 7, 100.0));
            }
        }
        let mut body = Column::new().spacing(12).push(top);
        match &w.preview {
            None => body = body.push(label::caption("Sınırlar ve değerler kullanıcının; KentOS değer vermez. Önizle yüzeyi okur, hiçbir şey yazmaz.").style(muted)),
            Some(Err(why)) => body = body.push(label::caption(why.clone()).style(danger)),
            Some(Ok(read)) => {
                let picture = container(iced::widget::canvas(Plan { read, measure: w.measure }).width(Fill).height(typography::scaled(360.0)))
                    .style(style::container::field_box);
                let mut side = Column::new().spacing(5).width(430);
                let line = |k: String, v: String| row![label::caption(k).style(muted).width(150), label::body(v)].spacing(8);
                match read {
                    Read::Analysis(a) => {
                        side = side.push(line("Alan".into(), format!("{:.2} m² · 3B {:.2} m²", a.area, a.surface_area)));
                        let unit = ["m", "%", "°"][w.measure];
                        if let Some(b) = a.elevations.as_ref().or(a.slopes.as_ref()).or(a.aspects.as_ref()) {
                            side = side.push(band_table(b, unit, a.area));
                        }
                        if let Some(r) = &a.roughness {
                            side = side.push(line("Pürüzlülük".into(), format!("ortalama {:.4} · en çok {:.4}", r.mean, r.max)));
                        }
                        if let Some(g) = &a.grid {
                            side = side.push(line(
                                "Raster ile TIN".into(),
                                format!("{} hücre · ortalama {:+.2} % · en çok {:.2} %", g.cells, g.mean, g.max_absolute),
                            ));
                        }
                    }
                    Read::Compare(c) => {
                        side = side
                            .push(line("Kazı".into(), format!("{:.2} m³ · {:.2} m²", c.cut, c.cut_area)))
                            .push(line("Dolgu".into(), format!("{:.2} m³ · {:.2} m²", c.fill, c.fill_area)))
                            .push(line("Net".into(), format!("{:+.2} m³", c.net)))
                            .push(line("Tabanın dışında".into(), format!("{:.2} m²", c.uncovered)));
                        if let (Some(l), Some(h)) = (&c.least, &c.largest) {
                            side = side.push(line("Fark".into(), format!("{:+.3} … {:+.3} m", l.difference, h.difference)));
                        }
                        for b in &c.bands {
                            side = side.push(line(format!("{:+.2} … {:+.2} m", b.from, b.to), format!("{:.2} m²", b.area)));
                        }
                    }
                    Read::Quality(q) => {
                        side = side.push(line("Köşe · üçgen".into(), format!("{} · {}", q.corners, q.triangles)));
                        if let Some(n) = q.near_count {
                            side = side.push(line("Yakın çift".into(), n.to_string()));
                        }
                        if let Some(n) = q.spike_count {
                            let top = q.spikes.first().map_or(String::new(), |s| format!(" · en çok {:+.2} m", s.difference));
                            side = side.push(line("Sivri nokta".into(), format!("{n}{top}")));
                        }
                        if let Some(g) = &q.gaps {
                            side = side.push(line("Boşluk".into(), format!("{} üçgen · {:.2} m² · en uzun kenar {:.2} m", g.triangles, g.area, g.longest)));
                        }
                        if let Some(c) = &q.checkpoints {
                            let head = |t: &'static str, wd: f32| label::caption(t).style(muted).width(wd);
                            let mut t = Column::new().spacing(2).push(
                                row![head("Sınıf", 80.0), head("Sayı", 50.0), head("Ort.", 60.0), head("Std.", 60.0), head("RMSE", 60.0), head("Dışı", 40.0)].spacing(6),
                            );
                            for k in c.classes.iter().chain(std::iter::once(&c.all)) {
                                let name = if k.class.is_empty() { if std::ptr::eq(k, &c.all) { "Hepsi" } else { "Sınıfsız" } } else { k.class.as_str() };
                                let cell = |v: String, wd: f32| label::caption(v).width(wd);
                                t = t.push(
                                    row![
                                        cell(name.to_owned(), 80.0),
                                        cell(k.count.to_string(), 50.0),
                                        cell(format!("{:+.3}", k.mean), 60.0),
                                        cell(format!("{:.3}", k.std), 60.0),
                                        cell(format!("{:.3}", k.rmse), 60.0),
                                        cell(k.outside.to_string(), 40.0)
                                    ]
                                    .spacing(6),
                                );
                            }
                            side = side.push(label::caption("Kontrol noktaları (nokta − yüzey, m)").style(muted)).push(t);
                        }
                    }
                }
                for wn in &w.warnings {
                    side = side.push(label::caption(wn.clone()).style(warn));
                }
                body = body.push(row![picture, scrollable(side).height(typography::scaled(360.0))].spacing(12));
            }
        }
        if let Some((_, read)) = &w.repair {
            match read {
                Ok(r) => {
                    let largest = r.largest.as_ref().map_or("değişiklik yok".to_owned(), |l| {
                        format!("en büyük {:+.3} m ({:.2}, {:.2})", l.difference, l.at[0], l.at[1])
                    });
                    body = body.push(
                        label::body(format!(
                            "Onarımın farkı (sonra − önce): kazı {:.3} m³ · dolgu {:.3} m³ · değişen alan {:.2} m² · {largest} · köşe {} → {} · bırakılan {}",
                            r.cut, r.fill, r.changed_area, r.corners_before, r.corners_after, r.excluded.len()
                        ))
                        .style(warn),
                    );
                }
                Err(why) => body = body.push(label::caption(why.clone()).style(danger)),
            }
        }
        let spikes = matches!(&w.preview, Some(Ok(Read::Quality(q))) if !q.spikes.is_empty());
        let left = model
            .and_then(|m| m.settings().road.as_ref())
            .and_then(|r| r.surfaces.iter().find(|s| Some(&s.id) == w.surface.as_ref()))
            .is_some_and(|s| !s.excluded.is_empty());
        let ready = matches!(&w.repair, Some((_, Ok(_))));
        let mut bottom = row![].spacing(8).align_y(iced::alignment::Vertical::Center);
        if w.mode == Mode::Quality {
            bottom = bottom
                .push(words::secondary(LEAVE, spikes.then(|| msg(Event::LeaveSpikes))))
                .push(words::secondary(RESTORE, left.then(|| msg(Event::RestoreAll))))
                .push(words::secondary(APPLY, ready.then(|| msg(Event::Apply))));
        }
        body = body.push(
            bottom
                .push(space::horizontal().width(Fill))
                .push(words::secondary(PREVIEW, Some(msg(Event::Preview))))
                .push(words::secondary(CLOSE, Some(msg(Event::Close)))),
        );
        Frame::new(TITLE).push(body).width(1180.0).modal(msg(Event::Close))
    }

    pub(crate) fn surface_analysis_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.surface_analysis_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(PREVIEW) => Some(msg(Event::Preview)),
            Control::Press(LEAVE) => Some(msg(Event::LeaveSpikes)),
            Control::Press(RESTORE) => Some(msg(Event::RestoreAll)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Fill(name, t) if !name.is_empty() && FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The ranges' table: bounds, plan and 3D area, share of the whole.
fn band_table<'a>(b: &TerrainBands, unit: &str, whole: f64) -> Element<'a, Message> {
    let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
    let head = |t: &'static str, wd: f32| label::caption(t).style(muted).width(wd);
    let mut col = Column::new().spacing(2).push(row![head("Aralık", 130.0), head("Alan m²", 80.0), head("3B m²", 80.0), head("Pay", 50.0)].spacing(6));
    let n = b.bands.len();
    for (k, band) in b.bands.iter().enumerate() {
        let swatch = iced::widget::canvas(Swatch { k, n }).width(10).height(10);
        let share = if whole > 0.0 { band.area / whole * 100.0 } else { 0.0 };
        col = col.push(
            row![
                row![swatch, label::caption(format!("{} … {} {unit}", band.from, band.to))].spacing(4).width(130),
                label::caption(format!("{:.2}", band.area)).width(80),
                label::caption(format!("{:.2}", band.surface_area)).width(80),
                label::caption(format!("%{share:.1}")).width(50)
            ]
            .spacing(6)
            .align_y(iced::alignment::Vertical::Center),
        );
    }
    if b.outside_area > 0.0 {
        col = col.push(row![label::caption("Aralık dışı").width(130), label::caption(format!("{:.2}", b.outside_area))].spacing(6));
    }
    if b.flat_area > 0.0 {
        col = col.push(row![label::caption("Düz (bakısız)").width(130), label::caption(format!("{:.2}", b.flat_area))].spacing(6));
    }
    col.into()
}

struct Swatch {
    k: usize,
    n: usize,
}

impl canvas::Program<Message> for Swatch {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), ramp(&Tokens::of(theme), self.k, self.n));
        vec![frame.into_geometry()]
    }
}

/// The plan: the ranges' parts coloured (Analiz), the difference's ranges as bars (Fark), the check points coloured by
/// their difference and the spikes (Kalite).
struct Plan<'a> {
    read: &'a Read,
    measure: usize,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let text = |frame: &mut canvas::Frame, s: String, p: Point| {
            frame.fill_text(Text { content: s, position: p, color: tk.muted, size: Pixels(11.0), ..Text::default() })
        };
        match self.read {
            Read::Analysis(a) => {
                let Some(b) = a.elevations.as_ref().or(a.slopes.as_ref()).or(a.aspects.as_ref()) else {
                    text(&mut frame, "Aralık sorulmadı.".into(), Point::new(10.0, 10.0));
                    return vec![frame.into_geometry()];
                };
                let pts: Vec<[f64; 2]> = b.bands.iter().flat_map(|x| x.parts.iter().flatten().copied()).collect();
                let Some(fit) = Fit::of(&pts, bounds) else { return vec![frame.into_geometry()] };
                let n = b.bands.len();
                for (k, band) in b.bands.iter().enumerate() {
                    let c = ramp(&tk, k, n);
                    for part in &band.parts {
                        let shape = Path::new(|p| {
                            p.move_to(fit.at(part[0]));
                            for q in &part[1..] {
                                p.line_to(fit.at(*q));
                            }
                            p.close();
                        });
                        frame.fill(&shape, c);
                    }
                }
                text(&mut frame, ["Kot aralıkları", "Eğim aralıkları", "Bakı aralıkları"][self.measure].into(), Point::new(10.0, 6.0));
            }
            Read::Compare(c) => {
                if c.bands.is_empty() {
                    text(&mut frame, "Fark aralığı yazılmadı: hacimler sağda.".into(), Point::new(10.0, 10.0));
                    return vec![frame.into_geometry()];
                }
                let most = c.bands.iter().map(|b| b.area).fold(0.0_f64, f64::max).max(1e-9);
                let h = (bounds.height - 40.0) / c.bands.len() as f32;
                for (k, b) in c.bands.iter().enumerate() {
                    let y = 20.0 + k as f32 * h;
                    let w = ((b.area / most) as f32) * (bounds.width - 220.0);
                    let color = if b.to <= 0.0 { tk.danger } else if b.from >= 0.0 { tk.success } else { tk.warning };
                    frame.fill_rectangle(Point::new(140.0, y + 3.0), iced::Size::new(w.max(1.0), (h - 6.0).max(2.0)), color);
                    text(&mut frame, format!("{:+.2} … {:+.2} m", b.from, b.to), Point::new(10.0, y + h / 2.0 - 7.0));
                    text(&mut frame, format!("{:.1} m²", b.area), Point::new(146.0 + w, y + h / 2.0 - 7.0));
                }
            }
            Read::Quality(q) => {
                let mut pts: Vec<[f64; 2]> = q.spikes.iter().map(|s| [s.at[0], s.at[1]]).collect();
                if let Some(c) = &q.checkpoints {
                    pts.extend(c.points.iter().map(|p| p.at));
                }
                pts.extend(q.near.iter().map(|p| [p.a[0], p.a[1]]));
                let Some(fit) = Fit::of(&pts, bounds) else {
                    text(&mut frame, "Gösterilecek nokta yok.".into(), Point::new(10.0, 10.0));
                    return vec![frame.into_geometry()];
                };
                if let Some(c) = &q.checkpoints {
                    let most = c.points.iter().filter_map(|p| p.difference).map(f64::abs).fold(0.0_f64, f64::max).max(1e-9);
                    for p in &c.points {
                        let at = fit.at(p.at);
                        match p.difference {
                            Some(d) => {
                                let color = if d >= 0.0 { tk.success } else { tk.danger };
                                let r = (3.0 + 7.0 * (d.abs() / most)) as f32;
                                frame.fill(&Path::circle(at, r), Color { a: 0.55, ..color });
                                text(&mut frame, format!("{d:+.3}"), at + iced::Vector::new(r + 2.0, -6.0));
                            }
                            None => frame.stroke(&Path::circle(at, 4.0), Stroke::default().with_width(1.2).with_color(tk.muted)),
                        }
                    }
                }
                for s in &q.spikes {
                    let at = fit.at([s.at[0], s.at[1]]);
                    let up = if s.difference >= 0.0 { -1.0 } else { 1.0 };
                    let tri = Path::new(|p| {
                        p.move_to(at + iced::Vector::new(0.0, 7.0 * up));
                        p.line_to(at + iced::Vector::new(-6.0, -5.0 * up));
                        p.line_to(at + iced::Vector::new(6.0, -5.0 * up));
                        p.close();
                    });
                    frame.stroke(&tri, Stroke::default().with_width(1.5).with_color(tk.warning));
                }
                for p in &q.near {
                    frame.stroke(&Path::circle(fit.at([p.a[0], p.a[1]]), 8.0), Stroke::default().with_width(1.5).with_color(tk.accent));
                }
            }
        }
        vec![frame.into_geometry()]
    }
}

/// Plan places fitted into the picture.
struct Fit {
    x0: f64,
    y1: f64,
    scale: f64,
    ox: f64,
    oy: f64,
}

impl Fit {
    fn of(pts: &[[f64; 2]], bounds: Rectangle) -> Option<Self> {
        if pts.is_empty() {
            return None;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in pts {
            x0 = x0.min(p[0]);
            y0 = y0.min(p[1]);
            x1 = x1.max(p[0]);
            y1 = y1.max(p[1]);
        }
        let pad = 26.0_f64;
        let (w, h) = (f64::from(bounds.width) - 2.0 * pad, f64::from(bounds.height) - 2.0 * pad);
        let span = (x1 - x0).max(y1 - y0).max(1e-6);
        let scale = (w / (x1 - x0).max(span * 0.05)).min(h / (y1 - y0).max(span * 0.05));
        Some(Self { x0, y1, scale, ox: pad + (w - (x1 - x0) * scale) / 2.0, oy: pad + (h - (y1 - y0) * scale) / 2.0 })
    }

    fn at(&self, p: [f64; 2]) -> Point {
        Point::new((self.ox + (p[0] - self.x0) * self.scale) as f32, (self.oy + (self.y1 - p[1]) * self.scale) as f32)
    }
}
