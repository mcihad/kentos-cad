//! `cad.forms.submit` v1 (docs/adr/0401 §9): a record written through a form, one undo step
//! (“Form: <ad>”). The values go by the layer fields' keys; the form's rules and its fields' decide
//! whether they go in, the whole record or nothing. A new record (`defaults`) takes the form's defaults
//! after the values given; the calculations follow; a soft check's warning does not stop the write
//! (docs/adr/0402 §3, §6). The shared cases are
//! `fixtures/commands/v1/cad.forms.submit.json`.

use kentos_contracts::{CommandResult, Entity, FormsSubmit, FormsSubmitted};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::{self, fail};

/// The object, its slot, the record afterwards, the release used, the undo step's name and the
/// form's warnings.
struct Checked {
    slot: Slot,
    entity: Entity,
    version: Option<u32>,
    label: String,
    warnings: Vec<kentos_contracts::CommandWarning>,
}

fn check(doc: &Document, input: &FormsSubmit) -> Result<Checked, Stop> {
    forms::uid_text(&input.uid, "uid")?;
    let variables = forms::variables_given(doc, input.moment.as_deref(), input.user.as_deref())?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let form = forms::form(doc.settings(), &input.form, "form")?;
    let (body, version) = forms::body(form, input.draft, "draft")?;
    let prepared = forms::prepared(doc, form, body, &variables, "form")?;
    let (slot, e) = forms::record(doc, form, &input.uid, "uid")?;
    if doc.layers().is_locked(&form.layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; kayıt değiştirilemez. Kilidi Katmanlar panelinden açın.",
                doc.layers()
                    .get(&form.layer)
                    .map_or(form.layer.as_str(), |n| n.name.as_str())
            ),
            "uid",
        ));
    }
    let object = forms::object(doc, slot, e);
    let values: Vec<(String, String)> = input
        .values
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let uid = doc.uid(slot).unwrap_or_default();
    let (after, warnings) = prepared
        .apply(&e.base().attrs, &values, &object, input.defaults)
        .and_then(|(after, warnings)| {
            let taken =
                prepared.unique_issues(&after, &object, forms::others(doc, &form.layer, uid));
            if taken.is_empty() {
                Ok((after, warnings))
            } else {
                Err(taken)
            }
        })
        .map_err(|issues| {
            let issues: Vec<_> = issues
                .into_iter()
                .filter(|i| i.severity == kentos_contracts::FormSeverity::Error)
                .collect();
            let first = &issues[0];
            let path = first
                .field
                .as_ref()
                .filter(|k| input.values.contains_key(*k))
                .map_or_else(|| "values".to_owned(), |k| format!("values.{k}"));
            let more = match issues.len() - 1 {
                0 => String::new(),
                n => format!(" ({n} sorun daha var.)"),
            };
            fail(
                codes::FORM_INVALID,
                format!("{}{more} Değeri düzeltip yeniden kaydedin.", first.message),
                &path,
            )
        })?;
    let mut entity = e.clone();
    entity.base_mut().attrs = after;
    Ok(Checked {
        slot,
        entity,
        version,
        label: format!("Form: {}", form.name),
        warnings: forms::warnings_of(&warnings),
    })
}

fn answer(doc: &Document, c: &Checked, changed: bool, uid: &str) -> FormsSubmitted {
    FormsSubmitted {
        uid: uid.to_owned(),
        version: c.version,
        attrs: c.entity.base().attrs.clone(),
        changed,
        revision: doc.revision().to_string(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsSubmit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsSubmit) -> CommandResult<FormsSubmitted> {
    match check(cx.doc, input) {
        Ok(c) => {
            let changed = cx.doc.get(c.slot) != Some(&c.entity);
            CommandResult::Completed {
                output: answer(cx.doc, &c, changed, &input.uid),
                warnings: c.warnings.clone(),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the record.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsSubmit) -> CommandResult<FormsSubmitted> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_FORMS_SUBMIT,
        kentos_contracts::CAD_FORMS_SUBMIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: FormsSubmit,
) -> CommandResult<FormsSubmitted> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let changed = cx.doc.get(c.slot) != Some(&c.entity);
    if changed {
        cx.doc
            .update_many(vec![(c.slot, c.entity.clone())], &c.label);
    }
    let warnings = c.warnings.clone();
    CommandResult::Completed {
        output: answer(cx.doc, &c, changed, &input.uid),
        warnings,
    }
}
