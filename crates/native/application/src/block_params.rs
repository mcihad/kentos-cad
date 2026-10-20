//! A dynamic block's parameter values (docs/adr/0318 §3): what an insert keeps, how values given are resolved, and
//! `cad.blocks.parameters` v1 (get, set, reset as one undo step “Blok parametreleri”).
//!
//! An insert keeps only the values that differ from the defaults. Setting resolves in this order:
//! - each name given: the block's parameter (`unknown_parameter`), not an expression's (`derived_parameter`); a
//!   lookup's value is one of its rows' labels and sets the row's values; any other value meets its parameter's rule
//!   (`invalid_parameter_value`);
//! - then every expression's parameter is computed from the number parameters' values (as `@name`), passes over the
//!   list as many times as there are such parameters; its result meets its rule (`invalid_parameter_value`);
//! - the values equal to their defaults are dropped.
//!
//! The checks of the command, in order: an operation's input (`no_entities`, `invalid_uid`, an empty `values` is
//! `invalid_parameter_value`); the expected revision; the inserts (`entity_not_found`, `not_insert`, `not_dynamic`);
//! the names and values against each insert's block; those on a locked layer left out (every one: `layer_locked`).

use std::collections::BTreeMap;

use kentos_commands::block_dynamic::{number_problem, same_param_value, value_problem};
use kentos_commands::{
    BlockDefinition, BlockParameter, BlocksParameters, BlocksParametersOut, CommandResult,
    CommandWarning, DynamicBlock, Entity, InsertParameters, ParamValue, ParametersOperation,
};
use kentos_domain::{Document, Slot};
use kentos_expression::host::{Schema, Variable};
use kentos_expression::{Scope, Value, compile_with};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

/// The undo step's name.
pub const STEP: &str = "Blok parametreleri";

/// A refusal's code, its path's tail and its words.
pub type Refusal = (&'static str, String, String);

fn refusal(code: &'static str, path: impl Into<String>, words: String) -> Refusal {
    (code, path.into(), words)
}

/// The number parameters' values as `@` names, the defaults where `values` has none.
fn variables(d: &DynamicBlock, values: &BTreeMap<String, ParamValue>) -> Vec<Variable> {
    d.parameters
        .iter()
        .filter_map(|p| {
            let n = match (
                p,
                values.get(p.name()).cloned().or_else(|| p.default_value()),
            ) {
                (
                    BlockParameter::Linear { .. } | BlockParameter::Angle { .. },
                    Some(ParamValue::Number(x)),
                ) => x,
                _ => return None,
            };
            Some(Variable {
                name: p.name().to_owned(),
                value: Value::Num(n),
                description: String::new(),
            })
        })
        .collect()
}

/// An expression's value with these variables; why not, when it gives no number or names an unknown one.
fn computed(source: &str, vars: Vec<Variable>) -> Result<f64, String> {
    struct Nothing;
    impl Scope for Nothing {
        fn field(&self, _: usize) -> Option<&str> {
            None
        }
        fn measured(&self) -> kentos_expression::Measured {
            kentos_expression::Measured::default()
        }
        fn vertices(&self) -> Option<f64> {
            None
        }
        fn kind(&self) -> &str {
            ""
        }
        fn layer(&self) -> &str {
            ""
        }
        fn label(&self) -> Option<&str> {
            None
        }
        fn index(&self) -> f64 {
            1.0
        }
        fn id(&self) -> f64 {
            0.0
        }
        fn scale(&self) -> Option<f64> {
            None
        }
    }
    let schema = Schema {
        variables: vars,
        ..Schema::default()
    };
    let e = compile_with(source, &schema).map_err(|e| format!("ifade okunamadı: {}", e.message))?;
    if let Some(name) = e.unknown_variables.first() {
        return Err(format!("ifadedeki “@{name}” bir sayı parametresi değil"));
    }
    if !e.fields.is_empty() {
        return Err("ifade yalnız parametreleri (@ad) okur".into());
    }
    match e.evaluate(&Nothing) {
        Value::Num(x) if x.is_finite() => Ok(x),
        _ => Err("ifade bir sayı vermiyor".into()),
    }
}

/// The expression parameters computed over `values` (passes as many as they are); why not, with the parameter.
fn derive(
    d: &DynamicBlock,
    values: &mut BTreeMap<String, ParamValue>,
) -> Result<(), (String, String)> {
    let derived: Vec<(&str, &str, &kentos_commands::NumberRule)> = d
        .parameters
        .iter()
        .filter_map(|p| {
            let rule = p.rule()?;
            Some((p.name(), rule.expression.as_deref()?, rule))
        })
        .collect();
    for _ in 0..derived.len() {
        for (name, source, rule) in &derived {
            let x = computed(source, variables(d, values)).map_err(|m| (name.to_string(), m))?;
            if let Some(m) = number_problem(rule, x) {
                return Err((name.to_string(), format!("ifadenin sonucu {x}: {m}")));
            }
            values.insert(name.to_string(), ParamValue::Number(x));
        }
    }
    Ok(())
}

/// Why a definition's expressions do not work (docs/adr/0318 §1): each computes a number from the defaults.
pub fn expressions_problem(d: &DynamicBlock) -> Option<(String, String)> {
    let mut values = BTreeMap::new();
    derive(d, &mut values).err().map(|(name, m)| {
        let i = d
            .parameters
            .iter()
            .position(|p| p.name() == name)
            .unwrap_or(0);
        (
            format!("dynamic.parameters[{i}].rule.expression"),
            format!("“{name}” parametresinin ifadesi: {m}"),
        )
    })
}

/// The values an insert keeps after `given` is set on `kept` (the order of the module's rules).
pub fn resolve(
    d: &DynamicBlock,
    kept: &BTreeMap<String, ParamValue>,
    given: &BTreeMap<String, ParamValue>,
) -> Result<BTreeMap<String, ParamValue>, Refusal> {
    let mut values = kept.clone();
    for (name, v) in given {
        let path = format!("values.{name}");
        let Some(p) = d.parameters.iter().find(|p| p.name() == name) else {
            let names: Vec<&str> = d.parameters.iter().map(BlockParameter::name).collect();
            return Err(refusal(
                codes::UNKNOWN_PARAMETER,
                path,
                format!(
                    "Blokta “{name}” parametresi yok. Parametreler: {}.",
                    names.join(", ")
                ),
            ));
        };
        if p.rule().is_some_and(|r| r.expression.is_some()) {
            return Err(refusal(
                codes::DERIVED_PARAMETER,
                path,
                format!(
                    "“{name}” ifadesiyle hesaplanır; değeri verilmez. Hesaplandığı parametreleri değiştirin."
                ),
            ));
        }
        if let Some(m) = value_problem(p, v) {
            return Err(refusal(
                codes::INVALID_PARAMETER_VALUE,
                path,
                format!("“{name}” parametresinin değeri: {m}."),
            ));
        }
        match (p, v) {
            (BlockParameter::Lookup { rows, .. }, ParamValue::Text(label)) => {
                if let Some(row) = rows.iter().find(|r| r.label == *label) {
                    for (k, x) in &row.values {
                        values.insert(k.clone(), x.clone());
                    }
                }
            }
            _ => {
                values.insert(name.clone(), v.clone());
            }
        }
    }
    derive(d, &mut values).map_err(|(name, m)| {
        refusal(
            codes::INVALID_PARAMETER_VALUE,
            format!("values.{name}"),
            format!("“{name}”: {m}."),
        )
    })?;
    values.retain(|name, v| {
        d.parameters
            .iter()
            .find(|p| p.name() == name)
            .and_then(BlockParameter::default_value)
            .is_none_or(|default| !same_param_value(&default, v))
    });
    Ok(values)
}

/// Every parameter's value as an insert shows it: its kept value, else its default; a lookup's the label of the
/// first row whose values all hold.
pub fn current(
    d: &DynamicBlock,
    kept: &BTreeMap<String, ParamValue>,
) -> BTreeMap<String, ParamValue> {
    let mut out = BTreeMap::new();
    for p in &d.parameters {
        if let Some(v) = kept.get(p.name()).cloned().or_else(|| p.default_value()) {
            out.insert(p.name().to_owned(), v);
        }
    }
    for p in &d.parameters {
        if let BlockParameter::Lookup { name, rows } = p
            && let Some(row) = rows.iter().find(|r| {
                r.values
                    .iter()
                    .all(|(k, v)| out.get(k).is_some_and(|x| same_param_value(x, v)))
            })
        {
            out.insert(name.clone(), ParamValue::Text(row.label.clone()));
        }
    }
    out
}

/// The kept values that still fit a block after its dynamic part changed (docs/adr/0318 §1): those of parameters
/// it still has, that still meet their rules, expressions computed again, defaults dropped; none for a plain block.
pub fn refit(
    block: &BlockDefinition,
    kept: &BTreeMap<String, ParamValue>,
) -> BTreeMap<String, ParamValue> {
    let Some(d) = &block.dynamic else {
        return BTreeMap::new();
    };
    let fits: BTreeMap<String, ParamValue> = kept
        .iter()
        .filter(|(name, v)| {
            d.parameters
                .iter()
                .find(|p| p.name() == *name)
                .is_some_and(|p| {
                    !matches!(p, BlockParameter::Lookup { .. })
                        && p.rule().is_none_or(|r| r.expression.is_none())
                        && value_problem(p, v).is_none()
                })
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    resolve(d, &fits, &BTreeMap::new()).unwrap_or_default()
}

/// The drawing's inserts of `next` with their values refitted (docs/adr/0318 §1): the changes, how many values were
/// dropped, the inserts on a locked layer left as they are.
pub(crate) fn refit_inserts(doc: &Document, next: &BlockDefinition) -> crate::blocks_sync::Synced {
    let mut changes = Vec::new();
    let mut dropped = 0u32;
    let mut locked = 0;
    let layers = doc.layers();
    for e in doc.entities() {
        let Entity::Insert(i) = e else { continue };
        if i.block != next.id || i.parameters.is_empty() {
            continue;
        }
        let values = refit(next, &i.parameters);
        if values == i.parameters {
            continue;
        }
        if layers.is_locked(&i.base.layer_id) {
            locked += 1;
            continue;
        }
        dropped += u32::try_from(
            i.parameters
                .keys()
                .filter(|k| !values.contains_key(*k))
                .count(),
        )
        .unwrap_or(0);
        let mut i = i.clone();
        i.parameters = values;
        changes.push((Slot(i.base.id), Entity::Insert(i)));
    }
    crate::blocks_sync::Synced {
        report: kentos_commands::AttributeSync {
            inserts: u32::try_from(changes.len()).unwrap_or(u32::MAX),
            parameters: dropped,
            ..Default::default()
        },
        changes,
        locked,
    }
}

struct Checked {
    out: BlocksParametersOut,
    changes: Vec<(Slot, Entity)>,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &BlocksParameters) -> Result<Checked, Stop> {
    checks::uids(
        &input.uids,
        "Parametreleri okunacak ya da değişecek yerleştirme verilmedi.",
    )?;
    let given = input.values.clone().unwrap_or_default();
    if input.operation == ParametersOperation::Set && given.is_empty() {
        return Err(Stop::failed(error(
            codes::INVALID_PARAMETER_VALUE,
            "Değer verilmedi. values ile parametrelerin adlarını ve değerlerini verin.".into(),
            Some("values".into()),
        )));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    for (i, _, e, _) in &named {
        let Entity::Insert(ins) = e else {
            return Err(Stop::failed(error(
                codes::NOT_INSERT,
                "Verilen nesne bir blok yerleştirmesi değil. Dinamik bir bloğun yerleştirmesini verin.".into(),
                Some(format!("uids[{i}]")),
            )));
        };
        if doc.block(ins.block).is_none_or(|b| b.dynamic.is_none()) {
            let name = doc
                .block(ins.block)
                .map_or("", |b| b.name.as_str())
                .to_owned();
            return Err(Stop::failed(error(
                codes::NOT_DYNAMIC,
                format!(
                    "“{name}” bloğunun parametresi yok; dinamik değil. Dinamik bir bloğun yerleştirmesini verin."
                ),
                Some(format!("uids[{i}]")),
            )));
        }
    }
    // The names and values against every insert's block first: nothing is written when one does not take them.
    let mut next: Vec<(
        Slot,
        &kentos_commands::InsertEntity,
        BTreeMap<String, ParamValue>,
    )> = Vec::new();
    for (_, slot, e, _) in &named {
        let Entity::Insert(ins) = e else { continue };
        let Some(d) = doc.block(ins.block).and_then(|b| b.dynamic.as_ref()) else {
            continue;
        };
        let values = match input.operation {
            ParametersOperation::Get => ins.parameters.clone(),
            ParametersOperation::Set => resolve(d, &ins.parameters, &given)
                .map_err(|(code, path, words)| Stop::failed(error(code, words, Some(path))))?,
            ParametersOperation::Reset => {
                let mut kept = ins.parameters.clone();
                match &input.names {
                    None => kept.clear(),
                    Some(names) => {
                        for (k, name) in names.iter().enumerate() {
                            if !d.parameters.iter().any(|p| p.name() == name) {
                                return Err(Stop::failed(error(
                                    codes::UNKNOWN_PARAMETER,
                                    format!("Blokta “{name}” parametresi yok."),
                                    Some(format!("names[{k}]")),
                                )));
                            }
                            // A lookup's reset is its rows' parameters'.
                            match d.parameters.iter().find(|p| p.name() == name) {
                                Some(BlockParameter::Lookup { rows, .. }) => {
                                    for row in rows {
                                        for key in row.values.keys() {
                                            kept.remove(key);
                                        }
                                    }
                                }
                                _ => {
                                    kept.remove(name);
                                }
                            }
                        }
                    }
                }
                resolve(d, &kept, &BTreeMap::new())
                    .map_err(|(code, path, words)| Stop::failed(error(code, words, Some(path))))?
            }
        };
        next.push((*slot, ins, values));
    }
    let mut warnings = Vec::new();
    let mut open: Vec<Slot> = next.iter().map(|(s, ..)| *s).collect();
    if input.operation != ParametersOperation::Get {
        let changing: Vec<checks::Named<'_>> = named
            .iter()
            .filter(|(_, slot, ..)| {
                next.iter()
                    .any(|(s, ins, v)| s == slot && *v != ins.parameters)
            })
            .cloned()
            .collect();
        if !changing.is_empty() {
            let (left, warning) = checks::skip_locked(doc, changing)?;
            warnings.extend(warning);
            let locked: Vec<Slot> = named
                .iter()
                .filter(|(_, s, ..)| !left.iter().any(|(_, l, ..)| l == s))
                .filter(|(_, s, ..)| {
                    next.iter()
                        .any(|(n, ins, v)| n == s && *v != ins.parameters)
                })
                .map(|(_, s, ..)| *s)
                .collect();
            open.retain(|s| !locked.contains(s));
        }
    }
    let mut out = BlocksParametersOut {
        inserts: Vec::with_capacity(next.len()),
        changed: Vec::new(),
        revision: doc.revision().to_string(),
    };
    let mut changes = Vec::new();
    for (slot, ins, values) in next {
        let uid = doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
        let writes = open.contains(&slot) && values != ins.parameters;
        let shown = if writes || input.operation == ParametersOperation::Get {
            values.clone()
        } else {
            ins.parameters.clone()
        };
        if let Some(d) = doc.block(ins.block).and_then(|b| b.dynamic.as_ref()) {
            out.inserts.push(InsertParameters {
                uid: uid.clone(),
                block: ins.block,
                values: current(d, &shown),
            });
        }
        if writes {
            out.changed.push(uid);
            let mut i = ins.clone();
            i.parameters = values;
            changes.push((slot, Entity::Insert(i)));
        }
    }
    Ok(Checked {
        out,
        changes,
        warnings,
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &BlocksParameters) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: (),
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would answer and write, writing nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &BlocksParameters,
) -> CommandResult<BlocksParametersOut> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed {
            output: c.out,
            warnings: c.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Reads, sets or resets the values; a write is one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: BlocksParameters,
) -> CommandResult<BlocksParametersOut> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_BLOCKS_PARAMETERS,
        kentos_commands::CAD_BLOCKS_PARAMETERS_VERSION,
        input,
        |cx, input| {
            let c = match check(cx.doc, &input) {
                Ok(c) => c,
                Err(stop) => return stop.into(),
            };
            let mut out = c.out;
            if !c.changes.is_empty() {
                cx.doc.update_many(c.changes, STEP);
            }
            out.revision = cx.doc.revision().to_string();
            CommandResult::Completed {
                output: out,
                warnings: c.warnings,
            }
        },
    )
}
