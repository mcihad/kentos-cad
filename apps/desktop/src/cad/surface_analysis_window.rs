//! Yüzey analizi (plan/road.md Y18.5, docs/adr/0916 §5, `terrain.analysis`): a terrain surface read three ways, nothing
//! written. Analiz: elevation, slope or aspect ranges coloured in plan with
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
    CompareZone, TerrainBands, TerrainCheckpointsAsk, TerrainSpikesAsk, TerrainsAnalysed,
    TerrainsAnalysis, TerrainsCompare, TerrainsCompared, TerrainsCoverage, TerrainsCoverageDraw,
    TerrainsCovered, TerrainsDensity, TerrainsDensityRead, TerrainsQuality, TerrainsQualityRead,
    TerrainsRepair, TerrainsRepaired, TerrainsSparseDraw,
};
use kentos_interaction::Level;
use kentos_native_application::{
    ExecutionContext, road_terrains_analysis, road_terrains_compare, road_terrains_coverage,
    road_terrains_coverage_draw, road_terrains_density, road_terrains_quality,
    road_terrains_repair, road_terrains_sparse,
};
use kentos_ui::label;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::select::{Choice, Select};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

use self::chart::Plan;
pub use self::content::Tab;
use self::content::{SUMMARY, content, legend};

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
pub(crate) const FIELDS: [&str; 22] = [
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
    // Sections along a drawn axis (docs/adr/4760): the widths and the start's kilometre.
    "Sola genişlik (m)",
    "Sağa genişlik (m)",
    "Başlangıç km (m)",
];
/// A section's width each side when none is typed, m (docs/adr/4760; KGM 2019: the ground generally at least 50 m wide).
const SECTION_WIDTH: f64 = 25.0;
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
#[allow(clippy::large_enum_variant)]
pub enum Read {
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
    pub surface: Option<String>,
    pub comparison: Option<String>,
    /// The two surfaces' heights taken as one vertical system when not shown to be (docs/adr/1029).
    pub(crate) assume_heights: bool,
    pub(crate) measure: usize,
    pub(crate) roughness: bool,
    /// Fark by a grid's cells (docs/adr/1600), else the exact TIN–TIN prisms.
    pub(crate) volume_grid: bool,
    /// Fark by parallel sections (docs/adr/4340) and their formula: the average end area, the prismoidal or (docs/adr/5140)
    /// the frustum's.
    pub(crate) volume_sections: bool,
    pub(crate) formula: kentos_commands::SectionFormula,
    /// Fark by sections along a drawn axis (docs/adr/4760): a line's or an open polyline's uid; none, the azimuth's.
    pub(crate) axis: Option<String>,
    /// Sections at the axis's corners too.
    pub(crate) corners: bool,
    /// The cut and fill zones' labels and quantity table (docs/adr/2340), their deepest places (docs/adr/2740).
    pub(crate) zone_labels: bool,
    pub(crate) zone_table: bool,
    pub(crate) zone_depths: bool,
    /// The material zones' names and volumes written with the zones (docs/adr/4760): asked, never by itself.
    pub(crate) zone_materials: bool,
    /// Fark's material zones (docs/adr/3540): the layer whose closed objects bound them; none, no zones.
    pub(crate) materials: Option<String>,
    pub(crate) checks: Option<String>,
    /// Kapsam's boundary: a closed object's uid; none, the surface itself.
    pub(crate) boundary: Option<String>,
    /// Kapsam's layer for the low confidence areas.
    pub(crate) layer: Option<String>,
    pub(crate) fields: [String; 22],
    /// The result's tab: the chart or the content; the content's table shown (one at a time, DESIGN.md §7.19).
    pub(crate) tab: Tab,
    pub(crate) list: &'static str,
    pub preview: Option<Result<Read, String>>,
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
    /// Fark by sections: the formula (docs/adr/5140).
    Formula(kentos_commands::SectionFormula),
    /// Fark by sections along a drawn axis (docs/adr/4760), none the azimuth's; and sections at its corners too.
    Axis(Option<String>),
    Corners(bool),
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
    /// Fark: the material zones' words and their table with the zones (docs/adr/4760).
    ZoneMaterials(bool),
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
    Message::TerrainWork(crate::terrain_work::Event::SurfaceAnalysis(event))
}

/// A single number: a decimal comma read as a point, as the other road windows read it (lists keep the comma as their
/// separator).
fn number(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.replace(',', ".")
        .parse::<f64>()
        .map(Some)
        .map_err(|_| format!("{what}: “{t}” bir sayı değil."))
}

fn list(text: &str, what: &str) -> Result<Option<Vec<f64>>, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    t.split([',', ';', ' '])
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<f64>()
                .map_err(|_| format!("{what}: “{s}” bir sayı değil."))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

impl Window {
    fn surface(&self) -> Result<String, String> {
        self.surface
            .clone()
            .ok_or_else(|| "Bir arazi yüzeyi seçin.".to_owned())
    }

    pub(crate) fn analysis(&self) -> Result<TerrainsAnalysis, String> {
        let breaks = list(&self.fields[0], FIELDS[0])?;
        // The step the user can take, before the query is sent (UR2-009): ranges need two bounds at least, unless
        // Pürüzlülük or a raster asks something else.
        let asks_else = self.roughness || !self.fields[1].trim().is_empty();
        if breaks.as_ref().is_none_or(|b| b.len() < 2) && !asks_else {
            let what = ["Kotu", "Eğimi", "Bakıyı"][self.measure.min(2)];
            return Err(format!(
                "{what} sınıflandırmak için Sınırlar alanına en az iki sınır yazın (ör. 0, 30, 70)."
            ));
        }
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
    fn materials_of(
        &self,
        doc: &kentos_domain::Document,
        layers: &[(String, String)],
    ) -> Result<Vec<CompareZone>, String> {
        let Some(layer) = &self.materials else {
            return Ok(Vec::new());
        };
        let layer_name = layers
            .iter()
            .find(|(id, _)| id == layer)
            .map_or_else(|| layer.clone(), |(_, n)| n.clone());
        let mut out = Vec::new();
        for (uid, e) in doc.entities_with_uids() {
            let uid = uid.to_string();
            if e.base().layer_id != *layer
                || road_terrains_coverage::boundary_region(doc, &uid).is_none()
            {
                continue;
            }
            let attrs = &e.base().attrs;
            let factor = |key: &str| attrs.get(key).map_or(Ok(None), |t| number(t, key));
            out.push(CompareZone {
                name: attrs
                    .get(MATERIAL)
                    .filter(|n| !n.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| layer_name.clone()),
                boundary: uid,
                cut_factor: factor(CUT_FACTOR)?,
                fill_factor: factor(FILL_FACTOR)?,
            });
        }
        Ok(out)
    }

    pub(crate) fn compare(
        &self,
        doc: &kentos_domain::Document,
        layers: &[(String, String)],
    ) -> Result<TerrainsCompare, String> {
        Ok(TerrainsCompare {
            base: self.surface()?,
            comparison: self
                .comparison
                .clone()
                .ok_or("Karşılaştırılacak ikinci yüzeyi seçin.")?,
            ranges: if self.volume_grid || self.volume_sections {
                None
            } else {
                list(&self.fields[2], FIELDS[2])?
            },
            method: if self.volume_sections {
                Some(kentos_commands::CompareMethod::Sections)
            } else {
                self.volume_grid
                    .then_some(kentos_commands::CompareMethod::Grid)
            },
            cell: if self.volume_grid {
                number(&self.fields[13], FIELDS[13])?
            } else {
                None
            },
            cut_factor: number(&self.fields[14], FIELDS[14])?,
            fill_factor: number(&self.fields[15], FIELDS[15])?,
            assume_heights: self.assume_heights,
            zones: if self.volume_grid || self.volume_sections {
                Vec::new()
            } else {
                self.materials_of(doc, layers)?
            },
            // Docs/adr/4340: the sections' spacing (none typed: one every 10 m), the axis's azimuth and the formula.
            interval: if self.volume_sections {
                Some(number(&self.fields[17], FIELDS[17])?.unwrap_or(SECTION_INTERVAL))
            } else {
                None
            },
            azimuth: if self.volume_sections && self.axis.is_none() {
                number(&self.fields[18], FIELDS[18])?
            } else {
                None
            },
            formula: (self.volume_sections
                && self.formula != kentos_commands::SectionFormula::EndArea)
                .then_some(self.formula),
            // Docs/adr/4760: along a drawn axis, its widths (none typed: 25 m each side) and its corners.
            axis: self.axis.clone().filter(|_| self.volume_sections),
            left: self.along().then(|| self.width(19)).transpose()?,
            right: self.along().then(|| self.width(20)).transpose()?,
            corners: (self.along() && self.corners).then_some(true),
        })
    }

    /// Whether Fark measures by sections along a drawn axis (docs/adr/4760).
    fn along(&self) -> bool {
        self.volume_sections && self.axis.is_some()
    }

    /// A section's width typed in field `k`, 25 m when none is.
    fn width(&self, k: usize) -> Result<f64, String> {
        Ok(number(&self.fields[k], FIELDS[k])?.unwrap_or(SECTION_WIDTH))
    }

    /// The sections a cut and fill drawing writes (docs/adr/4760): Fark's along its drawn axis, none without one.
    pub(crate) fn drawn_sections(
        &self,
    ) -> Result<Option<kentos_commands::CutFillSections>, String> {
        let Some(axis) = self.axis.clone().filter(|_| self.volume_sections) else {
            return Ok(None);
        };
        Ok(Some(kentos_commands::CutFillSections {
            axis,
            interval: number(&self.fields[17], FIELDS[17])?.unwrap_or(SECTION_INTERVAL),
            left: self.width(19)?,
            right: self.width(20)?,
            corners: self.corners.then_some(true),
            formula: (self.formula != kentos_commands::SectionFormula::EndArea)
                .then_some(self.formula),
            start: number(&self.fields[21], FIELDS[21])?,
            // Docs/adr/5140: the Brückner value with the comparison's cut and fill factors.
            cut_factor: number(&self.fields[14], FIELDS[14])?,
            fill_factor: number(&self.fields[15], FIELDS[15])?,
        }))
    }

    pub(crate) fn coverage(&self) -> Result<TerrainsCoverage, String> {
        Ok(TerrainsCoverage {
            surface: self.surface()?,
            boundary: self.boundary.clone(),
            longest: number(&self.fields[8], FIELDS[8])?,
        })
    }

    /// A count typed: a whole number, none when empty.
    fn least(&self) -> Result<Option<u32>, String> {
        let t = self.fields[10].trim();
        if t.is_empty() {
            return Ok(None);
        }
        t.parse::<u32>()
            .map(Some)
            .map_err(|_| format!("{} tam sayı olmalı: “{t}”.", FIELDS[10]))
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
        let (radius, limit) = (
            number(&self.fields[4], FIELDS[4])?,
            number(&self.fields[5], FIELDS[5])?,
        );
        let spikes = match (radius, limit) {
            (Some(radius), Some(limit)) => Some(TerrainSpikesAsk { radius, limit }),
            (None, None) => None,
            _ => return Err("Sivri nokta için erim ile sınır birlikte yazılır.".to_owned()),
        };
        let field = self.fields[7].trim();
        // The check points' own RMSE (docs/adr/0945): read only when there are check points.
        let sigma = self
            .checks
            .as_ref()
            .map(|_| number(&self.fields[12], FIELDS[12]))
            .transpose()?;
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
    let t = if n > 1 {
        k as f32 / (n - 1) as f32
    } else {
        0.0
    };
    let mix = |a: Color, b: Color, s: f32| {
        Color::from_rgb(
            a.r + (b.r - a.r) * s,
            a.g + (b.g - a.g) * s,
            a.b + (b.b - a.b) * s,
        )
    };
    if t < 0.5 {
        mix(tk.success, tk.warning, t * 2.0)
    } else {
        mix(tk.warning, tk.danger, (t - 0.5) * 2.0)
    }
}

impl App {
    /// Yüzey analizi: the window on the road project's first terrain surface.
    pub(crate) fn open_surface_analysis(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if !kentos_native_application::road_terrains::holds_surfaces(doc.model.settings().cad_work)
        {
            self.warn("Yüzey analizi Harita ve Yol projelerinde olur.".to_owned());
            return;
        }
        let surfaces: Vec<String> = doc
            .model
            .settings()
            .road
            .as_ref()
            .map_or_else(Vec::new, |r| {
                r.surfaces.iter().map(|s| s.id.clone()).collect()
            });
        // The selection's own surface when every selected object is its source (docs/adr/2480), else the first.
        let picked = self
            .selection_roles()
            .surface
            .filter(|id| surfaces.contains(id));
        let Some(first) = picked.or_else(|| surfaces.first().cloned()) else {
            self.warn("Yüzey analizi bir arazi yüzeyinde okunur: önce Arazi yüzeyleri'nde bir yüzey oluşturun.".to_owned());
            return;
        };
        let comparison = surfaces.iter().find(|s| **s != first).cloned();
        self.terrain_work.surface_analysis_window = Some(Box::new(Window {
            mode: Mode::Analysis,
            surface: Some(first),
            comparison,
            assume_heights: false,
            measure: 1,
            roughness: false,
            volume_grid: false,
            volume_sections: false,
            formula: kentos_commands::SectionFormula::EndArea,
            // A single selected line or open polyline is the sections' axis (docs/adr/4760).
            axis: self.selected_axis(),
            corners: false,
            zone_labels: false,
            zone_table: false,
            zone_depths: false,
            zone_materials: false,
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
        self.dialog = Some(Dialog::TerrainWork(
            crate::terrain_work::TerrainDialogKind::SurfaceAnalysis,
        ));
    }

    /// The sections' axis the selection gives (docs/adr/4760): its one object when it is a line or a single-part open
    /// polyline; none otherwise.
    fn selected_axis(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        let [slot] = self.selection.ids()[..] else {
            return None;
        };
        let open = match doc.model.get(slot)? {
            kentos_commands::Entity::Line(_) => true,
            kentos_commands::Entity::Polyline(p) => {
                !p.closed && p.parts.as_ref().is_none_or(|q| q.is_empty())
            }
            _ => false,
        };
        open.then(|| doc.model.uid(slot).map(|u| u.to_string()))
            .flatten()
    }

    fn preview_surface_analysis(&mut self) {
        let layers = self.relation_layers();
        let Some(doc) = &self.document else { return };
        let Some(w) = self.terrain_work.surface_analysis_window.as_mut() else {
            return;
        };
        let read = match w.mode {
            Mode::Analysis => w
                .analysis()
                .and_then(|q| road_terrains_analysis::preview(&doc.model, &q))
                .map(|(r, x)| (Read::Analysis(r), x)),
            Mode::Compare => w
                .compare(&doc.model, &layers)
                .and_then(|q| road_terrains_compare::preview(&doc.model, &q))
                .map(|(r, x)| (Read::Compare(r), x)),
            Mode::Quality => w
                .quality()
                .and_then(|q| road_terrains_quality::preview(&doc.model, &q))
                .map(|(r, x)| (Read::Quality(r), x)),
            Mode::Coverage => w.coverage().and_then(|q| {
                let (r, x) = road_terrains_coverage::preview(&doc.model, &q)?;
                Ok((
                    Read::Coverage(r, road_terrains_coverage::picture(&doc.model, &q)?),
                    x,
                ))
            }),
            Mode::Density => w.density().and_then(|q| {
                let (r, x) = road_terrains_density::preview(&doc.model, &q)?;
                Ok((
                    Read::Density(r, road_terrains_density::picture(&doc.model, &q)?),
                    x,
                ))
            }),
        };
        match read {
            Ok((r, warnings)) => {
                w.preview = Some(Ok(r));
                w.warnings = warnings
                    .into_iter()
                    .map(|x| crate::road_dialog::heights_words(&x.message, HEIGHTS_CHOICE))
                    .collect();
            }
            Err(why) => {
                w.preview = Some(Err(crate::road_dialog::heights_words(&why, HEIGHTS_CHOICE)));
                w.warnings.clear();
            }
        }
    }

    pub(crate) fn surface_analysis_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.terrain_work.surface_analysis_window.as_mut() else {
            return Task::none();
        };
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
            Event::Formula(f) => w.formula = f,
            Event::Axis(a) => w.axis = a,
            Event::Corners(on) => w.corners = on,
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
            Event::ZoneMaterials(on) => {
                w.zone_materials = on;
                return Task::none();
            }
            Event::Checks(l) => w.checks = l,
            Event::Boundary(b) => w.boundary = b,
            Event::Layer(l) => {
                w.layer = Some(l);
                return Task::none();
            }
            Event::DrawLow => {
                let (Some(surface), Some(layer)) = (w.surface.clone(), w.layer.clone()) else {
                    return Task::none();
                };
                let Ok(Some(longest)) = number(&w.fields[8], FIELDS[8]) else {
                    self.warn("Düşük güvenli alan için en uzun kenarı yazın.".to_owned());
                    return Task::none();
                };
                let input = TerrainsCoverageDraw {
                    surface,
                    longest,
                    layer_id: layer,
                    outline: true,
                    labels: true,
                    expected_revision: None,
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = road_terrains_coverage_draw::execute(
                    &mut ExecutionContext::new(&mut doc.model),
                    input,
                );
                if let Some(out) = self.answered(result) {
                    self.say(
                        Level::Success,
                        format!(
                            "{} düşük güvenli alan çizildi ({:.1} m²).",
                            out.areas, out.area
                        ),
                    );
                }
                return Task::none();
            }
            Event::DrawZero => {
                let (Some(base), Some(comparison), Some(layer)) =
                    (w.surface.clone(), w.comparison.clone(), w.layer.clone())
                else {
                    return Task::none();
                };
                let input = kentos_commands::TerrainsZeroLine {
                    base,
                    comparison,
                    layer_id: layer,
                    expected_revision: None,
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
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
                let (Some(base), Some(comparison), Some(layer)) =
                    (w.surface.clone(), w.comparison.clone(), w.layer.clone())
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
                // Docs/adr/4760: Fark's sections along its axis and its material zones drawn with the zones.
                let sections = match w.drawn_sections() {
                    Ok(s) => s,
                    Err(why) => {
                        self.warn(why);
                        return Task::none();
                    }
                };
                let layers = self.relation_layers();
                let materials = {
                    let Some(w) = self.terrain_work.surface_analysis_window.as_ref() else {
                        return Task::none();
                    };
                    let Some(doc) = self.document.as_ref() else {
                        return Task::none();
                    };
                    match w.materials_of(&doc.model, &layers) {
                        Ok(m) if w.zone_materials && !w.volume_grid && !w.volume_sections => m,
                        Ok(_) => Vec::new(),
                        Err(why) => {
                            self.warn(why);
                            return Task::none();
                        }
                    }
                };
                let Some(w) = self.terrain_work.surface_analysis_window.as_ref() else {
                    return Task::none();
                };
                let input = kentos_commands::TerrainsCutFill {
                    base,
                    comparison,
                    layer_id: layer,
                    interval,
                    labels: w.zone_labels.then_some(true),
                    table: w.zone_table.then_some(true),
                    depths: w.zone_depths.then_some(true),
                    sections,
                    materials,
                    expected_revision: None,
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = kentos_native_application::road_terrains_cutfill::execute(
                    &mut ExecutionContext::new(&mut doc.model),
                    input,
                );
                if let Some(out) = self.answered(result) {
                    let cut = out
                        .zones
                        .iter()
                        .filter(|z| z.kind == kentos_commands::CutFillKind::Cut)
                        .count();
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
                let (Some(surface), Some(layer)) = (w.surface.clone(), w.layer.clone()) else {
                    return Task::none();
                };
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
                let input = TerrainsSparseDraw {
                    surface,
                    cell: q.cell,
                    least,
                    boundary: q.boundary,
                    layer_id: layer,
                    labels: true,
                    expected_revision: None,
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = road_terrains_sparse::execute(
                    &mut ExecutionContext::new(&mut doc.model),
                    input,
                );
                if let Some(out) = self.answered(result) {
                    self.say(
                        Level::Success,
                        format!("{} seyrek alan çizildi ({:.1} m²).", out.areas, out.area),
                    );
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
                let Some(surface) = w.surface.clone() else {
                    return Task::none();
                };
                let input = if matches!(event, Event::LeaveSpikes) {
                    let spikes = match &w.preview {
                        Some(Ok(Read::Quality(q))) => {
                            q.spikes.iter().map(|s| [s.at[0], s.at[1]]).collect()
                        }
                        _ => Vec::new(),
                    };
                    TerrainsRepair {
                        surface,
                        exclude: spikes,
                        restore: Vec::new(),
                        expected_revision: None,
                    }
                } else {
                    let left = self
                        .document
                        .as_ref()
                        .and_then(|d| d.model.settings().road.as_ref())
                        .and_then(|r| {
                            r.surfaces
                                .iter()
                                .find(|s| Some(&s.id) == w.surface.as_ref())
                                .map(|s| s.excluded.iter().map(|e| [e.x, e.y]).collect())
                        });
                    TerrainsRepair {
                        surface,
                        exclude: Vec::new(),
                        restore: left.unwrap_or_default(),
                        expected_revision: None,
                    }
                };
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let read = road_terrains_repair::preview(&doc.model, &input);
                if let Some(w) = self.terrain_work.surface_analysis_window.as_mut() {
                    w.repair = Some((input, read));
                }
                return Task::none();
            }
            Event::Apply => {
                let Some((input, Ok(_))) = w.repair.clone() else {
                    return Task::none();
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = road_terrains_repair::execute(
                    &mut ExecutionContext::new(&mut doc.model),
                    input,
                );
                if let Some(out) = self.answered(result) {
                    self.say(
                        Level::Success,
                        format!(
                            "Yüzey onarıldı; {} nokta kaydı dışarıda.",
                            out.excluded.len()
                        ),
                    );
                    if let Some(w) = self.terrain_work.surface_analysis_window.as_mut() {
                        w.repair = None;
                    }
                    self.preview_surface_analysis();
                }
                return Task::none();
            }
            Event::Close => {
                self.terrain_work.surface_analysis_window = None;
                self.dialog = None;
                return Task::none();
            }
        }
        // The result follows the form (DESIGN.md §7.19): read again, no Önizle.
        if let Some(w) = self.terrain_work.surface_analysis_window.as_mut() {
            w.repair = None;
        }
        self.preview_surface_analysis();
        Task::none()
    }

    pub(crate) fn surface_analysis_view(&self) -> Element<'_, Message> {
        use crate::road_dialog::RoadDialog;
        use kentos_ui::widget::Banner;
        let Some(w) = &self.terrain_work.surface_analysis_window else {
            return iced::widget::text("").into();
        };
        let model = self.document.as_ref().map(|d| &d.model);
        let surfaces: Vec<(String, String)> = model
            .and_then(|m| m.settings().road.as_ref())
            .map_or_else(Vec::new, |r| {
                r.surfaces
                    .iter()
                    .map(|s| (s.id.clone(), s.name.clone()))
                    .collect()
            });
        // The road windows' form (DESIGN.md §7.19): a 300 px column, a 130 px label column, one control a row, grouped;
        // help under a field only where the label does not say it. The writes are the footer's buttons.
        let pick_surface = |at: &Option<String>, on: fn(String) -> Event| {
            let ids: Vec<String> = surfaces.iter().map(|s| s.0.clone()).collect();
            let chosen = at.as_ref().and_then(|s| ids.iter().position(|i| i == s));
            container(Select::new(
                surfaces.iter().map(|s| Choice::new(s.1.clone())),
                chosen,
                move |i| msg(on(ids[i].clone())),
            ))
            .width(BOX)
        };
        let input = |hint: &'static str, k: usize| {
            kentos_ui::widget::text_field(hint, &w.fields[k])
                .on_input(move |t| msg(Event::Field(k, t)))
                .width(BOX)
        };
        let layers = self.relation_layers();
        // A layer chosen from the drawing's, “Yok” first where none is a choice.
        let pick_layer =
            |at: Option<&String>, none: bool, on: Box<dyn Fn(Option<String>) -> Message>| {
                let mut lids: Vec<Option<String>> =
                    layers.iter().map(|(id, _)| Some(id.clone())).collect();
                let mut names: Vec<String> = layers.iter().map(|(_, n)| n.clone()).collect();
                if none {
                    lids.insert(0, None);
                    names.insert(0, "Yok".to_owned());
                }
                let chosen = lids.iter().position(|l| l.as_ref() == at);
                container(
                    Select::new(names.into_iter().map(Choice::new), chosen, move |i| {
                        on(lids[i].clone())
                    })
                    .placeholder("Katman"),
                )
                .width(BOX)
            };
        let modes = [
            Mode::Analysis,
            Mode::Compare,
            Mode::Quality,
            Mode::Coverage,
            Mode::Density,
        ];
        let at = modes.iter().position(|m| *m == w.mode);
        let kip = container(Select::new(
            MODES.iter().map(|m| Choice::new(*m)),
            at,
            move |i| msg(Event::Mode(modes[i])),
        ))
        .width(BOX);
        let mut form = kentos_ui::widget::Form::new()
            .label_width(LABEL)
            .field("Kip", kip)
            .section("Yüzey");
        form = if w.mode == Mode::Compare {
            form.field("Taban", pick_surface(&w.surface, Event::Surface))
                .field(
                    "Karşılaştırma",
                    pick_surface(&w.comparison, Event::Comparison),
                )
                .row(words::check(
                    w.assume_heights,
                    SAME_HEIGHTS,
                    Some(msg(Event::AssumeHeights(!w.assume_heights))),
                ))
        } else {
            form.field("Yüzey", pick_surface(&w.surface, Event::Surface))
        };
        match w.mode {
            Mode::Analysis => {
                let measure = container(Select::new(
                    MEASURES.iter().map(|m| Choice::new(*m)),
                    Some(w.measure),
                    |i| msg(Event::Measure(i)),
                ))
                .width(BOX);
                form = form
                    .section("Aralıklar")
                    .field("Ölçü", measure)
                    .field(FIELDS[0], input("ör. 0, 2, 5, 10", 0))
                    .row(words::check(
                        w.roughness,
                        "Pürüzlülük",
                        Some(msg(Event::Roughness(!w.roughness))),
                    ))
                    .field(FIELDS[1], input("yok", 1))
                    .help("Raster eğimi üçgenlerinkiyle karşılaştırılır.");
            }
            Mode::Compare => {
                // The method and the factors (docs/adr/1600): a grid's cell instead of the ranges; the factors and the
                // material zones (docs/adr/3540, the TIN's alone) are the user's, none is guessed.
                let chosen = if w.volume_sections {
                    2
                } else {
                    usize::from(w.volume_grid)
                };
                let method = container(Select::new(
                    ["TIN–TIN", "Izgara", "Kesit"].into_iter().map(Choice::new),
                    Some(chosen),
                    |i| msg(Event::VolumeMethod(i)),
                ))
                .width(BOX);
                form = form.section("Hacim").field("Yöntem", method);
                form = if w.volume_sections {
                    // Docs/adr/4340: parallel sections across an axis; the average end area and prismoidal methods.
                    // Docs/adr/5140: the frustum's (A₁ + A₂ + √(A₁·A₂))·L/3 too.
                    use kentos_commands::SectionFormula as F;
                    let formulas = [F::EndArea, F::Prismoidal, F::Frustum];
                    let at = formulas.iter().position(|f| *f == w.formula);
                    let formula = container(Select::new(
                        ["Ortalama alan", "Prizmoidal", "Kesik koni"]
                            .into_iter()
                            .map(Choice::new),
                        at,
                        move |i| msg(Event::Formula(formulas[i])),
                    ))
                    .width(BOX);
                    // Docs/adr/4760: a drawn axis (a line or an open polyline of the drawing) or the azimuth's.
                    let mut ids: Vec<Option<String>> = vec![None];
                    let mut names = vec!["Semtle (paralel kesitler)".to_owned()];
                    if let Some(m) = model {
                        for (uid, e) in m.entities_with_uids() {
                            if ids.len() > 200 {
                                break;
                            }
                            let open = match e {
                                kentos_commands::Entity::Line(_) => true,
                                kentos_commands::Entity::Polyline(p) => {
                                    !p.closed && p.parts.as_ref().is_none_or(|q| q.is_empty())
                                }
                                _ => false,
                            };
                            if open {
                                let layer = layers
                                    .iter()
                                    .find(|l| l.0 == e.base().layer_id)
                                    .map_or("", |l| l.1.as_str());
                                names.push(format!(
                                    "{} · {} #{}",
                                    crate::view::kind_name(e.kind()),
                                    layer,
                                    e.base().id
                                ));
                                ids.push(Some(uid.to_string()));
                            }
                        }
                    }
                    let at = ids.iter().position(|a| *a == w.axis);
                    let axis = container(Select::new(
                        names.into_iter().map(Choice::new),
                        at,
                        move |i| msg(Event::Axis(ids[i].clone())),
                    ))
                    .width(BOX);
                    let form = form.field(FIELDS[17], input("10", 17)).field("Eksen", axis);
                    let form = if w.axis.is_some() {
                        form.field(FIELDS[19], input("25", 19))
                            .field(FIELDS[20], input("25", 20))
                            .row(words::check(
                                w.corners,
                                "Eksen noktalarında da",
                                Some(msg(Event::Corners(!w.corners))),
                            ))
                            .field(FIELDS[21], input("0", 21))
                    } else {
                        form.field(FIELDS[18], input("0", 18))
                    };
                    form.field("Formül", formula).help(if w.axis.is_some() {
                        "Kesitler eksene dik (kurpta yarıçap doğrultusunda); boy eksen boyunca. KGM 2019: arazi genellikle en az 50 m genişlikte."
                    } else {
                        "Kesitler eksene dik; semt kuzeyden saat yönünde (0: kesitler doğu–batı)."
                    })
                } else if w.volume_grid {
                    form.field("Hücre (m)", input("ör. 1", 13))
                } else {
                    form.field(FIELDS[2], input("ör. -1, 0, 1", 2))
                };
                form = form
                    .field(FIELDS[14], input("1", 14))
                    .field(FIELDS[15], input("1", 15));
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
                    .field(
                        "Katman",
                        pick_layer(w.checks.as_ref(), true, Box::new(|l| msg(Event::Checks(l)))),
                    )
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
                            let layer = layers
                                .iter()
                                .find(|l| l.0 == e.base().layer_id)
                                .map_or("", |l| l.1.as_str());
                            names.push(format!(
                                "{} · {} #{}",
                                crate::view::kind_name(e.kind()),
                                layer,
                                e.base().id
                            ));
                            ids.push(Some(uid.to_string()));
                        }
                    }
                }
                let at = ids.iter().position(|b| *b == w.boundary);
                let boundary = container(Select::new(
                    names.into_iter().map(Choice::new),
                    at,
                    move |i| msg(Event::Boundary(ids[i].clone())),
                ))
                .width(BOX);
                form = form.section("Sınır").field("Proje sınırı", boundary);
                form = if w.mode == Mode::Coverage {
                    form.field(FIELDS[8], input("yok", 8))
                } else {
                    form.section("Yoğunluk")
                        .field(FIELDS[9], input("ör. 2", 9))
                        .field(FIELDS[10], input("yok", 10))
                        .field(FIELDS[11], input("yok", 11))
                };
            }
        }
        // What the drawing gets: a layer and the writes' own fields; the writes themselves are the footer's buttons.
        if matches!(w.mode, Mode::Compare | Mode::Coverage | Mode::Density) {
            form = form.section("Çizime yaz").field(
                "Katman",
                pick_layer(
                    w.layer.as_ref(),
                    false,
                    Box::new(|l| msg(Event::Layer(l.unwrap_or_default()))),
                ),
            );
            if w.mode == Mode::Compare {
                form = form.field(FIELDS[16], input("boş: kazı, dolgu", 16)).field(
                    "Bölgelere",
                    Column::new()
                        .spacing(6)
                        .push(words::check(
                            w.zone_labels,
                            "Yazı (tür, hacim)",
                            Some(msg(Event::ZoneLabels(!w.zone_labels))),
                        ))
                        .push(words::check(
                            w.zone_table,
                            "Miktar tablosu",
                            Some(msg(Event::ZoneTable(!w.zone_table))),
                        ))
                        .push(words::check(
                            w.zone_depths,
                            "En derin yer",
                            Some(msg(Event::ZoneDepths(!w.zone_depths))),
                        ))
                        // Docs/adr/4760: the material zones (Hacim's layer, TIN) written in them, their table with the
                        // quantity table.
                        .push(words::check(
                            w.zone_materials,
                            "Malzemeler (ad, hacim)",
                            (w.materials.is_some() && !w.volume_grid && !w.volume_sections)
                                .then(|| msg(Event::ZoneMaterials(!w.zone_materials))),
                        )),
                );
            }
        }
        // The result: the chart and the content in their own tabs (the owner's words: “grafik ve içerik ayrı sekmelerde
        // olsun”), the notes under either.
        let tabs = row![
            kentos_ui::widget::Segmented::new([Tab::Chart, Tab::Content], w.tab, |t| msg(
                Event::Tab(t)
            )),
            space::horizontal()
        ];
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close))
            .width(1180.0)
            .over(tabs);
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
                        format!(
                            "en büyük {:+.3} m ({:.2}, {:.2})",
                            l.difference, l.at[0], l.at[1]
                        )
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
                d = d
                    .secondary(DRAW_ZERO, can.then(|| msg(Event::DrawZero)))
                    .primary(DRAW_ZONES, can.then(|| msg(Event::DrawZones)));
            }
            Mode::Coverage => {
                let can = w.layer.is_some() && !w.fields[8].trim().is_empty();
                d = d.primary(DRAW_LOW, can.then(|| msg(Event::DrawLow)));
            }
            Mode::Density => {
                let can = w.layer.is_some()
                    && !w.fields[9].trim().is_empty()
                    && !w.fields[10].trim().is_empty();
                d = d.primary(DRAW_SPARSE, can.then(|| msg(Event::DrawSparse)));
            }
            Mode::Quality => {
                let spikes =
                    matches!(&w.preview, Some(Ok(Read::Quality(q))) if !q.spikes.is_empty());
                let left = model
                    .and_then(|m| m.settings().road.as_ref())
                    .and_then(|r| {
                        r.surfaces
                            .iter()
                            .find(|s| Some(&s.id) == w.surface.as_ref())
                    })
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

    pub(crate) fn surface_analysis_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.terrain_work.surface_analysis_window.is_none() {
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

#[path = "surface_analysis_window/chart.rs"]
mod chart;
#[path = "surface_analysis_window/content.rs"]
mod content;
