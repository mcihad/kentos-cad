//! `cad.parameters.edit` v1 (docs/adr/0336 §1; AutoCAD's parameter manager): the drawing's parameters added, set,
//! renamed and taken away by name, then the drawing solved with their new values; the parameters and every object
//! the solve moves in one undo step “Parametreler”.
//!
//! The checks, in order (the first that fails answers): the edits' own names and expressions; the expected revision;
//! each edit against the table as the edits before it left it (a name missing or taken, an expression for a
//! dimension that only measures, a parameter read or a dimension's taken away); then what `cad.constraints.edit`
//! checks of the set and its solve (`constraints_edit::settle`).

use std::collections::BTreeSet;

use kentos_commands::{
    BoundChange, CadParameter, CommandResult, ConstraintRule, ConstraintSet, ParameterEdit,
    ParameterValue, ParametersEdit, ParametersEditPlan, ParametersEdited, param_expr_names,
    parameter_name_valid, parse_param_expr, rename_in_expression,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::constraint_solve::{Solved, parameter_values};
use crate::constraints_edit::settle;

/// The undo step's name.
pub const LABEL: &str = "Parametreler";

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ParametersEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ParametersEdit,
) -> CommandResult<ParametersEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over a document read in place (the parameter window's preview).
pub fn plan_in(doc: &Document, input: &ParametersEdit) -> CommandResult<ParametersEditPlan> {
    match check(doc, input) {
        Ok((set, solved)) => CommandResult::Completed {
            output: ParametersEditPlan {
                values: values_of(doc, &set),
                parameters: set.parameters,
                changed: solved.changed.iter().map(|(_, e)| e.clone()).collect(),
                diagnosis: solved.diagnosis,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the parameters and the moved objects as one undo step “Parametreler”.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ParametersEdit,
) -> CommandResult<ParametersEdited> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_PARAMETERS_EDIT,
        kentos_commands::CAD_PARAMETERS_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ParametersEdit,
) -> CommandResult<ParametersEdited> {
    // A job's stop reaches the solve (docs/adr/0337 §4).
    let job = cx.job;
    let stopped = move || job.is_some_and(crate::jobs::Progress::stopped);
    let (set, solved) = match check_with(cx.doc, &input, &stopped) {
        Ok(planned) => planned,
        Err(stop) => return stop.into(),
    };
    let changed: Vec<String> = solved
        .changed
        .iter()
        .filter_map(|(slot, _)| cx.doc.uid(*slot))
        .map(|u| u.to_string())
        .collect();
    let diagnosis = solved.diagnosis.clone();
    let written = cx.doc.transact(LABEL, |doc| {
        for (slot, entity) in solved.changed {
            doc.update(slot, entity);
        }
        doc.set_constraints(set, LABEL).map(|_| ())
    });
    match written {
        Ok(()) => CommandResult::Completed {
            output: ParametersEdited {
                values: values_of(cx.doc, cx.doc.constraints()),
                changed,
                diagnosis,
                revision: cx.doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(refusal) => CommandResult::Failed {
            error: checks::error(
                codes::INVALID_CONSTRAINT,
                refusal.to_string(),
                Some("edits".into()),
            ),
        },
    }
}

/// Every parameter's value in its own unit, in the table's order (none written for one that cannot be worked out).
fn values_of(doc: &Document, set: &ConstraintSet) -> Vec<ParameterValue> {
    let values = parameter_values(doc, set).unwrap_or_default();
    set.parameters
        .iter()
        .filter_map(|p| {
            values.get(&p.name).map(|v| ParameterValue {
                name: p.name.clone(),
                kind: p.kind,
                value: *v,
            })
        })
        .collect()
}

fn refuse(code: &str, message: String, path: String) -> Stop {
    Stop::failed(checks::error(code, message, Some(path)))
}

fn name_check(name: &str, path: String) -> Result<(), Stop> {
    if parameter_name_valid(name) {
        return Ok(());
    }
    Err(refuse(
        codes::INVALID_PARAMETER,
        format!(
            "“{name}” bir parametre adı değil: harfle başlar; harf, rakam ve _ içerir, işlev adı olmaz."
        ),
        path,
    ))
}

fn expression_check(text: &str, path: String) -> Result<(), Stop> {
    match parse_param_expr(text) {
        Ok(_) => Ok(()),
        Err(words) => Err(refuse(
            codes::PARAMETER_EXPRESSION,
            format!("“{text}” okunamadı: {words}"),
            path,
        )),
    }
}

/// The edits' own names and expressions.
fn check_shape(input: &ParametersEdit) -> Result<(), Stop> {
    for (i, e) in input.edits.iter().enumerate() {
        let at = |field: &str| format!("edits[{i}].{field}");
        match e {
            ParameterEdit::Add { parameter } => {
                name_check(&parameter.name, at("parameter.name"))?;
                if parameter.expression.is_empty() {
                    return Err(refuse(
                        codes::PARAMETER_EXPRESSION,
                        "Kullanıcı parametresinin bir ifadesi olur (3.5, 120cm, genislik / 2)."
                            .into(),
                        at("parameter.expression"),
                    ));
                }
                expression_check(&parameter.expression, at("parameter.expression"))?;
            }
            ParameterEdit::Set {
                name, expression, ..
            } => {
                name_check(name, at("name"))?;
                if let Some(text) = expression {
                    expression_check(text, at("expression"))?;
                }
            }
            ParameterEdit::Rename { name, to } => {
                name_check(name, at("name"))?;
                name_check(to, at("to"))?;
            }
            ParameterEdit::Remove { name } => name_check(name, at("name"))?,
        }
    }
    Ok(())
}

/// The parameters another parameter reads.
fn read_names(set: &ConstraintSet) -> BTreeSet<String> {
    set.parameters
        .iter()
        .filter_map(|p| parse_param_expr(&p.expression).ok())
        .flat_map(|e| param_expr_names(&e))
        .collect()
}

fn find<'a>(
    set: &'a mut ConstraintSet,
    name: &str,
    path: String,
) -> Result<&'a mut CadParameter, Stop> {
    set.parameters
        .iter_mut()
        .find(|p| p.name == name)
        .ok_or_else(|| {
            refuse(
                codes::PARAMETER_NOT_FOUND,
                format!("“{name}” diye bir parametre yok."),
                path,
            )
        })
}

fn bound(change: Option<BoundChange>, now: Option<f64>) -> Option<f64> {
    match change {
        None => now,
        Some(BoundChange::Clear) => None,
        Some(BoundChange::To { value }) => Some(value),
    }
}

fn check(doc: &Document, input: &ParametersEdit) -> Result<(ConstraintSet, Solved), Stop> {
    check_with(doc, input, &|| false)
}

fn check_with(
    doc: &Document,
    input: &ParametersEdit,
    stop: &dyn Fn() -> bool,
) -> Result<(ConstraintSet, Solved), Stop> {
    check_shape(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut set = doc.constraints().clone();
    for (i, e) in input.edits.iter().enumerate() {
        let at = |field: &str| format!("edits[{i}].{field}");
        match e {
            ParameterEdit::Add { parameter } => {
                if set.parameter(&parameter.name).is_some() {
                    return Err(refuse(
                        codes::PARAMETER_EXISTS,
                        format!(
                            "“{}” adında bir parametre zaten var; başka bir ad verin.",
                            parameter.name
                        ),
                        at("parameter.name"),
                    ));
                }
                set.parameters.push(parameter.clone());
            }
            ParameterEdit::Set {
                name,
                expression,
                min,
                max,
                description,
            } => {
                let measured = set
                    .constraints
                    .iter()
                    .any(|c| c.reference && c.rule.parameter() == Some(name.as_str()));
                if expression.is_some() && measured {
                    return Err(refuse(
                        codes::PARAMETER_MEASURED,
                        format!(
                            "“{name}” yalnız ölçen bir ölçünün parametresi; önce ölçüyü sürücü yapın (cad.constraints.edit mode)."
                        ),
                        at("expression"),
                    ));
                }
                let p = find(&mut set, name, at("name"))?;
                if let Some(text) = expression {
                    p.expression.clone_from(text);
                }
                p.min = bound(*min, p.min);
                p.max = bound(*max, p.max);
                if let Some(words) = description {
                    p.description.clone_from(words);
                }
            }
            ParameterEdit::Rename { name, to } => {
                find(&mut set, name, at("name"))?;
                if set.parameter(to).is_some() {
                    return Err(refuse(
                        codes::PARAMETER_EXISTS,
                        format!("“{to}” adında bir parametre zaten var; başka bir ad verin."),
                        at("to"),
                    ));
                }
                for p in &mut set.parameters {
                    if p.name == *name {
                        p.name.clone_from(to);
                    }
                    p.expression = rename_in_expression(&p.expression, name, to);
                }
                for c in &mut set.constraints {
                    if let ConstraintRule::Distance { parameter, .. }
                    | ConstraintRule::Angle { parameter, .. }
                    | ConstraintRule::Radius { parameter, .. }
                    | ConstraintRule::Diameter { parameter, .. } = &mut c.rule
                        && parameter == name
                    {
                        parameter.clone_from(to);
                    }
                }
            }
            ParameterEdit::Remove { name } => {
                find(&mut set, name, at("name"))?;
                let owned = set
                    .constraints
                    .iter()
                    .any(|c| c.rule.parameter() == Some(name.as_str()));
                if owned || read_names(&set).contains(name) {
                    return Err(refuse(
                        codes::PARAMETER_READ,
                        if owned {
                            format!(
                                "“{name}” bir ölçü kısıtının parametresi; ölçüyü kaldırın (cad.constraints.edit remove)."
                            )
                        } else {
                            format!(
                                "“{name}” parametresini başka bir parametre okuyor; önce onun ifadesini değiştirin."
                            )
                        },
                        at("name"),
                    ));
                }
                set.parameters.retain(|p| p.name != *name);
            }
        }
    }
    let solved = settle(doc, &set, &[], &[], input.allow_flip, stop)?;
    Ok((set, solved))
}
