//! The forms core (docs/adr/0401, `plan/form.md`'s `core/1` profile): what a form means over a
//! layer's fields. The contracts keep a form's shape and its structural rules
//! (`kentos_contracts::forms`); this crate answers the rest, the same for the desktop, the headless
//! server, Python and MCP:
//!
//! - [`preflight`]: whether a form can be used: its fields exist, a widget takes its field's kind,
//!   its rules compile, a field the layer requires is always shown and writable;
//! - [`Prepared`]: a form ready for records: which items a record shows, requires and locks
//!   ([`Prepared::states`]), what is wrong with it ([`Prepared::validate`]), a form's write
//!   ([`Prepared::apply`]) and the check a write from elsewhere passes when the form enforces its
//!   rules ([`Prepared::enforce`]);
//! - [`generate`]: a form made of a layer's fields.
//!
//! A record's values are its attributes, kept as the fields' canonical text (docs/adr/0199 §1);
//! empty is no attribute. Rules are the expression language's (docs/adr/0100) over the layer's
//! fields, each read as its kind (a yes/no field is true or false). A hidden field keeps its value;
//! a rule decides only what the form shows and asks.

pub mod field_refs;
mod generate;
mod lookup;
mod object;
mod preflight;
mod record;
pub mod scenario;
mod session;

pub use generate::{generate, item_id_of, widget_of};
pub use lookup::{OtherLayer, World, labels_of};
pub use object::{kind_label, object_of};
pub use preflight::{preflight, preflight_in, widget_fits};
pub use record::{Attached, Attrs, Object, schema_in, schema_of};
pub use session::{CHOICE_CODE, CHOICE_LABEL, HOST_ONLY, MOMENT, Prepared};

use kentos_contracts::{FormIssue, FormSeverity};

/// The issues' codes: stable, the commands' warnings and refusals name them.
pub mod codes {
    // A form's preflight.
    /// The form's structure breaks the contract's rules.
    pub const BODY_INVALID: &str = "form_body_invalid";
    /// A field item names a key no field of the layer has.
    pub const FIELD_UNKNOWN: &str = "form_field_unknown";
    /// Two field items name one field.
    pub const FIELD_TWICE: &str = "form_field_twice";
    /// The widget does not take the field's kind.
    pub const WIDGET_UNFIT: &str = "form_widget_unfit";
    /// A list, radio or segmented widget on a field without a value list.
    pub const WIDGET_NEEDS_VALUES: &str = "form_widget_needs_values";
    /// A slider on a field without its least and most (docs/adr/0406 §2).
    pub const WIDGET_NEEDS_RANGE: &str = "form_widget_needs_range";
    /// Suggestions on a field that is not a free text field (docs/adr/0407 §3).
    pub const SUGGEST_UNFIT: &str = "form_suggest_unfit";
    /// A colour field's value that is not `#RRGGBB` or `#RRGGBBAA` (docs/adr/0407 §2).
    pub const COLOR_INVALID: &str = "form_color_invalid";
    /// Fewer attachments than the item asks for (docs/adr/0407 §1).
    pub const ATTACHMENTS_FEW: &str = "form_attachments_few";
    /// More attachments than the item allows.
    pub const ATTACHMENTS_MANY: &str = "form_attachments_many";
    /// The default does not keep the field's rules.
    pub const DEFAULT_INVALID: &str = "form_default_invalid";
    /// A rule or a check does not compile.
    pub const RULE_INVALID: &str = "form_rule_invalid";
    /// A rule reads a name that is not one of the layer's fields (a warning).
    pub const RULE_UNKNOWN_NAME: &str = "form_rule_unknown_name";
    /// A rule reads an `@` value the project does not have (a warning).
    pub const RULE_UNKNOWN_VARIABLE: &str = "form_rule_unknown_variable";
    /// A field's visibility or lock reads the field itself (a warning: typing may hide or lock it).
    pub const RULE_READS_ITSELF: &str = "form_rule_reads_itself";
    /// A field the layer requires that the form may hide or lock, or does not show.
    pub const REQUIRED_HIDDEN: &str = "form_required_hidden";
    /// A field required and read-only with no default: it can never be filled.
    pub const REQUIRED_READ_ONLY: &str = "form_required_read_only";
    /// A form with no field (a warning).
    pub const EMPTY: &str = "form_empty";
    /// Two settings of a field that cannot go together (`default` and `defaultExpr`; `calculate` and a
    /// default or `readOnlyWhen`).
    pub const RULE_CONFLICT: &str = "form_rule_conflict";
    /// Calculated fields that read each other in a circle.
    pub const CALC_CYCLE: &str = "form_calc_cycle";
    /// A choice filter on a field without a value list.
    pub const CHOICE_UNLISTED: &str = "form_choice_unlisted";
    /// A default or a calculation reads the moment or the user: only a command that gives them works
    /// it out (a warning).
    pub const MOMENT_ONLY: &str = "form_moment_only";
    /// A list from another layer, or a related item, names a layer the project has not (docs/adr/0404).
    pub const LOOKUP_LAYER: &str = "form_lookup_layer";
    /// A list from another layer, or a related item, names a field its layer has not.
    pub const LOOKUP_FIELD: &str = "form_lookup_field";
    // A record.
    /// A shown, required field is empty.
    pub const REQUIRED: &str = "form_required";
    /// A value does not keep its field's rules (kind, length, list, range).
    pub const VALUE_INVALID: &str = "form_value_invalid";
    /// A value written anew is retired from its field's list (docs/adr/0418).
    pub const VALUE_RETIRED: &str = "form_value_retired";
    /// A value the record's subtype does not offer, written anew or left when the subtype changed
    /// (docs/adr/0422 §2).
    pub const VALUE_NOT_IN_SUBTYPE: &str = "form_value_not_in_subtype";
    /// A check of a shown, filled field is not true.
    pub const CHECK_FAILED: &str = "form_check_failed";
    /// A write to a field the form shows read-only.
    pub const READ_ONLY: &str = "form_read_only";
    /// A write to a field the form hides.
    pub const HIDDEN: &str = "form_hidden";
    /// A write to a layer field the form does not show.
    pub const NOT_ON_FORM: &str = "form_not_on_form";
    /// A calculation whose value its field refuses.
    pub const CALC_INVALID: &str = "form_calc_invalid";
    /// A value the field's choice filter does not offer for the record.
    pub const CHOICE_FILTERED: &str = "form_choice_filtered";
    /// A value a field's list from another layer does not offer the record (docs/adr/0404 §2).
    pub const LOOKUP_MISSING: &str = "form_lookup_missing";
    /// A soft check that is not true (a warning).
    pub const CHECK_WARNING: &str = "form_check_warning";
    /// A unique field's value another object of the layer has.
    pub const UNIQUE: &str = "form_unique";
    /// The calculations and the hidden fields keep changing each other.
    pub const UNSTABLE: &str = "form_unstable";
}

/// An issue.
pub(crate) fn issue(
    code: &str,
    severity: FormSeverity,
    item: Option<&str>,
    field: Option<&str>,
    rule: Option<&str>,
    message: String,
) -> FormIssue {
    FormIssue {
        code: code.to_owned(),
        severity,
        item: item.map(str::to_owned),
        field: field.map(str::to_owned),
        rule: rule.map(str::to_owned),
        message,
    }
}

/// Whether any of the issues stops a save or a publish.
pub fn has_errors(issues: &[FormIssue]) -> bool {
    issues.iter().any(|i| i.severity == FormSeverity::Error)
}

/// Whether `value` is a colour a colour field holds: `#RRGGBB` or `#RRGGBBAA` (docs/adr/0407 §2).
pub fn is_colour(value: &str) -> bool {
    let v = value.trim();
    v.strip_prefix('#')
        .is_some_and(|hex| matches!(hex.len(), 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// What is said of a colour field's value that is not a colour.
pub fn colour_message(label: &str, value: &str) -> String {
    format!("“{label}” bir renk ister (#RRGGBB ya da #RRGGBBAA); “{value}” renk değil.")
}

/// The most suggestions a field offers.
pub const MOST_SUGGESTIONS: usize = 20;

/// A suggesting field's values in the layer (docs/adr/0407 §3): each distinct trimmed value once, the
/// most used first, a tie in the order the layer first holds them; at most [`MOST_SUGGESTIONS`].
pub fn suggestions_of<'a>(values: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen: Vec<(String, usize)> = Vec::new();
    let mut at: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for v in values {
        let v = kentos_contracts::js_trim(v);
        if v.is_empty() {
            continue;
        }
        match at.get(v) {
            Some(&i) => seen[i].1 += 1,
            None => {
                at.insert(v.to_owned(), seen.len());
                seen.push((v.to_owned(), 1));
            }
        }
    }
    // A stable sort keeps the layer's first order among equal counts.
    seen.sort_by_key(|s| std::cmp::Reverse(s.1));
    seen.into_iter()
        .take(MOST_SUGGESTIONS)
        .map(|(v, _)| v)
        .collect()
}

/// The suggestions offered while `typed` is in the field: those holding it, letters folded the Turkish
/// way (`ı` finds “Ihlamur”), the value itself left out; at most `most`, in their order.
pub fn suggestions_for<'a>(all: &'a [String], typed: &str, most: usize) -> Vec<&'a String> {
    use kentos_expression::js::text::fold_turkish;
    let typed = fold_turkish(typed);
    all.iter()
        .filter(|s| {
            let s = fold_turkish(s);
            s != typed && s.contains(&typed)
        })
        .take(most)
        .collect()
}
