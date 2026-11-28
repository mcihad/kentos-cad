//! A request in PostgreSQL's words (docs/adr/9100 §6): `$n` placeholders bound to typed values, never values written
//! into the text. A comparison binds the value as its field takes it (an integer, a double, a text, a yes or no; a
//! date or time cast to the column's own type), and a value of another kind than its field's keeps no row, as the
//! condition's own meaning says ([`kentos_data::condition::compare`]). Texts compared without case are folded the
//! Turkish way in SQL (`translate` for the dotted and dotless I and the Turkish capitals, then `lower`), whatever the
//! database's collation.

use kentos_data::condition::{escape_like, fold};
use kentos_data::{Condition, DataError, FieldKind, Request, Schema, Value};

use crate::catalog::{Shape, quote};

/// A bound value.
#[derive(Clone, Debug)]
pub enum Arg {
    Int(i64),
    Real(f64),
    Bool(bool),
    Text(String),
    /// A NULL of a text's type (cast where it is used).
    Null,
    Bytes(Vec<u8>),
    Ints(Vec<i64>),
}

/// Statement text being built with its values.
#[derive(Default)]
pub struct Sql {
    pub args: Vec<Arg>,
}

impl Sql {
    /// A placeholder for `a`.
    pub fn bind(&mut self, a: Arg) -> String {
        self.args.push(a);
        format!("${}", self.args.len())
    }
}

/// The Turkish fold of a text expression in SQL.
pub fn folded(expr: &str) -> String {
    format!("lower(translate({expr}, 'İIÇĞÖŞÜ', 'iıçğöşü'))")
}

fn column(
    shape: &Shape,
    table: &str,
    field: &str,
) -> Result<(String, FieldKind, String), DataError> {
    if field == shape.schema.key {
        return Ok((
            format!("t.{}", quote(field)),
            FieldKind::INT,
            "bigint".into(),
        ));
    }
    let f = shape
        .schema
        .field(field)
        .ok_or_else(|| DataError::NoField {
            table: table.to_owned(),
            field: field.to_owned(),
        })?;
    Ok((
        format!("t.{}", quote(field)),
        f.kind.clone(),
        f.native.clone(),
    ))
}

/// The column's expression as compared, and the placeholder of `v` as it is compared with it; none when the value
/// cannot be compared with that field.
fn operand(
    col: &str,
    kind: &FieldKind,
    native: &str,
    v: &Value,
    s: &mut Sql,
) -> Option<(String, String)> {
    match kind {
        FieldKind::Int { .. } | FieldKind::Real | FieldKind::Decimal { .. } => match (kind, v) {
            (FieldKind::Int { .. }, Value::Int(i)) => Some((col.to_owned(), s.bind(Arg::Int(*i)))),
            _ => v
                .number()
                .filter(|_| !matches!(v, Value::Bool(_)))
                .map(|x| (format!("{col}::float8"), s.bind(Arg::Real(x)))),
        },
        FieldKind::Bool => match v {
            Value::Bool(b) => Some((col.to_owned(), s.bind(Arg::Bool(*b)))),
            Value::Int(i) if *i == 0 || *i == 1 => {
                Some((col.to_owned(), s.bind(Arg::Bool(*i == 1))))
            }
            _ => None,
        },
        FieldKind::Text { .. } => match v {
            Value::Text(t) => Some((col.to_owned(), s.bind(Arg::Text(t.clone())))),
            _ => None,
        },
        FieldKind::Date | FieldKind::DateTime | FieldKind::Time => match v {
            Value::Date(t) | Value::DateTime(t) | Value::Time(t) | Value::Text(t) => {
                let p = s.bind(Arg::Text(t.clone()));
                Some((col.to_owned(), format!("CAST({p} AS {native})")))
            }
            _ => None,
        },
        FieldKind::Uuid | FieldKind::Json | FieldKind::Other => match v {
            Value::Text(t) | Value::Json(t) => {
                Some((format!("{col}::text"), s.bind(Arg::Text(t.clone()))))
            }
            _ => None,
        },
        FieldKind::Blob => None,
    }
}

/// A condition as SQL.
pub fn condition(
    c: &Condition,
    shape: &Shape,
    table: &str,
    s: &mut Sql,
) -> Result<String, DataError> {
    Ok(match c {
        Condition::And(cs) if cs.is_empty() => "true".into(),
        Condition::Or(cs) if cs.is_empty() => "false".into(),
        Condition::And(cs) => {
            let parts: Result<Vec<_>, _> =
                cs.iter().map(|c| condition(c, shape, table, s)).collect();
            format!("({})", parts?.join(" AND "))
        }
        Condition::Or(cs) => {
            let parts: Result<Vec<_>, _> =
                cs.iter().map(|c| condition(c, shape, table, s)).collect();
            format!("({})", parts?.join(" OR "))
        }
        Condition::Not(c) => format!("(NOT {})", condition(c, shape, table, s)?),
        Condition::Compare { field, op, value } => {
            let (col, kind, native) = column(shape, table, field)?;
            match operand(&col, &kind, &native, value, s) {
                Some((l, r)) => format!("({l} {} {r})", op.sql()),
                None => "NULL::boolean".into(),
            }
        }
        Condition::In { field, values } => {
            let (col, kind, native) = column(shape, table, field)?;
            let mut lhs = None;
            let mut list = Vec::new();
            for v in values.iter().filter(|v| !v.is_null()) {
                if let Some((l, r)) = operand(&col, &kind, &native, v, s) {
                    lhs = Some(l);
                    list.push(r);
                }
            }
            match lhs {
                Some(l) if !list.is_empty() => format!("({l} IN ({}))", list.join(", ")),
                _ => "false".into(),
            }
        }
        Condition::Between { field, low, high } => {
            let (col, kind, native) = column(shape, table, field)?;
            match (
                operand(&col, &kind, &native, low, s),
                operand(&col, &kind, &native, high, s),
            ) {
                (Some((l, a)), Some((_, b))) => format!("({l} BETWEEN {a} AND {b})"),
                _ => "NULL::boolean".into(),
            }
        }
        Condition::IsNull(f) => format!("({} IS NULL)", column(shape, table, f)?.0),
        Condition::NotNull(f) => format!("({} IS NOT NULL)", column(shape, table, f)?.0),
        // A typed column holds its own type: a text only in a text column (docs/adr/9109 §2).
        Condition::Text(f) => {
            let (col, kind, _) = column(shape, table, f)?;
            if kind.is_text() {
                format!("({col} IS NOT NULL)")
            } else {
                "FALSE".into()
            }
        }
        Condition::Like {
            field,
            pattern,
            fold: f,
        } => {
            let (col, ..) = column(shape, table, field)?;
            if *f {
                let p = s.bind(Arg::Text(fold(pattern)));
                format!("({} LIKE {p})", folded(&format!("{col}::text")))
            } else {
                let p = s.bind(Arg::Text(pattern.clone()));
                format!("({col}::text LIKE {p})")
            }
        }
        Condition::Contains { fields, text } => {
            if fields.is_empty() {
                "false".into()
            } else {
                let p = s.bind(Arg::Text(format!("%{}%", escape_like(&fold(text)))));
                let mut parts = Vec::with_capacity(fields.len());
                for f in fields {
                    let (col, ..) = column(shape, table, f)?;
                    parts.push(format!("{} LIKE {p}", folded(&format!("{col}::text"))));
                }
                format!("({})", parts.join(" OR "))
            }
        }
        Condition::Fids(fids) => {
            let p = s.bind(Arg::Ints(fids.clone()));
            format!("(t.{}::int8 = ANY({p}::int8[]))", quote(&shape.schema.key))
        }
    })
}

/// The geometry column's expression (`geometry`, a `geography` cast).
pub fn geometry_expr(shape: &Shape) -> Option<String> {
    shape.schema.geometry.as_ref().map(|g| {
        if shape.geography {
            format!("t.{}::geometry", quote(&g.name))
        } else {
            format!("t.{}", quote(&g.name))
        }
    })
}

/// A request's `WHERE` (with the word, or empty): its box over the index and its condition.
pub fn where_of(
    request: &Request,
    shape: &Shape,
    table: &str,
    s: &mut Sql,
) -> Result<String, DataError> {
    let mut parts = Vec::new();
    if let (Some(b), Some(g)) = (request.bbox, geometry_expr(shape)) {
        let srid = shape
            .schema
            .geometry
            .as_ref()
            .and_then(|g| g.srid)
            .unwrap_or(0);
        let (a, bb, c, d) = (
            s.bind(Arg::Real(b[0])),
            s.bind(Arg::Real(b[1])),
            s.bind(Arg::Real(b[2])),
            s.bind(Arg::Real(b[3])),
        );
        let envelope = format!("ST_MakeEnvelope({a}, {bb}, {c}, {d}, {srid})");
        // A geography column's index is on geography: its own `&&` with the box as geography uses it.
        if shape.geography {
            let col = format!(
                "t.{}",
                quote(
                    shape
                        .schema
                        .geometry
                        .as_ref()
                        .map_or("", |g| g.name.as_str())
                )
            );
            parts.push(format!("{col} && {envelope}::geography"));
        } else {
            parts.push(format!("{g} && {envelope}"));
        }
        if request.exact {
            parts.push(format!("ST_Intersects({g}, {envelope})"));
        }
    }
    if let Some(c) = &request.condition {
        parts.push(condition(c, shape, table, s)?);
    }
    Ok(if parts.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", parts.join(" AND "))
    })
}

/// A request's `ORDER BY` (NULLs last, the key last) and page.
pub fn order_and_page(request: &Request, shape: &Shape, table: &str) -> Result<String, DataError> {
    let mut by = Vec::with_capacity(request.order.len() + 1);
    for o in &request.order {
        let (col, ..) = column(shape, table, &o.field)?;
        // A text in the Turkish alphabet's order whatever the database's collation (docs/adr/9102 §3): ICU's Turkish
        // collation when the database has it, else each letter written as the byte of its place and compared bytewise.
        let text = shape
            .schema
            .fields
            .iter()
            .any(|f| f.name == o.field && matches!(f.kind, FieldKind::Text { .. }));
        let by_what = match (text, shape.turkish) {
            (false, _) => col,
            (true, true) => format!("{col} COLLATE \"tr-x-icu\""),
            (true, false) => {
                let (from, to) = kentos_data::turkish::sort_key_translation();
                format!("translate({col}::text, '{from}', '{to}') COLLATE \"C\"")
            }
        };
        by.push(format!(
            "{by_what} {} NULLS LAST",
            if o.descending { "DESC" } else { "ASC" }
        ));
    }
    by.push(format!("t.{}", quote(&shape.schema.key)));
    let mut out = format!(" ORDER BY {}", by.join(", "));
    if let Some(l) = request.limit {
        out.push_str(&format!(" LIMIT {l}"));
    }
    if request.offset > 0 {
        out.push_str(&format!(" OFFSET {}", request.offset));
    }
    Ok(out)
}

/// A field's value read as text in SQL (blobs as hex), so every type decodes the same way.
pub fn read_expr(col: &str, kind: &FieldKind) -> String {
    match kind {
        FieldKind::Blob => format!("encode({col}, 'hex')"),
        _ => format!("{col}::text"),
    }
}

/// A field's text as read back into a value.
pub fn value_of(text: Option<String>, kind: &FieldKind) -> Value {
    let Some(t) = text else {
        return Value::Null;
    };
    match kind {
        FieldKind::Bool => match t.as_str() {
            "true" | "t" => Value::Bool(true),
            "false" | "f" => Value::Bool(false),
            _ => Value::Text(t),
        },
        FieldKind::Int { .. } => t.parse().map_or(Value::Text(t), Value::Int),
        FieldKind::Real => t.parse().map_or(Value::Text(t), Value::Real),
        FieldKind::Decimal { .. } => Value::Decimal(t),
        FieldKind::Text { .. } | FieldKind::Uuid | FieldKind::Other => Value::Text(t),
        FieldKind::Date => Value::Date(t),
        FieldKind::DateTime => Value::DateTime(t.replacen(' ', "T", 1)),
        FieldKind::Time => Value::Time(t),
        FieldKind::Json => Value::Json(t),
        FieldKind::Blob => Value::Blob(
            (0..t.len() / 2)
                .filter_map(|i| u8::from_str_radix(t.get(2 * i..2 * i + 2)?, 16).ok())
                .collect(),
        ),
    }
}

/// A value as written to a field: its placeholder cast to the field's own type.
pub fn write_arg(v: &Value, native: &str, s: &mut Sql) -> String {
    let arg = match v {
        Value::Null => Arg::Null,
        Value::Bool(b) => Arg::Bool(*b),
        Value::Int(i) => Arg::Int(*i),
        Value::Real(r) => Arg::Real(*r),
        Value::Blob(b) => Arg::Bytes(b.clone()),
        Value::Decimal(t)
        | Value::Text(t)
        | Value::Date(t)
        | Value::DateTime(t)
        | Value::Time(t)
        | Value::Json(t) => Arg::Text(t.clone()),
    };
    let p = s.bind(arg);
    match v {
        // A typed value goes through text when the column is of another type (a number into a text column).
        Value::Bool(_) | Value::Int(_) | Value::Real(_) if native_is_text(native) => {
            format!("CAST({p}::text AS {native})")
        }
        Value::Blob(_) => format!("CAST({p} AS {native})"),
        _ => format!("CAST({p} AS {native})"),
    }
}

fn native_is_text(native: &str) -> bool {
    let n = native.to_ascii_lowercase();
    n.starts_with("text")
        || n.starts_with("character")
        || n.starts_with("varchar")
        || n.starts_with("citext")
}

/// The schema's fields read: all of them, or the request's in its order.
pub fn columns_of<'a>(
    request: &Request,
    schema: &'a Schema,
    table: &str,
) -> Result<Vec<&'a kentos_data::Field>, DataError> {
    match &request.columns {
        None => Ok(schema.fields.iter().collect()),
        Some(list) => list
            .iter()
            .map(|n| {
                schema.field(n).ok_or_else(|| DataError::NoField {
                    table: table.to_owned(),
                    field: n.clone(),
                })
            })
            .collect(),
    }
}
