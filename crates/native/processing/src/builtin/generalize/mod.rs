//! Genelleştirme (docs/adr/0221; the web's `builtin/generalize/`): smoothing,
//! squaring, building simplification, aggregation, part removal,
//! elimination and coverage simplification. The geometry is the core's
//! (`ops::generalize`); what the tools share is here.

pub mod aggregate;
pub mod building;
pub mod coverage;
pub mod eliminate;
pub mod parts;
pub mod smooth;
pub mod square;

use kentos_contracts::Entity;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::convert::Zs;
use kentos_native_application::geometry::shape;
use serde_json::{Value, json};

use super::convert::zs_of;
use crate::types::{ChangeSet, ParamDef, ParamKind, RunResult, Values};

/// The objects as the core takes them, and their elevations.
pub struct Inputs {
    pub shapes: Vec<Shape>,
    pub zs: Vec<Option<Zs>>,
}

impl Inputs {
    pub fn of(list: &[&Entity]) -> Inputs {
        Inputs {
            shapes: list.iter().map(|e| shape(e)).collect(),
            zs: list.iter().map(|e| zs_of(e)).collect(),
        }
    }

    pub fn refs(&self) -> Vec<&Shape> {
        self.shapes.iter().collect()
    }

    pub fn zs_refs(&self) -> Vec<Option<&Zs>> {
        self.zs.iter().map(Option::as_ref).collect()
    }
}

/// An angle parameter in degrees.
pub fn degrees_param(
    name: &str,
    label: &str,
    max: f64,
    fallback: f64,
    description: &str,
) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(0.0),
            max: Some(max),
            integer: false,
            unit: "°".into(),
            placeholder: None,
        },
    )
    .default_value(json!(fallback))
    .describe(description)
}

/// A measure parameter: `unit` m or m², at least `min`.
pub fn measure_param(
    name: &str,
    label: &str,
    unit: &str,
    min: f64,
    fallback: f64,
    description: &str,
) -> ParamDef {
    ParamDef::new(
        name,
        label,
        ParamKind::Number {
            min: Some(min),
            max: None,
            integer: false,
            unit: unit.into(),
            placeholder: None,
        },
    )
    .default_value(json!(fallback))
    .describe(description)
}

/// A run that wrote these objects, with its outputs (`count` and, when it has rows, a report).
pub fn finish(add: Vec<Entity>, summary: String, count: usize, report: Option<Value>) -> RunResult {
    let mut outputs: Values = [("count".to_owned(), json!(count))].into_iter().collect();
    if let Some(r) = report {
        outputs.insert("report".to_owned(), r);
    }
    RunResult {
        changes: Some(ChangeSet {
            add,
            ..ChangeSet::default()
        }),
        outputs,
        summary: Some(summary),
        ..RunResult::default()
    }
}

/// A report table: its columns and rows.
pub fn table(columns: &[&str], rows: Vec<Vec<String>>) -> Option<Value> {
    (!rows.is_empty()).then(|| json!({ "columns": columns, "rows": rows }))
}
