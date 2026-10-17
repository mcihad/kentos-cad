//! `cad.forms.compare` v1 (Sürümleri karşılaştır, docs/adr/0427): what changed from one version of a
//! form to another, read-only. The shared cases are `fixtures/commands/v1/cad.forms.compare.json`.

use kentos_contracts::{CommandResult, FormBody, FormsCompare, FormsCompared};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::forms::{self, fail};

fn answer(doc: &Document, input: &FormsCompare) -> Result<FormsCompared, Stop> {
    let form = forms::form(doc.settings(), &input.form, "form")?;
    let release = |v: u32, path: &str| -> Result<&FormBody, Stop> {
        form.releases
            .iter()
            .find(|r| r.version == v)
            .map(|r| &r.body)
            .ok_or_else(|| {
                fail(
                    codes::UNKNOWN_RELEASE,
                    format!("“{}” formunun {v}. yayını yok.", form.name),
                    path,
                )
            })
    };
    let (from, old) = match input.from {
        Some(v) => (v, release(v, "from")?),
        None => {
            let Some(last) = form.releases.last() else {
                return Err(fail(
                    codes::FORM_UNPUBLISHED,
                    format!(
                        "“{}” formu hiç yayınlanmadı; karşılaştırılacak yayın yok.",
                        form.name
                    ),
                    "from",
                ));
            };
            (last.version, &last.body)
        }
    };
    let new = match input.to {
        Some(v) => release(v, "to")?,
        None => &form.draft,
    };
    Ok(FormsCompared {
        from,
        to: input.to,
        changes: kentos_forms::compare::compare(old, new),
        requires_added: kentos_forms::requirements(new)
            .into_iter()
            .filter(|r| !kentos_forms::requirements(old).contains(r))
            .collect(),
        requires_removed: kentos_forms::requirements(old)
            .into_iter()
            .filter(|r| !kentos_forms::requirements(new).contains(r))
            .collect(),
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsCompare) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The changes; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsCompare) -> CommandResult<FormsCompared> {
    match answer(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// As plan: a query writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsCompare) -> CommandResult<FormsCompared> {
    plan(cx, &input)
}
