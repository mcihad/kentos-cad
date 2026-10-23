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

mod plan;
pub(crate) mod rules;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use plan::{Filter, texts};

use iced::Task;
use kentos_commands::{
    CommandResult, EditOperation, EntitiesEdit, EntityEdit, TopologyExceptionOperation,
    TopologyExceptions, TopologySettings,
};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::topology_rules::{self as core, Finding, fix, incremental};
use kentos_interaction::{Level, Vec2, ViewChange};
use kentos_native_application::geometry::edit_geometry;
use kentos_native_application::topology_check::{self, Gathered};
use kentos_native_application::{ExecutionContext, edit, topology_exceptions};

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::traces::Control;

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
    pub findings: Vec<Finding>,
    pub g: Gathered,
    /// The drawing (its session) and the state of it the check saw.
    pub session: u64,
    pub generation: u64,
    pub revision: u64,
    /// Whether the last check went over what changed alone (docs/adr/0524), and how many objects it looked at.
    pub incremental: Option<usize>,
}

impl Checked {
    /// A finding's lasting key (docs/adr/0524 §1).
    pub(crate) fn key(&self, f: &Finding) -> String {
        self.g.key(f)
    }

    /// The objects' ids as the table says them: a drawing's object its id, a row outside a part 0.
    pub(crate) fn ids(&self) -> Vec<u32> {
        self.g.slots.iter().map(|s| s.map_or(0, |s| s.0)).collect()
    }
}

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
        self.topology
            .checked
            .as_ref()
            .filter(|c| c.session == doc.session)
    }

    /// Whether the drawing changed since the check.
    pub(crate) fn topology_stale(&self) -> bool {
        let (Some(doc), Some(c)) = (self.document.as_ref(), self.topology_checked()) else {
            return false;
        };
        c.generation != doc.model.generation() || c.revision != doc.model.revision()
    }

    /// Denetle: every rule over the objects of the layers the rules name and the rows round their bound parts
    /// (docs/adr/0589); what it found said unless `quiet`.
    pub(crate) fn topology_check(&mut self, quiet: bool) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let model = &doc.model;
        let g = topology_check::gather(model, None);
        let found = topology_check::run(&g);
        let n_rules = g.rules.len();
        let n_objects = g.drawn;
        let missing = g.missing;
        let incomplete = g.incomplete.clone();
        let checked = Checked {
            findings: found.findings,
            g,
            session: doc.session,
            generation: model.generation(),
            revision: model.revision(),
            incremental: None,
        };
        let total = checked.findings.len();
        let open = checked.findings.iter().filter(|f| !f.exception).count();
        let lapsed = checked
            .findings
            .iter()
            .filter(|f| !f.exception && checked.g.lapsed(f))
            .count();
        self.topology.checked = Some(checked);
        self.topology.selected.clear();
        self.topology.anchor = None;
        self.topology.previews.clear();
        if quiet {
            return;
        }
        if missing > 0 {
            self.warn(plan::missing_layers(missing));
        }
        for gap in &incomplete {
            self.warn(plan::incomplete_text(&self.topology_layer_name(&gap.layer), &gap.reason));
        }
        if lapsed > 0 {
            self.warn(plan::lapsed_text(lapsed));
        }
        if total == 0 {
            if incomplete.is_empty() {
                self.say(Level::Success, plan::no_findings(n_rules, n_objects));
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

    /// Değişenleri denetle (docs/adr/0524): the rules over what changed since the check, its neighbours and the
    /// clusters a gap can span; the rest of the findings kept. Checked whole when there was no check, or the rules,
    /// the tolerance or the exceptions changed. The chosen rows follow their findings' keys.
    pub(crate) fn topology_recheck(&mut self, quiet: bool) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let model = &doc.model;
        let g = topology_check::gather(model, None);
        let Some(old) = self
            .topology_checked()
            .filter(|c| c.g.stamp.rules == g.stamp.rules && c.g.lapsed == g.lapsed)
            .cloned()
        else {
            return self.topology_check(quiet);
        };
        if !self.topology_stale() {
            return;
        }
        let chosen: Vec<String> = self
            .topology
            .selected
            .iter()
            .filter_map(|&i| old.findings.get(i).map(|f| old.key(f)))
            .collect();
        // What changed: an object added, removed, moved or put on another layer; where it was and is.
        let was: std::collections::HashMap<&str, (&Shape, &str)> = old
            .g
            .uids
            .iter()
            .zip(old.g.shapes.iter().zip(&old.g.layers))
            .map(|(u, (s, l))| (u.as_str(), (s, l.as_str())))
            .collect();
        let mut changed: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut touched = Vec::new();
        for (i, u) in g.uids.iter().enumerate() {
            match was.get(u.as_str()) {
                Some((s, l)) if **s == g.shapes[i] && *l == g.layers[i] => {}
                Some((s, _)) => {
                    changed.insert(u);
                    touched.push(incremental::shape_bounds(s));
                    touched.push(incremental::shape_bounds(&g.shapes[i]));
                }
                None => {
                    changed.insert(u);
                    touched.push(incremental::shape_bounds(&g.shapes[i]));
                }
            }
        }
        let now: std::collections::HashSet<&str> = g.uids.iter().map(String::as_str).collect();
        for (u, (s, _)) in &was {
            if !now.contains(u) {
                changed.insert(u);
                touched.push(incremental::shape_bounds(s));
            }
        }
        let n_changed = changed.len();
        let mut found = incremental::recheck(
            &g.objects(),
            &g.core_rules,
            g.tolerance,
            &g.exceptions,
            &incremental::Previous {
                findings: &old.findings,
                uids: &old.g.uids,
            },
            &changed,
            &touched,
        );
        // A part's rows alone are not its findings (docs/adr/0589 §1).
        found
            .findings
            .retain(|f| f.objects.is_empty() || f.objects.iter().any(|&k| !g.outside(k)));
        let checked = Checked {
            findings: found.findings,
            g,
            session: doc.session,
            generation: model.generation(),
            revision: model.revision(),
            incremental: Some(n_changed),
        };
        let total = checked.findings.len();
        let open = checked.findings.iter().filter(|f| !f.exception).count();
        let selected: Vec<usize> = checked
            .findings
            .iter()
            .enumerate()
            .filter(|(_, f)| chosen.contains(&checked.key(f)))
            .map(|(i, _)| i)
            .collect();
        self.topology.checked = Some(checked);
        self.topology.selected = selected;
        self.topology.anchor = None;
        self.topology_previews();
        if !quiet {
            self.say(
                Level::Info,
                format!(
                    "Değişenler denetlendi: {n_changed} nesne değişti; {total} bulgu (açık {open}, istisna {}).",
                    total - open
                ),
            );
        }
    }

    /// `topology.check`: the tab open and the check run (said when the project has no rules).
    pub(crate) fn open_topology_check(&mut self) -> Task<Message> {
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
            self.topology_check(false);
        } else {
            self.warn("Projede topoloji kuralı yok; Topoloji kuralları ile ekleyin.");
        }
        Task::none()
    }

    /// The rows shown now, as places in the findings.
    pub(crate) fn topology_shown(&self) -> Vec<usize> {
        self.topology_checked().map_or_else(Vec::new, |c| {
            let rule = self.topology.rule.filter(|&r| r < c.g.rules.len());
            plan::shown_findings(&c.findings, self.topology.filter, rule)
        })
    }

    /// The finding shown over the drawing: the one chosen row's.
    pub(crate) fn topology_problem(&self) -> Option<ProblemMark> {
        if self.topology.selected.len() != 1 {
            return None;
        }
        let f = self
            .topology_checked()?
            .findings
            .get(self.topology.selected[0])?;
        Some(ProblemMark {
            at: f.at,
            label: core::problem_label(f.problem).to_owned(),
            regions: f.regions.clone(),
            edges: f.edges.clone(),
        })
    }

    pub(crate) fn topology_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Check => self.topology_check(false),
            Event::Recheck => self.topology_recheck(false),
            Event::Rules => return self.open_topology_rules(),
            Event::Filter(f) => {
                self.topology.filter = f;
                self.topology.selected.clear();
                self.topology.anchor = None;
            }
            Event::Rule(r) => {
                self.topology.rule = r;
                self.topology.selected.clear();
                self.topology.anchor = None;
            }
            Event::Press(at) => self.topology_press(at),
            Event::Fix(key) => self.topology_fix(key),
            Event::Exception(on) => self.topology_exceptions(on),
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
        let (ctrl, shift) = (self.modifiers.control(), self.modifiers.shift());
        if ctrl || shift {
            let picked: Vec<Slot> = crate::points::click_pick(
                &self
                    .topology
                    .selected
                    .iter()
                    .map(|&i| Slot(i as u32))
                    .collect::<Vec<_>>(),
                &shown.iter().map(|&i| Slot(i as u32)).collect::<Vec<_>>(),
                at,
                self.topology.anchor,
                ctrl,
                shift,
            );
            if !shift {
                self.topology.anchor = Some(at);
            }
            self.topology.selected = picked.into_iter().map(|s| s.0 as usize).collect();
            self.topology_previews();
            return;
        }
        self.topology.anchor = Some(at);
        self.topology.selected = vec![finding];
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
        let view = plan::finding_view(&f.bounds);
        self.selection.set(slots);
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
    fn topology_fix(&mut self, key: &'static str) {
        if self.topology_stale() {
            self.warn("Çizim denetimden sonra değişti; önce Denetle ile yenileyin.");
            return;
        }
        let [at] = self.topology.selected[..] else {
            return;
        };
        let Some(c) = self.topology_checked() else {
            return;
        };
        let Some(f) = c.findings.get(at) else {
            return;
        };
        let changes = match fix(&c.g.objects(), &c.g.core_rules, f, key) {
            Ok(changes) => changes,
            Err(why) => {
                self.warn(why);
                return;
            }
        };
        if changes.iter().any(|ch| c.g.outside(ch.object)) {
            self.warn("Bu düzeltme alanın dışındaki bir satırı değiştirir; alanı genişletip satırı çalışma kopyasına alın.");
            return;
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
            return;
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
                return;
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => return,
        }
        self.topology_recheck(true);
        // The row at the same place, if any: the next finding of the list.
        if let Some(n) = self
            .topology_checked()
            .map(|c| c.findings.len())
            .filter(|n| *n > 0)
        {
            self.topology.selected = vec![at.min(n - 1)];
            self.topology_previews();
        }
    }

    /// Each fix of the one chosen finding previewed (docs/adr/0589 §4): its words in Düzelt ▾, with the measures it
    /// changes and the bound sources it would leave with rows to write.
    pub(crate) fn topology_previews(&mut self) {
        self.topology.previews.clear();
        let (Some(doc), Some(c)) = (self.document.as_ref(), self.topology_checked()) else {
            return;
        };
        let [at] = self.topology.selected[..] else {
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
        self.topology.previews = previews;
    }

    /// İstisna yap or İstisnayı kaldır on the chosen findings through `cad.topology.exceptions` (docs/adr/0589 §3):
    /// the project's settings (not an undo step), each exception bound to its objects' geometry; checked again.
    fn topology_exceptions(&mut self, on: bool) {
        let chosen = self.topology.selected.clone();
        let Some(c) = self.topology_checked() else {
            return;
        };
        let keys: Vec<String> = chosen
            .iter()
            .filter_map(|&at| c.findings.get(at))
            .filter(|f| {
                if on {
                    !f.exception
                } else {
                    f.exception || c.g.lapsed(f)
                }
            })
            .map(|f| c.key(f))
            .collect();
        if keys.is_empty() {
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = TopologyExceptions {
            operation: if on {
                TopologyExceptionOperation::Mark
            } else {
                TopologyExceptionOperation::Unmark
            },
            keys,
            expected_revision: None,
        };
        let n =
            match topology_exceptions::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                CommandResult::Completed { output, .. } => output.changed,
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    self.warn(error.message);
                    return;
                }
                CommandResult::Queued { .. } | CommandResult::Cancelled => return,
            };
        let keys: Vec<String> = chosen
            .iter()
            .filter_map(|&at| {
                self.topology_checked()
                    .and_then(|c| c.findings.get(at).map(|f| c.key(f)))
            })
            .collect();
        self.topology_check(true);
        // The chosen rows follow their keys.
        if let Some(c) = self.topology_checked() {
            self.topology.selected = c
                .findings
                .iter()
                .enumerate()
                .filter(|(_, f)| keys.contains(&c.key(f)))
                .map(|(i, _)| i)
                .collect();
        }
        self.topology_previews();
        self.say(
            Level::Info,
            if on {
                format!("{n} bulgu istisna yapıldı.")
            } else {
                format!("{n} bulgunun istisnası kaldırıldı.")
            },
        );
    }

    /// Whether İstisna yap marks (some chosen row is open) or unmarks.
    pub(crate) fn topology_marking(&self) -> bool {
        let Some(c) = self.topology_checked() else {
            return true;
        };
        self.topology.selected.is_empty()
            || self
                .topology
                .selected
                .iter()
                .any(|&i| c.findings.get(i).is_some_and(|f| !f.exception))
    }

    /// The tab's controls by their words (a trace's `panel` step): Denetle,
    /// Kurallar…, the filter, a rule, a row, Düzelt's fixes, İstisna yap.
    pub(crate) fn topology_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let msg = |e| Some(Message::Topology(e));
        Ok(match control {
            Control::Press(texts::CHECK) => msg(Event::Check),
            Control::Press(texts::RULES) => msg(Event::Rules),
            Control::Press(texts::MARK) => (!self.topology.selected.is_empty()
                && self.topology_marking())
            .then(|| Message::Topology(Event::Exception(true))),
            Control::Press(texts::UNMARK) => (!self.topology.selected.is_empty()
                && !self.topology_marking())
            .then(|| Message::Topology(Event::Exception(false))),
            Control::Press(texts::OPEN) => msg(Event::Filter(Filter::Open)),
            Control::Press(texts::EXCEPTIONS) => msg(Event::Filter(Filter::Exception)),
            Control::Press(texts::ALL) => msg(Event::Filter(Filter::All)),
            Control::Pick(texts::FIX, item) => {
                let [at] = self.topology.selected[..] else {
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
        let ids: Vec<u32> = self.topology_checked().map_or_else(Vec::new, Checked::ids);
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
        let ids: Vec<u32> = c.ids();
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
