//! Topoloji, the bottom panel's tab of the topology rules' findings
//! (docs/adr/0202 §3–§5; the web's ui/bottom/TopologyPanel.ts and
//! topologyRun.ts): Denetle runs the project's rules over the objects of the
//! layers they name (the shared core's `ops::topology_rules`), the findings
//! are rows; a row selects its objects, zooms to its box and shows it over
//! the drawing (marks.rs); Düzelt ▾ writes a fix through `cad.entities.edit`
//! (`topologyFix`, the step “Topoloji düzelt”) and checks again; İstisna yap
//! marks the chosen findings in the project's settings. The rules window is
//! `rules.rs`, the words and cells `plan.rs`, the view `view.rs`.

mod plan;
pub(crate) mod rules;
mod view;

#[cfg(test)]
mod tests;

pub(crate) use plan::{Filter, texts};

use iced::Task;
use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEdit, EntityEdit, TopologyException, TopologyRule,
    TopologySettings,
};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::topology_rules::{
    self as core, Exception, Finding, Kind, Objects, Rule, check, fix,
};
use kentos_interaction::{Level, Vec2, ViewChange};
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, edit};

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

/// The last check: its findings and what they were made of, when.
#[derive(Clone, Debug)]
pub(crate) struct Checked {
    pub findings: Vec<Finding>,
    /// Each object's slot, by the core's places.
    pub slots: Vec<Slot>,
    pub shapes: Vec<Shape>,
    pub layers: Vec<String>,
    pub uids: Vec<String>,
    /// The project's rules as checked, and as the core took them.
    pub rules: Vec<TopologyRule>,
    pub core_rules: Vec<Rule>,
    /// The drawing (its session) and the state of it the check saw.
    pub session: u64,
    pub generation: u64,
    pub revision: u64,
    /// Rules whose layer or other layer the project does not have.
    pub missing: usize,
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
}

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Check,
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

fn core_rules(rules: &[TopologyRule]) -> Vec<Rule> {
    rules
        .iter()
        .filter_map(|r| {
            Some(Rule {
                id: r.id.clone(),
                kind: Kind::of_key(r.kind.key())?,
                layer: r.layer.clone(),
                other: r.other.clone(),
                value: r.value,
            })
        })
        .collect()
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

    /// Denetle: every rule over the objects of the layers the rules name;
    /// what it found said unless `quiet`.
    pub(crate) fn topology_check(&mut self, quiet: bool) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let model = &doc.model;
        let settings = model.settings().topology.clone().unwrap_or_default();
        let leaves: Vec<String> = model
            .layers()
            .leaves()
            .iter()
            .map(|n| n.id.clone())
            .collect();
        let missing = settings
            .rules
            .iter()
            .filter(|r| {
                !leaves.contains(&r.layer) || r.other.as_ref().is_some_and(|o| !leaves.contains(o))
            })
            .count();
        let named: Vec<&str> = settings
            .rules
            .iter()
            .flat_map(|r| std::iter::once(r.layer.as_str()).chain(r.other.as_deref()))
            .collect();
        let objects: Vec<&kentos_contracts::Entity> = model
            .entities()
            .filter(|e| named.contains(&e.base().layer_id.as_str()))
            .collect();
        let slots: Vec<Slot> = objects.iter().map(|e| Slot(e.base().id)).collect();
        let shapes: Vec<Shape> = objects.iter().map(|e| shape(e)).collect();
        let layers: Vec<String> = objects.iter().map(|e| e.base().layer_id.clone()).collect();
        let uids: Vec<String> = slots
            .iter()
            .map(|s| model.uid(*s).map(|u| u.to_string()).unwrap_or_default())
            .collect();
        let rules = core_rules(&settings.rules);
        let exceptions: Vec<Exception> = settings
            .exceptions
            .iter()
            .map(|x| Exception {
                rule: x.rule.clone(),
                objects: x.objects.iter().map(|id| id.to_text()).collect(),
                at: Vec2::new(x.at.x, x.at.y),
            })
            .collect();
        let found = check(
            &Objects {
                shapes: &shapes,
                layers: &layers,
                uids: &uids,
            },
            &rules,
            settings.tolerance(),
            &exceptions,
        );
        let n_rules = settings.rules.len();
        let n_objects = objects.len();
        let checked = Checked {
            findings: found.findings,
            slots,
            shapes,
            layers,
            uids,
            rules: settings.rules,
            core_rules: rules,
            session: doc.session,
            generation: model.generation(),
            revision: model.revision(),
            missing,
        };
        let total = checked.findings.len();
        let open = checked.findings.iter().filter(|f| !f.exception).count();
        self.topology.checked = Some(checked);
        self.topology.selected.clear();
        self.topology.anchor = None;
        if quiet {
            return;
        }
        if missing > 0 {
            self.warn(plan::missing_layers(missing));
        }
        if total == 0 {
            self.say(Level::Success, plan::no_findings(n_rules, n_objects));
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
            let rule = self.topology.rule.filter(|&r| r < c.rules.len());
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
            return;
        }
        self.topology.anchor = Some(at);
        self.topology.selected = vec![finding];
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
            .filter_map(|&k| c.slots.get(k).copied())
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
    /// the check, the fix has no answer or the command refuses it.
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
        let objects = Objects {
            shapes: &c.shapes,
            layers: &c.layers,
            uids: &c.uids,
        };
        let changes = match fix(&objects, &c.core_rules, f, key) {
            Ok(changes) => changes,
            Err(why) => {
                self.warn(why);
                return;
            }
        };
        let edits: Vec<EntityEdit> = changes
            .into_iter()
            .filter_map(|ch| {
                let uid = c.uids.get(ch.object)?.clone();
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
        self.topology_check(true);
        // The row at the same place, if any: the next finding of the list.
        if let Some(n) = self
            .topology_checked()
            .map(|c| c.findings.len())
            .filter(|n| *n > 0)
        {
            self.topology.selected = vec![at.min(n - 1)];
        }
    }

    /// İstisna yap or İstisnayı kaldır on the chosen findings: the project's
    /// settings (not an undo step), the check's flags updated in place.
    fn topology_exceptions(&mut self, on: bool) {
        let chosen = self.topology.selected.clone();
        let (Some(c), Some(t)) = (self.topology_checked(), self.topology_settings()) else {
            return;
        };
        let tolerance = t.tolerance();
        let mut list: Vec<TopologyException> = t.exceptions.clone();
        let same = |x: &TopologyException, f: &Finding| {
            c.rules.get(f.rule).is_some_and(|r| r.id == x.rule)
                && x.objects.len() == f.objects.len()
                && x.objects
                    .iter()
                    .zip(&f.objects)
                    .all(|(u, &k)| c.uids.get(k).is_some_and(|v| u.to_text() == *v))
                && (x.at.x - f.at.x).hypot(x.at.y - f.at.y) <= tolerance
        };
        let mut n = 0;
        for &at in &chosen {
            let Some(f) = c.findings.get(at).filter(|f| f.exception != on) else {
                continue;
            };
            if on {
                let objects: Vec<kentos_contracts::EntityId> = f
                    .objects
                    .iter()
                    .filter_map(|&k| {
                        c.uids
                            .get(k)
                            .and_then(|u| kentos_contracts::EntityId::parse(u))
                    })
                    .collect();
                list.push(TopologyException {
                    rule: c.rules[f.rule].id.clone(),
                    objects,
                    at: kentos_contracts::Vec2 {
                        x: f.at.x,
                        y: f.at.y,
                    },
                });
            } else {
                list.retain(|x| !same(x, f));
            }
            n += 1;
        }
        if n == 0 {
            return;
        }
        let mut next = t.clone();
        next.exceptions = list;
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let mut settings = doc.model.settings().clone();
        settings.topology = Some(next);
        doc.model.set_settings(settings);
        let (generation, revision) = (doc.model.generation(), doc.model.revision());
        if let Some(c) = self.topology.checked.as_mut() {
            for &at in &chosen {
                if let Some(f) = c.findings.get_mut(at) {
                    f.exception = on;
                }
            }
            // The drawing did not change: the check holds.
            c.generation = generation;
            c.revision = revision;
        }
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
        let ids: Vec<u32> = self
            .topology_checked()
            .map_or_else(Vec::new, |c| c.slots.iter().map(|s| s.0).collect());
        let format = self.format();
        plan::fix_label(f, key, &ids, |m| format.length(m))
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
            .map(|c| c.rules.clone())
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
        let ids: Vec<u32> = c.slots.iter().map(|s| s.0).collect();
        let format = self.format();
        let rows = shown
            .iter()
            .map(|&i| {
                let f = &c.findings[i];
                let rule = &c.rules[f.rule];
                [
                    name(&rule.layer),
                    plan::rule_text(rule, name),
                    core::problem_label(f.problem).to_owned(),
                    plan::objects_text(f, &ids),
                    plan::measure_text(f, &format),
                ]
                .join(" | ")
            })
            .collect();
        Some((count, rows))
    }
}
