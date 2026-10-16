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

mod generate;
mod lookup;
mod object;
mod preflight;
mod record;
mod session;

pub use generate::{generate, item_id_of, widget_of};
pub use lookup::{OtherLayer, World};
pub use object::{kind_label, object_of};
pub use preflight::{preflight, preflight_in, widget_fits};
pub use record::{Attrs, Object, schema_in, schema_of};
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
