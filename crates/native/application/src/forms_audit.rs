//! `cad.forms.audit` v1 (docs/adr/0406 §1): every record of a form's layer checked by its rules,
//! writing nothing. Each record is completed as the document keeps it (its calculations; no defaults:
//! the record exists) and judged as a write through the form would judge it; a unique field's value
//! held by another object of the layer is said. The records are counted whole; those with an error
//! (and with `warnings` those with only warnings) are listed in the layer's order, up to `limit`. The
//! shared cases are `fixtures/commands/v1/cad.forms.audit.json`.

use std::collections::HashMap;

use kentos_contracts::{AuditedRecord, CommandResult, FormSeverity, FormsAudit, FormsAudited};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::forms::{self, fail};

/// The most records a call lists, and how many it lists when none is said.
const MOST: u32 = 10_000;
const LISTED: u32 = 500;

fn answer(doc: &Document, input: &FormsAudit) -> Result<FormsAudited, Stop> {
    if input.limit.is_some_and(|n| n == 0 || n > MOST) {
        return Err(fail(
            codes::INVALID_COUNT,
            format!("Listelenecek kayıt sayısı 1 ile {MOST} arasında olmalı."),
            "limit",
        ));
    }
    let variables = forms::variables_given(doc, input.moment.as_deref(), input.user.as_deref())?;
    let form = forms::form(doc.settings(), &input.form, "form")?;
    let (body, version) = forms::body(form, input.draft, "draft")?;
    let prepared = forms::prepared(doc, form, body, &variables, "form")?;
    let fields = forms::fields(doc, &form.layer);
    // A unique field's values, counted once over the layer (a record is said when another holds it).
    let unique: Vec<String> = prepared
        .unique_keys()
        .iter()
        .filter_map(|k| {
            fields
                .iter()
                .find(|f| f.key() == *k)
                .map(|f| f.name.clone())
        })
        .collect();
    let mut held: HashMap<(usize, String), u32> = HashMap::new();
    for (_, e) in doc.by_layer_with_uids(&form.layer) {
        for (i, name) in unique.iter().enumerate() {
            if let Some(v) = e
                .base()
                .attrs
                .get(name)
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
            {
                *held.entry((i, v.to_owned())).or_default() += 1;
            }
        }
    }
    let limit = input.limit.unwrap_or(LISTED) as usize;
    let mut out = FormsAudited {
        version,
        checked: 0,
        failing: 0,
        warned: 0,
        records: Vec::new(),
        more: false,
    };
    for (uid, e) in doc.by_layer_with_uids(&form.layer) {
        out.checked += 1;
        let Some(slot) = doc.slot_of(uid) else {
            continue;
        };
        let object = forms::object(doc, slot, e);
        let (after, mut issues) = prepared.complete(&e.base().attrs, &object, false);
        let said: Vec<Option<String>> = issues.iter().map(|i| i.field.clone()).collect();
        issues.extend(
            prepared
                .validate(&after, &object)
                .into_iter()
                .filter(|i| !said.contains(&i.field)),
        );
        if unique.iter().enumerate().any(|(i, name)| {
            after
                .get(name)
                .map(|v| v.trim())
                .is_some_and(|v| held.get(&(i, v.to_owned())).is_some_and(|n| *n > 1))
        }) {
            let others = doc
                .by_layer_with_uids(&form.layer)
                .filter(|(u, _)| *u != uid)
                .map(|(_, x)| &x.base().attrs);
            issues.extend(prepared.unique_issues(&after, &object, others));
        }
        let errored = issues.iter().any(|i| i.severity == FormSeverity::Error);
        if errored {
            out.failing += 1;
        } else if !issues.is_empty() {
            out.warned += 1;
        }
        if errored || (input.warnings && !issues.is_empty()) {
            if out.records.len() < limit {
                out.records.push(AuditedRecord {
                    uid: uid.to_string(),
                    issues,
                });
            } else {
                out.more = true;
            }
        }
    }
    Ok(out)
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsAudit) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The answer, as execute gives it.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsAudit) -> CommandResult<FormsAudited> {
    respond(cx.doc, input)
}

/// The answer; nothing is written.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsAudit) -> CommandResult<FormsAudited> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_FORMS_AUDIT,
        kentos_contracts::CAD_FORMS_AUDIT_VERSION,
        input,
        |cx, input| respond(cx.doc, &input),
    )
}

fn respond(doc: &Document, input: &FormsAudit) -> CommandResult<FormsAudited> {
    match answer(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
