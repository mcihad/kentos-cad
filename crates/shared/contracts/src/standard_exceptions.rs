//! Accepted standards findings (docs/adr/1129): a finding of `cad.standards.check` the project keeps on purpose, with
//! its reason. As a topology exception does (docs/adr/1260 §2), it names the finding (its kind, what it is about and
//! the layer), the standard it was found against (the source's id and revision) and what the drawing had then (the
//! finding's expected and found values); once the standard's revision or the drawing's value changes, the acceptance
//! lapses and the finding is to be looked at again. Who accepted it and when are kept when the caller says so.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The longest reason, in characters (as a topology exception's, docs/adr/1260 §2).
pub const MAX_STANDARD_EXCEPTION_REASON: usize = 500;
/// The longest name of who accepted a finding, and of a source id, in characters.
pub const MAX_STANDARD_EXCEPTION_NAME: usize = 200;
/// The most accepted findings a project keeps.
pub const MAX_STANDARD_EXCEPTIONS: usize = 10_000;

/// An accepted standards finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct StandardException {
    /// The finding's kind, as `cad.standards.check` names it (`layerColor`, `tableStyleDefinition` …).
    pub kind: String,
    /// What it is about: a layer's, a style's, a line type's or a block's name.
    pub subject: String,
    /// The drawing's layer it is about, by id, when it is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub layer: Option<String>,
    /// The standard it was found against: the source's id (a package's `id`, else the standard's name) …
    pub standard: String,
    /// … and its revision (empty for a standard without one).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<String>", optional))]
    pub revision: String,
    /// What the standard asked for and what the drawing had when it was accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub found: Option<String>,
    /// Why it is kept (not empty, at most `MAX_STANDARD_EXCEPTION_REASON` characters).
    pub reason: String,
    /// Who accepted it, when the caller says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub by: Option<String>,
    /// When, as the caller wrote it (ISO 8601).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub at: Option<String>,
}

impl StandardException {
    /// Whether it names the same finding against the same source as `other` (revision and values aside).
    pub fn same_finding(&self, other: &StandardException) -> bool {
        self.kind == other.kind && self.subject == other.subject && self.layer == other.layer && self.standard == other.standard
    }
}

/// What is wrong with a reason: empty or only white space, or too long.
pub fn standard_exception_reason_problem(reason: &str) -> Option<String> {
    if reason.trim().is_empty() {
        return Some("istisnanın gerekçesi boş olamaz; neden kabul edildiğini yazın".into());
    }
    let n = reason.chars().count();
    (n > MAX_STANDARD_EXCEPTION_REASON)
        .then(|| format!("istisnanın gerekçesi {n} karakter; en çok {MAX_STANDARD_EXCEPTION_REASON} olur"))
}

/// What is wrong with the project's accepted findings: one without a kind, subject or source, a reason as
/// [`standard_exception_reason_problem`] says, a name or time too long or with control characters, or two for the same
/// finding against the same source.
pub fn standard_exceptions_problem(list: &[StandardException]) -> Option<String> {
    if list.len() > MAX_STANDARD_EXCEPTIONS {
        return Some(format!("en çok {MAX_STANDARD_EXCEPTIONS} kabul edilmiş bulgu olur; {} verildi", list.len()));
    }
    let bad = |s: &str| s.chars().count() > MAX_STANDARD_EXCEPTION_NAME || s.chars().any(char::is_control);
    for (i, e) in list.iter().enumerate() {
        let n = i + 1;
        if e.kind.trim().is_empty() || e.subject.is_empty() || e.standard.trim().is_empty() {
            return Some(format!("{n}. kabul edilmiş bulgunun türü, konusu ya da standardı boş"));
        }
        if bad(&e.kind) || bad(&e.standard) || bad(&e.revision) {
            return Some(format!("{n}. kabul edilmiş bulgunun türü, standardı ya da revizyonu geçersiz"));
        }
        if let Some(p) = standard_exception_reason_problem(&e.reason) {
            return Some(format!("{n}. kabul edilmiş bulgu: {p}"));
        }
        if e.by.as_deref().is_some_and(|b| b.trim().is_empty() || bad(b)) || e.at.as_deref().is_some_and(|a| a.trim().is_empty() || bad(a)) {
            return Some(format!("{n}. kabul edilmiş bulgunun kabul edeni ya da zamanı geçersiz"));
        }
        if list[..i].iter().any(|o| o.same_finding(e)) {
            return Some(format!("“{}” ({}) bulgusu aynı standart için iki kez kabul edilmiş", e.subject, e.kind));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one() -> StandardException {
        StandardException {
            kind: "layerColor".into(),
            subject: "A-DUVAR".into(),
            layer: Some("duvar".into()),
            standard: "ofis-a".into(),
            revision: "2026.1".into(),
            expected: Some("#0000ff".into()),
            found: Some("#ff0000".into()),
            reason: "Müşteri kırmızı istedi".into(),
            by: None,
            at: None,
        }
    }

    #[test]
    fn a_list_is_checked_item_by_item() {
        assert_eq!(standard_exceptions_problem(&[one()]), None);
        for change in [
            (|e: &mut StandardException| e.reason = "  ".into()) as fn(&mut StandardException),
            |e| e.reason = "x".repeat(501),
            |e| e.kind.clear(),
            |e| e.standard.clear(),
            |e| e.by = Some(" ".into()),
            |e| e.at = Some("a\nb".into()),
        ] {
            let mut e = one();
            change(&mut e);
            assert!(standard_exceptions_problem(&[e]).is_some());
        }
        let mut other = one();
        other.revision = "2026.2".into();
        assert!(standard_exceptions_problem(&[one(), other]).is_some(), "one finding against one source once");
        let mut elsewhere = one();
        elsewhere.standard = "ofis-b".into();
        assert_eq!(standard_exceptions_problem(&[one(), elsewhere]), None);
    }
}
