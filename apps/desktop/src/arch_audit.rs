//! Denetim, the bottom panel's tab of the architectural audit (docs/adr/0703; `arch.audit`): `cad.arch.audit`'s
//! findings in a table, by group; a row selects its objects and fits the view on them (Archicad's “Seçili bölgelere
//! yakınlaştır”); Düzelt runs the chosen finding's fix — its command call, one undo step — and checks again. The
//! thresholds are the bar's choices. The drawing changes only by a fix.
//!
//! Since docs/adr/0732: Kapsam (the whole drawing, Etkin kat, the objects selected), İstisnalar (hidden, shown, only
//! they), rows chosen with Ctrl/⌘ and Shift, Düzelt (n) as one `cad.changeset.apply` step, İstisna yap… with a note
//! and İstisnayı kaldır through `cad.arch.exceptions`, and the chosen fixes previewed on the drawing: their objects'
//! new outlines dashed, those they remove in the danger colour, from a reading copy (nothing written). The writes are
//! write.rs's, the preview preview.rs's, the tab's drawing view.rs's.

use std::cell::RefCell;
use std::collections::BTreeSet;

use iced::widget::{button, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    ARCH_AUDIT_MAX_GAP, ARCH_AUDIT_MIN_AREA, ARCH_AUDIT_MIN_LENGTH, ARCH_AUDIT_MIN_WIDTH,
    ArchAudit, ArchAuditCheck, ArchAuditExceptionView, ArchAuditFinding, ArchAuditWithin,
    CommandResult,
};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::arch_audit;
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Elided, Tip, tip};

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::icons::from_web;

mod preview;
mod view;
mod write;

/// The tab's title, which a trace names it by.
pub(crate) const TAB: &str = "Denetim";
const CHECK: &str = "Denetle";
const CHECK_HINT: &str = "Duvar birleşimlerini, uç boşluklarını, kopuk açıklık, mahal, ölçü, kolon ve işaretleri, yinelenen numaraları, katı olmayan nesneleri, seçilen profillere göre rampaları ve merdivenleri bulur. Çizimi değiştirmez.";
const FIX: &str = "Düzelt";
const STALE: &str = "Çizim denetimden sonra değişti; yeniden denetleyin.";
const NOT_CHECKED: &str = "Denetle ile çizimin mimari sorunlarını bulun.";
const CLEAN: &str = "Bulgu yok. Bu, ruhsat ya da standart uygunluğunu göstermez.";
const NONE_SHOWN: &str = "Bu grupta bulgu yok.";
const ALL: &str = "Bütün gruplar";
const EXCEPT: &str = "İstisna yap";
const UNEXCEPT: &str = "İstisnayı kaldır";
const EXCEPT_HINT: &str = "Seçili bulguları bilerek bırakılmış sayar; notuyla projede saklanır ve listede gizlenir. Bulgu değişirse istisna düşer, bulgu yeniden açılır. Tek geri alma adımıdır.";
/// The part of the drawing a bar offers (docs/adr/0732 §2).
const SCOPES: [&str; 3] = ["Bütün çizim", "Etkin kat", "Seçili nesneler"];
/// What the exceptions a bar offers (docs/adr/0732 §3).
const VIEWS: [(ArchAuditExceptionView, &str); 3] = [
    (ArchAuditExceptionView::Hide, "Gizle"),
    (ArchAuditExceptionView::Show, "Göster"),
    (ArchAuditExceptionView::Only, "Yalnız istisnalar"),
];
const COLUMNS: [&str; 5] = ["#", "Grup", "Bulgu", "Ne yapılır", "Nesne"];

/// The thresholds a bar offers (docs/adr/0703 §1), metres and square metres.
const GAPS: [f64; 5] = [0.02, 0.05, ARCH_AUDIT_MAX_GAP, 0.20, 0.50];
const AREAS: [f64; 5] = [0.05, ARCH_AUDIT_MIN_AREA, 0.25, 0.50, 1.00];
const WIDTHS: [f64; 4] = [0.10, ARCH_AUDIT_MIN_WIDTH, 0.20, 0.30];
const LENGTHS: [f64; 4] = [0.02, ARCH_AUDIT_MIN_LENGTH, 0.10, 0.20];
/// The ramp profiles a bar offers (docs/adr/0723 §4): none, or one of the catalog's, by its words.
const RAMP_PROFILES: [(Option<&str>, &str); 2] = [
    (None, "Yok"),
    (Some("erisilebilirlik-rampa/1"), "TS 9111 özeti"),
];
/// The stair profiles a bar offers (docs/adr/0724 §2): none, or one of the catalog's, by its words.
const STAIR_PROFILES: [(Option<&str>, &str); 5] = [
    (None, "Yok"),
    (Some("imar-merdiven-konut/1"), "Konut, asansörlü"),
    (
        Some("imar-merdiven-konut-asansorsuz/1"),
        "Konut, asansörsüz",
    ),
    (Some("imar-merdiven-diger/1"), "Diğer yapılar"),
    (Some("imar-merdiven-daire-ici/1"), "Daire içi"),
];
/// The bar's measures for its layout, logical pixels at the default text size: the controls at its start, İstisna yap…
/// with the gaps, Kata ver ▾, and a character of its words, roughly.
const START: f32 = 700.0;
const BUTTONS: f32 = 140.0;
const LEVELS: f32 = 130.0;
const TEXT_ADVANCE: f32 = 6.8;
/// The most objects a fix preview outlines.
const PREVIEW_LIMIT: usize = 2000;

/// The part of the drawing a check reports on (docs/adr/0732 §2): all of it, Etkin kat's objects, or the objects
/// selected when it was chosen (by their persistent ids).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum AuditScope {
    #[default]
    All,
    ActiveLevel,
    Selection(Vec<String>),
}

/// The last check: its findings, what it left out, and the drawing's state it saw.
#[derive(Clone, Debug)]
pub(crate) struct Audited {
    pub findings: Vec<ArchAuditFinding>,
    pub generation: u64,
    pub revision: u64,
    /// The findings the scope left out.
    pub outside: u32,
    /// The exceptions in the scope, listed or not.
    pub excepted: u32,
    /// The exceptions whose finding is gone, by their keys.
    pub stale: Vec<String>,
}

/// A fix preview kept for the drawing's state and the fixes' keys.
type KeptPreview = (u64, u64, String, Option<kentos_interaction::Preview>);

/// The tab's state for as long as the app lives.
#[derive(Clone, Debug)]
pub(crate) struct ArchAuditPanel {
    pub audited: Option<Audited>,
    /// The chosen finding, by its place in the findings.
    pub selected: Option<usize>,
    /// One group; none: every group.
    pub group: Option<ArchAuditCheck>,
    pub max_gap: f64,
    pub min_area: f64,
    pub min_width: f64,
    /// The profile the ramps are measured against (docs/adr/0723 §4); none: not measured.
    pub ramp_profile: Option<&'static str>,
    /// The profile the stairs are measured against (docs/adr/0724 §2); none: not measured.
    pub stair_profile: Option<&'static str>,
    pub min_length: f64,
    pub scope: AuditScope,
    pub view: ArchAuditExceptionView,
    /// The rows chosen with Ctrl/⌘ and Shift, by their places in the findings; the one pressed last is `selected`.
    pub chosen: BTreeSet<usize>,
    /// İstisna yap's note being written; none: not asked.
    pub note: Option<String>,
    /// The chosen fixes' preview, kept until the drawing or the choice changes.
    pub preview: RefCell<Option<KeptPreview>>,
}

impl Default for ArchAuditPanel {
    fn default() -> Self {
        Self {
            audited: None,
            selected: None,
            group: None,
            max_gap: ARCH_AUDIT_MAX_GAP,
            min_area: ARCH_AUDIT_MIN_AREA,
            min_width: ARCH_AUDIT_MIN_WIDTH,
            ramp_profile: RAMP_PROFILES[1].0,
            stair_profile: STAIR_PROFILES[1].0,
            min_length: ARCH_AUDIT_MIN_LENGTH,
            scope: AuditScope::All,
            view: ArchAuditExceptionView::Hide,
            chosen: BTreeSet::new(),
            note: None,
            preview: RefCell::new(None),
        }
    }
}

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Check,
    /// A group; none: every group.
    Group(Option<ArchAuditCheck>),
    /// A row pressed, by its place in the order shown.
    Press(usize),
    /// Düzelt on the chosen finding.
    Fix,
    /// Kata ver ▾'s Seç ve göster (docs/adr/0717 §2): every object without a level selected, the view on them.
    ShowLevelless,
    /// Kata ver ▾'s level: every object without a level given it, one step.
    AssignLevel(String),
    Gap(usize),
    Area(usize),
    Width(usize),
    /// Rampa's choice, by its place in [`RAMP_PROFILES`].
    RampProfile(usize),
    /// Merdiven's choice, by its place in [`STAIR_PROFILES`].
    StairProfile(usize),
    /// Kısa <'s choice, by its place in [`LENGTHS`].
    Length(usize),
    /// Kapsam's choice, by its place in [`SCOPES`].
    Scope(usize),
    /// İstisnalar' choice, by its place in [`VIEWS`].
    View(usize),
    /// İstisna yap…: the note asked for the chosen findings, or for the row a right press opened the menu on (by its
    /// place shown).
    Except(Option<usize>),
    Note(String),
    /// The note given: the chosen findings marked.
    ExceptConfirm,
    ExceptCancel,
    /// İstisnayı kaldır: the chosen findings' exceptions taken away, or the row's.
    Unexcept(Option<usize>),
    /// The row menu's Düzelt: that row's fix, or the chosen ones' when it is among them.
    FixRow(usize),
    /// Eski istisnaları temizle: the exceptions whose finding is gone dropped.
    Prune,
}

fn msg(e: Event) -> Message {
    Message::ArchAudit(e)
}

/// A group's icon.
fn group_icon(check: ArchAuditCheck) -> &'static str {
    match check {
        ArchAuditCheck::Walls => "wall",
        ArchAuditCheck::Openings => "door",
        ArchAuditCheck::Spaces => "space",
        ArchAuditCheck::Numbers => "openingNumber",
        ArchAuditCheck::Tags => "tag",
        ArchAuditCheck::Dimensions => "wallDimensions",
        ArchAuditCheck::Grids => "archGrid",
        ArchAuditCheck::Columns => "column",
        ArchAuditCheck::Viewmarks => "viewSection",
        ArchAuditCheck::Levels => "archSettings",
        ArchAuditCheck::Accessibility => "ramp",
        ArchAuditCheck::Stairs => "stair",
        ArchAuditCheck::Ramps => "ramp",
        ArchAuditCheck::Casework => "casework",
        ArchAuditCheck::Program => "archProgram",
    }
}

/// A bar button: its icon and words, its hint above it.
fn bar_button<'a>(
    glyph: &str,
    words: &'a str,
    hint: &'a str,
    on: Option<Message>,
) -> Element<'a, Message> {
    tip(
        button(
            row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                .spacing(6)
                .align_y(Center),
        )
        .style(style::button::secondary)
        .padding(kentos_ui::theme::metrics::action())
        .on_press_maybe(on),
        Tip::new(words).body(hint),
        iced::widget::tooltip::Position::Top,
    )
}

/// How a row's words are drawn: plain, quiet (a note, an exception's row, an old check's), or in the warning colour
/// (a lapsed exception's finding, docs/adr/0732 §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ink {
    Plain,
    Quiet,
    Warning,
}

/// A row as the table draws it.
struct Line {
    check: ArchAuditCheck,
    /// Grup, Bulgu, Ne yapılır, Nesne.
    cells: [String; 4],
    ink: Ink,
    /// Among the rows chosen (the one pressed last too).
    chosen: bool,
    /// The one pressed last.
    current: bool,
    /// Its fix's words, when it has one.
    fix: Option<String>,
    /// An exception's, lapsed or not.
    exception: bool,
}

/// İstisna yap's note field, which the question focuses.
const NOTE_FIELD: &str = "arch-audit-note";

/// A table cell's words: one line, cut with an ellipsis.
fn cell<'a>(words: String, ink: Ink) -> Element<'a, Message> {
    Elided::new(words)
        .size(typography::body())
        .font(typography::ui())
        .style(move |theme: &iced::Theme| {
            let t = Tokens::of(theme);
            iced::widget::text::Style {
                color: Some(match ink {
                    Ink::Plain => t.text,
                    Ink::Quiet => t.muted,
                    Ink::Warning => t.warning,
                }),
            }
        })
        .width(Fill)
        .into()
}

impl App {
    /// Mimari denetim (`arch.audit`): the tab, checked.
    pub(crate) fn open_arch_audit(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.show_bottom(BottomTab::Audit);
        let bar = f64::from(kentos_ui::widget::tabs::height());
        if f64::from(self.bottom_log()) + bar < crate::search::PANEL_HEIGHT {
            self.bottom_dragged(
                Some((crate::search::PANEL_HEIGHT - bar) as f32),
                std::time::Instant::now(),
            );
        }
        self.arch_audit_check();
        Task::none()
    }

    /// The bar's thresholds and profiles as the command's input: the findings `cad.arch.exceptions` names.
    fn arch_audit_finding_input(&self) -> ArchAudit {
        ArchAudit {
            checks: Vec::new(),
            max_gap: Some(self.arch_audit.max_gap),
            min_area: Some(self.arch_audit.min_area),
            min_width: Some(self.arch_audit.min_width),
            // A level finding's Düzelt gives Etkin kat (docs/adr/0717 §3).
            level: self.active_level(),
            profiles: [self.arch_audit.ramp_profile, self.arch_audit.stair_profile]
                .into_iter()
                .flatten()
                .map(str::to_owned)
                .collect(),
            min_length: Some(self.arch_audit.min_length),
            within: None,
            exceptions: None,
        }
    }

    /// The bar's choices as the command's input: its thresholds, its profiles, its scope (docs/adr/0732 §2) and its
    /// exceptions (§3). A selection's objects since gone are left out of the scope.
    fn arch_audit_input(&self) -> ArchAudit {
        let within = match &self.arch_audit.scope {
            AuditScope::All => None,
            AuditScope::ActiveLevel => Some(ArchAuditWithin {
                levels: Some(vec![self.active_level()]),
                uids: None,
            }),
            AuditScope::Selection(uids) => {
                let model = self.document.as_ref().map(|d| &d.model);
                let kept: Vec<String> = uids
                    .iter()
                    .filter(|u| {
                        Uuid::parse_str(u)
                            .ok()
                            .and_then(|u| model.and_then(|m| m.slot_of(u)))
                            .is_some()
                    })
                    .cloned()
                    .collect();
                Some(ArchAuditWithin {
                    levels: None,
                    uids: Some(kept),
                })
            }
        };
        ArchAudit {
            within,
            exceptions: Some(self.arch_audit.view),
            ..self.arch_audit_finding_input()
        }
    }

    /// Denetle: every group checked; the chosen findings kept, by their keys, when they are still there.
    pub(crate) fn arch_audit_check(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let input = self.arch_audit_input();
        let key_of = |i: usize| {
            self.arch_audit
                .audited
                .as_ref()
                .and_then(|a| a.findings.get(i))
                .map(|f| f.key.clone())
        };
        let kept = self.arch_audit.selected.and_then(key_of);
        let chosen: Vec<String> = self
            .arch_audit
            .chosen
            .iter()
            .filter_map(|i| key_of(*i))
            .collect();
        match arch_audit::plan_in(&doc.model, &input) {
            CommandResult::Completed { output, warnings } => {
                let place = |key: &str| output.findings.iter().position(|f| f.key == key);
                self.arch_audit.selected = kept.as_deref().and_then(place);
                self.arch_audit.chosen = chosen.iter().filter_map(|k| place(k)).collect();
                self.arch_audit.audited = Some(Audited {
                    findings: output.findings,
                    generation: doc.model.generation(),
                    revision: doc.model.revision(),
                    outside: output.outside,
                    excepted: output.excepted,
                    stale: output.stale,
                });
                for w in warnings {
                    self.warn(w.message);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// The chosen findings, in the findings' order: those chosen with Ctrl/⌘ and Shift, else the one pressed.
    fn arch_audit_chosen(&self) -> Vec<&ArchAuditFinding> {
        let Some(a) = self.arch_audit.audited.as_ref() else {
            return Vec::new();
        };
        let mut places: BTreeSet<usize> = self.arch_audit.chosen.clone();
        if let Some(i) = self.arch_audit.selected {
            places.insert(i);
        }
        places
            .into_iter()
            .filter_map(|i| a.findings.get(i))
            .collect()
    }

    /// A row menu's row, by its place shown: it becomes the choice unless it is among the rows chosen already.
    fn arch_audit_take_row(&mut self, row: Option<usize>) {
        let Some(i) = row.and_then(|at| self.arch_audit_shown().get(at).copied()) else {
            return;
        };
        if self.arch_audit.selected != Some(i) && !self.arch_audit.chosen.contains(&i) {
            self.arch_audit.chosen.clear();
            self.arch_audit.selected = Some(i);
        }
    }

    /// Whether the drawing changed since the check.
    pub(crate) fn arch_audit_stale(&self) -> bool {
        match (&self.arch_audit.audited, &self.document) {
            (Some(a), Some(d)) => {
                a.generation != d.model.generation() || a.revision != d.model.revision()
            }
            _ => false,
        }
    }

    /// The rows shown, as places in the findings.
    pub(crate) fn arch_audit_shown(&self) -> Vec<usize> {
        self.arch_audit.audited.as_ref().map_or_else(Vec::new, |a| {
            a.findings
                .iter()
                .enumerate()
                .filter(|(_, f)| self.arch_audit.group.is_none_or(|g| f.check == g))
                .map(|(i, _)| i)
                .collect()
        })
    }

    pub(crate) fn arch_audit_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Check => self.arch_audit_check(),
            Event::Group(g) => {
                self.arch_audit.group = g;
                self.arch_audit.selected = None;
            }
            Event::Press(at) => {
                let shown = self.arch_audit_shown();
                if let Some(&i) = shown.get(at) {
                    let m = self.modifiers;
                    if m.command() {
                        // Ctrl/⌘: one more row, or one less (docs/adr/0732 §5).
                        if let Some(was) = self
                            .arch_audit
                            .selected
                            .filter(|_| self.arch_audit.chosen.is_empty())
                        {
                            self.arch_audit.chosen.insert(was);
                        }
                        if !self.arch_audit.chosen.insert(i) {
                            self.arch_audit.chosen.remove(&i);
                        }
                        self.arch_audit.selected = Some(i);
                        self.arch_audit_show_chosen();
                    } else if m.shift() {
                        // Shift: the rows shown from the one pressed last to this one.
                        let from = self
                            .arch_audit
                            .selected
                            .and_then(|s| shown.iter().position(|x| *x == s))
                            .unwrap_or(at);
                        let (a, b) = (from.min(at), from.max(at));
                        self.arch_audit.chosen = shown[a..=b].iter().copied().collect();
                        self.arch_audit.selected = Some(i);
                        self.arch_audit_show_chosen();
                    } else {
                        self.arch_audit.chosen.clear();
                        self.arch_audit.selected = Some(i);
                        self.arch_audit_show(i);
                    }
                }
            }
            Event::Fix => self.arch_audit_fix(),
            Event::Length(i) => {
                self.arch_audit.min_length = LENGTHS[i.min(LENGTHS.len() - 1)];
                self.arch_audit_check();
            }
            Event::Scope(i) => {
                self.arch_audit.scope = match i {
                    1 => AuditScope::ActiveLevel,
                    2 => {
                        let uids: Vec<String> = self
                            .document
                            .as_ref()
                            .map(|d| {
                                self.selection
                                    .ids()
                                    .iter()
                                    .filter_map(|s| d.model.uid(*s))
                                    .map(|u| u.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();
                        if uids.is_empty() {
                            self.warn(
                                "Seçili nesne yok; önce denetlenecek nesneleri seçin.".to_owned(),
                            );
                            return Task::none();
                        }
                        AuditScope::Selection(uids)
                    }
                    _ => AuditScope::All,
                };
                self.arch_audit_check();
            }
            Event::View(i) => {
                self.arch_audit.view = VIEWS[i.min(VIEWS.len() - 1)].0;
                self.arch_audit_check();
            }
            Event::Except(row) => {
                self.arch_audit_take_row(row);
                if self.arch_audit_chosen().iter().any(|f| !f.exception) {
                    self.arch_audit.note = Some(String::new());
                    return iced::widget::operation::focus(iced::widget::Id::new(NOTE_FIELD));
                }
            }
            Event::Note(t) => {
                if self.arch_audit.note.is_some() {
                    self.arch_audit.note = Some(t);
                }
            }
            Event::ExceptCancel => self.arch_audit.note = None,
            Event::ExceptConfirm => self.arch_audit_except(),
            Event::Unexcept(row) => {
                self.arch_audit_take_row(row);
                self.arch_audit_unexcept();
            }
            Event::FixRow(at) => {
                self.arch_audit_take_row(Some(at));
                self.arch_audit_fix();
            }
            Event::Prune => self.arch_audit_prune(),
            Event::ShowLevelless => self.arch_audit_show_levelless(),
            Event::AssignLevel(level) => self.arch_audit_assign_level(&level),
            Event::Gap(i) => {
                self.arch_audit.max_gap = GAPS[i.min(GAPS.len() - 1)];
                self.arch_audit_check();
            }
            Event::Area(i) => {
                self.arch_audit.min_area = AREAS[i.min(AREAS.len() - 1)];
                self.arch_audit_check();
            }
            Event::Width(i) => {
                self.arch_audit.min_width = WIDTHS[i.min(WIDTHS.len() - 1)];
                self.arch_audit_check();
            }
            Event::RampProfile(i) => {
                self.arch_audit.ramp_profile = RAMP_PROFILES[i.min(RAMP_PROFILES.len() - 1)].0;
                self.arch_audit_check();
            }
            Event::StairProfile(i) => {
                self.arch_audit.stair_profile = STAIR_PROFILES[i.min(STAIR_PROFILES.len() - 1)].0;
                self.arch_audit_check();
            }
        }
        Task::none()
    }

    /// The chosen findings' objects selected together and the view fitted on them all.
    fn arch_audit_show_chosen(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let mut slots: Vec<Slot> = Vec::new();
        for f in self.arch_audit_chosen() {
            for s in f
                .uids
                .iter()
                .filter_map(|u| Uuid::parse_str(u).ok())
                .filter_map(|u| doc.model.slot_of(u))
            {
                if !slots.contains(&s) {
                    slots.push(s);
                }
            }
        }
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
    }

    /// A finding's objects selected and the view fitted on them.
    fn arch_audit_show(&mut self, finding: usize) {
        let Some(f) = self
            .arch_audit
            .audited
            .as_ref()
            .and_then(|a| a.findings.get(finding))
            .cloned()
        else {
            return;
        };
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let slots: Vec<Slot> = f
            .uids
            .iter()
            .filter_map(|u| Uuid::parse_str(u).ok())
            .filter_map(|u| doc.model.slot_of(u))
            .collect();
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if ids.is_empty() {
            return;
        }
        self.navigating(|app| app.zoom_to_objects(&ids));
    }

    /// The level findings' objects of the last check.
    fn arch_audit_levelless(&self) -> Vec<String> {
        self.arch_audit.audited.as_ref().map_or_else(Vec::new, |a| {
            a.findings
                .iter()
                .filter(|f| f.check == ArchAuditCheck::Levels)
                .flat_map(|f| f.uids.iter().cloned())
                .collect()
        })
    }
}
