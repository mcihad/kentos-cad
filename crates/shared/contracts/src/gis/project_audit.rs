//! `project.audit`: an extension's audit rule run on a database project as a server job (docs/adr/8057 §42, the
//! server's part): what a user asks ([`ProjectAuditInput`]) and what the job keeps ([`ProjectAuditResult`]). The rule
//! is a published release of the organisation's extension catalog, else the public one's; the job runner measures the
//! project and runs the rule as the desktop does (`AuditRun`), and the job keeps its report. Nothing is written to the
//! project.

use serde::{Deserialize, Serialize};

use crate::ProcessingUsage;
use crate::audit_job::AuditReport;
use crate::processing_job::InputFault;

/// What a user asks a `project.audit` job for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectAuditInput {
    /// The extension's id, as the catalog lists it.
    pub extension: String,
    /// The rule: its local id among the manifest's audits.
    pub rule: String,
    /// The version to run, which must be published; the newest published version when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The project's data revision the job must read; refused when the project moved on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<String>,
}

/// The fault of a `project.audit` input, with its path; none when it is whole.
pub fn check_project_audit_input(input: &ProjectAuditInput) -> Option<InputFault> {
    let word = |s: &str| {
        !s.is_empty()
            && s.chars().count() <= 200
            && !s.chars().any(|c| c.is_control() || c.is_whitespace())
    };
    let fault = |path: &str, message: &str| Some((path.to_owned(), message.to_owned()));
    if !word(&input.extension) {
        return fault(
            "input.extension",
            "extension, katalogdaki eklentinin kimliği olmalı.",
        );
    }
    if !word(&input.rule) {
        return fault(
            "input.rule",
            "rule, denetim kuralının manifestteki kimliği olmalı.",
        );
    }
    if input.version.as_deref().is_some_and(|v| !word(v)) {
        return fault(
            "input.version",
            "version, eklentinin yayımlanmış bir sürümü olmalı.",
        );
    }
    if let Some(r) = &input.expected_revision
        && r.parse::<i64>().ok().filter(|n| *n >= 0).is_none()
    {
        return fault(
            "input.expectedRevision",
            "expectedRevision bir veri revizyonu (sıfır ya da artı tam sayı) olmalı.",
        );
    }
    None
}

/// What a `project.audit` job keeps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAuditResult {
    /// The catalog the release came from (`tenant:{slug}` or `public`).
    pub catalog: String,
    /// The project's data revision the rule read.
    pub base_revision: String,
    /// The findings in a sentence (“Adsız nesneler: 1 hata, 9 bilgi”).
    pub summary: String,
    /// The findings and their provenance, as the job runner wrote them.
    pub report: AuditReport,
    /// What the job runner took.
    pub usage: ProcessingUsage,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a user asks: camelCase, nothing it does not know, each field checked where it is.
    #[test]
    fn an_audit_job_s_input_is_checked_field_by_field() {
        let input: ProjectAuditInput = serde_json::from_value(serde_json::json!({
            "extension": "tr.ornek.denetim", "rule": "adsiz", "version": "1.0.0", "expectedRevision": "4"
        }))
        .unwrap();
        assert_eq!(check_project_audit_input(&input), None);
        assert!(
            serde_json::from_value::<ProjectAuditInput>(serde_json::json!({
                "extension": "x", "rule": "y", "python": "/usr/bin/python3"
            }))
            .is_err(),
            "the interpreter is the server's, never the request's"
        );
        let fault = |edit: &dyn Fn(&mut ProjectAuditInput)| {
            let mut i = input.clone();
            edit(&mut i);
            check_project_audit_input(&i).map(|(path, _)| path)
        };
        assert_eq!(
            fault(&|i| i.extension.clear()),
            Some("input.extension".into())
        );
        assert_eq!(
            fault(&|i| i.rule = "iki kelime".into()),
            Some("input.rule".into())
        );
        assert_eq!(
            fault(&|i| i.version = Some(String::new())),
            Some("input.version".into())
        );
        assert_eq!(fault(&|i| i.version = None), None, "the newest published");
        assert_eq!(
            fault(&|i| i.expected_revision = Some("-1".into())),
            Some("input.expectedRevision".into())
        );
    }
}
