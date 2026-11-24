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
//! - **Never in the way:** the facts, the model's mapping (docs/adr/8115: 3 s at a hundred flats) and both rules are
//!   worked out off the window's thread, on a reading copy of the drawing taken when the check is asked; the rules run
//!   in their own short processes, one after the other; an answer to an older ask (its generation) is dropped, and the
//!   last answer is put back after each check until the new one comes.
//! - **The model the form reads** (docs/adr/8116; Dijital Proje Yön. Md. 15, plan/bim-plan.md K4): the drawing's model
//!   now, written in memory and read back; or, when asked, the last delivered model as it went (its folder's IFC file,
//!   `kentos_native_application::delivered_model`). Either way the line over the table names the last delivered model
//!   and how the drawing differs from it, or why it could not be read.
//! - **The whole drawing only:** layers, blocks, numbering and storeys are the drawing's, so a check of Etkin kat or of
//!   the objects selected leaves them out.

use std::collections::BTreeSet;
use crate::arch::ArchState;

use iced::Task;
use kentos_commands::{
    ArchAuditCheck, ArchAuditFinding, ArchAuditFix, ArchAuditRuleSet, ArchAuditState, ChangesetStep, Vec2,
};
use kentos_extensions::audit_facts::ProjectAuditFacts;
use kentos_native_application::delivered_model::{self, Counted, Drift, Expected, Last};
use serde_json::{Value, json};

use super::{AuditScope, Event, msg};
use crate::app::{App, Message};

/// The packages and their functions.
const DIGITAL: (&str, &str) = ("dijital-proje", "dijital_proje.mimari");
const REGULATION: (&str, &str) = ("mimari-mevzuat", "mimari_mevzuat.mimari");
const FUNCTION: &str = "denetle";

/// The Dijital proje package's findings, by their codes' start (EK-3, EK-4, the sheets from the model, the form).
const CODES: &str = "digital_";

/// What both rules read: the drawing as it was when the check was asked, a reading copy (its objects shared, no undo
/// history); its facts and its model are worked out from it on the rules' thread.
pub(crate) struct Input {
    copy: kentos_domain::Document,
    /// The form reads the last delivered model.
    delivered: bool,
}

/// What the last answer said of the last delivered model (docs/adr/8116): the line over the table.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeliveredLine {
    pub number: u32,
    /// The model's file name and its delivery's day (YYYY-AA-GG); empty when its package could not be read.
    pub model: String,
    pub at: String,
    /// Why it could not be read as it went; none: it was.
    pub error: Option<String>,
    /// The drawing now against it; none: the drawing writes no model now, or the file is older (`older`).
    pub drift: Option<Drift>,
    /// The file was written before KentOS kept every GlobalId: not compared.
    pub older: bool,
    /// The form's answer came from it.
    pub used: bool,
    /// The source's objects were given to the form (from the drawing the package carries, or the drawing now when its
    /// model is the delivered one); not: EK-9/8, /17 and /21 are left to review.
    pub expectations: bool,
}

impl DeliveredLine {
    fn of(v: &Value) -> Option<Self> {
        let number = u32::try_from(v.get("number")?.as_u64()?).ok()?;
        let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
        let drift = v.get("drift").filter(|d| d.is_object()).map(|d| {
            let n = |k: &str| d.get(k).and_then(Value::as_u64).and_then(|n| usize::try_from(n).ok()).unwrap_or(0);
            Drift { added: n("added"), removed: n("removed"), changed: n("changed") }
        });
        Some(Self {
            number,
            model: text("model").unwrap_or_default(),
            at: text("at").unwrap_or_default(),
            error: text("error"),
            drift,
            older: v.get("older").and_then(Value::as_bool).unwrap_or(false),
            used: v.get("used").and_then(Value::as_bool).unwrap_or(false),
            expectations: v.get("expected").and_then(Value::as_bool).unwrap_or(true),
        })
    }

    /// The line's words.
    pub(crate) fn words(&self) -> String {
        let n = self.number;
        if let Some(why) = &self.error {
            return format!("{n}. teslimin modeli okunamadı: {why} EK-9 formu çizimin şimdiki hâlinden.");
        }
        let file = format!("{}, {}", self.model, day(&self.at));
        let drift = match self.drift {
            None if self.older => "Model, KentOS kalıcı kimlikleri yazmadan önce yazılmış; çizimle nesne nesne \
                                   karşılaştırılamadı."
                .to_owned(),
            None => "Çizimde şimdi bina yok.".to_owned(),
            Some(d) if d.is_none() => "Çizimin modeli teslim edilenle aynı.".to_owned(),
            Some(d) => {
                let parts: Vec<String> = [(d.changed, "değişti"), (d.added, "eklendi"), (d.removed, "silindi")]
                    .into_iter()
                    .filter(|(k, _)| *k > 0)
                    .map(|(k, what)| format!("{k} nesne {what}"))
                    .collect();
                format!("Teslimden bu yana çizimde {}.", parts.join(", "))
            }
        };
        if self.used {
            let mut words = format!("EK-9 formu {n}. teslimin modelinden ({file}). {drift}");
            if !self.expectations {
                words.push_str(" Paket kaynak çizimini taşımadığından 8., 17. ve 21. sorular incelemeye kaldı.");
            }
            words
        } else {
            format!("EK-9 formu çizimin şimdiki hâlinden. {n}. teslimin modeli: {file}. {drift}")
        }
    }

    /// The button's words and the model it asks (true: the delivered one); none: no button. `asked`: the panel asks
    /// for the delivered model now.
    pub(crate) fn switch(&self, asked: bool) -> Option<(&'static str, bool)> {
        if self.used || (asked && self.error.is_some()) {
            Some(("Şimdiki çizimi denetle", false))
        } else if self.error.is_none() {
            Some(("Teslim edileni denetle", true))
        } else {
            None
        }
    }
}

/// A day (YYYY-AA-GG) as people write it (GG.AA.YYYY); anything else as it is.
fn day(at: &str) -> String {
    match at.split('-').collect::<Vec<_>>().as_slice() {
        [y, m, d] if y.len() == 4 && m.len() == 2 && d.len() == 2 => format!("{d}.{m}.{y}"),
        _ => at.to_owned(),
    }
}

/// The facts the Dijital proje rule reads first: the CAD drawing's section alone.
fn standards_facts(doc: &kentos_domain::Document) -> Option<Value> {
    let facts =
        ProjectAuditFacts::new(0.0, Vec::new(), false).with_cad(kentos_processing::audit_facts::cad::measure(doc));
    serde_json::to_value(facts).ok()
}

/// The facts the national regulation's rule reads: the permit's particulars (the application date, the plan's storeys)
/// and the building's.
fn regulation_facts(doc: &kentos_domain::Document) -> Option<Value> {
    let mut facts = ProjectAuditFacts::new(0.0, Vec::new(), false)
        .with_building(kentos_processing::audit_facts::building::measure(doc));
    if let Some(permit) = kentos_processing::audit_facts::permit::measure(doc) {
        facts = facts.with_permit(permit);
    }
    serde_json::to_value(facts).ok()
}

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait ArchAuditStandardsApp {
    #[cfg(test)]
    fn arch_audit_standards_facts(&self) -> Option<Value>;
    fn arch_audit_regulation_facts(&self) -> Option<Value>;
    fn arch_audit_standards_input(&self) -> Option<Input>;
    fn arch_audit_standards_after_check(&mut self);
    fn arch_audit_standards_task(&mut self) -> Task<Message>;
    fn arch_audit_standards_answer(&mut self, generation: u64, answer: Result<Value, String>);
    #[cfg(test)]
    fn arch_audit_standards_settle(&mut self);
}

impl ArchAuditStandardsApp for App {
    /// The facts the Dijital proje rule reads first: the CAD drawing's section alone (the permit gate measures its own,
    /// permit_gate.rs).
    #[cfg(test)]
    fn arch_audit_standards_facts(&self) -> Option<Value>{
        standards_facts(&self.document.as_ref()?.model)
    }

    /// The facts the national regulation's rule reads: the permit's particulars (the application date, the plan's
    /// storeys) and the building's.
    fn arch_audit_regulation_facts(&self) -> Option<Value>{
        regulation_facts(&self.document.as_ref()?.model)
    }

    fn arch_audit_standards_input(&self) -> Option<Input>{
        Some(Input {
            copy: self.document.as_ref()?.model.reading_copy(),
            delivered: self.modules.arch().arch_audit.delivered_model,
        })
    }

    /// After a check: the last answer put back, and the rules asked again when the check was the whole drawing's.
    fn arch_audit_standards_after_check(&mut self){
        let doc = self.document.as_ref().map(|d| &d.model);
        let panel = &mut self.modules.arch_mut().arch_audit;
        let whole = matches!(panel.scope, AuditScope::All);
        let last: BTreeSet<&str> = panel.standards.iter().map(|f| f.key.as_str()).collect();
        if let Some(a) = panel.audited.as_mut() {
            a.findings.retain(|f| !f.code.starts_with(CODES) && !last.contains(f.key.as_str()));
            if whole && let Some(doc) = doc {
                put(doc, panel.view, &panel.standards, a);
            }
        }
        panel.standards_due = whole;
    }

    /// The rules' ask when a check made it due; their answer comes as [`Event::Standards`].
    fn arch_audit_standards_task(&mut self) -> Task<Message>{
        if !std::mem::take(&mut self.modules.arch_mut().arch_audit.standards_due) {
            return Task::none();
        }
        // The drawing now, a reading copy; its facts, its model, the file written and read back and the rules off this
        // thread.
        let Some(input) = self.arch_audit_standards_input() else {
            return Task::none();
        };
        self.modules.arch_mut().arch_audit.standards_generation += 1;
        let generation = self.modules.arch().arch_audit.standards_generation;
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
    fn arch_audit_standards_answer(&mut self, generation: u64, answer: Result<Value, String>){
        if generation != self.modules.arch().arch_audit.standards_generation {
            return;
        }
        let doc = self.document.as_ref().map(|d| &d.model);
        let panel = &mut self.modules.arch_mut().arch_audit;
        let last: BTreeSet<String> = panel.standards.iter().map(|f| f.key.clone()).collect();
        let (digital, regulation) = match &answer {
            Ok(v) => (part(&v["digital"]), part(&v["regulation"])),
            Err(why) => (Err(why.clone()), Err(why.clone())),
        };
        panel.delivered = answer.as_ref().ok().and_then(|v| DeliveredLine::of(&v["delivered"]));
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
            if let Some(doc) = doc {
                put(doc, panel.view, &panel.standards, a);
            }
            panel.selected = kept.and_then(|k| a.findings.iter().position(|f| f.key == k));
            panel.chosen.retain(|i| *i < a.findings.len());
        }
    }

    /// The rules' ask and answer now, on this thread: what the tab's task does (tests and pictures).
    #[cfg(test)]
    fn arch_audit_standards_settle(&mut self){
        let Some(input) = self.arch_audit_standards_input() else {
            return;
        };
        self.modules.arch_mut().arch_audit.standards_due = false;
        self.modules.arch_mut().arch_audit.standards_generation += 1;
        let generation = self.modules.arch().arch_audit.standards_generation;
        let answer = answer(input);
        self.arch_audit_standards_answer(generation, answer);
    }
}

/// The packages' findings put in the table: the project's exceptions laid on them as the command lays them on its own
/// (an exception on an uzman incelemesi finding is the reviewer's approval), listed as the bar's İstisnalar asks, those
/// that are exceptions counted.
fn put(
    doc: &kentos_domain::Document,
    view: kentos_commands::ArchAuditExceptionView,
    standards: &[ArchAuditFinding],
    audited: &mut super::Audited,
) {
    let (laid, excepted, _) = kentos_native_application::arch_audit::lay_exceptions(doc, standards.to_vec(), view);
    audited.excepted = audited.excepted.saturating_sub(audited.standards_excepted) + excepted;
    audited.standards_excepted = excepted;
    audited.findings.extend(laid);
}

/// Both packages' answers, each its own or its error: the Dijital proje rule after the file is written and read back
/// (its failure the rule's), then the national regulation's.
fn answer(input: Input) -> Result<Value, String> {
    let doc = &input.copy;
    let cad = standards_facts(doc).ok_or_else(|| "Çizimin olguları yazılamadı.".to_owned())?;
    let building = regulation_facts(doc).ok_or_else(|| "Binanın olguları yazılamadı.".to_owned())?;
    let (ifc, delivered) = model(doc, input.delivered);
    let digital = ifc.and_then(|ifc| crate::standards::call(DIGITAL.0, DIGITAL.1, FUNCTION, vec![cad, ifc]));
    let regulation = crate::standards::call(REGULATION.0, REGULATION.1, FUNCTION, vec![building]);
    let side = |r: Result<Value, String>| r.map_or_else(|e| json!({ "error": e }), |v| json!({ "answer": v }));
    Ok(json!({ "digital": side(digital), "regulation": side(regulation), "delivered": delivered }))
}

/// The facts of the model the form reads — the last delivered one when `asked` and it reads as it went, else the
/// drawing's now (none for a drawing without walls) — and what the line says of the last delivered model (null: none).
fn model(doc: &kentos_domain::Document, asked: bool) -> (Result<Value, String>, Value) {
    let now = || {
        crate::ifc_facts::plan(doc)
            .map(crate::ifc_facts::Planned::facts)
            .transpose()
            .map(|f| f.unwrap_or(Value::Null))
    };
    let put = |line: &mut Value, c: Counted| match c {
        Counted::Drift(d) => line["drift"] = json!({ "added": d.added, "removed": d.removed, "changed": d.changed }),
        Counted::NoModel => line["drift"] = Value::Null,
        Counted::Older => line["older"] = json!(true),
    };
    match delivered_model::last(doc) {
        Last::None => (now(), Value::Null),
        Last::Unreadable { number, why } => (now(), json!({ "number": number, "error": why })),
        Last::Found(d) => {
            let mut line = json!({ "number": d.number, "model": d.model, "at": d.at });
            if asked {
                match d.read(doc) {
                    Ok(read) => {
                        put(&mut line, read.counted);
                        line["used"] = json!(true);
                        line["expected"] = json!(read.expected != Expected::Missing);
                        return (Ok(read.facts), line);
                    }
                    Err(why) => line["error"] = json!(why),
                }
            } else {
                match d.counted(doc) {
                    Ok(c) => put(&mut line, c),
                    Err(why) => line["error"] = json!(why),
                }
            }
            (now(), line)
        }
    }
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
            outline: Vec::new(),
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
