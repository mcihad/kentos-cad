//! `cad.forms.recompute` v1 (Hesapları yenile, docs/adr/0421): every record of a form's layer completed
//! by the form's rules today, as a write through the form would leave it (its calculations, its hidden
//! fields emptied when the form says so; no defaults), the ones that change written as one undo step
//! (“Hesapları yenile: <ad>”). A record on which a calculation does not fit its field (or the form does
//! not settle) is left as it was and named in `failed`, with the warning `form_calc_failed`.
//! The shared cases are `fixtures/commands/v1/cad.forms.recompute.json`.

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, FormSeverity, FormsRecompute, FormsRecomputed,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::{self, fail};

struct Checked {
    checked: u32,
    changed: Vec<(Slot, String, Entity)>,
    failed: Vec<String>,
    version: Option<u32>,
    label: String,
}

fn check(doc: &Document, input: &FormsRecompute) -> Result<Checked, Stop> {
    let variables = forms::variables_given(doc, input.moment.as_deref(), input.user.as_deref())?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let form = forms::form(doc.settings(), &input.form, "form")?;
    let (body, version) = forms::body(form, input.draft, "draft")?;
    let prepared = forms::prepared(doc, form, body, &variables, "form")?;
    let layers = doc.layers();
    if layers.is_locked(&form.layer) {
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{}” katmanı kilitli; kayıtlar değiştirilemez. Kilidi Katmanlar panelinden açın.",
                layers
                    .get(&form.layer)
                    .map_or(form.layer.as_str(), |n| n.name.as_str())
            ),
            "form",
        ));
    }
    let mut checked = 0;
    let mut changed = Vec::new();
    let mut failed = Vec::new();
    for (uid, e) in doc.by_layer_with_uids(&form.layer) {
        checked += 1;
        let Some(slot) = doc.slot_of(uid) else {
            continue;
        };
        let object = forms::object(doc, slot, e);
        let (after, issues) = prepared.complete(&e.base().attrs, &object, false);
        if issues.iter().any(|i| i.severity == FormSeverity::Error) {
            failed.push(uid.to_string());
        } else if after != e.base().attrs {
            let mut entity = e.clone();
            entity.base_mut().attrs = after;
            changed.push((slot, uid.to_string(), entity));
        }
    }
    Ok(Checked {
        checked,
        changed,
        failed,
        version,
        label: format!("Hesapları yenile: {}", form.name),
    })
}

/// `form_calc_failed` when some records are left as they were.
fn warnings(c: &Checked) -> Vec<CommandWarning> {
    if c.failed.is_empty() {
        return Vec::new();
    }
    vec![CommandWarning {
        code: codes::FORM_CALC_FAILED.into(),
        message: format!(
            "{} kayıt olduğu gibi kaldı: formun hesabı alanına sığmıyor. Kayıtları formla açıp düzeltin.",
            c.failed.len()
        ),
        path: Some("form".into()),
    }]
}

fn answer(doc: &Document, c: &Checked) -> FormsRecomputed {
    FormsRecomputed {
        checked: c.checked,
        changed: c.changed.iter().map(|(_, uid, _)| uid.clone()).collect(),
        failed: c.failed.clone(),
        version: c.version,
        revision: doc.revision().to_string(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsRecompute) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would do; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsRecompute) -> CommandResult<FormsRecomputed> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: answer(cx.doc, &c),
            warnings: warnings(&c),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the records that change as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: FormsRecompute,
) -> CommandResult<FormsRecomputed> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_FORMS_RECOMPUTE,
        kentos_contracts::CAD_FORMS_RECOMPUTE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: FormsRecompute,
) -> CommandResult<FormsRecomputed> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    if !c.changed.is_empty() {
        cx.doc.update_many(
            c.changed
                .iter()
                .map(|(slot, _, e)| (*slot, e.clone()))
                .collect(),
            &c.label,
        );
    }
    CommandResult::Completed {
        output: answer(cx.doc, &c),
        warnings: warnings(&c),
    }
}
