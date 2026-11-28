//! A layer's field rules through a GeoPackage (gis-usage-report GIS-009, docs/adr/1075): written as KentOS's own
//! document of the table (`https://kentos.app/field-rules/1`: every field as the layer holds it, read back whole) and,
//! for other programs, as the schema extension (a field's shown name, its range or its values) beside a `NOT NULL`
//! column and a unique index where the data keeps them. Before writing, what the objects break of the rules is counted
//! by field and said with the first rows, so a field team's copy and a checked delivery are told apart.

use std::collections::{BTreeMap, HashMap};

use kentos_commands::{FieldChoice, LayerField, check_value};
use kentos_domain::Document;
use kentos_geopackage::GeoPackage;
use kentos_geopackage::schema::{DataColumn, Rule};

/// KentOS's document of a table's field rules: `{"fields": {"column": field}}`.
pub const RULES_STANDARD: &str = "https://kentos.app/field-rules/1";

/// Whether a field carries a rule a column's type does not say.
fn ruled(f: &LayerField) -> bool {
    f.alias.is_some()
        || f.min.is_some()
        || f.max.is_some()
        || f.required
        || f.unique
        || f.values.as_ref().is_some_and(|v| !v.is_empty())
        || f.default.is_some()
        || f.unit.is_some()
        || f.domain.is_some()
}

/// The rules document of a layer's fields written to `columns` (field key → column), none when no field has a rule.
pub fn rules_document(fields: &[LayerField], columns: &[(String, String)]) -> Option<String> {
    let map: serde_json::Map<String, serde_json::Value> = fields
        .iter()
        .filter(|f| ruled(f))
        .filter_map(|f| {
            let column = columns.iter().find(|(k, _)| *k == f.name)?;
            Some((column.1.clone(), serde_json::to_value(f).ok()?))
        })
        .collect();
    (!map.is_empty()).then(|| serde_json::json!({ "fields": map }).to_string())
}

/// The schema extension's rows of a layer's fields written to `table`'s `columns`: each shown name, and each number's
/// range or list's values as a rule named after the table and the column.
pub fn schema_rows(
    table: &str,
    fields: &[LayerField],
    columns: &[(String, String)],
) -> (Vec<DataColumn>, Vec<(String, Rule)>) {
    let mut out = Vec::new();
    let mut rules = Vec::new();
    for f in fields {
        let Some((_, column)) = columns.iter().find(|(k, _)| *k == f.name) else {
            continue;
        };
        let rule = if let Some(values) = f.values.as_ref().filter(|v| !v.is_empty()) {
            Some(Rule::Values(
                values
                    .iter()
                    .map(|c| (c.code.clone(), Some(c.label.clone())))
                    .collect(),
            ))
        } else if f.kind.is_number() && (f.min.is_some() || f.max.is_some()) {
            let number = |t: &Option<String>| t.as_deref().and_then(|t| t.parse::<f64>().ok());
            Some(Rule::Range {
                min: number(&f.min),
                min_included: !f.min_exclusive,
                max: number(&f.max),
                max_included: !f.max_exclusive,
            })
        } else {
            None
        };
        if f.alias.is_none() && rule.is_none() {
            continue;
        }
        let name = rule.as_ref().map(|_| format!("{table}_{column}"));
        if let (Some(name), Some(rule)) = (&name, rule) {
            rules.push((name.clone(), rule));
        }
        out.push(DataColumn {
            table: table.to_owned(),
            column: column.clone(),
            name: f.alias.clone(),
            description: None,
            constraint: name,
        });
    }
    (out, rules)
}

/// The fields KentOS's document gives a table's columns, by column; none without one.
pub fn rules_of(gp: &GeoPackage, table: &str) -> Result<BTreeMap<String, LayerField>, String> {
    let mut out = BTreeMap::new();
    for (standard, text) in gp.metadata(table).map_err(|e| e.0)? {
        if standard != RULES_STANDARD {
            continue;
        }
        let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        for (column, f) in doc["fields"].as_object().into_iter().flatten() {
            if let Ok(field) = serde_json::from_value::<LayerField>(f.clone()) {
                out.insert(column.clone(), field);
            }
        }
    }
    Ok(out)
}

/// A table's fields given their rules: KentOS's document's whole when it has one (the column's name kept, the
/// objects' values are under it); else the schema extension's shown name, range and values.
pub fn take_rules(gp: &GeoPackage, table: &str, fields: &mut [LayerField]) -> Result<(), String> {
    let own = rules_of(gp, table)?;
    if !own.is_empty() {
        for f in fields.iter_mut() {
            if let Some(rule) = own.get(&f.name) {
                *f = LayerField {
                    name: f.name.clone(),
                    ..rule.clone()
                };
            }
        }
        return Ok(());
    }
    let schema: HashMap<String, (DataColumn, Option<Rule>)> = gp
        .data_columns(table)
        .map_err(|e| e.0)?
        .into_iter()
        .map(|(c, r)| (c.column.clone(), (c, r)))
        .collect();
    // A file's UNIQUE constraint is the layer's unique index (docs/adr/0758 §5) and a bound table's constraint
    // (docs/adr/0755), not the field's Tekil: only KentOS's own document says that.
    for f in fields.iter_mut() {
        let Some((c, rule)) = schema.get(&f.name) else {
            continue;
        };
        if f.alias.is_none() {
            f.alias = c
                .name
                .clone()
                .filter(|n| !n.trim().is_empty() && *n != f.name);
        }
        match rule {
            Some(Rule::Range {
                min,
                min_included,
                max,
                max_included,
            }) if f.kind.is_number() => {
                let text = |v: f64| format!("{v}");
                if f.min.is_none() {
                    f.min = min.map(text);
                    f.min_exclusive = min.is_some() && !min_included;
                }
                if f.max.is_none() {
                    f.max = max.map(text);
                    f.max_exclusive = max.is_some() && !max_included;
                }
            }
            Some(Rule::Values(values)) if f.values.is_none() => {
                f.values = Some(
                    values
                        .iter()
                        .map(|(code, label)| {
                            FieldChoice::new(
                                code.clone(),
                                label.clone().unwrap_or_else(|| code.clone()),
                            )
                        })
                        .collect(),
                );
            }
            _ => {}
        }
    }
    Ok(())
}

/// What the objects of the layers break of their fields' rules, one line a field: a required value left empty, a
/// value its rule refuses (out of range, not in the list, not of the kind), a unique value given twice; with the first
/// objects' labels. Empty when every object keeps them.
pub fn rule_breaks(doc: &Document, layers: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for id in layers {
        let Some(node) = doc.layers().get(id) else {
            continue;
        };
        for f in &node.fields {
            let mut empty = Vec::new();
            let mut refused = Vec::new();
            let mut seen: HashMap<String, usize> = HashMap::new();
            let mut twice = Vec::new();
            for (i, (_, e)) in doc.by_layer_with_uids(id).enumerate() {
                let who = || {
                    let label = e.base().label.as_deref().unwrap_or("").trim();
                    if label.is_empty() {
                        format!("{}. nesne", i + 1)
                    } else {
                        label.to_owned()
                    }
                };
                let value = e.base().attrs.get(&f.name).map(|v| v.trim()).unwrap_or("");
                if value.is_empty() {
                    if f.required {
                        empty.push(who());
                    }
                    continue;
                }
                if check_value(f, value).is_err() {
                    refused.push(who());
                }
                if f.unique {
                    let n = seen.entry(value.to_owned()).or_insert(0);
                    *n += 1;
                    if *n == 2 {
                        twice.push(value.to_owned());
                    }
                }
            }
            let shown = f.alias.as_deref().unwrap_or(&f.name);
            let first = |v: &[String]| v.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            if !empty.is_empty() {
                out.push(format!(
                    "“{}” katmanının zorunlu “{shown}” alanı {} nesnede boş ({})",
                    node.name,
                    empty.len(),
                    first(&empty)
                ));
            }
            if !refused.is_empty() {
                out.push(format!(
                    "“{}” katmanının “{shown}” alanında {} değer kuralına uymuyor ({})",
                    node.name,
                    refused.len(),
                    first(&refused)
                ));
            }
            if !twice.is_empty() {
                out.push(format!(
                    "“{}” katmanının tekil “{shown}” alanında {} değer yineleniyor ({})",
                    node.name,
                    twice.len(),
                    first(&twice)
                ));
            }
        }
    }
    out
}
