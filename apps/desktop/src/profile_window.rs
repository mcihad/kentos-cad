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
const CURVES: [&str; 4] = ["Yok", "Parabol", "Asimetrik", "Çember"];

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
        Ok(ProfileDef {
            id: self.id.clone(),
            name: self.name.clone(),
            alignment: self.alignment.clone(),
            alignment_revision: self.alignment_revision,
            revision: self.revision,
            pvis,
            surface: self.surface.clone(),
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
    /// The vertical exaggeration's text.
    exaggeration: String,
    pub(crate) ground: Ground,
    /// Çizime yaz: the layer, and the frame's lower left as typed (“Y; X”).
    layer: Option<String>,
    place: String,
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
}

fn msg(event: Event) -> Message {
    Message::Profiles(event)
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
    if let Some(k) = v.curves.iter().filter_map(|c| c.k).min_by(f64::total_cmp) {
        words.push_str(&format!(" · en küçük K {}", comma(k, 1)));
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
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        if regrounded {
            self.refresh_ground();
        }
        Task::none()
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
            Some(r) => scrollable(self.road_profile_editor(w, r, stale_of(r))).height(typography::scaled(440.0)).into(),
        };
        let saved = chosen.is_some_and(|r| w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)));
        let body = column![
            row![container(left).width(220), container(right).width(Fill)].spacing(16),
            self.profile_draw_row(w, saved),
            row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                words::secondary(REBIND, (saved && chosen.is_some_and(stale_of)).then(|| msg(Event::Rebind))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, Some(msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center),
        ]
        .spacing(14);
        Frame::new(TITLE).push(body).width(1080.0).modal(msg(Event::Cancel))
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
        let mut col = Column::new().spacing(10).padding(iced::Padding::default().right(14));
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
        };
        col = col.push(
            container(canvas(chart).width(Fill).height(typography::scaled(CHART_HEIGHT)))
                .style(style::container::field_box)
                .width(Fill),
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
        let head = ["PVI", "Km (m)", "Kot", "Kurp", "L / R", "L çıkış"];
        let widths = [60.0, 110.0, 100.0, 130.0, 90.0, 90.0];
        let mut table = Column::new().spacing(3);
        table = table.push(
            head.iter().zip(widths).fold(row![].spacing(4), |row, (h, wd)| row.push(label::caption(*h).style(muted).width(wd))),
        );
        for (j, p) in r.pvis.iter().enumerate() {
            let end = j == 0 || j + 1 == r.pvis.len();
            let pick = Select::new(
                CURVES.iter().map(|c| Choice::new((*c).to_owned())),
                Some(p.curve),
                move |c| msg(Event::Curve(j, c)),
            );
            let curve: Element<'_, Message> =
                if end { label::caption("—").style(muted).width(widths[3]).into() } else { container(pick).width(widths[3]).into() };
            let size = |k: usize, hint: &'a str| -> Element<'a, Message> {
                let shown = matches!((p.curve, k), (1 | 3, 0) | (2, _));
                if shown && !end {
                    kentos_ui::widget::text_field(hint, &p.sizes[k]).on_input(move |t| msg(Event::Pvi(j, 3 + k, t))).width(widths[4 + k]).into()
                } else {
                    space::horizontal().width(widths[4 + k]).into()
                }
            };
            let line = row![
                kentos_ui::widget::text_field("", &p.id).on_input(move |t| msg(Event::Pvi(j, 0, t))).width(widths[0]),
                kentos_ui::widget::text_field("0", &p.station).on_input(move |t| msg(Event::Pvi(j, 1, t))).width(widths[1]),
                kentos_ui::widget::text_field("100", &p.elevation).on_input(move |t| msg(Event::Pvi(j, 2, t))).width(widths[2]),
                curve,
                size(0, match p.curve { 2 => "L giriş", 3 => "R", _ => "L" }),
                size(1, "L çıkış"),
                iced::widget::button(label::caption(if w.pvi == Some(j) { "●" } else { "○" }))
                    .on_press(msg(Event::PviChoose(j)))
                    .padding([2, 6])
                    .style(style::button::row(w.pvi == Some(j))),
            ]
            .spacing(4)
            .align_y(Center);
            table = table.push(line);
        }
        col = col.push(words::field("PVI'ler", table, None)).push(
            row![
                words::secondary(ADD, Some(msg(Event::Add))),
                words::ghost(REMOVE_PVI, w.pvi.filter(|j| *j < r.pvis.len() && r.pvis.len() > 2).map(|_| msg(Event::RemovePvi))),
                words::secondary(FROM_GROUND, Some(msg(Event::FromGround))),
            ]
            .spacing(8),
        );
        col.into()
    }

    pub(crate) fn profile_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.profile_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
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
        }
    }

    /// Short: no bands, the length across the width.
    pub(crate) fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
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
            .chain(self.vertical.iter().flat_map(|v| v.curves.iter().flat_map(|c| [c.start[1], c.end[1]])));
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
        // The elevation grid: a round step for about six lines.
        let step = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0].into_iter().find(|s| (hi - lo) / s <= 7.0).unwrap_or(200.0);
        let mut z = (lo / step).ceil() * step;
        while z <= hi {
            frame.stroke(&Path::line(Point::new(plot.x, y(z)), Point::new(x(length), y(z))), thin(t.border));
            word(&mut frame, comma(z, if step < 1.0 { 1 } else { 0 }), Point::new(plot.x - 34.0, y(z) - 7.0), t.muted, false);
            z += step;
        }
        for (s, _) in &self.ground.ticks {
            frame.stroke(&Path::line(Point::new(x(*s), plot.y), Point::new(x(*s), bounds.height)), thin(t.border));
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
                word(
                    &mut frame,
                    format!("{} {kind} K={}", c.pvi, comma(c.k.unwrap_or(0.0), 1)),
                    Point::new(x(0.5 * (c.start[0] + c.end[0])), place_y),
                    red,
                    true,
                );
            }
        }
        for (i, p) in self.pvis.iter().enumerate() {
            let on = self.chosen == Some(i);
            let mark = Path::circle(Point::new(x(p.0), y(p.1)), if on { 5.0 } else { 3.5 });
            frame.stroke(&mark, Stroke::default().with_color(if on { t.accent } else { red }).with_width(1.5));
            if let Some(name) = self.names.get(i) {
                word(&mut frame, name.clone(), Point::new(x(p.0) + 6.0, y(p.1) - 16.0), t.muted, false);
            }
        }
        // The bands.
        let names: &[&str] = if self.compact { &[] } else { &BANDS };
        for (k, name) in names.iter().enumerate() {
            let top = bands_top + BAND * k as f32;
            frame.stroke(&Path::line(Point::new(0.0, top), Point::new(bounds.width, top)), thin(t.border));
            word(&mut frame, (*name).to_owned(), Point::new(6.0, top + 3.0), if k == 3 { red } else { t.muted }, false);
        }
        let mut before: Option<f64> = None;
        for (s, km) in self.ground.ticks.iter().filter(|_| !self.compact) {
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
            for (k, cell) in cells.into_iter().enumerate() {
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
