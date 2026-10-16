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
mod preflight;
mod record;
mod session;

pub use generate::{generate, item_id_of, widget_of};
pub use preflight::{preflight, widget_fits};
pub use record::{Attrs, Object, schema_of};
pub use session::Prepared;

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
