//! `cad.forms.export` v1 (Formu dışa aktar, docs/adr/0435): a form as a package: its latest release (or
//! its draft) with the fields it binds or reads and the value lists they link; read-only. The shared
//! cases are `fixtures/commands/v1/cad.forms.export.json`.

use kentos_commands::{
    CommandResult, FORM_PACKAGE_FORMAT, FORM_PACKAGE_VERSION, FieldDomain, FormPackage, FormsExport,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::forms::{self, fail};

/// The package of `input`'s form.
pub(crate) fn package(doc: &Document, input: &FormsExport) -> Result<FormPackage, Stop> {
    let form = forms::form(doc.settings(), &input.form, "form")?;
    let body = if input.draft {
        form.draft.clone()
    } else {
        match form.releases.last() {
            Some(r) => r.body.clone(),
            None => {
                return Err(fail(
                    codes::FORM_UNPUBLISHED,
                    format!(
                        "“{}” formu hiç yayınlanmadı; taslağı draft ile dışa aktarın ya da önce yayınlayın.",
                        form.name
                    ),
                    "draft",
                ));
            }
        }
    };
    let layer_fields = doc
        .layers()
        .get(&form.layer)
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    let keys = kentos_forms::field_refs::read_by(&body, &layer_fields);
    let fields: Vec<_> = layer_fields
        .into_iter()
        .filter(|f| keys.iter().any(|k| k == f.key()))
        .collect();
    let mut domains: Vec<FieldDomain> = Vec::new();
    for f in &fields {
        if let Some(d) = f
            .domain
            .as_ref()
            .and_then(|id| doc.settings().domains.iter().find(|d| d.id == *id))
            && !domains.iter().any(|x| x.id == d.id)
        {
            domains.push(d.clone());
        }
    }
    Ok(FormPackage {
        format: FORM_PACKAGE_FORMAT.into(),
        version: FORM_PACKAGE_VERSION,
        name: form.name.clone(),
        body,
        fields,
        domains,
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsExport) -> CommandResult<()> {
    match package(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The package; nothing is written.
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsExport) -> CommandResult<FormPackage> {
    match package(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// As plan: a query writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsExport) -> CommandResult<FormPackage> {
    plan(cx, &input)
}
