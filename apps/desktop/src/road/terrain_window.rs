//! Arazi yüzeyleri (plan/road.md Y2.4, `terrain.surfaces`): a road project's terrain surfaces, as OpenRoads' Create
//! From Elements and Civil 3D's surface definition make them: the layers whose points and breaklines a surface is made
//! of, a boundary and voids taken from the selection, the edge method's longest outer edge and the height tolerance.
//!
//! ```text
//! ┌ Arazi yüzeyleri ───────────────────────────────────────────────────────────────┐
//! │ Yüzeyler                │ Ad                [Mevcut zemin                     ] │
//! │ ● Mevcut zemin  güncel  │ Nokta katmanları  [x] Zemin  [ ] Kırık hatlar …       │
//! │   Eski zemin    eskidi  │ Kırık hat katmanları [ ] Zemin [x] Kırık hatlar …     │
//! │ [Yeni yüzey]            │ Sınır             1 nesne  [Seçimden al] [Kaldır]     │
//! │                         │ Boşluklar         2 nesne  [Seçimden ekle] [Temizle]  │
//! │                         │ En uzun dış kenar (m) [60]   Kot toleransı (m) [ ]     │
//! │                         │ 24 üçgen · 900,0 m² · kot 100,16–103,13 · 1 kaynak eksik│
//! │ [Yüzeyi sil] [Kabul et]                                   [Vazgeç] [Kaydet]   │
//! └────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the definitions; Kaydet writes what changed through `road.terrains.edit` (removes,
//! sets) and `road.terrains.create` (the new ones) and closes the window. Kabul et accepts a saved surface whose
//! sources changed (`road.terrains.edit accept`). A definition is a project setting, not an undo step. Eğrileri çiz
//! writes a saved surface's contour lines on a layer (`road.terrains.contours`, one undo step).

use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    SurfaceSources, TerrainReport, TerrainState, TerrainSurfaceDef, TerrainsCreate, TerrainsEdit,
    TerrainsEditOperation,
};
use kentos_interaction::Level;
use kentos_native_application::{
    ExecutionContext, road_terrains, road_terrains_contours, road_terrains_create, road_terrains_edit,
};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Arazi yüzeyleri";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni yüzey";
const REMOVE: &str = "Yüzeyi sil";
const ACCEPT: &str = "Kabul et";
const BOUNDARY: &str = "Sınırı seçimden al";
const VOIDS: &str = "Boşlukları seçimden ekle";
const CONTOURS: &str = "Eğrileri çiz";
const LANDXML: &str = "LandXML al…";

/// A definition being edited, its numbers as the text the user typed (empty: none).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    def: TerrainSurfaceDef,
    max_edge: String,
    tolerance: String,
}

impl Row {
    /// Whether it is the project's definition as it is: its fields and the numbers typed.
    fn saved(&self, kept: &[TerrainSurfaceDef]) -> bool {
        kept.contains(&self.def) && *self == Row::of(self.def.clone())
    }

    fn of(def: TerrainSurfaceDef) -> Self {
        let text = |v: Option<f64>| v.map(|t| t.to_string().replace('.', ",")).unwrap_or_default();
        Self {
            max_edge: text(def.max_edge),
            tolerance: text(def.tolerance),
            def,
        }
    }
}

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    rows: Vec<Row>,
    /// The definitions as the project has them, and their reports.
    kept: Vec<TerrainSurfaceDef>,
    reports: Vec<TerrainReport>,
    chosen: Option<usize>,
    /// Eş yükselti eğrileri: the interval, every how many a major line, the layer they go on.
    interval: String,
    major: String,
    layer: Option<String>,
    /// Düzenlemeler (docs/adr/1560): the tool whose pick is awaited, the height typed, and the chosen definition made
    /// now with its edits (what each came to before it is saved).
    pub(crate) picking: Option<crate::terrain_edits::EditTool>,
    /// The right side shows Düzenlemeler instead of the sources.
    pub(crate) edits_pane: bool,
    pub(crate) edit_z: String,
    pub(crate) edit_report: Option<TerrainReport>,
    /// Yüzey yapıştır (docs/adr/3920): the saved surface chosen to paste into the chosen one.
    pub(crate) paste_from: Option<String>,
}

impl Window {
    /// The chosen surface's definition as the window has it.
    pub(crate) fn chosen_def(&self) -> Option<&TerrainSurfaceDef> {
        self.chosen.and_then(|i| self.rows.get(i)).map(|r| &r.def)
    }

    /// The definitions as the project has them (the surfaces a paste may name: docs/adr/3920).
    pub(crate) fn kept(&self) -> &[TerrainSurfaceDef] {
        &self.kept
    }

    /// An edit at the end of the chosen surface's edits.
    pub(crate) fn push_edit(&mut self, edit: kentos_commands::SurfaceEdit) {
        if let Some(r) = self.chosen.and_then(|i| self.rows.get_mut(i)) {
            r.def.edits.push(edit);
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Remove,
    Name(String),
    PointLayer(String),
    BreaklineLayer(String),
    Boundary,
    BoundaryClear,
    Voids,
    VoidsClear,
    MaxEdge(String),
    Tolerance(String),
    Interval(String),
    Major(String),
    ContourLayer(String),
    Contours,
    /// Düzenlemeler (docs/adr/1560): a tool's pick on the drawing, the height typed, the last edit or all taken back.
    EditTool(crate::terrain_edits::EditTool),
    /// Kaynaklar (false) or Düzenlemeler.
    Pane(bool),
    EditZ(String),
    EditUndo,
    EditClear,
    /// Yüzey yapıştır (docs/adr/3920): the saved surface to paste chosen, then pasted (an edit at the end).
    PasteFrom(String),
    Paste,
    /// LandXML al…: the file to take, asked; then the file chosen (none: given up).
    LandXml,
    LandXmlChosen(Option<std::path::PathBuf>),
    Accept,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Terrains(event)
}

fn toggle(list: &mut Vec<String>, id: String) {
    match list.iter().position(|l| *l == id) {
        Some(at) => {
            list.remove(at);
        }
        None => list.push(id),
    }
}

/// A number the user typed, metres with a comma or a point; empty: none.
fn number(text: &str) -> Result<Option<f64>, ()> {
    let text = text.trim().replace(',', ".");
    if text.is_empty() {
        return Ok(None);
    }
    text.parse::<f64>().map(Some).map_err(|_| ())
}

fn state_words(state: TerrainState) -> &'static str {
    match state {
        TerrainState::Current => "güncel",
        TerrainState::Stale => "eskidi",
        TerrainState::New => "kabul edilmedi",
        TerrainState::Unbuildable => "kurulamıyor",
    }
}

/// A report's one line: its figures, or why no surface is made.
fn figures_line(r: &TerrainReport) -> String {
    let fmt = |v: f64, d: usize| format!("{v:.d$}").replace('.', ",");
    let mut parts = Vec::new();
    match (&r.figures, &r.problem) {
        (Some(f), _) => {
            parts.push(format!("{} üçgen", f.triangles));
            parts.push(format!("{} m²", fmt(f.area, 1)));
            if let (Some(lo), Some(hi)) = (f.min_z, f.max_z) {
                parts.push(format!("kot {}–{} m", fmt(lo, 2), fmt(hi, 2)));
            }
        }
        (None, Some(p)) => parts.push(format!("kurulamıyor: {}", p.message)),
        (None, None) => {}
    }
    if !r.left.is_empty() {
        parts.push(format!("{} kaynak kullanılamadı", r.left.len()));
    }
    parts.join(" · ")
}

impl App {
    /// Arazi yüzeyleri: the window on the road project's surfaces.
    pub(crate) fn open_terrains(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if !kentos_native_application::road_terrains::holds_surfaces(doc.model.settings().cad_work) {
            self.warn("Arazi yüzeyi Harita ve Yol projelerinde olur.".to_owned());
            return;
        }
        let made = road_terrains::made_surfaces(&doc.model);
        let kept: Vec<TerrainSurfaceDef> = made.iter().map(|(d, _, _)| d.clone()).collect();
        let layer = self.relation_layers().into_iter().next().map(|(id, _)| id);
        // The selection's own surface's row when every selected object is its source (docs/adr/2480), else the first.
        let picked = self.selection_roles().surface.and_then(|id| kept.iter().position(|d| d.id == id));
        self.terrain_window = Some(Window {
            chosen: picked.or((!kept.is_empty()).then_some(0)),
            rows: kept.iter().cloned().map(Row::of).collect(),
            reports: made.into_iter().map(|(_, r, _)| r).collect(),
            kept,
            interval: contour_interval(doc.model.settings().plot_scale).to_owned(),
            major: "5".to_owned(),
            layer,
            picking: None,
            edits_pane: false,
            edit_z: String::new(),
            edit_report: None,
            paste_from: None,
        });
        self.dialog = Some(Dialog::Terrains);
        self.refresh_edit_report();
    }

    /// Yeni yüzey from the ribbon: the window with a new surface chosen.
    pub(crate) fn new_terrain(&mut self) {
        self.open_terrains();
        if self.terrain_window.is_some() {
            let _ = self.terrain_event(Event::New);
        }
    }

    fn refresh_terrains(&mut self) {
        let Some(doc) = &self.document else { return };
        let made = road_terrains::made_surfaces(&doc.model);
        if let Some(w) = self.terrain_window.as_mut() {
            w.kept = made.iter().map(|(d, _, _)| d.clone()).collect();
            w.reports = made.into_iter().map(|(_, r, _)| r).collect();
        }
    }

    pub(crate) fn terrain_event(&mut self, event: Event) -> Task<Message> {
        let picked = match event {
            Event::Boundary | Event::Voids => self.uids_of(self.selection.ids()),
            _ => Vec::new(),
        };
        let Some(w) = self.terrain_window.as_mut() else {
            return Task::none();
        };
        let chosen = w.chosen.and_then(|i| w.rows.get_mut(i));
        let mut warning = None;
        match event {
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                let taken: Vec<String> = w.rows.iter().map(|r| r.def.id.clone()).collect();
                let id = (1..)
                    .map(|n| format!("t-{n}"))
                    .find(|id| !taken.contains(id))
                    .unwrap_or_default();
                let n = w.rows.len() + 1;
                w.rows.push(Row::of(TerrainSurfaceDef {
                    id,
                    name: if n == 1 { "Mevcut zemin".to_owned() } else { format!("Yüzey {n}") },
                    points: SurfaceSources::default(),
                    breaklines: SurfaceSources::default(),
                    boundary: None,
                    voids: Vec::new(),
                    tolerance: None,
                    max_edge: None,
                    built: None,
                    clouds: Vec::new(),
                    excluded: Vec::new(),
                    edits: Vec::new(),
                }));
                w.chosen = Some(w.rows.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|i| *i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen = (!w.rows.is_empty()).then_some(i.min(w.rows.len().saturating_sub(1)));
                }
            }
            Event::Name(text) => {
                if let Some(r) = chosen {
                    r.def.name = text;
                }
            }
            Event::PointLayer(id) => {
                if let Some(r) = chosen {
                    toggle(&mut r.def.points.layers, id);
                }
            }
            Event::BreaklineLayer(id) => {
                if let Some(r) = chosen {
                    toggle(&mut r.def.breaklines.layers, id);
                }
            }
            Event::Boundary => {
                if let Some(r) = chosen {
                    match picked.as_slice() {
                        [one] => r.def.boundary = Some(one.clone()),
                        _ => warning = Some("Sınır için çizimde tek bir alan ya da kapalı çoklu çizgi seçin."),
                    }
                }
            }
            Event::BoundaryClear => {
                if let Some(r) = chosen {
                    r.def.boundary = None;
                }
            }
            Event::Voids => {
                if let Some(r) = chosen {
                    if picked.is_empty() {
                        warning = Some("Boşluk için çizimde alanlar ya da kapalı çoklu çizgiler seçin.");
                    }
                    for uid in picked {
                        if !r.def.voids.contains(&uid) && r.def.boundary.as_ref() != Some(&uid) {
                            r.def.voids.push(uid);
                        }
                    }
                }
            }
            Event::VoidsClear => {
                if let Some(r) = chosen {
                    r.def.voids.clear();
                }
            }
            Event::MaxEdge(text) => {
                if let Some(r) = chosen {
                    r.max_edge = text;
                }
            }
            Event::Tolerance(text) => {
                if let Some(r) = chosen {
                    r.tolerance = text;
                }
            }
            Event::EditTool(tool) => return self.start_terrain_edit(tool),
            Event::EditZ(text) => w.edit_z = text,
            Event::Pane(edits) => w.edits_pane = edits,
            Event::EditUndo => {
                if let Some(r) = chosen {
                    r.def.edits.pop();
                }
            }
            Event::EditClear => {
                if let Some(r) = chosen {
                    r.def.edits.clear();
                }
            }
            Event::PasteFrom(id) => w.paste_from = Some(id),
            Event::Paste => match (chosen, w.paste_from.clone()) {
                (Some(r), Some(terrain)) if terrain != r.def.id => {
                    r.def.edits.push(kentos_commands::SurfaceEdit::Paste { terrain });
                }
                (Some(_), Some(_)) => warning = Some("Yüzey kendini yapıştıramaz; başka bir yüzey seçin."),
                _ => warning = Some("Yapıştırılacak yüzeyi seçin."),
            },
            Event::Interval(text) => w.interval = text,
            Event::Major(text) => w.major = text,
            Event::ContourLayer(id) => w.layer = Some(id),
            Event::Cancel => {
                self.terrain_window = None;
                self.dialog = None;
            }
            Event::Contours => return self.draw_contours(),
            Event::LandXml => {
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("LandXML al")
                            .add_filter("LandXML (.xml)", &["xml", "landxml"])
                            .pick_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::LandXmlChosen(path)),
                );
            }
            Event::LandXmlChosen(path) => {
                if let Some(path) = path {
                    self.import_landxml(&path);
                }
                return Task::none();
            }
            Event::Accept => return self.accept_terrain(),
            Event::Save => return self.save_terrains(),
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        // The chosen definition made again: what its edits come to (docs/adr/1560).
        self.refresh_edit_report();
        Task::none()
    }

    /// Kabul et: the chosen saved surface accepted as its sources give it now.
    fn accept_terrain(&mut self) -> Task<Message> {
        let Some(w) = &self.terrain_window else {
            return Task::none();
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return Task::none();
        };
        if !r.saved(&w.kept) {
            self.warn("Önce yüzeyi kaydedin; kaydedilmiş yüzey kabul edilir.".to_owned());
            return Task::none();
        }
        let id = r.def.id.clone();
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = road_terrains_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            TerrainsEdit {
                operation: TerrainsEditOperation::Accept,
                surface: None,
                id: Some(id),
                expected_revision: None,
            },
        );
        if self.answered(result).is_some() {
            self.refresh_terrains();
            if let Some(w) = self.terrain_window.as_mut() {
                let kept = w.kept.clone();
                for row in &mut w.rows {
                    if let Some(k) = kept.iter().find(|k| k.id == row.def.id) {
                        row.def.built = k.built.clone();
                    }
                }
            }
            self.say(Level::Success, "Yüzey kabul edildi.");
        }
        Task::none()
    }

    /// Eğrileri çiz: the chosen saved surface's contour lines on the chosen layer, one undo step
    /// (`road.terrains.contours`).
    fn draw_contours(&mut self) -> Task<Message> {
        let Some(w) = &self.terrain_window else {
            return Task::none();
        };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            return Task::none();
        };
        if !r.saved(&w.kept) {
            self.warn("Önce yüzeyi kaydedin; eğriler kaydedilmiş yüzeyden çizilir.".to_owned());
            return Task::none();
        }
        let (Ok(Some(interval)), Ok(major)) = (number(&w.interval), number(&w.major)) else {
            self.warn("Eş yükselti aralığı ve ana eğri sıklığı sayı olmalı.".to_owned());
            return Task::none();
        };
        let input = kentos_commands::TerrainsContours {
            surface: r.def.id.clone(),
            interval,
            base: None,
            major_every: major.map(|m| m.max(0.0) as u32),
            layer_id: w.layer.clone().unwrap_or_default(),
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = road_terrains_contours::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.say(Level::Success, format!("{} eş yükselti eğrisi çizildi.", out.lines.len()));
        }
        Task::none()
    }

    /// LandXML al: the file's first surface on the window's layer, its terrain surface defined and accepted
    /// (`file.import.landxml`); the window then lists it.
    pub(crate) fn import_landxml(&mut self, path: &std::path::Path) {
        let layer = self.terrain_window.as_ref().and_then(|w| w.layer.clone()).unwrap_or_default();
        let input = kentos_commands::ImportLandxml {
            path: path.to_string_lossy().into_owned(),
            layer_id: layer,
            surface: None,
            name: None,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else { return };
        let result = kentos_native_application::file_import_landxml::execute(
            &mut ExecutionContext::new(&mut doc.model),
            input,
        );
        if let Some(out) = self.answered(result) {
            self.refresh_terrains();
            if let Some(w) = self.terrain_window.as_mut() {
                w.rows = w.kept.iter().cloned().map(Row::of).collect();
                w.chosen = (!w.rows.is_empty()).then(|| w.rows.len() - 1);
            }
            let differs = out.differs.map_or(String::new(), |d| {
                format!("; dosyanın üçgenleriyle en büyük kot farkı {} m", format!("{d:.3}").replace('.', ","))
            });
            self.say(
                Level::Success,
                format!(
                    "LandXML “{}”: {} nokta, {} kırık hat alındı, {} üçgenli yüzey kuruldu{differs}.",
                    out.surface,
                    out.points,
                    out.breaklines,
                    out.terrain.figures.as_ref().map_or(0, |f| f.triangles)
                ),
            );
        }
    }

    /// Kaydet: what changed, the removed first, then the changed, then the new.
    fn save_terrains(&mut self) -> Task<Message> {
        let Some(w) = self.terrain_window.clone() else {
            return Task::none();
        };
        let mut defs = Vec::new();
        for r in &w.rows {
            let (Ok(max_edge), Ok(tolerance)) = (number(&r.max_edge), number(&r.tolerance)) else {
                self.warn(format!(
                    "“{}” yüzeyinin en uzun dış kenarı ve kot toleransı sayı olmalı (metre).",
                    r.def.name
                ));
                return Task::none();
            };
            defs.push(TerrainSurfaceDef {
                max_edge,
                tolerance,
                ..r.def.clone()
            });
        }
        let removed: Vec<String> = w
            .kept
            .iter()
            .filter(|k| defs.iter().all(|d| d.id != k.id))
            .map(|k| k.id.clone())
            .collect();
        for id in removed {
            let edit = TerrainsEdit {
                operation: TerrainsEditOperation::Remove,
                surface: None,
                id: Some(id),
                expected_revision: None,
            };
            if !self.terrain_write(|cx| road_terrains_edit::execute(cx, edit)) {
                return Task::none();
            }
        }
        for d in defs {
            match w.kept.iter().find(|k| k.id == d.id) {
                Some(k) if *k == d => {}
                Some(_) => {
                    let edit = TerrainsEdit {
                        operation: TerrainsEditOperation::Set,
                        surface: Some(TerrainSurfaceDef { built: None, ..d }),
                        id: None,
                        expected_revision: None,
                    };
                    if !self.terrain_write(|cx| road_terrains_edit::execute(cx, edit)) {
                        return Task::none();
                    }
                }
                None => {
                    let create = TerrainsCreate {
                        name: d.name,
                        id: Some(d.id),
                        points: d.points,
                        breaklines: d.breaklines,
                        boundary: d.boundary,
                        voids: d.voids,
                        tolerance: d.tolerance,
                        max_edge: d.max_edge,
                        expected_revision: None,
                    };
                    if !self.terrain_write(|cx| road_terrains_create::execute(cx, create)) {
                        return Task::none();
                    }
                }
            }
        }
        self.terrain_window = None;
        self.dialog = None;
        self.say(Level::Success, "Arazi yüzeyleri kaydedildi.");
        Task::none()
    }

    /// One write; on a refusal the window stays with the project's definitions as they are now.
    fn terrain_write<T: serde::Serialize>(
        &mut self,
        write: impl FnOnce(&mut ExecutionContext<'_>) -> kentos_commands::CommandResult<T>,
    ) -> bool {
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = write(&mut ExecutionContext::new(&mut doc.model));
        if self.answered(result).is_none() {
            self.refresh_terrains();
            return false;
        }
        true
    }

    pub(crate) fn terrain_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.terrain_window else {
            return iced::widget::text("").into();
        };
        let report_of = |id: &str| w.reports.iter().find(|r| r.id == id);
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let state = match report_of(&r.def.id) {
                Some(rep) if r.saved(&w.kept) => state_words(rep.state).to_owned(),
                Some(_) => "değişti, kaydedilmedi".to_owned(),
                None => "kaydedilmedi".to_owned(),
            };
            let face = column![
                Elided::new(r.def.name.clone())
                    .size(typography::body())
                    .font(typography::ui())
                    .width(Fill),
                label::caption(state).style(muted),
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
                label::caption("Projede arazi yüzeyi yok. Yeni yüzey ile noktalardan ve kırık hatlardan bir yüzey kurun.")
                    .style(muted),
            );
        }
        let left = column![
            label::strong("Yüzeyler"),
            container(scrollable(list).height(typography::scaled(260.0)))
                .style(style::container::field_box)
                .width(Fill),
            row![
                words::secondary(NEW, Some(msg(Event::New))),
                words::secondary(LANDXML, Some(msg(Event::LandXml))),
            ]
            .spacing(8),
        ]
        .spacing(8);
        let chosen = w.chosen.and_then(|i| w.rows.get(i));
        let right: Element<'_, Message> = match chosen {
            None => container(label::caption("Bir yüzey seçin ya da yeni bir yüzey oluşturun.")).padding(12).into(),
            Some(r) => self.terrain_editor(r, report_of(&r.def.id), r.saved(&w.kept), w),
        };
        let can_accept = chosen.is_some_and(|r| {
            r.saved(&w.kept)
                && report_of(&r.def.id).is_some_and(|rep| matches!(rep.state, TerrainState::Stale | TerrainState::New))
        });
        let ids: Vec<String> = self.relation_layers().into_iter().map(|(id, _)| id).collect();
        let names: Vec<String> = self.relation_layers().into_iter().map(|(_, n)| n).collect();
        let layer = kentos_ui::widget::select::Select::new(
            names.iter().map(|n| kentos_ui::widget::select::Choice::new(n.clone())),
            w.layer.as_ref().and_then(|l| ids.iter().position(|id| id == l)),
            move |i| msg(Event::ContourLayer(ids[i].clone())),
        )
        .placeholder("Katman");
        let saved = chosen.is_some_and(|r| r.saved(&w.kept));
        let contours = row![
            label::strong("Eş yükselti eğrileri"),
            words::field(
                "Aralık (m)",
                kentos_ui::widget::text_field("1", &w.interval).on_input(|t| msg(Event::Interval(t))),
                None
            ),
            words::field(
                "Ana eğri her",
                kentos_ui::widget::text_field("5", &w.major).on_input(|t| msg(Event::Major(t))),
                None
            ),
            words::field("Katman", layer, None),
            words::secondary(CONTOURS, saved.then(|| msg(Event::Contours))),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        // The editor scrolls with the body (resized or full screen it grows); the drawing row stays above the footer.
        let body = row![container(left).width(230), container(right).width(Fill)].spacing(16);
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer = row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                words::secondary(ACCEPT, can_accept.then(|| msg(Event::Accept))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, Some(msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center);
        Frame::new(TITLE).scroll(body).push(contours).footer(footer).width(760.0).modal(msg(Event::Cancel))
    }

    fn terrain_editor<'a>(
        &'a self,
        r: &'a Row,
        report: Option<&'a TerrainReport>,
        saved: bool,
        w: &'a Window,
    ) -> Element<'a, Message> {
        let layers = self.relation_layers();
        let d = &r.def;
        let checks = |chosen: &[String], on: fn(String) -> Event| {
            let mut col = Column::new().spacing(4);
            for (id, name) in &layers {
                col = col.push(words::check(chosen.contains(id), name.clone(), Some(msg(on(id.clone())))));
            }
            col
        };
        let count = |n: usize| if n == 0 { "yok".to_owned() } else { format!("{n} nesne") };
        let boundary = row![
            label::caption(count(usize::from(d.boundary.is_some()))).width(80),
            words::secondary(BOUNDARY, Some(msg(Event::Boundary))),
            words::ghost("Sınırı kaldır", d.boundary.as_ref().map(|_| msg(Event::BoundaryClear))),
        ]
        .spacing(8)
        .align_y(Center);
        let voids = row![
            label::caption(count(d.voids.len())).width(80),
            words::secondary(VOIDS, Some(msg(Event::Voids))),
            words::ghost("Boşlukları temizle", (!d.voids.is_empty()).then(|| msg(Event::VoidsClear))),
        ]
        .spacing(8)
        .align_y(Center);
        let name = kentos_ui::widget::text_field("Yüzeyin adı", &d.name).on_input(|t| msg(Event::Name(t)));
        let max_edge =
            kentos_ui::widget::text_field("yok", &r.max_edge).on_input(|t| msg(Event::MaxEdge(t)));
        let tolerance =
            kentos_ui::widget::text_field("0,001", &r.tolerance).on_input(|t| msg(Event::Tolerance(t)));
        let status = match report {
            Some(rep) if saved => format!("{} · {}", state_words(rep.state), figures_line(rep)),
            Some(_) => "Değişiklik kaydedilince yüzey yeniden kurulur.".to_owned(),
            None => "Kaydedilince kurulur.".to_owned(),
        };
        // Kaynaklar or Düzenlemeler (docs/adr/1560): one at a time, so the edits are in sight without scrolling.
        let pane = kentos_ui::widget::segmented::Segmented::new(
            [Pane(false), Pane(true)],
            Pane(w.edits_pane),
            |p| msg(Event::Pane(p.0)),
        );
        let head = Column::new()
            .spacing(10)
            .padding(iced::Padding::default().right(14))
            .push(words::field("Ad", name, None))
            .push(pane);
        if w.edits_pane {
            return head
                .push(label::caption(status))
                .push(self.terrain_edits_view(
                    d,
                    w.edit_report.as_ref().filter(|rep| rep.id == d.id).or(report.filter(|_| saved)),
                    &w.edit_z,
                ))
                .push(
                    label::caption(
                        "Düzenlemeler yüzeyin tarifindedir: Kaydet onları yazar, yüzey her kurulduğunda sırayla \
                         uygulanır. Bugünkü kaynaklarla yapılamayan düzenleme silinmez, söylenir. Yapıştırılan \
                         yüzeyin üçgenleri olduğu gibi gelir, altında kalan kayıtlar çıkar.",
                    )
                    .style(muted),
                )
                .into();
        }
        head
            .push(words::field("Nokta katmanları", checks(&d.points.layers, Event::PointLayer), None))
            .extend((!d.points.uids.is_empty()).then(|| {
                label::caption(format!("ve kimliğiyle {} nokta nesnesi", d.points.uids.len())).style(muted).into()
            }))
            .push(words::field("Kırık hat katmanları", checks(&d.breaklines.layers, Event::BreaklineLayer), None))
            .extend((!d.breaklines.uids.is_empty()).then(|| {
                label::caption(format!("ve kimliğiyle {} kırık hat nesnesi", d.breaklines.uids.len())).style(muted).into()
            }))
            .push(words::field("Sınır", boundary, None))
            .push(words::field("Boşluklar", voids, None))
            .push(row![
                words::field("En uzun dış kenar (m)", max_edge, None),
                words::field("Kot toleransı (m)", tolerance, None),
            ]
            .spacing(12))
            .push(label::caption(status))
            .push(
                label::caption(
                    "Yüzeyin üçgenleri kaydedilmez: her açılışta kaynaklarından kurulur. Kaynakları değişen yüzey eskir; \
                     Kabul et onu bugünkü hâliyle kabul eder. Kotsuz nokta, yaylı kırık hat ve kapalı olmayan boşluk \
                     kullanılmaz, söylenir.",
                )
                .style(muted),
            )
            .into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn terrain_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.terrain_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ACCEPT) => Some(msg(Event::Accept)),
            Control::Press(BOUNDARY) => Some(msg(Event::Boundary)),
            Control::Press(VOIDS) => Some(msg(Event::Voids)),
            Control::Press(CONTOURS) => Some(msg(Event::Contours)),
            Control::Press(LANDXML) => Some(msg(Event::LandXml)),
            Control::Press("Kaynaklar") => Some(msg(Event::Pane(false))),
            Control::Press("Düzenlemeler") => Some(msg(Event::Pane(true))),
            Control::Press(crate::terrain_edits::UNDO_EDIT) => Some(msg(Event::EditUndo)),
            Control::Press(crate::terrain_edits::CLEAR_EDITS) => Some(msg(Event::EditClear)),
            Control::Press(crate::terrain_edits::PASTE) => Some(msg(Event::Paste)),
            Control::Pick(crate::terrain_edits::PASTE_FROM, name) => match w.kept.iter().find(|k| k.name == name) {
                Some(k) => Some(msg(Event::PasteFrom(k.id.clone()))),
                None => return Err(format!("“{TITLE}” penceresinde “{name}” adlı kayıtlı yüzey yok")),
            },
            Control::Press(words) => match w.rows.iter().position(|r| r.def.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("En uzun dış kenar (m)", text) => Some(msg(Event::MaxEdge(text.to_owned()))),
            Control::Fill("Kot toleransı (m)", text) => Some(msg(Event::Tolerance(text.to_owned()))),
            Control::Fill("Aralık (m)", text) => Some(msg(Event::Interval(text.to_owned()))),
            Control::Fill("Kot (m)", text) => Some(msg(Event::EditZ(text.to_owned()))),

            Control::Fill("Ana eğri her", text) => Some(msg(Event::Major(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// Kaynaklar or Düzenlemeler (`true`), as the segmented control shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pane(bool);

impl std::fmt::Display for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0 { "Düzenlemeler" } else { "Kaynaklar" })
    }
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}

/// The contour interval a window starts with, by the drawing's scale: BÖHHBÜY (RG 26.06.2018/30460) m. 78(1)(c) draws
/// them every 5 m at 1/5000, 2 m at 1/2000, 1 m at 1/1000 and 1/500 (every fifth thick, (d)); a scale it does not name
/// starts at 1 m, which the user changes (docs/adr/1520).
pub(crate) fn contour_interval(scale: f64) -> &'static str {
    if scale == 5000.0 {
        "5"
    } else if scale == 2000.0 {
        "2"
    } else {
        "1"
    }
}
