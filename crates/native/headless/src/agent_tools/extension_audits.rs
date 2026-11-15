//! An extension's audit rules through the agents' tools (docs/adr/8057 §27, plan/eklenti.md ADIM 20's AI/MCP
//! `audit.run`): a desktop's rule run by `audit.run`, its last report read by `audit.report`, a finding's fix applied
//! by `audit.apply_fix` through the owner's guard. The requests are the desktop's (`audit`, `audit_report`,
//! `audit_fixed`; `audits` lists the rules for `audit.list_providers`); a host without extensions says where they run.

use serde_json::{Value, json};

use super::{Guard, Host, descriptor, guarded, reads_only, text};
use crate::HeadlessError;

/// An extension's audit rule: its id (`ext.<extension>.<rule>`), as `audits` lists it.
pub(super) fn is_extension_rule(id: &str) -> bool {
    id.starts_with("ext.")
}

/// A host's refusal of an extension audit request: a host without extensions (a drawing of the server's own) says
/// where they run.
fn extensions_refused(e: HeadlessError) -> HeadlessError {
    if e.code == "unknown_method" {
        return HeadlessError::new(
            "not_offered",
            "Eklenti denetimleri KentOS masaüstünde çalışır: Asistan'da ya da masaüstünün ajan bağlantısında (MCP'de \
             desktop.attach).",
        );
    }
    e
}

/// What an extension's rule is, as `audit.list_providers` and `catalog.describe` say it.
pub(super) const EXTENSION_RULE: &str = "Kurulu bir eklentinin denetim kuralı: girdisi yok, audit.run ile çalışır; \
                                         bulguları ve düzeltmeleri raporunda.";

/// `catalog.describe` of an extension's rule: not a catalog command; what it is and how it runs, from the desktop's
/// list of rules.
pub(super) fn extension_describe(host: &mut dyn Host, rule: &str) -> Result<Value, HeadlessError> {
    let rules = host
        .request("audits", &json!({}))
        .map_err(extensions_refused)?;
    let found = rules["rules"]
        .as_array()
        .and_then(|list| list.iter().find(|r| r["rule"] == rule))
        .ok_or_else(|| {
            HeadlessError::new(
                "unknown_rule",
                format!(
                    "“{rule}” kurulu ve etkin bir eklenti denetimi değil: eklentinin İşlemler aracıysa \
                     processing.describe tanımlar; denetimleri audit.list_providers söyler."
                ),
            )
        })?;
    Ok(json!({
        "command": rule,
        "title": found["title"],
        "summary": EXTENSION_RULE,
        "extension": found["extension"],
        "door": "audit.run",
        "writes": false,
        "input": { "type": "object", "properties": {} },
        "ready": found["ready"],
        "why": found["why"],
    }))
}

/// `audit.run` of an extension's rule (docs/adr/8057 §27): started on the desktop's front drawing as its ribbon
/// button starts it, then its report when the run ends.
pub(super) fn extension_audit(host: &mut dyn Host, rule: &str) -> Result<Value, HeadlessError> {
    host.request("audit", &json!({ "rule": rule }))
        .map_err(extensions_refused)?;
    audit_outcome(host.audit_wait(rule)?)
}

/// An extension audit's report as its `audit.run` answers: a run that gave no findings (its rule failed, it was
/// stopped) is the call's failure with why; the desktop's chat answers its waiting call with this too.
pub fn audit_outcome(report: Value) -> Result<Value, HeadlessError> {
    match report["status"].as_str() {
        Some("failed") => Err(HeadlessError::new(
            "audit_failed",
            format!(
                "Denetim bulgu vermedi: {}",
                report["error"].as_str().unwrap_or("kural cevap vermedi.")
            ),
        )),
        Some("stopped") => Err(HeadlessError::new("stopped", "Denetim durduruldu.")),
        _ => Ok(report),
    }
}

/// `audit.report`: the last extension audit's report, its findings from `offset`.
pub(super) fn extension_report(host: &mut dyn Host, offset: u64) -> Result<Value, HeadlessError> {
    host.request("audit_report", &json!({ "offset": offset }))
        .map_err(extensions_refused)
}

/// `audit.apply_fix` of an extension's finding: its fix as the last run gave it (only one the user's Düzelt could
/// apply now), through the owner's guard like any write; executed and completed, the tab marks the finding fixed.
pub(super) fn extension_fix(
    host: &mut dyn Host,
    guard: &mut Guard,
    args: &Value,
) -> Result<Value, HeadlessError> {
    let rule = text(args, "rule")?;
    let finding = args
        .get("finding")
        .and_then(Value::as_u64)
        .filter(|n| *n >= 1)
        .ok_or_else(|| {
            HeadlessError::new(
                "invalid_input",
                "finding gerekli: bulgunun 1'den başlayan numarası (audit.run ve audit.report verir).",
            )
        })?;
    let report = extension_report(host, finding - 1)?;
    if report["rule"] != rule {
        return Err(HeadlessError::new(
            "stale_report",
            format!(
                "Son eklenti denetimi “{}”: düzeltmeden önce audit.run ile “{rule}” denetimini çalıştırın.",
                report["rule"].as_str().unwrap_or("yok")
            ),
        ));
    }
    if report["status"] == "running" {
        return Err(HeadlessError::new(
            "busy",
            "Denetim sürüyor: bitince düzeltin.",
        ));
    }
    let item = report["findings"]
        .as_array()
        .and_then(|list| list.first())
        .filter(|f| f["finding"].as_u64() == Some(finding))
        .ok_or_else(|| {
            HeadlessError::new(
                "unknown_finding",
                format!(
                    "Raporda {finding}. bulgu yok: audit.report bulguları numaralarıyla verir."
                ),
            )
        })?;
    if item["fixed"] == true {
        return Err(HeadlessError::new(
            "already_fixed",
            "Bu bulgu düzeltildi: yeniden denetlemek bulguları tazeler.",
        ));
    }
    let fix = item.get("fix").filter(|f| f.is_object()).ok_or_else(|| {
        HeadlessError::new(
            "not_fixable",
            item["fixRefused"]
                .as_str()
                .unwrap_or("Bu bulgunun düzeltmesi yok.")
                .to_owned(),
        )
    })?;
    let command = text(fix, "command")?;
    let Some(descriptor) = descriptor(command) else {
        return Err(HeadlessError::new(
            "unknown_command",
            format!("Düzeltmenin komutu “{command}” katalogda yok: eklentiye bildirin."),
        ));
    };
    if let Some(why) = host.not_offered(command) {
        return Err(HeadlessError::new("not_offered", why));
    }
    let input = fix.get("input").cloned().unwrap_or_else(|| json!({}));
    let answer = guarded(host, guard, command, reads_only(descriptor), input, args)?;
    let executed = args.get("mode").and_then(Value::as_str) == Some("execute")
        && answer.get("status").and_then(Value::as_str) == Some("completed");
    if !executed {
        return Ok(answer);
    }
    let marked = host
        .request("audit_fixed", &json!({ "rule": rule, "finding": finding }))
        .is_ok();
    let mut answer = answer;
    if let Some(fields) = answer.as_object_mut() {
        fields.insert("finding".into(), json!(finding));
        fields.insert("marked".into(), json!(marked));
    }
    Ok(answer)
}
