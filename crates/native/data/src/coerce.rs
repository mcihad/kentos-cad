//! A value made fit for the field it is written to, or refused with why (docs/adr/9100 §5): every driver writes only
//! what its field holds, so a decimal does not round itself into an integer column and a word does not land in a date.
//! Text that reads as the field's kind is taken (a table's cell typed by hand comes as text).

use crate::error::DataError;
use crate::schema::{Field, FieldKind};
use crate::value::Value;

fn refuse(field: &Field, v: &Value) -> DataError {
    DataError::Invalid(format!(
        "“{}” alanı {} tutar; “{v}” ({}) yazılamaz.",
        field.name,
        field.kind.words().to_lowercase(),
        v.kind_words()
    ))
}

fn iso_date(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// `v` as `field` holds it.
pub fn coerce(field: &Field, v: &Value) -> Result<Value, DataError> {
    if v.is_null() {
        return if field.nullable || field.default.is_some() || field.generated {
            Ok(Value::Null)
        } else {
            Err(DataError::Invalid(format!(
                "“{}” alanı boş geçilemez.",
                field.name
            )))
        };
    }
    let text = |v: &Value| match v {
        Value::Text(t) => Some(t.trim().to_owned()),
        _ => None,
    };
    Ok(match &field.kind {
        FieldKind::Bool => match v {
            Value::Bool(b) => Value::Bool(*b),
            Value::Int(0) => Value::Bool(false),
            Value::Int(1) => Value::Bool(true),
            _ => match text(v).map(|t| t.to_lowercase()).as_deref() {
                Some("evet" | "true" | "1" | "doğru") => Value::Bool(true),
                Some("hayır" | "false" | "0" | "yanlış") => Value::Bool(false),
                _ => return Err(refuse(field, v)),
            },
        },
        FieldKind::Int => match v.int() {
            Some(i) => Value::Int(i),
            None => match text(v).and_then(|t| t.parse::<i64>().ok()) {
                Some(i) => Value::Int(i),
                None => return Err(refuse(field, v)),
            },
        },
        FieldKind::Real => match v.number().filter(|_| !matches!(v, Value::Bool(_))) {
            Some(x) => Value::Real(x),
            None => match text(v).and_then(|t| t.replace(',', ".").parse::<f64>().ok()) {
                Some(x) => Value::Real(x),
                None => return Err(refuse(field, v)),
            },
        },
        FieldKind::Decimal { .. } => match v {
            Value::Decimal(d) => Value::Decimal(d.clone()),
            Value::Int(i) => Value::Decimal(i.to_string()),
            Value::Real(r) if r.is_finite() => Value::Decimal(r.to_string()),
            _ => match text(v).map(|t| t.replace(',', ".")) {
                Some(t) if t.parse::<f64>().is_ok() => Value::Decimal(t),
                _ => return Err(refuse(field, v)),
            },
        },
        FieldKind::Text { max } => {
            let t = match v {
                Value::Blob(_) => return Err(refuse(field, v)),
                other => other.to_string(),
            };
            if let Some(m) = max
                && t.chars().count() > *m as usize
            {
                return Err(DataError::Invalid(format!(
                    "“{}” alanı en çok {m} karakter tutar; yazılan {} karakter.",
                    field.name,
                    t.chars().count()
                )));
            }
            Value::Text(t)
        }
        FieldKind::Date => match v {
            Value::Date(d) => Value::Date(d.clone()),
            Value::DateTime(d) => Value::Date(d.chars().take(10).collect()),
            _ => match text(v) {
                Some(t) if iso_date(&t) => Value::Date(t),
                _ => return Err(refuse(field, v)),
            },
        },
        FieldKind::DateTime => match v {
            Value::DateTime(d) => Value::DateTime(d.clone()),
            Value::Date(d) => Value::DateTime(format!("{d}T00:00:00")),
            _ => match text(v) {
                Some(t) if t.len() >= 10 && iso_date(&t[..10]) => {
                    Value::DateTime(t.replacen(' ', "T", 1))
                }
                _ => return Err(refuse(field, v)),
            },
        },
        FieldKind::Time => match v {
            Value::Time(t) => Value::Time(t.clone()),
            _ => match text(v) {
                Some(t) if t.contains(':') => Value::Time(t),
                _ => return Err(refuse(field, v)),
            },
        },
        FieldKind::Blob => match v {
            Value::Blob(b) => Value::Blob(b.clone()),
            _ => return Err(refuse(field, v)),
        },
        FieldKind::Json => match v {
            Value::Json(j) => Value::Json(j.clone()),
            Value::Text(t) => Value::Json(t.clone()),
            _ => return Err(refuse(field, v)),
        },
        FieldKind::Uuid => match v {
            Value::Text(t) if t.len() == 36 => Value::Text(t.clone()),
            _ => return Err(refuse(field, v)),
        },
        FieldKind::Other => {
            return Err(DataError::Unsupported(format!(
                "“{}” alanının türü ({}) yazılamaz",
                field.name, field.native
            )));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_fit_their_fields_or_are_refused() {
        let int = Field::new("kat", FieldKind::Int);
        assert_eq!(coerce(&int, &Value::Real(4.0)), Ok(Value::Int(4)));
        assert!(coerce(&int, &Value::Real(4.5)).is_err());
        assert_eq!(coerce(&int, &Value::Text(" 7 ".into())), Ok(Value::Int(7)));
        let real = Field::new("alan", FieldKind::Real);
        assert_eq!(
            coerce(&real, &Value::Text("12,5".into())),
            Ok(Value::Real(12.5))
        );
        let date = Field::new("tarih", FieldKind::Date);
        assert!(coerce(&date, &Value::Text("20.11.2026".into())).is_err());
        assert_eq!(
            coerce(&date, &Value::Text("2026-11-20".into())),
            Ok(Value::Date("2026-11-20".into()))
        );
        let short = Field::new("kod", FieldKind::Text { max: Some(3) });
        assert!(coerce(&short, &Value::Text("ABCD".into())).is_err());
        let mut must = Field::new("ad", FieldKind::Text { max: None });
        must.nullable = false;
        assert!(coerce(&must, &Value::Null).is_err());
        let yes = Field::new("onayli", FieldKind::Bool);
        assert_eq!(
            coerce(&yes, &Value::Text("Evet".into())),
            Ok(Value::Bool(true))
        );
    }
}
