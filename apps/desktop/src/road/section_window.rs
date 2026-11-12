//! Enkesit (plan/road.md Y5, `section.editor`): a road project's templates and sample line sets, as Civil 3D's sample
//! line groups and section views and OpenRoads' templates make them: a template's sides (lanes and shoulders with
//! their widths and cross slopes, the cut and fill slopes, a ditch in the cut) seen on a sloping ground as it is typed;
//! a set's sections one by one, the terrain (black) along each sample line, the platform and slopes (red) on the
//! profile's elevation, the daylight points and the cut and fill areas. The sections are built as the set and its
//! template are typed (`kentos.section/1`), so why a side stays open shows before anything is saved.
//!
//! ```text
//! ┌ Enkesit ────────────────────────────────────────────────────────────────────────────────────┐
//! │ Kesit takımları     │ Ad [Enkesitler] Aks [Ana aks ▾] Boykesit [▾] Tip kesit [▾] Yüzey [▾]   │
//! │ ● Enkesitler        │ Sol [25] Sağ [25] Km [Aralık ▾] Baş [100] Son [300] Her [50] ☑ kurp  │
//! │ [Yeni takım]        │ ┌───────────────────────────────────────────────────────────────────┐ │
//! │ Tip kesitler        │ │      ╲___/‾‾‾‾‾\___/  black terrain, red platform and slopes      │ │
//! │ ● Tip 1             │ └───────────────────────────────────────────────────────────────────┘ │
//! │ [Yeni tip kesit]    │ [◀] km 1+150.000 · 2 / 8 [▶]   Yarma 2,84 m² · Dolgu 0,00 m²          │
//! │ Çizime  Katman [▾]  Yer (Y; X) [..]  Satırda [5]  [Çizime yaz]                               │
//! │ [Sil] [Aksa yeniden bağla]                                                [Vazgeç] [Kaydet] │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the definitions; Kaydet writes what changed: the templates first
//! (`road.templates.edit`), then the sets (`road.sections.edit`: removes, sets, the new ones), then the templates
//! removed, each one undo step, and closes. Aksa yeniden bağla rebinds a saved, stale set; Çizime yaz draws a saved
//! set's sections (`road.sections.draw`).

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Center, Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    RoadEditOperation, SectionRow, SectionSetDef, SectionStations, SectionsDraw, SectionsEdit, TemplateComponentDef,
    TemplateComponentKind, TemplateDef, TemplateDitchDef, TemplateSideDef, TemplatesEdit,
};
use kentos_interaction::Level;
use kentos_native_application::{
    ExecutionContext, road_sections, road_sections_draw, road_sections_edit, road_templates_edit,
};
use kentos_road::section::GroundPoint;
use kentos_road::section::template::{self as tpl, Kind};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::road_dialog;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Enkesit";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW_SET: &str = "Yeni takım";
const NEW_TEMPLATE: &str = "Yeni tip kesit";
const REMOVE: &str = "Sil";
const REBIND: &str = "Aksa yeniden bağla";
const ADD: &str = "Bileşen ekle";
const REMOVE_PART: &str = "Bileşeni sil";
const PREVIOUS: &str = "◀";
const NEXT: &str = "▶";
const DRAW: &str = "Çizime yaz";
pub(crate) const PAVEMENT: &str = "Üstyapı var";
pub(crate) const ADD_LAYER: &str = "Tabaka ekle";
const REMOVE_LAYER: &str = "Tabakayı sil";
pub(crate) const EDGE_SLOPE: &str = "Kenar şevi (y/d)";
const SHEETS: &str = "Paftalara diz";
const KINDS: [&str; 2] = ["Şerit", "Banket"];
const METHODS: [&str; 4] = ["Aralık", "Km'ler", "Çizgiler", "Arazi kırıkları"];
/// Kesişen kolları kırp (docs/adr/0933 §3).
pub(crate) const CLIP: &str = "Kesişen kolları kırp";

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A typed number, “,” or “.” as the decimal mark; none when empty.
fn number(text: &str) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| ())
}

/// An optional number: empty is none.
fn may(text: &str, what: &str) -> Result<Option<f64>, String> {
    number(text).map_err(|()| format!("{what} bir sayı olmalı."))
}

fn need(text: &str, what: &str) -> Result<f64, String> {
    match number(text) {
        Ok(Some(v)) => Ok(v),
        _ => Err(format!("{what} bir sayı olmalı.")),
    }
}

/// A component as typed: its kind (0 lane, 1 shoulder), width and cross slope.
#[derive(Debug, Clone, Default)]
pub struct PartRow {
    kind: usize,
    width: String,
    slope: String,
}

/// A side as typed.
#[derive(Debug, Clone, Default)]
pub struct SideRow {
    parts: Vec<PartRow>,
    cut: String,
    fill: String,
    ditch: bool,
    fore: String,
    depth: String,
    bottom: String,
    /// The cut slope's benches (docs/adr/0958): on, their height, width and outward fall (%).
    pub(crate) bench: bool,
    pub(crate) bench_fields: [String; 3],
    /// The fill slope's benches, the same three.
    pub(crate) fill_bench: bool,
    pub(crate) fill_bench_fields: [String; 3],
}

impl SideRow {
    fn of(s: &TemplateSideDef) -> Self {
        Self {
            parts: s
                .components
                .iter()
                .map(|c| PartRow {
                    kind: usize::from(c.kind == TemplateComponentKind::Shoulder),
                    width: trimmed(c.width),
                    slope: road_dialog::percent_text(c.slope),
                })
                .collect(),
            cut: trimmed(s.cut),
            fill: trimmed(s.fill),
            ditch: s.ditch.is_some(),
            fore: s.ditch.as_ref().map_or_else(String::new, |d| trimmed(d.fore)),
            depth: s.ditch.as_ref().map_or_else(String::new, |d| trimmed(d.depth)),
            bottom: s.ditch.as_ref().map_or_else(String::new, |d| trimmed(d.bottom)),
            bench: s.bench.is_some(),
            bench_fields: s.bench.map_or_else(Default::default, |b| [trimmed(b.height), trimmed(b.width), road_dialog::percent_text(b.fall)]),
            fill_bench: s.fill_bench.is_some(),
            fill_bench_fields: s.fill_bench.map_or_else(Default::default, |b| [trimmed(b.height), trimmed(b.width), road_dialog::percent_text(b.fall)]),
        }
    }

    fn def(&self, side: &str) -> Result<TemplateSideDef, String> {
        let mut components = Vec::new();
        for (i, p) in self.parts.iter().enumerate() {
            components.push(TemplateComponentDef {
                kind: if p.kind == 1 { TemplateComponentKind::Shoulder } else { TemplateComponentKind::Lane },
                width: need(&p.width, &format!("{side} yanın {}. bileşeninin genişliği", i + 1))?,
                slope: road_dialog::grade_of(&p.slope)
                    .ok()
                    .flatten()
                    .ok_or_else(|| format!("{side} yanın {}. bileşeninin enine eğimi bir sayı olmalı.", i + 1))?,
            });
        }
        let ditch = if self.ditch {
            Some(TemplateDitchDef {
                fore: need(&self.fore, &format!("{side} yanın hendek iç şevi"))?,
                depth: need(&self.depth, &format!("{side} yanın hendek derinliği"))?,
                bottom: number(&self.bottom).ok().flatten().unwrap_or(0.0),
            })
        } else {
            None
        };
        let benched = |on: bool, f: &[String; 3], what: &str| -> Result<Option<kentos_commands::TemplateBenchDef>, String> {
            if !on {
                return Ok(None);
            }
            Ok(Some(kentos_commands::TemplateBenchDef {
                height: need(&f[0], &format!("{side} yanın {what} basamak yüksekliği"))?,
                width: need(&f[1], &format!("{side} yanın {what} basamak genişliği"))?,
                fall: road_dialog::grade_of(&f[2]).ok().flatten().unwrap_or(0.0),
            }))
        };
        let bench = benched(self.bench, &self.bench_fields, "yarma")?;
        let fill_bench = benched(self.fill_bench, &self.fill_bench_fields, "dolgu")?;
        Ok(TemplateSideDef {
            components,
            cut: need(&self.cut, &format!("{side} yanın yarma şevi"))?,
            fill: need(&self.fill, &format!("{side} yanın dolgu şevi"))?,
            ditch,
            bench,
            fill_bench,
        })
    }
}

/// A template's pavement as typed (docs/adr/0975 §3): its layers (name, thickness, material code) from the top down and
/// its edge, vertical or a slope.
#[derive(Debug, Clone, Default)]
pub struct PavementRow {
    pub(crate) on: bool,
    pub(crate) layers: Vec<[String; 3]>,
    pub(crate) sloped: bool,
    pub(crate) h: String,
}

impl PavementRow {
    fn of(p: Option<&kentos_commands::PavementDef>) -> Self {
        let Some(p) = p else { return Self::default() };
        let (sloped, h) = match p.edge {
            kentos_commands::PavementEdge::Vertical => (false, String::new()),
            kentos_commands::PavementEdge::Slope { h } => (true, trimmed(h)),
        };
        Self {
            on: true,
            layers: p.layers.iter().map(|l| [l.name.clone(), trimmed(l.thickness), l.material.clone().unwrap_or_default()]).collect(),
            sloped,
            h,
        }
    }

    fn def(&self) -> Result<Option<kentos_commands::PavementDef>, String> {
        if !self.on {
            return Ok(None);
        }
        let mut layers = Vec::new();
        for (i, l) in self.layers.iter().enumerate() {
            let name = l[0].trim();
            if name.is_empty() {
                return Err(format!("Üstyapının {}. tabakasına bir ad verin (aşınma, binder, temel …).", i + 1));
            }
            let material = l[2].trim();
            layers.push(kentos_commands::PavementLayerDef {
                name: name.to_owned(),
                thickness: need(&l[1], &format!("{}. tabakanın kalınlığı", i + 1))?,
                material: (!material.is_empty()).then(|| material.to_owned()),
            });
        }
        if layers.is_empty() {
            return Err("Üstyapının en az bir tabakası olmalı: Tabaka ekle ile ekleyin.".into());
        }
        let edge = if self.sloped {
            kentos_commands::PavementEdge::Slope { h: need(&self.h, "üstyapı kenarının şevi")? }
        } else {
            kentos_commands::PavementEdge::Vertical
        };
        Ok(Some(kentos_commands::PavementDef { layers, edge }))
    }
}

/// A template as typed.
#[derive(Debug, Clone, Default)]
pub struct TemplateRow {
    pub(crate) id: String,
    pub(crate) name: String,
    revision: u32,
    left: SideRow,
    right: SideRow,
    /// The right side mirrors the left.
    mirror: bool,
    /// The pavement as typed (docs/adr/0975 §3).
    pub(crate) pavement: PavementRow,
}

impl TemplateRow {
    fn of(t: &TemplateDef) -> Self {
        Self {
            id: t.id.clone(),
            name: t.name.clone(),
            revision: t.revision,
            left: SideRow::of(&t.left),
            right: t.right.as_ref().map_or_else(|| SideRow::of(&t.left), SideRow::of),
            mirror: t.right.is_none(),
            pavement: PavementRow::of(t.pavement.as_ref()),
        }
    }

    fn def(&self) -> Result<TemplateDef, String> {
        Ok(TemplateDef {
            id: self.id.clone(),
            name: self.name.clone(),
            revision: self.revision,
            left: self.left.def("Sol")?,
            right: if self.mirror { None } else { Some(self.right.def("Sağ")?) },
            pavement: self.pavement.def()?,
        })
    }
}

/// A sample line set as typed.
#[derive(Debug, Clone, Default)]
pub struct SetRow {
    pub(crate) id: String,
    pub(crate) name: String,
    revision: u32,
    alignment: String,
    alignment_revision: u32,
    profile: Option<String>,
    template: Option<String>,
    surface: Option<String>,
    superelevation: Option<String>,
    left: String,
    right: String,
    /// 0 a range, 1 the stations written, 2 lines (kept as they are), 3 the terrain's breaks (docs/adr/0933 §3).
    pub(crate) method: usize,
    from: String,
    to: String,
    every: String,
    geometry: bool,
    stations: String,
    uids: Vec<String>,
    /// A range's step on the curves; the breaks' height tolerance and longest gap (docs/adr/0933 §3).
    pub(crate) curve: String,
    pub(crate) tolerance: String,
    pub(crate) most: String,
    /// Lines meeting on a curve's inside shortened to their meeting (docs/adr/0933 §1).
    pub(crate) clip: bool,
}

impl SetRow {
    fn of(d: &SectionSetDef) -> Self {
        let mut r = Self {
            id: d.id.clone(),
            name: d.name.clone(),
            revision: d.revision,
            alignment: d.alignment.clone(),
            alignment_revision: d.alignment_revision,
            profile: d.profile.clone(),
            template: d.template.clone(),
            surface: d.surface.clone(),
            superelevation: d.superelevation.clone(),
            left: trimmed(d.left),
            right: trimmed(d.right),
            clip: d.clip,
            ..Self::default()
        };
        match &d.stations {
            SectionStations::Range { from, to, every, geometry, curve } => {
                (r.from, r.to, r.every, r.geometry) = (trimmed(*from), trimmed(*to), trimmed(*every), *geometry);
                r.curve = curve.map(trimmed).unwrap_or_default();
            }
            SectionStations::Breaks { from, to, tolerance, most, geometry } => {
                r.method = 3;
                (r.from, r.to, r.tolerance, r.geometry) = (trimmed(*from), trimmed(*to), trimmed(*tolerance), *geometry);
                r.most = most.map(trimmed).unwrap_or_default();
            }
            SectionStations::Stations { stations } => {
                r.method = 1;
                r.stations = stations.iter().map(|s| trimmed(*s)).collect::<Vec<_>>().join("; ");
            }
            SectionStations::Lines { uids } => {
                r.method = 2;
                r.uids = uids.clone();
            }
        }
        r
    }

    fn def(&self) -> Result<SectionSetDef, String> {
        let stations = match self.method {
            0 => SectionStations::Range {
                from: need(&self.from, "Km aralığının başı")?,
                to: need(&self.to, "Km aralığının sonu")?,
                every: need(&self.every, "Km aralığı")?,
                geometry: self.geometry,
                curve: may(&self.curve, "Kurpta aralık")?,
            },
            3 => SectionStations::Breaks {
                from: need(&self.from, "Km aralığının başı")?,
                to: need(&self.to, "Km aralığının sonu")?,
                tolerance: need(&self.tolerance, "Kot toleransı")?,
                most: may(&self.most, "En büyük aralık")?,
                geometry: self.geometry,
            },
            1 => {
                let mut list = Vec::new();
                for t in self.stations.split(';').map(str::trim).filter(|t| !t.is_empty()) {
                    list.push(need(t, "Her km")?);
                }
                SectionStations::Stations { stations: list }
            }
            _ => SectionStations::Lines { uids: self.uids.clone() },
        };
        Ok(SectionSetDef {
            id: self.id.clone(),
            name: self.name.clone(),
            alignment: self.alignment.clone(),
            alignment_revision: self.alignment_revision,
            revision: self.revision,
            profile: self.profile.clone(),
            template: self.template.clone(),
            surface: self.surface.clone(),
            superelevation: self.superelevation.clone(),
            left: need(&self.left, "Sol şerit genişliği")?,
            right: need(&self.right, "Sağ şerit genişliği")?,
            stations,
            clip: self.clip,
        })
    }
}

/// What is chosen in the lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chosen {
    Set(usize),
    Template(usize),
}

/// The chosen set's sections as last built, or why they do not come out.
#[derive(Debug, Clone, Default)]
pub struct Preview {
    pub(crate) rows: Vec<SectionRow>,
    pub(crate) why: Option<String>,
    /// The sides open on the swath, as the query's warning says them.
    pub(crate) open: Option<String>,
    /// The lines meeting on a curve's inside, as the query's warning says them (docs/adr/0933 §3).
    pub(crate) crossing: Option<String>,
    /// The sections whose design lies over ground the surface does not give, as the query's warning says them
    /// (docs/adr/1034).
    pub(crate) missing: Option<String>,
    /// What the sections are computed from, as the query says it (docs/adr/1039).
    pub(crate) sources: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) sets: Vec<SetRow>,
    pub(crate) templates: Vec<TemplateRow>,
    kept_sets: Vec<SectionSetDef>,
    kept_templates: Vec<TemplateDef>,
    pub(crate) chosen: Option<Chosen>,
    /// The section shown, its index among the set's.
    pub(crate) at: usize,
    /// The template's side edited: false left, true right.
    right: bool,
    /// The vertical exaggeration's text.
    exaggeration: String,
    pub(crate) preview: Preview,
    /// Çizime yaz: the layer, the sheet's upper left as typed (“Y; X”), the views in a row.
    layer: Option<String>,
    pub(crate) place: String,
    /// Paftalara diz's scale (1:N), as typed; empty: the template's.
    pub(crate) sheet_scale: String,
    columns: String,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Chosen),
    NewSet,
    NewTemplate,
    Remove,
    Rebind,
    Name(String),
    Alignment(String),
    Profile(Option<String>),
    Template(Option<String>),
    Surface(Option<String>),
    /// The superelevation schedule the set's lanes take their slopes from (plan/road.md Y6).
    Superelevation(Option<String>),
    /// The set's field: 0 left, 1 right, 2 from, 3 to, 4 every, 5 the stations written.
    SetField(usize, String),
    Method(usize),
    Geometry,
    /// Kesişen kolları kırp on or off (docs/adr/0933 §3).
    Clip,
    Previous,
    Next,
    Exaggeration(String),
    Side(bool),
    Mirror,
    /// A component's field: 0 width, 1 slope.
    Part(usize, usize, String),
    PartKind(usize, usize),
    Add,
    RemovePart,
    /// The side's end: 0 cut, 1 fill, 2 the ditch's fore slope, 3 its depth, 4 its bottom; 5–7 the cut benches' height,
    /// width and fall, 8–10 the fill benches'.
    End(usize, String),
    Ditch,
    /// The cut slope's benches on or off (docs/adr/0958).
    Bench,
    /// The fill slope's benches on or off.
    FillBench,
    /// The template's pavement on or off (docs/adr/0975 §3).
    Pavement,
    /// The pavement's edge: a slope (true) or vertical.
    PavementSloped(bool),
    PavementSlope(String),
    /// A layer's field: 0 name, 1 thickness, 2 material.
    Layer(usize, usize, String),
    AddLayer,
    RemoveLayer,
    DrawLayer(String),
    Place(String),
    Columns(String),
    Draw,
    /// Paftalara diz: the views laid on sheets (`road.sheets.sections`, docs/adr/0908 §11).
    Sheets,
    /// The section sheets' scale, as typed.
    SheetScale(String),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Sections(event)
}

impl Window {
    fn side_mut(&mut self) -> Option<&mut SideRow> {
        let right = self.right;
        match self.chosen {
            Some(Chosen::Template(i)) => self.templates.get_mut(i).map(|t| if right && !t.mirror { &mut t.right } else { &mut t.left }),
            _ => None,
        }
    }

    fn set_mut(&mut self) -> Option<&mut SetRow> {
        match self.chosen {
            Some(Chosen::Set(i)) => self.sets.get_mut(i),
            _ => None,
        }
    }

    fn template_mut(&mut self) -> Option<&mut TemplateRow> {
        match self.chosen {
            Some(Chosen::Template(i)) => self.templates.get_mut(i),
            _ => None,
        }
    }
}

impl App {
    /// Enkesit: the window on the road project's templates and sample line sets.
    pub(crate) fn open_sections(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Enkesit yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let kept_sets = doc.model.road_sections().to_vec();
        let kept_templates = doc.model.road_templates().to_vec();
        // Docs/adr/0997: Yer empty, the sections go under everything drawn when they are drawn.
        let place = String::new();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        let chosen = if !kept_sets.is_empty() {
            Some(Chosen::Set(0))
        } else {
            (!kept_templates.is_empty()).then_some(Chosen::Template(0))
        };
        self.section_window = Some(Window {
            sets: kept_sets.iter().map(SetRow::of).collect(),
            templates: kept_templates.iter().map(TemplateRow::of).collect(),
            kept_sets,
            kept_templates,
            chosen,
            exaggeration: String::new(),
            layer,
            place,
            sheet_scale: "500".into(),
            columns: "5".into(),
            ..Window::default()
        });
        self.dialog = Some(Dialog::Sections);
        self.refresh_sections();
    }

    /// Kesit takımı oluştur from the ribbon: the window with a new set on the first alignment.
    pub(crate) fn new_section_set(&mut self) {
        self.open_sections();
        if self.section_window.is_some() {
            let _ = self.section_event(Event::NewSet);
        }
    }

    /// Tip kesit oluştur from the ribbon: the window with a new template.
    pub(crate) fn new_template(&mut self) {
        self.open_sections();
        if self.section_window.is_some() {
            let _ = self.section_event(Event::NewTemplate);
        }
    }

    /// The chosen set's sections built again with the templates as typed.
    fn refresh_sections(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.section_window.as_mut() else { return };
        let Some(Chosen::Set(i)) = w.chosen else {
            w.preview = Preview::default();
            return;
        };
        let Some(r) = w.sets.get(i) else { return };
        w.preview = match r.def() {
            Err(e) => Preview { why: Some(e), ..Preview::default() },
            Ok(def) => {
                let typed = def.template.as_ref().and_then(|t| w.templates.iter().find(|q| q.id == *t)).map(TemplateRow::def);
                match typed {
                    Some(Err(e)) => Preview { why: Some(format!("Tip kesit: {e}")), ..Preview::default() },
                    typed => match road_sections::preview(&doc.model, &def, typed.and_then(Result::ok).as_ref()) {
                        Err(e) => Preview { why: Some(e), ..Preview::default() },
                        Ok((rows, warnings)) => Preview {
                            sources: Some(sources_words(&kentos_native_application::road_sections_query::sources_of(&doc.model, &def))),
                            rows,
                            why: warnings.iter().find(|w| w.code == "section_surface_gone").map(|w| w.message.clone()),
                            open: warnings.iter().find(|w| w.code == "section_open").map(|w| w.message.clone()),
                            crossing: warnings.iter().find(|w| w.code == "section_lines_cross").map(|w| w.message.clone()),
                            missing: warnings.iter().find(|w| w.code == "section_ground_missing").map(|w| w.message.clone()),
                        },
                    },
                }
            }
        };
        w.at = w.at.min(w.preview.rows.len().saturating_sub(1));
    }

    fn section_names(&self) -> Names {
        self.document.as_ref().map_or_else(Default::default, |d| {
            let alignments =
                d.model.road_alignments().iter().filter(|a| a.offset_of.is_none()).map(|a| (a.id.clone(), a.name.clone())).collect();
            let profiles =
                d.model.road_profiles().iter().map(|p| (p.id.clone(), p.name.clone(), p.alignment.clone())).collect();
            let surfaces = d
                .model
                .settings()
                .road
                .as_ref()
                .map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect());
            (alignments, profiles, surfaces)
        })
    }

    pub(crate) fn section_event(&mut self, event: Event) -> Task<Message> {
        let (alignments, profiles, surfaces) = self.section_names();
        let revisions: Vec<(String, u32)> = self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model.road_alignments().iter().map(|a| (a.id.clone(), a.revision)).collect()
        });
        let Some(w) = self.section_window.as_mut() else {
            return Task::none();
        };
        let mut warning = None;
        let mut rebuilt = true;
        match event {
            Event::Choose(c) => {
                w.chosen = Some(c);
                w.at = 0;
                w.right = false;
            }
            Event::NewSet => match alignments.first() {
                None => warning = Some("Kesit takımı bir aksın üstünde kurulur: önce Güzergâh'ta bir aks oluşturun."),
                Some((alignment, _)) => {
                    let taken: Vec<String> = w.sets.iter().map(|r| r.id.clone()).collect();
                    let id = (1..).map(|n| format!("k-{n}")).find(|id| !taken.contains(id)).unwrap_or_default();
                    let n = w.sets.len() + 1;
                    w.sets.push(SetRow {
                        id,
                        name: if n == 1 { "Enkesitler".to_owned() } else { format!("Kesit takımı {n}") },
                        alignment: alignment.clone(),
                        alignment_revision: revisions.iter().find(|(a, _)| a == alignment).map_or(0, |(_, r)| *r),
                        profile: profiles.iter().find(|p| p.2 == *alignment).map(|p| p.0.clone()),
                        template: w.templates.first().map(|t| t.id.clone()),
                        surface: surfaces.first().map(|(id, _)| id.clone()),
                        left: "25".into(),
                        right: "25".into(),
                        from: "0".into(),
                        to: String::new(),
                        every: "20".into(),
                        geometry: true,
                        ..SetRow::default()
                    });
                    // The range's end: the alignment's length, read once.
                    if let Some(doc) = &self.document
                        && let Some(a) = doc.model.road_alignments().iter().find(|a| a.id == *alignment)
                        && let Some(Ok(b)) = kentos_native_application::road_alignments::made_alignments(&doc.model)
                            .into_iter()
                            .find(|(d, _)| d.id == a.id)
                            .map(|(_, b)| b)
                        && let Some(r) = w.sets.last_mut()
                    {
                        r.to = trimmed((b.core.length * 1000.0).floor() / 1000.0);
                    }
                    w.chosen = Some(Chosen::Set(w.sets.len() - 1));
                    w.at = 0;
                }
            },
            Event::NewTemplate => {
                let taken: Vec<String> = w.templates.iter().map(|r| r.id.clone()).collect();
                let id = (1..).map(|n| format!("tk-{n}")).find(|id| !taken.contains(id)).unwrap_or_default();
                let n = w.templates.len() + 1;
                // Nothing filled in for the user: the widths, slopes and the end condition come from the project.
                let side = SideRow { parts: vec![PartRow::default()], ..SideRow::default() };
                w.templates.push(TemplateRow {
                    id,
                    name: format!("Tip {n}"),
                    revision: 0,
                    left: side.clone(),
                    right: side,
                    mirror: true,
                    pavement: PavementRow::default(),
                });
                w.chosen = Some(Chosen::Template(w.templates.len() - 1));
                w.right = false;
            }
            Event::Remove => match w.chosen {
                Some(Chosen::Set(i)) if i < w.sets.len() => {
                    w.sets.remove(i);
                    w.chosen = (!w.sets.is_empty()).then_some(Chosen::Set(i.min(w.sets.len().saturating_sub(1))));
                }
                Some(Chosen::Template(i)) if i < w.templates.len() => {
                    let id = w.templates[i].id.clone();
                    let users: Vec<&str> =
                        w.sets.iter().filter(|s| s.template.as_deref() == Some(&id)).map(|s| s.name.as_str()).collect();
                    if users.is_empty() {
                        w.templates.remove(i);
                        w.chosen = (!w.templates.is_empty()).then_some(Chosen::Template(i.min(w.templates.len().saturating_sub(1))));
                    } else {
                        warning = Some("Bu tip kesiti bir kesit takımı kullanıyor; önce takımdan kaldırın.");
                    }
                }
                _ => {}
            },
            Event::Rebind => return self.rebind_set(),
            Event::Name(t) => match w.chosen {
                Some(Chosen::Set(i)) => {
                    if let Some(r) = w.sets.get_mut(i) {
                        r.name = t;
                    }
                }
                Some(Chosen::Template(i)) => {
                    if let Some(r) = w.templates.get_mut(i) {
                        r.name = t;
                    }
                }
                None => {}
            },
            Event::Alignment(id) => {
                let kept = w.kept_sets.iter().map(|k| k.id.clone()).collect::<Vec<_>>();
                if let Some(r) = w.set_mut() {
                    if kept.contains(&r.id) {
                        warning = Some("Kaydedilmiş takımın aksı değişmez; yeni bir takım oluşturun.");
                    } else {
                        r.alignment_revision = revisions.iter().find(|(a, _)| *a == id).map_or(0, |(_, v)| *v);
                        r.profile = profiles.iter().find(|p| p.2 == id).map(|p| p.0.clone());
                        r.alignment = id;
                    }
                }
            }
            Event::Profile(id) => {
                if let Some(r) = w.set_mut() {
                    r.profile = id;
                }
            }
            Event::Template(id) => {
                if let Some(r) = w.set_mut() {
                    r.template = id;
                }
            }
            Event::Surface(id) => {
                if let Some(r) = w.set_mut() {
                    r.surface = id;
                }
            }
            Event::Superelevation(id) => {
                if let Some(r) = w.set_mut() {
                    r.superelevation = id;
                }
            }
            Event::SetField(k, t) => {
                if let Some(r) = w.set_mut() {
                    match k {
                        0 => r.left = t,
                        1 => r.right = t,
                        2 => r.from = t,
                        3 => r.to = t,
                        4 => r.every = t,
                        6 => r.curve = t,
                        7 => r.tolerance = t,
                        8 => r.most = t,
                        _ => r.stations = t,
                    }
                }
            }
            Event::Method(m) => {
                if let Some(r) = w.set_mut() {
                    if m == 2 && r.uids.is_empty() {
                        warning = Some("Çizgilerden km'ler Python ya da yapay zekâ ile verilir (road.sections.edit, lines).");
                    } else {
                        r.method = m;
                    }
                }
            }
            Event::Geometry => {
                if let Some(r) = w.set_mut() {
                    r.geometry = !r.geometry;
                }
            }
            Event::Clip => {
                if let Some(r) = w.set_mut() {
                    r.clip = !r.clip;
                }
            }
            Event::Previous => {
                w.at = w.at.saturating_sub(1);
                rebuilt = false;
            }
            Event::Next => {
                w.at = (w.at + 1).min(w.preview.rows.len().saturating_sub(1));
                rebuilt = false;
            }
            Event::Exaggeration(t) => {
                w.exaggeration = t;
                rebuilt = false;
            }
            Event::Side(right) => {
                w.right = right;
                rebuilt = false;
            }
            Event::Mirror => {
                if let Some(t) = w.template_mut() {
                    t.mirror = !t.mirror;
                    if !t.mirror {
                        t.right = t.left.clone();
                    }
                }
                w.right = false;
            }
            Event::Part(j, k, t) => {
                if let Some(p) = w.side_mut().and_then(|s| s.parts.get_mut(j)) {
                    if k == 0 { p.width = t } else { p.slope = t }
                }
            }
            Event::PartKind(j, kind) => {
                if let Some(p) = w.side_mut().and_then(|s| s.parts.get_mut(j)) {
                    p.kind = kind;
                }
            }
            Event::Add => {
                if let Some(s) = w.side_mut() {
                    s.parts.push(PartRow { kind: 1, ..PartRow::default() });
                }
            }
            Event::RemovePart => {
                if let Some(s) = w.side_mut() {
                    s.parts.pop();
                }
            }
            Event::End(k, t) => {
                if let Some(s) = w.side_mut() {
                    match k {
                        0 => s.cut = t,
                        1 => s.fill = t,
                        2 => s.fore = t,
                        3 => s.depth = t,
                        4 => s.bottom = t,
                        5..=7 => s.bench_fields[k - 5] = t,
                        k => s.fill_bench_fields[(k - 8).min(2)] = t,
                    }
                }
            }
            Event::Ditch => {
                if let Some(s) = w.side_mut() {
                    s.ditch = !s.ditch;
                }
            }
            Event::Bench => {
                if let Some(s) = w.side_mut() {
                    s.bench = !s.bench;
                }
            }
            Event::FillBench => {
                if let Some(s) = w.side_mut() {
                    s.fill_bench = !s.fill_bench;
                }
            }
            Event::Pavement => {
                if let Some(t) = w.template_mut() {
                    t.pavement.on = !t.pavement.on;
                    if t.pavement.on && t.pavement.layers.is_empty() {
                        // Empty rows to fill in: KentOS gives no thickness of its own.
                        t.pavement.layers.push(Default::default());
                    }
                }
            }
            Event::PavementSloped(on) => {
                if let Some(t) = w.template_mut() {
                    t.pavement.sloped = on;
                }
            }
            Event::PavementSlope(v) => {
                if let Some(t) = w.template_mut() {
                    t.pavement.h = v;
                }
            }
            Event::Layer(j, k, v) => {
                if let Some(l) = w.template_mut().and_then(|t| t.pavement.layers.get_mut(j)) {
                    l[k.min(2)] = v;
                }
            }
            Event::AddLayer => {
                if let Some(t) = w.template_mut() {
                    t.pavement.layers.push(Default::default());
                }
            }
            Event::RemoveLayer => {
                if let Some(t) = w.template_mut() {
                    t.pavement.layers.pop();
                }
            }
            Event::DrawLayer(id) => {
                w.layer = Some(id);
                rebuilt = false;
            }
            Event::Place(t) => {
                w.place = t;
                rebuilt = false;
            }
            Event::Columns(t) => {
                w.columns = t;
                rebuilt = false;
            }
            Event::Draw => return self.draw_sections(),
            Event::Sheets => return self.section_sheets(),
            Event::SheetScale(t) => {
                w.sheet_scale = t;
                return Task::none();
            }
            Event::Cancel => {
                self.section_window = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Save => return self.save_sections(),
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        if rebuilt {
            self.refresh_sections();
        }
        Task::none()
    }

    fn chosen_saved_set(&mut self, what: &str) -> Option<SectionSetDef> {
        let w = self.section_window.as_ref()?;
        let Some(Chosen::Set(i)) = w.chosen else { return None };
        let def = w.sets.get(i)?.def().ok();
        if def.as_ref().is_some_and(|d| w.kept_sets.contains(d)) {
            def
        } else {
            self.warn(format!("Önce takımı kaydedin; kaydedilmiş takım {what}."));
            None
        }
    }

    /// Aksa yeniden bağla: the chosen saved set takes its alignment's present revision, one undo step.
    fn rebind_set(&mut self) -> Task<Message> {
        let Some(def) = self.chosen_saved_set("yeniden bağlanır") else { return Task::none() };
        let edit = SectionsEdit {
            operation: RoadEditOperation::Rebind,
            set: Some(def.id.clone()),
            name: None,
            alignment: None,
            profile: None,
            template: None,
            surface: None,
            superelevation: None,
            left: None,
            right: None,
            stations: None,
            clip: None,
            expected_revision: None,
        };
        if self.section_write(|cx| road_sections_edit::execute(cx, edit)) {
            self.reload_sections();
            self.say(Level::Success, "Kesit takımı aksın bugünkü hâline bağlandı.");
        }
        Task::none()
    }

    /// The window's copy taken again from the project, the choice kept.
    fn reload_sections(&mut self) {
        let Some(doc) = &self.document else { return };
        let sets = doc.model.road_sections().to_vec();
        let templates = doc.model.road_templates().to_vec();
        if let Some(w) = self.section_window.as_mut() {
            w.sets = sets.iter().map(SetRow::of).collect();
            w.templates = templates.iter().map(TemplateRow::of).collect();
            w.kept_sets = sets;
            w.kept_templates = templates;
        }
        self.refresh_sections();
    }

    /// The sections' upper left: as typed, or with Yer left empty under the drawing (docs/adr/0997): the views laid at
    /// the origin first (nothing written), then moved down by how far the highest reaches over it, its km row too.
    fn section_place(&mut self, def: &kentos_commands::SectionSetDef) -> Option<[f64; 2]> {
        let w = self.section_window.as_ref()?;
        if !w.place.trim().is_empty() {
            let parts: Vec<Option<f64>> =
                w.place.split([';', ' ']).filter(|s| !s.is_empty()).map(|s| number(s).ok().flatten()).collect();
            return match parts[..] {
                [Some(x), Some(y)] => Some([x, y]),
                _ => None,
            };
        }
        let probe = SectionsDraw {
            set: def.id.clone(),
            layer_id: w.layer.clone().unwrap_or_default(),
            at: [0.0, 0.0],
            stations: Vec::new(),
            columns: number(&w.columns).ok().flatten().filter(|v| *v >= 1.0).map(|v| v as u32),
            exaggeration: number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0),
            height: None,
            expected_revision: None,
        };
        let doc = self.document.as_mut()?;
        let row = 2.4 * 2.5 * doc.model.settings().plot_scale / 1000.0;
        let up = match road_sections_draw::plan(&ExecutionContext::new(&mut doc.model), &probe) {
            kentos_commands::CommandResult::Completed { output, .. } => {
                output.views.iter().map(|v| v.frame[1][1].max(v.frame[0][1])).fold(0.0, f64::max) + row
            }
            _ => row,
        };
        Some(self.under_drawing(up))
    }

    /// Çizime yaz: the chosen saved set's sections drawn on the chosen layer from the typed place, one undo step.
    fn draw_sections(&mut self) -> Task<Message> {
        let Some(def) = self.chosen_saved_set("çizilir") else { return Task::none() };
        let Some([x, y]) = self.section_place(&def) else {
            self.warn("Yeri “Y; X” olarak yazın ya da boş bırakın (çizimin altı).".to_owned());
            return Task::none();
        };
        let Some(w) = &self.section_window else { return Task::none() };
        let input = SectionsDraw {
            set: def.id.clone(),
            layer_id: w.layer.clone().unwrap_or_default(),
            at: [x, y],
            stations: Vec::new(),
            columns: number(&w.columns).ok().flatten().filter(|v| *v >= 1.0).map(|v| v as u32),
            exaggeration: number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0),
            height: None,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_sections_draw::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            let (cut, fill) = out.views.iter().fold((0.0, 0.0), |(c, f), v| (c + v.cut.unwrap_or(0.0), f + v.fill.unwrap_or(0.0)));
            self.say(
                Level::Success,
                format!(
                    "Enkesitler çizildi: {} görünüş, {} nesne; alanların toplamı yarma {} m², dolgu {} m².",
                    out.views.len(),
                    out.uids.len(),
                    comma(cut, 2),
                    comma(fill, 2)
                ),
            );
        }
        Task::none()
    }

    /// Paftalara diz: the chosen saved set's views laid on sheets from the typed place at the typed scale
    /// (`road.sheets.sections`), one undo step; the sheets shown.
    fn section_sheets(&mut self) -> Task<Message> {
        let Some(def) = self.chosen_saved_set("paftalara dizilir") else { return Task::none() };
        let Some([x, y]) = self.section_place(&def) else {
            self.warn("Yeri “Y; X” olarak yazın ya da boş bırakın (çizimin altı).".to_owned());
            return Task::none();
        };
        let Some(w) = &self.section_window else { return Task::none() };
        let scale = match number(&w.sheet_scale) {
            Ok(None) => None,
            Ok(Some(v)) if v >= 1.0 && v.fract() == 0.0 && v <= 1e6 => Some(v as u32),
            _ => {
                self.warn("Paftaların ölçeği 1 ile 1 000 000 arası bir tam sayı olmalı.".to_owned());
                return Task::none();
            }
        };
        let input = kentos_commands::RoadSheetsSections {
            set: def.id.clone(),
            layer_id: w.layer.clone().unwrap_or_default(),
            at: [x, y],
            stations: Vec::new(),
            exaggeration: number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0),
            height: None,
            scale,
            name: None,
            source: None,
            options: serde_json::json!({}),
            first: None,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result =
            kentos_native_application::road_sheets_sections::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(out) = self.answered(result) else { return Task::none() };
        self.say(
            Level::Success,
            format!(
                "“{}”: {} enkesit paftası ({}–{}).",
                out.name,
                out.sheets.len(),
                out.sheets.first().map_or("", |s| s.number.as_str()),
                out.sheets.last().map_or("", |s| s.number.as_str())
            ),
        );
        self.section_window = None;
        self.dialog = None;
        self.follow_sheet_book();
        self.update(Message::Sheet(kentos_sheet_ui::Message::Tab(1)))
    }

    /// Kaydet: the templates written, then the sets (removed, changed, new), then the templates removed.
    fn save_sections(&mut self) -> Task<Message> {
        let Some(w) = self.section_window.clone() else { return Task::none() };
        let mut templates = Vec::new();
        for t in &w.templates {
            match t.def() {
                Ok(d) => templates.push(d),
                Err(e) => {
                    self.warn(format!("“{}”: {e}", t.name));
                    return Task::none();
                }
            }
        }
        let mut sets = Vec::new();
        for s in &w.sets {
            match s.def() {
                Ok(d) => sets.push(d),
                Err(e) => {
                    self.warn(format!("“{}”: {e}", s.name));
                    return Task::none();
                }
            }
        }
        for d in &templates {
            let kept = w.kept_templates.iter().find(|k| k.id == d.id);
            if kept.is_some_and(|k| k.left == d.left && k.right == d.right && k.name == d.name && k.pavement == d.pavement) {
                continue;
            }
            let edit = TemplatesEdit {
                operation: RoadEditOperation::Set,
                template: Some(d.id.clone()),
                name: Some(d.name.clone()),
                left: Some(d.left.clone()),
                right: d.right.clone(),
                mirror: d.right.is_none(),
                remove_pavement: d.pavement.is_none() && kept.is_some_and(|k| k.pavement.is_some()),
                pavement: d.pavement.clone(),
                expected_revision: None,
            };
            if !self.section_write(|cx| road_templates_edit::execute(cx, edit)) {
                return Task::none();
            }
        }
        let blank = |operation, set: &str| SectionsEdit {
            operation,
            set: Some(set.to_owned()),
            name: None,
            alignment: None,
            profile: None,
            template: None,
            surface: None,
            superelevation: None,
            left: None,
            right: None,
            stations: None,
            clip: None,
            expected_revision: None,
        };
        for k in w.kept_sets.iter().filter(|k| sets.iter().all(|d| d.id != k.id)) {
            let edit = blank(RoadEditOperation::Remove, &k.id);
            if !self.section_write(|cx| road_sections_edit::execute(cx, edit)) {
                return Task::none();
            }
        }
        for d in &sets {
            let kept = w.kept_sets.iter().find(|k| k.id == d.id);
            if kept == Some(d) {
                continue;
            }
            let edit = SectionsEdit {
                name: Some(d.name.clone()),
                alignment: (kept.is_none_or(|k| k.alignment != d.alignment)).then(|| d.alignment.clone()),
                profile: Some(d.profile.clone().unwrap_or_default()),
                template: Some(d.template.clone().unwrap_or_default()),
                surface: Some(d.surface.clone().unwrap_or_default()),
                superelevation: Some(d.superelevation.clone().unwrap_or_default()),
                left: Some(d.left),
                right: Some(d.right),
                stations: Some(d.stations.clone()),
                clip: Some(d.clip),
                ..blank(RoadEditOperation::Set, &d.id)
            };
            if !self.section_write(|cx| road_sections_edit::execute(cx, edit)) {
                return Task::none();
            }
        }
        for k in w.kept_templates.iter().filter(|k| templates.iter().all(|d| d.id != k.id)) {
            let edit = TemplatesEdit {
                operation: RoadEditOperation::Remove,
                template: Some(k.id.clone()),
                name: None,
                left: None,
                right: None,
                mirror: false,
                pavement: None,
                remove_pavement: false,
                expected_revision: None,
            };
            if !self.section_write(|cx| road_templates_edit::execute(cx, edit)) {
                return Task::none();
            }
        }
        self.section_window = None;
        self.dialog = None;
        self.say(Level::Success, "Tip kesitler ve kesit takımları kaydedildi.");
        Task::none()
    }

    /// One write; on a refusal the window stays.
    fn section_write<T: serde::Serialize>(
        &mut self,
        write: impl FnOnce(&mut ExecutionContext<'_>) -> kentos_commands::CommandResult<T>,
    ) -> bool {
        let Some(doc) = self.document.as_mut() else { return false };
        let result = write(&mut ExecutionContext::new(&mut doc.model));
        self.answered(result).is_some()
    }

}

/// The sources a set's sections are computed from in a line (docs/adr/1039).
fn sources_words(s: &kentos_commands::SectionSources) -> String {
    let mut words = format!("Kaynaklar: takım {} ({}), aks {} ({})", s.set.id, s.set.revision, s.alignment.id, s.alignment.revision);
    for (what, src) in [("boykesit", &s.profile), ("tip kesit", &s.template), ("dever", &s.superelevation)] {
        if let Some(x) = src {
            words.push_str(&format!(", {what} {} ({})", x.id, x.revision));
        }
    }
    if let Some(g) = &s.surface {
        words.push_str(&format!(", {}", crate::road_dialog::surface_source_words(g)));
    }
    words
}

/// A sloping ground for a template's look, 16 m either side of the axis (from 40 m before docs/adr/0975 §3: the layers
/// were not readable): over the axis on the left, under it on the right.
fn sample_ground_near() -> Vec<GroundPoint> {
    [(-16.0, 101.5 + 4.5 / 28.0 * 4.0), (-12.0, 101.5), (0.0, 100.0), (14.0, 98.0), (16.0, 98.0 - 4.0 / 26.0 * 2.0)]
        .into_iter()
        .map(|(offset, z)| GroundPoint { offset, z: Some(z), gap: false, crossing: false })
        .collect()
}

/// A high cut on the left and a deep fill on the right for a template whose slopes are benched (docs/adr/0958): the
/// ground climbs 7 m between 5 and 8 m out on the left and keeps rising 1 in 3.5; on the right it falls as much and
/// keeps falling as fast, far enough for a benched fill to meet it. A few benches show on either before the slope meets the ground.
pub(crate) fn sample_ground_benched() -> Vec<GroundPoint> {
    [(-36.0, 116.0), (-8.0, 108.0), (-5.0, 101.0), (0.0, 100.0), (5.0, 99.0), (8.0, 92.0), (60.0, 77.14)]
        .into_iter()
        .map(|(offset, z)| GroundPoint { offset, z: Some(z), gap: false, crossing: false })
        .collect()
}

/// Which side the template editor shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SideChoice {
    Left,
    Right,
}

impl std::fmt::Display for SideChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if *self == SideChoice::Left { "Sol yan" } else { "Sağ yan" })
    }
}

/// How the pavement ends at the hinge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeChoice {
    Vertical,
    Slope,
}

impl std::fmt::Display for EdgeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if *self == EdgeChoice::Vertical { "Dik" } else { "Şevli" })
    }
}

/// The core's pavement of a definition (the volumes' conversion, one place).
fn road_volumes_core_pavement(p: &kentos_commands::PavementDef) -> kentos_road::section::pavement::Pavement {
    kentos_native_application::road_volumes::core_pavement(p)
}

/// The alignments (id, name), the profiles (id, name, alignment) and the surfaces (id, name) the lists offer.
type Names = (Vec<(String, String)>, Vec<(String, String, String)>, Vec<(String, String)>);

/// The ground (offset, height, off the surface before), the design line and the daylight points a chart draws.
type Drawn = (Vec<(f64, Option<f64>, bool)>, Vec<[f64; 2]>, Vec<[f64; 2]>);

fn kind_words(k: Kind) -> &'static str {
    match k {
        Kind::Cut => "yarma",
        Kind::Fill => "dolgu",
        Kind::Open => "açık",
    }
}

/// The chart's height.
const CHART_HEIGHT: f32 = 230.0;

/// A section drawn: the terrain, the platform and slopes, the daylight points, the axis and the grids.
/// A section drawn (the split views draw one too, road_views_window.rs).
pub(crate) struct Chart<'a> {
    pub(crate) row: Option<&'a SectionRow>,
    /// A template on the sample ground, for the template's look.
    pub(crate) sample: Option<(Vec<GroundPoint>, tpl::Section)>,
    /// The vertical exaggeration; none fills the height.
    pub(crate) exaggeration: Option<f64>,
    /// The pavement's layer bottoms on the sample (the last the formation); a row draws its own.
    pub(crate) layers: Option<Vec<Vec<[f64; 2]>>>,
}

/// The set's sample lines in plan (docs/adr/0933 §3): every line from its left end to its right, the one shown thick,
/// the ones meeting another (on a curve's inside, lines wider than the radius) in the danger colour; a set that clips
/// draws its lines ending where they meet, so none is red.
struct Plan<'a> {
    rows: &'a [SectionRow],
    at: usize,
}

#[path = "section_window/editors.rs"]
mod editors;

#[path = "section_window/charts.rs"]
mod charts;
