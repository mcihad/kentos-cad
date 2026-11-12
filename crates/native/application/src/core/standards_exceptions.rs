//! `cad.standards.exceptions` v1 (docs/adr/1129): standards findings accepted with a reason, or the acceptance
//! withdrawn. The findings are named as today's `cad.standards.check` gives them against the standard; an accepted one
//! keeps the standard's source and revision and the finding's expected and found values, so the check reports it
//! lapsed once either changes. The accepted findings are the project's settings (`standardExceptions`): writing them
//! makes the drawing dirty and is no undo step, as topology exceptions are (docs/adr/0589 §3, 1260 §2). Shared cases:
//! `fixtures/commands/v1/cad.standards.exceptions.json`.

use kentos_commands::{
    CommandResult, ProjectSettings, StandardException, StandardExceptionOperation, StandardIssueState,
    StandardsExceptions, StandardsExceptionsEdited, standard_exception_reason_problem, MAX_STANDARD_EXCEPTION_NAME, MAX_STANDARD_EXCEPTION_REASON,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::standards::{checked_report, kind_name, source_of, standard_of};

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The project's accepted findings after the operation, and how many it changed.
struct Checked {
    exceptions: Vec<StandardException>,
    changed: usize,
}

fn check_input(input: &StandardsExceptions) -> Result<(), Stop> {
    if input.findings.is_empty() {
        return Err(fail(
            codes::INVALID_INPUT,
            "Bulgu verilmedi; cad.standards.check'in bulgularından en az birini (tür, konu, katman) verin.".into(),
            "findings",
        ));
    }
    for (i, k) in input.findings.iter().enumerate() {
        if input.findings[..i].contains(k) {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("“{}” bulgusu birden çok kez verildi.", k.subject),
                &format!("findings[{i}]"),
            ));
        }
    }
    let withdraw = input.operation == StandardExceptionOperation::Withdraw;
    for (field, given) in [("reason", &input.reason), ("by", &input.by), ("at", &input.at)] {
        if withdraw && given.is_some() {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("Kabulü geri almak {field} istemez; yalnız bulguları verin."),
                field,
            ));
        }
    }
    for (field, given) in [("by", &input.by), ("at", &input.at)] {
        if let Some(v) = given
            && (v.trim().is_empty() || v.chars().count() > MAX_STANDARD_EXCEPTION_NAME || v.chars().any(char::is_control))
        {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("{field} boş, satır sonlu ya da {MAX_STANDARD_EXCEPTION_NAME} karakterden uzun olamaz."),
                field,
            ));
        }
    }
    if !withdraw {
        match input.reason.as_deref() {
            Some(r) if !r.trim().is_empty() => {
                if standard_exception_reason_problem(r).is_some() {
                    return Err(fail(
                        codes::INVALID_INPUT,
                        format!(
                            "Gerekçe {} karakter; en çok {MAX_STANDARD_EXCEPTION_REASON} karakter olur, kısaltın.",
                            r.chars().count()
                        ),
                        "reason",
                    ));
                }
            }
            _ => {
                return Err(fail(
                    codes::REASON_REQUIRED,
                    "Bulguyu kabul etmek için gerekçesini yazın (reason): neden standarttan ayrıldığı kayda geçer.".into(),
                    "reason",
                ));
            }
        }
    }
    Ok(())
}

fn check(doc: &Document, input: &StandardsExceptions) -> Result<Checked, Stop> {
    check_input(input)?;
    let standard = standard_of(input.name.as_deref(), input.standard.as_ref())?;
    let report = checked_report(doc, input.name.as_deref(), input.standard.as_ref(), input.resources.as_ref(), input.origin.as_ref())?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let origin = source_of(&standard, input.origin.as_ref());
    let mut exceptions = doc.settings().standard_exceptions.clone();
    let mut changed = 0;
    for (i, key) in input.findings.iter().enumerate() {
        let issue = report
            .issues
            .iter()
            .find(|f| f.kind == key.kind && f.subject == key.subject && f.layer == key.layer);
        let kind = kind_name(key.kind);
        let ours = |e: &StandardException| {
            e.kind == kind && e.subject == key.subject && e.layer == key.layer && e.standard == origin.id
        };
        match input.operation {
            StandardExceptionOperation::Accept => {
                let Some(issue) = issue else {
                    return Err(fail(
                        codes::FINDING_NOT_FOUND,
                        format!(
                            "Çizimin bugünkü denetiminde “{}” için {kind} bulgusu yok; önce cad.standards.check ile aynı standartla denetleyin.",
                            key.subject
                        ),
                        &format!("findings[{i}]"),
                    ));
                };
                let next = StandardException {
                    kind: kind.clone(),
                    subject: issue.subject.clone(),
                    layer: issue.layer.clone(),
                    standard: origin.id.clone(),
                    revision: origin.revision.clone(),
                    expected: issue.expected.clone(),
                    found: issue.found.clone(),
                    reason: input.reason.clone().unwrap_or_default(),
                    by: input.by.clone(),
                    at: input.at.clone(),
                };
                let same = issue.state == StandardIssueState::Accepted
                    && exceptions.iter().any(|e| ours(e) && *e == next);
                if same {
                    continue;
                }
                exceptions.retain(|e| !ours(e));
                exceptions.push(next);
                changed += 1;
            }
            StandardExceptionOperation::Withdraw => {
                if !exceptions.iter().any(ours) {
                    return Err(fail(
                        codes::FINDING_NOT_FOUND,
                        format!(
                            "“{}” için {kind} bulgusu “{}” standardına karşı kabul edilmemiş; geri alınacak kabul yok.",
                            key.subject, origin.id
                        ),
                        &format!("findings[{i}]"),
                    ));
                }
                exceptions.retain(|e| !ours(e));
                changed += 1;
            }
        }
    }
    Ok(Checked { exceptions, changed })
}

fn edited(doc: &Document, checked: &Checked) -> StandardsExceptionsEdited {
    StandardsExceptionsEdited {
        changed: u32::try_from(checked.changed).unwrap_or(u32::MAX),
        exceptions: u32::try_from(checked.exceptions.len()).unwrap_or(u32::MAX),
        revision: doc.revision().to_string(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &StandardsExceptions) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &StandardsExceptions) -> CommandResult<StandardsExceptionsEdited> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: edited(cx.doc, &c), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: StandardsExceptions) -> CommandResult<StandardsExceptionsEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_STANDARDS_EXCEPTIONS,
        kentos_commands::CAD_STANDARDS_EXCEPTIONS_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the document (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: StandardsExceptions) -> CommandResult<StandardsExceptionsEdited> {
    let checked = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    if checked.changed > 0 {
        let doc = &mut *cx.doc;
        doc.set_settings(ProjectSettings { standard_exceptions: checked.exceptions.clone(), ..doc.settings().clone() });
    }
    CommandResult::Completed { output: edited(cx.doc, &checked), warnings: Vec::new() }
}
