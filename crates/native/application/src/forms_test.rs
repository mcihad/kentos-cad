//! `cad.forms.test` v1 (docs/adr/0420): a form's scenarios run again on its draft or its latest
//! release, writing nothing; each passes or says what differs. The shared cases are
//! `fixtures/commands/v1/cad.forms.test.json`.

use kentos_contracts::{CommandResult, FormTestResult, FormsTest, FormsTested};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::forms;

fn answer(doc: &Document, input: &FormsTest) -> Result<FormsTested, Stop> {
    let form = forms::form(doc.settings(), &input.form, "form")?;
    let (body, _) = forms::body(form, input.draft, "draft")?;
    // Scenarios read no moment and no person: they are run again later (docs/adr/0420 §1).
    let variables = forms::variables_given(doc, None, None)?;
    let prepared = forms::prepared(doc, form, body, &variables, "form")?;
    let tests: Vec<&kentos_contracts::FormTest> = if input.names.is_empty() {
        body.tests.iter().collect()
    } else {
        let mut out = Vec::new();
        for (i, name) in input.names.iter().enumerate() {
            let t = body.tests.iter().find(|t| t.name == *name).ok_or_else(|| {
                forms::fail(
                    codes::UNKNOWN_TEST,
                    format!("“{}” formunda “{name}” senaryosu yok.", form.name),
                    &format!("names/{i}"),
                )
            })?;
            out.push(t);
        }
        out
    };
    let results: Vec<FormTestResult> = tests
        .into_iter()
        .map(|t| {
            let differences = kentos_forms::scenario::run(&prepared, t);
            FormTestResult {
                name: t.name.clone(),
                passed: differences.is_empty(),
                differences,
            }
        })
        .collect();
    Ok(FormsTested {
        passed: results.iter().filter(|r| r.passed).count() as u32,
        total: results.len() as u32,
        results,
    })
}

/// A body of `form` prepared as its scenarios are run (no moment, no person): the designer's
/// copy runs them as the command does; the reason it cannot be, else.
pub fn prepared_for(
    doc: &Document,
    form: &kentos_contracts::ProjectForm,
    body: &kentos_contracts::FormBody,
) -> Result<kentos_forms::Prepared, String> {
    let variables = forms::variables_given(doc, None, None).map_err(|s| s.into_error().message)?;
    forms::prepared(doc, form, body, &variables, "form").map_err(|s| s.into_error().message)
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &FormsTest) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The results; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &FormsTest) -> CommandResult<FormsTested> {
    match answer(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The results. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: FormsTest) -> CommandResult<FormsTested> {
    plan(cx, &input)
}
