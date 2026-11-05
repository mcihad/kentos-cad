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
/// The uncertainty's list (docs/adr/0941).
pub(crate) const BUDGET: &str = "Belirsizlik";
/// The uncertainty's fields: the heights' σ, their shift, the share accepted.
pub(crate) const UNCERTAINTY: [&str; 3] = ["Kot σ (m)", "Sistematik kayma (m)", "Kabul payı (%)"];
/// The progress's list and its fields (docs/adr/0942): the survey now and the one before it.
pub(crate) const PROGRESS: &str = "Dönem metrajı";
pub(crate) const SURVEY: &str = "Ara ölçü";
pub(crate) const PREVIOUS: &str = "Önceki ölçü";
/// The comparison's list and form section (docs/adr/0943).
pub(crate) const COMPARE: &str = "Karşılaştırma";
pub(crate) const ALTERNATIVES: &str = "Alternatifler";
/// The ledger's lists and form fields (docs/adr/0949).
pub(crate) const LEDGER: &str = "Defter";
pub(crate) const LARGEST: &str = "En büyük aralıklar";
pub(crate) const LEDGER_STEP: &str = "Dilim (m)";
pub(crate) const LEDGER_BY: &str = "Gruplama";
pub(crate) const ROUNDING: &str = "Yuvarlama";
const GROUPINGS: [&str; 2] = ["Km dilimleri", "Bölgeler"];
const DECIMALS: [&str; 4] = ["Tam sayı", "1 basamak", "2 basamak", "3 basamak"];
/// The previous survey's “none”.
const NO_PREVIOUS: &str = "Yok (ilk araziden bu yana)";

/// The curve correction's switch (docs/adr/0934 §3).
pub(crate) const CURVE: &str = "Kesitlerin ağırlık merkeziyle düzelt";

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
    /// The uncertainty budget (docs/adr/0941).
    Budget,
    /// The work done and left by a survey (docs/adr/0942).
    Progress,
    /// The chosen volume against its alternatives (docs/adr/0943).
    Compare,
    /// The total by km, region and material, and the largest stretches (docs/adr/0949).
    Ledger,
    Largest,
}

impl std::fmt::Display for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Shown::Diagram => DIAGRAM,
            Shown::Sheet => SHEET,
            Shown::Layers => LAYERS,
            Shown::Budget => BUDGET,
            Shown::Progress => PROGRESS,
            Shown::Compare => COMPARE,
            Shown::Ledger => LEDGER,
            Shown::Largest => LARGEST,
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
    /// The uncertainty asked (docs/adr/0941): σ, the shift, the share accepted; the window's, not the volume's.
    pub(crate) uncertainty: [String; 3],
    /// The budget read on the volume as typed, when σ is written.
    pub(crate) budget: Option<Result<(kentos_commands::VolumeUncertaintyReport, Vec<CommandWarning>), String>>,
    /// The survey now and the one before it (docs/adr/0942): terrain surfaces' ids; the window's, not the volume's.
    pub(crate) survey: Option<String>,
    pub(crate) previous: Option<String>,
    /// The progress read on the volume as typed, when a survey is chosen.
    pub(crate) progress: Option<Result<(kentos_commands::VolumeProgressReport, Vec<CommandWarning>), String>>,
    /// The saved volumes compared with the chosen one (docs/adr/0943), and the comparison with each one's alignment
    /// drawn in plan.
    pub(crate) alternatives: Vec<String>,
    pub(crate) comparison: Option<Result<(kentos_commands::VolumesCompared, Vec<CommandWarning>), String>>,
    pub(crate) traces: Vec<kentos_native_application::road_volumes_compare::Trace>,
    /// The ledger asked (docs/adr/0949): the km step, by regions instead, the rounding's decimals; read while shown.
    pub(crate) ledger_step: String,
    pub(crate) ledger_regions: bool,
    pub(crate) ledger_decimals: u32,
    pub(crate) ledger: Option<Result<(kentos_commands::VolumeLedgerRead, Vec<CommandWarning>), String>>,
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
    /// The curve correction on or off (docs/adr/0934 §3).
    Curve(bool),
    /// The list under the figures.
    View(Shown),
    /// The uncertainty's σ (0), shift (1) or share accepted (2) (docs/adr/0941).
    Uncertainty(usize, String),
    /// The survey now (docs/adr/0942); none: no progress read.
    Survey(Option<String>),
    /// The survey before it; none: since the original ground.
    Previous(Option<String>),
    /// A saved volume compared with the chosen one, or not (docs/adr/0943).
    Alternative(String, bool),
    /// The ledger's step, grouping and decimals (docs/adr/0949).
    LedgerStep(String),
    LedgerRegions(bool),
    LedgerDecimals(u32),
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
            uncertainty: Default::default(),
            budget: None,
            survey: None,
            previous: None,
            progress: None,
            alternatives: Vec::new(),
            comparison: None,
            traces: Vec::new(),
            ledger_step: "100".into(),
            ledger_regions: false,
            ledger_decimals: 2,
            ledger: None,
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
        let def = r.def();
        w.preview = match def.clone().and_then(|d| road_volumes::preview(&doc.model, &d)) {
            Err(e) => Preview { why: Some(e), ..Preview::default() },
            Ok((report, warnings)) => Preview { report: Some(report), warnings, why: None },
        };
        // The budget, when σ is written and the volume computes.
        w.budget = match (number(&w.uncertainty[0]), &w.preview.report, def) {
            (Ok(None), _, _) | (_, None, _) | (_, _, Err(_)) => None,
            (Err(()), _, _) => Some(Err(format!("{} bir sayı olmalı.", UNCERTAINTY[0]))),
            (Ok(Some(sigma)), Some(_), Ok(def)) => Some((|| {
                let read = |k: usize| number(&w.uncertainty[k]).map_err(|()| format!("{} bir sayı olmalı.", UNCERTAINTY[k]));
                let input = kentos_commands::VolumesUncertainty {
                    volume: def.id.clone(),
                    sigma,
                    systematic: read(1)?,
                    accept: read(2)?,
                };
                kentos_native_application::road_volumes_uncertainty::preview(&doc.model, &def, &input)
            })()),
        };
        // The progress, when a survey is chosen and the volume computes.
        w.progress = match (&w.survey, &w.preview.report, r.def()) {
            (Some(survey), Some(_), Ok(def)) => {
                let input = kentos_commands::VolumesProgress {
                    volume: def.id.clone(),
                    survey: survey.clone(),
                    previous: w.previous.clone(),
                };
                Some(kentos_native_application::road_volumes_progress::preview(&doc.model, &def, &input))
            }
            _ => None,
        };
        // The comparison, of the chosen volume as saved with the alternatives picked.
        let chosen_id = r.id.clone();
        let saved = w.kept.iter().any(|k| k.id == chosen_id);
        let others: Vec<String> = w.alternatives.iter().filter(|a| **a != chosen_id && w.kept.iter().any(|k| &k.id == *a)).cloned().collect();
        let input = kentos_commands::VolumesCompare {
            volumes: std::iter::once(chosen_id).chain(others).collect(),
            tolerance: None,
            height: None,
        };
        let asked = saved && input.volumes.len() > 1;
        w.comparison = asked.then(|| kentos_native_application::road_volumes_compare::preview(&doc.model, &input));
        w.traces = match &w.comparison {
            Some(Ok(_)) => kentos_native_application::road_volumes_compare::traces(&doc.model, &input),
            _ => Vec::new(),
        };
        // The ledger, while it is shown: it computes the volume once more.
        let shown = matches!(w.shown, Shown::Ledger | Shown::Largest);
        w.ledger = match (shown, &w.preview.report, r.def()) {
            (true, Some(_), Ok(def)) => Some(match number(&w.ledger_step) {
                Err(()) => Err(format!("{LEDGER_STEP} bir sayı olmalı.")),
                Ok(step) => {
                    let input = kentos_commands::VolumesLedger {
                        volume: def.id.clone(),
                        by: if w.ledger_regions {
                            kentos_commands::LedgerBy::Regions
                        } else {
                            kentos_commands::LedgerBy::Stations { step: step.unwrap_or(100.0) }
                        },
                        top: Some(10),
                        decimals: Some(w.ledger_decimals),
                    };
                    kentos_native_application::road_volumes_ledger::preview_of(&doc.model, &def, &input)
                }
            }),
            _ => None,
        };
    }

    /// A survey picked by its name in the window's agent controls: its id, none for “Yok” and the previous one's none.
    fn volume_surface_named(&self, name: &str) -> Result<Option<String>, String> {
        if name == "Yok" || name == NO_PREVIOUS {
            return Ok(None);
        }
        self.volume_surfaces()
            .into_iter()
            .find(|s| s.1 == name)
            .map(|s| Some(s.0))
            .ok_or_else(|| format!("“{TITLE}” penceresinde “{name}” yüzeyi yok"))
    }

    /// The project's terrain surfaces: id and name.
    fn volume_surfaces(&self) -> Vec<(String, String)> {
        self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model.settings().road.as_ref().map_or_else(Vec::new, |r| r.surfaces.iter().map(|s| (s.id.clone(), s.name.clone())).collect())
        })
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
                    // The curve correction is the end areas' (docs/adr/0934 §3): another method takes it off, or the
                    // save would be refused for a switch the window shows disabled.
                    if m != VolumeMethod::EndArea {
                        r.curve = false;
                    }
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
            Event::Uncertainty(k, t) => {
                if let Some(f) = w.uncertainty.get_mut(k) {
                    *f = t;
                }
            }
            Event::Survey(id) => {
                w.shown = if id.is_some() { Shown::Progress } else { w.shown };
                w.survey = id;
            }
            Event::Previous(id) => w.previous = id,
            Event::LedgerStep(t) => {
                w.ledger_step = t;
                w.shown = if w.shown == Shown::Largest { Shown::Largest } else { Shown::Ledger };
            }
            Event::LedgerRegions(on) => {
                w.ledger_regions = on;
                w.shown = if w.shown == Shown::Largest { Shown::Largest } else { Shown::Ledger };
            }
            Event::LedgerDecimals(d) => {
                w.ledger_decimals = d.min(3);
                w.shown = Shown::Ledger;
            }
            Event::Alternative(id, on) => {
                w.alternatives.retain(|a| *a != id);
                if on {
                    w.alternatives.push(id);
                    w.shown = Shown::Compare;
                }
            }
            Event::Rule(r) => {
                if let Some(row) = chosen_row(w) {
                    row.rule = r;
                }
            }
            Event::Curve(on) => {
                if let Some(row) = chosen_row(w) {
                    row.curve = on;
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
        let works = w.progress.as_ref().and_then(|p| p.as_ref().ok()).filter(|_| w.shown == Shown::Progress);
        let plans = (w.shown == Shown::Compare && !w.traces.is_empty()).then_some(&w.traces);
        let book = w.ledger.as_ref().and_then(|l| l.as_ref().ok()).filter(|_| matches!(w.shown, Shown::Ledger | Shown::Largest));
        if chosen.is_some() {
            d = match (works, plans) {
                _ if book.is_some() => {
                    let (read, _) = book.expect("checked");
                    d.picture(iced::widget::canvas(Book { read }).width(Fill).height(Fill), 190.0)
                }
                (Some((p, _)), _) => d.picture(iced::widget::canvas(Works { report: p }).width(Fill).height(Fill), 190.0),
                (None, Some(tr)) => d.picture(iced::widget::canvas(Departures { traces: tr }).width(Fill).height(Fill), 190.0),
                _ => d.picture(iced::widget::canvas(Mass { preview: &w.preview }).width(Fill).height(Fill), 190.0),
            };
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
                    if let Some(c) = rep.curve.as_ref() {
                        // The stretches left plain are said by the query's warning, listed below.
                        d = d.note(Banner::info(format!(
                            "Kurp düzeltmesi kazıyı {} m³, dolguyu {} m³ değiştirdi (düz ortalama alana göre).",
                            signed(c.cut),
                            signed(c.fill)
                        )));
                    }
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
                    let budget = w.budget.as_ref().and_then(|b| b.as_ref().ok());
                    match &w.budget {
                        Some(Err(e)) => d = d.note(Banner::info(e.clone())),
                        Some(Ok((b, warnings))) => {
                            d = d.note(Banner::info(budget_words(b)));
                            for wn in warnings.iter().filter(|x| x.code.starts_with("volume_uncertainty")) {
                                d = d.note(Banner::warning(wn.message.clone()));
                            }
                        }
                        None => {}
                    }
                    let progress = w.progress.as_ref().and_then(|p| p.as_ref().ok());
                    match &w.progress {
                        Some(Err(e)) => d = d.note(Banner::error(e.clone())),
                        Some(Ok((p, warnings))) => {
                            d = d.note(Banner::info(progress_words(p, &self.volume_surfaces())));
                            for wn in warnings {
                                d = d.note(Banner::warning(wn.message.clone()));
                            }
                        }
                        None => {}
                    }
                    let comparison = w.comparison.as_ref().and_then(|c| c.as_ref().ok());
                    match &w.comparison {
                        Some(Err(e)) => d = d.note(Banner::error(e.clone())),
                        Some(Ok((c, warnings))) => {
                            d = d.note(Banner::info(compare_words(c)));
                            for wn in warnings {
                                d = d.note(Banner::warning(wn.message.clone()));
                            }
                        }
                        None => {}
                    }
                    match &w.ledger {
                        Some(Err(e)) => d = d.note(Banner::error(e.clone())),
                        Some(Ok((l, _))) => d = d.note(Banner::info(ledger_words(l))),
                        None => {}
                    }
                    let mut options = vec![Shown::Sheet];
                    if !rep.layers.is_empty() {
                        options.push(Shown::Layers);
                    }
                    if budget.is_some() {
                        options.push(Shown::Budget);
                    }
                    if progress.is_some() {
                        options.push(Shown::Progress);
                    }
                    if comparison.is_some() {
                        options.push(Shown::Compare);
                    }
                    options.push(Shown::Ledger);
                    options.push(Shown::Largest);
                    let now = if options.contains(&w.shown) { w.shown } else { Shown::Sheet };
                    let list: Element<'_, Message> = match (now, budget, progress, comparison) {
                        (Shown::Ledger | Shown::Largest, _, _, _) => match &w.ledger {
                            Some(Ok((l, _))) if now == Shown::Ledger => ledger_table(l),
                            Some(Ok((l, _))) => largest_table(l),
                            _ => label::caption("Defter okunamadı; nedeni yukarıda.").into(),
                        },
                        (Shown::Layers, _, _, _) => layers_table(rep),
                        (Shown::Budget, Some((b, _)), _, _) => budget_table(b),
                        (Shown::Progress, _, Some((p, _)), _) => progress_table(p),
                        (Shown::Compare, _, _, Some((c, _))) => compare_table(c),
                        _ => sheet_table(rep),
                    };
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
        let fill = |title| road_dialog::Col { title, width: None, number: true };
        let factors = road_dialog::edit_table(
            vec![fill("Baş (m)"), fill("Son (m)"), fill("Kabarma"), fill("Sıkışma")],
            r.factors.len(),
            move |j, k| road_dialog::edit(&r.factors[j][k], true, move |v| msg(Event::Factor(j, k, v))),
            w.line,
            |j| msg(Event::Line(j)),
            "Katsayı satırı yok",
        );
        let strips = road_dialog::edit_table(
            vec![fill("Baş (m)"), fill("Son (m)"), fill("Kalınlık (m)")],
            r.strip.len(),
            move |j, k| road_dialog::edit(&r.strip[j][k], true, move |v| msg(Event::Strip(j, k, v))),
            w.strip_line,
            |j| msg(Event::StripLine(j)),
            "Sıyırma satırı yok",
        );
        let bench_field = |k: usize, hint: &'a str| kentos_ui::widget::text_field(hint, &r.bench[k]).on_input(move |v| msg(Event::Bench(k, v)));
        let mut form = Form::new()
            .label_width(110.0)
            .section("Tanım")
            .field("Ad", kentos_ui::widget::text_field("Kübajın adı", &r.name).on_input(|t| msg(Event::Name(t))))
            .field("Koridor", on_corridor)
            .field("Yöntem", on_method)
            .field(RULE, on_rule)
            .help("NETPRO'nun adlarıyla: Yol ya da Formasyon, Zemin ya da Bitkisel Toprak. Formasyon üstyapı ister.")
            .field(
                "Kurp",
                if r.method == VolumeMethod::EndArea {
                    kentos_ui::widget::Switch::new(r.curve, |on| msg(Event::Curve(on))).label(CURVE)
                } else {
                    kentos_ui::widget::Switch::disabled(false).label(CURVE)
                },
            )
            .help(if r.method == VolumeMethod::EndArea {
                "Kurpta kesitler yelpaze gibi açılır: iç yanın hacmi az, dışınki çok sayılmalı (Pappus). Kesitlerin ağırlık merkezi kullanılır."
            } else {
                "Kurp düzeltmesi ortalama alan yöntemindedir; prizmoidal ve yüzeyler yönteminde yoktur."
            })
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
        let field = |k: usize, hint: &'a str| {
            kentos_ui::widget::text_field(hint, &w.uncertainty[k]).on_input(move |v| msg(Event::Uncertainty(k, v)))
        };
        form = form
            .section("Belirsizlik")
            .field(UNCERTAINTY[0], field(0, "yok"))
            .help("Arazi kotlarının standart sapması: kontrol noktalarının RMSE'si, ölçme aletinin doğruluğu.")
            .field(UNCERTAINTY[1], field(1, "yok"))
            .help("Bütün kotların ortak kayması (datum, anten yüksekliği); eksi aşağı.")
            .field(UNCERTAINTY[2], field(2, "yok"))
            .help("Her hacmin %95 sınırının hacme oranı bunun altında kalmalı (sözleşmeden). KentOS sayı vermez.");
        // The surveys: any terrain surface but the corridor's own (its original ground).
        let original = r
            .corridor
            .as_ref()
            .and_then(|c| self.document.as_ref()?.model.road_corridors().iter().find(|x| &x.id == c).map(|x| x.surface.clone()));
        let surveys: Vec<(String, String)> =
            self.volume_surfaces().into_iter().filter(|s| Some(&s.0) != original.as_ref()).collect();
        let now_ids: Vec<Option<String>> = std::iter::once(None).chain(surveys.iter().map(|s| Some(s.0.clone()))).collect();
        let on_survey = Select::new(
            std::iter::once(Choice::new("Yok".to_owned())).chain(surveys.iter().map(|s| Choice::new(s.1.clone()))),
            now_ids.iter().position(|i| i == &w.survey),
            move |i| msg(Event::Survey(now_ids[i].clone())),
        );
        let before: Vec<&(String, String)> = surveys.iter().filter(|s| Some(&s.0) != w.survey.as_ref()).collect();
        let before_ids: Vec<Option<String>> = std::iter::once(None).chain(before.iter().map(|s| Some(s.0.clone()))).collect();
        let on_previous = Select::new(
            std::iter::once(Choice::new(NO_PREVIOUS.to_owned())).chain(before.iter().map(|s| Choice::new(s.1.clone()))),
            before_ids.iter().position(|i| i == &w.previous),
            move |i| msg(Event::Previous(before_ids[i].clone())),
        );
        form = form.section(PROGRESS);
        form = if surveys.is_empty() {
            form.row(label::caption("Ara ölçü için Arazi'de bir yüzey daha oluşturun (drone, GNSS ya da total station ölçüsü).").style(muted))
        } else {
            form.field(SURVEY, on_survey)
                .help("Şantiyenin bugünkü yüzeyi. İlk arazi koridorun yüzeyidir; ölçü onunla ve tasarımla karşılaştırılır.")
                .field(PREVIOUS, on_previous)
                .help("Bir önceki hakedişin ölçüsü: dönemin işi iki ölçünün birikmiş işlerinin farkıdır.")
        };
        // The ledger (docs/adr/0949): how the total is taken apart.
        let on_by = Select::new(GROUPINGS.map(Choice::new), Some(usize::from(w.ledger_regions)), |i| msg(Event::LedgerRegions(i == 1)));
        let on_decimals =
            Select::new(DECIMALS.map(Choice::new), Some(w.ledger_decimals as usize), |i| msg(Event::LedgerDecimals(i as u32)));
        form = form.section(LEDGER).field(LEDGER_BY, on_by);
        if !w.ledger_regions {
            form = form.field(
                LEDGER_STEP,
                kentos_ui::widget::text_field("100", &w.ledger_step).on_input(|t| msg(Event::LedgerStep(t))),
            );
        }
        form = form
            .field(ROUNDING, on_decimals)
            .help("Toplamı km dilimine, bölgeye ve malzemeye göre döker; aralıklar ortalarından bütün alınır, gruplar toplamı tam verir. Rapor toplamı yuvarlanmamış değerden yazılır.");
        // The alternatives: the other saved volumes, each with a switch.
        form = form.section(ALTERNATIVES);
        let saved = w.kept.iter().any(|k| k.id == r.id);
        let others: Vec<&VolumeDef> = w.kept.iter().filter(|k| k.id != r.id).collect();
        if !saved {
            form = form.row(label::caption("Kaydedilmiş kübajlar karşılaştırılır; önce bu kübajı kaydedin.").style(muted));
        } else if others.is_empty() {
            form = form.row(label::caption("Karşılaştırmak için başka bir koridorun ya da aynı koridorun başka bir kübajını oluşturun.").style(muted));
        } else {
            for k in others {
                let id = k.id.clone();
                let on = w.alternatives.contains(&k.id);
                form = form.field(
                    k.name.as_str(),
                    kentos_ui::widget::Switch::new(on, move |v| msg(Event::Alternative(id.clone(), v))).label("Karşılaştır"),
                );
            }
            form = form.help("Seçili kübaj taban sayılır. Aksın planda 1 cm, kırmızı kotun 1 cm'den çok ayrıldığı km aralıkları bulunur.");
        }
        form
    }

    pub(crate) fn volume_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.volume_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CURVE) => Some(msg(Event::Curve(!w.rows.get(w.chosen.unwrap_or(usize::MAX)).is_some_and(|r| r.curve)))),
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
            Control::Press(BUDGET) => Some(msg(Event::View(Shown::Budget))),
            Control::Press(PROGRESS) => Some(msg(Event::View(Shown::Progress))),
            Control::Press(COMPARE) => Some(msg(Event::View(Shown::Compare))),
            Control::Press(LEDGER) => Some(msg(Event::View(Shown::Ledger))),
            Control::Press(LARGEST) => Some(msg(Event::View(Shown::Largest))),
            Control::Fill(LEDGER_STEP, text) => Some(msg(Event::LedgerStep(text.to_owned()))),
            Control::Pick(LEDGER_BY, words) => Some(msg(Event::LedgerRegions(words == GROUPINGS[1]))),
            Control::Pick(ROUNDING, words) => {
                let k = DECIMALS.iter().position(|d| *d == words).ok_or_else(|| format!("“{words}” diye bir yuvarlama yok"))?;
                Some(msg(Event::LedgerDecimals(k as u32)))
            }
            Control::Fill(name, on) if w.kept.iter().any(|k| k.name == name) => {
                let id = w.kept.iter().find(|k| k.name == name).map(|k| k.id.clone()).unwrap_or_default();
                Some(msg(Event::Alternative(id, on == "açık")))
            }
            Control::Pick(SURVEY, name) => Some(msg(Event::Survey(self.volume_surface_named(name)?))),
            Control::Pick(PREVIOUS, name) => Some(msg(Event::Previous(self.volume_surface_named(name)?))),
            Control::Fill(name, text) if UNCERTAINTY.contains(&name) => {
                Some(msg(Event::Uncertainty(UNCERTAINTY.iter().position(|f| *f == name).unwrap_or(0), text.to_owned())))
            }
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
    use road_dialog::{col, num};
    let (strips, benches) = (rep.strip.is_some(), rep.bench.is_some());
    let mut cols = vec![col("Baş km", 88.0), col("Son km", 88.0), num("Kazı (m³)", 88.0), num("Dolgu (m³)", 88.0)];
    if strips {
        cols.push(num("B. toprak (m³)", 108.0));
    }
    if benches {
        cols.push(num("Diş (m³)", 80.0));
    }
    cols.extend([num("Kabarma", 72.0), num("Sıkışma", 72.0), num("Σ kazı (m³)", 100.0), num("Σ dolgu (m³)", 100.0), num("Kütle (m³)", 100.0)]);
    let opt = |v: Option<f64>, d: usize| v.map_or_else(|| "—".to_owned(), |x| grouped(x, d));
    let rows = rep
        .rows
        .iter()
        .map(|r| {
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
            cells
        })
        .collect();
    road_dialog::sheet(cols, rows, "Kesit aralığı yok", 220.0)
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
        // Zoomed in a picture (docs/adr/0937 §3): round volumes at least 40 px apart, finer as the chart is laid out
        // larger; their labels and the legend stay on the part shown.
        let part = crate::picture::Shown::of(bounds.size());
        let step = crate::picture::step(40.0 * (hi - lo) / ht.max(1.0));
        let mut volumes = Vec::new();
        let mut v = (lo / step).ceil() * step;
        while v <= hi + 1e-9 {
            let a = at(s0, v);
            if part.has_y(a.y) {
                frame.stroke(&Path::line(a, at(s1, v)), Stroke::default().with_color(Color { a: 0.35, ..t.border }).with_width(1.0));
                volumes.push(v);
            }
            v += step;
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
        let label_x = part.left(4.0);
        for v in volumes {
            let a = at(s0, v);
            crate::picture::label(&mut frame, grouped(v, 0), Point::new(label_x, a.y - 7.0), t.muted, false, (label_x > 4.5).then_some(t.field.scale_alpha(0.9)));
        }
        // The legend at the bottom of the part shown, on its own ground where it moved up over the drawing.
        let base = part.bottom(bounds.height - 18.0, 12.0);
        let mut x = part.left(left as f32 + 8.0);
        if base < bounds.height - 18.5 || x > left as f32 + 8.5 {
            frame.fill(&Path::rectangle(Point::new(x - 6.0, base - 4.0), iced::Size::new(430.0, 22.0)), t.field.scale_alpha(0.9));
        }
        for (words, color) in [("Σ kazı (gevşek)", t.danger), ("Σ dolgu (sıkışık)", t.accent), ("Kütle ordinatı", green)] {
            frame.fill_rectangle(Point::new(x, base + 6.0), iced::Size::new(14.0, 3.0), color);
            caption(&mut frame, words.into(), Point::new(x + 18.0, base), t.text);
            x += 140.0;
        }
        vec![frame.into_geometry()]
    }
}

/// The budget in a sentence (docs/adr/0941): the bounds at 95 %, their shares, the method's and the arithmetic's.
fn budget_words(b: &kentos_commands::VolumeUncertaintyReport) -> String {
    let share = |s: &kentos_commands::VolumeSpread| {
        if s.value.abs() > 0.0 { format!(" (%{})", grouped(100.0 * s.bound / s.value.abs(), 1)) } else { String::new() }
    };
    let method = match (b.other_method, b.other_cut, b.other_fill) {
        (Some(m), Some(c), Some(f)) => {
            let name = METHODS.iter().find(|x| x.0 == m).map_or("öbür yöntem", |x| x.1);
            format!(" {name} kazıyı {} m³, dolguyu {} m³ farklı verir.", signed(c - b.cut.value), signed(f - b.fill.value))
        }
        _ => String::new(),
    };
    format!(
        "Ölçme belirsizliği (σ {} m, {} nokta, %95): kazı ±{} m³{}, dolgu ±{} m³{}, net ±{} m³.{method} Aritmetiğin payı {:.0e} m³.",
        grouped(b.sigma, 3),
        b.points,
        grouped(b.cut.bound, 1),
        share(&b.cut),
        grouped(b.fill.bound, 1),
        share(&b.fill),
        grouped(b.net.bound, 1),
        b.numeric
    )
}

/// The budget's table: the cut, the fill and their net with σ, the shift's part, the 95 % bound, its share, accepted.
fn budget_table(b: &kentos_commands::VolumeUncertaintyReport) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let row = |name: &str, s: &kentos_commands::VolumeSpread| {
        vec![
            name.to_owned(),
            grouped(s.value, 1),
            grouped(s.sigma, 1),
            signed(s.bias),
            grouped(s.bound, 1),
            if s.value.abs() > 0.0 { format!("%{}", grouped(100.0 * s.bound / s.value.abs(), 2)) } else { "—".into() },
            match s.accepted {
                Some(true) => "tutar".into(),
                Some(false) => "aşıyor".into(),
                None => "—".into(),
            },
        ]
    };
    road_dialog::table(
        vec![
            col("Hacim", 64.0),
            num("Değer (m³)", 96.0),
            num("σ (m³)", 72.0),
            num("Kayma (m³)", 84.0),
            num("%95 sınır (m³)", 104.0),
            num("Oran", 64.0),
            col("Kabul", 64.0),
        ],
        vec![row("Kazı", &b.cut), row("Dolgu", &b.fill), row("Net", &b.net)],
        "",
    )
}

/// The progress in a sentence (docs/adr/0942): the share of the cut and the fill done, what is left, what went past the
/// design, and the period's work.
fn progress_words(p: &kentos_commands::VolumeProgressReport, surfaces: &[(String, String)]) -> String {
    let name = |id: &str| surfaces.iter().find(|s| s.0 == id).map_or_else(|| id.to_owned(), |s| s.1.clone());
    let n = &p.now;
    let part = |what: &str, share: Option<f64>, done: f64, left: f64| match share {
        Some(s) => format!("{what} %{} yapıldı ({} m³), {} m³ kaldı", grouped(s, 1), grouped(done, 0), grouped(left, 0)),
        None => format!("{what} yok"),
    };
    let mut out = format!(
        "{}: {}; {}.",
        name(&p.survey),
        part("kazının", p.cut_share, n.cut_done, n.cut_left),
        part("dolgunun", p.fill_share, n.fill_done, n.fill_left)
    );
    if n.cut_over >= 0.5 || n.fill_over >= 0.5 {
        out += &format!(" Tasarımın ötesinde: kazı {} m³, dolgu {} m³.", grouped(n.cut_over, 0), grouped(n.fill_over, 0));
    }
    if let (Some(prev), Some(d)) = (&p.previous, &p.period) {
        out += &format!(" Dönemde (önceki ölçü “{}”) kazı {} m³, dolgu {} m³.", name(prev), signed(d.cut_done), signed(d.fill_done));
    }
    out
}

/// The progress by stretch: done, left and past the design, for the cut and the fill (m³); the plan area the survey
/// leaves uncovered (m²).
fn progress_table(p: &kentos_commands::VolumeProgressReport) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let rows = p
        .stretches
        .iter()
        .map(|s| {
            let a = &s.amounts;
            let share = |done: f64, left: f64| {
                if done + left > 0.05 { format!("%{}", grouped(100.0 * done / (done + left), 0)) } else { "—".into() }
            };
            vec![
                s.from_text.clone(),
                s.to_text.clone(),
                grouped(a.cut_done, 1),
                grouped(a.cut_left, 1),
                grouped(a.cut_over, 1),
                share(a.cut_done, a.cut_left),
                grouped(a.fill_done, 1),
                grouped(a.fill_left, 1),
                grouped(a.fill_over, 1),
                share(a.fill_done, a.fill_left),
                if s.uncovered >= 0.05 { grouped(s.uncovered, 1) } else { "—".into() },
            ]
        })
        .collect();
    road_dialog::sheet(
        vec![
            col("Baş km", 88.0),
            col("Son km", 88.0),
            num("Kazı yapılan", 96.0),
            num("Kazı kalan", 88.0),
            num("Fazla kazı", 84.0),
            num("Kazı %", 64.0),
            num("Dolgu yapılan", 100.0),
            num("Dolgu kalan", 92.0),
            num("Fazla dolgu", 88.0),
            num("Dolgu %", 68.0),
            num("Örtülmeyen (m²)", 116.0),
        ],
        rows,
        "Kesit aralığı yok",
        220.0,
    )
}

/// The work along the stations (docs/adr/0942): each stretch a bar as wide as the stretch and as high as its volume per
/// metre, the cut above the axis and the fill below; done solid, left pale, past the design orange; under the axis a
/// grey band where the survey leaves part of the stretch uncovered (its work there is not counted).
struct Works<'a> {
    report: &'a kentos_commands::VolumeProgressReport,
}

impl canvas::Program<Message> for Works<'_> {
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
        let bars: Vec<_> = self.report.stretches.iter().filter(|s| s.to - s.from > 1e-6).collect();
        let (Some(first), Some(last)) = (bars.first(), bars.last()) else {
            caption(&mut frame, "Koridorun kesit aralığı yok.".into(), Point::new(16.0, 20.0), t.muted);
            return vec![frame.into_geometry()];
        };
        let (s0, s1) = (first.from, last.to);
        let per = |v: f64, s: &kentos_commands::StretchProgress| v / (s.to - s.from);
        let (mut lo, mut hi): (f64, f64) = (0.0, 0.0);
        for s in &bars {
            let a = &s.amounts;
            hi = hi.max(per(a.cut_done + a.cut_left + a.cut_over, s));
            lo = lo.min(-per(a.fill_done + a.fill_left + a.fill_over, s));
        }
        if hi - lo < 1e-9 {
            hi = lo + 1.0;
        }
        let (left, pad) = (64.0, 12.0);
        let (wd, ht) = (bounds.width as f64 - left - pad, bounds.height as f64 - 2.0 * pad - 14.0);
        let at = |s: f64, v: f64| {
            Point::new((left + (s - s0) / (s1 - s0).max(1e-9) * wd) as f32, (pad + (hi - v) / (hi - lo) * ht) as f32)
        };
        let part = crate::picture::Shown::of(bounds.size());
        let step = crate::picture::step(40.0 * (hi - lo) / ht.max(1.0));
        let mut marks = Vec::new();
        let mut v = (lo / step).ceil() * step;
        while v <= hi + 1e-9 {
            let a = at(s0, v);
            if part.has_y(a.y) {
                frame.stroke(&Path::line(a, at(s1, v)), Stroke::default().with_color(Color { a: 0.35, ..t.border }).with_width(1.0));
                marks.push(v);
            }
            v += step;
        }
        let orange = Color::from_rgb8(0xea, 0x58, 0x0c);
        // Left: the colour washed towards white, seen on the light and the dark ground alike.
        let pale = |c: Color| Color { r: c.r + (1.0 - c.r) * 0.62, g: c.g + (1.0 - c.g) * 0.62, b: c.b + (1.0 - c.b) * 0.62, a: 1.0 };
        let grey = Color { a: 0.55, ..t.muted };
        // A bar's piece from `a` to `b` m³/m.
        let piece = |frame: &mut canvas::Frame, s: &kentos_commands::StretchProgress, a: f64, b: f64, color: Color| {
            if (b - a).abs() < 1e-12 {
                return;
            }
            let (p, q) = (at(s.from, a.max(b)), at(s.to, a.min(b)));
            frame.fill_rectangle(p, iced::Size::new((q.x - p.x - 1.0).max(1.0), (q.y - p.y).max(0.5)), color);
        };
        for s in &bars {
            let a = &s.amounts;
            let (done, left, over) = (per(a.cut_done, s), per(a.cut_left, s), per(a.cut_over, s));
            piece(&mut frame, s, 0.0, done, t.danger);
            piece(&mut frame, s, done, done + left, pale(t.danger));
            piece(&mut frame, s, done + left, done + left + over, orange);
            let (done, left, over) = (per(a.fill_done, s), per(a.fill_left, s), per(a.fill_over, s));
            piece(&mut frame, s, 0.0, -done, t.accent);
            piece(&mut frame, s, -done, -done - left, pale(t.accent));
            piece(&mut frame, s, -done - left, -done - left - over, orange);
            if s.uncovered >= 0.05 {
                let (p, q) = (at(s.from, lo), at(s.to, lo));
                frame.fill_rectangle(Point::new(p.x, p.y + 2.0), iced::Size::new((q.x - p.x - 1.0).max(1.0), 4.0), grey);
            }
        }
        frame.stroke(&Path::line(at(s0, 0.0), at(s1, 0.0)), Stroke::default().with_color(t.muted).with_width(1.0));
        let label_x = part.left(4.0);
        for v in marks {
            let a = at(s0, v);
            crate::picture::label(&mut frame, grouped(v.abs(), 0), Point::new(label_x, a.y - 7.0), t.muted, false, (label_x > 4.5).then_some(t.field.scale_alpha(0.9)));
        }
        let base = part.bottom(bounds.height - 18.0, 12.0);
        let mut x = part.left(16.0);
        // The legend under the part shown, after the axis's unit.
        let legend = [
            ("Kazı yapılan", t.danger),
            ("Kazı kalan", pale(t.danger)),
            ("Dolgu yapılan", t.accent),
            ("Dolgu kalan", pale(t.accent)),
            ("Tasarımın ötesi", orange),
            ("Örtülmeyen", grey),
        ];
        let wide = |words: &str| 16.0 + 6.4 * words.chars().count() as f32 + 18.0;
        if base < bounds.height - 18.5 || x > 16.5 {
            let all: f32 = legend.iter().map(|(w, _)| wide(w)).sum();
            frame.fill(&Path::rectangle(Point::new(x - 6.0, base - 4.0), iced::Size::new(all + 50.0, 22.0)), t.field.scale_alpha(0.9));
        }
        caption(&mut frame, "m³/m:".into(), Point::new(x, base), t.muted);
        x += 44.0;
        for (words, color) in legend {
            frame.fill_rectangle(Point::new(x, base + 3.0), iced::Size::new(12.0, 9.0), color);
            caption(&mut frame, words.into(), Point::new(x + 16.0, base), t.text);
            x += wide(words);
        }
        vec![frame.into_geometry()]
    }
}

/// The comparison in a sentence (docs/adr/0943): each alternative's departures and its volumes' changes.
fn compare_words(c: &kentos_commands::VolumesCompared) -> String {
    let base = c.alternatives.first().map_or("", |a| a.name.as_str());
    let mut out = format!("Taban “{base}”.");
    for a in c.alternatives.iter().skip(1) {
        let Some(g) = &a.against else { continue };
        let stretch = |d: &kentos_commands::DepartureRow, unit: &str| {
            format!("km {}–{} (en çok {} {unit})", d.from_text, d.to_text, grouped(d.largest, 2))
        };
        let plan = if g.plan.is_empty() {
            "aks aynı".to_owned()
        } else {
            format!("aks {} yerde ayrılıyor: {}", g.plan.len(), g.plan.iter().map(|d| stretch(d, "m")).collect::<Vec<_>>().join(", "))
        };
        let height = if g.height.is_empty() {
            String::new()
        } else {
            format!("; kot {}", g.height.iter().map(|d| stretch(d, "m")).collect::<Vec<_>>().join(", "))
        };
        let volumes = g.figures_change.map_or_else(String::new, |f| format!("; kazı {} m³, dolgu {} m³", signed(f.cut), signed(f.fill)));
        out += &format!(" “{}”: {plan}{height}{volumes}.", a.name);
    }
    out
}

/// The comparison's table: each alternative's length, volumes, changes against the base, approval and departures.
fn compare_table(c: &kentos_commands::VolumesCompared) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let ranges = |list: &[kentos_commands::DepartureRow]| {
        if list.is_empty() { "—".to_owned() } else { list.iter().map(|d| format!("{}–{}", d.from_text, d.to_text)).collect::<Vec<_>>().join(", ") }
    };
    let rows = c
        .alternatives
        .iter()
        .map(|a| {
            let f = a.figures;
            let g = a.against.as_ref();
            let change = |pick: fn(&kentos_commands::AlternativeFigures) -> f64| {
                g.and_then(|g| g.figures_change.as_ref()).map_or_else(|| "—".to_owned(), |x| signed(pick(x)))
            };
            vec![
                a.name.clone(),
                a.length.map_or_else(|| "—".into(), |v| grouped(v, 1)),
                f.map_or_else(|| "—".into(), |f| grouped(f.cut, 0)),
                f.map_or_else(|| "—".into(), |f| grouped(f.fill, 0)),
                if g.is_some() { change(|x| x.cut) } else { "taban".into() },
                if g.is_some() { change(|x| x.fill) } else { "taban".into() },
                a.approved.as_ref().map_or_else(
                    || "—".into(),
                    |p| if p.changed { format!("{} · kazı {}", p.at, signed(p.cut_change)) } else { format!("{} · aynı", p.at) },
                ),
                g.map_or_else(|| "—".into(), |g| ranges(&g.plan)),
                g.map_or_else(|| "—".into(), |g| ranges(&g.height)),
            ]
        })
        .collect();
    road_dialog::sheet(
        vec![
            col("Kübaj", 120.0),
            num("Aks (m)", 76.0),
            num("Kazı (m³)", 84.0),
            num("Dolgu (m³)", 84.0),
            num("Δ kazı", 80.0),
            num("Δ dolgu", 80.0),
            col("Onay", 150.0),
            col("Planda ayrılan km", 200.0),
            col("Kotta ayrılan km", 200.0),
        ],
        rows,
        "Karşılaştırılacak kübaj yok",
        220.0,
    )
}

/// The alternatives' departures along their stations (docs/adr/0943): above, the distance from the base in plan;
/// below, the elevation difference where the two lie within the tolerance; each alternative in its colour, the 1 cm
/// tolerance dashed. A picture of the alignments in plan showed nothing: a 6 m departure on a 600 m road is a pixel.
struct Departures<'a> {
    traces: &'a [kentos_native_application::road_volumes_compare::Trace],
}

impl canvas::Program<Message> for Departures<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let caption = |frame: &mut canvas::Frame, words: String, p: Point, color: Color| {
            frame.fill_text(Text { content: words, position: p, color, size: Pixels(typography::caption()), font: typography::ui(), ..Text::default() });
        };
        let colours = [t.accent, Color::from_rgb8(0xea, 0x58, 0x0c), Color::from_rgb8(0x16, 0xa3, 0x4a), Color::from_rgb8(0x93, 0x33, 0xea), t.danger];
        let s1 = self.traces.iter().flat_map(|r| r.plan.last().map(|p| p.0)).fold(1.0, f64::max);
        let top_plan = self.traces.iter().flat_map(|r| r.plan.iter().map(|p| p.1)).fold(0.05, f64::max) * 1.1;
        let top_z = self.traces.iter().flat_map(|r| r.height.iter().filter_map(|p| p.1.map(f64::abs))).fold(0.05, f64::max) * 1.1;
        let (left, right, top, gap) = (64.0f64, 12.0f64, 18.0f64, 22.0f64);
        let usable = bounds.height as f64 - top - 22.0 - gap;
        let band = usable / 2.0;
        let width = bounds.width as f64 - left - right;
        let x = |s: f64| (left + s / s1 * width) as f32;
        // The plan band: 0 at its bottom; the height band: 0 in its middle.
        let y_plan = |v: f64| (top + band - v / top_plan * band) as f32;
        let z0 = top + band + gap + band / 2.0;
        let y_z = |v: f64| (z0 - v / top_z * band / 2.0) as f32;
        let grid = Stroke::default().with_color(Color { a: 0.35, ..t.border }).with_width(1.0);
        let axis = Stroke::default().with_color(t.muted).with_width(1.0);
        frame.stroke(&Path::line(Point::new(x(0.0), y_plan(0.0)), Point::new(x(s1), y_plan(0.0))), axis);
        frame.stroke(&Path::line(Point::new(x(0.0), z0 as f32), Point::new(x(s1), z0 as f32)), axis);
        for (v, y) in [(top_plan / 1.1, y_plan(top_plan / 1.1)), (top_z / 1.1, y_z(top_z / 1.1)), (-top_z / 1.1, y_z(-top_z / 1.1))] {
            frame.stroke(&Path::line(Point::new(x(0.0), y), Point::new(x(s1), y)), grid);
            caption(&mut frame, format!("{} m", grouped(v, if v.abs() < 1.0 { 2 } else { 1 })), Point::new(4.0, y - 7.0), t.muted);
        }
        caption(&mut frame, "Planda uzaklık".into(), Point::new(left as f32 + 6.0, top as f32 - 16.0), t.muted);
        caption(&mut frame, "Kot farkı".into(), Point::new(left as f32 + 6.0, (top + band + gap) as f32 - 16.0), t.muted);
        // The tolerance, dashed, in the plan band.
        let dashed = Stroke { line_dash: canvas::LineDash { segments: &[4.0, 4.0], offset: 0 }, ..Stroke::default().with_color(t.muted).with_width(1.0) };
        frame.stroke(&Path::line(Point::new(x(0.0), y_plan(0.01)), Point::new(x(s1), y_plan(0.01))), dashed);
        for (i, r) in self.traces.iter().enumerate() {
            let colour = colours[i % colours.len()];
            let line = Path::new(|b| {
                for (k, (s, d)) in r.plan.iter().enumerate() {
                    let p = Point::new(x(*s), y_plan(d.min(top_plan)));
                    if k == 0 { b.move_to(p) } else { b.line_to(p) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(colour).with_width(1.6));
            let z = Path::new(|b| {
                let mut on = false;
                for (s, dz) in &r.height {
                    match dz {
                        Some(v) => {
                            let p = Point::new(x(*s), y_z(*v));
                            if on { b.line_to(p) } else { b.move_to(p) }
                            on = true;
                        }
                        None => on = false,
                    }
                }
            });
            frame.stroke(&z, Stroke::default().with_color(colour).with_width(1.6));
        }
        let base = bounds.height - 18.0;
        let mut lx = left as f32;
        for (i, r) in self.traces.iter().enumerate() {
            let colour = colours[i % colours.len()];
            frame.fill_rectangle(Point::new(lx, base + 5.0), iced::Size::new(14.0, 3.0), colour);
            caption(&mut frame, r.name.clone(), Point::new(lx + 18.0, base), t.text);
            lx += 18.0 + 6.4 * r.name.chars().count() as f32 + 24.0;
        }
        caption(&mut frame, "Kesikli: 1 cm tolerans · yatay: alternatifin istasyonu".into(), Point::new(lx, base), t.muted);
        vec![frame.into_geometry()]
    }
}

/// A residue in m³ with its sign at `decimals`: “0”, “+1”, “−0,01”.
fn residue(v: f64, decimals: u32) -> String {
    let d = decimals as usize;
    if v.abs() < 0.5 * 10f64.powi(-(decimals as i32)) {
        "0".into()
    } else if v < 0.0 {
        format!("−{}", grouped(-v, d))
    } else {
        format!("+{}", grouped(v, d))
    }
}

/// The ledger in a sentence (docs/adr/0949): the heaviest group of the cut and the fill, how the cut crowds, the
/// largest stretch and why, the rounding's residue.
fn ledger_words(l: &kentos_commands::VolumeLedgerRead) -> String {
    let mut out = String::new();
    for (k, word) in [(0usize, "Kazının"), (1, "Dolgunun")] {
        if let Some(g) = l.groups.iter().filter(|g| g.shares[k].is_some()).max_by(|a, b| a.values[k].total_cmp(&b.values[k])) {
            out += &format!("{word} %{} kadarı {} grubunda. ", grouped(100.0 * g.shares[k].unwrap_or(0.0), 1), g.label);
        }
    }
    if let Some(c) = l.concentration.iter().find(|c| c.column == "cut") {
        out += &format!(
            "Yolun %{}'i kazının yarısını taşıyor (Gini {}). ",
            grouped(100.0 * c.half, 1),
            grouped(c.gini, 2)
        );
    }
    if let Some(s) = l.top.iter().find(|t| t.column == "cut").and_then(|t| t.stretches.first()) {
        let why = if s.reason == "section" {
            format!("kesiti medyanın {} katı", grouped(s.area_factor, 1))
        } else {
            format!("boyu medyanın {} katı", grouped(s.length_factor, 1))
        };
        out += &format!("En büyük kazı aralığı {} – {}: %{}, {why}. ", s.from_text, s.to_text, grouped(100.0 * s.share, 1));
    }
    let off: Vec<String> = l
        .columns
        .iter()
        .filter(|c| c.residue.abs() >= 0.5 * 10f64.powi(-(l.decimals as i32)))
        .map(|c| format!("{} {} m³", c.words.to_lowercase(), residue(c.residue, l.decimals)))
        .collect();
    if off.is_empty() {
        out += "Yuvarlanmış satırlar yuvarlanmış toplamı tutuyor.";
    } else {
        out += &format!("Yuvarlanmış satırlar toplamdan farklı: {}; toplam yuvarlanmamış değerden yazılır.", off.join(", "));
    }
    out
}

/// The groups: each column's sum and share; the totals; the rounded rows' residue.
fn ledger_table(l: &kentos_commands::VolumeLedgerRead) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let d = l.decimals as usize;
    let extra: Vec<usize> = (4..l.columns.len()).collect();
    let mut cols = vec![col("Grup", 150.0), num("Kazı (m³)", 96.0), num("Pay", 56.0), num("Dolgu (m³)", 96.0), num("Pay", 56.0)];
    for &k in &extra {
        cols.push(num(&l.columns[k].words, 110.0));
    }
    let share = |v: Option<f64>| v.map_or_else(|| "—".into(), |s| format!("%{}", grouped(100.0 * s, 1)));
    let mut rows: Vec<Vec<String>> = l
        .groups
        .iter()
        .map(|g| {
            let mut r = vec![g.label.clone(), grouped(g.values[0], d), share(g.shares[0]), grouped(g.values[1], d), share(g.shares[1])];
            for &k in &extra {
                r.push(grouped(g.values[k], d));
            }
            r
        })
        .collect();
    let mut total = vec!["Toplam".into(), grouped(l.columns[0].total, d), "%100".into(), grouped(l.columns[1].total, d), "%100".into()];
    let mut off = vec!["Yuvarlanmış satırlar farkı".into(), residue(l.columns[0].residue, l.decimals), String::new(), residue(l.columns[1].residue, l.decimals), String::new()];
    for &k in &extra {
        total.push(grouped(l.columns[k].total, d));
        off.push(residue(l.columns[k].residue, l.decimals));
    }
    rows.push(total);
    rows.push(off);
    road_dialog::sheet_fill(cols, rows, "Grup yok")
}

/// The cut's and the fill's largest stretches, with why each is large.
fn largest_table(l: &kentos_commands::VolumeLedgerRead) -> Element<'_, Message> {
    use road_dialog::{col, num};
    let rows: Vec<Vec<String>> = l
        .top
        .iter()
        .flat_map(|t| {
            let word = if t.column == "cut" { "Kazı" } else { "Dolgu" };
            t.stretches.iter().map(move |s| {
                vec![
                    word.to_owned(),
                    format!("{} – {}", s.from_text, s.to_text),
                    grouped(s.volume, 1),
                    format!("%{}", grouped(100.0 * s.share, 1)),
                    format!("%{}", grouped(100.0 * s.cumulative, 1)),
                    grouped(s.length, 1),
                    grouped(s.mean_area, 1),
                    format!("{}×", grouped(s.length_factor, 2)),
                    format!("{}×", grouped(s.area_factor, 2)),
                    if s.reason == "section" { "kesit" } else { "boy" }.to_owned(),
                ]
            })
        })
        .collect();
    road_dialog::sheet_fill(
        vec![
            col("Sütun", 58.0),
            col("Aralık", 170.0),
            num("Hacim (m³)", 90.0),
            num("Pay", 56.0),
            num("Birikimli", 70.0),
            num("Boy (m)", 64.0),
            num("Ort. alan (m²)", 92.0),
            num("Boy / medyan", 92.0),
            num("Alan / medyan", 96.0),
            col("Neden", 56.0),
        ],
        rows,
        "Hacimli aralık yok",
    )
}

/// The ledger drawn (docs/adr/0949): each group's cut above and fill below the axis with their shares; beside, the cut's
/// and the fill's concentration along the road against the even spread.
struct Book<'a> {
    read: &'a kentos_commands::VolumeLedgerRead,
}

impl canvas::Program<Message> for Book<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let l = self.read;
        let small = Pixels(typography::caption());
        let text = |frame: &mut canvas::Frame, words: String, at: Point, color: Color, center: bool| {
            frame.fill_text(Text {
                content: words,
                position: at,
                color,
                size: small,
                font: typography::ui(),
                align_x: if center { iced::alignment::Horizontal::Center.into() } else { iced::alignment::Horizontal::Left.into() },
                ..Text::default()
            });
        };
        let split = bounds.width * 0.62;
        // The groups' bars.
        let n = l.groups.len().max(1);
        let top = l.groups.iter().map(|g| g.values[0]).fold(0.0, f64::max);
        let bottom = l.groups.iter().map(|g| g.values[1]).fold(0.0, f64::max);
        let whole = (top + bottom).max(1e-9);
        let (left, pad, foot) = (12.0_f32, 22.0_f32, 18.0_f32);
        let h = bounds.height - 2.0 * pad - foot;
        let axis = pad + h * (top / whole) as f32;
        let slot = (split - left - 8.0) / n as f32;
        let bar = (slot * 0.7).min(56.0);
        frame.stroke(&Path::line(Point::new(left, axis), Point::new(split - 8.0, axis)), Stroke::default().with_color(t.muted).with_width(1.0));
        for (i, g) in l.groups.iter().enumerate() {
            let x = left + slot * (i as f32 + 0.5);
            for (k, color, up) in [(0usize, t.danger, true), (1, t.accent, false)] {
                let v = g.values[k];
                if v <= 0.0 {
                    continue;
                }
                let len = h * (v / whole) as f32;
                let y = if up { axis - len } else { axis };
                frame.fill_rectangle(Point::new(x - bar / 2.0, y), iced::Size::new(bar, len), Color { a: 0.85, ..color });
                if let Some(s) = g.shares[k].filter(|s| *s >= 0.05) {
                    let at = if up { Point::new(x, y - 15.0) } else { Point::new(x, y + len + 2.0) };
                    text(&mut frame, format!("%{}", grouped(100.0 * s, 0)), at, color, true);
                }
            }
            // The group's first km under the chart, thinned so they do not run together.
            let every = ((n as f32 * 70.0 / (split - left)).ceil() as usize).max(1);
            if i % every == 0 {
                // A km step's own first km (its first section may lie past it), a region's name.
                let words = if g.label.starts_with("Bölge") { g.label.clone() } else { g.label.split(" – ").next().unwrap_or(&g.from_text).to_owned() };
                text(&mut frame, words, Point::new(x, bounds.height - foot), t.muted, true);
            }
        }
        text(&mut frame, "Kazı ↑ · Dolgu ↓ (m³)".into(), Point::new(left, 4.0), t.muted, false);
        // The concentration: the share of the volume against the share of the road, densest first.
        let (cx0, cy0) = (split + 34.0, pad);
        let side = (bounds.width - cx0 - 14.0).min(bounds.height - pad - foot - 8.0).max(40.0);
        let at = |p: [f64; 2]| Point::new(cx0 + side * p[0] as f32, cy0 + side * (1.0 - p[1]) as f32);
        frame.stroke(&Path::rectangle(Point::new(cx0, cy0), iced::Size::new(side, side)), Stroke::default().with_color(t.border).with_width(1.0));
        frame.stroke(
            &Path::line(at([0.0, 0.0]), at([1.0, 1.0])),
            Stroke { line_dash: canvas::LineDash { segments: &[4.0, 4.0], offset: 0 }, ..Stroke::default().with_color(t.muted).with_width(1.0) },
        );
        let half = at([0.0, 0.5]);
        frame.stroke(&Path::line(half, Point::new(cx0 + side, half.y)), Stroke::default().with_color(t.faint).with_width(1.0));
        let mut legend = Vec::new();
        for c in &l.concentration {
            let color = if c.column == "cut" { t.danger } else { t.accent };
            let line = Path::new(|b| {
                for (j, p) in c.curve.iter().enumerate() {
                    if j == 0 { b.move_to(at(*p)) } else { b.line_to(at(*p)) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(color).with_width(2.0));
            let mark = at([c.half, 0.5]);
            frame.stroke(&Path::line(mark, at([c.half, 0.0])), Stroke::default().with_color(color).with_width(1.0));
            frame.fill(&Path::circle(mark, 3.5), color);
            legend.push((format!("{} {}", if c.column == "cut" { "Kazı" } else { "Dolgu" }, grouped(c.gini, 2)), color));
        }
        // The axes in words would not fit a narrow window: the title names the measure, the banner says the shares.
        text(&mut frame, "Yoğunlaşma (Gini)".into(), Point::new(cx0, 4.0), t.muted, false);
        for (j, (words, color)) in legend.into_iter().enumerate() {
            let at = Point::new(cx0 + side - 74.0, cy0 + side - 34.0 + 15.0 * j as f32);
            frame.fill_rectangle(Point::new(at.x - 10.0, at.y + 6.0), iced::Size::new(8.0, 3.0), color);
            text(&mut frame, words, at, t.text, false);
        }
        vec![frame.into_geometry()]
    }
}
