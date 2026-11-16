//! The Denetim tab's Dijital proje findings: EK-3 and EK-4's (docs/adr/8064; plan/bim-plan.md BIM-03), the
//! sheets-from-the-model ones (docs/adr/8065, BIM-04) and the model quality control form's (EK-9, docs/adr/8074), the
//! standards package's Python rules (`dijital_proje.mimari.denetle`, bim-plan K10) on the drawing's CAD facts
//! (`kentos_processing::audit_facts::cad`) and on the facts of the IFC file KentOS would write now, read back
//! (`crate::ifc_facts`). Asked after each check of the whole drawing and listed in the Mevzuat group beside the
//! regulation's other findings, each naming the article and the annex's clause and page. A fix is the rule's KentOS
//! command, run as any finding's: one undo step.
//!
//! - **Never in the way:** the rule runs in its own short process, off the window's thread; an answer to an older ask
//!   (its generation) is dropped, and the last answer is put back after each check until the new one comes.
//! - **The whole drawing only:** layers, blocks and numbering are the drawing's, so a check of Etkin kat or of the objects
//!   selected leaves them out.

use iced::Task;
use kentos_commands::{ArchAuditCheck, ArchAuditFinding, ArchAuditFix, ChangesetStep, Vec2};
use kentos_extensions::audit_facts::ProjectAuditFacts;
use serde_json::Value;

use super::{AuditScope, Event, msg};
use crate::app::{App, Message};

/// The standards package and its function.
const PACKAGE: &str = "dijital-proje";
const MODULE: &str = "dijital_proje.mimari";
const FUNCTION: &str = "denetle";

/// The findings this module adds, by their codes' start: every Dijital proje finding is the package's (EK-3, EK-4, the
/// sheets from the model, the model quality control form's 21 questions).
const CODES: &str = "digital_";

/// Whether a finding is this module's.
fn ours(code: &str) -> bool {
    code.starts_with(CODES)
}

impl App {
    /// The facts the rule reads first: the CAD drawing's section alone.
    pub(crate) fn arch_audit_standards_facts(&self) -> Option<Value> {
        let doc = &self.document.as_ref()?.model;
        let facts = ProjectAuditFacts::new(0.0, Vec::new(), false)
            .with_cad(kentos_processing::audit_facts::cad::measure(doc));
        serde_json::to_value(facts).ok()
    }

    /// Both facts the rule reads: the drawing's, and the model the written file's are taken from (none for a drawing
    /// without walls, whose rule then reads the drawing's alone).
    fn arch_audit_standards_input(&self) -> Option<(Value, Option<crate::ifc_facts::Planned>)> {
        let facts = self.arch_audit_standards_facts()?;
        let doc = &self.document.as_ref()?.model;
        Some((facts, crate::ifc_facts::plan(doc)))
    }

    /// After a check: the last answer put back, and the rule asked again when the check was the whole drawing's.
    pub(super) fn arch_audit_standards_after_check(&mut self) {
        let panel = &mut self.arch.arch_audit;
        let whole = matches!(panel.scope, AuditScope::All);
        if let Some(a) = panel.audited.as_mut() {
            a.findings.retain(|f| !ours(&f.code));
            if whole {
                a.findings.extend(panel.standards.iter().cloned());
            }
        }
        panel.standards_due = whole;
    }

    /// The rule's ask when a check made it due; its answer comes as [`Event::Standards`].
    pub(super) fn arch_audit_standards_task(&mut self) -> Task<Message> {
        if !std::mem::take(&mut self.arch.arch_audit.standards_due) {
            return Task::none();
        }
        // The drawing's facts and the model now, on this thread; the file written and read back, and the rule, off it.
        let Some((facts, planned)) = self.arch_audit_standards_input() else {
            return Task::none();
        };
        self.arch.arch_audit.standards_generation += 1;
        let generation = self.arch.arch_audit.standards_generation;
        Task::perform(
            async move {
                let (sent, got) = iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let _ = sent.send(answer(facts, planned));
                });
                got.await.unwrap_or_else(|_| Err("Kuralın süreci cevap vermeden kapandı.".to_owned()))
            },
            move |answer| msg(Event::Standards(generation, answer)),
        )
    }

    /// The rule's answer: its findings in place of the last ones, for the ask still current.
    pub(super) fn arch_audit_standards_answer(&mut self, generation: u64, answer: Result<Value, String>) {
        if generation != self.arch.arch_audit.standards_generation {
            return;
        }
        match answer {
            Ok(answer) => {
                self.arch.arch_audit.standards = findings(&answer);
                self.arch.arch_audit.standards_failed = None;
            }
            Err(why) => {
                self.arch.arch_audit.standards = Vec::new();
                self.arch.arch_audit.standards_failed = Some(why);
            }
        }
        let panel = &mut self.arch.arch_audit;
        if let Some(a) = panel.audited.as_mut()
            && matches!(panel.scope, AuditScope::All)
        {
            let kept = panel.selected.and_then(|i| a.findings.get(i)).map(|f| f.key.clone());
            a.findings.retain(|f| !ours(&f.code));
            a.findings.extend(panel.standards.iter().cloned());
            panel.selected = kept.and_then(|k| a.findings.iter().position(|f| f.key == k));
            panel.chosen.retain(|i| *i < a.findings.len());
        }
    }

    /// The rule's ask and answer now, on this thread: what the tab's task does (tests and pictures).
    #[cfg(test)]
    pub(crate) fn arch_audit_standards_settle(&mut self) {
        let Some((facts, planned)) = self.arch_audit_standards_input() else {
            return;
        };
        self.arch.arch_audit.standards_due = false;
        self.arch.arch_audit.standards_generation += 1;
        let generation = self.arch.arch_audit.standards_generation;
        let answer = answer(facts, planned);
        self.arch_audit_standards_answer(generation, answer);
    }
}

/// The rule's answer for the drawing's facts and the model: the file written and read back first (its failure the
/// answer's), then the rule on both.
fn answer(facts: Value, planned: Option<crate::ifc_facts::Planned>) -> Result<Value, String> {
    let ifc = planned.map(crate::ifc_facts::Planned::facts).transpose()?.unwrap_or(Value::Null);
    crate::standards::call(PACKAGE, MODULE, FUNCTION, vec![facts, ifc])
}

/// The rule's findings as the tab's: the Mevzuat group's, the key `code:uids`, the rule's words, its fix as one step.
/// Information only (a reading the administration confirms) is said in the repair, never as a finding of its own kind.
fn findings(answer: &Value) -> Vec<ArchAuditFinding> {
    let mut out: Vec<ArchAuditFinding> = Vec::new();
    for f in answer.as_array().into_iter().flatten() {
        let text = |k: &str| f.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
        let code = text("code");
        // A broken tag or dimension is the audit's own Etiketler or Ölçüler finding already.
        if !ours(&code) {
            continue;
        }
        let uids: Vec<String> =
            f.get("uids").and_then(Value::as_array).into_iter().flatten().filter_map(|u| u.as_str().map(str::to_owned)).collect();
        let at = f
            .get("at")
            .and_then(Value::as_array)
            .and_then(|a| Some(Vec2 { x: a.first()?.as_f64()?, y: a.get(1)?.as_f64()? }))
            .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
        let fix = f.get("fix").filter(|x| !x.is_null()).and_then(|x| {
            Some(ArchAuditFix {
                label: x.get("label")?.as_str()?.to_owned(),
                step: ChangesetStep {
                    command: x.get("command")?.as_str()?.to_owned(),
                    version: None,
                    input: x.get("input")?.clone(),
                    name: None,
                },
            })
        });
        let mut key = format!("{code}:{}", uids.join(","));
        if uids.is_empty() {
            // A layer's or a typeface's finding names no object: its message tells it apart.
            key.push(':');
            key.push_str(&text("message"));
        }
        while out.iter().any(|o| o.key == key) {
            key.push_str("#2");
        }
        out.push(ArchAuditFinding {
            key,
            check: ArchAuditCheck::Regulation,
            code,
            message: text("message"),
            repair: text("repair"),
            uids,
            at,
            fix,
            exception: false,
            lapsed: false,
            note: None,
            nature: None,
            rule: Some(text("cite")),
        });
    }
    out
}
