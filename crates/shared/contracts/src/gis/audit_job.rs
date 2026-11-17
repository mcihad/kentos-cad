//! An extension's audit rule run as a server job (plan/eklenti.md ADIM 20, docs/adr/8057 §42): what the process
//! `kentos-jobrun` is asked ([`AuditRun`], a [`RunRequest::Audit`](crate::processing_job::RunRequest)) and what it
//! answers ([`AuditReport`], in `answer.json`'s `details`).
//!
//! - **One reading of a rule's answer** ([`findings_of`]): the `done` of `kentos.ext_host` read as the desktop's Eklenti
//!   denetimi reads it (a known severity, the words to 2000 characters, a fix's title to 120, at most
//!   [`MOST_FINDINGS`]), so a desktop and a job that audit the same drawing with the same rule keep the same findings.
//! - **Provenance** (the plan's köken): the rule by its extension's id, version and package digest, its local id and
//!   handler; the drawing by its content stamp; what the rule was given by its facts' digest; the KentOS that measured
//!   them. None of it is a clock or a place, so the same input gives the same report, byte for byte.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::processing_job::InputFault;

/// The most findings a report keeps; the rest are counted (`dropped`), as the desktop's tab counts them.
pub const MOST_FINDINGS: usize = 10_000;

/// How long a rule may take when the request does not say, seconds: the desktop's.
pub const AUDIT_TIMEOUT_SECONDS: u32 = 120;

/// The longest a request may let a rule take, seconds.
pub const AUDIT_TIMEOUT_MAX_SECONDS: u32 = 3600;

/// An audit rule the process runs on the folder's drawing (`snapshot.kcad`), from the extension package beside the
/// request (`extension.kek`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditRun {
    /// The package's extension id, version and content digest (`sha256:…`): the package must be this one.
    pub extension: String,
    pub version: String,
    pub digest: String,
    /// The rule: its local id among the manifest's `entrypoints.audits`.
    pub rule: String,
    /// The interpreter of the Python environment the server prepared for this extension version (an absolute path),
    /// run offline; none when the server has prepared none, and the rule then cannot run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<String>,
    /// How long the rule may take, seconds: [`AUDIT_TIMEOUT_SECONDS`] when absent, at most
    /// [`AUDIT_TIMEOUT_MAX_SECONDS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u32>,
}

/// The fault of an audit request, with its path; none when it is whole.
pub fn check_audit_run(run: &AuditRun) -> Option<InputFault> {
    let word = |s: &str| {
        !s.is_empty()
            && s.chars().count() <= 200
            && !s.chars().any(|c| c.is_control() || c.is_whitespace())
    };
    let fault = |path: &str, message: &str| Some((path.to_owned(), message.to_owned()));
    if !word(&run.extension) {
        return fault("input.extension", "extension, eklentinin kimliği olmalı.");
    }
    if !word(&run.version) {
        return fault("input.version", "version, eklentinin sürümü olmalı.");
    }
    let hex = run.digest.strip_prefix("sha256:").unwrap_or_default();
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return fault(
            "input.digest",
            "digest, paketin içerik özeti olmalı (sha256: ve 64 küçük onaltılık hane).",
        );
    }
    if !word(&run.rule) {
        return fault(
            "input.rule",
            "rule, denetim kuralının manifestteki kimliği olmalı.",
        );
    }
    if let Some(python) = &run.python
        && (python.is_empty() || !std::path::Path::new(python).is_absolute())
    {
        return fault(
            "input.python",
            "python, sunucunun hazırladığı ortamın yorumlayıcısı (mutlak yol) olmalı.",
        );
    }
    if run
        .timeout_seconds
        .is_some_and(|t| t == 0 || t > AUDIT_TIMEOUT_MAX_SECONDS)
    {
        return fault(
            "input.timeoutSeconds",
            &format!("timeoutSeconds 1 ile {AUDIT_TIMEOUT_MAX_SECONDS} arasında olmalı."),
        );
    }
    None
}

/// How much a finding matters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    /// As a rule and an agent write it: `error`, `warning`, `info`.
    pub fn code(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// As the user reads it: Hata, Uyarı, Bilgi.
    pub fn word(self) -> &'static str {
        match self {
            Severity::Error => "Hata",
            Severity::Warning => "Uyarı",
            Severity::Info => "Bilgi",
        }
    }
}

/// A finding's fix: a KentOS command and its input, for the user to apply.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fix {
    pub title: String,
    pub command: String,
    pub args: Value,
}

/// One finding of a rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
    /// The object it is about, by its persistent id.
    pub feature_id: Option<String>,
    pub fix: Option<Fix>,
}

/// The rule a report was made by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleRef {
    pub extension: String,
    pub version: String,
    /// The package's content digest (`sha256:…`).
    pub digest: String,
    /// Its local id and its handler (`module:function`), as the manifest gives them.
    pub rule: String,
    pub handler: String,
    pub title: String,
}

/// What a rule found and where it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditReport {
    pub rule: RuleRef,
    /// The drawing audited: its content stamp (16 hex digits of the SHA-256 of its parts' stamps, docs/adr/0253 §2),
    /// the same in every host for the same content.
    pub drawing: String,
    /// What the rule was given: `sha256:` and the SHA-256 of its facts' canonical JSON.
    pub facts: String,
    /// The KentOS that measured the facts (its version).
    pub kentos: String,
    pub findings: Vec<Finding>,
    /// Findings past [`MOST_FINDINGS`], counted.
    pub dropped: u64,
    /// Whether the drawing was larger than the facts take: the rule saw its first objects.
    pub cut: bool,
}

/// The rule's answer (`kentos.ext_host`'s `done`) read: its findings and how many were past [`MOST_FINDINGS`], or why
/// it gave none and its traceback, as the desktop's Eklenti denetimi reads it.
pub fn findings_of(done: &Value) -> Result<(Vec<Finding>, u64), (String, Option<String>)> {
    if done["ok"] != true {
        let message = done["message"].as_str().unwrap_or("Kural cevap vermedi.");
        let words = match done["exception"].as_str() {
            Some(kind) => format!("{kind}: {message}"),
            None if done["stopped"] == true => "Durduruldu.".to_owned(),
            None => message.to_owned(),
        };
        return Err((words, done["error"].as_str().map(str::to_owned)));
    }
    let result = &done["result"];
    if result["kind"] != "findings" {
        return Err(("Kural bulgu listesi döndürmedi.".to_owned(), None));
    }
    let list = result["findings"].as_array().cloned().unwrap_or_default();
    let dropped = list.len().saturating_sub(MOST_FINDINGS) as u64;
    let findings = list
        .iter()
        .take(MOST_FINDINGS)
        .filter_map(|f| {
            let severity = match f["severity"].as_str()? {
                "error" => Severity::Error,
                "warning" => Severity::Warning,
                "info" => Severity::Info,
                _ => return None,
            };
            let fix = f.get("fix").and_then(|x| {
                Some(Fix {
                    title: x["title"].as_str()?.chars().take(120).collect(),
                    command: x["command"].as_str()?.to_owned(),
                    args: x
                        .get("args")
                        .cloned()
                        .unwrap_or_else(|| Value::Object(Default::default())),
                })
            });
            Some(Finding {
                severity,
                message: f["message"].as_str()?.chars().take(2000).collect(),
                feature_id: f["featureId"].as_str().map(str::to_owned),
                fix,
            })
        })
        .collect();
    Ok((findings, dropped))
}

/// The findings by their severities in a sentence, as the desktop's tab says them: “2 hata, 1 uyarı”, or “Bulgu yok”.
pub fn counts(severities: impl IntoIterator<Item = Severity>) -> String {
    let all: Vec<Severity> = severities.into_iter().collect();
    let n = |s: Severity| all.iter().filter(|k| **k == s).count();
    let parts: Vec<String> = [
        (n(Severity::Error), "hata"),
        (n(Severity::Warning), "uyarı"),
        (n(Severity::Info), "bilgi"),
    ]
    .into_iter()
    .filter(|(k, _)| *k > 0)
    .map(|(k, w)| format!("{k} {w}"))
    .collect();
    if parts.is_empty() {
        "Bulgu yok".to_owned()
    } else {
        parts.join(", ")
    }
}

/// `sha256:` and the SHA-256 of `value`'s canonical JSON: what a rule was given, the same text in every host.
pub fn digest_of(value: &Value) -> String {
    format!(
        "sha256:{}",
        crate::encoding::sha256_hex(crate::canonical::canonical_json(value).as_bytes())
    )
}

#[cfg(test)]
#[path = "audit_job_tests.rs"]
mod tests;
