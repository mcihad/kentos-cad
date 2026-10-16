//! `cad.forms.validate` v1 (docs/adr/0401 §9): what a form and a record are worth, writing nothing.
//! The form's preflight; with a record, its items (shown, required, read-only) and its issues after
//! the values given, as a submit would leave it. A form whose preflight has an error is answered,
//! not refused: its record is not asked. The shared cases are
//! `fixtures/commands/v1/cad.forms.validate.json`.

use kentos_contracts::{CommandResult, FormsValidate, FormsValidated};
use kentos_domain::Document;
use kentos_forms::Prepared;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::forms;

fn answer(doc: &Document, input: &FormsValidate) -> Result<FormsValidated, Stop> {
    if let Some(uid) = &input.uid {
        forms::uid_text(uid, "uid")?;
    }
    let form = forms::form(doc.settings(), &input.form, "form")?;
    let (body, version) = forms::body(form, input.draft, "draft")?;
    let record = match &input.uid {
        Some(uid) => Some(forms::record(doc, form, uid, "uid")?),
        None => None,
    };
    let fields = forms::fields(doc, &form.layer);
    let variables = forms::variables(doc);
    let mut issues = kentos_forms::preflight(body, fields, &variables);
    let mut items = Vec::new();
    let mut attrs = None;
    if let Some((slot, e)) = record
        && let Ok(prepared) = Prepared::new(body, fields, &variables)
    {
        let object = forms::object(doc, slot, e);
        let before = if input.defaults {
            prepared.with_defaults(&e.base().attrs)
        } else {
            e.base().attrs.clone()
        };
        let values: Vec<(String, String)> = input
            .values
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let (after, found) = prepared.assess(&before, &values, &object);
        // The defaults are a write too: what they fill is asked as given.
        items = prepared.states(&after, &object);
        issues.extend(found);
        attrs = Some(after);
    }
    Ok(FormsValidated {
        version,
        valid: !kentos_forms::has_errors(&issues),
        issues,
        items,
        attrs,
    })
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsValidate) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The answer, as execute gives it.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsValidate) -> CommandResult<FormsValidated> {
    respond(cx.doc, input)
}

/// The answer; nothing is written.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: FormsValidate,
) -> CommandResult<FormsValidated> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_FORMS_VALIDATE,
        kentos_contracts::CAD_FORMS_VALIDATE_VERSION,
        input,
        |cx, input| respond(cx.doc, &input),
    )
}

fn respond(doc: &Document, input: &FormsValidate) -> CommandResult<FormsValidated> {
    match answer(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
