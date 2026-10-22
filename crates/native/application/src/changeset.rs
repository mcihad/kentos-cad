//! `cad.changeset.apply` v1 (docs/adr/0253 §8): several commands as one
//! change. Every step runs through the registry, the same handler a tool
//! runs, inside the one step of the document the set's `execute` is: a step
//! refused takes back every step, the parts outside the undo history too;
//! the linked texts, hatches and dimensions follow once, after the last
//! step, and the document's integrity is checked then (`call::recorded`).
//!
//! A later step names an earlier one's output by a text that is the whole
//! value: `"$ad"` (its `uid`), `"$ad[i]"` (its `created[i]`), `"$ad.alan"`,
//! `"$ad.alan[i]"`. The plan is the set executed on a reading copy.

use std::collections::BTreeMap;

use kentos_commands::{
    CAD_CHANGESET_APPLY, CAD_CHANGESET_APPLY_VERSION, CHANGESET_LABEL, CallOp, ChangesetApplied,
    ChangesetApply, ChangesetStepDone, CommandError, CommandResult, CommandWarning, FILE_COMMANDS,
    codes,
};
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::registry;

/// Checks the set, and that every step would be written, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ChangesetApply) -> CommandResult<()> {
    match plan_in(cx, input) {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed {
            output: (),
            warnings,
        },
        other => relay(other),
    }
}

/// What execute would write now: the set run on a reading copy, its steps'
/// outputs, and the revision to expect for exactly that.
pub fn plan(cx: &ExecutionContext<'_>, input: &ChangesetApply) -> CommandResult<ChangesetApplied> {
    plan_in(cx, input)
}

fn plan_in(cx: &ExecutionContext<'_>, input: &ChangesetApply) -> CommandResult<ChangesetApplied> {
    if let Err(stop) = check(cx.doc, input) {
        return stop.into();
    }
    let revision = cx.doc.revision().to_string();
    let mut copy = cx.doc.reading_copy();
    let mut inner = ExecutionContext::with_optional_store(&mut copy, cx.store);
    match execute(&mut inner, input.clone()) {
        CommandResult::Completed {
            output: mut applied,
            warnings,
        } => {
            // A plan writes nothing: its revision is the one to expect, its steps' too.
            for step in &mut applied.steps {
                if let Some(r) = step.output.get_mut("revision")
                    && r.is_string()
                {
                    *r = Value::String(revision.clone());
                }
            }
            applied.revision = revision;
            CommandResult::Completed {
                output: applied,
                warnings,
            }
        }
        other => other,
    }
}

/// Writes every step as one change named `label`, or nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ChangesetApply,
) -> CommandResult<ChangesetApplied> {
    crate::call::recorded(
        cx,
        CAD_CHANGESET_APPLY,
        CAD_CHANGESET_APPLY_VERSION,
        input,
        write,
    )
}

fn write(cx: &mut ExecutionContext<'_>, input: ChangesetApply) -> CommandResult<ChangesetApplied> {
    // run_step owns atomic refusal; this scope also makes successful paper
    // changes part of the model's undo step (ADR 0590).
    let label = input
        .label
        .as_deref()
        .filter(|label| !checks::is_blank(label))
        .unwrap_or(CHANGESET_LABEL);
    let group = cx.doc.begin_group(label);
    let mut result = write_steps(cx, input);
    if matches!(result, CommandResult::Completed { .. }) {
        cx.doc.end_group(group);
        if let CommandResult::Completed { output, .. } = &mut result {
            let revision = cx.doc.revision().to_string();
            output.revision = revision.clone();
            for step in &mut output.steps {
                if let Some(r) = step.output.get_mut("revision")
                    && r.is_string()
                {
                    *r = Value::String(revision.clone());
                }
            }
        }
    } else {
        cx.doc.cancel_group(group);
    }
    result
}

fn write_steps(
    cx: &mut ExecutionContext<'_>,
    input: ChangesetApply,
) -> CommandResult<ChangesetApplied> {
    if let Err(stop) = check(cx.doc, &input) {
        return stop.into();
    }
    let label = input
        .label
        .clone()
        .filter(|l| !checks::is_blank(l))
        .unwrap_or_else(|| CHANGESET_LABEL.to_owned());
    cx.doc.name_step(&label);
    let mut outputs: BTreeMap<String, Value> = BTreeMap::new();
    let mut done = Vec::with_capacity(input.steps.len());
    let mut warnings = Vec::new();
    for (i, step) in input.steps.iter().enumerate() {
        let at = format!("steps[{i}].input");
        let filled = match fill(&step.input, &outputs, &input, i, &at) {
            Ok(v) => v,
            Err(stop) => return stop.into(),
        };
        let answer = match registry::run(cx, &step.command, CallOp::Execute, filled) {
            Ok(v) => v,
            Err(unrun) => {
                return CommandResult::Failed {
                    error: error(unrun.code, unrun.message, Some(at)),
                };
            }
        };
        let result: CommandResult<Value> = match serde_json::from_value(answer) {
            Ok(r) => r,
            Err(e) => {
                return CommandResult::Failed {
                    error: error(
                        codes::INVALID_INPUT,
                        format!("{} adımının cevabı okunamadı: {e}", i + 1),
                        Some(at),
                    ),
                };
            }
        };
        match result {
            CommandResult::Completed {
                output,
                warnings: own,
            } => {
                warnings.extend(own.iter().map(|w| CommandWarning {
                    path: Some(match &w.path {
                        Some(p) => format!("{at}.{p}"),
                        None => at.clone(),
                    }),
                    ..w.clone()
                }));
                if let Some(name) = &step.name {
                    outputs.insert(name.clone(), output.clone());
                }
                done.push(ChangesetStepDone {
                    command: step.command.clone(),
                    output,
                    warnings: own,
                });
            }
            CommandResult::Failed { error } => {
                return CommandResult::Failed {
                    error: within(error, &at, i),
                };
            }
            CommandResult::Conflict { error } => {
                return CommandResult::Conflict {
                    error: within(error, &at, i),
                };
            }
            CommandResult::NeedsInput { error } => {
                return CommandResult::NeedsInput {
                    error: within(error, &at, i),
                };
            }
            CommandResult::Queued { job_id } => return CommandResult::Queued { job_id },
            CommandResult::Cancelled => return CommandResult::Cancelled,
        }
        if cx.stopped() {
            return CommandResult::Cancelled;
        }
        cx.progress(i as u64 + 1, input.steps.len() as u64);
    }
    // A step's revision is the set's: the steps are one write.
    let revision = cx.doc.revision().to_string();
    for step in &mut done {
        if let Some(r) = step.output.get_mut("revision")
            && r.is_string()
        {
            *r = Value::String(revision.clone());
        }
    }
    CommandResult::Completed {
        output: ChangesetApplied {
            label,
            steps: done,
            revision,
        },
        warnings,
    }
}

/// A step's error as the set's: at the step's input, and saying which step.
fn within(error: CommandError, at: &str, i: usize) -> CommandError {
    CommandError {
        path: Some(match &error.path {
            Some(p) => format!("{at}.{p}"),
            None => at.to_owned(),
        }),
        message: format!("{}. adım: {}", i + 1, error.message),
        ..error
    }
}

/// The set's own checks, in order (see the contract).
fn check(doc: &kentos_domain::Document, input: &ChangesetApply) -> Result<(), Stop> {
    if let Some(scope) = &input.model_scope {
        let book = doc.sheets().ok_or_else(|| Stop::failed(error("scope_mismatch", "Projede pafta kitabı yok.".into(), Some("modelScope".into()))))?;
        let (book,_) = kentos_sheet::project::read_project_sheets(book).map_err(|e|Stop::failed(error(&e.code,e.message,Some("modelScope".into()))))?;
        kentos_sheet::model_scope::Viewport::resolve_with_views(&book,&scope.sheet,&scope.item,&doc.settings().named_views).map_err(|e|Stop::failed(error(&e.code,e.message,Some("modelScope".into()))))?;
        for (i,step) in input.steps.iter().enumerate() {
            if !step.command.starts_with("cad.") || ["cad.sheets.","cad.views.","cad.settings.","cad.units.","cad.project.","cad.plot."].iter().any(|p| step.command.starts_with(p)) {
                return Err(Stop::failed(error("model_scope_command", "Bu komut model düzenleme kapsamında kullanılamaz.".into(), Some(format!("steps[{i}].command")))));
            }
        }
    }
    if input.steps.is_empty() {
        return Err(Stop::failed(error(
            codes::NO_STEPS,
            "Değişiklik kümesinde hiç adım yok; en az bir komut verin.".into(),
            Some("steps".into()),
        )));
    }
    let mut names: Vec<&str> = Vec::new();
    for (i, step) in input.steps.iter().enumerate() {
        let at = |field: &str| Some(format!("steps[{i}].{field}"));
        if step.command == CAD_CHANGESET_APPLY {
            return Err(Stop::failed(error(
                codes::NESTED_CHANGESET,
                "Değişiklik kümesi iç içe verilemez; adımlarını bu kümeye ekleyin.".into(),
                at("command"),
            )));
        }
        if step.command.starts_with("file.")
            || FILE_COMMANDS.iter().any(|(c, _)| *c == step.command)
        {
            return Err(Stop::failed(error(
                codes::SIDE_EFFECT_IN_CHANGESET,
                format!(
                    "{} dosyaya yazar; yan etkisi geri alınamadığı için değişiklik kümesine giremez. Kümeden sonra ayrıca çağırın.",
                    step.command
                ),
                at("command"),
            )));
        }
        let Some(version) = registry::version_of(&step.command) else {
            return Err(Stop::failed(error(
                codes::UNKNOWN_COMMAND,
                format!("“{}” bu çizimde çalışan bir komut değil.", step.command),
                at("command"),
            )));
        };
        if let Some(v) = step.version
            && v != version
        {
            return Err(Stop::failed(error(
                codes::UNKNOWN_VERSION,
                format!(
                    "{} için {v}. sürüm yok; bu sürüm {version}. sürümü çalıştırır.",
                    step.command
                ),
                at("version"),
            )));
        }
        if let Some(name) = &step.name {
            let plain = name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !plain || names.contains(&name.as_str()) {
                return Err(Stop::failed(error(
                    codes::INVALID_INPUT,
                    format!(
                        "Adımın adı “{name}” olamaz: bir harf ya da _ ile başlamalı, harf, rakam ve _ içermeli ve bir kez verilmeli."
                    ),
                    at("as"),
                )));
            }
            names.push(name);
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())
}

/// The step's input with its references to earlier steps filled in.
fn fill(
    value: &Value,
    outputs: &BTreeMap<String, Value>,
    input: &ChangesetApply,
    step: usize,
    at: &str,
) -> Result<Value, Stop> {
    Ok(match value {
        Value::String(text) => match reference(text) {
            Some((name, field, index)) => {
                let declared = input
                    .steps
                    .iter()
                    .position(|s| s.name.as_deref() == Some(name));
                match (declared, outputs.get(name)) {
                    (Some(_), Some(output)) => {
                        let field =
                            field.unwrap_or(if index.is_some() { "created" } else { "uid" });
                        let found = output.get(field).and_then(|v| match index {
                            Some(i) => v.get(i),
                            None => Some(v),
                        });
                        match found {
                            Some(v) => v.clone(),
                            None => return Err(unknown_reference(text, at, step)),
                        }
                    }
                    (Some(_), None) => return Err(unknown_reference(text, at, step)),
                    // Not a step's name: the text is the text.
                    (None, _) => value.clone(),
                }
            }
            None => value.clone(),
        },
        Value::Array(items) => Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(i, v)| fill(v, outputs, input, step, &format!("{at}[{i}]")))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| {
                    Ok((
                        k.clone(),
                        fill(v, outputs, input, step, &format!("{at}.{k}"))?,
                    ))
                })
                .collect::<Result<_, Stop>>()?,
        ),
        other => other.clone(),
    })
}

/// `$ad`, `$ad[i]`, `$ad.alan`, `$ad.alan[i]`: the name, the field, the index.
fn reference(text: &str) -> Option<(&str, Option<&str>, Option<usize>)> {
    let rest = text.strip_prefix('$')?;
    let (head, index) = match rest.strip_suffix(']').and_then(|r| r.rsplit_once('[')) {
        Some((head, i)) => (head, Some(i.parse::<usize>().ok()?)),
        None => (rest, None),
    };
    let (name, field) = match head.split_once('.') {
        Some((n, f)) => (n, Some(f)),
        None => (head, None),
    };
    let ident = |s: &str| {
        s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    (ident(name) && field.is_none_or(ident)).then_some((name, field, index))
}

fn unknown_reference(text: &str, at: &str, step: usize) -> Stop {
    Stop::failed(error(
        codes::UNKNOWN_REFERENCE,
        format!(
            "{}. adımın “{text}” başvurusu daha önceki bir adımın çıktısını adlamıyor: adı önceki bir adımın “as”'i olmalı, alanı ve sırası o adımın çıktısında bulunmalı.",
            step + 1
        ),
        Some(at.to_owned()),
    ))
}

/// The set's answer that wrote nothing, as another type's.
fn relay<T, U>(result: CommandResult<T>) -> CommandResult<U> {
    match result {
        CommandResult::Completed { .. } | CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::Failed { error } => CommandResult::Failed { error },
    }
}

#[cfg(test)]
mod tests {
    use super::reference;

    #[test]
    fn references_are_whole_texts_naming_a_step() {
        assert_eq!(reference("$yol"), Some(("yol", None, None)));
        assert_eq!(reference("$yol[2]"), Some(("yol", None, Some(2))));
        assert_eq!(reference("$k.layer"), Some(("k", Some("layer"), None)));
        assert_eq!(
            reference("$k.created[0]"),
            Some(("k", Some("created"), Some(0)))
        );
        assert_eq!(reference("$5"), None);
        assert_eq!(reference("fiyat $5"), None);
        assert_eq!(reference("$a b"), None);
    }
}
