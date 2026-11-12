//! The limit a map work item's shift since its approval is set against (plan/map-cad.md MAP-01, docs/adr/5280): the
//! user's own value or an official one the user picks: a detail's position (BÖHHBÜY m. 46(1)), a point's place on the
//! drawing at the scale (m. 78(1)(h)) or the point position accuracy of a production method at a scale (the error
//! limit regulation's m. 9(1) table). The values are data (`catalog/v1/rules/konum-dogrulugu-2022.json`), not
//! constants in code. Being within a limit is a measure, not a ruling that a change is no technical error.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The largest own limit, in metres.
pub const MOST_SHIFT_LIMIT: f64 = 1000.0;
/// The largest scale a drawing limit takes (1/1 000 000).
pub const MOST_LIMIT_SCALE: u32 = 1_000_000;

/// What a shift limit rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum LimitBasis {
    /// The user's own value (`metres`).
    Own,
    /// A detail's position: BÖHHBÜY m. 46(1).
    Detail,
    /// A point's place on the drawing at `scale`: BÖHHBÜY m. 78(1)(h).
    Drawing,
    /// The point position accuracy of `method` at `scale`: the error limit regulation's m. 9(1) table.
    Method,
}

/// Input of a shift limit: its basis, and what the basis takes: `metres` (own), `scale` (drawing, method), `method`
/// (method: a table's method id).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShiftLimit {
    pub basis: LimitBasis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub metres: Option<f64>,
    /// The scale's denominator (1000 for 1/1000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub method: Option<String>,
}

/// The limit a query used: its value and what it rests on, in words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ShiftLimitUsed {
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub metres: f64,
    pub source: String,
}

/// A value the rules give, and where.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitRule {
    #[serde(default)]
    pub metres: Option<f64>,
    #[serde(default)]
    pub mm: Option<f64>,
    pub source: String,
    pub words: String,
}

/// A production method's row of the table: its id, its title and its value at each of the table's scales (none: the
/// table gives none).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitMethod {
    pub id: String,
    pub title: String,
    pub values: Vec<Option<f64>>,
}

/// The m. 9(1) table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitTable {
    pub source: String,
    pub read: String,
    pub scales: Vec<u32>,
    pub methods: Vec<LimitMethod>,
}

/// The rules file: the detail's value, the drawing's and the table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionAccuracy {
    pub id: String,
    pub release: u32,
    pub jurisdiction: String,
    pub detail: LimitRule,
    pub drawing: LimitRule,
    pub table: LimitTable,
}

/// The position accuracy rules KentOS carries.
pub fn position_accuracy() -> Option<PositionAccuracy> {
    serde_json::from_str(include_str!("../../../../catalog/v1/rules/konum-dogrulugu-2022.json")).ok()
}

/// Thousands grouped by a dot, as Turkish writes a scale (1/10.000).
fn scale_words(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    format!("1/{out}")
}

/// A limit's value in metres and its source in words; else the input's path and why it does not hold.
pub fn limit_metres(limit: &ShiftLimit) -> Result<ShiftLimitUsed, (String, String)> {
    let rules = position_accuracy().ok_or_else(|| ("limit".to_owned(), "Konum doğruluğu kuralları okunamadı.".to_owned()))?;
    let scale = |what: &str| -> Result<u32, (String, String)> {
        match limit.scale {
            Some(s) if (1..=MOST_LIMIT_SCALE).contains(&s) => Ok(s),
            Some(s) => Err(("limit.scale".into(), format!("Ölçek paydası {s}: 1 ile {MOST_LIMIT_SCALE} arasında olmalı."))),
            None => Err(("limit.scale".into(), format!("{what} için ölçek gerekir (1000: 1/1.000)."))),
        }
    };
    match limit.basis {
        LimitBasis::Own => match limit.metres {
            Some(m) if m.is_finite() && m > 0.0 && m <= MOST_SHIFT_LIMIT => {
                Ok(ShiftLimitUsed { metres: m, source: "kullanıcının değeri".into() })
            }
            Some(m) => Err((
                "limit.metres".into(),
                format!("Sınır {m} m: 0'dan büyük, en çok {MOST_SHIFT_LIMIT} m olmalı."),
            )),
            None => Err(("limit.metres".into(), "Kendi sınırınız için metres verin (ör. 0,05).".into())),
        },
        LimitBasis::Detail => {
            let m = rules.detail.metres.unwrap_or_default();
            Ok(ShiftLimitUsed { metres: m, source: format!("{}: {}", rules.detail.source, rules.detail.words) })
        }
        LimitBasis::Drawing => {
            let s = scale("Çizimde nokta konumu")?;
            let mm = rules.drawing.mm.unwrap_or_default();
            Ok(ShiftLimitUsed {
                metres: mm * f64::from(s) / 1000.0,
                source: format!("{}: {}, {}", rules.drawing.source, rules.drawing.words, scale_words(s)),
            })
        }
        LimitBasis::Method => {
            let ids: Vec<&str> = rules.table.methods.iter().map(|m| m.id.as_str()).collect();
            let Some(id) = limit.method.as_deref() else {
                return Err(("limit.method".into(), format!("Üretim yöntemini verin: {}.", ids.join(", "))));
            };
            let Some(row) = rules.table.methods.iter().find(|m| m.id == id) else {
                return Err(("limit.method".into(), format!("“{id}” yöntemi tabloda yok; yöntemler: {}.", ids.join(", "))));
            };
            let s = scale("Üretim yöntemi")?;
            let Some(k) = rules.table.scales.iter().position(|x| *x == s) else {
                let have: Vec<String> = rules.table.scales.iter().map(|x| scale_words(*x)).collect();
                return Err((
                    "limit.scale".into(),
                    format!(
                        "{} tabloda yok (tablonun ölçekleri {}); tabloda olmayan ölçeğin değerini Genel Müdürlük belirler (m. 9(3)).",
                        scale_words(s),
                        have.join(", ")
                    ),
                ));
            };
            let Some(m) = row.values.get(k).copied().flatten() else {
                return Err((
                    "limit.scale".into(),
                    format!("Tablo {} için “{}” değeri vermiyor.", scale_words(s), row.title),
                ));
            };
            Ok(ShiftLimitUsed {
                metres: m,
                source: format!("{}: {}, {}", rules.table.source, row.title, scale_words(s)),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn method(id: &str, scale: u32) -> Result<ShiftLimitUsed, (String, String)> {
        limit_metres(&ShiftLimit { basis: LimitBasis::Method, metres: None, scale: Some(scale), method: Some(id.into()) })
    }

    #[test]
    fn the_rules_read_and_every_row_has_a_value_per_scale() {
        let r = position_accuracy().expect("rules");
        assert_eq!(r.table.scales.len(), 10);
        assert!(r.table.methods.iter().all(|m| m.values.len() == r.table.scales.len()));
    }

    #[test]
    fn the_table_gives_its_cells_and_refuses_an_empty_one_and_another_scale() {
        assert_eq!(method("prizmatik-pafta", 3000).expect("cell").metres, 1.11);
        assert_eq!(method("sayisal-fotogrametrik", 1000).expect("cell").metres, 0.09);
        assert_eq!(method("fotogrametrik", 1000).expect_err("empty").0, "limit.scale");
        assert_eq!(method("sayisal", 1500).expect_err("not in the table").0, "limit.scale");
        assert_eq!(method("yok", 1000).expect_err("unknown").0, "limit.method");
    }

    #[test]
    fn the_drawing_limit_is_its_millimetres_at_the_scale() {
        let l = limit_metres(&ShiftLimit { basis: LimitBasis::Drawing, metres: None, scale: Some(1000), method: None });
        assert_eq!(l.expect("drawing").metres, 0.2);
        assert_eq!(scale_words(10000), "1/10.000");
    }
}
