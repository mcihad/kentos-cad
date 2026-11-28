//! A data layer's field counted by its source (docs/adr/9181): İstatistik and Grafik count a data layer's values with
//! the file's or the database's own `GROUP BY` (`Source::groups`) instead of reading its rows. One row a value comes
//! back, no geometry is read and no object is built. Each value carries the text a row of the layer reads it as
//! (`attribute_text`, what the drawing's attributes keep), so a count from the source and one from the rows read meet
//! value for value. The rows the drawing holds (taken or new in the edit session) are left out by their keys; the
//! caller counts them as the drawing has them. A field with more values than [`GROUPS_MOST`], or a drawing holding
//! more rows than [`HELD_MOST`], is not counted here: the rows are read and counted then.
//!
//! [`value_keys`] is the way back: the keys of the rows holding some of the counted values, so that a press on a value
//! (İstatistik's value list, a bar of Grafik) selects them in the layer's table.

use std::collections::BTreeSet;

use kentos_commands::LayerNode;
use kentos_data::{Cancel, Condition, Request, Value};

use crate::geopackage::attribute_text;

/// The most values a field's count takes from the source, a guard on the memory the values take (a field whose
/// every row holds its own value comes back a value a row, still without geometry): past it the rows are read and
/// counted instead.
pub const GROUPS_MOST: usize = 5_000_000;
/// The most rows the drawing may hold while the source counts the others: past it the rows are read instead.
pub const HELD_MOST: usize = 10_000;
/// The values one question for keys names (a database binds each): more are asked a part at a time.
const VALUES_AT_ONCE: usize = 500;

/// A value of a field as the source counted it.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldGroup {
    /// The value as the source holds it: a press asks for the rows holding it.
    pub value: Value,
    /// The value as a row of the layer reads it; none for an empty one.
    pub text: Option<String>,
    /// How many rows hold it.
    pub rows: u64,
}

fn source_of(
    node: &LayerNode,
) -> Result<
    (
        std::sync::Arc<dyn kentos_data::Source>,
        &kentos_commands::DataLayer,
    ),
    String,
> {
    let Some(data) = &node.data else {
        return Err(format!("“{}” bir veri katmanı değil.", node.name));
    };
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| format!("“{}” tablosu açılamadı: {e}", data.table))?;
    Ok((source, data))
}

/// The rows the drawing holds, left out of a question to the source.
fn left_out(leave: &BTreeSet<i64>) -> Option<Condition> {
    (!leave.is_empty())
        .then(|| Condition::Not(Box::new(Condition::Fids(leave.iter().copied().collect()))))
}

/// `field` of `node`'s table counted by its source, the rows `leave` names left out: its values with their rows, in
/// the source's order of values (the empty one last). None when the field has more than [`GROUPS_MOST`] values or
/// `leave` names more than [`HELD_MOST`] rows (the rows are read then); the message of why the source could not
/// count it.
pub fn field_counts(
    node: &LayerNode,
    field: &str,
    leave: &BTreeSet<i64>,
) -> Result<Option<Vec<FieldGroup>>, String> {
    if leave.len() > HELD_MOST {
        return Ok(None);
    }
    let (source, data) = source_of(node)?;
    let groups = source
        .groups(&data.table, field, left_out(leave).as_ref(), GROUPS_MOST)
        .map_err(|e| {
            format!(
                "“{}” tablosunun “{field}” alanı sayılamadı: {e}",
                data.table
            )
        })?;
    if groups.len() >= GROUPS_MOST {
        return Ok(None);
    }
    Ok(Some(
        groups
            .into_iter()
            .map(|(value, rows)| FieldGroup {
                text: attribute_text(&value),
                value,
                rows,
            })
            .collect(),
    ))
}

/// The keys of the rows of `node`'s table holding one of `values` in `field` (the empty value: the rows without one),
/// the rows `leave` names left out, in the table's order: at most `most` of them, and whether there were more. The
/// message of why they could not be read.
pub fn value_keys(
    node: &LayerNode,
    field: &str,
    values: &[Value],
    leave: &BTreeSet<i64>,
    most: usize,
    stopped: &dyn Fn() -> bool,
) -> Result<(Vec<i64>, bool), String> {
    let (source, data) = source_of(node)?;
    let given: Vec<Value> = values
        .iter()
        .filter(|v| !matches!(v, Value::Null))
        .cloned()
        .collect();
    let mut asks: Vec<Condition> = given
        .chunks(VALUES_AT_ONCE)
        .map(|part| Condition::In {
            field: field.to_owned(),
            values: part.to_vec(),
        })
        .collect();
    if values.iter().any(|v| matches!(v, Value::Null)) {
        asks.push(Condition::IsNull(field.to_owned()));
    }
    let mut keys: Vec<i64> = Vec::new();
    for ask in asks {
        let condition = match left_out(leave) {
            Some(out) => Condition::And(vec![ask, out]),
            None => ask,
        };
        let request = Request {
            condition: Some(condition),
            columns: Some(Vec::new()),
            geometry: false,
            // One past the most: whether there were more.
            limit: Some(most as u64 + 1),
            ..Request::default()
        };
        source
            .features(&data.table, &request, &Cancel::new(), &mut |part| {
                keys.extend(part.iter().map(|f| f.fid));
                !stopped()
            })
            .map_err(|e| format!("“{}” tablosunun satırları okunamadı: {e}", data.table))?;
        if stopped() {
            return Err("durduruldu".into());
        }
    }
    keys.sort_unstable();
    keys.dedup();
    let more = keys.len() > most;
    keys.truncate(most);
    Ok((keys, more))
}
