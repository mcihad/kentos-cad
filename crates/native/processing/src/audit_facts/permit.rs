//! The drawing's permit facts for the audit facts (`permit`, docs/adr/8063): the dated scope decision's particulars as
//! the drawing's variables hold them (`kentos_digital_project::scope::inputs`) and what the drawing measures for it —
//! the building's yapı inşaat alanı (`Document::construction_area`, PAİY 4/oooo) and its storeys above and below
//! ±0.00. Their order, trimming and rounding are `kentos_extensions::audit_facts::permit`'s.
//!
//! - **Only where it was asked:** a drawing none of whose particulars has a value gives no section ([`measure`]).
//! - **A window's own values:** the permit set measures with what its form holds before it is written ([`facts`]).

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{Entity, VariableValue, variable_key};
use kentos_digital_project::scope;
use kentos_domain::Document;
use kentos_extensions::audit_facts::permit::{PermitFacts, StoreyCount};

/// The drawing's permit facts, its variables giving the particulars; none when none of them has a value.
pub fn measure(doc: &Document) -> Option<PermitFacts> {
    let values = particulars(doc);
    (!values.is_empty()).then(|| facts(doc, values))
}

/// The scope decision's particulars the drawing's variables give a value, by the inputs' names (a variable is found
/// as `@ad` finds it: Turkish letters and case aside).
pub fn particulars(doc: &Document) -> BTreeMap<String, String> {
    let vars = &doc.settings().variables;
    scope::inputs()
        .iter()
        .filter_map(|input| {
            let key = variable_key(&input.name);
            let v = vars.iter().find(|v| variable_key(&v.name) == key)?;
            let text = match &v.value {
                VariableValue::Text(t) => t.trim().to_owned(),
                VariableValue::Number(n) if n.is_finite() => n.to_string(),
                VariableValue::Bool(true) => "evet".to_owned(),
                VariableValue::Bool(false) => "hayır".to_owned(),
                VariableValue::Number(_) | VariableValue::Null => return None,
            };
            (!text.is_empty()).then(|| (input.name.clone(), text))
        })
        .collect()
}

/// The facts of `particulars` (by the inputs' names) with the drawing's measures.
pub fn facts(doc: &Document, particulars: BTreeMap<String, String>) -> PermitFacts {
    PermitFacts {
        particulars,
        construction_area: doc.construction_area(),
        storeys: storeys(doc),
    }
    .normalized()
}

/// The project's storeys holding a room, by their floor's elevation to the millimetre: at or above ±0.00, below it.
fn storeys(doc: &Document) -> Option<StoreyCount> {
    let levels = &doc.settings().arch.as_ref()?.levels;
    let held: BTreeSet<&str> = doc
        .entities()
        .filter(|e| matches!(e, Entity::Space(_)))
        .filter_map(Entity::own_level)
        .collect();
    let mut count = StoreyCount::default();
    let mut any = false;
    for level in levels.iter().filter(|l| held.contains(l.id.as_str())) {
        any = true;
        if (level.elevation * 1e3).round() >= 0.0 {
            count.above += 1;
        } else {
            count.below += 1;
        }
    }
    any.then_some(count)
}
