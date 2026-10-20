//! `cad.forms.fill` v1 (docs/adr/0403 §5): the same values written to many records through a form,
//! one undo step (“Form: <ad> (n nesne)”). Each record goes through the form's rules and
//! calculations with its own values and shape, as `cad.forms.submit` without `defaults`; when one
//! does not go in, none does. A unique field takes no value twice: not another object's, not
//! another record's of the same call. The shared cases are `fixtures/commands/v1/cad.forms.fill.json`.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, FormSeverity, FormsFill, FormsFilled,
};
use kentos_domain::{Document, Slot};
use kentos_forms::Attrs;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::forms::{self, fail};

/// The records afterwards, the release used, the undo step's name and the form's warnings.
struct Checked {
    records: Vec<(Slot, String, Entity)>,
    version: Option<u32>,
    label: String,
    warnings: Vec<CommandWarning>,
}

/// `form_invalid` for the record `uid`: its first issue, at the value that gave it.
fn invalid(input: &FormsFill, uid: &str, issues: &[kentos_commands::FormIssue]) -> Stop {
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == FormSeverity::Error)
        .collect();
    let first = errors[0];
    let path = first
        .field
        .as_ref()
        .filter(|k| input.values.contains_key(*k))
        .map_or_else(|| "values".to_owned(), |k| format!("values.{k}"));
    let more = match errors.len() - 1 {
        0 => String::new(),
        n => format!(" ({n} sorun daha var.)"),
    };
    let mut e = error(
        codes::FORM_INVALID,
        format!(
            "“{uid}” kaydı: {}{more} Değeri düzeltip yeniden yazın.",
            first.message
        ),
        Some(path),
    );
    e.uids = vec![uid.to_owned()];
    Stop::failed(e)
}

fn check(doc: &Document, input: &FormsFill) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Formla yazılacak kayıt verilmedi.")?;
    let variables = forms::variables_given(doc, input.moment.as_deref(), input.user.as_deref())?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let form = forms::form(doc.settings(), &input.form, "form")?;
    forms::writable(form)?;
    let (body, version) = forms::body(form, input.draft, "draft")?;
    let prepared = forms::prepared(doc, form, body, &variables, "form")?;
    let named = checks::named(doc, &input.uids)?;
    for (at, _, e, _) in &named {
        if e.base().layer_id != form.layer {
            return Err(fail(
                codes::FORM_LAYER_MISMATCH,
                format!(
                    "Nesne “{}” formunun katmanında değil. Formun katmanındaki nesneleri seçin.",
                    form.name
                ),
                &format!("uids[{at}]"),
            ));
        }
    }
    if doc.layers().is_locked(&form.layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; kayıtlar değiştirilemez. Kilidi Katmanlar panelinden açın.",
                doc.layers()
                    .get(&form.layer)
                    .map_or(form.layer.as_str(), |n| n.name.as_str())
            ),
            "uids",
        ));
    }
    let values: Vec<(String, String)> = input
        .values
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut warnings: Vec<CommandWarning> = Vec::new();
    let mut records: Vec<(Slot, String, Entity, kentos_forms::Object)> = Vec::new();
    for (_, slot, e, uid) in &named {
        let object = forms::object(doc, *slot, e);
        let (after, found) = prepared
            .apply(&e.base().attrs, &values, &object, false)
            .map_err(|issues| invalid(input, uid, &issues))?;
        for w in forms::warnings_of(&found) {
            if !warnings
                .iter()
                .any(|k| k.code == w.code && k.path == w.path)
            {
                warnings.push(w);
            }
        }
        let mut entity = (*e).clone();
        entity.base_mut().attrs = after;
        records.push((*slot, (*uid).clone(), entity, object));
    }
    // A unique field: against the layer's other objects and the call's other records.
    if !prepared.unique_keys().is_empty() {
        let ids: Vec<_> = records
            .iter()
            .filter_map(|(slot, ..)| doc.uid(*slot))
            .collect();
        for (i, (_, uid, e, object)) in records.iter().enumerate() {
            let others: Vec<&Attrs> = doc
                .by_layer_with_uids(&form.layer)
                .filter(|(u, _)| !ids.contains(u))
                .map(|(_, e)| &e.base().attrs)
                .chain(
                    records
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, r)| &r.2.base().attrs),
                )
                .collect();
            let taken = prepared.unique_issues(&e.base().attrs, object, others.into_iter());
            if !taken.is_empty() {
                return Err(invalid(input, uid, &taken));
            }
        }
    }
    Ok(Checked {
        records: records
            .into_iter()
            .map(|(slot, uid, e, _)| (slot, uid, e))
            .collect(),
        version,
        label: format!("Form: {} ({} nesne)", form.name, named.len()),
        warnings,
    })
}

/// The records that change, in the input's order.
fn changing(doc: &Document, c: &Checked) -> Vec<(Slot, String, Entity)> {
    c.records
        .iter()
        .filter(|(slot, _, e)| doc.get(*slot) != Some(e))
        .cloned()
        .collect()
}

fn answer(doc: &Document, c: &Checked, changed: &[(Slot, String, Entity)]) -> FormsFilled {
    FormsFilled {
        changed: changed.iter().map(|(_, uid, _)| uid.clone()).collect(),
        version: c.version,
        revision: doc.revision().to_string(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsFill) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsFill) -> CommandResult<FormsFilled> {
    match check(cx.doc, input) {
        Ok(c) => {
            let changed = changing(cx.doc, &c);
            CommandResult::Completed {
                output: answer(cx.doc, &c, &changed),
                warnings: c.warnings.clone(),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the records.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsFill) -> CommandResult<FormsFilled> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_FORMS_FILL,
        kentos_commands::CAD_FORMS_FILL_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: FormsFill) -> CommandResult<FormsFilled> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let changed = changing(cx.doc, &c);
    if !changed.is_empty() {
        cx.doc.update_many(
            changed
                .iter()
                .map(|(slot, _, e)| (*slot, e.clone()))
                .collect(),
            &c.label,
        );
    }
    CommandResult::Completed {
        output: answer(cx.doc, &c, &changed),
        warnings: c.warnings.clone(),
    }
}
