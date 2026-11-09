//! Yüzey analizi (plan/road.md Y18.5, docs/adr/0916 §5, `terrain.analysis`): a terrain surface read three ways, nothing
//! written. Analiz: elevation, slope or aspect ranges (Civil 3D's Elevations, Slopes, Directions) coloured in plan with
//! their plan and 3D areas, the roughness and a raster's slope against the triangles' (`road.terrains.analysis`). Fark:
//! a second surface over the first, cut, fill and net and the difference's ranges (`road.terrains.compare`); the
//! material zones (docs/adr/3540) are the closed objects of a layer, named and factored by their attributes. Kalite: near
//! pairs, spikes, coverage gaps and check points by class (`road.terrains.quality`); the spikes found can be left out
//! of the surface, the difference said first and written in one undo step, or everything left out taken back
//! (`road.terrains.repair`, Y18.7). Kapsam: how much of a project boundary the surface covers, the no-data rest and the
//! part estimated across too wide gaps (`road.terrains.coverage`, Y19.1), the triangles drawn measured or estimated.
//! Düşük güvenli alanları çiz writes the estimated areas' outlines on a layer (`road.terrains.lowconfidence`, Y19.2). The
//! breaks and limits are the user's; KentOS fills in none.
//!
//! ```text
//! ┌ Yüzey analizi ─────────────────────────────────────────────────────────────────── × ┐
//! │ Kip          [Fark ▾]        (Grafik | İçerik)                                       │
//! │ Yüzey ─────────────────      ┌ Grafik: the chart, zoomed and saved ──────────────┐ │
//! │  Taban       [Teras ▾]       │                                                    │ │
//! │  Karşılaştırma [Set ▾]       └────────────────────────────────────────────────────┘ │
//! │  ☐ Kotları aynı say          legend                                                 │
//! │ Hacim ─────────────────      İçerik: [Kazı] [Dolgu] [Net] [Malzemeye göre net]      │
//! │  Yöntem, aralıklar, katsayılar, Malzeme bölgeleri (3540)   the tables               │
//! │ Çizime yaz ────────────      ⚠ notes                                                │
//! │  Katman, Bant aralığı, Bölgelere                                                    │
//! ├─────────────────────────────────────────────────────────────────────────────────────┤
//! │                         [Sıfır hattını çiz] [Kapat] [■ Kazı ve dolgu bölgelerini çiz] │
//! └─────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The road windows' layout (DESIGN.md §7.19, `road_dialog::RoadDialog`): the form on the left in groups, the result
//! on the right, read again as the form changes (no Önizle), the chart and the content in their own tabs.

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, container, row, space};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CompareZone, TerrainBands, TerrainCheckpointsAsk, TerrainSpikesAsk, TerrainsAnalysed, TerrainsAnalysis, TerrainsCompare,
    TerrainsCompared, TerrainsCoverage, TerrainsCoverageDraw, TerrainsCovered, TerrainsDensity, TerrainsDensityRead,
    TerrainsQuality, TerrainsQualityRead, TerrainsRepair, TerrainsRepaired, TerrainsSparseDraw,
};
use kentos_interaction::Level;
use kentos_native_application::{
    ExecutionContext, road_terrains_analysis, road_terrains_compare, road_terrains_coverage, road_terrains_coverage_draw,
    road_terrains_density, road_terrains_quality, road_terrains_repair, road_terrains_sparse,
};
use kentos_ui::theme::Tokens;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::label;

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
const DRAW_LOW: &str = "Düşük güvenli alanları çiz";
const DRAW_SPARSE: &str = "Seyrek alanları çiz";
/// Fark's cut and fill boundary written on the layer chosen (docs/adr/1620).
const DRAW_ZERO: &str = "Sıfır hattını çiz";
/// Fark's cut and fill zones written as areas on the layer chosen, banded by the interval when given (docs/adr/1840).
const DRAW_ZONES: &str = "Kazı ve dolgu bölgelerini çiz";
/// A material zone's attributes (docs/adr/3540): its material's name (none: the layer's) and its factors (none: the
/// comparison's).
pub(crate) const MATERIAL: &str = "Malzeme";
pub(crate) const CUT_FACTOR: &str = "Kazı katsayısı";
pub(crate) const FILL_FACTOR: &str = "Dolgu katsayısı";
/// The typed fields, in order.
pub(crate) const FIELDS: [&str; 19] = [
    "Sınırlar",
    "Raster hücresi (m)",
    "Fark aralıkları (m)",
    "Yakın (m)",
    "Sivri erimi (m)",
    "Sivri sınırı (m)",
    "Boşluk kenarı (m)",
    "Sınıf alanı",
    "En uzun kenar (m)",
    "Hücre (m)",
    "En az nokta",
    "İstenen pay",
    "Kontrol doğruluğu (m)",
    // Fark's method and factors (docs/adr/1600).
    "Izgara hücresi (m)",
    "Kazı katsayısı",
    "Dolgu katsayısı",
    // Fark's cut and fill zones' bands (docs/adr/1840): empty, the cut and the fill only.
    "Bant aralığı (m)",
    // Fark by sections (docs/adr/4340): their spacing and the axis's azimuth (degrees from north, clockwise).
    "Kesit aralığı (m)",
    "Eksenin semti (°)",
];
const MODES: [&str; 5] = ["Analiz", "Fark", "Kalite", "Kapsam", "Yoğunluk"];
/// Fark by sections' spacing when none is typed, m (docs/adr/4340).
const SECTION_INTERVAL: f64 = 10.0;
/// The form's label column and a control's width in it (DESIGN.md §7.19: a 300 px form, 130 px labels).
const LABEL: f32 = 130.0;
const BOX: f32 = 160.0;
const MEASURES: [&str; 3] = ["Kot (m)", "Eğim (%)", "Bakı (°)"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Analysis,
    Compare,
    Quality,
    Coverage,
    Density,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let k = match self {
            Mode::Analysis => 0,
            Mode::Compare => 1,
            Mode::Quality => 2,
            Mode::Coverage => 3,
            Mode::Density => 4,
        };
        f.write_str(MODES[k])
    }
}

/// What was read last.
#[derive(Debug, Clone)]
pub(crate) enum Read {
    Analysis(TerrainsAnalysed),
    Compare(TerrainsCompared),
    Quality(TerrainsQualityRead),
    Coverage(TerrainsCovered, road_terrains_coverage::CoveragePicture),
    Density(TerrainsDensityRead, road_terrains_density::DensityPicture),
}

/// The two surfaces' heights taken as one vertical system (docs/adr/1029): the comparison is then a draft.
pub(crate) const SAME_HEIGHTS: &str = "Kotları aynı say";
const HEIGHTS_CHOICE: &str = "“Kotları aynı say › Var”";

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) mode: Mode,
    pub(crate) surface: Option<String>,
    pub(crate) comparison: Option<String>,
    /// The two surfaces' heights taken as one vertical system when not shown to be (docs/adr/1029).
    pub(crate) assume_heights: bool,
    pub(crate) measure: usize,
    pub(crate) roughness: bool,
    /// Fark by a grid's cells (docs/adr/1600), else the exact TIN–TIN prisms.
    pub(crate) volume_grid: bool,
    /// Fark by parallel sections (docs/adr/4340), the prismoidal formula instead of the average end area.
    pub(crate) volume_sections: bool,
    pub(crate) prismoidal: bool,
    /// The cut and fill zones' labels and quantity table (docs/adr/2340), their deepest places (docs/adr/2740).
    pub(crate) zone_labels: bool,
    pub(crate) zone_table: bool,
    pub(crate) zone_depths: bool,
    /// Fark's material zones (docs/adr/3540): the layer whose closed objects bound them; none, no zones.
    pub(crate) materials: Option<String>,
    pub(crate) checks: Option<String>,
    /// Kapsam's boundary: a closed object's uid; none, the surface itself.
    pub(crate) boundary: Option<String>,
    /// Kapsam's layer for the low confidence areas.
    pub(crate) layer: Option<String>,
    pub(crate) fields: [String; 19],
    /// The result's tab: the chart or the content; the content's table shown (one at a time, DESIGN.md §7.19).
    pub(crate) tab: Tab,
    pub(crate) list: &'static str,
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
    AssumeHeights(bool),
    Measure(usize),
    Roughness(bool),
    /// Fark's method by its place in TIN–TIN | Izgara | Kesit (docs/adr/1600, 4340).
    VolumeMethod(usize),
    /// Fark by sections: the prismoidal formula (true) or the average end area.
    Prismoidal(bool),
    Checks(Option<String>),
    Boundary(Option<String>),
    Layer(String),
    DrawLow,
    /// Fark: the cut and fill boundary written (`road.terrains.zeroline`).
    DrawZero,
    /// Fark: the cut and fill zones written as areas (`road.terrains.cutfill`).
    DrawZones,
    /// Fark: the zones' labels and their quantity table with them (docs/adr/2340), their deepest places (docs/adr/2740).
    ZoneLabels(bool),
    ZoneTable(bool),
    ZoneDepths(bool),
    /// Fark: the layer of the material zones (docs/adr/3540).
    Materials(Option<String>),
    DrawSparse,
    Field(usize, String),
    /// The result's tab (Grafik | İçerik), and the content's table.
    Tab(Tab),
    List(&'static str),
    Preview,
    LeaveSpikes,
    RestoreAll,
    Apply,
    Close,
}

fn msg(event: Event) -> Message {
    Message::SurfaceAnalysis(event)
}

/// A single number: a decimal comma read as a point, as the other road windows read it (lists keep the comma as their
/// separator).
fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".").parse::<f64>().map(Some).map_err(|_| format!("{what}: “{t}” bir sayı değil."))
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

    /// The material zones (docs/adr/3540): each closed straight-edged object on the layer chosen, in the drawing's
    /// order, named by its `Malzeme` attribute (none: the layer's name), its factors its `Kazı katsayısı` and `Dolgu
    /// katsayısı` (none: the comparison's).
    fn materials_of(&self, doc: &kentos_domain::Document, layers: &[(String, String)]) -> Result<Vec<CompareZone>, String> {
        let Some(layer) = &self.materials else {
            return Ok(Vec::new());
        };
        let layer_name = layers.iter().find(|(id, _)| id == layer).map_or_else(|| layer.clone(), |(_, n)| n.clone());
        let mut out = Vec::new();
        for (uid, e) in doc.entities_with_uids() {
            let uid = uid.to_string();
            if e.base().layer_id != *layer || road_terrains_coverage::boundary_region(doc, &uid).is_none() {
                continue;
            }
            let attrs = &e.base().attrs;
            let factor = |key: &str| attrs.get(key).map_or(Ok(None), |t| number(t, key));
            out.push(CompareZone {
                name: attrs.get(MATERIAL).filter(|n| !n.trim().is_empty()).cloned().unwrap_or_else(|| layer_name.clone()),
                boundary: uid,
                cut_factor: factor(CUT_FACTOR)?,
                fill_factor: factor(FILL_FACTOR)?,
            });
        }
        Ok(out)
    }

    pub(crate) fn compare(&self, doc: &kentos_domain::Document, layers: &[(String, String)]) -> Result<TerrainsCompare, String> {
        Ok(TerrainsCompare {
            base: self.surface()?,
            comparison: self.comparison.clone().ok_or("Karşılaştırılacak ikinci yüzeyi seçin.")?,
            ranges: if self.volume_grid || self.volume_sections { None } else { list(&self.fields[2], FIELDS[2])? },
            method: if self.volume_sections {
                Some(kentos_commands::CompareMethod::Sections)
            } else {
                self.volume_grid.then_some(kentos_commands::CompareMethod::Grid)
            },
            cell: if self.volume_grid { number(&self.fields[13], FIELDS[13])? } else { None },
            cut_factor: number(&self.fields[14], FIELDS[14])?,
            fill_factor: number(&self.fields[15], FIELDS[15])?,
            assume_heights: self.assume_heights,
            zones: if self.volume_grid || self.volume_sections { Vec::new() } else { self.materials_of(doc, layers)? },
            // Docs/adr/4340: the sections' spacing (none typed: one every 10 m), the axis's azimuth and the formula.
            interval: if self.volume_sections { Some(number(&self.fields[17], FIELDS[17])?.unwrap_or(SECTION_INTERVAL)) } else { None },
            azimuth: if self.volume_sections { number(&self.fields[18], FIELDS[18])? } else { None },
            formula: (self.volume_sections && self.prismoidal).then_some(kentos_commands::SectionFormula::Prismoidal),
        })
    }

    pub(crate) fn coverage(&self) -> Result<TerrainsCoverage, String> {
        Ok(TerrainsCoverage { surface: self.surface()?, boundary: self.boundary.clone(), longest: number(&self.fields[8], FIELDS[8])? })
    }

    /// A count typed: a whole number, none when empty.
    fn least(&self) -> Result<Option<u32>, String> {
        let t = self.fields[10].trim();
        if t.is_empty() {
            return Ok(None);
        }
        t.parse::<u32>().map(Some).map_err(|_| format!("{} tam sayı olmalı: “{t}”.", FIELDS[10]))
    }

    pub(crate) fn density(&self) -> Result<TerrainsDensity, String> {
        let Some(cell) = number(&self.fields[9], FIELDS[9])? else {
            return Err("Yoğunluk ızgarası için hücre boyunu yazın: USGS'nin ızgarası nominal nokta aralığının iki katıdır.".into());
        };
        Ok(TerrainsDensity {
            surface: self.surface()?,
            cell,
            boundary: self.boundary.clone(),
            least: self.least()?,
            share: number(&self.fields[11], FIELDS[11])?,
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
        // The check points' own RMSE (docs/adr/0945): read only when there are check points.
        let sigma = self.checks.as_ref().map(|_| number(&self.fields[12], FIELDS[12])).transpose()?;
        Ok(TerrainsQuality {
            surface: self.surface()?,
            near: number(&self.fields[3], FIELDS[3])?,
            spikes,
            gaps: number(&self.fields[6], FIELDS[6])?,
            checkpoints: self.checks.clone().map(|layer| TerrainCheckpointsAsk {
                layer: Some(layer),
                uids: Vec::new(),
                class_field: (!field.is_empty()).then(|| field.to_owned()),
                sigma: sigma.flatten(),
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
        if !kentos_native_application::road_terrains::holds_surfaces(doc.model.settings().cad_work) {
            self.warn("Yüzey analizi Harita ve Yol projelerinde olur.".to_owned());
            return;
        }
        let surfaces: Vec<String> = doc.model.settings().road.as_ref().map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| s.id.clone()).collect());
        // The selection's own surface when every selected object is its source (docs/adr/2480), else the first.
        let picked = self.selection_roles().surface.filter(|id| surfaces.contains(id));
        let Some(first) = picked.or_else(|| surfaces.first().cloned()) else {
            self.warn("Yüzey analizi bir arazi yüzeyinde okunur: önce Arazi yüzeyleri'nde bir yüzey oluşturun.".to_owned());
            return;
        };
        let comparison = surfaces.iter().find(|s| **s != first).cloned();
        self.surface_analysis_window = Some(Box::new(Window {
            mode: Mode::Analysis,
            surface: Some(first),
            comparison,
            assume_heights: false,
            measure: 1,
            roughness: false,
            volume_grid: false,
            volume_sections: false,
            prismoidal: false,
            zone_labels: false,
            zone_table: false,
            zone_depths: false,
            // A layer named for its material zones (docs/adr/3540), else none.
            materials: self
                .relation_layers()
                .into_iter()
                .find(|(_, n)| n.to_lowercase().contains("malzeme"))
                .map(|(id, _)| id),
            checks: None,
            boundary: None,
            layer: self.relation_layers().into_iter().next().map(|(id, _)| id),
            fields: Default::default(),
            tab: Tab::Chart,
            list: SUMMARY,
            preview: None,
            warnings: Vec::new(),
            repair: None,
        }));
        // The result follows the form (DESIGN.md §7.19): read at once.
        self.preview_surface_analysis();
        self.dialog = Some(Dialog::SurfaceAnalysis);
    }

    fn preview_surface_analysis(&mut self) {
        let layers = self.relation_layers();
        let Some(doc) = &self.document else { return };
        let Some(w) = self.surface_analysis_window.as_mut() else { return };
        let read = match w.mode {
            Mode::Analysis => w.analysis().and_then(|q| road_terrains_analysis::preview(&doc.model, &q)).map(|(r, x)| (Read::Analysis(r), x)),
            Mode::Compare => w.compare(&doc.model, &layers).and_then(|q| road_terrains_compare::preview(&doc.model, &q)).map(|(r, x)| (Read::Compare(r), x)),
            Mode::Quality => w.quality().and_then(|q| road_terrains_quality::preview(&doc.model, &q)).map(|(r, x)| (Read::Quality(r), x)),
            Mode::Coverage => w.coverage().and_then(|q| {
                let (r, x) = road_terrains_coverage::preview(&doc.model, &q)?;
                Ok((Read::Coverage(r, road_terrains_coverage::picture(&doc.model, &q)?), x))
            }),
            Mode::Density => w.density().and_then(|q| {
                let (r, x) = road_terrains_density::preview(&doc.model, &q)?;
                Ok((Read::Density(r, road_terrains_density::picture(&doc.model, &q)?), x))
            }),
        };
        match read {
            Ok((r, warnings)) => {
                w.preview = Some(Ok(r));
                w.warnings = warnings.into_iter().map(|x| crate::road_dialog::heights_words(&x.message, HEIGHTS_CHOICE)).collect();
            }
            Err(why) => {
                w.preview = Some(Err(crate::road_dialog::heights_words(&why, HEIGHTS_CHOICE)));
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
            Event::AssumeHeights(b) => w.assume_heights = b,
            Event::Measure(m) => w.measure = m.min(2),
            Event::Roughness(on) => w.roughness = on,
            Event::VolumeMethod(i) => {
                w.volume_grid = i == 1;
                w.volume_sections = i == 2;
            }
            Event::Prismoidal(on) => w.prismoidal = on,
            Event::Materials(layer) => w.materials = layer,
            Event::ZoneLabels(on) => {
                w.zone_labels = on;
                return Task::none();
            }
            Event::ZoneTable(on) => {
                w.zone_table = on;
                return Task::none();
            }
            Event::ZoneDepths(on) => {
                w.zone_depths = on;
                return Task::none();
            }
            Event::Checks(l) => w.checks = l,
            Event::Boundary(b) => w.boundary = b,
            Event::Layer(l) => {
                w.layer = Some(l);
                return Task::none();
            }
            Event::DrawLow => {
                let (Some(surface), Some(layer)) = (w.surface.clone(), w.layer.clone()) else { return Task::none() };
                let Ok(Some(longest)) = number(&w.fields[8], FIELDS[8]) else {
                    self.warn("Düşük güvenli alan için en uzun kenarı yazın.".to_owned());
                    return Task::none();
                };
                let input = TerrainsCoverageDraw { surface, longest, layer_id: layer, outline: true, labels: true, expected_revision: None };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let result = road_terrains_coverage_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if let Some(out) = self.answered(result) {
                    self.say(Level::Success, format!("{} düşük güvenli alan çizildi ({:.1} m²).", out.areas, out.area));
                }
                return Task::none();
            }
            Event::DrawZero => {
                let (Some(base), Some(comparison), Some(layer)) = (w.surface.clone(), w.comparison.clone(), w.layer.clone())
                else {
                    return Task::none();
                };
                let input = kentos_commands::TerrainsZeroLine { base, comparison, layer_id: layer, expected_revision: None };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let result = kentos_native_application::road_terrains_zeroline::execute(
                    &mut ExecutionContext::new(&mut doc.model),
                    input,
                );
                if let Some(out) = self.answered(result) {
                    let closed = out.lines.iter().filter(|l| l.closed).count();
                    self.say(
                        Level::Success,
                        format!(
                            "Sıfır hattı çizildi: {} çizgi ({closed} kapalı), {:.2} m.",
                            out.lines.len(),
                            out.length
                        ),
                    );
                }
                return Task::none();
            }
            Event::DrawZones => {
                let (Some(base), Some(comparison), Some(layer)) = (w.surface.clone(), w.comparison.clone(), w.layer.clone())
                else {
                    return Task::none();
                };
                let interval = match number(&w.fields[16], FIELDS[16]) {
                    Ok(i) => i,
                    Err(why) => {
                        self.warn(why);
                        return Task::none();
                    }
                };
                let input = kentos_commands::TerrainsCutFill {
                    base,
                    comparison,
                    layer_id: layer,
                    interval,
                    labels: w.zone_labels.then_some(true),
                    table: w.zone_table.then_some(true),
                    depths: w.zone_depths.then_some(true),
                    expected_revision: None,
                };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let result = kentos_native_application::road_terrains_cutfill::execute(
                    &mut ExecutionContext::new(&mut doc.model),
                    input,
                );
                if let Some(out) = self.answered(result) {
                    let cut = out.zones.iter().filter(|z| z.kind == kentos_commands::CutFillKind::Cut).count();
                    self.say(
                        Level::Success,
                        format!(
                            "Kazı ve dolgu bölgeleri çizildi: {cut} kazı ({:.3} m³), {} dolgu ({:.3} m³).",
                            out.cut,
                            out.zones.len() - cut,
                            out.fill
                        ),
                    );
                }
                return Task::none();
            }
            Event::DrawSparse => {
                let (Some(surface), Some(layer)) = (w.surface.clone(), w.layer.clone()) else { return Task::none() };
                let q = match w.density() {
                    Ok(q) => q,
                    Err(why) => {
                        self.warn(why);
                        return Task::none();
                    }
                };
                let Some(least) = q.least else {
                    self.warn("Seyrek alan için en az nokta sayısını yazın.".to_owned());
                    return Task::none();
                };
                let input = TerrainsSparseDraw { surface, cell: q.cell, least, boundary: q.boundary, layer_id: layer, labels: true, expected_revision: None };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                let result = road_terrains_sparse::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if let Some(out) = self.answered(result) {
                    self.say(Level::Success, format!("{} seyrek alan çizildi ({:.1} m²).", out.areas, out.area));
                }
                return Task::none();
            }
            Event::Field(k, t) => w.fields[k.min(FIELDS.len() - 1)] = t,
            Event::Tab(t) => {
                w.tab = t;
                return Task::none();
            }
            Event::List(l) => {
                w.list = l;
                return Task::none();
            }
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
        // The result follows the form (DESIGN.md §7.19): read again, no Önizle.
        if let Some(w) = self.surface_analysis_window.as_mut() {
            w.repair = None;
        }
        self.preview_surface_analysis();
        Task::none()
    }

    pub(crate) fn surface_analysis_view(&self) -> Element<'_, Message> {
        use crate::road_dialog::RoadDialog;
        use kentos_ui::widget::Banner;
        let Some(w) = &self.surface_analysis_window else {
            return iced::widget::text("").into();
        };
        let model = self.document.as_ref().map(|d| &d.model);
        let surfaces: Vec<(String, String)> = model
            .and_then(|m| m.settings().road.as_ref())
            .map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect());
        // The road windows' form (DESIGN.md §7.19): a 300 px column, a 130 px label column, one control a row, grouped;
        // help under a field only where the label does not say it. The writes are the footer's buttons.
        let pick_surface = |at: &Option<String>, on: fn(String) -> Event| {
            let ids: Vec<String> = surfaces.iter().map(|s| s.0.clone()).collect();
            let chosen = at.as_ref().and_then(|s| ids.iter().position(|i| i == s));
            container(Select::new(surfaces.iter().map(|s| Choice::new(s.1.clone())), chosen, move |i| msg(on(ids[i].clone())))).width(BOX)
        };
        let input = |hint: &'static str, k: usize| {
            kentos_ui::widget::text_field(hint, &w.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(BOX)
        };
        let layers = self.relation_layers();
        // A layer chosen from the drawing's, “Yok” first where none is a choice.
        let pick_layer = |at: Option<&String>, none: bool, on: Box<dyn Fn(Option<String>) -> Message>| {
            let mut lids: Vec<Option<String>> = layers.iter().map(|(id, _)| Some(id.clone())).collect();
            let mut names: Vec<String> = layers.iter().map(|(_, n)| n.clone()).collect();
            if none {
                lids.insert(0, None);
                names.insert(0, "Yok".to_owned());
            }
            let chosen = lids.iter().position(|l| l.as_ref() == at);
            container(Select::new(names.into_iter().map(Choice::new), chosen, move |i| on(lids[i].clone())).placeholder("Katman")).width(BOX)
        };
        let modes = [Mode::Analysis, Mode::Compare, Mode::Quality, Mode::Coverage, Mode::Density];
        let at = modes.iter().position(|m| *m == w.mode);
        let kip = container(Select::new(MODES.iter().map(|m| Choice::new(*m)), at, move |i| msg(Event::Mode(modes[i])))).width(BOX);
        let mut form = kentos_ui::widget::Form::new().label_width(LABEL).field("Kip", kip).section("Yüzey");
        form = if w.mode == Mode::Compare {
            form.field("Taban", pick_surface(&w.surface, Event::Surface))
                .field("Karşılaştırma", pick_surface(&w.comparison, Event::Comparison))
                .row(words::check(w.assume_heights, SAME_HEIGHTS, Some(msg(Event::AssumeHeights(!w.assume_heights)))))
        } else {
            form.field("Yüzey", pick_surface(&w.surface, Event::Surface))
        };
        match w.mode {
            Mode::Analysis => {
                let measure = container(Select::new(MEASURES.iter().map(|m| Choice::new(*m)), Some(w.measure), |i| msg(Event::Measure(i)))).width(BOX);
                form = form
                    .section("Aralıklar")
                    .field("Ölçü", measure)
                    .field(FIELDS[0], input("ör. 0, 2, 5, 10", 0))
                    .row(words::check(w.roughness, "Pürüzlülük", Some(msg(Event::Roughness(!w.roughness)))))
                    .field(FIELDS[1], input("yok", 1))
                    .help("Raster eğimi üçgenlerinkiyle karşılaştırılır.");
            }
            Mode::Compare => {
                // The method and the factors (docs/adr/1600): a grid's cell instead of the ranges; the factors and the
                // material zones (docs/adr/3540, the TIN's alone) are the user's, none is guessed.
                let chosen = if w.volume_sections { 2 } else { usize::from(w.volume_grid) };
                let method = container(Select::new(["TIN–TIN", "Izgara", "Kesit"].into_iter().map(Choice::new), Some(chosen), |i| {
                    msg(Event::VolumeMethod(i))
                }))
                .width(BOX);
                form = form.section("Hacim").field("Yöntem", method);
                form = if w.volume_sections {
                    // Docs/adr/4340: parallel sections across an axis; Civil 3D's Average End Area and Prismoidal.
                    let formula = container(Select::new(["Ortalama alan", "Prizmoidal"].into_iter().map(Choice::new), Some(usize::from(w.prismoidal)), |i| {
                        msg(Event::Prismoidal(i == 1))
                    }))
                    .width(BOX);
                    form.field(FIELDS[17], input("10", 17))
                        .field(FIELDS[18], input("0", 18))
                        .field("Formül", formula)
                        .help("Kesitler eksene dik; semt kuzeyden saat yönünde (0: kesitler doğu–batı).")
                } else if w.volume_grid {
                    form.field("Hücre (m)", input("ör. 1", 13))
                } else {
                    form.field(FIELDS[2], input("ör. -1, 0, 1", 2))
                };
                form = form.field(FIELDS[14], input("1", 14)).field(FIELDS[15], input("1", 15));
                if !w.volume_grid && !w.volume_sections {
                    form = form
                        .field("Malzeme bölgeleri", pick_layer(w.materials.as_ref(), true, Box::new(|l| msg(Event::Materials(l)))))
                        .help("Katmanın kapalı nesneleri; ad Malzeme, katsayılar Kazı katsayısı ve Dolgu katsayısı özniteliklerinden.");
                }
            }
            Mode::Quality => {
                form = form
                    .section("Denetim")
                    .field(FIELDS[3], input("yok", 3))
                    .field(FIELDS[4], input("yok", 4))
                    .field(FIELDS[5], input("yok", 5))
                    .field(FIELDS[6], input("yok", 6))
                    .section("Kontrol noktaları")
                    .field("Katman", pick_layer(w.checks.as_ref(), true, Box::new(|l| msg(Event::Checks(l)))))
                    .field(FIELDS[7], input("ör. Sınıf", 7))
                    .field("Doğruluk (m)", input("yok", 12));
            }
            Mode::Coverage | Mode::Density => {
                // The drawing's closed straight-edged objects, by their layer and number.
                let mut ids: Vec<Option<String>> = vec![None];
                let mut names = vec!["Yüzeyin kendisi".to_owned()];
                if let Some(m) = model {
                    for (uid, e) in m.entities_with_uids() {
                        if ids.len() > 200 {
                            break;
                        }
                        if road_terrains_coverage::boundary_region(m, &uid.to_string()).is_some() {
                            let layer = layers.iter().find(|l| l.0 == e.base().layer_id).map_or("", |l| l.1.as_str());
                            names.push(format!("{} · {} #{}", crate::view::kind_name(e.kind()), layer, e.base().id));
                            ids.push(Some(uid.to_string()));
                        }
                    }
                }
                let at = ids.iter().position(|b| *b == w.boundary);
                let boundary = container(Select::new(names.into_iter().map(Choice::new), at, move |i| msg(Event::Boundary(ids[i].clone())))).width(BOX);
                form = form.section("Sınır").field("Proje sınırı", boundary);
                form = if w.mode == Mode::Coverage {
                    form.field(FIELDS[8], input("yok", 8))
                } else {
                    form.section("Yoğunluk").field(FIELDS[9], input("ör. 2", 9)).field(FIELDS[10], input("yok", 10)).field(FIELDS[11], input("yok", 11))
                };
            }
        }
        // What the drawing gets: a layer and the writes' own fields; the writes themselves are the footer's buttons.
        if matches!(w.mode, Mode::Compare | Mode::Coverage | Mode::Density) {
            form = form.section("Çizime yaz").field("Katman", pick_layer(w.layer.as_ref(), false, Box::new(|l| msg(Event::Layer(l.unwrap_or_default())))));
            if w.mode == Mode::Compare {
                form = form.field(FIELDS[16], input("boş: kazı, dolgu", 16)).field(
                    "Bölgelere",
                    Column::new()
                        .spacing(6)
                        .push(words::check(w.zone_labels, "Yazı (tür, hacim)", Some(msg(Event::ZoneLabels(!w.zone_labels)))))
                        .push(words::check(w.zone_table, "Miktar tablosu", Some(msg(Event::ZoneTable(!w.zone_table)))))
                        .push(words::check(w.zone_depths, "En derin yer", Some(msg(Event::ZoneDepths(!w.zone_depths))))),
                );
            }
        }
        // The result: the chart and the content in their own tabs (the owner's words: “grafik ve içerik ayrı sekmelerde
        // olsun”), the notes under either.
        let tabs = row![kentos_ui::widget::Segmented::new([Tab::Chart, Tab::Content], w.tab, |t| msg(Event::Tab(t))), space::horizontal()];
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close)).width(1180.0).over(tabs);
        match &w.preview {
            None => d = d.empty("Sınırlar ve değerler kullanıcının; KentOS değer vermez. Sonuç form değiştikçe okunur, çizime yazılmaz."),
            Some(Err(why)) => d = d.note(Banner::error(why.clone())),
            Some(Ok(read)) => {
                match w.tab {
                    Tab::Chart => {
                        let plan = iced::widget::canvas(Plan { read, measure: w.measure, least: w.least().ok().flatten() }).width(Fill).height(Fill);
                        d = d.picture(plan, 380.0);
                        if let Some(legend) = legend(read) {
                            d = d.legend(legend);
                        }
                    }
                    Tab::Content => {
                        let (figures, mut lists) = content(w, read);
                        for (label, value) in figures {
                            d = d.figure(label, value);
                        }
                        // One table at a time, the choice over it (DESIGN.md §7.19); the first when the one chosen is not
                        // this reading's.
                        let names: Vec<&'static str> = lists.iter().map(|l| l.0).collect();
                        let at = names.iter().position(|n| *n == w.list).unwrap_or(0);
                        if !lists.is_empty() {
                            let (name, table) = lists.swap_remove(at);
                            d = d.detail(if names.len() > 1 {
                                crate::road_dialog::detail_choice(names, name, |l| msg(Event::List(l)), table)
                            } else {
                                table
                            });
                        }
                    }
                }
                for wn in &w.warnings {
                    d = d.note(Banner::warning(wn.clone()));
                }
            }
        }
        if let Some((_, read)) = &w.repair {
            d = match read {
                Ok(r) => {
                    let largest = r.largest.as_ref().map_or("değişiklik yok".to_owned(), |l| {
                        format!("en büyük {:+.3} m ({:.2}, {:.2})", l.difference, l.at[0], l.at[1])
                    });
                    d.note(Banner::info(format!(
                        "Onarımın farkı (sonra − önce): kazı {:.3} m³ · dolgu {:.3} m³ · değişen alan {:.2} m² · {largest} · köşe {} → {} · bırakılan {}",
                        r.cut, r.fill, r.changed_area, r.corners_before, r.corners_after, r.excluded.len()
                    )))
                }
                Err(why) => d.note(Banner::error(why.clone())),
            };
        }
        // The footer: the mode's writes, the last of them the primary.
        match w.mode {
            Mode::Compare => {
                let can = w.layer.is_some() && w.comparison.is_some();
                d = d.secondary(DRAW_ZERO, can.then(|| msg(Event::DrawZero))).primary(DRAW_ZONES, can.then(|| msg(Event::DrawZones)));
            }
            Mode::Coverage => {
                let can = w.layer.is_some() && !w.fields[8].trim().is_empty();
                d = d.primary(DRAW_LOW, can.then(|| msg(Event::DrawLow)));
            }
            Mode::Density => {
                let can = w.layer.is_some() && !w.fields[9].trim().is_empty() && !w.fields[10].trim().is_empty();
                d = d.primary(DRAW_SPARSE, can.then(|| msg(Event::DrawSparse)));
            }
            Mode::Quality => {
                let spikes = matches!(&w.preview, Some(Ok(Read::Quality(q))) if !q.spikes.is_empty());
                let left = model
                    .and_then(|m| m.settings().road.as_ref())
                    .and_then(|r| r.surfaces.iter().find(|s| Some(&s.id) == w.surface.as_ref()))
                    .is_some_and(|s| !s.excluded.is_empty());
                let ready = matches!(&w.repair, Some((_, Ok(_))));
                d = d
                    .secondary(LEAVE, spikes.then(|| msg(Event::LeaveSpikes)))
                    .secondary(RESTORE, left.then(|| msg(Event::RestoreAll)))
                    .primary(APPLY, ready.then(|| msg(Event::Apply)));
            }
            Mode::Analysis => {}
        }
        d.view()
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
            Control::Press(DRAW_LOW) => Some(msg(Event::DrawLow)),
            Control::Press(DRAW_SPARSE) => Some(msg(Event::DrawSparse)),
            Control::Press(DRAW_ZERO) => Some(msg(Event::DrawZero)),
            Control::Press(DRAW_ZONES) => Some(msg(Event::DrawZones)),
            Control::Pick(SAME_HEIGHTS, "Var") => Some(msg(Event::AssumeHeights(true))),
            Control::Pick(SAME_HEIGHTS, "Yok") => Some(msg(Event::AssumeHeights(false))),
            Control::Fill(name, t) if !name.is_empty() && FIELDS.contains(&name) => {
                let k = FIELDS.iter().position(|f| *f == name).unwrap_or(0);
                Some(msg(Event::Field(k, t.to_owned())))
            }
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The ranges' table: bounds, plan and 3D area, share of the whole.
/// The result's two tabs (the owner's words: “grafik ve içerik ayrı sekmelerde olsun”): the chart, or the figures and
/// the tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Chart,
    Content,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Chart => "Grafik",
            Tab::Content => "İçerik",
        })
    }
}

/// The line under the chart that says what it shows (DESIGN.md §7.19: no words inside a picture's lines).
fn legend(read: &Read) -> Option<String> {
    Some(match read {
        Read::Analysis(_) => "Renkler aralıklardır, ilki en düşük; alanlar İçerik'te.".to_owned(),
        Read::Compare(c) if c.bands.is_empty() && !c.zones.is_empty() => {
            "Her malzemenin katsayılı kazısı (kırmızı) ve dolgusu (yeşil); bölge dışı karşılaştırmanın katsayılarıyla.".to_owned()
        }
        Read::Compare(c) if !c.bands.is_empty() => "Fark aralıklarının plan alanı: kırmızı kazı, yeşil dolgu, sarı ikisini de kapsayan.".to_owned(),
        Read::Compare(_) => return None,
        Read::Quality(_) => {
            "Sarı üçgen sivri nokta, halka yakın çift; kontrol noktası farkıyla: yeşil yüzeyin üstünde, kırmızı altında.".to_owned()
        }
        Read::Coverage(..) => "Yeşil ölçülen, sarı uzun kenarla tahmin edilen üçgenler; çizgi proje sınırı.".to_owned(),
        Read::Density(..) => "Hücreler nokta sayısıyla koyulaşır; kırmızı boş, sarı seyrek; çizgi proje sınırı.".to_owned(),
    })
}

/// A table of facts: what and how much, the kit's data table (DESIGN.md §7.19: no hand-made label rows).
fn facts<'a>(rows: Vec<(String, String)>) -> Element<'a, Message> {
    use crate::road_dialog::{Col, col};
    crate::road_dialog::table(
        vec![col("Ölçü", 170.0), Col { title: "Değer", width: None, number: false }],
        rows.into_iter().map(|(k, v)| vec![k, v]).collect(),
        "",
    )
}

/// The content tab's tables: the facts first.
const SUMMARY: &str = "Özet";
const RANGES: &str = "Aralıklar";
const MATERIALS: &str = "Malzemeler";
/// Fark by sections' table (docs/adr/4340): each section's distance and areas.
const SECTIONS: &str = "Kesitler";
const CHECKS: &str = "Kontrol noktaları";
const ROBUST: &str = "Sağlam ölçüler";

/// A table with the line under it that says what its numbers are.
fn said<'a>(table: Element<'a, Message>, words: String) -> Element<'a, Message> {
    let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
    Column::new().spacing(6).push(table).push(label::caption(words).style(muted)).into()
}

/// The content tab: at most four figures read at a glance, then the reading's tables by name, one shown at a time.
fn content<'a>(w: &'a Window, read: &'a Read) -> (Vec<(String, String)>, Vec<(&'static str, Element<'a, Message>)>) {
    use crate::road_dialog::{col, grouped, num};
    let m3 = |v: f64| format!("{} m³", grouped(v, 2));
    let m2 = |v: f64| format!("{} m²", grouped(v, 2));
    let mut figures: Vec<(String, String)> = Vec::new();
    let mut parts: Vec<(&'static str, Element<'a, Message>)> = Vec::new();
    match read {
        Read::Analysis(a) => {
            figures.push(("Alan".into(), m2(a.area)));
            figures.push(("3B alan".into(), m2(a.surface_area)));
            let unit = ["m", "%", "°"][w.measure];
            if let Some(b) = a.elevations.as_ref().or(a.slopes.as_ref()).or(a.aspects.as_ref()) {
                figures.push(("Aralık".into(), b.bands.len().to_string()));
                parts.push((RANGES, band_table(b, unit, a.area)));
            }
            let mut rows = Vec::new();
            if let Some(r) = &a.roughness {
                rows.push(("Pürüzlülük".into(), format!("ortalama {:.4} · en çok {:.4}", r.mean, r.max)));
            }
            if let Some(g) = &a.grid {
                rows.push(("Raster ile TIN".into(), format!("{} hücre · ortalama {:+.2} % · en çok {:.2} %", g.cells, g.mean, g.max_absolute)));
            }
            if !rows.is_empty() {
                parts.push((SUMMARY, facts(rows)));
            }
        }
        Read::Compare(c) => {
            figures.push(("Kazı".into(), m3(c.cut)));
            figures.push(("Dolgu".into(), m3(c.fill)));
            figures.push(("Net".into(), format!("{}{} m³", if c.net > 0.0 { "+" } else { "" }, grouped(c.net, 2))));
            if let Some(t) = &c.zoned {
                figures.push(("Malzemeye göre net".into(), format!("{}{} m³", if t.net > 0.0 { "+" } else { "" }, grouped(t.net, 2))));
            } else if let Some(a) = &c.adjusted {
                figures.push(("Katsayılı net".into(), format!("{}{} m³", if a.net > 0.0 { "+" } else { "" }, grouped(a.net, 2))));
            }
            let method = match (c.cell, c.interval) {
                (Some(cell), _) => format!("Izgara, {cell} m hücre (orta nokta)"),
                // Docs/adr/4340: the sections' spacing, the axis and the formula.
                (None, Some(interval)) => format!(
                    "Kesit, {} m arayla, eksenin semti {}° ({})",
                    format!("{interval}").replace('.', ","),
                    format!("{}", c.azimuth.unwrap_or(0.0)).replace('.', ","),
                    if c.formula == Some(kentos_commands::SectionFormula::Prismoidal) { "prizmoidal" } else { "ortalama alan" }
                ),
                (None, None) => "TIN–TIN, kesin prizmalar".to_owned(),
            };
            let mut rows = vec![
                ("Yöntem".to_owned(), method),
                ("Kazı alanı".to_owned(), m2(c.cut_area)),
                ("Dolgu alanı".to_owned(), m2(c.fill_area)),
                ("Tabanın dışında".to_owned(), m2(c.uncovered)),
            ];
            if let (Some(l), Some(h)) = (&c.least, &c.largest) {
                rows.push(("Fark".into(), format!("{:+.3} … {:+.3} m", l.difference, h.difference).replace('.', ",")));
            }
            // The grid beside the exact TIN: their difference is the method's.
            if let Some(e) = &c.exact {
                rows.push(("Kesin (TIN)".into(), format!("kazı {} · dolgu {} m³", grouped(e.cut, 2), grouped(e.fill, 2))));
                rows.push(("Yöntem farkı".into(), format!("kazı {:+.2} · dolgu {:+.2} m³", c.cut - e.cut, c.fill - e.fill)));
            }
            if let Some(a) = &c.adjusted {
                rows.push((
                    format!("Katsayılı (×{} / ×{})", a.cut_factor, a.fill_factor).replace('.', ","),
                    format!("kazı {} · dolgu {} m³", grouped(a.cut, 2), grouped(a.fill, 2)),
                ));
            }
            parts.push((SUMMARY, facts(rows)));
            if !c.bands.is_empty() {
                let mut rows: Vec<Vec<String>> = c.bands.iter().map(|b| vec![format!("{:+.2} … {:+.2}", b.from, b.to).replace('.', ","), grouped(b.area, 2)]).collect();
                if let Some(o) = c.outside_area {
                    rows.push(vec!["Aralıkların dışı".to_owned(), grouped(o, 2)]);
                }
                parts.push((RANGES, crate::road_dialog::table(vec![col("Fark (m)", 170.0), num("Alan m²", 110.0)], rows, "")));
            }
            // The sections (docs/adr/4340): each one's distance from the first and its cut and fill areas.
            if !c.sections.is_empty() {
                let rows: Vec<Vec<String>> = c
                    .sections
                    .iter()
                    .enumerate()
                    .map(|(k, s)| vec![(k + 1).to_string(), grouped(s.station, 2), grouped(s.cut_area, 2), grouped(s.fill_area, 2)])
                    .collect();
                let table = crate::road_dialog::table(
                    vec![num("Kesit", 60.0), num("Uzaklık m", 100.0), num("Kazı alanı m²", 120.0), num("Dolgu alanı m²", 120.0)],
                    rows,
                    "Kesit yok",
                );
                parts.push((SECTIONS, said(table, format!("{} kesit; alanlar iki yüzeyin ortak yerinde kesin.", c.sections.len()))));
            }
            // The material zones (docs/adr/3540): each zone and the rest (the exact TIN's cut and fill, the factors, the
            // adjusted net), then the quantity by material.
            if let (false, Some(r), Some(t)) = (c.zones.is_empty(), &c.rest, &c.zoned) {
                let factors = |a: &kentos_commands::TerrainAdjusted| format!("{} / {}", a.cut_factor, a.fill_factor).replace('.', ",");
                let mut rows: Vec<Vec<String>> = c
                    .zones
                    .iter()
                    .map(|z| vec![z.name.clone(), grouped(z.covered, 2), grouped(z.cut, 2), grouped(z.fill, 2), factors(&z.adjusted), grouped(z.adjusted.net, 2)])
                    .collect();
                rows.push(vec!["Bölge dışı".to_owned(), grouped(r.covered, 2), grouped(r.cut, 2), grouped(r.fill, 2), factors(&r.adjusted), grouped(r.adjusted.net, 2)]);
                let table = crate::road_dialog::table(
                    vec![col("Malzeme", 120.0), num("Alan m²", 90.0), num("Kazı m³", 90.0), num("Dolgu m³", 90.0), num("Katsayı", 90.0), num("Net m³", 90.0)],
                    rows,
                    "Bölge yok",
                );
                let words = format!(
                    "Kazı ve dolgu kesin TIN'in, net katsayılı. Malzemeye göre katsayılı kazı {} · dolgu {} m³.",
                    grouped(t.cut, 2),
                    grouped(t.fill, 2)
                );
                parts.push((MATERIALS, said(table, words)));
            }
        }
        Read::Quality(q) => {
            figures.push(("Köşe".into(), q.corners.to_string()));
            figures.push(("Üçgen".into(), q.triangles.to_string()));
            if let Some(n) = q.near_count {
                figures.push(("Yakın çift".into(), n.to_string()));
            }
            if let Some(n) = q.spike_count {
                figures.push(("Sivri nokta".into(), n.to_string()));
            }
            let mut rows = Vec::new();
            if let Some(s) = q.spikes.first() {
                rows.push(("En büyük sivri".to_owned(), format!("{:+.2} m", s.difference)));
            }
            if let Some(g) = &q.gaps {
                rows.push(("Boşluk".into(), format!("{} üçgen · {} · en uzun kenar {:.2} m", g.triangles, m2(g.area), g.longest)));
            }
            if !rows.is_empty() {
                parts.push((SUMMARY, facts(rows)));
            }
            if let Some(c) = &q.checkpoints {
                let name = |k: &'a kentos_commands::TerrainCheckClass| -> &'a str {
                    if k.class.is_empty() {
                        if std::ptr::eq(k, &c.all) { "Hepsi" } else { "Sınıfsız" }
                    } else {
                        k.class.as_str()
                    }
                };
                let d = |v: f64| format!("{v:.3}").replace('.', ",");
                let rows = c
                    .classes
                    .iter()
                    .chain(std::iter::once(&c.all))
                    .map(|k| vec![name(k).to_owned(), k.count.to_string(), format!("{}{}", if k.mean >= 0.0 { "+" } else { "" }, d(k.mean)), d(k.std), d(k.rmse), k.outside.to_string()])
                    .collect();
                let table = crate::road_dialog::table(
                    vec![col("Sınıf", 120.0), num("Sayı", 60.0), num("Ort.", 80.0), num("Std.", 80.0), num("RMSE", 80.0), num("Dışı", 60.0)],
                    rows,
                    "Kontrol noktası yok",
                );
                parts.push((CHECKS, said(table, "Nokta − yüzey, m.".to_owned())));
                // Beyond one RMSE (docs/adr/0945): the robust measures and the RMSE's 95 % interval.
                let robust: Vec<Vec<String>> = c
                    .classes
                    .iter()
                    .chain(std::iter::once(&c.all))
                    .filter_map(|k| {
                        let r = k.robust.as_ref()?;
                        Some(vec![
                            name(k).to_owned(),
                            d(r.median),
                            d(r.nmad),
                            d(r.q68),
                            d(r.q95),
                            format!("{}–{}", d(r.rmse_low), d(r.rmse_high)),
                            r.combined.map_or_else(|| "—".to_owned(), d),
                            r.far.to_string(),
                        ])
                    })
                    .collect();
                if !robust.is_empty() {
                    let pick = |ks: &[usize]| robust.iter().map(|r| ks.iter().map(|k| r[*k].clone()).collect()).collect::<Vec<Vec<String>>>();
                    // The robust measures and the RMSE's own interval side by side, one table (Höhle and Höhle).
                    let table = crate::road_dialog::table(
                        vec![
                            col("Sınıf", 110.0),
                            num("Medyan", 70.0),
                            num("NMAD", 70.0),
                            num("%68", 64.0),
                            num("%95", 64.0),
                            num("Uzak", 54.0),
                            num("RMSE %95 aralığı", 130.0),
                            num("Kontrolle birleşik", 130.0),
                        ],
                        pick(&[0, 1, 2, 3, 4, 7, 5, 6]),
                        "",
                    );
                    let words = "Medyan, NMAD ve |fark|'ın yüzdelikleri aykırı noktalardan etkilenmez (Höhle ve Höhle); RMSE'nin aralığı χ² ile %95, birleşik hata kontrol noktalarının doğruluğuyla.";
                    parts.push((ROBUST, said(table, words.to_owned())));
                }
            }
        }
        Read::Coverage(c, _) => {
            figures.push(("Yüzeyin alanı".into(), m2(c.surface_area)));
            if let Some(share) = c.share {
                figures.push(("Örtülen".into(), format!("%{}", format!("{:.1}", 100.0 * share).replace('.', ","))));
            }
            figures.push(("Tahmin edilen".into(), format!("%{}", format!("{:.1}", 100.0 * c.estimated_share).replace('.', ","))));
            figures.push(("En uzun kenar".into(), format!("{} m", grouped(c.longest_edge, 2))));
            let mut rows = Vec::new();
            if let (Some(b), Some(gap), Some(out)) = (c.boundary_area, c.no_data, c.outside) {
                rows.push(("Sınırın alanı".to_owned(), m2(b)));
                rows.push(("Örtülen".to_owned(), m2(c.covered)));
                rows.push(("Verisiz".to_owned(), m2(gap)));
                rows.push(("Sınırın dışında".to_owned(), m2(out)));
            }
            rows.push(("Tahmin edilen".to_owned(), format!("{} · {} üçgen", m2(c.estimated), c.estimated_triangles)));
            parts.push((SUMMARY, facts(rows)));
        }
        Read::Density(d, _) => {
            figures.push(("Sayılan hücre".into(), d.cells.to_string()));
            figures.push(("Noktalı".into(), format!("%{}", format!("{:.1}", 100.0 * d.share.unwrap_or(0.0)).replace('.', ","))));
            figures.push(("Boş".into(), d.empty.to_string()));
            if let Some(n) = d.sparse {
                figures.push(("Seyrek".into(), n.to_string()));
            }
            let mut rows = vec![
                ("Izgara".to_owned(), format!("{} × {} · {} m hücre", d.columns, d.rows, d.cell)),
                ("Sayılan alan".to_owned(), m2(d.area)),
                ("Noktalı hücre".to_owned(), d.filled.to_string()),
                ("Nokta".to_owned(), format!("{} · en kalabalık hücre {}", d.points, d.most)),
                ("Ortalama yoğunluk".to_owned(), d.mean.map_or("—".into(), |m| format!("{m:.3} nokta/m²").replace('.', ","))),
            ];
            if let Some(meets) = d.meets {
                rows.push(("İstenen pay".into(), if meets { "tutuyor".into() } else { "tutmuyor".into() }));
            }
            parts.push((SUMMARY, facts(rows)));
        }
    }
    figures.truncate(4);
    // The facts first, then the reading's own lists.
    parts.sort_by_key(|(name, _)| *name != SUMMARY);
    (figures, parts)
}

fn band_table<'a>(b: &TerrainBands, unit: &str, whole: f64) -> Element<'a, Message> {
    use crate::road_dialog::{col, num};
    let d = |v: f64| crate::road_dialog::grouped(v, 2);
    let n = b.bands.len();
    let mut rows: Vec<Vec<Element<'a, Message>>> = b
        .bands
        .iter()
        .enumerate()
        .map(|(k, band)| {
            let swatch = iced::widget::canvas(Swatch { k, n }).width(10).height(10);
            let share = if whole > 0.0 { band.area / whole * 100.0 } else { 0.0 };
            vec![
                row![swatch, kentos_ui::widget::table::text(format!("{} … {} {unit}", band.from, band.to))]
                    .spacing(6)
                    .align_y(iced::alignment::Vertical::Center)
                    .into(),
                kentos_ui::widget::table::number(d(band.area)).into(),
                kentos_ui::widget::table::number(d(band.surface_area)).into(),
                kentos_ui::widget::table::number(format!("%{}", format!("{share:.1}").replace('.', ","))).into(),
            ]
        })
        .collect();
    let plain = |what: &'a str, area: f64| -> Vec<Element<'a, Message>> {
        vec![
            kentos_ui::widget::table::text(what).into(),
            kentos_ui::widget::table::number(d(area)).into(),
            kentos_ui::widget::table::text("").into(),
            kentos_ui::widget::table::text("").into(),
        ]
    };
    if b.outside_area > 0.0 {
        rows.push(plain("Aralık dışı", b.outside_area));
    }
    if b.flat_area > 0.0 {
        rows.push(plain("Düz (bakısız)", b.flat_area));
    }
    crate::road_dialog::table_of(
        vec![col("Aralık", 116.0), num("Alan (m²)", 84.0), num("3B (m²)", 84.0), num("Pay", 48.0)],
        rows,
        "Aralık yok",
    )
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
    /// Yoğunluk's least: a cell with fewer points is sparse.
    least: Option<u32>,
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
                // The material zones (docs/adr/3540) without ranges: each zone's and the rest's adjusted cut and fill.
                if c.bands.is_empty() && !c.zones.is_empty() {
                    let mut rows: Vec<(String, f64, f64)> = c.zones.iter().map(|z| (z.name.clone(), z.adjusted.cut, z.adjusted.fill)).collect();
                    if let Some(r) = &c.rest {
                        rows.push(("Bölge dışı".to_owned(), r.adjusted.cut, r.adjusted.fill));
                    }
                    let most = rows.iter().flat_map(|r| [r.1, r.2]).fold(0.0_f64, f64::max).max(1e-9);
                    let h = ((bounds.height - 40.0) / rows.len() as f32).min(56.0);
                    text(&mut frame, "Malzemeye göre katsayılı kazı ve dolgu".into(), Point::new(10.0, 6.0));
                    for (k, (name, cut, fill)) in rows.iter().enumerate() {
                        let y = 28.0 + k as f32 * h;
                        text(&mut frame, name.clone(), Point::new(10.0, y + h / 2.0 - 9.0));
                        let bar = ((h - 10.0) / 2.0).max(2.0);
                        for (j, (v, color, word)) in [(*cut, tk.danger, "kazı"), (*fill, tk.success, "dolgu")].into_iter().enumerate() {
                            let w = ((v / most) as f32) * (bounds.width - 290.0).max(10.0);
                            let top = y + 2.0 + j as f32 * (bar + 2.0);
                            frame.fill_rectangle(Point::new(130.0, top), iced::Size::new(w.max(1.0), bar), color);
                            text(&mut frame, format!("{word} {v:.2} m³"), Point::new(136.0 + w, top + bar / 2.0 - 6.0));
                        }
                    }
                    return vec![frame.into_geometry()];
                }
                // The sections (docs/adr/4340) along the axis: the fill's area up, the cut's down.
                if !c.sections.is_empty() {
                    let last = c.sections.last().map_or(0.0, |s| s.station).max(1e-9);
                    let most = c.sections.iter().flat_map(|s| [s.cut_area, s.fill_area]).fold(0.0_f64, f64::max).max(1e-9);
                    text(&mut frame, "Kesit alanları boyunca: dolgu yukarı, kazı aşağı".into(), Point::new(10.0, 6.0));
                    let (left, right) = (70.0_f32, (bounds.width - 24.0).max(80.0));
                    let half = ((bounds.height - 70.0) / 2.0).max(8.0);
                    let mid = 30.0 + half;
                    frame.stroke(
                        &Path::line(Point::new(left, mid), Point::new(right, mid)),
                        Stroke::default().with_width(1.0).with_color(tk.muted),
                    );
                    let bar = ((right - left) / c.sections.len() as f32 * 0.6).clamp(2.0, 16.0);
                    for s in &c.sections {
                        let x = left + (s.station / last) as f32 * (right - left);
                        let (up, down) = ((s.fill_area / most) as f32 * half, (s.cut_area / most) as f32 * half);
                        if up > 0.0 {
                            frame.fill_rectangle(Point::new(x - bar / 2.0, mid - up), iced::Size::new(bar, up), tk.success);
                        }
                        if down > 0.0 {
                            frame.fill_rectangle(Point::new(x - bar / 2.0, mid), iced::Size::new(bar, down), tk.danger);
                        }
                    }
                    text(&mut frame, format!("{most:.1} m²"), Point::new(10.0, mid - half));
                    text(&mut frame, format!("{most:.1} m²"), Point::new(10.0, mid + half - 12.0));
                    text(&mut frame, "0 m".into(), Point::new(left - 8.0, mid + half + 8.0));
                    text(&mut frame, format!("{last:.0} m"), Point::new(right - 40.0, mid + half + 8.0));
                    return vec![frame.into_geometry()];
                }
                if c.bands.is_empty() {
                    let words = if c.cell.is_some() {
                        "Izgara yönteminde fark aralığı yok: hacimler ve kesin TIN'e göre yöntem farkı İçerik'te."
                    } else {
                        "Fark aralığı yazılmadı: hacimler İçerik'te."
                    };
                    text(&mut frame, words.into(), Point::new(10.0, 10.0));
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
            Read::Coverage(_, pic) => {
                let pts: Vec<[f64; 2]> =
                    pic.triangles.iter().flat_map(|t| t.0).chain(pic.rings.iter().flatten().copied()).collect();
                let Some(fit) = Fit::of(&pts, bounds) else {
                    text(&mut frame, "Gösterilecek üçgen yok.".into(), Point::new(10.0, 10.0));
                    return vec![frame.into_geometry()];
                };
                for (t, estimated) in &pic.triangles {
                    let shape = Path::new(|p| {
                        p.move_to(fit.at(t[0]));
                        p.line_to(fit.at(t[1]));
                        p.line_to(fit.at(t[2]));
                        p.close();
                    });
                    let color = if *estimated { tk.warning } else { tk.success };
                    frame.fill(&shape, Color { a: 0.35, ..color });
                    frame.stroke(&shape, Stroke::default().with_width(0.6).with_color(Color { a: 0.6, ..color }));
                }
                for ring in &pic.rings {
                    let shape = Path::new(|p| {
                        p.move_to(fit.at(ring[0]));
                        for q in &ring[1..] {
                            p.line_to(fit.at(*q));
                        }
                        p.close();
                    });
                    frame.stroke(&shape, Stroke::default().with_width(2.0).with_color(tk.accent));
                }
                text(&mut frame, "Yeşil ölçülen, turuncu tahmin edilen; çizgi proje sınırı".into(), Point::new(10.0, 6.0));
            }
            Read::Density(d, pic) => {
                let (w, h) = (pic.columns as f64 * pic.cell, pic.rows as f64 * pic.cell);
                let corners = [pic.origin, [pic.origin[0] + w, pic.origin[1] + h]];
                let pts: Vec<[f64; 2]> = corners.into_iter().chain(pic.rings.iter().flatten().copied()).collect();
                let Some(fit) = Fit::of(&pts, bounds) else {
                    text(&mut frame, "Gösterilecek hücre yok.".into(), Point::new(10.0, 10.0));
                    return vec![frame.into_geometry()];
                };
                let most = d.most.max(1);
                for (k, n) in pic.counts.iter().enumerate() {
                    let Some(n) = *n else { continue };
                    let (i, j) = (k % pic.columns, k / pic.columns);
                    let x = pic.origin[0] + i as f64 * pic.cell;
                    let y = pic.origin[1] + j as f64 * pic.cell;
                    let (a, b) = (fit.at([x, y + pic.cell]), fit.at([x + pic.cell, y]));
                    let size = iced::Size::new((b.x - a.x).max(1.0), (b.y - a.y).max(1.0));
                    let sparse = d.sparse.is_some() && self.least.is_some_and(|l| n < l);
                    let color = if n == 0 {
                        Color { a: 0.45, ..tk.danger }
                    } else if sparse {
                        Color { a: 0.5, ..tk.warning }
                    } else {
                        Color { a: 0.2 + 0.6 * (n as f32 / most as f32), ..tk.success }
                    };
                    frame.fill_rectangle(a, size, color);
                    if size.width > 14.0 && size.height > 12.0 {
                        frame.fill_text(Text {
                            content: n.to_string(),
                            position: Point::new(a.x + size.width / 2.0, a.y + size.height / 2.0),
                            color: tk.text,
                            size: Pixels(10.0),
                            align_x: iced::widget::text::Alignment::Center,
                            align_y: iced::alignment::Vertical::Center,
                            ..Text::default()
                        });
                    }
                }
                for ring in &pic.rings {
                    let shape = Path::new(|p| {
                        p.move_to(fit.at(ring[0]));
                        for q in &ring[1..] {
                            p.line_to(fit.at(*q));
                        }
                        p.close();
                    });
                    frame.stroke(&shape, Stroke::default().with_width(2.0).with_color(tk.accent));
                }
                text(&mut frame, "Hücrelerde nokta sayısı: kırmızı boş, turuncu seyrek; çizgi proje sınırı".into(), Point::new(10.0, 6.0));
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
                                // Far from its class (docs/adr/0945): a ring, so it is seen before it is counted.
                                if p.far {
                                    frame.stroke(&Path::circle(at, r + 4.0), Stroke::default().with_width(2.0).with_color(tk.warning));
                                }
                                text(&mut frame, if p.far { format!("{d:+.3} uzak") } else { format!("{d:+.3}") }, at + iced::Vector::new(r + 6.0, -6.0));
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
