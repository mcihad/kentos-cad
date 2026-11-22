//! Topoloji, the bottom panel's tab of the topology rules' findings
//! (docs/adr/0202 §3–§5; the web's ui/bottom/TopologyPanel.ts and
//! topologyRun.ts): Denetle runs the project's rules over the objects of the
//! layers they name (the shared core's `ops::topology_rules`), the findings
//! are rows; a row selects its objects, zooms to its box and shows it over
//! the drawing (marks.rs); Düzelt ▾ writes a fix through `cad.entities.edit`
//! (`topologyFix`, the step “Topoloji düzelt”) and checks again; İstisna yap
//! marks the chosen findings in the project's settings. The rules window is
//! `rules.rs`, the words and cells `plan.rs`, the view `view.rs`.
//!
//! A check is bound to the drawing's state it saw (docs/adr/0524): once the
//! drawing changes its findings are said to be old, and Değişenleri denetle
//! (and a fix) checks again only what the change can touch
//! (`topology_rules::incremental::recheck`): the same findings as checking all.
//! A finding keeps its lasting key across checks; the chosen rows follow it.
//!
//! The review (docs/adr/1260, `review.rs`): Durum says each finding's place in its life cycle (not reviewed, error,
//! exception, to review again) with its reason; Hata işaretle and İstisna yap write Gerekçe's words; Tarifle onar
//! plans a repair recipe of the chosen findings and shows what it would do before Uygula writes it.
//!
//! A check of many objects goes to its own thread (docs/adr/0756, `running.rs`): the bar under the tab's bar moves on
//! smoothly as it goes and Durdur ends it; the drawing is not held meanwhile.

pub(crate) mod object;
mod plan;
mod review;
pub(crate) mod rules;
mod running;
mod tolerance_words;
mod view;

#[cfg(test)]
mod object_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod source_tests;
#[cfg(test)]
mod municipal_tests;

pub(crate) use plan::{Filter, texts};

use crate::gis;
use std::sync::Arc;

use iced::Task;
use kentos_commands::{
    CommandResult, EditOperation, EntitiesEdit, EntityEdit, TopologyExceptionOperation,
    TopologySettings,
};
use kentos_domain::Slot;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::topology_rules::{self as core, Finding, fix};
use kentos_interaction::{Level, Vec2, ViewChange};
use kentos_native_application::geometry::edit_geometry;
use kentos_native_application::topology_check::{self, Gathered};
use kentos_native_application::{ExecutionContext, edit};

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::traces::Control;
use running::{Ran, Reply, Running, Work};

/// A finding shown over the drawing: its regions filled, its edges bold, its
/// place marked with the problem's name (marks.rs).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProblemMark {
    pub at: Vec2,
    pub label: String,
    pub regions: Vec<Area>,
    pub edges: Vec<Edge>,
}

/// The last check: its findings and what they were made of (the drawing's objects and the rows read round bound
/// parts, docs/adr/0589), when.
#[derive(Clone, Debug)]
pub(crate) struct Checked {
    pub findings: Arc<Vec<Finding>>,
    pub g: Arc<Gathered>,
    /// The drawing (its session) and the state of it the check saw.
    pub session: u64,
    pub generation: u64,
    pub revision: u64,
    /// Where the drawing's journal of changed objects stood then (docs/adr/9146): what changed since tells whether
    /// the findings are old.
    pub mark: kentos_domain::ChangeMark,
    /// Whether the last check went over what changed alone (docs/adr/0524), and how many objects it looked at.
    pub incremental: Option<usize>,
}

impl Checked {
    /// A finding's lasting key (docs/adr/0524 §1).
    pub(crate) fn key(&self, f: &Finding) -> String {
        self.g.key(f)
    }

    /// The objects as the table says them: a drawing's object its id (“#12”), a data layer's row its key (“satır 2”,
    /// docs/adr/9124), a row outside a part “dışarıda”.
    pub(crate) fn object_words(&self) -> Vec<String> {
        (0..self.g.shapes.len())
            .map(|k| match (self.g.slots.get(k).copied().flatten(), self.g.rows.get(k).cloned().flatten()) {
                (_, Some((_, key))) => format!("satır {key}"),
                (Some(slot), None) => format!("#{}", slot.0),
                (None, None) => "dışarıda".to_owned(),
            })
            .collect()
    }
}

/// The check's generation and revision and the drawing's now: what a staleness answer was worked out for.
type StaleKey = (u64, u64, u64, u64);

/// The tab's state for as long as the app lives.
#[derive(Default)]
pub(crate) struct TopologyPanel {
    pub checked: Option<Checked>,
    /// The chosen rows, as places in the findings.
    pub selected: Vec<usize>,
    pub filter: Filter,
    /// One rule's place in the rules; none: every rule.
    pub rule: Option<usize>,
    /// The last click without Shift, in the order shown.
    anchor: Option<usize>,
    /// The one chosen finding's fixes, previewed (docs/adr/0589 §4): their words in Düzelt ▾.
    pub previews: Vec<(&'static str, Result<String, String>)>,
    /// A check going on its own thread (docs/adr/0756), and the next one's id.
    pub running: Option<Running>,
    next: u64,
    /// Whether the findings are old, worked out for the check's and the drawing's state (docs/adr/9146): the drawing
    /// changes far more often than it is asked.
    stale: std::cell::Cell<Option<(StaleKey, bool)>>,
    /// Gerekçe: the words a mark is written with (docs/adr/1260 §1).
    pub reason: String,
    /// The repair recipe planned and shown (docs/adr/1260 §4).
    pub recipe: Option<review::Recipe>,
}

/// What follows a check's answer.
#[derive(Clone, Debug)]
pub(crate) enum Then {
    /// Denetle: the rows unchosen; what it found said unless quiet.
    Check { quiet: bool },
    /// Değişenleri denetle: the chosen rows follow their keys; said unless quiet.
    Recheck { quiet: bool, chosen: Vec<String> },
    /// After a fix: the row at the fixed one's place chosen.
    Fixed { at: usize },
    /// After İstisna yap: the chosen rows follow their keys, and what was marked said.
    Marked { keys: Vec<String>, said: String },
}

impl Then {
    fn quiet(&self) -> bool {
        match self {
            Then::Check { quiet } | Then::Recheck { quiet, .. } => *quiet,
            Then::Fixed { .. } | Then::Marked { .. } => true,
        }
    }
}

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Check,
    /// Değişenleri denetle (docs/adr/0524): only what changed since the check.
    Recheck,
    Rules,
    Filter(Filter),
    /// A rule by its place in the rules; none: every rule.
    Rule(Option<usize>),
    /// A row pressed, by its place in the order shown.
    Press(usize),
    /// Düzelt ▾ chose a fix of the chosen finding.
    Fix(&'static str),
    /// İstisna yap (true) or İstisnayı kaldır (false) on the chosen rows.
    Exception(bool),
    /// Hata işaretle on the chosen rows (docs/adr/1260 §1).
    Error,
    /// Gerekçe typed.
    Reason(String),
    /// Tarifle onar: the chosen rows' recipe planned and shown; Uygula writes it, Vazgeç drops it.
    Recipe,
    Apply,
    Drop,
    /// What a check on its own thread said, by its id.
    Ran(u64, Reply),
    /// Durdur: the check going on ends.
    Stop,
    /// A frame while a check goes on: the bar moves.
    Frame,
    /// A finding by its lasting key, from Öznitelikler or Formla düzenle (docs/adr/3800): the tab opened on it.
    Show(String),
    /// A finding's state chosen in Öznitelikler (docs/adr/4600): its lasting key and the mark, with Gerekçe's words; the
    /// tab's chosen rows stay as they are.
    Review(String, TopologyExceptionOperation),
}

impl App {
    /// The project's topology settings.
    fn topology_settings(&self) -> Option<&TopologySettings> {
        self.document
            .as_ref()
            .and_then(|d| d.model.settings().topology.as_ref())
    }

    /// The last check, if it is the open drawing's.
    pub(crate) fn topology_checked(&self) -> Option<&Checked> {
        let doc = self.document.as_ref()?;
        self.gis
            .topology
            .checked
            .as_ref()
            .filter(|c| c.session == doc.session)
    }

    /// Whether what the check read changed since (docs/adr/9146): its rules, tolerance, exceptions or profile; an
    /// object of the rules' layers added, removed, reshaped or moved to another layer; a bound layer's source. A
    /// change of style, labels or attributes alone leaves the findings as they are.
    pub(crate) fn topology_stale(&self) -> bool {
        let (Some(doc), Some(c)) = (self.document.as_ref(), self.topology_checked()) else {
            return false;
        };
        if c.generation == doc.model.generation() && c.revision == doc.model.revision() {
            return false;
        }
        let key = (c.generation, c.revision, doc.model.generation(), doc.model.revision());
        if let Some((k, old)) = self.gis.topology.stale.get()
            && k == key
        {
            return old;
        }
        let old = inputs_changed(&doc.model, c);
        self.gis.topology.stale.set(Some((key, old)));
        old
    }

    /// Denetle: every rule over the objects of the layers the rules name and the rows round their bound parts
    /// (docs/adr/0589); what it found said unless `quiet`.
    pub(crate) fn topology_check(&mut self, quiet: bool) -> Task<Message> {
        self.topology_run(Work::Check, Then::Check { quiet })
    }

    /// Değişenleri denetle (docs/adr/0524): the rules over what changed since the check, its neighbours and the
    /// clusters a gap can span; the rest of the findings kept. Checked whole when there was no check, or the rules,
    /// the tolerance or the exceptions changed. The chosen rows follow their findings' keys.
    pub(crate) fn topology_recheck(&mut self, quiet: bool) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let Some(old) = self.topology_checked().cloned() else {
            return self.topology_check(quiet);
        };
        // Nothing changed since the check: the same rules over the same drawing find the same.
        if !self.topology_stale()
            && old.g.stamp.rules == topology_check::rules_digest(&doc.model, None)
        {
            return Task::none();
        }
        let chosen: Vec<String> = self
            .gis
            .topology
            .selected
            .iter()
            .filter_map(|&i| old.findings.get(i).map(|f| old.key(f)))
            .collect();
        self.topology_run(
            Work::Recheck(Box::new(old)),
            Then::Recheck { quiet, chosen },
        )
    }

    /// A check: at once when the rules' layers hold few objects, else on its own thread with the bar (docs/adr/0756),
    /// a check going on stopped for it.
    fn topology_run(&mut self, work: Work, then: Then) -> Task<Message> {
        if let Some(r) = self.gis.topology.running.take() {
            r.stop();
        }
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let state = (doc.session, doc.model.generation(), doc.model.revision(), doc.model.change_mark());
        let mut layers: Vec<&str> = Vec::new();
        for r in self.topology_settings().map_or(&[][..], |t| &t.rules[..]) {
            for l in std::iter::once(r.layer.as_str()).chain(r.other.as_deref()) {
                if !layers.contains(&l) {
                    layers.push(l);
                }
            }
        }
        let objects: usize = layers.iter().map(|l| doc.model.count(l)).sum();
        // A data layer's table is read from its source (docs/adr/9124): never on the interface's thread.
        let sourced = layers.iter().any(|l| doc.model.layers().get(l).is_some_and(|n| n.data.is_some()));
        if objects < running::THREAD_FROM && !sourced {
            if let Some(ran) = running::compute(&work, &doc.model, &core::Unwatched) {
                self.topology_found(ran, then, state);
            }
            return Task::none();
        }
        let id = self.gis.topology.next;
        self.gis.topology.next += 1;
        let copy = doc.model.reading_copy();
        let (running, task) = running::start(id, work, copy, objects, then, state, |id, reply| {
            Message::Gis(gis::Event::Topology(Event::Ran(id, reply)))
        });
        self.gis.topology.running = Some(running);
        task
    }

    /// What a check's thread said: its progress, or its answer, which applies to the drawing it was started on.
    fn topology_heard(&mut self, id: u64, reply: Reply) {
        let Some(r) = self.gis.topology.running.as_mut().filter(|r| r.id == id) else {
            // A check stopped or replaced: its words are not waited for.
            return;
        };
        match reply {
            Reply::Progress(..) | Reply::Stretch(..) => r.heard(&reply),
            Reply::Done => {
                let Some(r) = self.gis.topology.running.take() else {
                    return;
                };
                let open = self.document.as_ref().map(|d| d.session);
                if let Some(ran) = r.answer().filter(|_| open == Some(r.session)) {
                    self.topology_found(ran, r.then, (r.session, r.generation, r.revision, r.mark));
                }
            }
            Reply::Failed(why) => {
                self.gis.topology.running = None;
                self.warn(format!("Topoloji denetimi yapılamadı: {why}"));
            }
        }
    }

    /// Durdur: the check going on ends; the last check's findings stay.
    fn topology_stop(&mut self) {
        if let Some(r) = self.gis.topology.running.take() {
            r.stop();
            self.say(Level::Info, "Topoloji denetimi durduruldu.");
        }
    }

    /// A check's answer kept, for the drawing's state it saw (`session`, `generation`, `revision`, the journal's
    /// `mark`), and what follows.
    fn topology_found(
        &mut self,
        ran: Ran,
        then: Then,
        (session, generation, revision, mark): (u64, u64, u64, kentos_domain::ChangeMark),
    ) {
        let quiet = then.quiet();
        match ran {
            Ran::Checked { g, found } => {
                let n_rules = g.rules.len();
                let n_objects = g.checked();
                let sources: Vec<(String, usize)> =
                    g.sources_read.iter().map(|s| (self.topology_layer_name(&s.layer), s.checked)).collect();
                let missing = g.missing;
                let incomplete = g.incomplete.clone();
                let checked = Checked {
                    findings: Arc::new(found.findings),
                    g: Arc::new(g),
                    session,
                    generation,
                    revision,
                    mark,
                    incremental: None,
                };
                let total = checked.findings.len();
                let open = checked.findings.iter().filter(|f| !f.exception).count();
                let lapsed = checked
                    .findings
                    .iter()
                    .filter(|f| !f.exception && checked.g.lapsed(f))
                    .count();
                self.gis.topology.checked = Some(checked);
                self.gis.topology.selected.clear();
                self.gis.topology.anchor = None;
                self.gis.topology.previews.clear();
                if !quiet {
                    if missing > 0 {
                        self.warn(plan::missing_layers(missing));
                    }
                    for gap in &incomplete {
                        self.warn(plan::incomplete_text(
                            &self.topology_layer_name(&gap.layer),
                            &gap.reason,
                        ));
                    }
                    if lapsed > 0 {
                        self.warn(plan::lapsed_text(lapsed));
                    }
                    if total == 0 {
                        if incomplete.is_empty() && n_objects == 0 && n_rules > 0 {
                            self.warn(plan::nothing_checked(n_rules));
                        } else if incomplete.is_empty() {
                            self.say(Level::Success, plan::no_findings(n_rules, n_objects, &sources));
                        }
                    } else {
                        self.say(
                            Level::Info,
                            format!(
                                "Topoloji denetlendi: {n_rules} kural, {total} bulgu (açık {open}, istisna {}).",
                                total - open
                            ),
                        );
                    }
                }
            }
            Ran::Rechecked { g, found, changed } => {
                let checked = Checked {
                    findings: Arc::new(found.findings),
                    g: Arc::new(g),
                    session,
                    generation,
                    revision,
                    mark,
                    incremental: Some(changed),
                };
                let total = checked.findings.len();
                let open = checked.findings.iter().filter(|f| !f.exception).count();
                let chosen: &[String] = match &then {
                    Then::Recheck { chosen, .. } => chosen,
                    _ => &[],
                };
                let selected: Vec<usize> = checked
                    .findings
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| chosen.contains(&checked.key(f)))
                    .map(|(i, _)| i)
                    .collect();
                self.gis.topology.checked = Some(checked);
                self.gis.topology.selected = selected;
                self.gis.topology.anchor = None;
                self.topology_previews();
                if !quiet {
                    self.say(
                        Level::Info,
                        format!(
                            "Değişenler denetlendi: {changed} nesne değişti; {total} bulgu (açık {open}, istisna {}).",
                            total - open
                        ),
                    );
                }
            }
        }
        match then {
            Then::Check { .. } | Then::Recheck { .. } => {}
            // The row at the same place, if any: the next finding of the list.
            Then::Fixed { at } => {
                if let Some(n) = self
                    .topology_checked()
                    .map(|c| c.findings.len())
                    .filter(|n| *n > 0)
                {
                    self.gis.topology.selected = vec![at.min(n - 1)];
                    self.topology_previews();
                }
            }
            // The chosen rows follow their keys.
            Then::Marked { keys, said } => {
                if let Some(c) = self.topology_checked() {
                    self.gis.topology.selected = c
                        .findings
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| keys.contains(&c.key(f)))
                        .map(|(i, _)| i)
                        .collect();
                }
                self.topology_previews();
                self.say(Level::Info, said);
            }
        }
    }

    /// `topology.check`: the tab open and the check run (said when the project has no rules).
    pub(crate) fn open_topology_check(&mut self) -> Task<Message> {
        // A program without the tab says where it is (module_surfaces.rs).
        if !self.open_bottom_tab(BottomTab::Topology) {
            return Task::none();
        }
        self.show_bottom(BottomTab::Topology);
        let bar = f64::from(kentos_ui::widget::tabs::height());
        if f64::from(self.bottom_log()) + bar < crate::search::PANEL_HEIGHT {
            self.bottom_dragged(
                Some((crate::search::PANEL_HEIGHT - bar) as f32),
                std::time::Instant::now(),
            );
        }
        if self
            .topology_settings()
            .is_some_and(|t| !t.rules.is_empty())
        {
            return self.topology_check(false);
        }
        self.warn("Projede topoloji kuralı yok; Topoloji kuralları ile ekleyin.");
        Task::none()
    }

    /// The rows shown now, as places in the findings.
    pub(crate) fn topology_shown(&self) -> Vec<usize> {
        self.topology_checked().map_or_else(Vec::new, |c| {
            let rule = self.gis.topology.rule.filter(|&r| r < c.g.rules.len());
            plan::shown_findings(&c.findings, self.gis.topology.filter, rule)
        })
    }

    /// The finding shown over the drawing: the one chosen row's.
    pub(crate) fn topology_problem(&self) -> Option<ProblemMark> {
        if self.gis.topology.selected.len() != 1 {
            return None;
        }
        let f = self
            .topology_checked()?
            .findings
            .get(self.gis.topology.selected[0])?;
        Some(ProblemMark {
            at: f.at,
            label: core::problem_label(f.problem).to_owned(),
            regions: f.regions.clone(),
            edges: f.edges.clone(),
        })
    }

    pub(crate) fn topology_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Check => return self.topology_check(false),
            Event::Recheck => return self.topology_recheck(false),
            Event::Ran(id, reply) => self.topology_heard(id, reply),
            Event::Stop => self.topology_stop(),
            Event::Frame => {
                if let Some(r) = self.gis.topology.running.as_mut() {
                    r.frame(std::time::Instant::now());
                }
            }
            Event::Rules => return self.open_topology_rules(),
            Event::Filter(f) => {
                self.gis.topology.filter = f;
                self.gis.topology.selected.clear();
                self.gis.topology.anchor = None;
            }
            Event::Rule(r) => {
                self.gis.topology.rule = r;
                self.gis.topology.selected.clear();
                self.gis.topology.anchor = None;
            }
            Event::Press(at) => self.topology_press(at),
            Event::Fix(key) => return self.topology_fix(key),
            Event::Exception(on) => {
                return self.topology_review(if on {
                    TopologyExceptionOperation::Mark
                } else {
                    TopologyExceptionOperation::Unmark
                });
            }
            Event::Error => return self.topology_review(TopologyExceptionOperation::Error),
            Event::Reason(t) => self.gis.topology.reason = t,
            Event::Recipe => self.topology_plan_recipe(),
            Event::Apply => return self.topology_apply_recipe(),
            Event::Drop => self.gis.topology.recipe = None,
            Event::Show(key) => self.topology_open_finding(&key),
            Event::Review(key, op) => {
                let keep: Vec<String> = match self.topology_checked() {
                    Some(c) => self
                        .gis
                        .topology
                        .selected
                        .iter()
                        .filter_map(|&at| c.findings.get(at))
                        .map(|f| c.key(f))
                        .collect(),
                    None => Vec::new(),
                };
                return self.topology_review_keys(vec![key], op, keep);
            }
        }
        Task::none()
    }

    /// A row pressed: alone it is chosen and shown; Ctrl turns one over,
    /// Shift takes the run from the last click without Shift.
    fn topology_press(&mut self, at: usize) {
        let shown = self.topology_shown();
        let Some(&finding) = shown.get(at) else {
            return;
        };
        // Gerekçe is the finding's it was typed for (GIS-032): another finding starts it empty.
        if !self.gis.topology.selected.contains(&finding) {
            self.gis.topology.reason.clear();
        }
        let (ctrl, shift) = (self.modifiers.control(), self.modifiers.shift());
        if ctrl || shift {
            let picked: Vec<Slot> = crate::points::click_pick(
                &self
                    .gis
                    .topology
                    .selected
                    .iter()
                    .map(|&i| Slot(i as u32))
                    .collect::<Vec<_>>(),
                &shown.iter().map(|&i| Slot(i as u32)).collect::<Vec<_>>(),
                at,
                self.gis.topology.anchor,
                ctrl,
                shift,
            );
            if !shift {
                self.gis.topology.anchor = Some(at);
            }
            self.gis.topology.selected = picked.into_iter().map(|s| s.0 as usize).collect();
            self.topology_previews();
            return;
        }
        self.gis.topology.anchor = Some(at);
        // Another finding chosen: Gerekçe is its own recorded reason, or empty (UR2-034: the last row's words were
        // written on the next finding marked).
        if self.gis.topology.selected != [finding] {
            self.gis.topology.reason = self
                .topology_checked()
                .and_then(|c| c.findings.get(finding).and_then(|f| c.g.review(f)).and_then(|r| r.reason.clone()))
                .unwrap_or_default();
        }
        self.gis.topology.selected = vec![finding];
        self.topology_previews();
        self.topology_show(finding);
    }

    /// A finding's objects selected and the view on its box.
    fn topology_show(&mut self, finding: usize) {
        let Some(c) = self.topology_checked() else {
            return;
        };
        let Some(f) = c.findings.get(finding) else {
            return;
        };
        let slots: Vec<Slot> = f
            .objects
            .iter()
            .filter_map(|&k| c.g.slots.get(k).copied().flatten())
            .collect();
        // A data layer's rows read from its source: the table's selection of that layer (docs/adr/9124), outlined on
        // the map while its table shows.
        let mut rows: Vec<(String, i64)> =
            f.objects.iter().filter(|&&k| c.g.in_source(k)).filter_map(|&k| c.g.rows.get(k).cloned().flatten()).collect();
        let view = plan::finding_view(&f.bounds);
        self.selection.set(slots);
        if let Some((layer, _)) = rows.first().cloned() {
            rows.retain(|(l, _)| *l == layer);
            self.gis.features.data.selected.clear();
            self.gis.features.data.select_rows(&layer, rows.into_iter().map(|(_, k)| k).collect());
        }
        self.navigating(|app| {
            app.viewport.change(ViewChange::Fit {
                bounds: view,
                padding: 96.0,
            });
        });
    }

    /// Düzelt: the fix `key` of the one chosen finding in one undo step; the
    /// check run again. Refused (and said) when the drawing changed since
    /// the check, the fix has no answer, it would change a row outside a bound part or the command refuses it.
    fn topology_fix(&mut self, key: &'static str) -> Task<Message> {
        if self.topology_stale() {
            self.warn("Çizim denetimden sonra değişti; önce Denetle ile yenileyin.");
            return Task::none();
        }
        let [at] = self.gis.topology.selected[..] else {
            return Task::none();
        };
        let Some(c) = self.topology_checked() else {
            return Task::none();
        };
        let Some(f) = c.findings.get(at) else {
            return Task::none();
        };
        let changes = match fix(&c.g.objects(), &c.g.core_rules, f, key) {
            Ok(changes) => changes,
            Err(why) => {
                self.warn(why);
                return Task::none();
            }
        };
        if changes.iter().any(|ch| c.g.outside(ch.object)) {
            self.warn("Bu düzeltme alanın dışındaki bir satırı değiştirir; alanı genişletip satırı çalışma kopyasına alın.");
            return Task::none();
        }
        // A data layer's row read from its source is not the drawing's to change (docs/adr/9124).
        if let Some(ch) = changes.iter().find(|ch| c.g.in_source(ch.object))
            && let Some(doc) = self.document.as_ref()
        {
            let words = kentos_native_application::topology_check::source_row_words(&doc.model, &c.g, ch.object);
            self.warn(format!(
                "Bu düzeltme {words} değiştirir; satır çizimde değil. Katmanı düzenlemeye açıp satırı alın (Düzenlemeyi başlat, sonra satırı seçin), sonra düzeltin."
            ));
            return Task::none();
        }
        let edits: Vec<EntityEdit> = changes
            .into_iter()
            .filter_map(|ch| {
                let uid = c.g.uids.get(ch.object)?.clone();
                Some(match ch.shape {
                    Some(s) => EntityEdit::Update {
                        uid,
                        geometry: edit_geometry(s)?,
                    },
                    None => EntityEdit::Remove { uid },
                })
            })
            .collect();
        let said = format!(
            "Topoloji düzeltildi: {}, {}.",
            core::problem_label(f.problem),
            core::fix_label(key)
        );
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = EntitiesEdit {
            operation: EditOperation::TopologyFix,
            changes: edits,
            expected_revision: None,
        };
        match edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { warnings, .. } => {
                self.say(Level::Success, said);
                for w in warnings {
                    self.warn(w.message);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                return Task::none();
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => return Task::none(),
        }
        // Checked again over what the fix changed; then the row at the same place, if any: the next finding.
        let Some(old) = self.topology_checked().cloned() else {
            return self.topology_check(true);
        };
        self.topology_run(Work::Recheck(Box::new(old)), Then::Fixed { at })
    }

    /// Each fix of the one chosen finding previewed (docs/adr/0589 §4): its words in Düzelt ▾, with the measures it
    /// changes and the bound sources it would leave with rows to write.
    pub(crate) fn topology_previews(&mut self) {
        self.gis.topology.previews.clear();
        let (Some(doc), Some(c)) = (self.document.as_ref(), self.topology_checked()) else {
            return;
        };
        let [at] = self.gis.topology.selected[..] else {
            return;
        };
        let Some(f) = c.findings.get(at) else {
            return;
        };
        let format = self.format();
        let previews: Vec<(&'static str, Result<String, String>)> = f
            .fixes
            .iter()
            .map(|k| {
                let words = topology_check::preview(&doc.model, &c.g, f, k)
                    .map(|p| plan::preview_text(&p, |a| format.area(a), |l| format.length(l)));
                (*k, words)
            })
            .collect();
        self.gis.topology.previews = previews;
    }

    /// Whether İstisna yap marks (some chosen row is open) or unmarks.
    pub(crate) fn topology_marking(&self) -> bool {
        let Some(c) = self.topology_checked() else {
            return true;
        };
        self.gis.topology.selected.is_empty()
            || self
                .gis
                .topology
                .selected
                .iter()
                .any(|&i| c.findings.get(i).is_some_and(|f| !f.exception))
    }

    /// The tab's controls by their words (a trace's `panel` step): Denetle,
    /// Kurallar…, the filter, a rule, a row, Düzelt's fixes, İstisna yap.
    pub(crate) fn topology_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let msg = |e| Some(Message::Gis(gis::Event::Topology(e)));
        Ok(match control {
            Control::Press(texts::CHECK) => msg(Event::Check),
            Control::Press(texts::RULES) => msg(Event::Rules),
            Control::Press(texts::MARK) => (!self.gis.topology.selected.is_empty()
                && self.topology_marking())
            .then(|| Message::Gis(gis::Event::Topology(Event::Exception(true)))),
            Control::Press(texts::UNMARK) => (!self.gis.topology.selected.is_empty()
                && !self.topology_marking())
            .then(|| Message::Gis(gis::Event::Topology(Event::Exception(false)))),
            Control::Press(texts::ERROR) => (!self.gis.topology.selected.is_empty())
                .then(|| Message::Gis(gis::Event::Topology(Event::Error))),
            Control::Press(texts::RECIPE) => (!self.gis.topology.selected.is_empty())
                .then(|| Message::Gis(gis::Event::Topology(Event::Recipe))),
            Control::Press(texts::APPLY) => self
                .topology_recipe()
                .map(|_| Message::Gis(gis::Event::Topology(Event::Apply))),
            Control::Press(texts::DROP) => msg(Event::Drop),
            Control::Fill(texts::REASON, t) => msg(Event::Reason(t.to_owned())),
            Control::Press(texts::OPEN) => msg(Event::Filter(Filter::Open)),
            Control::Press(texts::EXCEPTIONS) => msg(Event::Filter(Filter::Exception)),
            Control::Press(texts::ALL) => msg(Event::Filter(Filter::All)),
            Control::Pick(texts::FIX, item) => {
                let [at] = self.gis.topology.selected[..] else {
                    return Err("Düzelt için tek bulgu seçilmeli".to_owned());
                };
                let c = self.topology_checked().ok_or("denetim yok")?;
                let f = c.findings.get(at).ok_or("bulgu yok")?;
                let key = f
                    .fixes
                    .iter()
                    .find(|k| core::fix_label(k) == item || self.topology_fix_label(f, k) == item)
                    .ok_or_else(|| format!("“{item}” düzeltmesi yok"))?;
                msg(Event::Fix(key))
            }
            Control::Pick(texts::RULE_PICK, item) => {
                if item == texts::ALL_RULES {
                    msg(Event::Rule(None))
                } else {
                    let rules = self.topology_rule_names();
                    let at = rules
                        .iter()
                        .position(|r| r == item)
                        .ok_or_else(|| format!("“{item}” kuralı yok"))?;
                    msg(Event::Rule(Some(at)))
                }
            }
            Control::Row(n) => {
                if n == 0 || n > self.topology_shown().len() {
                    return Err(format!("{n}. bulgu satırı yok"));
                }
                msg(Event::Press(n - 1))
            }
            other => return Err(format!("“Topoloji” sekmesinde {other} yok")),
        })
    }

    /// A fix's words in Düzelt ▾ (plan.rs's `fix_label`).
    pub(crate) fn topology_fix_label(&self, f: &Finding, key: &str) -> String {
        let ids: Vec<String> = self.topology_checked().map_or_else(Vec::new, Checked::object_words);
        let format = self.format();
        plan::fix_label(f, key, &ids, |m| format.length(m))
    }

    /// A layer's name as the drawing has it; its id when it has none.
    pub(crate) fn topology_layer_name(&self, id: &str) -> String {
        self.document
            .as_ref()
            .and_then(|doc| doc.model.layers().get(id).map(|n| n.name.clone()))
            .unwrap_or_else(|| id.to_owned())
    }

    /// The rules as the Kural list names them: “Parsel: Çakışmamalı”.
    pub(crate) fn topology_rule_names(&self) -> Vec<String> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        let layers = doc.model.layers();
        let name = |id: &str| {
            layers
                .get(id)
                .map_or_else(|| id.to_owned(), |n| n.name.clone())
        };
        let rules = self
            .topology_checked()
            .map(|c| c.g.rules.clone())
            .unwrap_or_else(|| {
                self.topology_settings()
                    .map(|t| t.rules.clone())
                    .unwrap_or_default()
            });
        rules
            .iter()
            .map(|r| format!("{}: {}", name(&r.layer), plan::rule_text(r, name)))
            .collect()
    }

    /// What a trace's `topology` expectation reads: the count line and the
    /// rows (Katman, Kural, Sorun, Nesneler, Ölçü, joined by “ | ”), as the
    /// page shows them.
    pub(crate) fn topology_seen(&self) -> Option<(String, Vec<String>)> {
        if !self.command_expanded || self.bottom_tab != BottomTab::Topology {
            return None;
        }
        let doc = self.document.as_ref()?;
        let c = self.topology_checked();
        let shown = self.topology_shown();
        let count = c
            .filter(|c| !c.findings.is_empty())
            .map(|c| {
                let open = c.findings.iter().filter(|f| !f.exception).count();
                plan::count_text(shown.len(), open, c.findings.len() - open)
            })
            .unwrap_or_default();
        let Some(c) = c else {
            return Some((count, Vec::new()));
        };
        let layers = doc.model.layers();
        let name = |id: &str| {
            layers
                .get(id)
                .map_or_else(|| id.to_owned(), |n| n.name.clone())
        };
        let ids: Vec<String> = c.object_words();
        let format = self.format();
        let rows = shown
            .iter()
            .map(|&i| {
                let f = &c.findings[i];
                let rule = &c.g.rules[f.rule];
                [
                    name(&rule.layer),
                    plan::rule_text(rule, name),
                    plan::problem_text(f, c.g.lapsed(f)),
                    plan::objects_text(f, &ids),
                    plan::measure_text(f, &format),
                ]
                .join(" | ")
            })
            .collect();
        Some((count, rows))
    }
}

/// Whether what check `c` read changed in `doc` since (docs/adr/9146). The rules are geometric: an object's style,
/// label or attributes are no part of them.
fn inputs_changed(doc: &kentos_domain::Document, c: &Checked) -> bool {
    use kentos_domain::Changes;
    if topology_check::rules_digest(doc, None) != c.g.stamp.rules {
        return true;
    }
    // A bound layer's source read at another version.
    let sources = &doc.settings().sources;
    if c.g.stamp.sources.iter().any(|s| sources.iter().find(|l| l.layer == s.layer).is_none_or(|l| l.revision != s.revision)) {
        return true;
    }
    let slots = match doc.changes_since(c.mark) {
        Changes::Slots(slots) => slots,
        Changes::All => return true,
    };
    if slots.is_empty() {
        return false;
    }
    let layers: std::collections::HashSet<&str> = c
        .g
        .rules
        .iter()
        .flat_map(|r| std::iter::once(r.layer.as_str()).chain(r.other.as_deref()))
        .collect();
    let was: std::collections::HashMap<Slot, usize> = c
        .g
        .slots
        .iter()
        .take(c.g.drawn)
        .enumerate()
        .filter_map(|(k, s)| s.map(|s| (s, k)))
        .collect();
    let mut seen: std::collections::HashSet<Slot> = std::collections::HashSet::new();
    for &slot in slots {
        if !seen.insert(slot) {
            continue;
        }
        let changed = match (doc.get(slot), was.get(&slot)) {
            // Made and removed since, or never on the rules' layers.
            (None, None) => false,
            (None, Some(_)) => true,
            (Some(e), None) => layers.contains(e.base().layer_id.as_str()),
            (Some(e), Some(&k)) => {
                c.g.layers.get(k).is_none_or(|l| *l != e.base().layer_id)
                    || c.g.shapes.get(k).is_none_or(|s| *s != kentos_native_application::geometry::shape(e))
            }
        };
        if changed {
            return true;
        }
    }
    false
}
