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
use iced::widget::{Column, column, container, row, scrollable, space};
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
const SHEETS: &str = "Paftalara diz";
const KINDS: [&str; 2] = ["Şerit", "Banket"];
const METHODS: [&str; 3] = ["Aralık", "Km'ler", "Çizgiler"];

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
                    slope: trimmed(c.slope),
                })
                .collect(),
            cut: trimmed(s.cut),
            fill: trimmed(s.fill),
            ditch: s.ditch.is_some(),
            fore: s.ditch.as_ref().map_or_else(String::new, |d| trimmed(d.fore)),
            depth: s.ditch.as_ref().map_or_else(String::new, |d| trimmed(d.depth)),
            bottom: s.ditch.as_ref().map_or_else(String::new, |d| trimmed(d.bottom)),
        }
    }

    fn def(&self, side: &str) -> Result<TemplateSideDef, String> {
        let mut components = Vec::new();
        for (i, p) in self.parts.iter().enumerate() {
            components.push(TemplateComponentDef {
                kind: if p.kind == 1 { TemplateComponentKind::Shoulder } else { TemplateComponentKind::Lane },
                width: need(&p.width, &format!("{side} yanın {}. bileşeninin genişliği", i + 1))?,
                slope: need(&p.slope, &format!("{side} yanın {}. bileşeninin enine eğimi", i + 1))?,
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
        Ok(TemplateSideDef {
            components,
            cut: need(&self.cut, &format!("{side} yanın yarma şevi"))?,
            fill: need(&self.fill, &format!("{side} yanın dolgu şevi"))?,
            ditch,
        })
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
        }
    }

    fn def(&self) -> Result<TemplateDef, String> {
        Ok(TemplateDef {
            id: self.id.clone(),
            name: self.name.clone(),
            revision: self.revision,
            left: self.left.def("Sol")?,
            right: if self.mirror { None } else { Some(self.right.def("Sağ")?) },
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
    /// 0 a range, 1 the stations written, 2 lines (kept as they are).
    method: usize,
    from: String,
    to: String,
    every: String,
    geometry: bool,
    stations: String,
    uids: Vec<String>,
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
            ..Self::default()
        };
        match &d.stations {
            SectionStations::Range { from, to, every, geometry } => {
                (r.from, r.to, r.every, r.geometry) = (trimmed(*from), trimmed(*to), trimmed(*every), *geometry);
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
    place: String,
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
    /// The side's end: 0 cut, 1 fill, 2 the ditch's fore slope, 3 its depth, 4 its bottom.
    End(usize, String),
    Ditch,
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
        // The sheet goes under the alignments, where nothing is drawn yet.
        let (mut x0, mut y0) = (f64::INFINITY, f64::INFINITY);
        for p in doc.model.road_alignments().iter().flat_map(|a| a.pis.iter()) {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
        }
        let place = if x0.is_finite() { format!("{}; {}", trimmed(x0), trimmed(y0 - 600.0)) } else { "0; 0".into() };
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
                            rows,
                            why: warnings.iter().find(|w| w.code == "section_surface_gone").map(|w| w.message.clone()),
                            open: warnings.iter().find(|w| w.code == "section_open").map(|w| w.message.clone()),
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
                        _ => s.bottom = t,
                    }
                }
            }
            Event::Ditch => {
                if let Some(s) = w.side_mut() {
                    s.ditch = !s.ditch;
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

    /// Çizime yaz: the chosen saved set's sections drawn on the chosen layer from the typed place, one undo step.
    fn draw_sections(&mut self) -> Task<Message> {
        let Some(def) = self.chosen_saved_set("çizilir") else { return Task::none() };
        let Some(w) = &self.section_window else { return Task::none() };
        let parts: Vec<Option<f64>> =
            w.place.split([';', ' ']).filter(|s| !s.is_empty()).map(|s| number(s).ok().flatten()).collect();
        let [Some(x), Some(y)] = parts[..] else {
            self.warn("Yeri “Y; X” olarak yazın.".to_owned());
            return Task::none();
        };
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
        let Some(w) = &self.section_window else { return Task::none() };
        let parts: Vec<Option<f64>> =
            w.place.split([';', ' ']).filter(|s| !s.is_empty()).map(|s| number(s).ok().flatten()).collect();
        let [Some(x), Some(y)] = parts[..] else {
            self.warn("Yeri “Y; X” olarak yazın.".to_owned());
            return Task::none();
        };
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
            if kept.is_some_and(|k| k.left == d.left && k.right == d.right && k.name == d.name) {
                continue;
            }
            let edit = TemplatesEdit {
                operation: RoadEditOperation::Set,
                template: Some(d.id.clone()),
                name: Some(d.name.clone()),
                left: Some(d.left.clone()),
                right: d.right.clone(),
                mirror: d.right.is_none(),
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

    pub(crate) fn road_section_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.section_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let (alignments, _, _) = self.section_names();
        let revisions: Vec<(String, u32)> = self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model.road_alignments().iter().map(|a| (a.id.clone(), a.revision)).collect()
        });
        let stale_of = |r: &SetRow| {
            w.kept_sets.iter().any(|k| k.id == r.id)
                && revisions.iter().find(|(a, _)| *a == r.alignment).is_some_and(|(_, v)| *v != r.alignment_revision)
        };
        let item = |title: String, kind: String, on: bool, event: Event| -> Element<'_, Message> {
            let face = column![
                Elided::new(title).size(typography::body()).font(typography::ui()).width(Fill),
                label::caption(kind).style(muted),
            ]
            .spacing(1);
            iced::widget::button(face).on_press(msg(event)).padding([4, 8]).width(Fill).style(style::button::row(on)).into()
        };
        let mut sets = Column::new().spacing(1).padding(2);
        for (i, r) in w.sets.iter().enumerate() {
            let on = alignments.iter().find(|(id, _)| *id == r.alignment).map_or(r.alignment.clone(), |(_, n)| n.clone());
            let kind = if stale_of(r) { format!("{on} · eski") } else { on };
            sets = sets.push(item(r.name.clone(), kind, w.chosen == Some(Chosen::Set(i)), Event::Choose(Chosen::Set(i))));
        }
        if w.sets.is_empty() {
            sets = sets.push(label::caption("Takım yok. Yeni takım ile bir aksın km'lerinde kesit hatları kurun.").style(muted));
        }
        let mut templates = Column::new().spacing(1).padding(2);
        for (i, t) in w.templates.iter().enumerate() {
            let n = t.left.parts.len();
            let kind = format!("{n} bileşen{}", if t.mirror { " · simetrik" } else { " · asimetrik" });
            templates = templates.push(item(t.name.clone(), kind, w.chosen == Some(Chosen::Template(i)), Event::Choose(Chosen::Template(i))));
        }
        if w.templates.is_empty() {
            templates = templates.push(label::caption("Tip kesit yok.").style(muted));
        }
        let left = column![
            label::strong("Kesit takımları"),
            container(scrollable(sets).height(typography::scaled(96.0))).style(style::container::field_box).width(Fill),
            words::secondary(NEW_SET, Some(msg(Event::NewSet))),
            label::strong("Tip kesitler"),
            container(scrollable(templates).height(typography::scaled(84.0))).style(style::container::field_box).width(Fill),
            words::secondary(NEW_TEMPLATE, Some(msg(Event::NewTemplate))),
        ]
        .spacing(8);
        let right: Element<'_, Message> = match w.chosen {
            Some(Chosen::Set(i)) if i < w.sets.len() => {
                scrollable(self.section_set_editor(w, &w.sets[i], stale_of(&w.sets[i]))).height(typography::scaled(360.0)).into()
            }
            Some(Chosen::Template(i)) if i < w.templates.len() => {
                scrollable(section_template_editor(w, &w.templates[i])).height(typography::scaled(360.0)).into()
            }
            _ => container(label::caption("Bir kesit takımı ya da tip kesit seçin ya da yeni birini oluşturun.")).padding(12).into(),
        };
        let saved_set = match w.chosen {
            Some(Chosen::Set(i)) => w.sets.get(i).and_then(|r| r.def().ok()).is_some_and(|d| w.kept_sets.contains(&d)),
            _ => false,
        };
        let stale = matches!(w.chosen, Some(Chosen::Set(i)) if w.sets.get(i).is_some_and(stale_of));
        let body = column![
            row![container(left).width(220), container(right).width(Fill)].spacing(16),
            self.section_draw_row(w, saved_set),
            row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                words::secondary(REBIND, (saved_set && stale).then(|| msg(Event::Rebind))),
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

    fn section_draw_row<'a>(&'a self, w: &'a Window, saved: bool) -> Element<'a, Message> {
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
            words::field("Katman", container(layer).width(220), None),
            words::field("Yer (Y; X)", kentos_ui::widget::text_field("Y; X", &w.place).on_input(|t| msg(Event::Place(t))).width(200), None),
            words::field("Satırda", kentos_ui::widget::text_field("5", &w.columns).on_input(|t| msg(Event::Columns(t))).width(60), None),
            words::secondary(DRAW, saved.then(|| msg(Event::Draw))),
            words::field("Pafta 1:", kentos_ui::widget::text_field("500", &w.sheet_scale).on_input(|t| msg(Event::SheetScale(t))).width(70), None),
            words::secondary(SHEETS, saved.then(|| msg(Event::Sheets))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom)
        .into()
    }

    fn section_set_editor<'a>(&'a self, w: &'a Window, r: &'a SetRow, stale: bool) -> Element<'a, Message> {
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let (alignments, profiles, surfaces) = self.section_names();
        let pick = |list: Vec<(String, String)>, now: Option<&String>, none: bool, make: fn(Option<String>) -> Event| {
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
        let on_alignment = pick(alignments, Some(&r.alignment), false, |id| Event::Alignment(id.unwrap_or_default()));
        let on_profile = pick(
            profiles.into_iter().filter(|p| p.2 == r.alignment).map(|p| (p.0, p.1)).collect(),
            r.profile.as_ref(),
            true,
            Event::Profile,
        );
        let on_template = pick(
            w.templates.iter().map(|t| (t.id.clone(), t.name.clone())).collect(),
            r.template.as_ref(),
            true,
            Event::Template,
        );
        let on_surface = pick(surfaces, r.surface.as_ref(), true, Event::Surface);
        let schedules = self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model
                .road_superelevations()
                .iter()
                .filter(|s| s.alignment == r.alignment)
                .map(|s| (s.id.clone(), s.name.clone()))
                .collect()
        });
        let on_superelevation = pick(schedules, r.superelevation.as_ref(), true, Event::Superelevation);
        let field = |hint: &'a str, value: &'a str, k: usize, width: f32| {
            kentos_ui::widget::text_field(hint, value).on_input(move |t| msg(Event::SetField(k, t))).width(width)
        };
        let mut col = Column::new().spacing(10).padding(iced::Padding::default().right(14));
        col = col.push(
            row![
                words::field("Ad", kentos_ui::widget::text_field("Takımın adı", &r.name).on_input(|t| msg(Event::Name(t))), None),
                words::field("Aks", on_alignment, None),
                words::field("Boykesit", on_profile, None),
                words::field("Tip kesit", on_template, None),
                words::field("Yüzey", on_surface, None),
            ]
            .spacing(10),
        );
        let method = Select::new(
            METHODS.iter().map(|m| Choice::new((*m).to_owned())),
            Some(r.method),
            |m| msg(Event::Method(m)),
        );
        let mut stations = row![
            words::field("Sol (m)", field("25", &r.left, 0, 70.0), None),
            words::field("Sağ (m)", field("25", &r.right, 1, 70.0), None),
            words::field("Dever", container(on_superelevation).width(150), None),
            words::field("Km", method, None),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        stations = match r.method {
            0 => stations
                .push(words::field("Baş (m)", field("0", &r.from, 2, 80.0), None))
                .push(words::field("Son (m)", field("", &r.to, 3, 80.0), None))
                .push(words::field("Her (m)", field("20", &r.every, 4, 60.0), None))
                .push(words::check(r.geometry, "Kurp noktaları ve PVI'ler", Some(msg(Event::Geometry)))),
            1 => stations.push(words::field("Km'ler (m; ile)", field("100; 150", &r.stations, 5, 300.0), None)),
            _ => stations.push(label::caption(format!("{} çizginin aksı kestiği yerler", r.uids.len())).style(muted)),
        };
        col = col.push(stations);
        let shown = w.preview.rows.get(w.at);
        let exaggeration = number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0);
        col = col.push(
            container(
                iced::widget::canvas(Chart { row: shown, sample: None, exaggeration })
                    .width(Fill)
                    .height(typography::scaled(CHART_HEIGHT)),
            )
            .style(style::container::field_box)
            .width(Fill),
        );
        let n = w.preview.rows.len();
        let place = shown.map_or_else(|| "Kesit yok".to_owned(), |s| format!("km {} · {} / {n}", s.km_text, w.at + 1));
        let areas = shown.and_then(|s| Some((s.cut?, s.fill?))).map_or_else(String::new, |(c, f)| {
            format!("Yarma {} m² · Dolgu {} m²", comma(c, 2), comma(f, 2))
        });
        col = col.push(
            row![
                words::secondary(PREVIOUS, (w.at > 0).then(|| msg(Event::Previous))),
                label::body(place),
                words::secondary(NEXT, (w.at + 1 < n).then(|| msg(Event::Next))),
                space::horizontal().width(16),
                label::body(areas),
                space::horizontal().width(Fill),
                words::field(
                    "Düşey abartma",
                    kentos_ui::widget::text_field("oto", &w.exaggeration).on_input(|t| msg(Event::Exaggeration(t))).width(60),
                    None,
                ),
            ]
            .spacing(8)
            .align_y(Center),
        );
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        if let Some(why) = &w.preview.why {
            col = col.push(label::caption(why.clone()).style(danger));
        }
        if let Some(open) = &w.preview.open {
            col = col.push(label::caption(open.clone()).style(warn));
        }
        if stale {
            col = col.push(label::caption("Aks değişti: km'leri denetleyip aksa yeniden bağlayın.").style(warn));
        }
        if r.profile.is_none() || r.template.is_none() || r.surface.is_none() {
            col = col.push(
                label::caption("Platform için takımın boykesiti, tip kesiti ve yüzeyi olmalı; eksikken yalnız arazi çizilir.")
                    .style(muted),
            );
        }
        col.into()
    }

    pub(crate) fn section_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.section_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW_SET) => Some(msg(Event::NewSet)),
            Control::Press(NEW_TEMPLATE) => Some(msg(Event::NewTemplate)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(REBIND) => Some(msg(Event::Rebind)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE_PART) => Some(msg(Event::RemovePart)),
            Control::Press(PREVIOUS) => Some(msg(Event::Previous)),
            Control::Press(NEXT) => Some(msg(Event::Next)),
            Control::Press(DRAW) => Some(msg(Event::Draw)),
            Control::Press(SHEETS) => Some(msg(Event::Sheets)),
            Control::Press(words) => {
                if let Some(i) = w.sets.iter().position(|r| r.name == words) {
                    Some(msg(Event::Choose(Chosen::Set(i))))
                } else if let Some(i) = w.templates.iter().position(|r| r.name == words) {
                    Some(msg(Event::Choose(Chosen::Template(i))))
                } else {
                    return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok"));
                }
            }
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("Sol (m)", text) => Some(msg(Event::SetField(0, text.to_owned()))),
            Control::Fill("Sağ (m)", text) => Some(msg(Event::SetField(1, text.to_owned()))),
            Control::Fill("Düşey abartma", text) => Some(msg(Event::Exaggeration(text.to_owned()))),
            Control::Fill("Yer (Y; X)", text) => Some(msg(Event::Place(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A sloping ground for a template's look: from 6 m over the axis on the left to 6 m under it on the right, 40 m out.
fn sample_ground() -> Vec<GroundPoint> {
    [(-40.0, 106.0), (-12.0, 101.5), (0.0, 100.0), (14.0, 98.0), (40.0, 94.0)]
        .into_iter()
        .map(|(offset, z)| GroundPoint { offset, z: Some(z), gap: false, crossing: false })
        .collect()
}

fn section_template_editor<'a>(w: &'a Window, t: &'a TemplateRow) -> Element<'a, Message> {
    let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
    let mut col = Column::new().spacing(10).padding(iced::Padding::default().right(14));
    let editing_right = w.right && !t.mirror;
    let side = if editing_right { &t.right } else { &t.left };
    col = col.push(
        row![
            words::field("Ad", kentos_ui::widget::text_field("Tip kesitin adı", &t.name).on_input(|v| msg(Event::Name(v))), None),
            words::secondary("Sol yan", (editing_right).then(|| msg(Event::Side(false)))),
            words::secondary("Sağ yan", (!editing_right && !t.mirror).then(|| msg(Event::Side(true)))),
            words::check(t.mirror, "Sağ yan solun aynısı", Some(msg(Event::Mirror))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom),
    );
    // The look on a sloping ground: a cut on the left, a fill on the right.
    let built = t.def().ok().and_then(|d| {
        let core = road_sections::core_template(&d);
        tpl::build(&core, 100.0, &sample_ground()).ok()
    });
    let sample = built.as_ref().map(|s| (sample_ground(), s.clone()));
    let exaggeration = number(&w.exaggeration).ok().flatten().filter(|v| *v > 0.0);
    col = col.push(
        container(
            iced::widget::canvas(Chart { row: None, sample, exaggeration })
                .width(Fill)
                .height(typography::scaled(180.0)),
        )
        .style(style::container::field_box)
        .width(Fill),
    );
    match (&built, t.def()) {
        (_, Err(e)) => col = col.push(label::caption(e).style(muted)),
        (Some(s), _) => {
            col = col.push(label::caption(format!(
                "Örnek arazide: sol {} · sağ {} · yarma {} m² · dolgu {} m²",
                kind_words(s.left.kind),
                kind_words(s.right.kind),
                comma(s.cut, 2),
                comma(s.fill, 2)
            )))
        }
        (None, Ok(_)) => col = col.push(label::caption("Tip kesitin değerleri kurallara uymuyor.").style(muted)),
    }
    let head = ["Bileşen", "Genişlik (m)", "Enine eğim (%)"];
    let widths = [140.0, 120.0, 120.0];
    let mut table = Column::new().spacing(3);
    table = table.push(head.iter().zip(widths).fold(row![].spacing(4), |r, (h, wd)| r.push(label::caption(*h).style(muted).width(wd))));
    for (j, p) in side.parts.iter().enumerate() {
        let kind = Select::new(KINDS.iter().map(|k| Choice::new((*k).to_owned())), Some(p.kind), move |k| msg(Event::PartKind(j, k)));
        table = table.push(
            row![
                container(kind).width(widths[0]),
                kentos_ui::widget::text_field("3,5", &p.width).on_input(move |v| msg(Event::Part(j, 0, v))).width(widths[1]),
                kentos_ui::widget::text_field("-2,5", &p.slope).on_input(move |v| msg(Event::Part(j, 1, v))).width(widths[2]),
            ]
            .spacing(4)
            .align_y(Center),
        );
    }
    let title = if editing_right { "Sağ yanın bileşenleri (eksenden dışa)" } else { "Sol yanın bileşenleri (eksenden dışa)" };
    col = col.push(words::field(title, table, None)).push(
        row![
            words::secondary(ADD, Some(msg(Event::Add))),
            words::ghost(REMOVE_PART, (!side.parts.is_empty()).then(|| msg(Event::RemovePart))),
        ]
        .spacing(8),
    );
    let end = |hint: &'a str, value: &'a str, k: usize| {
        kentos_ui::widget::text_field(hint, value).on_input(move |v| msg(Event::End(k, v))).width(80)
    };
    let mut ends = row![
        words::field("Yarma şevi (y/d)", end("1", &side.cut, 0), None),
        words::field("Dolgu şevi (y/d)", end("1,5", &side.fill, 1), None),
        words::check(side.ditch, "Yarmada hendek", Some(msg(Event::Ditch))),
    ]
    .spacing(10)
    .align_y(iced::alignment::Vertical::Bottom);
    if side.ditch {
        ends = ends
            .push(words::field("İç şev (y/d)", end("2", &side.fore, 2), None))
            .push(words::field("Derinlik (m)", end("0,5", &side.depth, 3), None))
            .push(words::field("Taban (m)", end("0", &side.bottom, 4), None));
    }
    col = col.push(ends).push(
        label::caption("Değerler KGM'nin standardından ya da projeden yazılır; KentOS öntanımlı sayı vermez.").style(muted),
    );
    col.into()
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
struct Chart<'a> {
    row: Option<&'a SectionRow>,
    /// A template on the sample ground, for the template's look.
    sample: Option<(Vec<GroundPoint>, tpl::Section)>,
    /// The vertical exaggeration; none fills the height.
    exaggeration: Option<f64>,
}

impl canvas::Program<Message> for Chart<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        // What is drawn: the ground as (offset, z or none, gap), the design line, the daylight points.
        let (ground, design, daylight): Drawn = match (self.row, &self.sample) {
            (Some(r), _) => (
                r.ground.iter().map(|g| (g.offset, g.z, g.gap)).collect(),
                road_sections_draw::design_line(r),
                r.left.iter().chain(r.right.iter()).filter_map(|s| s.daylight).collect(),
            ),
            (None, Some((g, s))) => (
                g.iter().map(|p| (p.offset, p.z, p.gap)).collect(),
                s.left.points.iter().rev().chain(s.right.points.iter().skip(1)).copied().collect(),
                [s.left.daylight, s.right.daylight].into_iter().flatten().collect(),
            ),
            _ => (Vec::new(), Vec::new(), Vec::new()),
        };
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
        let offsets: Vec<f64> = ground.iter().map(|g| g.0).chain(design.iter().map(|p| p[0])).collect();
        let zs: Vec<f64> = ground.iter().filter_map(|g| g.1).chain(design.iter().map(|p| p[1])).chain(self.row.and_then(|r| r.axis)).collect();
        let (Some(o0), Some(o1), Some(z0), Some(z1)) = (
            offsets.iter().copied().reduce(f64::min),
            offsets.iter().copied().reduce(f64::max),
            zs.iter().copied().reduce(f64::min),
            zs.iter().copied().reduce(f64::max),
        ) else {
            word(&mut frame, "Takımın yüzeyi ve km'leri verilince enkesit burada çizilir.".into(), Point::new(16.0, 20.0), t.muted, false);
            return vec![frame.into_geometry()];
        };
        let pad = ((z1 - z0) * 0.1).max(0.5);
        let (z0, z1) = (z0 - pad, z1 + pad);
        let plot = Rectangle { x: 54.0, y: 10.0, width: bounds.width - 70.0, height: bounds.height - 34.0 };
        let sx = plot.width as f64 / (o1 - o0);
        let sy = match self.exaggeration {
            Some(e) => (sx * e).min(plot.height as f64 / (z1 - z0)),
            None => plot.height as f64 / (z1 - z0),
        };
        // The heights centred in the plot when they do not fill it.
        let lift = 0.5 * (plot.height as f64 - (z1 - z0) * sy);
        let x = |o: f64| plot.x + ((o - o0) * sx) as f32;
        let y = |z: f64| plot.y + plot.height - (lift + (z - z0) * sy) as f32;
        let thin = |c: Color| Stroke::default().with_color(c).with_width(1.0);
        let step = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0].into_iter().find(|g| (z1 - z0) / g <= 6.0).unwrap_or(100.0);
        let (z0, z1) = (z0 - lift / sy, z1 + lift / sy);
        let step = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0].into_iter().find(|g| (z1 - z0) / g <= 6.0).unwrap_or(step);
        let mut z = (z0 / step).ceil() * step;
        while z <= z1 {
            frame.stroke(&Path::line(Point::new(x(o0), y(z)), Point::new(x(o1), y(z))), thin(t.border));
            word(&mut frame, comma(z, if step < 1.0 { 1 } else { 0 }), Point::new(4.0, y(z) - 7.0), t.muted, false);
            z += step;
        }
        let ostep = road_sections_draw::offset_step(o1 - o0);
        let mut o = (o0 / ostep).ceil() * ostep;
        while o <= o1 + 1e-9 {
            frame.stroke(&Path::line(Point::new(x(o), plot.y), Point::new(x(o), plot.y + plot.height)), thin(t.border));
            word(&mut frame, format!("{}", o + 0.0), Point::new(x(o), plot.y + plot.height + 6.0), t.muted, true);
            o += ostep;
        }
        // The axis.
        if o0 <= 0.0 && o1 >= 0.0 {
            frame.stroke(
                &Path::line(Point::new(x(0.0), plot.y), Point::new(x(0.0), plot.y + plot.height)),
                Stroke { line_dash: LineDash { segments: &[5.0, 4.0], offset: 0 }, ..thin(t.accent) },
            );
        }
        let terrain = Path::new(|b| {
            let mut open = false;
            for (o, z, gap) in &ground {
                match z {
                    Some(z) if open && !gap => b.line_to(Point::new(x(*o), y(*z))),
                    Some(z) => {
                        b.move_to(Point::new(x(*o), y(*z)));
                        open = true;
                    }
                    None => open = false,
                }
            }
        });
        frame.stroke(&terrain, Stroke::default().with_color(t.text).with_width(1.4));
        if design.len() > 1 {
            let line = Path::new(|b| {
                for (i, p) in design.iter().enumerate() {
                    let at = Point::new(x(p[0]), y(p[1]));
                    if i == 0 { b.move_to(at) } else { b.line_to(at) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(t.danger).with_width(2.0));
        }
        for d in &daylight {
            frame.fill(&Path::circle(Point::new(x(d[0]), y(d[1])), 3.5), t.danger);
        }
        if let Some(a) = self.row.and_then(|r| r.axis) {
            word(&mut frame, format!("Eksen {}", comma(a, 2)), Point::new(x(0.0) + 6.0, y(a) - 20.0), t.danger, false);
        }
        vec![frame.into_geometry()]
    }
}
