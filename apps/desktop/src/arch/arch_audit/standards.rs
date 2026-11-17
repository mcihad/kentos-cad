//! The Denetim tab's standards findings (bim-plan K10): the standards packages' Python rules, listed in the Mevzuat group
//! beside the regulation's other findings, each naming its article.
//!
//! - **Dijital proje** (`dijital_proje.mimari.denetle`): EK-3 and EK-4 (docs/adr/8064; plan/bim-plan.md BIM-03), the
//!   sheets from the model (docs/adr/8065, BIM-04) and the model quality control form (EK-9, docs/adr/8074), on the
//!   drawing's CAD facts (`kentos_processing::audit_facts::cad`) and on the facts of the IFC file KentOS would write now,
//!   read back (`crate::ifc_facts`).
//! - **Mimari mevzuat** (`mimari_mevzuat.mimari.denetle`, docs/adr/8078; BIM-13): the national and local regulation's
//!   articles by the version in force on the application date (PAİY 34/1 and 34/8), on the building facts
//!   (`kentos_processing::audit_facts::building`) and the permit's particulars. Its findings may be eksik veri or uzman
//!   incelemesi (`ArchAuditFinding::state`); its summary says each article's outcome and version above the table.
//!
//! A fix is the rule's KentOS command, run as any finding's: one undo step.
//!
//! - **Never in the way:** both rules run in their own short processes, one after the other, off the window's thread;
//!   an answer to an older ask (its generation) is dropped, and the last answer is put back after each check until the
//!   new one comes.
//! - **The whole drawing only:** layers, blocks, numbering and storeys are the drawing's, so a check of Etkin kat or of
//!   the objects selected leaves them out.

use std::collections::BTreeSet;

use iced::Task;
use kentos_commands::{
    ArchAuditCheck, ArchAuditFinding, ArchAuditFix, ArchAuditRuleSet, ArchAuditState, ChangesetStep, Vec2,
};
use kentos_extensions::audit_facts::ProjectAuditFacts;
use serde_json::{Value, json};

use super::{AuditScope, Event, msg};
use crate::app::{App, Message};

/// The packages and their functions.
const DIGITAL: (&str, &str) = ("dijital-proje", "dijital_proje.mimari");
const REGULATION: (&str, &str) = ("mimari-mevzuat", "mimari_mevzuat.mimari");
const FUNCTION: &str = "denetle";

/// The Dijital proje package's findings, by their codes' start (EK-3, EK-4, the sheets from the model, the form).
const CODES: &str = "digital_";

/// What both rules read, taken on the window's thread: the CAD facts, the model the written file's are taken from (none
/// for a drawing without walls) and the building facts.
struct Input {
    digital: Value,
    planned: Option<crate::ifc_facts::Planned>,
    regulation: Value,
}

impl App {
    /// The facts the Dijital proje rule reads first: the CAD drawing's section alone.
    pub(crate) fn arch_audit_standards_facts(&self) -> Option<Value> {
        let doc = &self.document.as_ref()?.model;
        let facts = ProjectAuditFacts::new(0.0, Vec::new(), false)
            .with_cad(kentos_processing::audit_facts::cad::measure(doc));
        serde_json::to_value(facts).ok()
    }

    /// The facts the national regulation's rule reads: the permit's particulars (the application date, the plan's
    /// storeys) and the building's.
    pub(crate) fn arch_audit_regulation_facts(&self) -> Option<Value> {
        let doc = &self.document.as_ref()?.model;
        let mut facts = ProjectAuditFacts::new(0.0, Vec::new(), false)
            .with_building(kentos_processing::audit_facts::building::measure(doc));
        if let Some(permit) = kentos_processing::audit_facts::permit::measure(doc) {
            facts = facts.with_permit(permit);
        }
        serde_json::to_value(facts).ok()
    }

    fn arch_audit_standards_input(&self) -> Option<Input> {
        let digital = self.arch_audit_standards_facts()?;
        let doc = &self.document.as_ref()?.model;
        Some(Input {
            digital,
            planned: crate::ifc_facts::plan(doc),
            regulation: self.arch_audit_regulation_facts()?,
        })
    }

    /// After a check: the last answer put back, and the rules asked again when the check was the whole drawing's.
    pub(super) fn arch_audit_standards_after_check(&mut self) {
        let panel = &mut self.arch.arch_audit;
        let whole = matches!(panel.scope, AuditScope::All);
        let last: BTreeSet<&str> = panel.standards.iter().map(|f| f.key.as_str()).collect();
        if let Some(a) = panel.audited.as_mut() {
            a.findings.retain(|f| !f.code.starts_with(CODES) && !last.contains(f.key.as_str()));
            if whole {
                a.findings.extend(panel.standards.iter().cloned());
            }
        }
        panel.standards_due = whole;
    }

    /// The rules' ask when a check made it due; their answer comes as [`Event::Standards`].
    pub(super) fn arch_audit_standards_task(&mut self) -> Task<Message> {
        if !std::mem::take(&mut self.arch.arch_audit.standards_due) {
            return Task::none();
        }
        // The facts and the model now, on this thread; the file written and read back, and the rules, off it.
        let Some(input) = self.arch_audit_standards_input() else {
            return Task::none();
        };
        self.arch.arch_audit.standards_generation += 1;
        let generation = self.arch.arch_audit.standards_generation;
        Task::perform(
            async move {
                let (sent, got) = iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let _ = sent.send(answer(input));
                });
                got.await.unwrap_or_else(|_| Err("Kuralın süreci cevap vermeden kapandı.".to_owned()))
            },
            move |answer| msg(Event::Standards(generation, answer)),
        )
    }

    /// The rules' answer: their findings in place of the last ones, for the ask still current. A package that gave no
    /// answer says why over the table, so a clean table is not taken for a pass.
    pub(super) fn arch_audit_standards_answer(&mut self, generation: u64, answer: Result<Value, String>) {
        if generation != self.arch.arch_audit.standards_generation {
            return;
        }
        let panel = &mut self.arch.arch_audit;
        let last: BTreeSet<String> = panel.standards.iter().map(|f| f.key.clone()).collect();
        let (digital, regulation) = match &answer {
            Ok(v) => (part(&v["digital"]), part(&v["regulation"])),
            Err(why) => (Err(why.clone()), Err(why.clone())),
        };
        let mut found = Vec::new();
        match digital {
            Ok(d) => {
                found.extend(findings(&d, true));
                panel.standards_failed = None;
            }
            Err(why) => panel.standards_failed = Some(why),
        }
        match regulation {
            Ok(r) => {
                found.extend(findings(&r["findings"], false));
                panel.rule_set = rule_set(&r);
                panel.regulation_failed = None;
            }
            Err(why) => {
                panel.rule_set = None;
                panel.regulation_failed = Some(why);
            }
        }
        panel.standards = found;
        if let Some(a) = panel.audited.as_mut()
            && matches!(panel.scope, AuditScope::All)
        {
            let kept = panel.selected.and_then(|i| a.findings.get(i)).map(|f| f.key.clone());
            a.findings.retain(|f| !f.code.starts_with(CODES) && !last.contains(&f.key));
            a.findings.extend(panel.standards.iter().cloned());
            panel.selected = kept.and_then(|k| a.findings.iter().position(|f| f.key == k));
            panel.chosen.retain(|i| *i < a.findings.len());
        }
    }

    /// The rules' ask and answer now, on this thread: what the tab's task does (tests and pictures).
    #[cfg(test)]
    pub(crate) fn arch_audit_standards_settle(&mut self) {
        let Some(input) = self.arch_audit_standards_input() else {
            return;
        };
        self.arch.arch_audit.standards_due = false;
        self.arch.arch_audit.standards_generation += 1;
        let generation = self.arch.arch_audit.standards_generation;
        let answer = answer(input);
        self.arch_audit_standards_answer(generation, answer);
    }
}

/// Both packages' answers, each its own or its error: the Dijital proje rule after the file is written and read back
/// (its failure the rule's), then the national regulation's.
fn answer(input: Input) -> Result<Value, String> {
    let digital = input
        .planned
        .map(crate::ifc_facts::Planned::facts)
        .transpose()
        .and_then(|ifc| {
            crate::standards::call(DIGITAL.0, DIGITAL.1, FUNCTION, vec![input.digital, ifc.unwrap_or(Value::Null)])
        });
    let regulation = crate::standards::call(REGULATION.0, REGULATION.1, FUNCTION, vec![input.regulation]);
    let side = |r: Result<Value, String>| r.map_or_else(|e| json!({ "error": e }), |v| json!({ "answer": v }));
    Ok(json!({ "digital": side(digital), "regulation": side(regulation) }))
}

/// The national regulation's rule set its answer gives (its `rules`, `date`, `dateNote`).
pub(crate) fn rule_set(answer: &Value) -> Option<ArchAuditRuleSet> {
    serde_json::from_value(serde_json::json!({
        "date": answer.get("date"),
        "dateNote": answer.get("dateNote"),
        "rules": answer.get("rules"),
    }))
    .ok()
}

/// One package's side of the answer.
fn part(side: &Value) -> Result<Value, String> {
    match side.get("answer") {
        Some(v) => Ok(v.clone()),
        None => Err(side.get("error").and_then(Value::as_str).unwrap_or("Kural cevap vermedi.").to_owned()),
    }
}

/// A rule's findings as the tab's: the Mevzuat group's, the key `code:uids`, the rule's words, its state, its fix as
/// one step. `digital`: only the Dijital proje package's codes are taken (a broken tag or dimension is the audit's own
/// Etiketler or Ölçüler finding already). Information only (a reading the administration confirms) is said in the
/// repair, never as a finding of its own kind.
fn findings(answer: &Value, digital: bool) -> Vec<ArchAuditFinding> {
    let mut out: Vec<ArchAuditFinding> = Vec::new();
    for f in answer.as_array().into_iter().flatten() {
        let text = |k: &str| f.get(k).and_then(Value::as_str).unwrap_or_default().to_owned();
        let code = text("code");
        if digital && !code.starts_with(CODES) {
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
        let state = match f.get("state").and_then(Value::as_str) {
            Some("missingData") => Some(ArchAuditState::MissingData),
            Some("expertReview") => Some(ArchAuditState::ExpertReview),
            _ => None,
        };
        let mut key = format!("{code}:{}", uids.join(","));
        if uids.is_empty() {
            // A layer's, a typeface's or a building's finding names no object: its message tells it apart.
            key.push(':');
            key.push_str(&text("message"));
        }
        while out.iter().any(|o| o.key == key) {
            key.push_str("#2");
        }
        let rule = Some(text(if digital { "cite" } else { "rule" })).filter(|r| !r.is_empty());
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
            rule,
            state,
        });
    }
    out
}
