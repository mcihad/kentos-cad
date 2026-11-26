//! What keeps a form from being published, said kind by kind (docs/adr/9161): the preflight's errors and an empty form,
//! grouped as the designer and the publish command say them. An empty form, a required field the form cannot fill, a
//! default that is no value of its field, a list without values, and the rest each have their own heading, so one
//! refusal names every kind it met and not its first error alone. A draft may still be kept with them.

use kentos_contracts::{FormIssue, FormSeverity};

use crate::codes;

/// A kind of problem that keeps a form from being published, in the order they are said.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublishKind {
    /// No field on the form: a release edits no record.
    Empty,
    /// A field the layer requires that the form leaves out, may hide or lock, or locks without a default.
    Required,
    /// A default that is not a value of its field (its kind, its list, its range).
    Default,
    /// A list widget whose field has no values, or a choice filter on a field without a list.
    List,
    /// A field the data layer's table does not have (docs/adr/9318): a record could not be written.
    Source,
    /// Every other error of the preflight.
    Other,
}

impl PublishKind {
    /// The kind's heading.
    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "Boş form",
            Self::Required => "Zorunlu alan",
            Self::Default => "Varsayılan değer",
            Self::List => "Değer listesi",
            Self::Source => "Kaynak tablosu",
            Self::Other => "Öbür sorunlar",
        }
    }

    fn of(code: &str) -> Self {
        match code {
            codes::EMPTY => Self::Empty,
            codes::REQUIRED_HIDDEN | codes::REQUIRED_READ_ONLY => Self::Required,
            codes::DEFAULT_INVALID => Self::Default,
            codes::WIDGET_NEEDS_VALUES | codes::CHOICE_UNLISTED => Self::List,
            codes::FIELD_NOT_IN_SOURCE => Self::Source,
            _ => Self::Other,
        }
    }
}

/// Whether `issue` keeps the form from being published: an error, or the form's emptiness (a warning while it is drawn:
/// an empty draft may be kept).
pub fn blocks_publishing(issue: &FormIssue) -> bool {
    issue.severity == FormSeverity::Error || issue.code == codes::EMPTY
}

/// The issues that keep the form from being published, by kind in [`PublishKind`]'s order, each kind's in the form's;
/// empty when it may be published.
pub fn publish_blockers(issues: &[FormIssue]) -> Vec<(PublishKind, Vec<&FormIssue>)> {
    let mut groups: Vec<(PublishKind, Vec<&FormIssue>)> = Vec::new();
    for issue in issues.iter().filter(|i| blocks_publishing(i)) {
        let kind = PublishKind::of(&issue.code);
        match groups.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, list)) => list.push(issue),
            None => groups.push((kind, vec![issue])),
        }
    }
    groups.sort_by_key(|(k, _)| *k);
    groups
}

/// The blockers in one text, kind by kind: “Zorunlu alan: “durum” alanı katmanda zorunlu ama formda yok; ekleyin.
/// Varsayılan değer: …”. Empty when the form may be published.
pub fn publish_words(issues: &[FormIssue]) -> String {
    publish_blockers(issues)
        .into_iter()
        .map(|(kind, list)| {
            let messages: Vec<&str> = list.iter().map(|i| i.message.trim().trim_end_matches('.')).collect();
            format!("{}: {}.", kind.label(), messages.join("; "))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue;

    fn at(code: &str, severity: FormSeverity, message: &str) -> FormIssue {
        issue(code, severity, None, None, None, message.into())
    }

    #[test]
    fn each_kind_is_said_under_its_heading_in_order() {
        let issues = vec![
            at(codes::DEFAULT_INVALID, FormSeverity::Error, "“tur” alanında “Köşk” değeri listede yok."),
            at(codes::FIELD_UNKNOWN, FormSeverity::Error, "Katmanda “eski” alanı yok."),
            at(codes::REQUIRED_HIDDEN, FormSeverity::Error, "“durum” alanı katmanda zorunlu ama formda yok; ekleyin."),
            at(codes::WIDGET_NEEDS_VALUES, FormSeverity::Error, "Açılır liste değer listesi ister; “tip” alanının listesi yok."),
            at(codes::REQUIRED_READ_ONLY, FormSeverity::Error, "“no” hem zorunlu hem salt okunur; varsayılanı olmadan doldurulamaz."),
            at(codes::SUGGEST_UNFIT, FormSeverity::Warning, "Bir uyarı."),
        ];
        let kinds: Vec<(PublishKind, usize)> = publish_blockers(&issues).iter().map(|(k, l)| (*k, l.len())).collect();
        assert_eq!(
            kinds,
            [(PublishKind::Required, 2), (PublishKind::Default, 1), (PublishKind::List, 1), (PublishKind::Other, 1)],
            "a warning blocks nothing"
        );
        assert_eq!(
            publish_words(&issues),
            "Zorunlu alan: “durum” alanı katmanda zorunlu ama formda yok; ekleyin; “no” hem zorunlu hem salt okunur; \
             varsayılanı olmadan doldurulamaz. Varsayılan değer: “tur” alanında “Köşk” değeri listede yok. Değer listesi: \
             Açılır liste değer listesi ister; “tip” alanının listesi yok. Öbür sorunlar: Katmanda “eski” alanı yok."
        );
    }

    #[test]
    fn an_empty_form_blocks_publishing_though_it_is_a_warning_while_drawn() {
        let issues = vec![at(codes::EMPTY, FormSeverity::Warning, "Formda alan yok; kayıt bu formla düzenlenemez.")];
        assert!(!crate::has_errors(&issues), "a draft may be kept empty");
        assert_eq!(publish_words(&issues), "Boş form: Formda alan yok; kayıt bu formla düzenlenemez.");
        assert!(publish_blockers(&[at(codes::SUGGEST_UNFIT, FormSeverity::Warning, "Uyarı.")]).is_empty());
    }
}
