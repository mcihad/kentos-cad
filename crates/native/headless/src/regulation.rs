//! The national regulation's Python rules in a headless audit (docs/adr/8078; plan/bim-plan.md BIM-13, K10):
//! `cad.arch.audit` over MCP and Python gives the standards package's findings (catalog/standards/mimari-mevzuat:
//! PAİY 34/1 and 34/8 by the application date's version) beside the Rust ones, and its rule set, as the desktop's
//! Denetim tab does. An article moved to Python is never taken from an agent.
//!
//! - **When:** the regulation group asked by name (`checks` holds `regulation`), of the whole drawing (no `within`).
//! - **Where in the list:** after the Mevzuat group's own findings, in the rule's order; their keys `code:uids` (a
//!   building's finding, naming no object, `code::message`).
//! - **A rule that does not answer** (no Python, a broken package) leaves the Rust findings as they are and says why in
//!   a warning (`regulation_rule_unavailable`): an empty list is not taken for a pass.
//! - **`cad.arch.ifc`** (docs/adr/8088): a call that gives no `compartments` gets the package's fire compartments,
//!   written as zones of their rooms; one that does not answer writes the file without them and says so.

use kentos_commands::{
    ArchAuditCheck, ArchAuditExceptionView, ArchAuditFinding, ArchAuditState, CommandResult, CommandWarning, Vec2,
};
use kentos_domain::Document;
use kentos_extensions::audit_facts::ProjectAuditFacts;
use serde_json::{Value, json};

/// The package and its function.
const PACKAGE: &str = "mimari-mevzuat";
const MODULE: &str = "mimari_mevzuat.mimari";
const FUNCTION: &str = "denetle";

/// Whether the audit asks the regulation group of the whole drawing.
fn asks(input: &Value) -> bool {
    let named = input
        .get("checks")
        .and_then(Value::as_array)
        .is_some_and(|c| c.iter().any(|x| x == "regulation"));
    named && input.get("within").is_none_or(Value::is_null)
}

/// The facts the rule reads: the permit's particulars and the building's.
pub fn facts(doc: &Document) -> Value {
    let mut facts = ProjectAuditFacts::new(0.0, Vec::new(), false)
        .with_building(kentos_processing::audit_facts::building::measure(doc));
    if let Some(permit) = kentos_processing::audit_facts::permit::measure(doc) {
        facts = facts.with_permit(permit);
    }
    serde_json::to_value(facts).unwrap_or(Value::Null)
}

/// `cad.arch.ifc`'s input with the fire compartments the package finds (docs/adr/8088; `kompartimanlar`), when the
/// caller gave none (`compartments` absent or null): the file's zones. What to say when they are not written: the
/// package does not answer, or the application's day is one its rules do not know; the file is written without them.
pub fn compartments(doc: &Document, command: &str, input: &mut Value) -> Option<CommandWarning> {
    if command != kentos_commands::CAD_ARCH_IFC
        || !input.is_object()
        || input.get("compartments").is_some_and(|v| !v.is_null())
    {
        return None;
    }
    match kentos_standards::call(PACKAGE, MODULE, "kompartimanlar", vec![facts(doc)]) {
        Ok(answer) => {
            // Each compartment's rooms: the command's input (its storeys and area are the package's words).
            let rooms: Vec<Value> = answer
                .get("compartments")
                .and_then(Value::as_array)
                .map(|list| list.iter().map(|c| json!({"rooms": c.get("rooms").cloned().unwrap_or_else(|| json!([]))})).collect())
                .unwrap_or_default();
            input["compartments"] = Value::Array(rooms);
            answer.get("note").and_then(Value::as_str).map(|note| CommandWarning {
                code: kentos_commands::codes::EXPORT_LOSSES.into(),
                message: format!("Yangın kompartımanları IFC'ye yazılmadı: {note}"),
                path: None,
            })
        }
        Err(why) => {
            input["compartments"] = json!([]);
            Some(CommandWarning {
                code: kentos_commands::codes::REGULATION_RULE_UNAVAILABLE.into(),
                message: format!(
                    "Yangın kompartımanları IFC'ye yazılmadı: mevzuatın Python kuralları çalışmadı: {why} Ayarlar › \
                     Python'dan bir Python seçin."
                ),
                path: None,
            })
        }
    }
}

/// A warning added to a completed result.
pub fn warn(result: &mut CommandResult<Value>, warning: CommandWarning) {
    if let CommandResult::Completed { warnings, .. } = result {
        warnings.push(warning);
    }
}

/// `cad.arch.audit`'s completed result with the package's findings and rule set added, when it asked for them.
pub fn add(doc: &Document, command: &str, input: &Value, result: &mut CommandResult<Value>) {
    if command != kentos_commands::CAD_ARCH_AUDIT || !asks(input) {
        return;
    }
    let CommandResult::Completed { output, warnings } = result else {
        return;
    };
    match kentos_standards::call(PACKAGE, MODULE, FUNCTION, vec![facts(doc)]) {
        Ok(answer) => {
            // The project's exceptions laid on them as on the command's own (docs/adr/0732 §3), listed as asked.
            let view: ArchAuditExceptionView =
                input.get("exceptions").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
            let (laid, excepted, lapsed) =
                kentos_native_application::arch_audit::lay_exceptions(doc, findings(&answer), view);
            let found: Vec<Value> = laid.iter().filter_map(|f| serde_json::to_value(f).ok()).collect();
            if let Some(n) = output.get("excepted").and_then(Value::as_u64) {
                output["excepted"] = json!(n + u64::from(excepted));
            }
            if lapsed > 0 {
                warnings.push(CommandWarning {
                    code: kentos_commands::codes::EXCEPTION_LAPSED.into(),
                    message: format!(
                        "Mevzuat kurallarının {lapsed} istisnasının bulgusu verildiğinden beri değişti; bulgular yeniden \
                         açık. İstisnayı yeniden verin ya da kaldırın."
                    ),
                    path: None,
                });
            }
            if let Some(list) = output.get_mut("findings").and_then(Value::as_array_mut) {
                let at = list
                    .iter()
                    .rposition(|f| f["check"] == "regulation")
                    .map_or(list.len(), |i| i + 1);
                list.splice(at..at, found);
            }
            output["ruleSet"] = json!({
                "date": answer.get("date"),
                "dateNote": answer.get("dateNote"),
                "rules": answer.get("rules"),
            });
            if output["ruleSet"]["date"].is_null() {
                output["ruleSet"].as_object_mut().map(|o| o.remove("date"));
            }
        }
        Err(why) => warnings.push(CommandWarning {
            code: kentos_commands::codes::REGULATION_RULE_UNAVAILABLE.into(),
            message: format!(
                "Mevzuatın Python kuralları (asansör 34/1 ve 34/8) çalışmadı: {why} Ayarlar › Python'dan bir Python seçin."
            ),
            path: None,
        }),
    }
}

/// The rule's findings as the audit's: the Mevzuat group's, keyed, with their state and article.
fn findings(answer: &Value) -> Vec<ArchAuditFinding> {
    let mut out: Vec<ArchAuditFinding> = Vec::new();
    for f in answer["findings"].as_array().into_iter().flatten() {
        let text = |k: &str| f[k].as_str().unwrap_or_default().to_owned();
        let code = text("code");
        let uids: Vec<String> = f["uids"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect();
        let mut key = format!("{code}:{}", uids.join(","));
        if uids.is_empty() {
            key.push(':');
            key.push_str(&text("message"));
        }
        while out.iter().any(|o| o.key == key) {
            key.push_str("#2");
        }
        let state = match f["state"].as_str() {
            Some("missingData") => Some(ArchAuditState::MissingData),
            Some("expertReview") => Some(ArchAuditState::ExpertReview),
            _ => None,
        };
        // Where it is and its fix as one step, as the desktop's Denetim reads them (docs/adr/8159).
        let at = f["at"]
            .as_array()
            .and_then(|a| Some(Vec2 { x: a.first()?.as_f64()?, y: a.get(1)?.as_f64()? }))
            .unwrap_or(Vec2 { x: 0.0, y: 0.0 });
        let fix = Some(&f["fix"]).filter(|x| !x.is_null()).and_then(|x| {
            Some(kentos_commands::ArchAuditFix {
                label: x["label"].as_str()?.to_owned(),
                step: kentos_commands::ChangesetStep {
                    command: x["command"].as_str()?.to_owned(),
                    version: None,
                    input: x.get("input")?.clone(),
                    name: None,
                },
            })
        });
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
            rule: f["rule"].as_str().map(str::to_owned),
            state,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_regulation_group_of_the_whole_drawing_asks() {
        assert!(asks(&json!({"checks": ["regulation"]})));
        assert!(asks(&json!({"checks": ["walls", "regulation"], "within": null})));
        assert!(!asks(&json!({"checks": ["walls"]})));
        assert!(!asks(&json!({})));
        assert!(!asks(&json!({"checks": ["regulation"], "within": {"levels": ["k1"]}})));
    }
}
