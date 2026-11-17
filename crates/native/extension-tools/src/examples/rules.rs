//! An audit rule's examples (docs/adr/8057 §75): each made into a synthetic drawing and measured as any drawing, the
//! architecture it gives put in the facts, the rule run as its host runs it (the desktop's pool job, a headless host's
//! pool), and its findings checked: how many, and the ones it must have.

use kentos_commands::audit_job::{Finding, findings_of};
use kentos_extension_host::Registry;
use kentos_extensions::audit_examples::{self, AuditExample, ExpectedFinding};
use kentos_extensions::manifest::AuditEntry;
use kentos_processing::audit_job;
use serde_json::{Value, json};

use crate::synthetic::{TRIAL_SRID, synthetic_on};

/// An audit rule's runner: the `audit` invoke given (`audit_job::message`), the process's answer (`done`), or why
/// there is none. The desktop's pool job, a headless host's pool.
pub type RunRule = Box<dyn Fn(Value) -> Result<Value, String> + Send>;

/// An audit rule's examples, and how its host runs it.
pub struct Rule {
    /// `ext.<extension>.<local id>`.
    pub id: String,
    pub title: String,
    pub examples: Vec<AuditExample>,
    pub run: RunRule,
}

/// The audit rules of extension `ext` installed in `registry` that give examples, each read from its package: the
/// entry and its examples, or why they cannot be read (gate 1 checked them; a package changed since says so here).
pub fn rule_examples(
    registry: &Registry,
    ext: &str,
) -> Vec<(AuditEntry, Result<Vec<AuditExample>, String>)> {
    let Some(manifest) = registry.manifest(ext) else {
        return Vec::new();
    };
    manifest
        .entrypoints
        .audits
        .iter()
        .filter_map(|audit| {
            let path = audit.examples.as_deref()?;
            let read = registry
                .package_file(ext, path)
                .ok_or_else(|| format!("{path} pakette yok."))
                .and_then(|data| {
                    audit_examples::read(path, &audit.id, &data)
                        .map(|file| file.examples)
                        .map_err(|found| {
                            found
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join("; ")
                        })
                });
            Some((audit.clone(), read))
        })
        .collect()
}

/// One rule's example run and checked: `passed`, or `failed` with what was not as it expects (a failure of its code
/// with its traceback, `detail`).
pub(super) fn one_rule(rule: &Rule, example: &AuditExample) -> Value {
    let row = |problems: Vec<String>, findings: Value, detail: Option<String>| {
        json!({
            "rule": rule.id, "label": rule.title, "title": example.title,
            "status": if problems.is_empty() { "passed" } else { "failed" },
            "problems": problems, "findings": findings, "detail": detail,
        })
    };
    let made = match synthetic_on(&example.features, TRIAL_SRID) {
        Ok(made) => made,
        Err(why) => return row(vec![format!("örneğin nesneleri: {why}")], Value::Null, None),
    };
    let Some(mut facts) = audit_job::facts_of(&made.doc, &|| false) else {
        return row(
            vec!["çizimin olguları ölçülemedi".to_owned()],
            Value::Null,
            None,
        );
    };
    if let Some(architecture) = &example.architecture {
        facts["project"]["architecture"] = serde_json::to_value(architecture).unwrap_or_default();
    }
    let done = match (rule.run)(audit_job::message(facts)) {
        Ok(done) => done,
        Err(why) => return row(vec![format!("kural çalışmadı: {why}")], Value::Null, None),
    };
    let (findings, dropped) = match findings_of(&done) {
        Ok(found) => found,
        Err((words, traceback)) => {
            return row(
                vec![format!("kural hata verdi: {words}")],
                Value::Null,
                traceback,
            );
        }
    };
    let uids: Vec<String> = made.uids.iter().map(ToString::to_string).collect();
    let mut problems = Vec::new();
    let total = findings.len() as u64 + dropped;
    if let Some(n) = example.expect.count
        && total != n
    {
        problems.push(format!("{n} bulgu beklendi, {total} geldi"));
    }
    for want in &example.expect.has {
        if !findings.iter().any(|f| matches(f, want, &uids)) {
            problems.push(format!("{} bulgusu gelmedi", described(want)));
        }
    }
    if !problems.is_empty() && !findings.is_empty() {
        let came: Vec<String> = findings
            .iter()
            .take(3)
            .map(|f| {
                let words: String = f.message.chars().take(100).collect();
                format!("{} “{words}”", f.severity.code())
            })
            .collect();
        problems.push(format!("gelenler: {}", came.join("; ")));
    }
    row(problems, json!(total), None)
}

/// Whether finding `f` is the one `want` describes: every key given matches; its object by the example's place.
fn matches(f: &Finding, want: &ExpectedFinding, uids: &[String]) -> bool {
    want.severity
        .is_none_or(|s| s.as_str() == f.severity.code())
        && want
            .message
            .as_deref()
            .is_none_or(|m| f.message.contains(m))
        && want.feature.is_none_or(|n| {
            uids.get(n)
                .is_some_and(|u| f.feature_id.as_deref() == Some(u))
        })
        && want
            .fix
            .as_deref()
            .is_none_or(|c| f.fix.as_ref().is_some_and(|x| x.command == c))
}

/// An expected finding as the report says it: `(error, “çakışıyor”, features[1], düzeltme ext.….duzelt)`.
fn described(want: &ExpectedFinding) -> String {
    let mut parts = Vec::new();
    if let Some(s) = want.severity {
        parts.push(s.as_str().to_owned());
    }
    if let Some(m) = &want.message {
        parts.push(format!("“{m}”"));
    }
    if let Some(n) = want.feature {
        parts.push(format!("features[{n}]"));
    }
    if let Some(c) = &want.fix {
        parts.push(format!("düzeltme {c}"));
    }
    format!("({})", parts.join(", "))
}
