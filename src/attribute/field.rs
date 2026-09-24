//! Alanlar: bir öznitelik sütununun adı, türü ve kısıtları.

use super::time::{Date, DateTime, Time};
use super::value::{ObjectId, Value};
use super::{number, text};

/// Alanın türü ve değer kısıtı. ArcGIS'teki alan türleri ve etki alanlarına
/// (domain) karşılık gelir.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Text,
    /// Tam sayı; isteğe bağlı alt ve üst sınırla.
    Integer {
        min: Option<i64>,
        max: Option<i64>,
    },
    /// Ondalık sayı; gösterimdeki basamak sayısıyla.
    Real {
        decimals: u8,
    },
    Bool,
    /// Kodlu değer alanı: yalnızca listedeki seçenekler.
    Choice(Vec<String>),
    /// Aralık alanı: alt ve üst sınır arasında, adımlı ondalık sayı.
    Range {
        min: f64,
        max: f64,
        step: f64,
    },
    Date,
    Time,
    DateTime,
    /// Başka bir katman ya da tablodaki nesneye başvuru.
    Object {
        /// Hedef katmanın ya da tablonun adı.
        target: String,
    },
}

/// Öznitelik alanı.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub kind: FieldKind,
    /// Sayısal değerlerin birimi (ör. "km/sa").
    pub unit: Option<String>,
    /// Kullanıcı değiştirebilir mi.
    pub editable: bool,
    /// Boş bırakılamaz mı.
    pub required: bool,
}

impl Field {
    pub fn new(name: impl Into<String>, kind: FieldKind) -> Self {
        Self {
            name: name.into(),
            kind,
            unit: None,
            editable: true,
            required: false,
        }
    }

    pub fn text(name: impl Into<String>) -> Self {
        Self::new(name, FieldKind::Text)
    }

    pub fn integer(name: impl Into<String>) -> Self {
        Self::new(
            name,
            FieldKind::Integer {
                min: None,
                max: None,
            },
        )
    }

    pub fn real(name: impl Into<String>, decimals: u8) -> Self {
        Self::new(name, FieldKind::Real { decimals })
    }

    pub fn boolean(name: impl Into<String>) -> Self {
        Self::new(name, FieldKind::Bool)
    }

    pub fn choice(
        name: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::new(
            name,
            FieldKind::Choice(options.into_iter().map(Into::into).collect()),
        )
    }

    pub fn range(name: impl Into<String>, min: f64, max: f64, step: f64) -> Self {
        Self::new(name, FieldKind::Range { min, max, step })
    }

    pub fn date(name: impl Into<String>) -> Self {
        Self::new(name, FieldKind::Date)
    }

    pub fn time(name: impl Into<String>) -> Self {
        Self::new(name, FieldKind::Time)
    }

    pub fn datetime(name: impl Into<String>) -> Self {
        Self::new(name, FieldKind::DateTime)
    }

    pub fn object(name: impl Into<String>, target: impl Into<String>) -> Self {
        Self::new(
            name,
            FieldKind::Object {
                target: target.into(),
            },
        )
    }

    /// Tam sayı alanının sınırları.
    pub fn between(mut self, min: i64, max: i64) -> Self {
        if let FieldKind::Integer { .. } = self.kind {
            self.kind = FieldKind::Integer {
                min: Some(min),
                max: Some(max),
            };
        }

        self
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn read_only(mut self) -> Self {
        self.editable = false;
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Sayısal mı (sağa hizalı ve eş aralıklı gösterilir).
    pub fn is_numeric(&self) -> bool {
        matches!(
            self.kind,
            FieldKind::Integer { .. } | FieldKind::Real { .. } | FieldKind::Range { .. }
        )
    }

    /// Değeri gösterim için yazar. Boş değer boş metindir; nesne
    /// başvuruları "#12" olarak yazılır.
    pub fn format(&self, value: &Value) -> String {
        match (&self.kind, value) {
            (_, Value::Null) => String::new(),
            (_, Value::Bool(value)) => if *value { "Evet" } else { "Hayır" }.to_owned(),
            (_, Value::Integer(value)) => number::integer(*value),
            (FieldKind::Real { decimals }, Value::Real(value)) => {
                number::real(*value, usize::from(*decimals))
            }
            (FieldKind::Range { step, .. }, Value::Real(value)) => {
                number::real(*value, number::decimals_of(*step))
            }
            (_, Value::Real(value)) => number::real(*value, 2),
            (_, Value::Text(value)) => value.clone(),
            (_, Value::Date(date)) => date.to_string(),
            (_, Value::Time(time)) => time.to_string(),
            (_, Value::DateTime(moment)) => moment.to_string(),
            (_, Value::Object(id)) => format!("#{id}"),
        }
    }

    /// Değeri birimiyle yazar: "15.840.900 kişi".
    pub fn format_with_unit(&self, value: &Value) -> String {
        let formatted = self.format(value);

        match &self.unit {
            Some(unit) if !formatted.is_empty() => format!("{formatted} {unit}"),
            _ => formatted,
        }
    }

    /// Metin girişi için örnek biçim.
    pub fn placeholder(&self) -> &'static str {
        match self.kind {
            FieldKind::Date => "GG.AA.YYYY",
            FieldKind::Time => "SS:DD",
            FieldKind::DateTime => "GG.AA.YYYY SS:DD",
            FieldKind::Integer { .. } => "0",
            FieldKind::Real { .. } | FieldKind::Range { .. } => "0,0",
            FieldKind::Object { .. } => "#1",
            _ => "",
        }
    }

    /// Metni alanın türüne göre çözümler ve kısıtlarını denetler. Boş metin
    /// boş değerdir; zorunlu alanda hatadır.
    pub fn parse(&self, input: &str) -> Result<Value, String> {
        let input = input.trim();

        if input.is_empty() {
            return if self.required {
                Err("Bu alan boş bırakılamaz.".to_owned())
            } else {
                Ok(Value::Null)
            };
        }

        let value = match &self.kind {
            FieldKind::Text => Value::Text(input.to_owned()),
            FieldKind::Integer { .. } => {
                Value::Integer(number::parse_integer(input).ok_or("Tam sayı girin, ör. 42.")?)
            }
            FieldKind::Real { .. } | FieldKind::Range { .. } => {
                Value::Real(number::parse_real(input).ok_or("Sayı girin, ör. 12,5.")?)
            }
            FieldKind::Bool => match text::fold(input).as_str() {
                "evet" | "e" | "dogru" | "true" | "1" => Value::Bool(true),
                "hayir" | "h" | "yanlis" | "false" | "0" => Value::Bool(false),
                _ => return Err("Evet ya da Hayır girin.".to_owned()),
            },
            FieldKind::Choice(options) => options
                .iter()
                .find(|option| text::fold(option) == text::fold(input))
                .map(|option| Value::Text(option.clone()))
                .ok_or("Listedeki seçeneklerden birini girin.")?,
            FieldKind::Date => {
                Value::Date(Date::parse(input).ok_or("Tarihi GG.AA.YYYY biçiminde girin.")?)
            }
            FieldKind::Time => {
                Value::Time(Time::parse(input).ok_or("Saati SS:DD biçiminde girin.")?)
            }
            FieldKind::DateTime => {
                Value::DateTime(DateTime::parse(input).ok_or("GG.AA.YYYY SS:DD biçiminde girin.")?)
            }
            FieldKind::Object { .. } => Value::Object(ObjectId(
                input
                    .trim_start_matches('#')
                    .parse()
                    .map_err(|_| "Nesne numarası girin, ör. #12.")?,
            )),
        };

        self.validate(&value).map(|()| value)
    }

    /// Değerin alanın kısıtlarına uygunluğu.
    pub fn validate(&self, value: &Value) -> Result<(), String> {
        if value.is_null() {
            return if self.required {
                Err("Bu alan boş bırakılamaz.".to_owned())
            } else {
                Ok(())
            };
        }

        let out_of_range =
            |min: String, max: String| format!("Değer {min} ile {max} arasında olmalı.");

        match (&self.kind, value) {
            (FieldKind::Integer { min, max }, Value::Integer(value)) => {
                let below = min.is_some_and(|min| *value < min);
                let above = max.is_some_and(|max| *value > max);

                if below || above {
                    return Err(out_of_range(
                        min.map_or("-∞".to_owned(), number::integer),
                        max.map_or("∞".to_owned(), number::integer),
                    ));
                }
            }
            (FieldKind::Range { min, max, step }, Value::Real(value)) => {
                if value < min || value > max {
                    let decimals = number::decimals_of(*step);
                    return Err(out_of_range(
                        number::real(*min, decimals),
                        number::real(*max, decimals),
                    ));
                }
            }
            (FieldKind::Choice(options), Value::Text(value)) if !options.contains(value) => {
                return Err("Listedeki seçeneklerden biri olmalı.".to_owned());
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_formatted_by_kind() {
        let population = Field::integer("Nüfus").unit("kişi");
        assert_eq!(
            population.format_with_unit(&Value::Integer(15_840_900)),
            "15.840.900 kişi"
        );

        let height = Field::real("Yükseklik", 2);
        assert_eq!(height.format(&Value::Real(1234.5)), "1.234,50");

        let speed = Field::range("Hız", 30.0, 140.0, 10.0);
        assert_eq!(speed.format(&Value::Real(90.0)), "90");

        assert_eq!(Field::boolean("Kıyı").format(&Value::Bool(true)), "Evet");
        assert_eq!(Field::text("Not").format(&Value::Null), "");
    }

    #[test]
    fn input_is_parsed_and_validated() {
        let plate = Field::integer("Plaka").between(1, 81);
        assert_eq!(plate.parse("34"), Ok(Value::Integer(34)));
        assert!(plate.parse("82").is_err());
        assert!(plate.parse("otuz dört").is_err());
        assert_eq!(plate.parse(""), Ok(Value::Null));

        let name = Field::text("Ad").required();
        assert!(name.parse("  ").is_err());

        let region = Field::choice("Bölge", ["Marmara", "Ege"]);
        assert_eq!(region.parse("marmara"), Ok(Value::from("Marmara")));
        assert!(region.parse("Akdeniz").is_err());

        let coastal = Field::boolean("Kıyı");
        assert_eq!(coastal.parse("Hayır"), Ok(Value::Bool(false)));
        assert_eq!(coastal.parse("EVET"), Ok(Value::Bool(true)));

        let speed = Field::range("Hız", 30.0, 140.0, 10.0);
        assert_eq!(speed.parse("90"), Ok(Value::Real(90.0)));
        assert!(speed.parse("150").is_err());

        assert!(Field::date("Tarih").parse("24.09.2026").is_ok());
        assert!(Field::date("Tarih").parse("2026").is_err());
        assert_eq!(
            Field::object("Şehir", "Şehirler").parse("#12"),
            Ok(Value::Object(ObjectId(12)))
        );
    }
}
