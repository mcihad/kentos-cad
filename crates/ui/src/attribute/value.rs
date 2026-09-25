//! Öznitelik değerleri.

use std::cmp::Ordering;
use std::fmt;

use super::text;
use super::time::{Date, DateTime, Time};

/// Bir katman ya da tablo içindeki nesnenin kalıcı numarası (ArcGIS'teki
/// OBJECTID). Nesne silinse de başka nesnelerin numarası değişmez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ObjectId(pub u64);

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tek bir öznitelik değeri.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
    /// Değer girilmemiş (ArcGIS'teki `<Null>`).
    #[default]
    Null,
    Bool(bool),
    Integer(i64),
    Real(f64),
    Text(String),
    Date(Date),
    Time(Time),
    DateTime(DateTime),
    /// Başka bir nesneye başvuru.
    Object(ObjectId),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Sayısal değer (tam sayı ya da ondalık).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Integer(value) => Some(*value as f64),
            Value::Real(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<ObjectId> {
        match self {
            Value::Object(id) => Some(*id),
            _ => None,
        }
    }

    /// Sıralama için karşılaştırır: boş değerler başta gelir, metinler Türk
    /// alfabesine, sayılar büyüklüğe, tarihler zamana göre sıralanır.
    pub fn compare(&self, other: &Value) -> Ordering {
        match (self, other) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Less,
            (_, Value::Null) => Ordering::Greater,
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Text(a), Value::Text(b)) => text::compare(a, b),
            (Value::Date(a), Value::Date(b)) => a.cmp(b),
            (Value::Time(a), Value::Time(b)) => a.cmp(b),
            (Value::DateTime(a), Value::DateTime(b)) => a.cmp(b),
            (Value::Object(a), Value::Object(b)) => a.cmp(b),
            (a, b) => match (a.as_f64(), b.as_f64()) {
                (Some(a), Some(b)) => a.total_cmp(&b),
                _ => a.rank().cmp(&b.rank()),
            },
        }
    }

    /// Farklı türlerin kendi aralarındaki sırası.
    fn rank(&self) -> u8 {
        match self {
            Value::Null => 0,
            Value::Bool(_) => 1,
            Value::Integer(_) | Value::Real(_) => 2,
            Value::Text(_) => 3,
            Value::Date(_) => 4,
            Value::Time(_) => 5,
            Value::DateTime(_) => 6,
            Value::Object(_) => 7,
        }
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::Text(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Text(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Integer(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Real(value)
    }
}

impl From<Date> for Value {
    fn from(value: Date) -> Self {
        Value::Date(value)
    }
}

impl From<Time> for Value {
    fn from(value: Time) -> Self {
        Value::Time(value)
    }
}

impl From<DateTime> for Value {
    fn from(value: DateTime) -> Self {
        Value::DateTime(value)
    }
}

impl From<ObjectId> for Value {
    fn from(value: ObjectId) -> Self {
        Value::Object(value)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map_or(Value::Null, Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nulls_sort_first_and_numbers_compare_across_types() {
        let mut values = vec![
            Value::Integer(10),
            Value::Null,
            Value::Real(2.5),
            Value::Integer(-1),
        ];
        values.sort_by(Value::compare);

        assert_eq!(
            values,
            [
                Value::Null,
                Value::Integer(-1),
                Value::Real(2.5),
                Value::Integer(10)
            ]
        );
    }

    #[test]
    fn text_sorts_in_turkish_order() {
        assert_eq!(
            Value::from("Çorum").compare(&Value::from("Denizli")),
            Ordering::Less
        );
        assert_eq!(
            Value::from("Çorum").compare(&Value::from("Cizre")),
            Ordering::Greater
        );
    }
}
