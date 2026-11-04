//! Koridor (plan/road.md Y7, `corridor.editor`): a road project's corridors, as Civil 3D's corridor properties and
//! OpenRoads' corridor with its template drops make them: on an alignment, its profile, surface and superelevation
//! schedule, the swath searched, how often it is sampled (the tangent and curve intervals, a chord tolerance, the
//! profile's points, the critical stations, the stations written) and its regions each with a template. The corridor
//! is built as it is typed (`kentos.corridor/1`): the plan shows its top surface and feature lines, the stretches not
//! solved and where the sections fold; the warnings say why before anything is saved.
//!
//! ```text
//! ┌ Koridor ────────────────────────────────────────────────────────────────────────────────────┐
//! │ Koridorlar          │ Ad [Ana koridor] Aks [▾] Boykesit [▾] Yüzey [▾] Dever [▾]              │
//! │ ● Ana koridor       │ Sol [30] Sağ [30] Aliyman [25] Kurp [10] Sapma [ ] ☑ profil ☐ kritik  │
//! │                     │ Düşey kiriş [ ] Plan sapması [0,05] Kot sapması [0,05] En kısa adım [0,5] │
//! │ [Yeni koridor]      │ ┌───────────────────────────────────────────────────────────────────┐ │
//! │                     │ │  plan: top surface, feature lines, stretches not solved            │ │
//! │                     │ └───────────────────────────────────────────────────────────────────┘ │
//! │                     │ Baş  Son  Tip kesit  ○        [Bölge ekle] [Bölgeyi sil]              │
//! │ Şevli plan  Katman [▾]  Tarama [5]  [Şevli plan çiz]                                         │
//! │ [Sil] [Aksa yeniden bağla]                                               [Vazgeç] [Kaydet] │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the corridors; Kaydet writes what changed through `road.corridors.edit` (removes, then
//! sets), each one undo step, and closes. Aksa yeniden bağla rebinds a saved, stale one; Şevli plan çiz draws a saved
//! corridor's slope plan (`road.corridors.draw`). The intervals and the regions are the user's: nothing is filled in.
//! The sampling check (docs/adr/0939) is asked by writing its least step: the plan draws each section across, the
//! ones the check added in purple, and a sentence says what was measured and the largest strays before and after.

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandWarning, CorridorDef, CorridorRefineDef, CorridorRegionDef, CorridorReport, CorridorsDraw, CorridorsEdit,
    RoadEditOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_corridors, road_corridors_draw, road_corridors_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::road_dialog;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Koridor";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni koridor";
const REMOVE: &str = "Sil";
const REBIND: &str = "Aksa yeniden bağla";
const ADD: &str = "Bölge ekle";
const REMOVE_ROW: &str = "Bölgeyi sil";
const DRAW: &str = "Şevli plan çiz";

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

fn number(text: &str) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| ())
}

fn need(text: &str, what: &str) -> Result<f64, String> {
    match number(text) {
        Ok(Some(v)) => Ok(v),
        _ => Err(format!("{what} bir sayı olmalı.")),
    }
}

/// A corridor as typed.
#[derive(Debug, Clone, Default)]
pub struct Row {
    pub(crate) id: String,
    pub(crate) name: String,
    revision: u32,
    alignment: String,
    alignment_revision: u32,
    profile: Option<String>,
    surface: Option<String>,
    superelevation: Option<String>,
    /// Left, right, tangent, curve, sag; the vertical chord height; the sampling check's plan and height tolerances
    /// and least step (docs/adr/0939); the stations written ("; " between).
    fields: [String; 9],
    stations: String,
    profile_points: bool,
    critical: bool,
    /// From, to and the template's id.
    regions: Vec<(String, String, Option<String>)>,
}

impl Row {
    fn of(d: &CorridorDef) -> Self {
        Self {
            id: d.id.clone(),
            name: d.name.clone(),
            revision: d.revision,
            alignment: d.alignment.clone(),
            alignment_revision: d.alignment_revision,
            profile: Some(d.profile.clone()),
            surface: Some(d.surface.clone()),
            superelevation: d.superelevation.clone(),
            fields: [
                trimmed(d.left),
                trimmed(d.right),
                trimmed(d.tangent),
                trimmed(d.curve),
                d.sag.map(trimmed).unwrap_or_default(),
                d.vertical_sag.map(trimmed).unwrap_or_default(),
                d.refine.as_ref().and_then(|r| r.plan).map(trimmed).unwrap_or_default(),
                d.refine.as_ref().and_then(|r| r.height).map(trimmed).unwrap_or_default(),
                d.refine.as_ref().map(|r| trimmed(r.least)).unwrap_or_default(),
            ],
            stations: d.stations.iter().map(|s| trimmed(*s)).collect::<Vec<_>>().join("; "),
            profile_points: d.profile_points,
            critical: d.critical,
            regions: d.regions.iter().map(|r| (trimmed(r.from), trimmed(r.to), Some(r.template.clone()))).collect(),
        }
    }

    fn def(&self) -> Result<CorridorDef, String> {
        let profile = self.profile.clone().ok_or("Bir boykesit seçin: koridor eksen kotunu ondan alır.")?;
        let surface = self.surface.clone().ok_or("Bir yüzey seçin: şev araziyle onda buluşur.")?;
        let mut stations = Vec::new();
        for (i, t) in self.stations.split(';').map(str::trim).filter(|t| !t.is_empty()).enumerate() {
            stations.push(need(t, &format!("{}. km", i + 1))?);
        }
        let mut regions = Vec::new();
        for (i, (a, b, t)) in self.regions.iter().enumerate() {
            let n = i + 1;
            regions.push(CorridorRegionDef {
                from: need(a, &format!("{n}. bölgenin başı"))?,
                to: need(b, &format!("{n}. bölgenin sonu"))?,
                template: t.clone().ok_or(format!("{n}. bölgenin tip kesitini seçin."))?,
            });
        }
        if regions.is_empty() {
            return Err("Bölge ekle ile en az bir bölge verin.".into());
        }
        // The sampling check (docs/adr/0939 §2): asked when its least step is written.
        let tolerance = |k: usize, what: &str| number(&self.fields[k]).map_err(|_| format!("{what} bir sayı olmalı."));
        let (plan, height) = (tolerance(6, "Plan sapması")?, tolerance(7, "Kot sapması")?);
        let refine = match tolerance(8, "En kısa adım")? {
            Some(least) => Some(CorridorRefineDef { plan, height, least }),
            None if plan.is_some() || height.is_some() => {
                return Err("Kesitleri denetlemek için en kısa adımı da yazın.".into());
            }
            None => None,
        };
        Ok(CorridorDef {
            id: self.id.clone(),
            name: self.name.clone(),
            alignment: self.alignment.clone(),
            alignment_revision: self.alignment_revision,
            revision: self.revision,
            profile,
            surface,
            superelevation: self.superelevation.clone(),
            left: need(&self.fields[0], "Sol arama genişliği")?,
            right: need(&self.fields[1], "Sağ arama genişliği")?,
            tangent: need(&self.fields[2], "Aliyman aralığı")?,
            curve: need(&self.fields[3], "Kurp aralığı")?,
            sag: number(&self.fields[4]).map_err(|_| "Sapma toleransı bir sayı olmalı.".to_owned())?,
            profile_points: self.profile_points,
            critical: self.critical,
            stations,
            regions,
            vertical_sag: number(&self.fields[5]).map_err(|_| "Düşey kiriş yüksekliği bir sayı olmalı.".to_owned())?,
            refine,
        })
    }
}

/// The chosen corridor as last built: its report, the top surface and the warnings, or why it does not build.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<CorridorReport>,
    pub(crate) triangles: Vec<[[f64; 3]; 3]>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) rows: Vec<Row>,
    kept: Vec<CorridorDef>,
    pub(crate) chosen: Option<usize>,
    line: Option<usize>,
    pub(crate) preview: Preview,
    layer: Option<String>,
    tick: String,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Remove,
    Rebind,
    Name(String),
    Alignment(String),
    Profile(Option<String>),
    Surface(Option<String>),
    Superelevation(Option<String>),
    /// 0 left, 1 right, 2 tangent, 3 curve, 4 sag, 5 vertical chord height, 6 plan and 7 height tolerance, 8 least step.
    Field(usize, String),
    Stations(String),
    ProfilePoints,
    Critical,
    /// A region's from (0) or to (1).
    Region(usize, usize, String),
    Template(usize, Option<String>),
    Line(usize),
    Add,
    RemoveRow,
    DrawLayer(String),
    Tick(String),
    Draw,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Corridors(event)
}

/// The chosen corridor's row.
fn chosen_row(w: &mut Window) -> Option<&mut Row> {
    w.chosen.and_then(|i| w.rows.get_mut(i))
}

type Named = Vec<(String, String)>;
/// The project's alignments (id, name, revision), profiles and schedules (id, name, alignment), surfaces and templates.
type Names = (Vec<(String, String, u32)>, Vec<(String, String, String)>, Named, Vec<(String, String, String)>, Named);

impl App {
    /// Koridor: the window on the road project's corridors.
    pub(crate) fn open_corridors(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Koridor yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let kept = doc.model.road_corridors().to_vec();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        self.corridor_window = Some(Box::new(Window {
            chosen: (!kept.is_empty()).then_some(0),
            rows: kept.iter().map(Row::of).collect(),
            kept,
            layer,
            tick: "5".into(),
            ..Window::default()
        }));
        self.dialog = Some(Dialog::Corridors);
        self.refresh_corridor();
    }

    /// Koridor oluştur from the ribbon: the window with a new corridor on the first alignment.
    pub(crate) fn new_corridor(&mut self) {
        self.open_corridors();
        if self.corridor_window.is_some() {
            let _ = self.corridor_event(Event::New);
        }
    }

    /// Şevli plan from the ribbon: the window on the first corridor, to draw it.
    pub(crate) fn corridor_slope_plan(&mut self) {
        self.open_corridors();
    }

    fn refresh_corridor(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.corridor_window.as_mut() else { return };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            w.preview = Preview::default();
            return;
        };
        w.preview = match r.def() {
            Err(e) => Preview { why: Some(e), ..Preview::default() },
            Ok(def) => match road_corridors::preview(&doc.model, &def, doc.model.road_templates()) {
                Err(e) => Preview { why: Some(e), ..Preview::default() },
                Ok((report, built, warnings)) => Preview { report: Some(report), triangles: built.triangles, warnings, why: None },
            },
        };
    }

    fn corridor_names(&self) -> Names {
        self.document.as_ref().map_or_else(Default::default, |d| {
            let m = &d.model;
            (
                m.road_alignments().iter().filter(|a| a.offset_of.is_none()).map(|a| (a.id.clone(), a.name.clone(), a.revision)).collect(),
                m.road_profiles().iter().map(|p| (p.id.clone(), p.name.clone(), p.alignment.clone())).collect(),
                m.settings().road.as_ref().map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect()),
                m.road_superelevations().iter().map(|s| (s.id.clone(), s.name.clone(), s.alignment.clone())).collect(),
                m.road_templates().iter().map(|t| (t.id.clone(), t.name.clone())).collect(),
            )
        })
    }

    pub(crate) fn corridor_event(&mut self, event: Event) -> Task<Message> {
        let (alignments, profiles, surfaces, _, templates) = self.corridor_names();
        let Some(w) = self.corridor_window.as_mut() else { return Task::none() };
        let mut warning = None;
        match event {
            Event::Choose(i) => {
                w.chosen = Some(i);
                w.line = None;
            }
            Event::New => match alignments.first() {
                None => warning = Some("Koridor bir aksın üstünde kurulur: önce Güzergâh'ta bir aks oluşturun."),
                Some((id, _, rev)) => {
                    let taken: Vec<String> = w.rows.iter().map(|r| r.id.clone()).collect();
                    let new_id = (1..).map(|n| format!("c-{n}")).find(|x| !taken.contains(x)).unwrap_or_default();
                    let n = w.rows.len() + 1;
                    // The sources picked on the alignment; the swath, intervals and regions are the user's.
                    w.rows.push(Row {
                        id: new_id,
                        name: if n == 1 { "Koridor".to_owned() } else { format!("Koridor {n}") },
                        alignment: id.clone(),
                        alignment_revision: *rev,
                        profile: profiles.iter().find(|p| p.2 == *id).map(|p| p.0.clone()),
                        surface: surfaces.first().map(|s| s.0.clone()),
                        ..Row::default()
                    });
                    w.chosen = Some(w.rows.len() - 1);
                    w.line = None;
                }
            },
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|i| *i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen = (!w.rows.is_empty()).then_some(i.min(w.rows.len().saturating_sub(1)));
                }
            }
            Event::Rebind => return self.rebind_corridor(),
            Event::Draw => return self.draw_corridor(),
            Event::Name(t) => {
                if let Some(r) = chosen_row(w) {
                    r.name = t;
                }
            }
            Event::Alignment(id) => {
                let saved = w.chosen.and_then(|i| w.rows.get(i)).is_some_and(|r| w.kept.iter().any(|k| k.id == r.id));
                if saved {
                    warning = Some("Kaydedilmiş koridorun aksı değişmez; yeni bir koridor oluşturun.");
                } else if let Some(r) = chosen_row(w) {
                    r.alignment_revision = alignments.iter().find(|a| a.0 == id).map_or(0, |a| a.2);
                    r.profile = profiles.iter().find(|p| p.2 == id).map(|p| p.0.clone());
                    r.superelevation = None;
                    r.alignment = id;
                }
            }
            Event::Profile(p) => {
                if let Some(r) = chosen_row(w) {
                    r.profile = p;
                }
            }
            Event::Surface(s) => {
                if let Some(r) = chosen_row(w) {
                    r.surface = s;
                }
            }
            Event::Superelevation(s) => {
                if let Some(r) = chosen_row(w) {
                    r.superelevation = s;
                }
            }
            Event::Field(k, t) => {
                if let Some(r) = chosen_row(w) {
                    r.fields[k.min(8)] = t;
                }
            }
            Event::Stations(t) => {
                if let Some(r) = chosen_row(w) {
                    r.stations = t;
                }
            }
            Event::ProfilePoints => {
                if let Some(r) = chosen_row(w) {
                    r.profile_points = !r.profile_points;
                }
            }
            Event::Critical => {
                if let Some(r) = chosen_row(w) {
                    r.critical = !r.critical;
                }
            }
            Event::Region(j, k, t) => {
                if let Some(x) = chosen_row(w).and_then(|r| r.regions.get_mut(j)) {
                    if k == 0 {
                        x.0 = t;
                    } else {
                        x.1 = t;
                    }
                }
                w.line = Some(j);
            }
            Event::Template(j, t) => {
                if let Some(x) = chosen_row(w).and_then(|r| r.regions.get_mut(j)) {
                    x.2 = t;
                }
                w.line = Some(j);
            }
            Event::Line(j) => w.line = Some(j),
            Event::Add => {
                let first = templates.first().map(|t| t.0.clone());
                if let Some(r) = chosen_row(w) {
                    // A new region starts where the last ends; its end is the user's.
                    let from = r.regions.last().map(|x| x.1.clone()).unwrap_or_else(|| "0".into());
                    r.regions.push((from, String::new(), first));
                    let n = r.regions.len() - 1;
                    w.line = Some(n);
                }
            }
            Event::RemoveRow => {
                if let Some(j) = w.line {
                    if let Some(r) = chosen_row(w).filter(|r| j < r.regions.len()) {
                        r.regions.remove(j);
                    }
                    w.line = None;
                }
            }
            Event::DrawLayer(l) => w.layer = Some(l),
            Event::Tick(t) => w.tick = t,
            Event::Cancel => {
                self.corridor_window = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Save => return self.save_corridors(),
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        self.refresh_corridor();
        Task::none()
    }

    fn blank_corridor_edit(operation: RoadEditOperation, id: &str) -> CorridorsEdit {
        CorridorsEdit {
            operation,
            corridor: Some(id.to_owned()),
            name: None,
            alignment: None,
            profile: None,
            surface: None,
            superelevation: None,
            left: None,
            right: None,
            tangent: None,
            curve: None,
            sag: None,
            vertical_sag: None,
            refine: None,
            profile_points: None,
            critical: None,
            stations: None,
            regions: None,
            expected_revision: None,
        }
    }

    fn corridor_write(&mut self, edit: CorridorsEdit) -> bool {
        let Some(doc) = self.document.as_mut() else { return false };
        let result = road_corridors_edit::execute(&mut ExecutionContext::new(&mut doc.model), edit);
        self.answered(result).is_some()
    }

    /// The chosen corridor, saved as it is typed; else a warning and none.
    fn chosen_saved_corridor(&mut self, what: &str) -> Option<CorridorDef> {
        let w = self.corridor_window.as_ref()?;
        let r = w.chosen.and_then(|i| w.rows.get(i))?;
        let def = r.def().ok();
        match def.filter(|d| w.kept.contains(d)) {
            Some(d) => Some(d),
            None => {
                self.warn(format!("Önce koridoru kaydedin; kaydedilmiş koridor {what}."));
                None
            }
        }
    }

    fn reload_corridors(&mut self) {
        let Some(doc) = &self.document else { return };
        let kept = doc.model.road_corridors().to_vec();
        if let Some(w) = self.corridor_window.as_mut() {
            w.rows = kept.iter().map(Row::of).collect();
            w.kept = kept;
        }
        self.refresh_corridor();
    }

    fn rebind_corridor(&mut self) -> Task<Message> {
        let Some(d) = self.chosen_saved_corridor("yeniden bağlanır") else { return Task::none() };
        if self.corridor_write(Self::blank_corridor_edit(RoadEditOperation::Rebind, &d.id)) {
            self.reload_corridors();
            self.say(Level::Success, "Koridor aksın bugünkü hâline bağlandı.");
        }
        Task::none()
    }

    fn draw_corridor(&mut self) -> Task<Message> {
        let Some(d) = self.chosen_saved_corridor("çizilir") else { return Task::none() };
        let Some(w) = &self.corridor_window else { return Task::none() };
        let Some(layer) = w.layer.clone() else {
            self.warn("Şevli planın çizileceği katmanı seçin.".to_owned());
            return Task::none();
        };
        let tick = match number(&w.tick) {
            Ok(v) => v,
            Err(()) => {
                self.warn("Şev taramasının aralığı bir sayı olmalı.".to_owned());
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let input = CorridorsDraw { corridor: d.id.clone(), layer_id: layer, tick, height: None, expected_revision: None };
        let result = road_corridors_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(
                Level::Success,
                format!("“{}” şevli planı çizildi: {} çizgi, {} şev çizgisi, {} kot.", d.name, out.lines, out.ticks, out.elevations),
            );
        }
        Task::none()
    }

    fn save_corridors(&mut self) -> Task<Message> {
        let Some(w) = self.corridor_window.clone() else { return Task::none() };
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
            if !self.corridor_write(Self::blank_corridor_edit(RoadEditOperation::Remove, &k.id)) {
                return Task::none();
            }
        }
        for d in &defs {
            let kept = w.kept.iter().find(|k| k.id == d.id);
            if kept == Some(d) {
                continue;
            }
            let edit = CorridorsEdit {
                name: Some(d.name.clone()),
                alignment: kept.is_none_or(|k| k.alignment != d.alignment).then(|| d.alignment.clone()),
                profile: Some(d.profile.clone()),
                surface: Some(d.surface.clone()),
                superelevation: Some(d.superelevation.clone().unwrap_or_default()),
                left: Some(d.left),
                right: Some(d.right),
                tangent: Some(d.tangent),
                curve: Some(d.curve),
                sag: Some(d.sag.unwrap_or(0.0)),
                vertical_sag: Some(d.vertical_sag.unwrap_or(0.0)),
                refine: Some(d.refine.clone().unwrap_or(CorridorRefineDef { plan: None, height: None, least: 0.0 })),
                profile_points: Some(d.profile_points),
                critical: Some(d.critical),
                stations: Some(d.stations.clone()),
                regions: Some(d.regions.clone()),
                ..Self::blank_corridor_edit(RoadEditOperation::Set, &d.id)
            };
            if !self.corridor_write(edit) {
                return Task::none();
            }
        }
        self.corridor_window = None;
        self.dialog = None;
        self.say(Level::Success, "Koridorlar kaydedildi.");
        Task::none()
    }

    pub(crate) fn road_corridor_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.corridor_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let (alignments, ..) = self.corridor_names();
        let stale_of = |r: &Row| {
            w.kept.iter().any(|k| k.id == r.id) && alignments.iter().find(|a| a.0 == r.alignment).is_some_and(|a| a.2 != r.alignment_revision)
        };
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let on = alignments.iter().find(|a| a.0 == r.alignment).map_or(r.alignment.clone(), |a| a.1.clone());
            let kind = if stale_of(r) { format!("{on} · eski") } else { format!("{on} · {} bölge", r.regions.len()) };
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
            list = list.push(label::caption("Koridor yok. Yeni koridor ile bir aksın bölgelerine tip kesit verin.").style(muted));
        }
        let left = column![
            label::strong("Koridorlar"),
            container(scrollable(list).height(typography::scaled(300.0))).style(style::container::field_box).width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let chosen = w.chosen.and_then(|i| w.rows.get(i));
        let right: Element<'_, Message> = match chosen {
            None => container(label::caption("Bir koridor seçin ya da yeni bir koridor oluşturun.")).padding(12).into(),
            Some(r) => self.corridor_editor(w, r, stale_of(r)),
        };
        let saved = chosen.is_some_and(|r| r.def().ok().is_some_and(|d| w.kept.contains(&d)));
        let layers = self.relation_layers();
        let ids: Vec<String> = layers.iter().map(|l| l.0.clone()).collect();
        let layer = Select::new(
            layers.iter().map(|l| Choice::new(l.1.clone())),
            w.layer.as_ref().and_then(|l| ids.iter().position(|i| i == l)),
            move |i| msg(Event::DrawLayer(ids[i].clone())),
        );
        let draw = row![
            label::strong("Şevli plan"),
            words::field("Katman", container(layer).width(200), None),
            words::field("Tarama (m)", kentos_ui::widget::text_field("5", &w.tick).on_input(|t| msg(Event::Tick(t))).width(70), None),
            words::secondary(DRAW, saved.then(|| msg(Event::Draw))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        // The editor scrolls with the body (resized or full screen it grows); the drawing row stays above the footer.
        let body = row![container(left).width(220), container(right).width(Fill)].spacing(16);
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer = row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                words::secondary(REBIND, (saved && chosen.is_some_and(stale_of)).then(|| msg(Event::Rebind))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, Some(msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center);
        Frame::new(TITLE).scroll(body).push(draw).footer(footer).width(1120.0).modal(msg(Event::Cancel))
    }

    fn corridor_editor<'a>(&'a self, w: &'a Window, r: &'a Row, stale: bool) -> Element<'a, Message> {
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let (alignments, profiles, surfaces, supers, templates) = self.corridor_names();
        let pick = |list: Named, now: Option<&String>, none: bool, make: fn(Option<String>) -> Event| {
            let ids: Vec<String> = list.iter().map(|(id, _)| id.clone()).collect();
            let at = now.and_then(|n| ids.iter().position(|id| id == n));
            let choices: Vec<Choice> = if none {
                std::iter::once(Choice::new("Yok".to_owned())).chain(list.iter().map(|(_, n)| Choice::new(n.clone()))).collect()
            } else {
                list.iter().map(|(_, n)| Choice::new(n.clone())).collect()
            };
            let shown = if none { Some(at.map_or(0, |i| i + 1)) } else { at };
            Select::new(choices, shown, move |i| {
                msg(make(if none { (i > 0).then(|| ids[i - 1].clone()) } else { Some(ids[i].clone()) }))
            })
        };
        let on_alignment = pick(
            alignments.iter().map(|a| (a.0.clone(), a.1.clone())).collect(),
            Some(&r.alignment),
            false,
            |id| Event::Alignment(id.unwrap_or_default()),
        );
        let on_profile = pick(
            profiles.into_iter().filter(|p| p.2 == r.alignment).map(|p| (p.0, p.1)).collect(),
            r.profile.as_ref(),
            false,
            Event::Profile,
        );
        let on_surface = pick(surfaces, r.surface.as_ref(), false, Event::Surface);
        let on_super = pick(
            supers.into_iter().filter(|s| s.2 == r.alignment).map(|s| (s.0, s.1)).collect(),
            r.superelevation.as_ref(),
            true,
            Event::Superelevation,
        );
        let field = |hint: &'a str, k: usize, width: f32| {
            kentos_ui::widget::text_field(hint, &r.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width)
        };
        // The picture takes what a resized or full screen window has over (docs/adr/0937 §2).
        let mut col = kentos_ui::widget::grow::Grows::new().spacing(10.0);
        col = col.push(
            row![
                words::field("Ad", kentos_ui::widget::text_field("Koridorun adı", &r.name).on_input(|t| msg(Event::Name(t))), None),
                words::field("Aks", on_alignment, None),
                words::field("Boykesit", on_profile, None),
                words::field("Yüzey", on_surface, None),
                words::field("Dever", on_super, None),
            ]
            .spacing(10),
        );
        col = col.push(
            row![
                words::field("Sol (m)", field("30", 0, 64.0), None),
                words::field("Sağ (m)", field("30", 1, 64.0), None),
                words::field("Aliyman (m)", field("20", 2, 70.0), None),
                words::field("Kurp (m)", field("10", 3, 70.0), None),
                words::field("Sapma (m)", field("", 4, 70.0), None),
                words::field(
                    "Km'ler (m; ile)",
                    kentos_ui::widget::text_field("", &r.stations).on_input(|t| msg(Event::Stations(t))).width(150),
                    None,
                ),
                words::check(r.profile_points, "Profil noktaları", Some(msg(Event::ProfilePoints))),
                words::check(r.critical, "Kritik km'ler", Some(msg(Event::Critical))),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        // How the sampling is densified and checked (docs/adr/0939): the vertical curves by their chord height, the
        // sections between measured against the join of their neighbours.
        col = col.push(
            row![
                words::field("Düşey kiriş (m)", field("", 5, 96.0), None),
                words::field("Plan sapması (m)", field("", 6, 104.0), None),
                words::field("Kot sapması (m)", field("", 7, 104.0), None),
                words::field("En kısa adım (m)", field("", 8, 104.0), None),
                label::caption("Aradaki kesit ölçülür; toleransı aşan aralık en kısa adıma dek ikiye bölünür.").style(muted),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        col = col.push_grow(crate::picture::picture(iced::widget::canvas(Plan { preview: &w.preview }).width(Fill).height(Fill), "Koridor"), typography::scaled(190.0));
        match (&w.preview.why, &w.preview.report) {
            (Some(e), _) => col = col.push(label::caption(e.clone()).style(danger)),
            (None, Some(rep)) => {
                col = col.push(label::caption(format!(
                    "{} km · {} öznitelik çizgisi · {} üçgen · üst yüzey {} m² · çözülmeyen {} aralık",
                    rep.stations.len(),
                    rep.features.len(),
                    rep.triangles,
                    comma(rep.area, 0),
                    rep.unsolved.len()
                )));
                if let Some(sm) = &rep.sampling {
                    col = col.push(label::caption(sampling_words(sm)));
                }
                for wn in w.preview.warnings.iter().filter(|x| x.code != "corridor_stale") {
                    col = col.push(label::caption(wn.message.clone()).style(warn));
                }
            }
            _ => {}
        }
        if stale {
            col = col.push(label::caption("Aks değişti: bölgeleri denetleyip aksa yeniden bağlayın.").style(warn));
        }
        let tids: Vec<String> = templates.iter().map(|t| t.0.clone()).collect();
        let names: Vec<String> = templates.iter().map(|t| t.1.clone()).collect();
        let regions = &r.regions;
        let table = road_dialog::edit_table(
            vec![
                road_dialog::Col { title: "Baş (m)", width: None, number: true },
                road_dialog::Col { title: "Son (m)", width: None, number: true },
                road_dialog::Col { title: "Tip kesit", width: None, number: false },
            ],
            regions.len(),
            move |j, k| {
                let (a, b, t) = &regions[j];
                match k {
                    0 => road_dialog::edit(a, true, move |v| msg(Event::Region(j, 0, v))),
                    1 => road_dialog::edit(b, true, move |v| msg(Event::Region(j, 1, v))),
                    _ => {
                        let ids = tids.clone();
                        road_dialog::pick(names.clone(), t.as_ref().and_then(|t| tids.iter().position(|i| i == t)), move |i| {
                            msg(Event::Template(j, Some(ids[i].clone())))
                        })
                    }
                }
            },
            w.line,
            |j| msg(Event::Line(j)),
            "Bölge yok",
        );
        col = col.push(words::field("Bölgeler", table, None)).push(
            row![words::secondary(ADD, Some(msg(Event::Add))), words::ghost(REMOVE_ROW, w.line.map(|_| msg(Event::RemoveRow)))]
                .spacing(8),
        );
        col = col.push(
            label::caption("Aralıklar ve bölgeler projeden yazılır; KentOS öntanımlı sayı vermez. Bölgeler kesişmez, aralarında boşluk kalabilir.")
                .style(muted),
        );
        container(col).padding(iced::Padding::default().right(14)).into()
    }

    pub(crate) fn corridor_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.corridor_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(REBIND) => Some(msg(Event::Rebind)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE_ROW) => Some(msg(Event::RemoveRow)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(words) => match w.rows.iter().position(|r| r.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("Sol (m)", text) => Some(msg(Event::Field(0, text.to_owned()))),
            Control::Fill("Sağ (m)", text) => Some(msg(Event::Field(1, text.to_owned()))),
            Control::Fill("Aliyman (m)", text) => Some(msg(Event::Field(2, text.to_owned()))),
            Control::Fill("Kurp (m)", text) => Some(msg(Event::Field(3, text.to_owned()))),
            Control::Fill("Sapma (m)", text) => Some(msg(Event::Field(4, text.to_owned()))),
            Control::Fill("Tarama (m)", text) => Some(msg(Event::Tick(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The corridor in plan: its top surface, its feature lines and the stretches not solved, fitted to the canvas.
struct Plan<'a> {
    preview: &'a Preview,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let Some(rep) = &self.preview.report else {
            frame.fill_text(Text {
                content: "Kaynakları, aralıkları ve bölgeleri yazınca koridor burada planda çizilir.".into(),
                position: Point::new(16.0, 20.0),
                color: t.muted,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                ..Text::default()
            });
            return vec![frame.into_geometry()];
        };
        let all = rep.features.iter().flat_map(|f| f.points.iter().map(|p| [p[1], p[2]]));
        let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in all {
            x0 = x0.min(p[0]);
            y0 = y0.min(p[1]);
            x1 = x1.max(p[0]);
            y1 = y1.max(p[1]);
        }
        if !x0.is_finite() {
            return vec![frame.into_geometry()];
        }
        let pad = 12.0;
        let scale = ((bounds.width as f64 - 2.0 * pad) / (x1 - x0).max(1e-6)).min((bounds.height as f64 - 2.0 * pad) / (y1 - y0).max(1e-6));
        let ox = pad + ((bounds.width as f64 - 2.0 * pad) - (x1 - x0) * scale) / 2.0;
        let oy = pad + ((bounds.height as f64 - 2.0 * pad) - (y1 - y0) * scale) / 2.0;
        let at = |x: f64, y: f64| Point::new((ox + (x - x0) * scale) as f32, (bounds.height as f64 - oy - (y - y0) * scale) as f32);
        let fill = Color { a: 0.16, ..t.accent };
        for tri in &self.preview.triangles {
            let path = Path::new(|b| {
                b.move_to(at(tri[0][0], tri[0][1]));
                b.line_to(at(tri[1][0], tri[1][1]));
                b.line_to(at(tri[2][0], tri[2][1]));
                b.close();
            });
            frame.fill(&path, fill);
        }
        let green = Color::from_rgb8(0x16, 0xa3, 0x4a);
        for f in &rep.features {
            let kind = f.code.rsplit('.').next().unwrap_or("");
            let (color, width) = match kind {
                "axis" => (t.danger, 1.6),
                "hinge" => (t.text, 1.4),
                "daylight" => (green, 1.4),
                "ditchToe" | "ditchBottom" => (t.accent, 1.0),
                _ => (t.muted, 0.7),
            };
            let line = Path::new(|b| {
                for (i, p) in f.points.iter().enumerate() {
                    let q = at(p[1], p[2]);
                    if i == 0 { b.move_to(q) } else { b.line_to(q) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(color).with_width(width));
        }
        // The sections across, each from its farthest point on one side to the other's: the stations the check added
        // stronger (docs/adr/0939), the others faint, so where the corridor was densified shows at a glance.
        let mut across: std::collections::BTreeMap<u64, Vec<[f64; 2]>> = std::collections::BTreeMap::new();
        for f in &rep.features {
            for p in &f.points {
                across.entry(p[0].to_bits()).or_default().push([p[1], p[2]]);
            }
        }
        let refined: std::collections::BTreeSet<u64> =
            rep.stations.iter().filter(|s| s.why == "refine").map(|s| s.station.to_bits()).collect();
        for (key, pts) in &across {
            let mut far = (0.0, pts[0], pts[0]);
            for (i, a) in pts.iter().enumerate() {
                for b in &pts[i + 1..] {
                    let d = (a[0] - b[0]).hypot(a[1] - b[1]);
                    if d > far.0 {
                        far = (d, *a, *b);
                    }
                }
            }
            let (color, width) = if refined.contains(key) { (REFINED, 1.2) } else { (Color { a: 0.35, ..t.muted }, 0.6) };
            let line = Path::line(at(far.1[0], far.1[1]), at(far.2[0], far.2[1]));
            frame.stroke(&line, Stroke::default().with_color(color).with_width(width));
        }
        // The stretches not solved, over the axis.
        let orange = Color::from_rgb8(0xea, 0x58, 0x0c);
        if let Some(axis) = rep.features.iter().find(|f| f.code == "axis") {
            for u in &rep.unsolved {
                let pts: Vec<Point> =
                    axis.points.iter().filter(|p| p[0] >= u.from - 1e-6 && p[0] <= u.to + 1e-6).map(|p| at(p[1], p[2])).collect();
                for p in &pts {
                    frame.fill(&Path::circle(*p, 3.0), orange);
                }
            }
        }
        vec![frame.into_geometry()]
    }
}

/// The colour of the sections the sampling check added (docs/adr/0939).
const REFINED: Color = Color::from_rgb8(0x7c, 0x3a, 0xed);

/// The sampling check in words (docs/adr/0939): what was measured and added, the largest strays before and after.
fn sampling_words(sm: &kentos_commands::CorridorSampling) -> String {
    let m = |v: f64| comma(v, 3);
    let changes = match sm.changes {
        0 => String::new(),
        n => format!(" Kesitin değiştiği {n} yer en kısa adımda bulundu."),
    };
    format!(
        "Örnekleme denetimi: {} ara kesit ölçüldü, {} km eklendi (planda mor). En büyük sapma: plan {} → {} m, kot {} → {} m.{changes}",
        sm.measured,
        sm.added,
        m(sm.before.plan),
        m(sm.after.plan),
        m(sm.before.height),
        m(sm.after.height),
    )
}
