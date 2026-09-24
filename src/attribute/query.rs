//! Öznitelik sorguları: "öznitelikle seç" ve tablo filtreleri.
//!
//! Bir sorgu koşullardan oluşur; her koşul bir alan, bir işleç ve metin
//! olarak girilmiş bir değerdir. Değer, alanın türüne göre çözümlenir.
//! SQL'deki gibi boş değerler karşılaştırmaları sağlamaz; yalnızca "boş" ve
//! "boş değil" işleçleriyle yakalanır.

use std::cmp::Ordering;
use std::fmt;

use super::field::{Field, FieldKind};
use super::text;
use super::value::Value;

/// Karşılaştırma işleci.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    Equals,
    NotEquals,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
    Contains,
    StartsWith,
    IsNull,
    IsNotNull,
}

impl Operator {
    /// Bir değere ihtiyaç duyar mı ("boş" ve "boş değil" duymaz).
    pub fn needs_value(self) -> bool {
        !matches!(self, Operator::IsNull | Operator::IsNotNull)
    }

    /// Alan türüne uygun işleçler.
    pub fn for_kind(kind: &FieldKind) -> &'static [Operator] {
        use Operator::*;

        match kind {
            FieldKind::Text => &[Equals, NotEquals, Contains, StartsWith, IsNull, IsNotNull],
            FieldKind::Choice(_) => &[Equals, NotEquals, IsNull, IsNotNull],
            FieldKind::Bool => &[Equals, IsNull, IsNotNull],
            FieldKind::Object { .. } => &[Equals, NotEquals, IsNull, IsNotNull],
            FieldKind::Integer { .. }
            | FieldKind::Real { .. }
            | FieldKind::Range { .. }
            | FieldKind::Date
            | FieldKind::Time
            | FieldKind::DateTime => &[
                Equals,
                NotEquals,
                Less,
                LessOrEqual,
                Greater,
                GreaterOrEqual,
                IsNull,
                IsNotNull,
            ],
        }
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Operator::Equals => "=",
            Operator::NotEquals => "≠",
            Operator::Less => "<",
            Operator::LessOrEqual => "≤",
            Operator::Greater => ">",
            Operator::GreaterOrEqual => "≥",
            Operator::Contains => "içerir",
            Operator::StartsWith => "ile başlar",
            Operator::IsNull => "boş",
            Operator::IsNotNull => "boş değil",
        })
    }
}

/// Koşulların nasıl birleştirileceği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Combinator {
    /// Bütün koşullar sağlanmalı (VE).
    #[default]
    All,
    /// Koşullardan biri yeter (VEYA).
    Any,
}

impl Combinator {
    pub const ALL: [Combinator; 2] = [Combinator::All, Combinator::Any];

    /// Sorgu açıklamasındaki bağlaç.
    pub fn keyword(self) -> &'static str {
        match self {
            Combinator::All => "VE",
            Combinator::Any => "VEYA",
        }
    }
}

impl fmt::Display for Combinator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Combinator::All => "Tüm koşullar (VE)",
            Combinator::Any => "Herhangi biri (VEYA)",
        })
    }
}

/// Tek koşul: `alan işleç değer`.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// Şemadaki alan numarası.
    pub field: usize,
    pub operator: Operator,
    /// Kullanıcının yazdığı değer; alan türüne göre çözümlenir.
    pub value: String,
}

/// Öznitelik sorgusu.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Query {
    pub conditions: Vec<Condition>,
    pub combinator: Combinator,
}

/// Sorguda yapılan değişiklik; sorgu oluşturucunun ürettiği mesaj.
#[derive(Debug, Clone, PartialEq)]
pub enum Edit {
    /// Sona yeni koşul ekler.
    Add,
    Remove(usize),
    /// Koşulun alanını değiştirir; işleç yeni türe uymuyorsa ilk uygun
    /// işlece döner ve değer temizlenir.
    Field(usize, usize),
    Operator(usize, Operator),
    Value(usize, String),
    Combinator(Combinator),
    Clear,
}

impl Query {
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    /// Değişikliği uygular.
    pub fn apply(&mut self, edit: Edit, schema: &[Field]) {
        match edit {
            Edit::Add => {
                let field = self
                    .conditions
                    .last()
                    .map_or(0, |condition| condition.field)
                    .min(schema.len().saturating_sub(1));
                let operator = schema
                    .get(field)
                    .map_or(Operator::Equals, |field| Operator::for_kind(&field.kind)[0]);

                self.conditions.push(Condition {
                    field,
                    operator,
                    value: String::new(),
                });
            }
            Edit::Remove(index) => {
                if index < self.conditions.len() {
                    self.conditions.remove(index);
                }
            }
            Edit::Field(index, field) => {
                if let (Some(condition), Some(definition)) =
                    (self.conditions.get_mut(index), schema.get(field))
                {
                    let operators = Operator::for_kind(&definition.kind);

                    if condition.field != field {
                        condition.value.clear();
                    }

                    condition.field = field;

                    if !operators.contains(&condition.operator) {
                        condition.operator = operators[0];
                    }
                }
            }
            Edit::Operator(index, operator) => {
                if let Some(condition) = self.conditions.get_mut(index) {
                    condition.operator = operator;
                }
            }
            Edit::Value(index, value) => {
                if let Some(condition) = self.conditions.get_mut(index) {
                    condition.value = value;
                }
            }
            Edit::Combinator(combinator) => self.combinator = combinator,
            Edit::Clear => self.conditions.clear(),
        }
    }

    /// Kaydın değerleri sorguyu sağlıyor mu. Koşulsuz sorgu her kaydı
    /// sağlar; değeri çözümlenemeyen koşul hiçbir kaydı sağlamaz.
    pub fn matches(&self, schema: &[Field], values: &[Value]) -> bool {
        if self.conditions.is_empty() {
            return true;
        }

        let mut results = self.conditions.iter().map(|condition| {
            schema.get(condition.field).is_some_and(|field| {
                evaluate(
                    condition,
                    field,
                    values.get(condition.field).unwrap_or(&Value::Null),
                )
            })
        });

        match self.combinator {
            Combinator::All => results.all(|matched| matched),
            Combinator::Any => results.any(|matched| matched),
        }
    }

    /// Değeri çözümlenemeyen koşullar ve hata açıklamaları.
    pub fn errors(&self, schema: &[Field]) -> Vec<(usize, String)> {
        self.conditions
            .iter()
            .enumerate()
            .filter_map(|(index, condition)| {
                let field = schema.get(condition.field)?;

                if !condition.operator.needs_value() || is_text_search(condition.operator) {
                    return None;
                }

                if condition.value.trim().is_empty() {
                    return Some((index, "Bir değer girin.".to_owned()));
                }

                field
                    .parse(&condition.value)
                    .err()
                    .map(|error| (index, error))
            })
            .collect()
    }

    /// Sorgunun okunur açıklaması: `Bölge = "Marmara" VE Nüfus > 2.000.000`.
    pub fn describe(&self, schema: &[Field]) -> String {
        self.conditions
            .iter()
            .map(|condition| {
                let name = schema
                    .get(condition.field)
                    .map_or("?", |field| field.name.as_str());

                if !condition.operator.needs_value() {
                    return format!("{name} {}", condition.operator);
                }

                let value = match schema.get(condition.field).map(|field| &field.kind) {
                    Some(FieldKind::Text | FieldKind::Choice(_)) => {
                        format!("\"{}\"", condition.value.trim())
                    }
                    _ => condition.value.trim().to_owned(),
                };

                format!("{name} {} {value}", condition.operator)
            })
            .collect::<Vec<_>>()
            .join(&format!(" {} ", self.combinator.keyword()))
    }
}

fn is_text_search(operator: Operator) -> bool {
    matches!(operator, Operator::Contains | Operator::StartsWith)
}

fn evaluate(condition: &Condition, field: &Field, value: &Value) -> bool {
    match condition.operator {
        Operator::IsNull => return value.is_null(),
        Operator::IsNotNull => return !value.is_null(),
        _ if value.is_null() => return false,
        _ => {}
    }

    if is_text_search(condition.operator) {
        let haystack = text::fold(&field.format(value));
        let needle = text::fold(condition.value.trim());

        return match condition.operator {
            Operator::Contains => haystack.contains(&needle),
            _ => haystack.starts_with(&needle),
        };
    }

    let Ok(expected) = field.parse(&condition.value) else {
        return false;
    };

    if expected.is_null() {
        return false;
    }

    let ordering = match (value, &expected) {
        (Value::Text(actual), Value::Text(expected)) => {
            if text::fold(actual) == text::fold(expected) {
                Ordering::Equal
            } else {
                text::compare(actual, expected)
            }
        }
        _ => value.compare(&expected),
    };

    match condition.operator {
        Operator::Equals => ordering == Ordering::Equal,
        Operator::NotEquals => ordering != Ordering::Equal,
        Operator::Less => ordering == Ordering::Less,
        Operator::LessOrEqual => ordering != Ordering::Greater,
        Operator::Greater => ordering == Ordering::Greater,
        Operator::GreaterOrEqual => ordering != Ordering::Less,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::Date;

    fn schema() -> Vec<Field> {
        vec![
            Field::text("Ad"),
            Field::choice("Bölge", ["Marmara", "Ege", "Akdeniz"]),
            Field::integer("Nüfus"),
            Field::boolean("Kıyı"),
            Field::date("Tarih"),
        ]
    }

    fn city(name: &str, region: &str, population: i64, coastal: bool) -> Vec<Value> {
        vec![
            Value::from(name),
            Value::from(region),
            Value::Integer(population),
            Value::Bool(coastal),
            Value::Null,
        ]
    }

    fn condition(field: usize, operator: Operator, value: &str) -> Condition {
        Condition {
            field,
            operator,
            value: value.to_owned(),
        }
    }

    #[test]
    fn empty_query_matches_everything() {
        assert!(Query::default().matches(&schema(), &city("Bursa", "Marmara", 1, true)));
    }

    #[test]
    fn conditions_are_combined() {
        let schema = schema();
        let mut query = Query {
            conditions: vec![
                condition(1, Operator::Equals, "marmara"),
                condition(2, Operator::Greater, "5.000.000"),
            ],
            combinator: Combinator::All,
        };

        assert!(query.matches(&schema, &city("İstanbul", "Marmara", 15_840_900, true)));
        assert!(!query.matches(&schema, &city("Bursa", "Marmara", 3_214_600, true)));
        assert!(!query.matches(&schema, &city("İzmir", "Ege", 4_479_500, true)));

        query.combinator = Combinator::Any;
        assert!(query.matches(&schema, &city("Bursa", "Marmara", 3_214_600, true)));
        assert!(!query.matches(&schema, &city("Antalya", "Akdeniz", 2_696_000, true)));
    }

    #[test]
    fn text_search_ignores_case_and_turkish_letters() {
        let schema = schema();
        let query = Query {
            conditions: vec![condition(0, Operator::StartsWith, "iz")],
            ..Query::default()
        };

        assert!(query.matches(&schema, &city("İzmir", "Ege", 1, true)));
        assert!(!query.matches(&schema, &city("Bursa", "Marmara", 1, true)));
    }

    #[test]
    fn nulls_only_match_null_operators() {
        let schema = schema();
        let record = city("Ankara", "Marmara", 1, false);

        let before = Query {
            conditions: vec![condition(4, Operator::Less, "01.01.2030")],
            ..Query::default()
        };
        let missing = Query {
            conditions: vec![condition(4, Operator::IsNull, "")],
            ..Query::default()
        };

        assert!(!before.matches(&schema, &record));
        assert!(missing.matches(&schema, &record));

        let mut dated = record.clone();
        dated[4] = Value::Date(Date::new(2026, 9, 24).expect("tarih"));
        assert!(before.matches(&schema, &dated));
    }

    #[test]
    fn unparsable_values_are_reported_and_match_nothing() {
        let schema = schema();
        let query = Query {
            conditions: vec![condition(2, Operator::Greater, "çok")],
            ..Query::default()
        };

        assert_eq!(query.errors(&schema).len(), 1);
        assert!(!query.matches(&schema, &city("İstanbul", "Marmara", 1, true)));
    }

    #[test]
    fn edits_keep_operators_valid() {
        let schema = schema();
        let mut query = Query::default();

        query.apply(Edit::Add, &schema);
        query.apply(Edit::Operator(0, Operator::Contains), &schema);
        query.apply(Edit::Value(0, "ist".to_owned()), &schema);
        query.apply(Edit::Field(0, 2), &schema);

        assert_eq!(query.conditions[0].field, 2);
        assert_eq!(query.conditions[0].operator, Operator::Equals);
        assert!(query.conditions[0].value.is_empty());

        query.apply(Edit::Value(0, "100".to_owned()), &schema);
        assert_eq!(query.describe(&schema), "Nüfus = 100");
    }
}
