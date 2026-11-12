//! Topoloji's review and repair recipe (docs/adr/1260): the chosen findings marked as exceptions, confirmed as errors
//! or unmarked through `cad.topology.exceptions` with the reason typed in Gerekçe (a project with a quality profile
//! refuses a mark without one), and a repair recipe of the chosen findings in the order shown, each with its first
//! fix: `cad.topology.repair` plans it first and the tab shows what it would do (the objects, the source vertices it
//! moves and how far, the area it changes, the open findings before and after); Uygula writes it as one undo step, Vazgeç
//! drops it. No blind “repair all”: the recipe is what was chosen, seen before it is written.

use iced::Task;
use kentos_commands::{
    CommandResult, TopologyExceptionOperation, TopologyExceptions, TopologyFindingState,
    TopologyRepair, TopologyRepairStep, TopologyRepaired,
};
use kentos_geometry_core::ops::topology_rules::Finding;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, topology_exceptions, topology_repair};

use super::running::Work;
use super::{Checked, Then};
use crate::app::{App, Message};

/// A recipe planned and shown, bound to the drawing's state it was planned on.
#[derive(Clone, Debug)]
pub(crate) struct Recipe {
    pub input: TopologyRepair,
    pub planned: TopologyRepaired,
    pub session: u64,
    pub generation: u64,
    pub revision: u64,
}

/// A finding's place in its life cycle as the Durum column says it (docs/adr/1260 §1), with its reason.
pub(crate) fn state_text(c: &Checked, f: &Finding) -> String {
    let words = match c.g.state(f) {
        TopologyFindingState::Open => "Değerlendirilmedi",
        TopologyFindingState::Error => "Hata",
        TopologyFindingState::Exception => "İstisna",
        TopologyFindingState::Lapsed => "Yeniden değerlendir",
    };
    match c.g.review(f).and_then(|r| r.reason.as_deref()) {
        Some(why) => format!("{words}: {why}"),
        None => words.to_owned(),
    }
}

impl App {
    /// İstisna yap, Hata işaretle or İşareti kaldır on the chosen findings (docs/adr/0589 §3, 1260 §1): the project's
    /// settings (not an undo step), each mark bound to its objects' geometry and its rule's version, with Gerekçe's
    /// words; checked again, the chosen rows following their keys.
    pub(super) fn topology_review(&mut self, op: TopologyExceptionOperation) -> Task<Message> {
        let Some(c) = self.topology_checked() else {
            return Task::none();
        };
        let keys: Vec<String> = self.topology.selected.iter().filter_map(|&at| c.findings.get(at)).map(|f| c.key(f)).collect();
        self.topology_review_keys(keys.clone(), op, keys)
    }

    /// The findings of `keys` marked (those already so left out), from the tab or from Öznitelikler (docs/adr/4600);
    /// checked again, then the rows of `chosen` are the tab's chosen ones.
    pub(super) fn topology_review_keys(&mut self, keys: Vec<String>, op: TopologyExceptionOperation, chosen: Vec<String>) -> Task<Message> {
        let Some(c) = self.topology_checked() else {
            return Task::none();
        };
        let asked: std::collections::HashSet<&str> = keys.iter().map(String::as_str).collect();
        let todo: Vec<String> = c
            .findings
            .iter()
            .filter(|f| asked.contains(c.key(f).as_str()))
            .filter(|f| {
                let state = c.g.state(f);
                match op {
                    TopologyExceptionOperation::Mark => state != TopologyFindingState::Exception,
                    TopologyExceptionOperation::Error => state != TopologyFindingState::Error,
                    TopologyExceptionOperation::Unmark => state != TopologyFindingState::Open,
                }
            })
            .map(|f| c.key(f))
            .collect();
        if todo.is_empty() {
            return Task::none();
        }
        let reason = self.topology.reason.trim().to_owned();
        let reason = (op != TopologyExceptionOperation::Unmark && !reason.is_empty()).then_some(reason);
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = TopologyExceptions {
            operation: op,
            keys: todo,
            reason,
            scope: None,
            expected_revision: None,
        };
        let n =
            match topology_exceptions::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                CommandResult::Completed { output, .. } => output.changed,
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    self.warn(error.message);
                    return Task::none();
                }
                CommandResult::Queued { .. } | CommandResult::Cancelled => return Task::none(),
            };
        let said = match op {
            TopologyExceptionOperation::Mark => format!("{n} bulgu istisna yapıldı."),
            TopologyExceptionOperation::Error => format!("{n} bulgu hata diye işaretlendi."),
            TopologyExceptionOperation::Unmark => format!("{n} bulgunun istisnası kaldırıldı."),
        };
        self.topology.recipe = None;
        // Checked again (the marks are part of the rules' stamp); the chosen rows follow their keys.
        self.topology_run(Work::Check, Then::Marked { keys: chosen, said })
    }

    /// Tarifle onar: the chosen findings in the order shown, each with its first fix, planned by `cad.topology.repair`
    /// and shown; nothing is written. Refused (and said) when the drawing changed since the check or a chosen finding
    /// has no fix.
    pub(super) fn topology_plan_recipe(&mut self) {
        self.topology.recipe = None;
        if self.topology_stale() {
            self.warn("Çizim denetimden sonra değişti; önce Denetle ile yenileyin.");
            return;
        }
        let shown = self.topology_shown();
        let Some(c) = self.topology_checked() else {
            return;
        };
        let mut steps = Vec::new();
        for &at in shown.iter().filter(|at| self.topology.selected.contains(at)) {
            let Some(f) = c.findings.get(at) else {
                continue;
            };
            let Some(fix) = f.fixes.first() else {
                let problem = kentos_geometry_core::ops::topology_rules::problem_label(f.problem);
                self.warn(format!(
                    "Seçili “{problem}” bulgusunun düzeltmesi yok; tariften çıkarın."
                ));
                return;
            };
            steps.push(TopologyRepairStep {
                key: c.key(f),
                fix: (*fix).to_owned(),
            });
        }
        if steps.is_empty() {
            return;
        }
        let input = TopologyRepair {
            steps,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let (session, generation, revision) =
            (doc.session, doc.model.generation(), doc.model.revision());
        let cx = ExecutionContext::new(&mut doc.model);
        match topology_repair::plan(&cx, &input) {
            CommandResult::Completed { output, .. } => {
                self.topology.recipe = Some(Recipe {
                    input,
                    planned: output,
                    session,
                    generation,
                    revision,
                });
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// The recipe shown, while the drawing is as it was planned on.
    pub(crate) fn topology_recipe(&self) -> Option<&Recipe> {
        let doc = self.document.as_ref()?;
        self.topology.recipe.as_ref().filter(|r| {
            r.session == doc.session
                && r.generation == doc.model.generation()
                && r.revision == doc.model.revision()
        })
    }

    /// Uygula: the recipe planned, written as one undo step; checked again over what it changed.
    pub(super) fn topology_apply_recipe(&mut self) -> Task<Message> {
        let Some(recipe) = self.topology_recipe().cloned() else {
            self.topology.recipe = None;
            return Task::none();
        };
        self.topology.recipe = None;
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        match topology_repair::execute(&mut ExecutionContext::new(&mut doc.model), recipe.input) {
            CommandResult::Completed { output, warnings } => {
                self.say(
                    Level::Success,
                    format!(
                        "Topoloji onarıldı: {} adım, {} nesne; açık bulgu {} → {}.",
                        output.recipe.len(),
                        output.changes.len(),
                        output.open_before,
                        output.open_after
                    ),
                );
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
        let Some(old) = self.topology_checked().cloned() else {
            return self.topology_check(true);
        };
        self.topology_run(Work::Recheck(Box::new(old)), Then::Fixed { at: 0 })
    }

    /// What the recipe would do, in the project's units: its steps, the objects, the source vertices moved and the
    /// farthest, the change of the areas' sum, the open findings before and after.
    pub(crate) fn topology_recipe_text(&self, r: &Recipe) -> String {
        let format = self.format();
        let p = &r.planned;
        let removed = p.changes.iter().filter(|c| c.removed).count();
        let mut parts = vec![
            format!("{} adım", p.recipe.len()),
            if removed > 0 {
                format!("{} nesne değişir ({removed} silinir)", p.changes.len())
            } else {
                format!("{} nesne değişir", p.changes.len())
            },
        ];
        parts.push(if p.moved == 0 {
            "kaynak köşeler yerinde".to_owned()
        } else {
            format!(
                "{} kaynak köşe taşınır, en çok {}",
                p.moved,
                format.length(p.max_moved)
            )
        });
        if let Some(n) = p.below_precision.filter(|n| *n > 0) {
            parts.push(format!("{n} köşe depolama hassasiyetinin altında taşınır"));
        }
        parts.push(format!("alan farkı {}", format.area(p.area_change)));
        parts.push(format!("açık bulgu {} → {}", p.open_before, p.open_after));
        format!("Onarım tarifi: {}", parts.join(" · "))
    }
}
