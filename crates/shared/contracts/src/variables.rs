//! Projenin değişkenleri (docs/adr/0214 §2.3): `@name` values of the
//! expression language that the project keeps (an order number, a client,
//! a coefficient), beside the built-in ones every host gives (the project's
//! name, its coordinate system, its scale, the date, the user). Their shape
//! is the sheet's variables' (docs/adr/0164); the rules here are the
//! readers', the settings windows' and the server's.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The most variables a project keeps.
pub const VARIABLES_MAX: usize = 200;
/// The longest name and the longest text value, in characters.
pub const VARIABLE_NAME_MAX: usize = 64;
pub const VARIABLE_TEXT_MAX: usize = 4_000;

/// What a variable holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum VariableKind {
    #[default]
    Text,
    Number,
    Bool,
    /// ISO text (YYYY-AA-GG).
    Date,
}

/// A variable's value: none yet, true/false, a number or text.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
#[cfg_attr(feature = "ts", ts(export))]
pub enum VariableValue {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
}

/// One of the project's variables.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectVariable {
    /// Without the `@`: a letter or `_`, then letters, digits and `_` (`is_no`).
    pub name: String,
    /// What the settings window shows beside it (“İş numarası”).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<String>"))]
    pub label: String,
    #[serde(default)]
    pub kind: VariableKind,
    /// None yet: written as no value (an empty `@ad`), as the file keeps it.
    #[serde(default, skip_serializing_if = "VariableValue::is_null")]
    #[cfg_attr(feature = "ts", ts(optional, as = "Option<VariableValue>"))]
    pub value: VariableValue,
}

impl VariableValue {
    pub fn is_null(&self) -> bool {
        matches!(self, VariableValue::Null)
    }
}

/// The built-in variables every host gives, with what they hold: a
/// project's variable may not take one of these names.
pub const BUILTIN_VARIABLES: [(&str, &str); 9] = [
    ("proje_adi", "Projenin adı"),
    ("koordinat_sistemi", "Koordinat sisteminin adı"),
    ("epsg", "Koordinat sisteminin EPSG kodu"),
    ("olcek", "Çizim ölçeğinin paydası"),
    ("tarih", "Bugünün tarihi"),
    ("simdi", "Şimdiki tarih ve saat"),
    ("katman_adi", "Değerlendirilen nesnenin katmanının adı"),
    ("katman", "Değerlendirilen nesnenin katmanının adı"),
    ("kullanici", "Oturumdaki kullanıcının adı"),
];

/// A name as it is looked for: Turkish letters and case aside (`Proje_Adı` is `proje_adi`).
pub fn variable_key(name: &str) -> String {
    name.chars()
        .flat_map(|c| match c {
            'ı' | 'I' | 'i' | 'İ' => vec!['I'],
            'ğ' | 'Ğ' => vec!['G'],
            'ü' | 'Ü' => vec!['U'],
            'ş' | 'Ş' => vec!['S'],
            'ö' | 'Ö' => vec!['O'],
            'ç' | 'Ç' => vec!['C'],
            c => c.to_uppercase().collect(),
        })
        .collect()
}

/// Whether a name may be a variable's: a letter or `_`, then letters, digits and `_`.
pub fn variable_name_problem(name: &str) -> Option<String> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Some("değişkenin adı boş".into());
    };
    if name.chars().count() > VARIABLE_NAME_MAX {
        return Some(format!(
            "“{name}” değişken adı {VARIABLE_NAME_MAX} karakterden uzun"
        ));
    }
    if !(first.is_alphabetic() || first == '_') || !chars.all(|c| c.is_alphanumeric() || c == '_') {
        return Some(format!(
            "“{name}” değişken adı olamaz: bir harf ya da _ ile başlar, harf, rakam ve _ içerir"
        ));
    }
    let key = variable_key(name);
    if BUILTIN_VARIABLES
        .iter()
        .any(|(builtin, _)| variable_key(builtin) == key)
    {
        return Some(format!("@{name} yerleşik bir değişkendir; başka ad seçin"));
    }
    None
}

impl ProjectVariable {
    /// What is wrong with it, when anything is.
    pub fn problem(&self) -> Option<String> {
        if let Some(p) = variable_name_problem(&self.name) {
            return Some(p);
        }
        let name = &self.name;
        match (&self.kind, &self.value) {
            (_, VariableValue::Null) => None,
            (VariableKind::Number, VariableValue::Number(x)) if x.is_finite() => None,
            (VariableKind::Number, _) => {
                Some(format!("@{name} sayı değişkeni; değeri sayı olmalı"))
            }
            (VariableKind::Bool, VariableValue::Bool(_)) => None,
            (VariableKind::Bool, _) => Some(format!(
                "@{name} doğru/yanlış değişkeni; değeri doğru ya da yanlış olmalı"
            )),
            (VariableKind::Text, VariableValue::Text(t)) => (t.chars().count() > VARIABLE_TEXT_MAX)
                .then(|| format!("@{name} değeri {VARIABLE_TEXT_MAX} karakterden uzun")),
            (VariableKind::Text, _) => {
                Some(format!("@{name} metin değişkeni; değeri metin olmalı"))
            }
            (VariableKind::Date, VariableValue::Text(t)) if is_date(t) => None,
            (VariableKind::Date, _) => Some(format!(
                "@{name} tarih değişkeni; değeri YYYY-AA-GG biçiminde bir tarih olmalı"
            )),
        }
    }
}

/// `YYYY-AA-GG`, a day of the calendar (years 1–9999).
pub fn is_date(t: &str) -> bool {
    let b = t.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |s: &str| {
        (s.bytes().all(|c| c.is_ascii_digit()))
            .then(|| s.parse::<u32>().ok())
            .flatten()
    };
    let (Some(y), Some(m), Some(d)) = (num(&t[0..4]), num(&t[5..7]), num(&t[8..10])) else {
        return false;
    };
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match m {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return false,
    };
    (1..=9999).contains(&y) && (1..=days).contains(&d)
}

/// What is wrong with a project's variables, when anything is: a broken
/// one, two of one name, too many.
pub fn variables_problem(list: &[ProjectVariable]) -> Option<String> {
    if list.len() > VARIABLES_MAX {
        return Some(format!(
            "projede {} değişken var; en çok {VARIABLES_MAX}",
            list.len()
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for v in list {
        if let Some(p) = v.problem() {
            return Some(p);
        }
        if !seen.insert(variable_key(&v.name)) {
            return Some(format!("@{} adında iki değişken var", v.name));
        }
    }
    None
}

/// The variables as a project keeps them: of those with one name the first,
/// none that breaks a rule, at most [`VARIABLES_MAX`].
pub fn sanitized_variables(list: Vec<ProjectVariable>) -> Vec<ProjectVariable> {
    let mut seen = std::collections::HashSet::new();
    list.into_iter()
        .filter(|v| v.problem().is_none() && seen.insert(variable_key(&v.name)))
        .take(VARIABLES_MAX)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, kind: VariableKind, value: VariableValue) -> ProjectVariable {
        ProjectVariable {
            name: name.into(),
            label: String::new(),
            kind,
            value,
        }
    }

    #[test]
    fn variables_keep_their_rules() {
        let ok = [
            var(
                "is_no",
                VariableKind::Text,
                VariableValue::Text("2026/41".into()),
            ),
            var("Katsayı", VariableKind::Number, VariableValue::Number(1.5)),
            var(
                "teslim",
                VariableKind::Date,
                VariableValue::Text("2026-02-28".into()),
            ),
            var("boş_olan", VariableKind::Text, VariableValue::Null),
        ];
        assert_eq!(variables_problem(&ok), None);
        let twice = [
            ok[0].clone(),
            var("IS_NO", VariableKind::Text, VariableValue::Null),
        ];
        assert!(variables_problem(&twice).is_some_and(|p| p.contains("iki")));
        assert!(
            var("1a", VariableKind::Text, VariableValue::Null)
                .problem()
                .is_some()
        );
        assert!(
            var("proje_adı", VariableKind::Text, VariableValue::Null)
                .problem()
                .is_some()
        );
        assert!(
            var("x", VariableKind::Number, VariableValue::Text("1".into()))
                .problem()
                .is_some()
        );
        assert!(
            var(
                "d",
                VariableKind::Date,
                VariableValue::Text("2026-02-29".into())
            )
            .problem()
            .is_some()
        );
        assert_eq!(variable_key("Proje_Adı"), variable_key("PROJE_ADI"));
        let json = serde_json::to_string(&ok[1]).expect("JSON");
        assert_eq!(json, r#"{"name":"Katsayı","kind":"number","value":1.5}"#);
    }
}
