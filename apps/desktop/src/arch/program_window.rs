//! Bina programı (docs/adr/0773, `arch.program`): the building's groups of spaces and the design brief, and the
//! program read against the drawing. Its look is `program_view.rs` (the list/details pattern, DESIGN.md §7.9).
//!
//! The window edits a copy of the program; Kaydet writes it through `cad.program.edit`, Seçilenleri ver gives the
//! selected spaces to a group at once (one undo step each). Denetle reads the program against the drawing through
//! `cad.program.check` on a reading copy on a thread of its own: its progress is said, Durdur stops it at once. The
//! targets are the designer's brief, never a legal limit.
//!
//! Binalar (docs/adr/0784 §1) lists the buildings and their wings with the levels each spans; a building or a wing a
//! group still names is not removed (the window says which, and so does the command). Ortak alan payı picks the rule the
//! common areas are shared out by (`kentos.share/1`); Durum shows each unit's net, share and gross after Denetle.
//!
//! Kat irtifakı (docs/adr/8031): a group's nature (Niteliği: daire, dükkân, iş bürosu …), an annex's unit (Bölümü), an
//! independent unit's value factor in the land shares (Değer katsayısı) and the land shares' whole (Arsa payı paydası,
//! empty: the parcel's area).

use crate::arch;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::futures::channel::mpsc;
use iced::Task;
use kentos_commands::{
    BuildingProgram, CommandResult, ProgramAssign, ProgramBuilding, ProgramCheck, ProgramChecked, ProgramEdit,
    ProgramUnit, ProgramWing, ShareRule, SpaceBrief, UnitKind,
};
use kentos_native_application::{ExecutionContext, program_check, program_edit};

use crate::app::{App, Dialog, Message};

// The window's look: its parts, lists and forms (DESIGN.md §7.9).
#[path = "program_view.rs"]
mod view;
use crate::exchange::words::Kind;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Bina programı";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const CHECK: &str = "Denetle";
const STOP: &str = "Durdur";
const ADD_UNIT: &str = "Bölüm ekle";
const ADD_BRIEF: &str = "Hedef ekle";
const GIVE: &str = "Seçilenleri ver";
const ADD_BUILDING: &str = "Bina ekle";
const ADD_WING: &str = "Kanat ekle";
const SHOW_ON_PLAN: &str = "Planda göster";

/// The part shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    Buildings,
    #[default]
    Units,
    Briefs,
    Status,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Buildings => "Binalar",
            Tab::Units => "Bölümler",
            Tab::Briefs => "Hedefler",
            Tab::Status => "Durum",
        })
    }
}

/// A group being edited: its number and recorded area as typed; its nature and value factor too (docs/adr/8031).
#[derive(Debug, Clone, PartialEq)]
pub struct UnitRow {
    unit: ProgramUnit,
    name: String,
    area: String,
    weight: String,
    nature: String,
    factor: String,
}

/// A brief being edited: its numbers and lists as typed.
#[derive(Debug, Clone, PartialEq)]
pub struct BriefRow {
    brief: SpaceBrief,
    texts: [String; 6],
}

fn number_text(v: Option<f64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

impl BriefRow {
    fn of(brief: SpaceBrief) -> Self {
        let texts = [
            number_text(brief.area_min),
            number_text(brief.area_max),
            number_text(brief.min_width),
            number_text(brief.min_passage),
            brief.adjacent.join(", "),
            brief.reach.join(", "),
        ];
        Self { brief, texts }
    }
}

/// A check running on its own thread.
#[derive(Debug, Clone)]
pub struct Running {
    id: u64,
    stop: Arc<AtomicBool>,
    done: u64,
    total: u64,
}

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    tab: Tab,
    buildings: Vec<ProgramBuilding>,
    /// Each building's ±0.00 as typed (docs/adr/8072 §1), in the buildings' order; empty: not said.
    zeros: Vec<String>,
    /// Each building's own plan notes as typed (docs/adr/8163), in the buildings' order; read when Kaydet writes.
    notes: Vec<Vec<crate::project::plan_notes::Note>>,
    /// The levels two or more buildings span and the building each counts in (docs/adr/8072 §1).
    shared: Vec<kentos_commands::SharedLevel>,
    share_rule: Option<ShareRule>,
    units: Vec<UnitRow>,
    briefs: Vec<BriefRow>,
    version: String,
    /// Arsa payı paydası as typed (docs/adr/8031); empty: the parcel's area.
    land_total: String,
    /// The rest of the program as the project has it (buildings, clearances).
    kept: BuildingProgram,
    checked: Option<Arc<ProgramChecked>>,
    running: Option<Running>,
    next: u64,
    said: Option<(Kind, String)>,
    /// The program as this window would write it untouched: what closing compares with (docs/adr/8006). The rows
    /// keep numbers as text, so the project's own program may not come back byte for byte.
    opened: Option<Result<BuildingProgram, String>>,
    /// Closed with unsaved changes: “Kaydedilmemiş değişiklikler” asks.
    closing: bool,
    /// The building, its wing, the unit and the brief chosen in their lists (DESIGN.md §7.9).
    building: usize,
    wing: usize,
    unit: usize,
    brief: usize,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tab(Tab),
    /// A row chosen in a part's list: a building, one of its wings, a unit, a brief.
    ChooseBuilding(usize),
    ChooseWing(usize),
    ChooseUnit(usize),
    ChooseBrief(usize),
    Version(String),
    Rule(Option<ShareRule>),
    AddBuilding,
    RemoveBuilding(usize),
    BuildingName(usize, String),
    BuildingLevel(usize, String),
    /// A building's code, use and ±0.00 as typed; a shared level's building (docs/adr/8072 §1).
    BuildingCode(usize, String),
    BuildingUsage(usize, String),
    BuildingZero(usize, String),
    /// A building's own Bölge chosen; none: the project's (docs/adr/8144).
    BuildingZone(usize, Option<kentos_commands::Zone>),
    /// A building's own structural system chosen; none: the project's (docs/adr/8145).
    BuildingStructure(usize, Option<kentos_commands::StructuralSystem>),
    /// Its traditional architectural elements switched (cut stone masonry only).
    BuildingTraditional(usize, bool),
    /// A row of a building's own plan notes changed or taken away; a row added (docs/adr/8163).
    BuildingPlanNote(usize, usize, crate::project::plan_notes::Edit),
    AddBuildingPlanNote(usize),
    SharedBuilding(String, String),
    AddWing(usize),
    RemoveWing(usize, usize),
    WingName(usize, usize, String),
    WingLevel(usize, usize, String),
    UnitBuilding(usize, Option<String>),
    UnitWing(usize, Option<String>),
    UnitWeight(usize, String),
    UnitNumber(usize, String),
    UnitName(usize, String),
    UnitKind(usize, UnitKind),
    UnitArea(usize, String),
    /// Kat irtifakı (docs/adr/8031): a group's nature, an annex's unit, a unit's value factor, the land shares' whole.
    UnitNature(usize, String),
    UnitAnnexOf(usize, Option<String>),
    UnitFactor(usize, String),
    LandTotal(String),
    AddUnit,
    RemoveUnit(usize),
    Give(usize),
    BriefUsage(usize, String),
    BriefText(usize, usize, String),
    BriefOutdoor(usize),
    AddBrief,
    RemoveBrief(usize),
    Check,
    Stop,
    Progress(u64, u64, u64),
    Checked(u64, Arc<CommandResult<ProgramChecked>>),
    ShowFinding(usize),
    Cancel,
    Save,
    /// A click beside the window, its × or Esc: asks when there are unsaved changes (docs/adr/8006).
    Close,
    Answer(crate::unsaved_close::Answer),
}

fn msg(event: Event) -> Message {
    Message::Arch(arch::Event::Program(event))
}

fn number(t: &str, what: &str) -> Result<Option<f64>, String> {
    let t = t.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| format!("{what} bir sayı olmalı: “{t}”."))
}

fn usages(t: &str) -> Vec<String> {
    t.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

impl Window {
    fn of(program: &BuildingProgram) -> Self {
        let mut w = Self {
            units: program
                .units
                .iter()
                .map(|u| UnitRow {
                    unit: u.clone(),
                    name: u.name.clone().unwrap_or_default(),
                    area: number_text(u.recorded_area),
                    weight: u.share_weight.clone().unwrap_or_default(),
                    nature: u.nature.clone().unwrap_or_default(),
                    factor: u.land_factor.clone().unwrap_or_default(),
                })
                .collect(),
            land_total: program
                .land_total
                .map(|t| t.to_string())
                .unwrap_or_default(),
            buildings: program.buildings.clone(),
            zeros: program.buildings.iter().map(|b| b.zero.map(|z| z.to_string()).unwrap_or_default()).collect(),
            notes: program.buildings.iter().map(|b| crate::project::plan_notes::rows(&b.plan_notes)).collect(),
            shared: program.shared.clone(),
            share_rule: program.share_rule,
            briefs: program.briefs.iter().cloned().map(BriefRow::of).collect(),
            version: program.version.clone().unwrap_or_default(),
            kept: program.clone(),
            ..Self::default()
        };
        w.opened = Some(w.program());
        w
    }

    /// Whether “Kaydedilmemiş değişiklikler” is open (docs/adr/8006).
    pub(crate) fn closing(&self) -> bool {
        self.closing
    }

    /// Whether the window holds a change Kaydet has not written (docs/adr/8006).
    fn unsaved(&self) -> bool {
        self.opened.as_ref().is_some_and(|opened| *opened != self.program())
    }

    /// The program as typed; the words of what does not read.
    #[cfg(test)]
    pub(crate) fn buildings_len(&self) -> usize {
        self.buildings.len()
    }

    /// The numbers of the groups that name a building or a wing of `id`, as the window has them.
    fn users_of(&self, id: &str) -> Vec<String> {
        self.units
            .iter()
            .filter(|r| r.unit.building.as_deref() == Some(id) || r.unit.wing.as_deref() == Some(id))
            .map(|r| format!("“{}”", r.unit.number))
            .collect()
    }

    fn program(&self) -> Result<BuildingProgram, String> {
        let mut p = self.kept.clone();
        p.buildings = self.buildings.clone();
        for (b, z) in p.buildings.iter_mut().zip(&self.zeros) {
            b.zero = number(z, "Binanın ±0.00'ı")?;
        }
        for (b, rows) in p.buildings.iter_mut().zip(&self.notes) {
            b.plan_notes = crate::project::plan_notes::read(rows)
                .map_err(|why| format!("“{}” binasının plan notları: {why}", b.name))?;
        }
        // A level's choice kept while two or more buildings span it, the chosen one among them.
        p.shared = self
            .shared
            .iter()
            .filter(|sh| {
                let spanning: Vec<&ProgramBuilding> = p.buildings.iter().filter(|b| b.levels.contains(&sh.level)).collect();
                spanning.len() > 1 && spanning.iter().any(|b| b.id == sh.building)
            })
            .cloned()
            .collect();
        p.share_rule = self.share_rule;
        p.units = Vec::with_capacity(self.units.len());
        for r in &self.units {
            let mut u = r.unit.clone();
            let name = r.name.trim();
            u.name = (!name.is_empty()).then(|| name.to_owned());
            u.recorded_area = number(&r.area, "Kayıtlı alan")?;
            let weight = r.weight.trim().replace(',', ".");
            u.share_weight = (!weight.is_empty()).then_some(weight);
            let nature = r.nature.trim();
            u.nature = (!nature.is_empty()).then(|| nature.to_owned());
            let factor = r.factor.trim().replace(',', ".");
            u.land_factor = (!factor.is_empty() && u.kind == UnitKind::Unit).then_some(factor);
            if u.kind != UnitKind::Annex {
                u.annex_of = None;
            }
            p.units.push(u);
        }
        let total = self.land_total.trim().replace(['.', ' '], "");
        p.land_total = if total.is_empty() {
            None
        } else {
            Some(
                total
                    .parse::<u64>()
                    .map_err(|_| format!("Arsa payı paydası bir tam sayı olmalı: “{total}”."))?,
            )
        };
        p.briefs = Vec::with_capacity(self.briefs.len());
        for r in &self.briefs {
            let mut b = r.brief.clone();
            b.area_min = number(&r.texts[0], "En az alan")?;
            b.area_max = number(&r.texts[1], "En çok alan")?;
            b.min_width = number(&r.texts[2], "Net en")?;
            b.min_passage = number(&r.texts[3], "Net geçiş")?;
            b.adjacent = usages(&r.texts[4]);
            b.reach = usages(&r.texts[5]);
            p.briefs.push(b);
        }
        let v = self.version.trim();
        p.version = (!v.is_empty()).then(|| v.to_owned());
        Ok(p)
    }
}

/// A level in the list or out of it.
fn toggle(levels: &mut Vec<String>, level: String) {
    match levels.iter().position(|l| *l == level) {
        Some(k) => {
            levels.remove(k);
        }
        None => levels.push(level),
    }
}

/// A new id not among `taken`.
fn fresh_id(base: &str, taken: &[&str]) -> String {
    (1..)
        .map(|n| format!("{base}{n}"))
        .find(|id| !taken.contains(&id.as_str()))
        .unwrap_or_else(|| base.to_owned())
}

impl App {
    /// Bina programı: the window on the project's program.
    pub(crate) fn open_program(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        self.arch.program_window = Some(Window::of(&doc.model.program()));
        self.dialog = Some(Dialog::Arch(arch::ArchDialogKind::Program));
    }

    pub(crate) fn program_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.arch.program_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Tab(t) => w.tab = t,
            Event::ChooseBuilding(i) => {
                if i != w.building {
                    w.wing = 0;
                }
                w.building = i;
            }
            Event::ChooseWing(k) => w.wing = k,
            Event::ChooseUnit(i) => w.unit = i,
            Event::ChooseBrief(i) => w.brief = i,
            Event::Version(t) => w.version = t,
            Event::Rule(r) => w.share_rule = r,
            Event::AddBuilding => {
                let taken: Vec<String> = w
                    .buildings
                    .iter()
                    .flat_map(|b| std::iter::once(b.id.clone()).chain(b.wings.iter().map(|x| x.id.clone())))
                    .collect();
                let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
                let id = fresh_id("bina-", &taken);
                let n = w.buildings.len() + 1;
                w.buildings.push(ProgramBuilding { id, name: format!("{n}. bina"), ..Default::default() });
                w.zeros.push(String::new());
                w.notes.push(Vec::new());
                (w.building, w.wing) = (w.buildings.len() - 1, 0);
            }
            Event::RemoveBuilding(i) => {
                let Some(b) = w.buildings.get(i) else { return Task::none() };
                let mut users = w.users_of(&b.id);
                // A wing's flats name its building too: each flat is said once.
                for x in &b.wings {
                    for u in w.users_of(&x.id) {
                        if !users.contains(&u) {
                            users.push(u);
                        }
                    }
                }
                if users.is_empty() {
                    w.buildings.remove(i);
                    if i < w.zeros.len() {
                        w.zeros.remove(i);
                    }
                    if i < w.notes.len() {
                        w.notes.remove(i);
                    }
                    w.building = w.building.min(w.buildings.len().saturating_sub(1));
                    w.wing = 0;
                } else {
                    let why = format!(
                        "“{}” binası kaldırılamaz: {} bölümü ona bağlı; önce Bölümler'de başka binaya verin.",
                        b.name,
                        users.join(", ")
                    );
                    self.warn(why);
                }
            }
            Event::BuildingName(i, t) => {
                if let Some(b) = w.buildings.get_mut(i) {
                    b.name = t;
                }
            }
            // Its code in capitals as typed (EK-2 §2.3: two letters or digits), its use; empty: not said.
            Event::BuildingCode(i, t) => {
                if let Some(b) = w.buildings.get_mut(i) {
                    let t: String = t.trim().to_uppercase().chars().take(2).collect();
                    b.code = (!t.is_empty()).then_some(t);
                }
            }
            Event::BuildingUsage(i, t) => {
                if let Some(b) = w.buildings.get_mut(i) {
                    let t = t.trim();
                    b.usage = (!t.is_empty()).then(|| t.to_owned());
                }
            }
            Event::BuildingZero(i, t) => {
                if let Some(z) = w.zeros.get_mut(i) {
                    *z = t;
                }
            }
            Event::BuildingZone(i, zone) => {
                if let Some(b) = w.buildings.get_mut(i) {
                    b.zone = zone;
                }
            }
            // Its own plan notes (docs/adr/8163): rows as typed, read when Kaydet writes.
            Event::BuildingPlanNote(i, k, edit) => {
                if let Some(rows) = w.notes.get_mut(i) {
                    crate::project::plan_notes::edit(rows, k, edit);
                }
            }
            Event::AddBuildingPlanNote(i) => {
                if let Some(rows) = w.notes.get_mut(i) {
                    let row = crate::project::plan_notes::new_row(rows);
                    rows.push(row);
                }
            }
            // A system other than cut stone masonry keeps no traditional elements (docs/adr/8094).
            Event::BuildingStructure(i, system) => {
                if let Some(b) = w.buildings.get_mut(i) {
                    let kept = b.structure.is_some_and(|x| x.traditional);
                    b.structure = system.map(|system| kentos_commands::BuildingStructure {
                        system,
                        traditional: kept && system == kentos_commands::StructuralSystem::CutStoneMasonry,
                    });
                }
            }
            Event::BuildingTraditional(i, on) => {
                if let Some(x) = w.buildings.get_mut(i).and_then(|b| b.structure.as_mut()) {
                    x.traditional = on;
                }
            }
            Event::SharedBuilding(level, building) => {
                w.shared.retain(|sh| sh.level != level);
                w.shared.push(kentos_commands::SharedLevel { level, building });
            }
            Event::BuildingLevel(i, l) => {
                if let Some(b) = w.buildings.get_mut(i) {
                    toggle(&mut b.levels, l);
                }
            }
            Event::AddWing(i) => {
                let taken: Vec<String> = w
                    .buildings
                    .iter()
                    .flat_map(|b| std::iter::once(b.id.clone()).chain(b.wings.iter().map(|x| x.id.clone())))
                    .collect();
                let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
                let id = fresh_id("kanat-", &taken);
                if let Some(b) = w.buildings.get_mut(i) {
                    let n = b.wings.len() + 1;
                    b.wings.push(ProgramWing { id, name: format!("{n}. kanat"), levels: Vec::new() });
                    (w.building, w.wing) = (i, b.wings.len() - 1);
                }
            }
            Event::RemoveWing(i, k) => {
                let Some(x) = w.buildings.get(i).and_then(|b| b.wings.get(k)) else { return Task::none() };
                let users = w.users_of(&x.id);
                if users.is_empty() {
                    if let Some(b) = w.buildings.get_mut(i) {
                        b.wings.remove(k);
                        w.wing = w.wing.min(b.wings.len().saturating_sub(1));
                    }
                } else {
                    let why = format!(
                        "“{}” kanadı kaldırılamaz: {} bölümü ona bağlı; önce Bölümler'de başka kanada verin.",
                        x.name,
                        users.join(", ")
                    );
                    self.warn(why);
                }
            }
            Event::WingName(i, k, t) => {
                if let Some(x) = w.buildings.get_mut(i).and_then(|b| b.wings.get_mut(k)) {
                    x.name = t;
                }
            }
            Event::WingLevel(i, k, l) => {
                if let Some(x) = w.buildings.get_mut(i).and_then(|b| b.wings.get_mut(k)) {
                    toggle(&mut x.levels, l);
                }
            }
            Event::UnitBuilding(i, b) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.unit.building = b;
                    r.unit.wing = None;
                }
            }
            Event::UnitWing(i, x) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.unit.wing = x;
                }
            }
            Event::UnitWeight(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.weight = t;
                }
            }
            Event::UnitNumber(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.unit.number = t;
                }
            }
            Event::UnitName(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.name = t;
                }
            }
            Event::UnitKind(i, k) => {
                // An annex belongs to a unit (docs/adr/8031): the first independent unit until another is chosen.
                let first = w
                    .units
                    .iter()
                    .enumerate()
                    .find(|(j, x)| *j != i && x.unit.kind == UnitKind::Unit)
                    .map(|(_, x)| x.unit.id.clone());
                if let Some(r) = w.units.get_mut(i) {
                    r.unit.kind = k;
                    r.unit.annex_of = if k == UnitKind::Annex {
                        r.unit.annex_of.clone().or(first)
                    } else {
                        None
                    };
                }
            }
            Event::UnitNature(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.nature = t;
                }
            }
            Event::UnitAnnexOf(i, of) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.unit.annex_of = of;
                }
            }
            Event::UnitFactor(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.factor = t;
                }
            }
            Event::LandTotal(t) => w.land_total = t,
            Event::UnitArea(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.area = t;
                }
            }
            Event::AddUnit => {
                let taken: Vec<&str> = w.units.iter().map(|r| r.unit.id.as_str()).collect();
                let id = fresh_id("bolum-", &taken);
                let numbers: Vec<&str> = w.units.iter().map(|r| r.unit.number.as_str()).collect();
                let number = (1..).map(|n| n.to_string()).find(|n| !numbers.contains(&n.as_str())).unwrap_or_default();
                w.units.push(UnitRow {
                    unit: ProgramUnit { id, number, ..Default::default() },
                    name: String::new(),
                    area: String::new(),
                    weight: String::new(),
                    nature: String::new(),
                    factor: String::new(),
                });
                w.unit = w.units.len() - 1;
            }
            Event::RemoveUnit(i) => {
                if i < w.units.len() {
                    w.units.remove(i);
                    w.unit = w.unit.min(w.units.len().saturating_sub(1));
                }
            }
            Event::Give(i) => return self.give_to_unit(i),
            Event::BriefUsage(i, t) => {
                if let Some(r) = w.briefs.get_mut(i) {
                    r.brief.usage = t;
                }
            }
            Event::BriefText(i, k, t) => {
                if let Some(slot) = w.briefs.get_mut(i).and_then(|r| r.texts.get_mut(k)) {
                    *slot = t;
                }
            }
            Event::BriefOutdoor(i) => {
                if let Some(r) = w.briefs.get_mut(i) {
                    r.brief.outdoor = !r.brief.outdoor;
                }
            }
            Event::AddBrief => {
                w.briefs.push(BriefRow::of(SpaceBrief { usage: "Yeni kullanım".into(), ..Default::default() }));
                w.brief = w.briefs.len() - 1;
            }
            Event::RemoveBrief(i) => {
                if i < w.briefs.len() {
                    w.briefs.remove(i);
                    w.brief = w.brief.min(w.briefs.len().saturating_sub(1));
                }
            }
            Event::Check => return self.check_program(),
            Event::Stop => {
                if let Some(r) = w.running.take() {
                    r.stop.store(true, Ordering::Relaxed);
                    w.said = Some((Kind::Info, "Denetim durduruldu; sonuç yok.".into()));
                }
            }
            Event::Progress(id, done, total) => {
                if let Some(r) = w.running.as_mut().filter(|r| r.id == id) {
                    (r.done, r.total) = (done, total);
                }
            }
            Event::Checked(id, result) => {
                if w.running.as_ref().is_none_or(|r| r.id != id) {
                    return Task::none();
                }
                w.running = None;
                match &*result {
                    CommandResult::Completed { output, .. } => {
                        let n = output.findings.len();
                        w.said = Some(if n == 0 {
                            (Kind::Ok, "Tasarım hedeflerinden sapma yok.".into())
                        } else {
                            (Kind::Warn, format!("{n} bulgu: tasarım hedefi, yasal sınır değil."))
                        });
                        w.checked = Some(Arc::new(output.clone()));
                        w.tab = Tab::Status;
                    }
                    CommandResult::Cancelled => w.said = Some((Kind::Info, "Denetim durduruldu.".into())),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        w.said = Some((Kind::Error, error.message.clone()))
                    }
                    CommandResult::Queued { .. } => {}
                }
            }
            Event::ShowFinding(index) => {
                if w.unsaved() || w.running.is_some() {
                    return Task::none();
                }
                return self.show_program_finding(index);
            }
            Event::Cancel | Event::Answer(crate::unsaved_close::Answer::Discard) => {
                if let Some(r) = w.running.take() {
                    r.stop.store(true, Ordering::Relaxed);
                }
                self.arch.program_window = None;
                self.dialog = None;
            }
            Event::Close => {
                if w.unsaved() {
                    w.closing = true;
                } else {
                    if let Some(r) = w.running.take() {
                        r.stop.store(true, Ordering::Relaxed);
                    }
                    self.arch.program_window = None;
                    self.dialog = None;
                }
            }
            Event::Answer(crate::unsaved_close::Answer::Stay) => w.closing = false,
            Event::Answer(crate::unsaved_close::Answer::Save) => {
                let task = self.save_program();
                // Written: the window is the saved program again and closes; refused, it stays and says why.
                if self
                    .arch
                    .program_window
                    .as_ref()
                    .is_some_and(|w| !w.unsaved())
                {
                    self.arch.program_window = None;
                    self.dialog = None;
                } else if let Some(w) = self.arch.program_window.as_mut() {
                    w.closing = false;
                }
                return task;
            }
            Event::Save => return self.save_program(),
        }
        Task::none()
    }

    fn show_program_finding(&mut self, index: usize) -> Task<Message> {
        let (Some(doc), Some(finding)) = (
            self.document.as_ref(),
            self.arch.program_window.as_ref().and_then(|w| w.checked.as_ref())
                .and_then(|checked| checked.findings.get(index)),
        ) else {
            return Task::none();
        };
        let slots: Vec<_> = finding.uids.iter().filter_map(|uid| doc.model.slot_of(uid.parse().ok()?)).collect();
        let Some(&first) = slots.first() else { return Task::none() };
        let level_of = |slot| doc.model.get(slot).and_then(|entity| entity.own_level());
        let level = level_of(first)
            .filter(|level| slots.iter().all(|&slot| level_of(slot) == Some(*level)))
            .map(str::to_owned);
        self.arch.program_window = None;
        self.dialog = None;
        if let Some(level) = level {
            if self.active_level().as_deref() != Some(&level) {
                self.choose_level(Some(level));
            }
            if self.level_view != kentos_interaction::level_view::LevelView::Active {
                self.choose_level_view(kentos_interaction::level_view::LevelView::Active);
            }
        }
        self.selection.set(slots);
        self.navigating(|app| app.zoom_selection());
        Task::none()
    }

    /// Kaydet: the program as typed through `cad.program.edit`; the window stays open on what was written.
    fn save_program(&mut self) -> Task<Message> {
        let Some(w) = &self.arch.program_window else {
            return Task::none();
        };
        let program = match w.program() {
            Ok(p) => p,
            Err(why) => {
                self.warn(why);
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = ProgramEdit { program: Some(program), ..Default::default() };
        let result = program_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.output("Bina programı yazıldı.");
            let tab = self
                .arch
                .program_window
                .as_ref()
                .map(|w| w.tab)
                .unwrap_or_default();
            let mut fresh = Window::of(&out.program);
            fresh.tab = tab;
            self.arch.program_window = Some(fresh);
        }
        Task::none()
    }

    /// Seçilenleri ver: the selected spaces given to the group of row `i` (the program written first).
    fn give_to_unit(&mut self, i: usize) -> Task<Message> {
        let Some(w) = &self.arch.program_window else {
            return Task::none();
        };
        let program = match w.program() {
            Ok(p) => p,
            Err(why) => {
                self.warn(why);
                return Task::none();
            }
        };
        let Some(unit) = program.units.get(i).map(|u| u.id.clone()) else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let uids: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(doc.model.get(**s), Some(kentos_commands::Entity::Space(_))))
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .collect();
        if uids.is_empty() {
            self.warn("Seçili mahal yok: önce mahalleri seçin, sonra Seçilenleri ver'e basın.");
            return Task::none();
        }
        let n = uids.len();
        let input = ProgramEdit { program: Some(program), assign: vec![ProgramAssign { uids, unit: Some(unit) }], ..Default::default() };
        let result = program_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.output(format!("{n} mahal bölüme verildi."));
            let tab = self
                .arch
                .program_window
                .as_ref()
                .map(|w| w.tab)
                .unwrap_or_default();
            let mut fresh = Window::of(&out.program);
            fresh.tab = tab;
            self.arch.program_window = Some(fresh);
        }
        Task::none()
    }

    /// Denetle: `cad.program.check` on the drawing's reading copy, on a thread of its own.
    fn check_program(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let copy = doc.model.reading_copy();
        let Some(w) = self.arch.program_window.as_mut() else {
            return Task::none();
        };
        if let Some(r) = w.running.take() {
            r.stop.store(true, Ordering::Relaxed);
        }
        w.next += 1;
        let id = w.next;
        let stop = Arc::new(AtomicBool::new(false));
        w.running = Some(Running { id, stop: stop.clone(), done: 0, total: 0 });
        w.said = None;
        let (out, replies) = mpsc::unbounded();
        let spawned = std::thread::Builder::new().name("kentos-program".into()).spawn(move || {
            let tell = out.clone();
            let stopped = move |done: u64, total: u64| {
                if done.is_multiple_of(8) || done == total {
                    let _ = tell.unbounded_send(Event::Progress(id, done, total));
                }
                stop.load(Ordering::Relaxed)
            };
            let result = program_check::check_watched(&copy, &ProgramCheck::default(), &stopped);
            let _ = out.unbounded_send(Event::Checked(id, Arc::new(result)));
        });
        match spawned {
            Ok(_) => Task::run(replies, msg),
            Err(e) => Task::done(msg(Event::Checked(
                id,
                Arc::new(CommandResult::Failed {
                    error: kentos_commands::CommandError {
                        code: "host".into(),
                        message: format!("Denetim arka plana gönderilemedi: {e}"),
                        ..Default::default()
                    },
                }),
            ))),
        }
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn program_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.arch.program_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(CHECK) => Some(msg(Event::Check)),
            Control::Press(STOP) => Some(msg(Event::Stop)),
            Control::Press(SHOW_ON_PLAN) => Some(msg(Event::ShowFinding(0))),
            Control::Press(ADD_UNIT) => Some(msg(Event::AddUnit)),
            Control::Press(ADD_BUILDING) => Some(msg(Event::AddBuilding)),
            Control::Press("Binalar") => Some(msg(Event::Tab(Tab::Buildings))),
            Control::Press(ADD_BRIEF) => Some(msg(Event::AddBrief)),
            Control::Press("Bölümler") => Some(msg(Event::Tab(Tab::Units))),
            Control::Press("Hedefler") => Some(msg(Event::Tab(Tab::Briefs))),
            Control::Press("Durum") => Some(msg(Event::Tab(Tab::Status))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

fn fixed2(v: f64) -> String {
    kentos_geometry_core::display::fixed(v, 2)
}
