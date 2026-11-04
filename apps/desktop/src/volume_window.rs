//! Kübaj (plan/road.md Y8, `volume.editor`): a road project's earthwork volumes, as Civil 3D's sample line group
//! volumes, OpenRoads' end area volumes report and Netpro's kübaj hesabı make them: a corridor's, by the average end
//! area, the prismoidal formula or between the corridor's top surface and the terrain, with the swell and shrink
//! factors written by stations. The volume is computed as it is typed (`road.volumes.query`'s rule): the mass diagram
//! shows the factored running cut and fill and their difference, the sheet its rows; the warnings say why.
//!
//! ```text
//! ┌ Kübaj ──────────────────────────────────────────────────────────────────────────────────────┐
//! │ Kübaj hesapları     │ Ad [Ana kübaj]  Koridor [▾]  Yöntem [Ortalama alan ▾]                 │
//! │ ● Ana kübaj         │ Baş  Son  Kabarma  Sıkışma  ○      [Satır ekle] [Satırı sil]          │
//! │ [Yeni kübaj]        │ ┌───────────────────────────────────────────────────────────────────┐ │
//! │                     │ [Kütle diyagramı] [Cetvel]                                            │
//! │                     │ │  mass diagram: Σ cut, Σ fill, mass ordinate; or the sheet's rows   │ │
//! │                     │ └───────────────────────────────────────────────────────────────────┘ │
//! │                     │ totals · warnings                                                     │
//! │ [Sil]   Biçim [Excel ▾] [Cetveli yaz…]                                   [Vazgeç] [Kaydet] │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the volumes; Kaydet writes what changed through `road.volumes.edit` (removes, then
//! sets), each one undo step, and closes. Cetveli yaz writes the chosen volume's sheet as typed (an Excel workbook, an
//! OpenDocument spreadsheet or a CSV file, docs/adr/0223's writer). The factors are the user's: nothing is filled in.

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandWarning, RoadEditOperation, VolumeApprovalDef, VolumeBenchDef, VolumeDef, VolumeFactorDef, VolumeMethod, VolumeReport,
    VolumeStripDef, VolumesEdit, WorkbookFormat,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_volumes, road_volumes_approve, road_volumes_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::Elided;
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::road_dialog::{self, RoadDialog};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Kübaj";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni kübaj";
const REMOVE: &str = "Sil";
const ADD: &str = "Satır ekle";
const REMOVE_ROW: &str = "Satırı sil";
const WRITE: &str = "Cetveli yaz…";
const APPROVE: &str = "Onayla";
const REVOKE: &str = "Onayı kaldır";
const ADD_STRIP: &str = "Sıyırma ekle";
const REMOVE_STRIP: &str = "Sıyırmayı sil";
const DIAGRAM: &str = "Kütle diyagramı";
const SHEET: &str = "Cetvel";
const LAYERS: &str = "Tabakalar";
pub(crate) const RULE: &str = "Kural";

/// The rules a volume offers (docs/adr/0930 §2): none (as before the rule) and Netpro's four.
const RULES: [(Option<kentos_commands::VolumeRule>, &str); 5] = [
    (None, "Kuralsız (sıyırma varsa bitkisel toprağa göre)"),
    (Some(kentos_commands::VolumeRule::RoadGround), "Yol–Zemin"),
    (Some(kentos_commands::VolumeRule::FormationGround), "Formasyon–Zemin"),
    (Some(kentos_commands::VolumeRule::RoadTopsoil), "Yol–Bitkisel Toprak"),
    (Some(kentos_commands::VolumeRule::FormationTopsoil), "Formasyon–Bitkisel Toprak"),
];

/// What the window shows under its figures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Shown {
    #[default]
    Diagram,
    Sheet,
    Layers,
}

impl std::fmt::Display for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Shown::Diagram => DIAGRAM,
            Shown::Sheet => SHEET,
            Shown::Layers => LAYERS,
        })
    }
}

const METHODS: [(VolumeMethod, &str); 3] = [
    (VolumeMethod::EndArea, "Ortalama alan"),
    (VolumeMethod::Prismoidal, "Prizmoidal"),
    (VolumeMethod::Surfaces, "Yüzeyler arası"),
];

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A number in Turkish: thousands with points, the fraction after a comma.
fn grouped(v: f64, digits: usize) -> String {
    let text = format!("{:.digits$}", v.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let mut out = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let sign = if v < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0') { "-" } else { "" };
    if fraction.is_empty() { format!("{sign}{out}") } else { format!("{sign}{out},{fraction}") }
}

/// A change with its sign: “+12,5”, “−3,0”.
fn signed(v: f64) -> String {
    if v < -0.05 { format!("−{}", grouped(-v, 1)) } else { format!("+{}", grouped(v.max(0.0), 1)) }
}

fn number(text: &str) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| ())
}

/// An earthwork volume as typed.
#[derive(Debug, Clone)]
pub struct Row {
    pub(crate) id: String,
    pub(crate) name: String,
    revision: u32,
    corridor: Option<String>,
    method: VolumeMethod,
    /// From, to, the cut's and the fill's factor.
    factors: Vec<[String; 4]>,
    /// From, to and the topsoil's depth.
    strip: Vec<[String; 3]>,
    /// The bench's slope (%) and height; both empty: no benching.
    bench: [String; 2],
    /// Its figures as approved, kept as they are.
    approved: Option<VolumeApprovalDef>,
    /// Its rule (docs/adr/0930 §2); none: as before the rule.
    pub(crate) rule: Option<kentos_commands::VolumeRule>,
    /// The curve correction (docs/adr/0934).
    pub(crate) curve: bool,
}

impl Row {
    fn of(d: &VolumeDef) -> Self {
        Self {
            id: d.id.clone(),
            name: d.name.clone(),
            revision: d.revision,
            corridor: Some(d.corridor.clone()),
            method: d.method,
            factors: d
                .factors
                .iter()
                .map(|f| [trimmed(f.from), trimmed(f.to), f.cut.map(trimmed).unwrap_or_default(), f.fill.map(trimmed).unwrap_or_default()])
                .collect(),
            strip: d.strip.iter().map(|f| [trimmed(f.from), trimmed(f.to), trimmed(f.depth)]).collect(),
            bench: d.bench.as_ref().map_or_else(Default::default, |b| [trimmed(b.slope), trimmed(b.height)]),
            approved: d.approved.clone(),
            rule: d.rule,
            curve: d.curve,
        }
    }

    fn def(&self) -> Result<VolumeDef, String> {
        let corridor = self.corridor.clone().ok_or("Bir koridor seçin: kübaj onun kesitlerinden ve üst yüzeyinden hesaplanır.")?;
        let mut factors = Vec::new();
        for (i, f) in self.factors.iter().enumerate() {
            let n = i + 1;
            let need = |t: &str, what: &str| match number(t) {
                Ok(Some(v)) => Ok(v),
                _ => Err(format!("{n}. katsayı satırının {what} bir sayı olmalı.")),
            };
            let may = |t: &str, what: &str| number(t).map_err(|()| format!("{n}. katsayı satırının {what} bir sayı olmalı."));
            factors.push(VolumeFactorDef {
                from: need(&f[0], "başı")?,
                to: need(&f[1], "sonu")?,
                cut: may(&f[2], "kabarması")?,
                fill: may(&f[3], "sıkışması")?,
            });
        }
        let mut strip = Vec::new();
        for (i, f) in self.strip.iter().enumerate() {
            let n = i + 1;
            let need = |t: &str, what: &str| match number(t) {
                Ok(Some(v)) => Ok(v),
                _ => Err(format!("{n}. sıyırma satırının {what} bir sayı olmalı.")),
            };
            strip.push(VolumeStripDef { from: need(&f[0], "başı")?, to: need(&f[1], "sonu")?, depth: need(&f[2], "kalınlığı")? });
        }
        let bench = match (number(&self.bench[0]), number(&self.bench[1])) {
            (Ok(None), Ok(None)) => None,
            (Ok(Some(slope)), Ok(Some(height))) => Some(VolumeBenchDef { slope, height }),
            _ => return Err("Diş kazısı için eğimi (%) ve basamak yüksekliğini sayı olarak birlikte yazın.".into()),
        };
        Ok(VolumeDef {
            id: self.id.clone(),
            name: self.name.clone(),
            corridor,
            revision: self.revision,
            method: self.method,
            factors,
            strip,
            bench,
            approved: self.approved.clone(),
            rule: self.rule,
            curve: self.curve,
        })
    }
}

/// The chosen volume as last computed: its report and warnings, or why it does not compute.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<VolumeReport>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub(crate) rows: Vec<Row>,
    kept: Vec<VolumeDef>,
    pub(crate) chosen: Option<usize>,
    line: Option<usize>,
    strip_line: Option<usize>,
    pub(crate) preview: Preview,
    format: WorkbookFormat,
    /// The sheet shown in place of the mass diagram.
    pub(crate) sheet: bool,
    /// The list shown: the mass diagram, the sheet or the pavement layers.
    pub(crate) shown: Shown,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Remove,
    Name(String),
    Corridor(String),
    Method(VolumeMethod),
    /// The volume's rule (docs/adr/0930 §2).
    Rule(Option<kentos_commands::VolumeRule>),
    /// The list under the figures.
    View(Shown),
    /// A factor row's from (0), to (1), cut (2) or fill (3).
    Factor(usize, usize, String),
    /// A strip row's from (0), to (1) or depth (2).
    Strip(usize, usize, String),
    StripLine(usize),
    AddStrip,
    RemoveStrip,
    /// The bench's slope (0) or height (1).
    Bench(usize, String),
    Line(usize),
    Add,
    RemoveRow,
    Format(WorkbookFormat),
    /// Approve the chosen, saved volume as it computes now (true), or take its approval off (false).
    Approve(bool),
    /// The mass diagram (false) or the sheet (true).
    Show(bool),
    Write,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Volumes(event)
}

fn chosen_row(w: &mut Window) -> Option<&mut Row> {
    w.chosen.and_then(|i| w.rows.get_mut(i))
}

impl App {
    /// Kübaj: the window on the road project's earthwork volumes.
    pub(crate) fn open_volumes(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Kübaj yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let kept = doc.model.road_volumes().to_vec();
        self.volume_window = Some(Box::new(Window {
            chosen: (!kept.is_empty()).then_some(0),
            rows: kept.iter().map(Row::of).collect(),
            kept,
            line: None,
            strip_line: None,
            preview: Preview::default(),
            format: WorkbookFormat::Xlsx,
            sheet: false,
            shown: Shown::Sheet,
        }));
        self.dialog = Some(Dialog::Volumes);
        self.refresh_volume();
    }

    /// Kübaj hesabı oluştur from the ribbon: the window with a new volume of the first corridor.
    pub(crate) fn new_volume(&mut self) {
        self.open_volumes();
        if self.volume_window.is_some() {
            let _ = self.volume_event(Event::New);
        }
    }

    /// Kübaj cetveli from the ribbon: the window on the first volume, to write its sheet.
    pub(crate) fn volume_sheet(&mut self) {
        self.open_volumes();
    }

    fn refresh_volume(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.volume_window.as_mut() else { return };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            w.preview = Preview::default();
            return;
        };
        w.preview = match r.def().and_then(|d| road_volumes::preview(&doc.model, &d)) {
            Err(e) => Preview { why: Some(e), ..Preview::default() },
            Ok((report, warnings)) => Preview { report: Some(report), warnings, why: None },
        };
    }

    /// The project's corridors: id and name.
    fn volume_corridors(&self) -> Vec<(String, String)> {
        self.document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_corridors().iter().map(|c| (c.id.clone(), c.name.clone())).collect())
    }

    pub(crate) fn volume_event(&mut self, event: Event) -> Task<Message> {
        let corridors = self.volume_corridors();
        let Some(w) = self.volume_window.as_mut() else { return Task::none() };
        let mut warning = None;
        match event {
            Event::Choose(i) => {
                w.chosen = Some(i);
                w.line = None;
            }
            Event::New => match corridors.first() {
                None => warning = Some("Kübaj bir koridorun hacmidir: önce Koridor'da bir koridor oluşturun."),
                Some((id, _)) => {
                    let taken: Vec<String> = w.rows.iter().map(|r| r.id.clone()).collect();
                    let new_id = (1..).map(|n| format!("v-{n}")).find(|x| !taken.contains(x)).unwrap_or_default();
                    let n = w.rows.len() + 1;
                    // The first corridor and the average end area; the factors are the user's.
                    w.rows.push(Row {
                        id: new_id,
                        name: if n == 1 { "Kübaj".to_owned() } else { format!("Kübaj {n}") },
                        revision: 0,
                        corridor: Some(id.clone()),
                        method: VolumeMethod::EndArea,
                        factors: Vec::new(),
                        strip: Vec::new(),
                        bench: Default::default(),
                        approved: None,
                        rule: None,
                        curve: false,
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
            Event::Name(t) => {
                if let Some(r) = chosen_row(w) {
                    r.name = t;
                }
            }
            Event::Corridor(id) => {
                let saved = w.chosen.and_then(|i| w.rows.get(i)).is_some_and(|r| w.kept.iter().any(|k| k.id == r.id));
                if saved {
                    warning = Some("Kaydedilmiş kübajın koridoru değişmez; başka koridor için yeni bir kübaj oluşturun.");
                } else if let Some(r) = chosen_row(w) {
                    r.corridor = Some(id);
                }
            }
            Event::Method(m) => {
                if let Some(r) = chosen_row(w) {
                    r.method = m;
                }
            }
            Event::Factor(j, k, t) => {
                if let Some(x) = chosen_row(w).and_then(|r| r.factors.get_mut(j)) {
                    x[k.min(3)] = t;
                }
                w.line = Some(j);
            }
            Event::Line(j) => w.line = Some(j),
            Event::Add => {
                if let Some(r) = chosen_row(w) {
                    // A new row starts where the last ends; its end and factors are the user's.
                    let from = r.factors.last().map(|x| x[1].clone()).unwrap_or_else(|| "0".into());
                    r.factors.push([from, String::new(), String::new(), String::new()]);
                    let n = r.factors.len() - 1;
                    w.line = Some(n);
                }
            }
            Event::Strip(j, k, t) => {
                if let Some(x) = chosen_row(w).and_then(|r| r.strip.get_mut(j)) {
                    x[k.min(2)] = t;
                }
                w.strip_line = Some(j);
            }
            Event::StripLine(j) => w.strip_line = Some(j),
            Event::AddStrip => {
                if let Some(r) = chosen_row(w) {
                    let from = r.strip.last().map(|x| x[1].clone()).unwrap_or_else(|| "0".into());
                    r.strip.push([from, String::new(), String::new()]);
                    let n = r.strip.len() - 1;
                    w.strip_line = Some(n);
                }
            }
            Event::RemoveStrip => {
                if let Some(j) = w.strip_line {
                    if let Some(r) = chosen_row(w).filter(|r| j < r.strip.len()) {
                        r.strip.remove(j);
                    }
                    w.strip_line = None;
                }
            }
            Event::Bench(k, t) => {
                if let Some(r) = chosen_row(w) {
                    r.bench[k.min(1)] = t;
                }
            }
            Event::RemoveRow => {
                if let Some(j) = w.line {
                    if let Some(r) = chosen_row(w).filter(|r| j < r.factors.len()) {
                        r.factors.remove(j);
                    }
                    w.line = None;
                }
            }
            Event::Format(f) => w.format = f,
            Event::Approve(on) => return self.approve_volume(on),
            Event::Show(sheet) => {
                w.sheet = sheet;
                w.shown = if sheet { Shown::Sheet } else { Shown::Diagram };
            }
            Event::View(v) => {
                w.shown = v;
                w.sheet = v == Shown::Sheet;
            }
            Event::Rule(r) => {
                if let Some(row) = chosen_row(w) {
                    row.rule = r;
                }
            }
            Event::Write => return self.write_volume_sheet(),
            Event::Cancel => {
                self.volume_window = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Save => return self.save_volumes(),
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        self.refresh_volume();
        Task::none()
    }

    fn blank_volume_edit(operation: RoadEditOperation, id: &str) -> VolumesEdit {
        VolumesEdit {
            operation,
            volume: Some(id.to_owned()),
            name: None,
            corridor: None,
            method: None,
            factors: None,
            strip: None,
            bench: None,
            rule: None,
            curve: None,
            expected_revision: None,
        }
    }

    fn volume_write(&mut self, edit: VolumesEdit) -> bool {
        let Some(doc) = self.document.as_mut() else { return false };
        let result = road_volumes_edit::execute(&mut ExecutionContext::new(&mut doc.model), edit);
        self.answered(result).is_some()
    }

    /// The chosen volume approved today as it computes, or its approval taken off; then the window reads the saved
    /// volumes again.
    fn approve_volume(&mut self, on: bool) -> Task<Message> {
        let Some(w) = &self.volume_window else { return Task::none() };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else { return Task::none() };
        if !r.def().is_ok_and(|d| w.kept.contains(&d)) {
            self.warn("Önce kübajı kaydedin; kaydedilmiş kübaj onaylanır.".to_owned());
            return Task::none();
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let zone = crate::cloud::local_time::Zone::system();
        let day: String = crate::cloud::local_time::iso_local(i64::try_from(now).unwrap_or(0), zone).chars().take(10).collect();
        let input = kentos_commands::VolumesApprove {
            volume: r.id.clone(),
            revoke: !on,
            at: on.then_some(day),
            note: None,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else { return Task::none() };
        let result = road_volumes_approve::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if self.answered(result).is_some() {
            let kept = doc_volumes(self);
            if let Some(w) = self.volume_window.as_mut() {
                w.rows = kept.iter().map(Row::of).collect();
                w.kept = kept;
            }
            self.refresh_volume();
            self.say(Level::Success, if on { "Kübaj onaylandı." } else { "Kübajın onayı kaldırıldı." });
        }
        Task::none()
    }

    /// The chosen volume's sheet, as typed, written where the user says.
    fn write_volume_sheet(&mut self) -> Task<Message> {
        let Some(w) = &self.volume_window else { return Task::none() };
        let Some(report) = &w.preview.report else {
            let why = w.preview.why.clone().unwrap_or_else(|| "Bir kübaj seçin.".into());
            self.warn(format!("Cetvel yazılamaz: {why}"));
            return Task::none();
        };
        crate::table_write::save(vec![road_volumes::sheet(report)], w.format, format!("{} cetveli", report.name))
    }

    fn save_volumes(&mut self) -> Task<Message> {
        let Some(w) = self.volume_window.clone() else { return Task::none() };
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
            if !self.volume_write(Self::blank_volume_edit(RoadEditOperation::Remove, &k.id)) {
                return Task::none();
            }
        }
        for d in &defs {
            let kept = w.kept.iter().find(|k| k.id == d.id);
            if kept == Some(d) {
                continue;
            }
            let edit = VolumesEdit {
                name: Some(d.name.clone()),
                corridor: kept.is_none().then(|| d.corridor.clone()),
                method: Some(d.method),
                factors: Some(d.factors.clone()),
                strip: Some(d.strip.clone()),
                bench: Some(d.bench.clone()),
                rule: d.rule,
                curve: Some(d.curve),
                ..Self::blank_volume_edit(RoadEditOperation::Set, &d.id)
            };
            if !self.volume_write(edit) {
                return Task::none();
            }
        }
        self.volume_window = None;
        self.dialog = None;
        self.say(Level::Success, "Kübaj hesapları kaydedildi.");
        Task::none()
    }

    pub(crate) fn road_volume_view(&self) -> Element<'_, Message> {
        use kentos_ui::widget::{Banner, Form};
        let Some(w) = &self.volume_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let corridors = self.volume_corridors();
        // The volumes, over the form.
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let on = corridors.iter().find(|c| Some(&c.0) == r.corridor.as_ref()).map_or_else(String::new, |c| c.1.clone());
            let how = METHODS.iter().find(|m| m.0 == r.method).map_or("", |m| m.1);
            let face = column![
                Elided::new(r.name.clone()).size(typography::body()).font(typography::ui()).width(Fill),
                label::caption(format!("{on} · {how}")).style(muted),
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
            list = list.push(label::caption("Kübaj yok. Yeni kübaj ile bir koridorun hacmini hesaplayın.").style(muted));
        }
        let head = column![
            row![label::strong("Kübaj hesapları"), space::horizontal().width(Fill), words::secondary(NEW, Some(msg(Event::New)))]
                .align_y(Center),
            container(scrollable(list).height(typography::scaled(96.0))).style(style::container::field_box).width(Fill),
        ]
        .spacing(6);
        let chosen = w.chosen.and_then(|i| w.rows.get(i));
        let saved = chosen.is_some_and(|r| r.def().is_ok_and(|d| w.kept.contains(&d)));
        let approved = chosen.is_some_and(|r| r.approved.is_some());
        let form = match chosen {
            None => Form::new().row(label::caption("Bir kübaj seçin ya da yeni bir kübaj oluşturun.").style(muted)),
            Some(r) => self.volume_form(w, r, &corridors),
        };
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Cancel))
            .head(head)
            .form_width(400.0)
            .width(1180.0)
            .close_caption(CANCEL)
            .ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove)))
            .ghost(APPROVE, (saved && !approved).then(|| msg(Event::Approve(true))))
            .ghost(REVOKE, approved.then(|| msg(Event::Approve(false))));
        if chosen.is_some() {
            d = d.picture(iced::widget::canvas(Mass { preview: &w.preview }).width(Fill).height(Fill), 190.0);
            match (&w.preview.why, &w.preview.report) {
                (Some(e), _) => d = d.note(Banner::error(e.clone())),
                (None, Some(rep)) => {
                    d = d
                        .legend(rule_words(rep))
                        .figure("Kazı", format!("{} m³", grouped(rep.cut, 1)))
                        .figure("Dolgu", format!("{} m³", grouped(rep.fill, 1)))
                        .figure("Düzeltilmiş fark", format!("{} m³", grouped(rep.factored_cut - rep.factored_fill, 1)))
                        .figure(
                            if rep.layers.is_empty() { "Bitkisel toprak" } else { "Üstyapı" },
                            if rep.layers.is_empty() {
                                rep.strip.map_or_else(|| "—".into(), |v| format!("{} m³", grouped(v, 1)))
                            } else {
                                format!("{} m³", grouped(rep.layers.iter().map(|l| l.volume).sum(), 1))
                            },
                        );
                    if let Some(a) = rep.approval.as_ref() {
                        let note = a.approved.note.as_ref().map_or_else(String::new, |n| format!(" · {n}"));
                        d = d.note(if a.changed {
                            Banner::warning(format!(
                                "Onaylı ({}{note}) hâline göre kazı {} m³, dolgu {} m³; {} aralık değişti.",
                                a.approved.at,
                                signed(a.cut_change),
                                signed(a.fill_change),
                                a.rows.len()
                            ))
                        } else {
                            Banner::info(format!("Onaylı: {}{note}; bugün de aynı hacmi veriyor. Değiştirmek için önce onayı kaldırın.", a.approved.at))
                        });
                    }
                    for wn in &w.preview.warnings {
                        d = d.note(Banner::warning(wn.message.clone()));
                    }
                    let list: Element<'_, Message> = match w.shown {
                        Shown::Layers => layers_table(rep),
                        _ => sheet_table(rep),
                    };
                    let options: Vec<Shown> = if rep.layers.is_empty() { vec![Shown::Sheet] } else { vec![Shown::Sheet, Shown::Layers] };
                    let now = if w.shown == Shown::Layers && !rep.layers.is_empty() { Shown::Layers } else { Shown::Sheet };
                    d = d.detail(road_dialog::detail_choice(options, now, |v| msg(Event::View(v)), list));
                }
                _ => {}
            }
            let formats: Vec<WorkbookFormat> = crate::table_write::FORMATS.iter().map(|c| c.format).collect();
            let format = Select::new(
                crate::table_write::FORMATS.iter().map(|c| Choice::new(c.label.to_owned())),
                formats.iter().position(|f| *f == w.format),
                move |i| msg(Event::Format(formats[i])),
            );
            d = d.tail(
                row![
                    label::caption("Kübaj cetveli").style(muted),
                    container(format).width(240),
                    words::secondary(WRITE, w.preview.report.as_ref().map(|_| msg(Event::Write))),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        d.primary(SAVE, Some(msg(Event::Save))).view()
    }

    /// The chosen volume's definition as a grouped form (DESIGN.md §7.19).
    fn volume_form<'a>(&'a self, w: &'a Window, r: &'a Row, corridors: &[(String, String)]) -> kentos_ui::widget::Form<'a, Message> {
        use kentos_ui::widget::Form;
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let ids: Vec<String> = corridors.iter().map(|c| c.0.clone()).collect();
        let on_corridor = Select::new(
            corridors.iter().map(|c| Choice::new(c.1.clone())),
            r.corridor.as_ref().and_then(|c| ids.iter().position(|i| i == c)),
            move |i| msg(Event::Corridor(ids[i].clone())),
        )
        .placeholder("Koridor");
        let on_method = Select::new(
            METHODS.iter().map(|m| Choice::new(m.1.to_owned())),
            METHODS.iter().position(|m| m.0 == r.method),
            |i| msg(Event::Method(METHODS[i].0)),
        );
        let on_rule = Select::new(RULES.iter().map(|m| Choice::new(m.1.to_owned())), RULES.iter().position(|m| m.0 == r.rule), |i| {
            msg(Event::Rule(RULES[i].0))
        });
        let sections = r.method != VolumeMethod::Surfaces;
        let head = |words: &[&'a str], widths: &[f32]| {
            words.iter().zip(widths).fold(row![].spacing(4), |row, (h, wd)| row.push(label::caption(*h).style(muted).width(*wd)))
        };
        let widths = [70.0, 70.0, 70.0, 70.0];
        let mut factors = Column::new().spacing(3).push(head(&["Baş (m)", "Son (m)", "Kabarma", "Sıkışma"], &widths));
        for (j, f) in r.factors.iter().enumerate() {
            let cell = |k: usize, hint: &'a str| {
                kentos_ui::widget::text_field(hint, &f[k]).on_input(move |v| msg(Event::Factor(j, k, v))).width(widths[k])
            };
            factors = factors.push(
                row![
                    cell(0, "0"),
                    cell(1, ""),
                    cell(2, ""),
                    cell(3, ""),
                    iced::widget::button(label::caption(if w.line == Some(j) { "●" } else { "○" }))
                        .on_press(msg(Event::Line(j)))
                        .padding([2, 6])
                        .style(style::button::row(w.line == Some(j))),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
        let swidths = [70.0, 70.0, 80.0];
        let mut strips = Column::new().spacing(3).push(head(&["Baş (m)", "Son (m)", "Kalınlık (m)"], &swidths));
        for (j, f) in r.strip.iter().enumerate() {
            let cell = |k: usize, hint: &'a str| {
                kentos_ui::widget::text_field(hint, &f[k]).on_input(move |v| msg(Event::Strip(j, k, v))).width(swidths[k])
            };
            strips = strips.push(
                row![
                    cell(0, "0"),
                    cell(1, ""),
                    cell(2, ""),
                    iced::widget::button(label::caption(if w.strip_line == Some(j) { "●" } else { "○" }))
                        .on_press(msg(Event::StripLine(j)))
                        .padding([2, 6])
                        .style(style::button::row(w.strip_line == Some(j))),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
        let bench_field = |k: usize, hint: &'a str| kentos_ui::widget::text_field(hint, &r.bench[k]).on_input(move |v| msg(Event::Bench(k, v)));
        let mut form = Form::new()
            .label_width(110.0)
            .section("Tanım")
            .field("Ad", kentos_ui::widget::text_field("Kübajın adı", &r.name).on_input(|t| msg(Event::Name(t))))
            .field("Koridor", on_corridor)
            .field("Yöntem", on_method)
            .field(RULE, on_rule)
            .help("NETPRO'nun adlarıyla: Yol ya da Formasyon, Zemin ya da Bitkisel Toprak. Formasyon üstyapı ister.")
            .section("Kabarma ve sıkışma")
            .wide(factors)
            .wide(
                row![words::secondary(ADD, Some(msg(Event::Add))), words::ghost(REMOVE_ROW, w.line.map(|_| msg(Event::RemoveRow)))]
                    .spacing(8),
            )
            .help("Katsayılar zemin raporundan yazılır; KentOS öntanımlı sayı vermez.");
        if sections {
            form = form
                .section("Sıyırma ve diş kazısı")
                .wide(strips)
                .wide(
                    row![
                        words::secondary(ADD_STRIP, Some(msg(Event::AddStrip))),
                        words::ghost(REMOVE_STRIP, w.strip_line.map(|_| msg(Event::RemoveStrip))),
                    ]
                    .spacing(8),
                )
                .field("Diş eğimi (%)", bench_field(0, ""))
                .field("Basamak (m)", bench_field(1, ""))
                .help("Arazinin enine eğimi bu yüzdeden dikken dolgunun altında basamak kazılır.");
        } else {
            form = form.row(label::caption("Sıyırma ve diş kazısı yalnız kesit yöntemleriyle hesaplanır.").style(muted));
        }
        form
    }

    pub(crate) fn volume_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.volume_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE_ROW) => Some(msg(Event::RemoveRow)),
            Control::Press(WRITE) => Some(msg(Event::Write)),
            Control::Press(APPROVE) => Some(msg(Event::Approve(true))),
            Control::Press(REVOKE) => Some(msg(Event::Approve(false))),
            Control::Press(ADD_STRIP) => Some(msg(Event::AddStrip)),
            Control::Press(REMOVE_STRIP) => Some(msg(Event::RemoveStrip)),
            Control::Press(DIAGRAM) => Some(msg(Event::Show(false))),
            Control::Press(SHEET) => Some(msg(Event::Show(true))),
            Control::Press(LAYERS) => Some(msg(Event::View(Shown::Layers))),
            Control::Press(words) => match w.rows.iter().position(|r| r.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => match METHODS.iter().find(|m| m.1 == words) {
                    Some(m) => Some(msg(Event::Method(m.0))),
                    None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
                },
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The road project's volumes as saved.
fn doc_volumes(app: &App) -> Vec<VolumeDef> {
    app.document.as_ref().map_or_else(Vec::new, |d| d.model.road_volumes().to_vec())
}

/// The line under the mass diagram (the diagram has its own legend): the rule the volume measures by.
fn rule_words(rep: &VolumeReport) -> String {
    let rule = match rep.rule {
        Some(r) => r.words().to_owned(),
        None if rep.strip.is_some() => "Yol–Bitkisel Toprak (kuralsız)".to_owned(),
        None => "Yol–Zemin (kuralsız)".to_owned(),
    };
    format!("Kural: {rule}")
}

/// The pavement layers' volumes (docs/adr/0930 §2): each layer's name, material and volume in all.
fn layers_table(rep: &VolumeReport) -> Element<'_, Message> {
    road_dialog::table(
        vec![road_dialog::col("Tabaka", 200.0), road_dialog::col("Malzeme kodu", 140.0), road_dialog::num("Hacim (m³)", 120.0)],
        rep.layers
            .iter()
            .map(|l| vec![l.name.clone(), l.material.clone().unwrap_or_default(), grouped(l.volume, 2)])
            .collect(),
        "Tip kesitlerde üstyapı yok",
    )
}

/// The sheet's rows as the window shows them: km, cut and fill, the factors, the running totals and the mass ordinate.
fn sheet_table(rep: &VolumeReport) -> Element<'_, Message> {
    let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
    let (strips, benches) = (rep.strip.is_some(), rep.bench.is_some());
    let mut head: Vec<(&str, f32)> = vec![("Baş km", 84.0), ("Son km", 84.0), ("Kazı", 70.0), ("Dolgu", 70.0)];
    if strips {
        head.push(("B. toprak", 70.0));
    }
    if benches {
        head.push(("Diş", 56.0));
    }
    head.extend([("Kab.", 40.0), ("Sık.", 40.0), ("Σ kazı", 80.0), ("Σ dolgu", 80.0), ("Kütle", 80.0)]);
    let widths: Vec<f32> = head.iter().map(|h| h.1).collect();
    let header = head.iter().fold(row![].spacing(6), |row, (h, wd)| row.push(label::caption(*h).style(muted).width(*wd)));
    let mut rows = Column::new().spacing(2);
    let opt = |v: Option<f64>, d: usize| v.map_or_else(|| "—".to_owned(), |x| grouped(x, d));
    for r in &rep.rows {
        let mut cells = vec![r.from_text.clone(), r.to_text.clone(), opt(r.cut, 1), opt(r.fill, 1)];
        if strips {
            cells.push(opt(r.strip, 1));
        }
        if benches {
            cells.push(opt(r.bench, 1));
        }
        cells.extend([
            r.cut_factor.map_or_else(|| "—".into(), |x| trimmed(x).replace('.', ",")),
            r.fill_factor.map_or_else(|| "—".into(), |x| trimmed(x).replace('.', ",")),
            grouped(r.total_cut, 1),
            grouped(r.total_fill, 1),
            grouped(r.mass, 1),
        ]);
        rows = rows.push(cells.into_iter().zip(widths.iter().copied()).fold(row![].spacing(6), |row, (c, wd)| row.push(label::caption(c).width(wd))));
    }
    column![header, scrollable(rows).height(typography::scaled(146.0))].spacing(4).padding([6, 8]).into()
}

/// What a line of the mass diagram reads of a row.
type Value = fn(&kentos_commands::VolumeRow) -> f64;

/// The mass diagram: the factored running cut and fill and the mass ordinate along the stations.
struct Mass<'a> {
    preview: &'a Preview,
}

impl canvas::Program<Message> for Mass<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let caption = |frame: &mut canvas::Frame, words: String, at: Point, color: Color| {
            frame.fill_text(Text {
                content: words,
                position: at,
                color,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                ..Text::default()
            });
        };
        let Some(rep) = self.preview.report.as_ref().filter(|r| !r.rows.is_empty()) else {
            caption(&mut frame, "Koridoru ve yöntemi seçince kütle diyagramı burada çizilir.".into(), Point::new(16.0, 20.0), t.muted);
            return vec![frame.into_geometry()];
        };
        let (s0, s1) = (rep.rows[0].from, rep.rows[rep.rows.len() - 1].to);
        let mut lo: f64 = 0.0;
        let mut hi: f64 = 0.0;
        for r in &rep.rows {
            for v in [r.total_cut, r.total_fill, r.mass] {
                lo = lo.min(v);
                hi = hi.max(v);
            }
        }
        if hi - lo < 1e-9 {
            hi = lo + 1.0;
        }
        let (left, pad) = (64.0, 12.0);
        let (wd, ht) = (bounds.width as f64 - left - pad, bounds.height as f64 - 2.0 * pad - 14.0);
        let at = |s: f64, v: f64| {
            Point::new((left + (s - s0) / (s1 - s0).max(1e-9) * wd) as f32, (pad + (hi - v) / (hi - lo) * ht) as f32)
        };
        for k in 0..=4 {
            let v = lo + (hi - lo) * f64::from(k) / 4.0;
            let a = at(s0, v);
            frame.stroke(&Path::line(a, at(s1, v)), Stroke::default().with_color(Color { a: 0.35, ..t.border }).with_width(1.0));
            caption(&mut frame, grouped(v, 0), Point::new(4.0, a.y - 7.0), t.muted);
        }
        frame.stroke(&Path::line(at(s0, 0.0), at(s1, 0.0)), Stroke::default().with_color(t.muted).with_width(1.0));
        let green = Color::from_rgb8(0x16, 0xa3, 0x4a);
        let series: [(Value, Color); 3] = [(|r| r.total_cut, t.danger), (|r| r.total_fill, t.accent), (|r| r.mass, green)];
        for (k, (value, color)) in series.iter().enumerate() {
            let line = Path::new(|b| {
                b.move_to(at(s0, 0.0));
                for r in &rep.rows {
                    b.line_to(at(r.to, value(r)));
                }
            });
            let width = if k == 2 { 1.4 } else { 2.0 };
            frame.stroke(&line, Stroke::default().with_color(*color).with_width(width));
        }
        // The stretches without volume, under the axis.
        let orange = Color::from_rgb8(0xea, 0x58, 0x0c);
        for g in &rep.gaps {
            let (a, b) = (at(g[0].max(s0), lo), at(g[1].min(s1), lo));
            frame.fill_rectangle(Point::new(a.x, a.y + 2.0), iced::Size::new((b.x - a.x).max(2.0), 4.0), orange);
        }
        let mut x = left as f32 + 8.0;
        for (words, color) in [("Σ kazı (gevşek)", t.danger), ("Σ dolgu (sıkışık)", t.accent), ("Kütle ordinatı", green)] {
            frame.fill_rectangle(Point::new(x, bounds.height - 12.0), iced::Size::new(14.0, 3.0), color);
            caption(&mut frame, words.into(), Point::new(x + 18.0, bounds.height - 18.0), t.text);
            x += 140.0;
        }
        vec![frame.into_geometry()]
    }
}
