//! A layer's record rules asked of a record (Kayıt kuralları, docs/adr/0438 §1): each compiled once
//! over the layer's fields, then asked of a write that changes a field it reads (every rule of a new
//! record). A rule breaks when its answer is a false one; an empty answer (a value it reads is empty)
//! holds, as an SQL CHECK does.

use kentos_contracts::{LayerField, LayerRule};
use kentos_expression::{Expr, Unit, Variable, compile_with};

use crate::record::{Attrs, Object, evaluate, schema_in};

/// The rules of a layer, compiled.
#[derive(Clone, Debug, Default)]
pub struct LayerRules {
    rules: Vec<(LayerRule, Expr)>,
}

/// A rule a record breaks, and the field it is shown on.
#[derive(Clone, Copy, Debug)]
pub struct Broken<'a> {
    pub rule: &'a LayerRule,
    pub field: Option<&'a LayerField>,
}

impl LayerRules {
    /// The rules compiled over `fields`; the first that does not compile, else: its place and why.
    pub fn new(
        rules: &[LayerRule],
        fields: &[LayerField],
        variables: &[Variable],
        unit: Unit,
    ) -> Result<LayerRules, (usize, String)> {
        let schema = schema_in(fields, variables, unit);
        let mut out = Vec::with_capacity(rules.len());
        for (i, r) in rules.iter().enumerate() {
            match compile_with(&r.rule, &schema) {
                Ok(e) => {
                    if let Some(name) = e
                        .fields
                        .iter()
                        .find(|n| !fields.iter().any(|f| &f.name == *n))
                    {
                        return Err((
                            i,
                            format!(
                                "“{}” kuralı katmanda olmayan “{name}” alanını okuyor.",
                                r.name
                            ),
                        ));
                    }
                    out.push((r.clone(), e));
                }
                Err(e) => {
                    return Err((i, format!("“{}” kuralı okunamadı: {}", r.name, e.text())));
                }
            }
        }
        Ok(LayerRules { rules: out })
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// The rules the record `after` breaks among those the write from `before` asks (none: a new
    /// record, every rule): a rule is asked when a field it reads changed. In the rules' order.
    pub fn broken<'a>(
        &'a self,
        fields: &'a [LayerField],
        before: Option<&Attrs>,
        after: &Attrs,
        object: &Object,
    ) -> Vec<Broken<'a>> {
        self.rules
            .iter()
            .filter(|(_, e)| {
                before.is_none_or(|b| e.fields.iter().any(|n| b.get(n) != after.get(n)))
            })
            .filter(|(_, e)| breaks(e, after, object))
            .map(|(r, e)| Broken {
                rule: r,
                field: shown_on(r, e, fields),
            })
            .collect()
    }
}

/// Whether `expr`'s answer for the record is a false one (an empty one holds).
fn breaks(expr: &Expr, attrs: &Attrs, object: &Object) -> bool {
    let v = evaluate(expr, attrs, object);
    !kentos_expression::value::is_empty(&v) && !kentos_expression::value::truthy(&v)
}

/// The field a rule is shown on: its first named field, else the first field it reads.
fn shown_on<'a>(rule: &LayerRule, expr: &Expr, fields: &'a [LayerField]) -> Option<&'a LayerField> {
    rule.fields
        .first()
        .and_then(|k| fields.iter().find(|f| f.key() == k))
        .or_else(|| {
            expr.fields
                .iter()
                .find_map(|n| fields.iter().find(|f| &f.name == n))
        })
}
