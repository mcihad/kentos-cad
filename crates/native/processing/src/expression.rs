//! Expressions over objects (the web's `model/expression/expression.ts`):
//! the style core's language (`kentos_style_core::expr`), evaluated for each
//! object with what it reads of it, and the dialog's one-line preview.

use kentos_contracts::Entity;
use kentos_geometry_core::store::draw::measure_record;
use kentos_native_application::geometry::shape;
/// The language's variables and functions with their help: the expression
/// field's Değişkenler and İşlevler menus (the web's `exprCatalog`).
pub use kentos_style_core::expr::library::{FUNCTIONS, FuncDef, VARIABLES, VarDef};
use kentos_style_core::expr::rows::{As, MEASURE_STRIDE};
use kentos_style_core::expr::{Expr, Measured, Scope, Value};

use crate::features::kind_label;
use crate::types::Returns;

/// Corners of a path or area (holes included), for `$köşe`.
pub fn vertex_count(e: &Entity) -> Option<f64> {
    let n = match e {
        Entity::Polyline(p) => p.pts.len(),
        Entity::Polygon(p) => {
            p.pts.len() + p.holes.iter().flatten().map(|h| h.pts.len()).sum::<usize>()
        }
        Entity::Line(_) => 2,
        Entity::Spline(s) => s.pts.len(),
        Entity::Point(_) => 1,
        _ => return None,
    };
    Some(n as f64)
}

/// The geometry values of objects without a store: the core's, one object
/// at a time (the web's `measuresOf`).
pub fn measures_of(list: &[&Entity]) -> Vec<f64> {
    let mut out = Vec::with_capacity(list.len() * MEASURE_STRIDE);
    for e in list {
        measure_record(Some(&shape(e)), &mut out);
    }
    out
}

/// One object as an expression reads it.
struct Row<'a> {
    e: &'a Entity,
    fields: &'a [String],
    layer: &'a str,
    index: usize,
    measures: Option<&'a [f64]>,
}

impl Scope for Row<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        let name = self.fields.get(i)?;
        self.e.base().attrs.get(name).map(String::as_str)
    }

    fn measured(&self) -> Measured {
        let k = self.index * MEASURE_STRIDE;
        let Some(m) = self.measures.and_then(|m| m.get(k..k + MEASURE_STRIDE)) else {
            return Measured::default();
        };
        let flags = m[0] as u32;
        Measured {
            length: (flags & 1 != 0).then_some(m[1]),
            area: (flags & 2 != 0).then_some(m[2]),
            anchor: (flags & 4 != 0).then_some((m[3], m[4])),
        }
    }

    fn vertices(&self) -> Option<f64> {
        vertex_count(self.e)
    }

    fn kind(&self) -> &str {
        kind_label(self.e.kind())
    }

    fn layer(&self) -> &str {
        self.layer
    }

    fn label(&self) -> Option<&str> {
        self.e.base().label.as_deref()
    }

    fn index(&self) -> f64 {
        (self.index + 1) as f64
    }

    fn id(&self) -> f64 {
        f64::from(self.e.base().id)
    }

    fn scale(&self) -> Option<f64> {
        None
    }
}

/// An expression's value for each object, as `want` asks: `measures` are
/// the geometry store's values of these objects (six numbers each), asked
/// only when the expression reads `$alan`, `$uzunluk`, `$y` or `$x`.
pub fn evaluate_all(
    expr: &Expr,
    list: &[&Entity],
    layer_name: &dyn Fn(&str) -> String,
    measures: &mut dyn FnMut() -> Vec<f64>,
    want: As,
) -> Vec<Value<'static>> {
    let measured = expr.needs.measured.then(measures);
    list.iter()
        .enumerate()
        .map(|(index, e)| {
            let layer = if expr.needs.layer {
                layer_name(&e.base().layer_id)
            } else {
                String::new()
            };
            let row = Row {
                e,
                fields: &expr.fields,
                layer: &layer,
                index,
                measures: measured.as_deref(),
            };
            want.convert(expr.evaluate(&row)).into_owned()
        })
        .collect()
}

/// The dialog's line for an expression on the objects it will read:
/// "12 / 68 nesne koşulu sağlıyor." or the first value (the web's `previewExpression`).
pub fn preview_expression(
    expr: &Expr,
    entities: &[&Entity],
    returns: Returns,
    layer_name: &dyn Fn(&str) -> String,
    measures: &mut dyn FnMut(&[&Entity]) -> Vec<f64>,
) -> String {
    let Some(first_object) = entities.first() else {
        return "Önizleme için uygun nesne yok.".into();
    };
    let missing: Vec<String> = expr
        .fields
        .iter()
        .filter(|f| !entities.iter().any(|e| e.base().attrs.contains_key(*f)))
        .map(|f| format!("“{f}”"))
        .collect();
    let note = if missing.is_empty() {
        String::new()
    } else {
        format!(" {} alanı bu nesnelerde yok.", missing.join(", "))
    };
    let list = &entities[..entities.len().min(20_000)];
    let mut ask = || measures(list);
    if returns == Returns::Condition {
        let values = evaluate_all(expr, list, layer_name, &mut ask, As::Bool);
        let hits = values.iter().filter(|v| **v == Value::Bool(true)).count();
        return format!("{hits} / {} nesne koşulu sağlıyor.{note}", list.len());
    }
    let values = evaluate_all(expr, list, layer_name, &mut ask, As::Text);
    let empty = values.iter().filter(|v| **v == Value::Null).count();
    let who = first_object
        .base()
        .label
        .as_deref()
        .filter(|l| !l.is_empty())
        .map(|l| format!(" ({l})"))
        .unwrap_or_default();
    let first = match values.first() {
        Some(Value::Text(t)) => format!("“{t}”"),
        _ => "boş".into(),
    };
    let empties = if empty > 0 {
        format!(" {empty} nesnede sonuç boş.")
    } else {
        String::new()
    };
    format!("İlk nesnede{who}: {first}.{empties}{note}")
}
