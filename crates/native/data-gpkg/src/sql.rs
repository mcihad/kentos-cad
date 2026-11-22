//! A request in SQLite's words (docs/adr/9100 §4, §5): the box over the spatial index, the condition, the order and
//! the page; values bound, never written into the text. A text compared without case goes through `kentos_fold`, the
//! same folding as [`kentos_data::condition::fold`] (SQLite's own `LIKE` folds ASCII letters only).

use kentos_data::condition::{escape_like, fold, like};
use kentos_data::{Condition, DataError, FieldKind, Request, Schema, Value};
use kentos_geopackage::quote;
use rusqlite::Connection;
use rusqlite::functions::FunctionFlags;
use rusqlite::types::Value as Sql;

use crate::convert::sql_of;

/// Registers `kentos_fold(text)`, `kentos_like(text, pattern)` (SQL `LIKE` with case, `\` escaping) and
/// `kentos_order(text)` (the bytes a text is ordered by in the Turkish alphabet's order, `kentos_data::turkish`) on a
/// connection.
pub fn register_fold(conn: &Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function(
        "kentos_order",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            Ok(match ctx.get_raw(0) {
                rusqlite::types::ValueRef::Text(t) => Some(rusqlite::types::Value::Blob(
                    kentos_data::turkish::sort_key(&String::from_utf8_lossy(t)),
                )),
                rusqlite::types::ValueRef::Null => None,
                // A number in a text column orders below the texts, as SQLite orders it.
                rusqlite::types::ValueRef::Integer(i) => Some(rusqlite::types::Value::Integer(i)),
                rusqlite::types::ValueRef::Real(r) => Some(rusqlite::types::Value::Real(r)),
                rusqlite::types::ValueRef::Blob(b) => Some(rusqlite::types::Value::Blob(b.to_vec())),
            })
        },
    )?;
    conn.create_scalar_function(
        "kentos_like",
        2,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let text = match ctx.get_raw(0) {
                rusqlite::types::ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                rusqlite::types::ValueRef::Integer(i) => i.to_string(),
                rusqlite::types::ValueRef::Real(r) => r.to_string(),
                _ => return Ok(None),
            };
            let pattern = match ctx.get_raw(1) {
                rusqlite::types::ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                _ => return Ok(None),
            };
            Ok(Some(like(&text, &pattern)))
        },
    )?;
    conn.create_scalar_function(
        "kentos_fold",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            Ok(match ctx.get_raw(0) {
                rusqlite::types::ValueRef::Null => None,
                rusqlite::types::ValueRef::Text(t) => Some(fold(&String::from_utf8_lossy(t))),
                rusqlite::types::ValueRef::Integer(i) => Some(i.to_string()),
                rusqlite::types::ValueRef::Real(r) => Some(r.to_string()),
                rusqlite::types::ValueRef::Blob(_) => None,
            })
        },
    )
}

/// The parts of a statement: its `WHERE` (without the word) and its bound values.
#[derive(Default)]
pub struct Where {
    pub sql: Vec<String>,
    pub args: Vec<Sql>,
}

impl Where {
    pub fn text(&self) -> String {
        if self.sql.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", self.sql.join(" AND "))
        }
    }
}

/// The column of a field, refused when the table has none of that name.
fn column(schema: &Schema, table: &str, field: &str) -> Result<String, DataError> {
    if field == schema.key || schema.index(field).is_some() {
        Ok(format!("t.{}", quote(field)))
    } else {
        Err(DataError::NoField {
            table: table.to_owned(),
            field: field.to_owned(),
        })
    }
}

/// A condition as SQL with its values.
pub fn condition(
    c: &Condition,
    schema: &Schema,
    table: &str,
    args: &mut Vec<Sql>,
) -> Result<String, DataError> {
    Ok(match c {
        Condition::And(cs) if cs.is_empty() => "1".into(),
        Condition::Or(cs) if cs.is_empty() => "0".into(),
        Condition::And(cs) => {
            let parts: Result<Vec<_>, _> = cs
                .iter()
                .map(|c| condition(c, schema, table, args))
                .collect();
            format!("({})", parts?.join(" AND "))
        }
        Condition::Or(cs) => {
            let parts: Result<Vec<_>, _> = cs
                .iter()
                .map(|c| condition(c, schema, table, args))
                .collect();
            format!("({})", parts?.join(" OR "))
        }
        Condition::Not(c) => format!("(NOT {})", condition(c, schema, table, args)?),
        Condition::Compare { field, op, value } => {
            let col = column(schema, table, field)?;
            if value.is_null() {
                // A comparison with NULL keeps no row, as SQL's own.
                "NULL".into()
            } else {
                args.push(sql_of(value));
                format!("({col} {} ?)", op.sql())
            }
        }
        Condition::In { field, values } => {
            let col = column(schema, table, field)?;
            let kept: Vec<&Value> = values.iter().filter(|v| !v.is_null()).collect();
            if kept.is_empty() {
                "0".into()
            } else {
                args.extend(kept.iter().map(|v| sql_of(v)));
                format!("({col} IN ({}))", vec!["?"; kept.len()].join(", "))
            }
        }
        Condition::Between { field, low, high } => {
            let col = column(schema, table, field)?;
            args.push(sql_of(low));
            args.push(sql_of(high));
            format!("({col} BETWEEN ? AND ?)")
        }
        Condition::IsNull(f) => format!("({} IS NULL)", column(schema, table, f)?),
        Condition::Text(f) => format!("(typeof({}) = 'text')", column(schema, table, f)?),
        Condition::NotNull(f) => format!("({} IS NOT NULL)", column(schema, table, f)?),
        Condition::Like {
            field,
            pattern,
            fold: folded,
        } => {
            let col = column(schema, table, field)?;
            if *folded {
                args.push(Sql::Text(fold(pattern)));
                format!("(kentos_fold({col}) LIKE ? ESCAPE '\\')")
            } else {
                args.push(Sql::Text(pattern.clone()));
                // SQLite's own LIKE ignores ASCII letters' case: ours keeps it.
                format!("(kentos_like({col}, ?))")
            }
        }
        Condition::Contains { fields, text } => {
            if fields.is_empty() {
                "0".into()
            } else {
                let pattern = format!("%{}%", escape_like(&fold(text)));
                let mut parts = Vec::with_capacity(fields.len());
                for f in fields {
                    let col = column(schema, table, f)?;
                    args.push(Sql::Text(pattern.clone()));
                    parts.push(format!(
                        "kentos_fold(CAST({col} AS TEXT)) LIKE ? ESCAPE '\\'"
                    ));
                }
                format!("({})", parts.join(" OR "))
            }
        }
        Condition::Fids(fids) => {
            if fids.is_empty() {
                "0".into()
            } else {
                let key = format!("t.{}", quote(&schema.key));
                args.extend(fids.iter().map(|f| Sql::Integer(*f)));
                format!("({key} IN ({}))", vec!["?"; fids.len()].join(", "))
            }
        }
    })
}

/// A request's `WHERE`: its box (through the spatial index when there is one) and its condition.
pub fn where_of(
    request: &Request,
    schema: &Schema,
    table: &str,
    indexed: bool,
) -> Result<Where, DataError> {
    let mut w = Where::default();
    if let (Some(b), Some(g)) = (request.bbox, &schema.geometry) {
        let col = quote(&g.name);
        if indexed {
            let rtree = quote(&format!("rtree_{table}_{}", g.name));
            w.sql.push(format!(
                "t.{} IN (SELECT id FROM {rtree} WHERE minx <= ? AND maxx >= ? AND miny <= ? AND maxy >= ?)",
                quote(&schema.key)
            ));
        } else {
            w.sql.push(format!(
                "(ST_MinX(t.{col}) <= ? AND ST_MaxX(t.{col}) >= ? AND ST_MinY(t.{col}) <= ? AND ST_MaxY(t.{col}) >= ?)"
            ));
        }
        w.args.extend([
            Sql::Real(b[2]),
            Sql::Real(b[0]),
            Sql::Real(b[3]),
            Sql::Real(b[1]),
        ]);
    }
    if let Some(c) = &request.condition {
        let mut args = Vec::new();
        let s = condition(c, schema, table, &mut args)?;
        w.sql.push(s);
        w.args.extend(args);
    }
    Ok(w)
}

/// A request's `ORDER BY` (NULLs last, the key last) and page.
pub fn order_and_page(
    request: &Request,
    schema: &Schema,
    table: &str,
) -> Result<String, DataError> {
    let mut by = Vec::with_capacity(request.order.len() + 1);
    for o in &request.order {
        let col = column(schema, table, &o.field)?;
        // A text in the Turkish alphabet's order, not by its bytes (docs/adr/9102 §3).
        let text = schema
            .fields
            .iter()
            .any(|f| f.name == o.field && matches!(f.kind, FieldKind::Text { .. }));
        let by_what = if text { format!("kentos_order({col})") } else { col };
        by.push(format!(
            "{by_what} {} NULLS LAST",
            if o.descending { "DESC" } else { "ASC" }
        ));
    }
    by.push(format!("t.{}", quote(&schema.key)));
    let mut s = format!(" ORDER BY {}", by.join(", "));
    match (request.limit, request.offset) {
        (Some(l), o) => s.push_str(&format!(" LIMIT {l} OFFSET {o}")),
        (None, 0) => {}
        (None, o) => s.push_str(&format!(" LIMIT -1 OFFSET {o}")),
    }
    Ok(s)
}
