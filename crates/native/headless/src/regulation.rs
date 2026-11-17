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

use kentos_commands::{CommandResult, CommandWarning};
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
            let found = findings(&answer);
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
fn findings(answer: &Value) -> Vec<Value> {
    let mut keys: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for f in answer["findings"].as_array().into_iter().flatten() {
        let code = f["code"].as_str().unwrap_or_default();
        let uids: Vec<&str> = f["uids"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let mut key = format!("{code}:{}", uids.join(","));
        if uids.is_empty() {
            key.push(':');
            key.push_str(f["message"].as_str().unwrap_or_default());
        }
        while keys.contains(&key) {
            key.push_str("#2");
        }
        keys.push(key.clone());
        let mut x = json!({
            "key": key,
            "check": "regulation",
            "code": code,
            "message": f["message"],
            "repair": f["repair"],
            "uids": uids,
            "at": {"x": 0.0, "y": 0.0},
        });
        if let Some(rule) = f["rule"].as_str() {
            x["rule"] = json!(rule);
        }
        if let Some(state) = f["state"].as_str() {
            x["state"] = json!(state);
        }
        out.push(x);
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
