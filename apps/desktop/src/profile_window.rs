//! Boykesit (plan/road.md Y4, `profile.editor`): a road project's vertical alignments, as OpenRoads' Profile Complex
//! by PI and Netcad's boykesit make them: the terrain (black) along an alignment from a surface, the design (red) line
//! through PVIs with a symmetric or asymmetric parabola or a true circle on each, drawn with its own vertical
//! exaggeration, and the bands under it (km, distance, black and red elevations, their difference, the grades). The
//! profile being edited is built as it is typed (`kentos.profile/1`), so its curves' K or why it does not build show
//! before it is saved.
//!
//! ```text
//! ┌ Boykesit ───────────────────────────────────────────────────────────────────────────────────┐
//! │ Boykesitler         │ Ad [Proje kotu]   Aks [Ana aks ▾]   Yüzey [Mevcut zemin ▾]  Abartma [10]│
//! │ ● Proje kotu        │ ┌───────────────────────────────────────────────────────────────────┐ │
//! │   Ana aks           │ │  ╱╲  black terrain, red design, PVIs ○, curves' ends ·, K values  │ │
//! │   Yan kot · eski    │ ├───────────────────────────────────────────────────────────────────┤ │
//! │                     │ │ Km · Mesafe · Siyah kot · Kırmızı kot · Fark · Eğim               │ │
//! │ [Yeni boykesit]     │ └───────────────────────────────────────────────────────────────────┘ │
//! │                     │ PVI   Km        Kot       Kurp        L / R     L çıkış               │
//! │                     │ [Satır ekle] [Satırı sil] [Araziden uçlar]                            │
//! │                     │ 5 PVI · 3 kurp · en küçük K 4,3                                       │
//! │ [Boykesiti sil] [Aksa yeniden bağla]                                     [Vazgeç] [Kaydet]  │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the definitions; Kaydet writes what changed through `road.profiles.edit` (removes, sets)
//! and `road.profiles.create` (the new ones), each one undo step, and closes. Aksa yeniden bağla rebinds a saved,
//! stale profile (`road.profiles.edit rebind`).

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Size, Task, Theme, mouse};
use kentos_commands::{ProfileDef, ProfilePvi, ProfilesCreate, ProfilesEdit, ProfilesEditOperation};
use kentos_interaction::Level;
use kentos_native_application::{
    ExecutionContext, road_alignments, road_profiles_create, road_profiles_edit, road_terrains,
};
use kentos_road::alignment::km_text;
use kentos_road::profile::{self as core, CurveReport, ProfilePoint, VCurve, Vertical, VerticalError};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::road_dialog;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Boykesit";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni boykesit";
const REMOVE: &str = "Boykesiti sil";
const REBIND: &str = "Aksa yeniden bağla";
const ADD: &str = "Satır ekle";
const REMOVE_PVI: &str = "Satırı sil";
const FROM_GROUND: &str = "Araziden uçlar";
const DRAW: &str = "Çizime yaz";
const LENGTHS: &str = "Kurp boyu öner…";
const CURVES: [&str; 4] = ["Yok", "Parabol", "Asimetrik", "Çember"];
/// The crossings' table and its buttons (docs/adr/0946).
pub(crate) const CROSSINGS: &str = "Kesişmeler";
const ADD_CROSSING: &str = "Kesişme ekle";
const REMOVE_CROSSING: &str = "Kesişmeyi sil";
/// A crossing's side: the structure over the road, or under it.
const SIDES: [&str; 2] = ["Üstte (köprü, üst geçit)", "Altta (menfez, boru)"];

/// A crossing as typed (docs/adr/0946): its name, station, side (an index of [`SIDES`]), elevation (underside or top)
/// and clearance or cover.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossingRow {
    name: String,
    station: String,
    side: usize,
    elevation: String,
    clearance: String,
}

/// A PVI as typed: its id, station and elevation texts, its curve's kind (an index of [`CURVES`]) and sizes' texts
/// (a parabola's length; an asymmetric one's lengths before and after; a circle's radius).
#[derive(Debug, Clone, PartialEq)]
pub struct PviRow {
    id: String,
    station: String,
    elevation: String,
    curve: usize,
    sizes: [String; 2],
}

/// A profile being edited.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    id: String,
    name: String,
    revision: u32,
    alignment: String,
    alignment_revision: u32,
    surface: Option<String>,
    pvis: Vec<PviRow>,
    crossings: Vec<CrossingRow>,
}

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

/// A number the user typed, with a comma or a point; empty: none.
fn number(text: &str) -> Result<Option<f64>, ()> {
    let text = text.trim().replace(',', ".");
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some).ok_or(())
}

impl Row {
    /// Its id, for the tests' pictures.
    #[cfg(test)]
    pub(crate) fn id_text(&self) -> String {
        self.id.clone()
    }

    fn of(d: &ProfileDef) -> Self {
        Self {
            id: d.id.clone(),
            name: d.name.clone(),
            revision: d.revision,
            alignment: d.alignment.clone(),
            alignment_revision: d.alignment_revision,
            surface: d.surface.clone(),
            pvis: d
                .pvis
                .iter()
                .map(|p| {
                    let (curve, sizes) = if p.radius > 0.0 {
                        (3, [trimmed(p.radius), String::new()])
                    } else if p.length > 0.0 {
                        (1, [trimmed(p.length), String::new()])
                    } else if p.length_in > 0.0 || p.length_out > 0.0 {
                        (2, [trimmed(p.length_in), trimmed(p.length_out)])
                    } else {
                        (0, [String::new(), String::new()])
                    };
                    PviRow {
                        id: p.id.clone(),
                        station: trimmed(p.station),
                        elevation: trimmed(p.elevation),
                        curve,
                        sizes,
                    }
                })
                .collect(),
            crossings: d
                .crossings
                .iter()
                .map(|c| CrossingRow {
                    name: c.name.clone(),
                    station: trimmed(c.station),
                    side: usize::from(c.side == kentos_commands::CrossingSide::Below),
                    elevation: trimmed(c.elevation),
                    clearance: trimmed(c.clearance),
                })
                .collect(),
        }
    }

    /// The definition the row says, or what is wrong with a number.
    fn def(&self) -> Result<ProfileDef, String> {
        let mut pvis = Vec::new();
        for (i, p) in self.pvis.iter().enumerate() {
            let need = |t: &str, what: &str| match number(t) {
                Ok(Some(v)) => Ok(v),
                _ => Err(format!("{}. satırın {what} değeri sayı olmalı.", i + 1)),
            };
            let station = need(&p.station, "km")?;
            let elevation = need(&p.elevation, "kot")?;
            let mut pvi = ProfilePvi {
                id: p.id.clone(),
                station,
                elevation,
                length: 0.0,
                length_in: 0.0,
                length_out: 0.0,
                radius: 0.0,
            };
            match p.curve {
                1 => pvi.length = need(&p.sizes[0], "kurp boyu")?,
                2 => {
                    pvi.length_in = need(&p.sizes[0], "giriş kolu")?;
                    pvi.length_out = need(&p.sizes[1], "çıkış kolu")?;
                }
                3 => pvi.radius = need(&p.sizes[0], "yarıçap")?,
                _ => {}
            }
            pvis.push(pvi);
        }
        let mut crossings = Vec::new();
        for (i, c) in self.crossings.iter().enumerate() {
            let need = |t: &str, what: &str| match number(t) {
                Ok(Some(v)) => Ok(v),
                _ => Err(format!("{}. kesişmenin {what} değeri sayı olmalı.", i + 1)),
            };
            crossings.push(kentos_commands::ProfileCrossingDef {
                name: c.name.trim().to_owned(),
                station: need(&c.station, "km")?,
                side: if c.side == 1 { kentos_commands::CrossingSide::Below } else { kentos_commands::CrossingSide::Above },
                elevation: need(&c.elevation, "kot")?,
                clearance: need(&c.clearance, "açıklık ya da örtü")?,
            });
        }
        Ok(ProfileDef {
            id: self.id.clone(),
            name: self.name.clone(),
            alignment: self.alignment.clone(),
            alignment_revision: self.alignment_revision,
            revision: self.revision,
            pvis,
            surface: self.surface.clone(),
            crossings,
        })
    }

    fn vertical(&self) -> Result<Vertical, String> {
        let def = self.def()?;
        core::vertical::build(&kentos_native_application::road_profiles::pvis_of(&def)).map_err(|e| vertical_words(&e))
    }
}

fn vertical_words(e: &VerticalError) -> String {
    use VerticalError as E;
    match e {
        E::TooFewPoints => "en az iki PVI gerekir".into(),
        E::DuplicateId { pvi } => format!("{pvi} kimliği iki kez var"),
        E::NotFinite { pvi } => format!("{pvi} PVI'sinin sayıları sonlu değil"),
        E::NotIncreasing { pvi } => format!("{pvi} PVI'sinin km'si bir öncekinden büyük olmalı"),
        E::BadCurve { pvi } => format!("{pvi} PVI'sinde kurp olamaz"),
        E::Overlap { a, b, excess } => format!("{a}–{b} arasına düşey kurplar sığmıyor, {} m taşıyor", comma(*excess, 3)),
    }
}

/// The terrain profile along an alignment on a surface, made when the window opens or the chosen profile's alignment
/// or surface changes: the alignment's length, its round kms, and the dense terrain points (linear between them within
/// 1 mm).
#[derive(Debug, Clone, Default)]
pub struct Ground {
    alignment: String,
    surface: Option<String>,
    pub(crate) length: f64,
    /// Round kms: station and km written.
    ticks: Vec<(f64, String)>,
    /// The km at the start, and whether the km grows with the station (no equations): then the chart marks its own
    /// kms, finer as it is zoomed in (docs/adr/0937 §3).
    km0: f64,
    plain: bool,
    points: Vec<ProfilePoint>,
    /// Why there is no terrain (no surface, it does not build).
    why: Option<String>,
}

impl Ground {
    /// The terrain's height at a station, linear between its points; none off the surface.
    fn at(&self, s: f64) -> Option<f64> {
        let i = self.points.partition_point(|p| p.station <= s);
        if i == 0 {
            return self.points.first().filter(|p| p.station == s).and_then(|p| p.z);
        }
        let a = &self.points[i - 1];
        if a.station == s || i == self.points.len() {
            return a.z.filter(|_| a.station == s);
        }
        let b = &self.points[i];
        if b.gap {
            return None;
        }
        let (za, zb) = (a.z?, b.z?);
        Some(za + (zb - za) * (s - a.station) / (b.station - a.station))
    }
}

/// A round step for the km ticks: about a dozen along the length.
fn tick_step(length: f64) -> f64 {
    [10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0]
        .into_iter()
        .find(|s| length / s <= 14.0)
        .unwrap_or(10_000.0)
}

pub(crate) fn ground_of(doc: &kentos_domain::Document, alignment: &str, surface: Option<&str>) -> Ground {
    let mut g = Ground {
        alignment: alignment.to_owned(),
        surface: surface.map(str::to_owned),
        ..Ground::default()
    };
    let Some(built) = road_alignments::made_alignments(doc)
        .into_iter()
        .find(|(d, _)| d.id == alignment)
        .and_then(|(_, b)| b.ok())
    else {
        g.why = Some("Aks kurulamıyor.".into());
        return g;
    };
    g.length = built.core.length;
    let step = tick_step(g.length);
    let (k0, k1) = (built.km(0.0), built.km(g.length));
    g.km0 = k0;
    g.plain = built.regions.len() <= 1 && (k1 - k0 - g.length).abs() < 1e-6;
    let mut k = (k0.min(k1) / step).ceil() * step;
    while k <= k0.max(k1) + 1e-9 {
        // Without equations the km grows from the start; with them the ticks are the stations' own.
        let s = k - k0;
        if (0.0..=g.length + 1e-9).contains(&s) && (built.km(s.min(g.length)) - k).abs() < 1e-6 {
            g.ticks.push((s.min(g.length), km_text(k, 0)));
        }
        k += step;
    }
    if g.ticks.is_empty() {
        g.ticks = vec![(0.0, km_text(k0, 0)), (g.length, km_text(k1, 0))];
    }
    let Some(id) = surface else {
        g.why = Some("Yüzey seçilmedi: siyah kot yok.".into());
        return g;
    };
    match road_terrains::made_surfaces(doc).into_iter().find(|(d, _, _)| d.id == id) {
        Some((_, _, Some(s))) => g.points = core::terrain(&s, &built.core),
        Some((d, _, None)) => g.why = Some(format!("“{}” yüzeyi kurulamıyor: siyah kot yok.", d.name)),
        None => g.why = Some("Yüzey projede yok: siyah kot yok.".into()),
    }
    g
}

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) rows: Vec<Row>,
    kept: Vec<ProfileDef>,
    chosen: Option<usize>,
    /// The PVI row chosen in the table.
    pvi: Option<usize>,
    /// The crossing row chosen (docs/adr/0946).
    crossing: Option<usize>,
    /// The vertical exaggeration's text.
    exaggeration: String,
    pub(crate) ground: Ground,
    /// Çizime yaz: the layer, and the frame's lower left as typed (“Y; X”).
    layer: Option<String>,
    place: String,
    /// Düşey kurp boyu over the window (profile_lengths_window.rs, docs/adr/0935 §3).
    pub(crate) lengths: Option<Box<crate::profile_lengths_window::Lengths>>,
    /// The values typed there, kept while the program runs when it is closed.
    pub(crate) lengths_kept: Option<([String; 11], bool)>,
    /// What the PVIs typed would make stale (docs/adr/0962): read while they differ from the saved ones.
    pub(crate) impact: Option<Result<(kentos_commands::ProfileImpact, Vec<kentos_commands::CommandWarning>), String>>,
}

impl Window {
    /// The chosen profile as typed; none when none is chosen.
    pub(crate) fn chosen_def(&self) -> Option<Result<ProfileDef, String>> {
        self.chosen.and_then(|i| self.rows.get(i)).map(Row::def)
    }

    /// Kısaları uzat: each named PVI of the chosen profile set to its suggested length `(pvi, length, A %)`; a kink
    /// and a parabola take a parabola that long, a circle the radius giving it, an asymmetric curve both arms in
    /// their ratio. Where the suggestions do not fit between two PVIs (one curve's arm out and the next one's in
    /// longer than the gap) the lengthened arms on that gap share what is left in their ratio, never under the
    /// length the curve had: the profile still builds and stays as near the suggestion as its PVIs allow. Lengths
    /// are rounded up at the millimetre, or down where cut to fit. The PVIs set, and those cut short of the
    /// suggestion.
    pub(crate) fn lengthen(&mut self, short: &[(String, f64, f64)]) -> (usize, Vec<String>) {
        let Some(r) = self.chosen.and_then(|i| self.rows.get_mut(i)) else { return (0, Vec::new()) };
        let Ok(def) = r.def() else { return (0, Vec::new()) };
        let n = def.pvis.len();
        // Each PVI's arms now (in, out), and those wanted.
        let grade = |a: usize, b: usize| (def.pvis[b].elevation - def.pvis[a].elevation) / (def.pvis[b].station - def.pvis[a].station);
        let arms_now: Vec<[f64; 2]> = (0..n)
            .map(|j| {
                let p = &def.pvis[j];
                if j == 0 || j + 1 == n {
                    [0.0, 0.0]
                } else if p.radius > 0.0 {
                    let t = p.radius * (100.0 * (grade(j, j + 1) - grade(j - 1, j))).abs() / 200.0;
                    [t, t]
                } else if p.length > 0.0 {
                    [p.length / 2.0, p.length / 2.0]
                } else {
                    [p.length_in, p.length_out]
                }
            })
            .collect();
        let mut wanted = arms_now.clone();
        let mut changed = vec![false; n];
        for (pvi, length, _) in short {
            let Some(j) = r.pvis.iter().position(|p| p.id.trim() == pvi).filter(|j| (1..n - 1).contains(j)) else { continue };
            let [a, b] = arms_now[j];
            let share = if r.pvis[j].curve == 2 && a + b > 0.0 { a / (a + b) } else { 0.5 };
            wanted[j] = [length * share, length * (1.0 - share)];
            changed[j] = true;
        }
        // Each lengthened PVI's factor: the least over its two gaps of what that gap leaves to its lengthened arms.
        let mut factor = vec![1.0f64; n];
        for j in 0..n.saturating_sub(1) {
            let gap = def.pvis[j + 1].station - def.pvis[j].station;
            let (mut fixed, mut grown) = (0.0, 0.0);
            for (k, arm) in [(j, wanted[j][1]), (j + 1, wanted[j + 1][0])] {
                if changed[k] { grown += arm } else { fixed += arm }
            }
            if grown > 0.0 && fixed + grown > gap {
                let f = ((gap - fixed) / grown).max(0.0);
                for k in [j, j + 1] {
                    if changed[k] {
                        factor[k] = factor[k].min(f);
                    }
                }
            }
        }
        let up = |v: f64| (v * 1000.0 - 1e-6).ceil() / 1000.0;
        let down = |v: f64| (v * 1000.0 + 1e-6).floor() / 1000.0;
        let (mut set, mut cut) = (0, Vec::new());
        for (pvi, length, a) in short {
            let Some(j) = r.pvis.iter().position(|p| p.id.trim() == pvi).filter(|j| (1..n - 1).contains(j)) else { continue };
            let f = factor[j];
            let now = arms_now[j][0] + arms_now[j][1];
            let fits = f >= 1.0;
            let total = if fits { up(*length) } else { down((length * f).max(now)) };
            if !fits {
                cut.push(pvi.clone());
            }
            let p = &mut r.pvis[j];
            match p.curve {
                3 if a.abs() > 0.0 => {
                    let radius = 100.0 * total / a.abs();
                    p.sizes = [trimmed(if fits { up(radius) } else { down(radius) }), String::new()];
                }
                2 => {
                    let share = wanted[j][0] / (wanted[j][0] + wanted[j][1]);
                    let first = down(total * share);
                    p.sizes = [trimmed(first), trimmed(if fits { up(total - first) } else { down(total - first) })];
                }
                _ => {
                    p.curve = 1;
                    p.sizes = [trimmed(total), String::new()];
                }
            }
            set += 1;
        }
        (set, cut)
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Remove,
    Rebind,
    Name(String),
    Alignment(String),
    Surface(Option<String>),
    Exaggeration(String),
    PviChoose(usize),
    /// A PVI's field: 0 id, 1 station, 2 elevation, 3 and 4 its curve's sizes.
    Pvi(usize, usize, String),
    Curve(usize, usize),
    Add,
    RemovePvi,
    FromGround,
    DrawLayer(String),
    Place(String),
    Draw,
    Save,
    Cancel,
    /// Kurp boyu öner…: the curve length window on the chosen profile.
    OpenLengths,
    Lengths(crate::profile_lengths_window::Event),
    /// A crossing's name (0), station (1), elevation (3) or clearance (4) (docs/adr/0946).
    Crossing(usize, usize, String),
    CrossingSide(usize, usize),
    CrossingChoose(usize),
    AddCrossing,
    RemoveCrossing,
}

fn msg(event: Event) -> Message {
    Message::Profiles(event)
}

/// The impact in words (docs/adr/0962): the stretches that change and by how much at most, and what stands on them.
fn impact_words(imp: &kentos_commands::ProfileImpact) -> String {
    if imp.clear {
        return "Kırmızı kot kaydedilenden 1 mm'den çok ayrılmıyor: hiçbir şey eskimez.".into();
    }
    let stretches: Vec<String> = imp
        .changes
        .iter()
        .map(|c| match c.kind {
            kentos_commands::ProfileChangeKind::Changed => {
                format!("km {} – {} (en çok {} m)", c.from_text, c.to_text, comma(c.most.unwrap_or(0.0), 3))
            }
            kentos_commands::ProfileChangeKind::Added => format!("km {} – {} eklenir", c.from_text, c.to_text),
            kentos_commands::ProfileChangeKind::Removed => format!("km {} – {} kalkar", c.from_text, c.to_text),
        })
        .collect();
    let mut said = format!("Kaydedilirse kırmızı kot değişir: {}.", stretches.join("; "));
    let mut stale: Vec<String> = Vec::new();
    for c in &imp.corridors {
        let mut regions: Vec<u32> = c.regions.iter().map(|r| r.region + 1).collect();
        regions.dedup();
        let regions: Vec<String> = regions.iter().map(|r| format!("{r}.")).collect();
        stale.push(format!("{} ({} bölge)", c.name, regions.join(", ")));
    }
    stale.extend(imp.sections.iter().map(|s| format!("{} ({}/{} kesit)", s.name, s.stations, s.of)));
    let with = |d: &kentos_commands::ImpactDerived| if d.approved { format!("{} (onaylı)", d.name) } else { d.name.clone() };
    stale.extend(imp.volumes.iter().map(with));
    stale.extend(imp.hauls.iter().map(with));
    if !stale.is_empty() {
        said.push_str(&format!(" Eskiyecekler: {}.", stale.join(", ")));
    }
    said
}

/// What the row builds to now, or why it does not build.
fn status(r: &Row, stale: bool, ground: &Ground) -> (String, bool) {
    let v = match r.vertical() {
        Ok(v) => v,
        Err(e) => return (format!("Kurulamıyor: {e}"), true),
    };
    if let (Some(first), Some(last)) = (r.def().ok().and_then(|d| d.pvis.first().map(|p| p.station)), r.def().ok().and_then(|d| d.pvis.last().map(|p| p.station)))
        && ground.length > 0.0
        && (first < -1e-9 || last > ground.length + 1e-9)
    {
        return (format!("PVI aksın dışında: aksın km'si 0 ile {} m arası.", comma(ground.length, 3)), true);
    }
    let mut words = format!("{} PVI · {} kurp", r.pvis.len(), v.curves.len());
    // K in its unit and the radius H beside it (docs/adr/0961): a standard writes the one or the other.
    if let Some(k) = v.curves.iter().filter_map(|c| c.k).min_by(f64::total_cmp) {
        words.push_str(&format!(" · en küçük K {} m/%", comma(k, 1)));
    }
    if let Some(h) = v.curves.iter().filter_map(|c| c.h).min_by(f64::total_cmp) {
        words.push_str(&format!(" · en küçük yarıçap {} m", comma(h, 0)));
    }
    if stale {
        words.push_str(" · aks değişti: PVI'ları denetleyip aksa yeniden bağlayın");
    }
    (words, false)
}

impl App {
    /// Boykesit: the window on the road project's profiles.
    pub(crate) fn open_profiles(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Boykesit yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let kept = doc.model.road_profiles().to_vec();
        // The drawing goes under the alignments, where nothing is drawn yet.
        let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
        for p in doc.model.road_alignments().iter().flat_map(|a| a.pis.iter()) {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
        }
        let place = if x0.is_finite() { format!("{}; {}", trimmed(x0), trimmed(y0 - 300.0)) } else { "0; 0".into() };
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        self.profile_window = Some(Window {
            chosen: (!kept.is_empty()).then_some(0),
            rows: kept.iter().map(Row::of).collect(),
            kept,
            exaggeration: "10".into(),
            layer,
            place,
            ..Window::default()
        });
        self.dialog = Some(Dialog::Profiles);
        self.refresh_ground();
    }

    /// Boykesit oluştur from the ribbon: the window with a new profile on the first alignment.
    pub(crate) fn new_profile(&mut self) {
        self.open_profiles();
        if self.profile_window.is_some() {
            let _ = self.profile_event(Event::New);
        }
    }

    /// The terrain of the chosen profile's alignment and surface, made again when either changed.
    fn refresh_ground(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.profile_window.as_mut() else { return };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            w.ground = Ground::default();
            return;
        };
        if w.ground.alignment == r.alignment && w.ground.surface == r.surface && w.ground.length > 0.0 {
            return;
        }
        w.ground = ground_of(&doc.model, &r.alignment, r.surface.as_deref());
    }

    /// The alignments a profile can lie on: id and name.
    fn profile_alignments(&self) -> Vec<(String, String)> {
        self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model
                .road_alignments()
                .iter()
                .filter(|a| a.offset_of.is_none())
                .map(|a| (a.id.clone(), a.name.clone()))
                .collect()
        })
    }

    fn profile_surfaces(&self) -> Vec<(String, String)> {
        self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model
                .settings()
                .road
                .as_ref()
                .map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect())
        })
    }

    pub(crate) fn profile_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::OpenLengths => {
                self.open_profile_lengths();
                return Task::none();
            }
            Event::Lengths(e) => {
                self.profile_lengths_event(e);
                return Task::none();
            }
            _ => {}
        }
        let alignments = self.profile_alignments();
        let surfaces = self.profile_surfaces();
        let revisions: Vec<(String, u32)> = self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model.road_alignments().iter().map(|a| (a.id.clone(), a.revision)).collect()
        });
        let Some(w) = self.profile_window.as_mut() else {
            return Task::none();
        };
        let mut warning = None;
        let mut regrounded = false;
        match event {
            Event::Choose(i) => {
                w.chosen = Some(i);
                w.pvi = None;
                regrounded = true;
            }
            Event::New => match alignments.first() {
                None => warning = Some("Boykesit bir aksın üstünde kurulur: önce Güzergâh'ta bir aks oluşturun."),
                Some((alignment, _)) => {
                    let taken: Vec<String> = w.rows.iter().map(|r| r.id.clone()).collect();
                    let id = (1..).map(|n| format!("b-{n}")).find(|id| !taken.contains(id)).unwrap_or_default();
                    let n = w.rows.len() + 1;
                    let blank = |id: &str| PviRow {
                        id: id.to_owned(),
                        station: String::new(),
                        elevation: String::new(),
                        curve: 0,
                        sizes: Default::default(),
                    };
                    w.rows.push(Row {
                        id,
                        name: if n == 1 { "Proje kotu".to_owned() } else { format!("Boykesit {n}") },
                        revision: 0,
                        alignment: alignment.clone(),
                        alignment_revision: revisions.iter().find(|(a, _)| a == alignment).map_or(0, |(_, r)| *r),
                        surface: surfaces.first().map(|(id, _)| id.clone()),
                        pvis: vec![blank("a"), blank("b")],
                        crossings: Vec::new(),
                    });
                    w.chosen = Some(w.rows.len() - 1);
                    w.pvi = None;
                    regrounded = true;
                }
            },
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|i| *i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen = (!w.rows.is_empty()).then_some(i.min(w.rows.len().saturating_sub(1)));
                    regrounded = true;
                }
            }
            Event::Rebind => return self.rebind_profile(),
            Event::Name(t) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.name = t;
                }
            }
            Event::Alignment(id) => {
                // Only a profile not yet saved moves to another alignment.
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    if w.kept.iter().any(|k| k.id == r.id) {
                        warning = Some("Kaydedilmiş boykesitin aksı değişmez; yeni bir boykesit oluşturun.");
                    } else {
                        r.alignment_revision = revisions.iter().find(|(a, _)| *a == id).map_or(0, |(_, v)| *v);
                        r.alignment = id;
                        regrounded = true;
                    }
                }
            }
            Event::Surface(id) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.surface = id;
                    regrounded = true;
                }
            }
            Event::Exaggeration(t) => w.exaggeration = t,
            Event::Crossing(j, k, t) => {
                if let Some(c) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.crossings.get_mut(j)) {
                    match k {
                        0 => c.name = t,
                        1 => c.station = t,
                        3 => c.elevation = t,
                        _ => c.clearance = t,
                    }
                }
                w.crossing = Some(j);
            }
            Event::CrossingSide(j, side) => {
                if let Some(c) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.crossings.get_mut(j)) {
                    c.side = side;
                }
                w.crossing = Some(j);
            }
            Event::CrossingChoose(j) => w.crossing = Some(j),
            Event::AddCrossing => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    // A new crossing's name follows the count; its station, elevation and clearance are the user's.
                    let n = r.crossings.len() + 1;
                    r.crossings.push(CrossingRow {
                        name: format!("Kesişme {n}"),
                        station: String::new(),
                        side: 0,
                        elevation: String::new(),
                        clearance: String::new(),
                    });
                    w.crossing = Some(n - 1);
                }
            }
            Event::RemoveCrossing => {
                if let Some(j) = w.crossing.take()
                    && let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)).filter(|r| j < r.crossings.len())
                {
                    r.crossings.remove(j);
                }
            }
            Event::PviChoose(j) => w.pvi = Some(j),
            Event::Pvi(j, k, t) => {
                if let Some(p) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.pvis.get_mut(j)) {
                    match k {
                        0 => p.id = t,
                        1 => p.station = t,
                        2 => p.elevation = t,
                        3 => p.sizes[0] = t,
                        _ => p.sizes[1] = t,
                    }
                }
                w.pvi = Some(j);
            }
            Event::Curve(j, c) => {
                if let Some(p) = w.chosen.and_then(|i| w.rows.get_mut(i)).and_then(|r| r.pvis.get_mut(j))
                    && p.curve != c
                {
                    p.curve = c;
                    p.sizes = Default::default();
                }
                w.pvi = Some(j);
            }
            Event::Add => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    let taken: Vec<String> = r.pvis.iter().map(|p| p.id.clone()).collect();
                    let id = (1..).map(|n| format!("v{n}")).find(|id| !taken.contains(id)).unwrap_or_default();
                    let at = w.pvi.map_or(r.pvis.len().saturating_sub(1), |j| (j + 1).min(r.pvis.len().saturating_sub(1))).max(1);
                    // Halfway along the grade it falls on.
                    let (a, b) = (&r.pvis[at - 1], &r.pvis[at.min(r.pvis.len() - 1)]);
                    let mid = |x: &str, y: &str| match (number(x), number(y)) {
                        (Ok(Some(x)), Ok(Some(y))) => trimmed(0.5 * (x + y)),
                        _ => String::new(),
                    };
                    let row = PviRow {
                        id,
                        station: mid(&a.station, &b.station),
                        elevation: mid(&a.elevation, &b.elevation),
                        curve: 0,
                        sizes: Default::default(),
                    };
                    r.pvis.insert(at.min(r.pvis.len()), row);
                    w.pvi = Some(at);
                }
            }
            Event::RemovePvi => {
                if let (Some(r), Some(j)) = (w.chosen.and_then(|i| w.rows.get_mut(i)), w.pvi)
                    && j < r.pvis.len()
                {
                    r.pvis.remove(j);
                    w.pvi = None;
                }
            }
            Event::FromGround => {
                let g = &w.ground;
                match (w.chosen.and_then(|i| w.rows.get_mut(i)), g.length > 0.0) {
                    (Some(r), true) if r.pvis.len() >= 2 => {
                        // The first and the last PVI at the alignment's ends, at the terrain's height there or, off
                        // the surface, at its nearest height along the alignment.
                        let ends = [0.0, g.length];
                        let n = r.pvis.len();
                        let first = g.points.iter().find_map(|p| p.z);
                        let last = g.points.iter().rev().find_map(|p| p.z);
                        for (k, j) in [0, n - 1].into_iter().enumerate() {
                            r.pvis[j].station = trimmed(ends[k]);
                            if let Some(z) = g.at(ends[k]).or(if k == 0 { first } else { last }) {
                                r.pvis[j].elevation = trimmed((z * 100.0).round() / 100.0);
                            }
                        }
                    }
                    _ => warning = Some("Uçlar aksın uçlarına konur: önce bir aks seçin."),
                }
            }
            Event::DrawLayer(id) => w.layer = Some(id),
            Event::Place(t) => w.place = t,
            Event::Draw => return self.draw_profile(),
            Event::Cancel => {
                self.profile_window = None;
                self.dialog = None;
            }
            Event::Save => return self.save_profiles(),
            // Answered before the window is borrowed.
            Event::OpenLengths | Event::Lengths(_) => return Task::none(),
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        if regrounded {
            self.refresh_ground();
        }
        self.refresh_profile_impact();
        Task::none()
    }

    /// The impact of the chosen profile's PVIs as typed against the saved ones (`road.profiles.impact`, docs/adr/0962):
    /// none for a new profile, for PVIs that do not read or are the saved ones.
    fn refresh_profile_impact(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.profile_window.as_mut() else { return };
        w.impact = w.chosen.and_then(|i| w.rows.get(i)).and_then(|r| r.def().ok()).and_then(|d| {
            let kept = w.kept.iter().find(|k| k.id == d.id)?;
            (kept.pvis != d.pvis).then(|| {
                let input = kentos_commands::ProfilesImpact { profile: d.id.clone(), pvis: d.pvis.clone(), tolerance: None };
                kentos_native_application::road_profiles_impact::preview(&doc.model, &input)
            })
        });
    }

    /// Aksa yeniden bağla: the chosen saved profile takes its alignment's present revision (`road.profiles.edit
    /// rebind`), one undo step.
    fn rebind_profile(&mut self) -> Task<Message> {
        let Some(w) = &self.profile_window else {
            return Task::none();
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return Task::none();
        };
        if !w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)) {
            self.warn("Önce boykesiti kaydedin; kaydedilmiş boykesit yeniden bağlanır.".to_owned());
            return Task::none();
        }
        let edit = ProfilesEdit {
            profile: r.id.clone(),
            operation: ProfilesEditOperation::Rebind,
            name: None,
            pvis: None,
            surface: None,
            crossings: None,
            expected_revision: None,
        };
        if self.profile_write(|cx| road_profiles_edit::execute(cx, edit)) {
            let Some(doc) = &self.document else { return Task::none() };
            let kept = doc.model.road_profiles().to_vec();
            if let Some(w) = self.profile_window.as_mut() {
                w.rows = kept.iter().map(Row::of).collect();
                w.kept = kept;
                w.chosen = w.chosen.filter(|i| *i < w.rows.len());
            }
            self.say(Level::Success, "Boykesit aksın bugünkü hâline bağlandı.");
        }
        Task::none()
    }

    /// Çizime yaz: the chosen saved profile drawn on the chosen layer at the typed place (`road.profiles.draw`), one
    /// undo step.
    fn draw_profile(&mut self) -> Task<Message> {
        let Some(w) = &self.profile_window else {
            return Task::none();
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return Task::none();
        };
        if !w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)) {
            self.warn("Önce boykesiti kaydedin; kaydedilmiş boykesit çizilir.".to_owned());
            return Task::none();
        }
        let parts: Vec<Option<f64>> =
            w.place.split([';', ' ']).filter(|s| !s.is_empty()).map(|s| number(s).ok().flatten()).collect();
        let [Some(x), Some(y)] = parts[..] else {
            self.warn("Yeri “Y; X” olarak yazın.".to_owned());
            return Task::none();
        };
        let input = kentos_commands::ProfilesDraw {
            profile: r.id.clone(),
            layer_id: w.layer.clone().unwrap_or_default(),
            at: [x, y],
            every: None,
            datum: None,
            exaggeration: number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0),
            height: None,
            from: None,
            to: None,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result =
            kentos_native_application::road_profiles_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(
                Level::Success,
                format!("Boykesit çizildi: {} nesne, kıyas kotu {}.", out.uids.len(), comma(out.datum, 2)),
            );
        }
        Task::none()
    }

    /// Kaydet: what changed, the removed first, then the changed, then the new.
    fn save_profiles(&mut self) -> Task<Message> {
        let Some(w) = self.profile_window.clone() else {
            return Task::none();
        };
        let mut defs = Vec::new();
        for r in &w.rows {
            match r.def() {
                Ok(d) => defs.push(d),
                Err(e) => {
                    self.warn(format!("“{}”: {e}", r.name));
                    return Task::none();
                }
            }
        }
        for k in w.kept.iter().filter(|k| defs.iter().all(|d| d.id != k.id)) {
            let edit = ProfilesEdit {
                profile: k.id.clone(),
                operation: ProfilesEditOperation::Remove,
                name: None,
                pvis: None,
                surface: None,
                crossings: None,
                expected_revision: None,
            };
            if !self.profile_write(|cx| road_profiles_edit::execute(cx, edit)) {
                return Task::none();
            }
        }
        for d in &defs {
            match w.kept.iter().find(|k| k.id == d.id) {
                Some(k) if k == d => {}
                Some(k) => {
                    let edit = ProfilesEdit {
                        profile: d.id.clone(),
                        operation: ProfilesEditOperation::Set,
                        name: (k.name != d.name).then(|| d.name.clone()),
                        pvis: (k.pvis != d.pvis).then(|| d.pvis.clone()),
                        surface: (k.surface != d.surface).then(|| d.surface.clone().unwrap_or_default()),
                        crossings: (k.crossings != d.crossings).then(|| d.crossings.clone()),
                        expected_revision: None,
                    };
                    if !self.profile_write(|cx| road_profiles_edit::execute(cx, edit)) {
                        return Task::none();
                    }
                }
                None => {
                    let create = ProfilesCreate {
                        name: d.name.clone(),
                        alignment: d.alignment.clone(),
                        pvis: d.pvis.clone(),
                        id: Some(d.id.clone()),
                        surface: d.surface.clone(),
                        expected_revision: None,
                    };
                    if !self.profile_write(|cx| road_profiles_create::execute(cx, create)) {
                        return Task::none();
                    }
                }
            }
        }
        self.profile_window = None;
        self.dialog = None;
        self.say(Level::Success, "Boykesitler kaydedildi.");
        self.refresh_profile_impact();
        Task::none()
    }

    /// One write; on a refusal the window stays.
    fn profile_write<T: serde::Serialize>(
        &mut self,
        write: impl FnOnce(&mut ExecutionContext<'_>) -> kentos_commands::CommandResult<T>,
    ) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = write(&mut ExecutionContext::new(&mut doc.model));
        self.answered(result).is_some()
    }

    pub(crate) fn road_profile_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.profile_window else {
            return iced::widget::text("").into();
        };
        if let Some(x) = &w.lengths {
            return self.profile_lengths_view(x);
        }
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let alignments = self.profile_alignments();
        let revisions: Vec<(String, u32)> = self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model.road_alignments().iter().map(|a| (a.id.clone(), a.revision)).collect()
        });
        let stale_of = |r: &Row| {
            w.kept.iter().any(|k| k.id == r.id)
                && revisions.iter().find(|(a, _)| *a == r.alignment).is_some_and(|(_, v)| *v != r.alignment_revision)
        };
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let on = alignments.iter().find(|(id, _)| *id == r.alignment).map_or(r.alignment.clone(), |(_, n)| n.clone());
            let kind = if stale_of(r) { format!("{on} · eski") } else { on };
            let face = column![
                Elided::new(r.name.clone()).size(typography::body()).font(typography::ui()).width(Fill),
                label::caption(kind).style(muted),
            ]
            .spacing(1);
            list = list.push(
                iced::widget::button(face)
                    .on_press(msg(Event::Choose(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(w.chosen == Some(i))),
            );
        }
        if w.rows.is_empty() {
            list = list.push(
                label::caption("Projede boykesit yok. Yeni boykesit ile bir aksın üstünde PVI'lerden bir düşey güzergâh kurun.")
                    .style(muted),
            );
        }
        let left = column![
            label::strong("Boykesitler"),
            container(scrollable(list).height(typography::scaled(330.0))).style(style::container::field_box).width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let chosen = w.chosen.and_then(|i| w.rows.get(i));
        let right: Element<'_, Message> = match chosen {
            None => container(label::caption("Bir boykesit seçin ya da yeni bir boykesit oluşturun.")).padding(12).into(),
            Some(r) => self.road_profile_editor(w, r, stale_of(r)),
        };
        let saved = chosen.is_some_and(|r| w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)));
        // The editor scrolls with the body (resized or full screen it grows); the drawing row stays above the footer.
        let body = row![container(left).width(220), container(right).width(Fill)].spacing(16);
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer = row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                words::secondary(REBIND, (saved && chosen.is_some_and(stale_of)).then(|| msg(Event::Rebind))),
                words::ghost(LENGTHS, chosen.is_some_and(|r| r.def().is_ok()).then(|| msg(Event::OpenLengths))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, Some(msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center);
        Frame::new(TITLE).scroll(body).push(self.profile_draw_row(w, saved)).footer(footer).width(1080.0).modal(msg(Event::Cancel))
    }

    fn profile_draw_row<'a>(&'a self, w: &'a Window, saved: bool) -> Element<'a, Message> {
        let ids: Vec<String> = self.relation_layers().into_iter().map(|(id, _)| id).collect();
        let names: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let layer = Select::new(
            names.iter().map(|n| Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| ids.iter().position(|id| id == l)),
            move |i| msg(Event::DrawLayer(ids[i].clone())),
        )
        .placeholder("Katman");
        row![
            label::strong("Çizime"),
            words::field("Katman", layer, None),
            words::field("Yer (Y; X)", kentos_ui::widget::text_field("Y; X", &w.place).on_input(|t| msg(Event::Place(t))).width(220), None),
            words::secondary(DRAW, saved.then(|| msg(Event::Draw))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom)
        .into()
    }

    fn road_profile_editor<'a>(&'a self, w: &'a Window, r: &'a Row, stale: bool) -> Element<'a, Message> {
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let alignments = self.profile_alignments();
        let surfaces = self.profile_surfaces();
        let name = kentos_ui::widget::text_field("Boykesitin adı", &r.name).on_input(|t| msg(Event::Name(t)));
        let ids: Vec<String> = alignments.iter().map(|(id, _)| id.clone()).collect();
        let on_alignment = Select::new(
            alignments.iter().map(|(_, n)| Choice::new(n.clone())),
            ids.iter().position(|id| *id == r.alignment),
            move |i| msg(Event::Alignment(ids[i].clone())),
        )
        .placeholder("Aks");
        let sids: Vec<String> = surfaces.iter().map(|(id, _)| id.clone()).collect();
        let on_surface = Select::new(
            std::iter::once(Choice::new("Yok".to_owned())).chain(surfaces.iter().map(|(_, n)| Choice::new(n.clone()))),
            Some(r.surface.as_ref().and_then(|s| sids.iter().position(|id| id == s)).map_or(0, |i| i + 1)),
            move |i| msg(Event::Surface(if i == 0 { None } else { Some(sids[i - 1].clone()) })),
        );
        let ex = kentos_ui::widget::text_field("10", &w.exaggeration).on_input(|t| msg(Event::Exaggeration(t)));
        // The chart takes what a resized or full screen window has over (docs/adr/0937 §2).
        let mut col = kentos_ui::widget::grow::Grows::new().spacing(10.0);
        col = col.push(
            row![
                words::field("Ad", name, None),
                words::field("Aks", on_alignment, None),
                words::field("Yüzey", on_surface, None),
                words::field("Düşey abartma", ex.width(70), None),
            ]
            .spacing(12),
        );
        let vertical = r.vertical().ok();
        let exaggeration = number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0).unwrap_or(10.0);
        let pvis: Vec<(f64, f64)> = r.def().ok().map_or_else(Vec::new, |d| d.pvis.iter().map(|p| (p.station, p.elevation)).collect());
        let chart = Chart {
            ground: &w.ground,
            vertical,
            pvis,
            names: r.pvis.iter().map(|p| p.id.clone()).collect(),
            chosen: w.pvi,
            exaggeration,
            cursor: None,
            on_station: None,
            compact: false,
            crossings: r.def().ok().map_or_else(Vec::new, |d| chart_crossings(&d)),
            changed: match &w.impact {
                Some(Ok((imp, _))) => imp.changes.iter().map(|c| (c.from, c.to)).collect(),
                _ => Vec::new(),
            },
        };
        col = col.push_grow(
            crate::picture::picture(canvas(chart).width(Fill).height(Fill), format!("Boykesit — {}", r.name)),
            typography::scaled(CHART_HEIGHT),
        );
        // What it builds to, under the chart where it is seen at once.
        let (said, bad) = status(r, stale, &w.ground);
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        col = col.push(if bad {
            label::caption(said).style(danger)
        } else if stale {
            label::caption(said).style(warn)
        } else {
            label::caption(said)
        });
        if let Some(why) = &w.ground.why {
            col = col.push(label::caption(why.clone()).style(muted));
        }
        // What saving the PVIs typed would make stale (docs/adr/0962).
        if let Some(Ok((imp, warnings))) = &w.impact {
            let said = impact_words(imp);
            col = col.push(if !warnings.is_empty() { label::caption(said).style(warn) } else { label::caption(said).style(muted) });
            for x in warnings {
                col = col.push(label::caption(x.message.clone()).style(warn));
            }
        }
        let pvis = &r.pvis;
        let table = road_dialog::edit_table(
            vec![
                road_dialog::col("PVI", 90.0),
                road_dialog::Col { title: "Km (m)", width: None, number: true },
                road_dialog::Col { title: "Kot (m)", width: None, number: true },
                road_dialog::col("Kurp", 140.0),
                road_dialog::Col { title: "L / R (m)", width: None, number: true },
                road_dialog::Col { title: "L çıkış (m)", width: None, number: true },
            ],
            pvis.len(),
            move |j, k| {
                let p = &pvis[j];
                let end = j == 0 || j + 1 == pvis.len();
                let blank = || kentos_ui::widget::table::text("—").into();
                match k {
                    0 => road_dialog::edit(&p.id, false, move |t| msg(Event::Pvi(j, 0, t))),
                    1 => road_dialog::edit(&p.station, true, move |t| msg(Event::Pvi(j, 1, t))),
                    2 => road_dialog::edit(&p.elevation, true, move |t| msg(Event::Pvi(j, 2, t))),
                    3 if end => blank(),
                    3 => road_dialog::pick(CURVES, Some(p.curve), move |c| msg(Event::Curve(j, c))),
                    _ => {
                        let size = k - 4;
                        // L for a parabola and a circle's R, the asymmetric parabola's two lengths.
                        if !end && matches!((p.curve, size), (1 | 3, 0) | (2, _)) {
                            road_dialog::edit(&p.sizes[size], true, move |t| msg(Event::Pvi(j, 3 + size, t)))
                        } else {
                            blank()
                        }
                    }
                }
            },
            w.pvi,
            |j| msg(Event::PviChoose(j)),
            "PVI yok",
        );
        col = col.push(words::field("PVI'ler", table, None)).push(
            row![
                words::secondary(ADD, Some(msg(Event::Add))),
                words::ghost(REMOVE_PVI, w.pvi.filter(|j| *j < r.pvis.len() && r.pvis.len() > 2).map(|_| msg(Event::RemovePvi))),
                words::secondary(FROM_GROUND, Some(msg(Event::FromGround))),
            ]
            .push(if r.crossings.is_empty() {
                Element::from(words::secondary(ADD_CROSSING, Some(msg(Event::AddCrossing))))
            } else {
                Element::from(space::horizontal().width(0))
            })
            .spacing(8),
        );
        // What the road crosses (docs/adr/0946): each with its margin on the line as typed. With none, only the button
        // above: an empty table would take the picture's room for nothing.
        if r.crossings.is_empty() {
            return container(col).padding(iced::Padding::default().right(14)).into();
        }
        let line = r.vertical().ok();
        let crossings = &r.crossings;
        let margins: Vec<Option<f64>> = r.def().ok().map_or_else(Vec::new, |d| {
            chart_crossings(&d)
                .iter()
                .map(|(_, s, above, _, limit)| {
                    let z = line.as_ref().and_then(|v| v.at(*s)).map(|q| q.0)?;
                    Some(if *above { limit - z } else { z - limit })
                })
                .collect()
        });
        let table = road_dialog::edit_table(
            vec![
                road_dialog::Col { title: "Ad", width: None, number: false },
                road_dialog::col("Km (m)", 90.0),
                road_dialog::col("Yer", 190.0),
                road_dialog::col("Kot (m)", 90.0),
                road_dialog::col("Açıklık/örtü (m)", 120.0),
                road_dialog::col("Pay (m)", 80.0),
            ],
            crossings.len(),
            move |j, k| {
                let c = &crossings[j];
                match k {
                    0 => road_dialog::edit(&c.name, false, move |t| msg(Event::Crossing(j, 0, t))),
                    1 => road_dialog::edit(&c.station, true, move |t| msg(Event::Crossing(j, 1, t))),
                    2 => road_dialog::pick(SIDES, Some(c.side), move |i| msg(Event::CrossingSide(j, i))),
                    3 => road_dialog::edit(&c.elevation, true, move |t| msg(Event::Crossing(j, 3, t))),
                    4 => road_dialog::edit(&c.clearance, true, move |t| msg(Event::Crossing(j, 4, t))),
                    _ => kentos_ui::widget::table::text(margins.get(j).copied().flatten().map_or_else(
                        || "—".to_owned(),
                        |m| format!("{:+.3}", if m.abs() < 5e-4 { 0.0 } else { m }).replace('.', ","),
                    ))
                    .into(),
                }
            },
            w.crossing,
            |j| msg(Event::CrossingChoose(j)),
            "Kesişme yok",
        );
        col = col.push(words::field(CROSSINGS, table, None)).push(
            row![
                words::secondary(ADD_CROSSING, Some(msg(Event::AddCrossing))),
                words::ghost(REMOVE_CROSSING, w.crossing.filter(|j| *j < r.crossings.len()).map(|_| msg(Event::RemoveCrossing))),
                label::caption("Üstteki yapının kotu alt yüzüdür, kırmızı kot ondan açıklık kadar aşağıda kalır; alttakinin kotu üstüdür, kırmızı kot ondan örtü kadar yukarıda. Değerler projenin. Eksi pay yetmeyen açıklıktır.").style(muted),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        );
        container(col).padding(iced::Padding::default().right(14)).into()
    }

    pub(crate) fn profile_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.profile_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        if w.lengths.is_some() {
            use crate::profile_lengths_window::{APPLY, BACK, Event as X, FIELDS as LF, TITLE as LT};
            let x = |e: X| msg(Event::Lengths(e));
            return Ok(match control {
                Control::Press(APPLY) => Some(x(X::Apply)),
                Control::Press(BACK) => Some(x(X::Back)),
                Control::Fill(name, text) => match LF.iter().position(|f| *f == name) {
                    Some(k) => Some(x(X::Field(k, text.to_owned()))),
                    None => return Err(format!("“{LT}” penceresinde “{name}” alanı yok")),
                },
                other => return Err(format!("“{LT}” penceresinde {other} yok")),
            });
        }
        Ok(match control {
            Control::Press(LENGTHS) => Some(msg(Event::OpenLengths)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(ADD_CROSSING) => Some(msg(Event::AddCrossing)),
            Control::Press(REMOVE_CROSSING) => Some(msg(Event::RemoveCrossing)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(REBIND) => Some(msg(Event::Rebind)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE_PVI) => Some(msg(Event::RemovePvi)),
            Control::Press(FROM_GROUND) => Some(msg(Event::FromGround)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(words) => match w.rows.iter().position(|r| r.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("Düşey abartma", text) => Some(msg(Event::Exaggeration(text.to_owned()))),
            Control::Fill("Yer (Y; X)", text) => Some(msg(Event::Place(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

fn canvas<'a>(chart: Chart<'a>) -> iced::widget::Canvas<Chart<'a>, Message> {
    iced::widget::canvas(chart)
}

/// The chart's height: the drawing and its bands.
const CHART_HEIGHT: f32 = 330.0;
const BANDS: [&str; 6] = ["Km", "Mesafe", "Siyah kot", "Kırmızı kot", "Fark", "Eğim"];
const BAND: f32 = 18.0;
const LEFT: f32 = 92.0;

/// The profile drawn: the terrain, the design line, its PVIs and curves, and the bands under them; for the split views
/// (road_views_window.rs) a km's vertical line, and a press telling the station under it.
pub(crate) struct Chart<'a> {
    ground: &'a Ground,
    vertical: Option<Vertical>,
    pvis: Vec<(f64, f64)>,
    names: Vec<String>,
    chosen: Option<usize>,
    exaggeration: f64,
    cursor: Option<f64>,
    on_station: Option<fn(f64) -> Message>,
    /// The split views' short chart: no bands, the length across the width, the exaggeration as far as it fits.
    compact: bool,
    /// The crossings (docs/adr/0946): name, station, over the road, elevation, the grade line's limit.
    crossings: Vec<(String, f64, bool, f64, f64)>,
    /// The stretches the PVIs typed change (docs/adr/0962), shaded.
    changed: Vec<(f64, f64)>,
}

/// Where the chart draws: the plot, its horizontal scale (px per m) and the length shown.
struct Plot {
    area: Rectangle,
    sx: f64,
    length: f64,
}

impl<'a> Chart<'a> {
    /// A profile on its terrain, for the split views: the design from `def` (none when it does not build), the km line
    /// at `cursor`, a press telling its station.
    pub(crate) fn of(
        ground: &'a Ground,
        def: Option<&ProfileDef>,
        exaggeration: f64,
        cursor: Option<f64>,
        on_station: fn(f64) -> Message,
    ) -> Self {
        let pvis: Vec<(f64, f64)> = def.map_or_else(Vec::new, |d| d.pvis.iter().map(|p| (p.station, p.elevation)).collect());
        let vertical = def.and_then(|d| core::vertical::build(&kentos_native_application::road_profiles::pvis_of(d)).ok());
        Chart {
            ground,
            vertical,
            pvis,
            names: Vec::new(),
            chosen: None,
            exaggeration,
            cursor,
            on_station: Some(on_station),
            compact: false,
            crossings: def.map_or_else(Vec::new, chart_crossings),
            changed: Vec::new(),
        }
    }

    /// Short: no bands, the length across the width.
    pub(crate) fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// The kms marked, only those near the part shown: where the km grows with the station, the chart's own at least
    /// 90 px apart (finer as it is laid out larger, docs/adr/0937 §3); else the alignment's round kms.
    fn ticks(&self, sx: f64, shown: &crate::picture::Shown, left: f32) -> Vec<(f64, String)> {
        let g = self.ground;
        let near = |s: f64| shown.has_x(left + (s * sx) as f32);
        if !g.plain || g.length <= 0.0 || sx <= 0.0 {
            return g.ticks.iter().filter(|(s, _)| near(*s)).cloned().collect();
        }
        let step = crate::picture::step(90.0 / sx).max(1.0);
        let digits = crate::picture::decimals(step);
        let from = (f64::from(shown.part.x - left - 60.0) / sx).max(0.0);
        let to = (f64::from(shown.part.x + shown.part.width - left + 60.0) / sx).min(g.length);
        let mut out = Vec::new();
        let mut k = ((g.km0 + from) / step).ceil() * step;
        while k <= g.km0 + to + 1e-9 {
            out.push((k - g.km0, km_text(k, digits)));
            k += step;
        }
        out
    }

    /// The bands' top in a chart `height` high.
    fn bands_top(&self, height: f32) -> f32 {
        if self.compact { height } else { height - BAND * BANDS.len() as f32 }
    }

    /// The plot and its scale in `bounds`, none when there is nothing to draw.
    fn plot(&self, bounds: Size) -> Option<Plot> {
        let bands_top = self.bands_top(bounds.height);
        // The plot starts a little right of the bands' names, so the first km's values stay clear of them.
        let area = Rectangle { x: LEFT + 30.0, y: 10.0, width: bounds.width - LEFT - 60.0, height: bands_top - 22.0 };
        let length = self.ground.length.max(self.pvis.last().map_or(0.0, |p| p.0));
        let (lo, hi) = self.range().filter(|_| length > 0.0)?;
        // One horizontal scale fits the length; the vertical is it times the exaggeration, unless the heights do not fit.
        let across = area.width as f64 / length;
        let sx = if self.compact { across } else { across.min(area.height as f64 / ((hi - lo) * self.exaggeration)) };
        Some(Plot { area, sx, length })
    }

    /// The elevations shown: the terrain's and the design's, a little wider.
    fn range(&self) -> Option<(f64, f64)> {
        let zs = self
            .ground
            .points
            .iter()
            .filter_map(|p| p.z)
            .chain(self.pvis.iter().map(|p| p.1))
            .chain(self.vertical.iter().flat_map(|v| v.curves.iter().flat_map(|c| [c.start[1], c.end[1]])))
            .chain(self.crossings.iter().flat_map(|c| [c.3, c.4]));
        let (lo, hi) = zs.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(z), hi.max(z)));
        (lo.is_finite() && hi.is_finite()).then(|| {
            let pad = ((hi - lo) * 0.08).max(0.5);
            (lo - pad, hi + pad)
        })
    }
}

impl canvas::Program<Message> for Chart<'_> {
    /// Whether the km line is being dragged.
    type State = bool;

    fn update(
        &self,
        dragging: &mut bool,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let on = self.on_station?;
        let station = |at: Point| {
            let p = self.plot(bounds.size())?;
            Some(((f64::from(at.x - p.area.x)) / p.sx).clamp(0.0, p.length))
        };
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let s = station(cursor.position_in(bounds)?)?;
                *dragging = true;
                Some(canvas::Action::publish(on(s)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) if *dragging => {
                let at = cursor.position()?;
                let s = station(Point::new(at.x - bounds.x, at.y - bounds.y))?;
                Some(canvas::Action::publish(on(s)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if *dragging => {
                *dragging = false;
                Some(canvas::Action::request_redraw())
            }
            _ => None,
        }
    }

    fn draw(&self, _: &bool, renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let bands_top = self.bands_top(bounds.height);
        let (Some(Plot { area: plot, sx, length }), Some((lo, hi))) = (self.plot(bounds.size()), self.range()) else {
            frame.fill_text(Text {
                content: "PVI'leri ve yüzeyi verince boykesit burada çizilir.".into(),
                position: Point::new(LEFT, 20.0),
                color: t.muted,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                ..Text::default()
            });
            return vec![frame.into_geometry()];
        };
        let sy = if self.compact {
            (sx * self.exaggeration).min(f64::from(plot.height) / (hi - lo))
        } else {
            sx * self.exaggeration
        };
        let x = |s: f64| plot.x + (s * sx) as f32;
        let y = |z: f64| plot.y + plot.height - ((z - lo) * sy) as f32;
        let thin = |c: Color| Stroke::default().with_color(c).with_width(1.0);
        // The stretches the PVIs typed change, under everything (docs/adr/0962).
        for (a, b) in &self.changed {
            let (xa, xb) = (x(*a), x(*b).max(x(*a) + 1.0));
            frame.fill_rectangle(Point::new(xa, plot.y), Size::new(xb - xa, plot.height), Color { a: 0.16, ..t.warning });
        }
        let word = |frame: &mut canvas::Frame, s: String, at: Point, color: Color, centred: bool| {
            frame.fill_text(Text {
                content: s,
                position: at,
                color,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                align_x: if centred { iced::widget::text::Alignment::Center } else { iced::widget::text::Alignment::Default },
                ..Text::default()
            });
        };
        // Zoomed in a picture (docs/adr/0937 §3): the grid from the pixels (heights at least 40 px apart, kms 90 px),
        // finer as the chart is laid out larger; the elevations' labels and the bands stay on the part shown.
        let shown = crate::picture::Shown::of(bounds.size());
        let rows = if self.compact { 0.0 } else { BANDS.len() as f32 };
        let bands_top = shown.bottom(bands_top, BAND * rows);
        let bands_moved = bands_top < self.bands_top(bounds.height) - 0.5;
        // A canvas writes all its words over all its fills: a backdrop cannot hide a word under it. So the words that
        // would fall on the moved bands, or under the moved band names, are not written (docs/adr/0937 §3).
        let names_x = shown.left(6.0);
        let names_right = if names_x > 6.5 { names_x - 4.0 + LEFT - 8.0 } else { f32::NEG_INFINITY };
        let above_bands = |top: f32| !bands_moved || top + 15.0 < bands_top;
        let step = crate::picture::step(40.0 / sy);
        let places = crate::picture::decimals(step);
        let labels_x = shown.left(plot.x - 34.0);
        let mut heights = Vec::new();
        let mut z = (lo / step).ceil() * step;
        while z <= hi {
            if shown.has_y(y(z)) {
                frame.stroke(&Path::line(Point::new(plot.x, y(z)), Point::new(x(length), y(z))), thin(t.border));
                heights.push(z);
            }
            z += step;
        }
        let ticks = self.ticks(sx, &shown, plot.x);
        for (s, _) in &ticks {
            frame.stroke(&Path::line(Point::new(x(*s), plot.y), Point::new(x(*s), bands_top + BAND * rows)), thin(t.border));
        }
        // The terrain, broken where it is off the surface.
        let terrain = Path::new(|b| {
            let mut open = false;
            for p in &self.ground.points {
                match p.z {
                    Some(z) if open && !p.gap => b.line_to(Point::new(x(p.station), y(z))),
                    Some(z) => {
                        b.move_to(Point::new(x(p.station), y(z)));
                        open = true;
                    }
                    None => open = false,
                }
            }
        });
        frame.stroke(&terrain, Stroke::default().with_color(t.text).with_width(1.3));
        // The PVIs' tangent polygon, dashed, and the design line.
        if self.pvis.len() > 1 {
            let tangents = Path::new(|b| {
                for (i, p) in self.pvis.iter().enumerate() {
                    let at = Point::new(x(p.0), y(p.1));
                    if i == 0 { b.move_to(at) } else { b.line_to(at) }
                }
            });
            frame.stroke(
                &tangents,
                Stroke { line_dash: LineDash { segments: &[4.0, 3.0], offset: 0 }, ..thin(t.muted) },
            );
        }
        let red = t.danger;
        // What the road crosses (docs/adr/0946): a bridge's deck over the line, a culvert's box under it, the limit
        // dashed between; green where the line keeps it, red where it does not.
        for (name, s, above, z, limit) in &self.crossings {
            let line_z = self.vertical.as_ref().and_then(|v| v.at(*s)).map(|q| q.0);
            let kept = line_z.is_none_or(|lz| if *above { lz <= *limit + 1e-9 } else { lz >= *limit - 1e-9 });
            let colour = if kept { t.success } else { t.danger };
            let (cx, half) = (x(*s), 14.0f32);
            if !shown.has_x(cx) {
                continue;
            }
            if *above {
                // The deck: its underside at z, a slab above.
                frame.fill_rectangle(Point::new(cx - half, y(*z) - 6.0), Size::new(2.0 * half, 6.0), Color { a: 0.85, ..t.muted });
            } else {
                // The culvert: a box whose top is z.
                frame.stroke(&Path::rectangle(Point::new(cx - 8.0, y(*z)), Size::new(16.0, 12.0)), Stroke::default().with_color(t.muted).with_width(1.5));
            }
            let dash = Stroke { line_dash: LineDash { segments: &[3.0, 2.0], offset: 0 }, ..Stroke::default().with_color(colour).with_width(1.5) };
            frame.stroke(&Path::line(Point::new(cx - half - 6.0, y(*limit)), Point::new(cx + half + 6.0, y(*limit))), dash);
            frame.stroke(&Path::line(Point::new(cx, y(*z)), Point::new(cx, y(*limit))), thin(colour));
            let top = if *above { y(*z) - 22.0 } else { y(*z) + 16.0 };
            if above_bands(top) {
                word(&mut frame, name.clone(), Point::new(cx, top), colour, true);
            }
        }
        if let Some(v) = &self.vertical {
            let n = (plot.width as usize).max(50);
            let design = Path::new(|b| {
                for i in 0..=n {
                    let s = v.start + (v.end - v.start) * i as f64 / n as f64;
                    if let Some((z, _, _)) = v.at(s) {
                        let at = Point::new(x(s), y(z));
                        if i == 0 { b.move_to(at) } else { b.line_to(at) }
                    }
                }
            });
            frame.stroke(&design, Stroke::default().with_color(red).with_width(2.0));
            for c in &v.curves {
                curve_marks(&mut frame, c, &x, &y, red);
                // Away from the PVI: under a crest's curve, over a sag's.
                let (hi_z, lo_z) = [c.start[1], c.end[1]].into_iter().chain(c.extreme.map(|e| e[1])).fold(
                    (f64::NEG_INFINITY, f64::INFINITY),
                    |(h, l), z| (h.max(z), l.min(z)),
                );
                let place_y = if c.crest { y(lo_z) + 8.0 } else { y(hi_z) - 22.0 };
                let kind = match c.curve {
                    VCurve::Asymmetric { .. } => "asim.",
                    VCurve::Circle { .. } => "R",
                    _ => "par.",
                };
                if above_bands(place_y) {
                    word(
                        &mut frame,
                        format!("{} {kind} K={}", c.pvi, comma(c.k.unwrap_or(0.0), 1)),
                        Point::new(x(0.5 * (c.start[0] + c.end[0])), place_y),
                        red,
                        true,
                    );
                }
            }
        }
        for (i, p) in self.pvis.iter().enumerate() {
            let on = self.chosen == Some(i);
            let mark = Path::circle(Point::new(x(p.0), y(p.1)), if on { 5.0 } else { 3.5 });
            frame.stroke(&mark, Stroke::default().with_color(if on { t.accent } else { red }).with_width(1.5));
            if let Some(name) = self.names.get(i).filter(|_| above_bands(y(p.1) - 16.0)) {
                word(&mut frame, name.clone(), Point::new(x(p.0) + 6.0, y(p.1) - 16.0), t.muted, false);
            }
        }
        // The elevations' labels over the drawing, above the bands.
        for z in heights {
            if y(z) < bands_top - 4.0 {
                let moved = labels_x > plot.x - 33.5;
                crate::picture::label(&mut frame, comma(z, places), Point::new(labels_x, y(z) - 7.0), t.muted, false, moved.then_some(t.field.scale_alpha(0.9)));
            }
        }
        // The bands: moved up over the drawing on its own ground when the view is above them.
        let names: &[&str] = if self.compact { &[] } else { &BANDS };
        if bands_moved && !names.is_empty() {
            // Down to the part's bottom: the scrollbar's strip under the bands shows no drawing either.
            let down = (shown.part.y + shown.part.height - bands_top).max(BAND * rows);
            frame.fill(&Path::rectangle(Point::new(shown.part.x, bands_top), Size::new(shown.part.width, down)), t.field);
        }
        if names_x > 6.5 && !names.is_empty() {
            frame.fill(&Path::rectangle(Point::new(names_x - 4.0, bands_top), Size::new(LEFT - 8.0, BAND * rows)), t.field);
        }
        for (k, name) in names.iter().enumerate() {
            let top = bands_top + BAND * k as f32;
            frame.stroke(&Path::line(Point::new(0.0, top), Point::new(bounds.width, top)), thin(t.border));
            word(&mut frame, (*name).to_owned(), Point::new(names_x, top + 3.0), if k == 3 { red } else { t.muted }, false);
        }
        let mut before: Option<f64> = None;
        for (s, km) in ticks.iter().filter(|_| !self.compact) {
            let cx = x(*s);
            let black = self.ground.at(*s);
            let design = self.vertical.as_ref().and_then(|v| v.at(*s)).map(|r| r.0);
            let cells = [
                Some(km.clone()),
                before.map(|b| comma(s - b, 0)),
                black.map(|z| comma(z, 2)),
                design.map(|z| comma(z, 2)),
                design.zip(black).map(|(r, b)| format!("{}{}", if r >= b { "+" } else { "" }, comma(r - b, 2))),
            ];
            for (k, cell) in cells.into_iter().enumerate().filter(|_| cx - 34.0 > names_right) {
                if let Some(text) = cell {
                    let color = if k == 3 { red } else { t.text };
                    word(&mut frame, text, Point::new(cx, bands_top + BAND * k as f32 + 3.0), color, true);
                }
            }
            before = Some(*s);
        }
        // The km of the split views: a line through the plot and the bands, its km and elevations at its foot.
        if let Some(c) = self.cursor.filter(|c| (0.0..=length).contains(c)) {
            let cx = x(c);
            let line = Stroke { line_dash: LineDash { segments: &[5.0, 3.0], offset: 0 }, ..thin(t.accent) };
            frame.stroke(&Path::line(Point::new(cx, plot.y), Point::new(cx, bands_top)), Stroke { width: 1.4, ..line });
            for z in [self.ground.at(c), self.vertical.as_ref().and_then(|v| v.at(c)).map(|r| r.0)].into_iter().flatten() {
                frame.fill(&Path::circle(Point::new(cx, y(z)), 3.0), t.accent);
            }
        }
        // The grades between the PVIs.
        let grade_top = bands_top + BAND * 5.0;
        for w in self.pvis.windows(2).filter(|_| !self.compact) {
            let ((s0, z0), (s1, z1)) = (w[0], w[1]);
            if s1 <= s0 {
                continue;
            }
            let g = (z1 - z0) / (s1 - s0);
            frame.stroke(&Path::line(Point::new(x(s1), grade_top), Point::new(x(s1), grade_top + BAND)), thin(t.muted));
            if x(0.5 * (s0 + s1)) - 48.0 <= names_right {
                continue;
            }
            word(
                &mut frame,
                format!("%{} L={}", comma(100.0 * g, 2), comma(s1 - s0, 1)),
                Point::new(x(0.5 * (s0 + s1)), grade_top + 3.0),
                t.text,
                true,
            );
        }
        vec![frame.into_geometry()]
    }
}

/// A curve's start and end, short ticks across the design line.
fn curve_marks(frame: &mut canvas::Frame, c: &CurveReport, x: &dyn Fn(f64) -> f32, y: &dyn Fn(f64) -> f32, color: Color) {
    for p in [c.start, c.end] {
        frame.stroke(
            &Path::line(Point::new(x(p[0]), y(p[1]) - 6.0), Point::new(x(p[0]), y(p[1]) + 6.0)),
            Stroke::default().with_color(color).with_width(1.2),
        );
    }
}

/// A profile's crossings for its chart (docs/adr/0946).
fn chart_crossings(d: &ProfileDef) -> Vec<(String, f64, bool, f64, f64)> {
    d.crossings
        .iter()
        .map(|c| {
            let above = c.side == kentos_commands::CrossingSide::Above;
            let limit = if above { c.elevation - c.clearance } else { c.elevation + c.clearance };
            (c.name.clone(), c.station, above, c.elevation, limit)
        })
        .collect()
}
