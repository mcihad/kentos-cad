//! A drawing's permit facts: the `permit` section of `kentos.auditFacts` 1 (docs/adr/8063). The particulars the dated
//! scope decision asks (Dijital Hazırlama Yönetmeliği Md. 1, Geçici 1–2; e-PYS Md. 1, 4/4), as the drawing's variables
//! hold them, and what the drawing measures for it: the building's yapı inşaat alanı (Planlı Alanlar İmar Yönetmeliği
//! 4/oooo) and its storeys above and below ±0.00. The decision is a Python rule's (plan/bim-plan.md K10): the facts say
//! what is written and measured, never what it means.
//!
//! An optional section of version 1, as [`super::architecture`]: absent when none of the particulars has a value, so the
//! facts of a drawing that never asked stay byte for byte what they were. The particulars by name (a map's keys in
//! order), each value trimmed and at most [`MOST_TEXT`] characters; the area to 0.0001 m².

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{AREA, round};

/// The most characters a particular's value keeps.
pub const MOST_TEXT: usize = 200;

/// The permit's particulars and the drawing's measures for them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PermitFacts {
    /// The scope decision's particulars that have a value, by their variable's name (`basvuru_tarihi`, `il_nufusu`,
    /// `kullanim` …, kentos_digital_project::scope): the text as the user wrote it.
    pub particulars: BTreeMap<String, String>,
    /// The building's yapı inşaat alanı, m²: every room's gross area on every storey; none without a room.
    pub construction_area: Option<f64>,
    /// The storeys holding rooms; none when no room stands on a storey of the project.
    pub storeys: Option<StoreyCount>,
}

/// The storeys holding rooms, by their place as the sheets' storey codes read it (EK-2 Tablo 2.14): at or above ±0.00
/// (K00, K01 …) and below it (B01, B02 …, the basements).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StoreyCount {
    pub above: u32,
    pub below: u32,
}

impl PermitFacts {
    /// These facts with their values trimmed and bounded, the empty ones out and the area rounded. Doing it again
    /// changes nothing.
    pub fn normalized(mut self) -> PermitFacts {
        self.particulars = std::mem::take(&mut self.particulars)
            .into_iter()
            .filter_map(|(name, value)| {
                let value: String = value.trim().chars().take(MOST_TEXT).collect();
                let value = value.trim_end().to_owned();
                (!name.is_empty() && !value.is_empty()).then_some((name, value))
            })
            .collect();
        self.construction_area = self.construction_area.map(|a| round(a, AREA));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_trimmed_bounded_and_the_empty_ones_left_out() {
        let facts = PermitFacts {
            particulars: [
                ("kullanim", "  konut "),
                ("belediye", ""),
                ("nufus_kaynagi", &"x".repeat(MOST_TEXT + 5)),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
            construction_area: Some(5_000.000_049),
            storeys: Some(StoreyCount { above: 3, below: 1 }),
        }
        .normalized();
        assert_eq!(facts.particulars.get("kullanim").map(String::as_str), Some("konut"));
        assert!(!facts.particulars.contains_key("belediye"));
        assert_eq!(facts.particulars["nufus_kaynagi"].chars().count(), MOST_TEXT);
        assert_eq!(facts.construction_area, Some(5000.0));
        assert_eq!(facts.clone().normalized(), facts);
        let json = serde_json::to_string(&facts).unwrap_or_default();
        assert!(json.contains(r#""constructionArea":5000.0"#) && json.contains(r#""storeys":{"above":3,"below":1}"#), "{json}");
    }
}
