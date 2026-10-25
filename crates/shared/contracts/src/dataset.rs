//! A layer as a dataset (docs/adr/0633): what it is (its title, description, responsible party, license and use
//! constraints, keywords, the day its data was last brought up to date, the scale or resolution it was made for, its
//! positional accuracy and completeness, how it was made) and, for a layer a processing tool made, what it was derived
//! from (the tool, its values, the layers it read and their content digests, the engine, when). Its extent, system,
//! object counts and data dictionary are worked out from the drawing whenever asked, never stored. The rules here are
//! the readers', the commands' and the server's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "ts")]
use ts_rs::TS;

/// The longest title, party name, license and keyword, in characters.
pub const MAX_DATASET_LINE: usize = 200;
/// The longest description, constraint and statement, in characters.
pub const MAX_DATASET_TEXT: usize = 10_000;
/// The most keywords.
pub const MAX_DATASET_KEYWORDS: usize = 50;
/// The most inputs a derivation names.
pub const MAX_DERIVATION_INPUTS: usize = 200;

/// Who answers for the data (ISO 19115's responsible party, OGC API Records' contact).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetParty {
    /// A person's or a unit's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub organisation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub email: Option<String>,
}

/// A layer's description as a dataset; every part may be left out, an empty one is not written.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DatasetInfo {
    /// Its title; the layer's name stands for it when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub responsible: Option<DatasetParty>,
    /// A license's name or address (`CC-BY-4.0`, `https://…`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub license: Option<String>,
    /// What its use is bound by (Kullanım kısıtı).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub constraints: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub keywords: Vec<String>,
    /// The day its data was last brought up to date (`YYYY-AA-GG`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub updated: Option<String>,
    /// The scale it was made for: the denominator (1000 for 1:1000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "1")))]
    pub scale: Option<u32>,
    /// Its resolution: a raster's cell or the smallest detail it keeps, in metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub resolution: Option<f64>,
    /// Its positional accuracy (one standard deviation), in metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "m")))]
    pub accuracy: Option<f64>,
    /// How complete it is, in words (“ilçenin bütün parselleri”).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub completeness: Option<String>,
    /// How it was made (Üretim açıklaması): survey, digitising, import.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub statement: Option<String>,
}

impl DatasetInfo {
    /// Whether nothing is said: such a description is not kept.
    pub fn is_empty(&self) -> bool {
        *self == DatasetInfo::default()
    }
}

/// A layer a derivation read: the layer then (its id and name), its content digest when read
/// (`sources::layer_digest`, docs/adr/0511), and for a bound layer its source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct DerivationInput {
    pub layer: String,
    pub name: String,
    /// SHA-256, 64 lowercase hex digits.
    pub digest: String,
    /// The file or database address (without a password) a bound layer's table is in, and its table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub source: Option<String>,
}

/// What a layer a processing tool made was derived from (docs/adr/0633 §2): written by the run, in its undo step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct LayerDerivation {
    /// The tool's id and its name.
    pub tool: String,
    pub label: String,
    /// The tool's version (what it computes from the same values).
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub tool_version: u32,
    /// The values it took, but its features inputs (they are `inputs`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(
        feature = "ts",
        ts(type = "Record<string, unknown> | undefined", optional)
    )]
    #[cfg_attr(
        feature = "schema",
        schemars(with = "BTreeMap<String, serde_json::Value>")
    )]
    pub parameters: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<DerivationInput>>", optional))]
    pub inputs: Vec<DerivationInput>,
    /// The drawing's revision when it ran.
    pub document_revision: String,
    /// `KentOS <version>`.
    pub engine: String,
    /// When it ran, UTC (`2026-10-25T09:30:00Z`).
    pub at: String,
}

fn line_ok(t: &str, most: usize) -> bool {
    let n = t.chars().count();
    (1..=most).contains(&n) && t.trim() == t && !t.chars().any(char::is_control)
}

fn text_ok(t: &str) -> bool {
    let n = t.chars().count();
    (1..=MAX_DATASET_TEXT).contains(&n) && !t.trim().is_empty()
}

/// What is wrong with a dataset description; none when it holds.
pub fn dataset_problem(d: &DatasetInfo) -> Option<String> {
    if d.is_empty() {
        return Some("Veri kümesi bilgisi boş: hiçbir şey söylemeyen bilgi yazılmaz; silmek için boş bırakın.".into());
    }
    for (what, v) in [("başlığı", &d.title), ("lisansı", &d.license)] {
        if let Some(v) = v
            && !line_ok(v, MAX_DATASET_LINE)
        {
            return Some(format!(
                "Veri kümesinin {what} boş, baş ya da sonda boşluklu, {MAX_DATASET_LINE} karakterden uzun ya da denetim karakterli."
            ));
        }
    }
    for (what, v) in [
        ("açıklaması", &d.description),
        ("kullanım kısıtı", &d.constraints),
        ("tamlığı", &d.completeness),
        ("üretim açıklaması", &d.statement),
    ] {
        if let Some(v) = v
            && !text_ok(v)
        {
            return Some(format!(
                "Veri kümesinin {what} boş ya da {MAX_DATASET_TEXT} karakterden uzun."
            ));
        }
    }
    if let Some(p) = &d.responsible {
        if p.name.is_none() && p.organisation.is_none() && p.email.is_none() {
            return Some("Sorumlunun adı, kurumu ya da e-postası yazılmalı.".into());
        }
        for v in [&p.name, &p.organisation].into_iter().flatten() {
            if !line_ok(v, MAX_DATASET_LINE) {
                return Some(
                    "Sorumlunun adı ya da kurumu boş, baş ya da sonda boşluklu ya da çok uzun."
                        .into(),
                );
            }
        }
        if let Some(e) = &p.email {
            let ok = line_ok(e, MAX_DATASET_LINE)
                && !e.contains(char::is_whitespace)
                && e.split_once('@').is_some_and(|(u, h)| {
                    !u.is_empty() && h.contains('.') && !h.starts_with('.') && !h.ends_with('.')
                });
            if !ok {
                return Some(format!("“{e}” bir e-posta adresi değil."));
            }
        }
    }
    if d.keywords.len() > MAX_DATASET_KEYWORDS {
        return Some(format!(
            "Veri kümesinde en çok {MAX_DATASET_KEYWORDS} anahtar sözcük olur; {} verildi.",
            d.keywords.len()
        ));
    }
    for (i, k) in d.keywords.iter().enumerate() {
        if !line_ok(k, MAX_DATASET_LINE) {
            return Some(
                "Anahtar sözcüklerden biri boş, baş ya da sonda boşluklu ya da çok uzun.".into(),
            );
        }
        if d.keywords[..i].contains(k) {
            return Some(format!("“{k}” anahtar sözcüğü iki kez yazılmış."));
        }
    }
    if let Some(u) = &d.updated
        && !crate::variables::is_date(u)
    {
        return Some(format!(
            "“{u}” bir gün değil: güncelleme tarihi YYYY-AA-GG biçiminde yazılır."
        ));
    }
    if d.scale == Some(0) {
        return Some("Ölçeğin paydası sıfırdan büyük olur (1:1000 için 1000).".into());
    }
    for (what, v) in [
        ("çözünürlüğü", d.resolution),
        ("konum doğruluğu", d.accuracy),
    ] {
        if let Some(v) = v
            && !(v.is_finite() && v > 0.0)
        {
            return Some(format!(
                "Veri kümesinin {what} sıfırdan büyük bir metre değeri olur."
            ));
        }
    }
    None
}

fn hex64(t: &str) -> bool {
    t.len() == 64
        && t.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// What is wrong with a derivation record; none when it holds.
pub fn derivation_problem(d: &LayerDerivation) -> Option<String> {
    if d.tool.is_empty() || d.label.is_empty() || d.engine.is_empty() || d.at.is_empty() {
        return Some("Türetim kaydının aracı, adı, motoru ve zamanı yazılmalı.".into());
    }
    if d.document_revision.is_empty() || !d.document_revision.bytes().all(|b| b.is_ascii_digit()) {
        return Some("Türetim kaydının çizim sürümü bir sayı olur.".into());
    }
    if d.inputs.len() > MAX_DERIVATION_INPUTS {
        return Some(format!(
            "Türetim kaydı en çok {MAX_DERIVATION_INPUTS} girdi anar."
        ));
    }
    for (i, input) in d.inputs.iter().enumerate() {
        if input.layer.is_empty() || !hex64(&input.digest) {
            return Some(
                "Türetim kaydının bir girdisinin katmanı ya da özeti (64 haneli SHA-256) geçersiz."
                    .into(),
            );
        }
        if d.inputs[..i].iter().any(|x| x.layer == input.layer) {
            return Some(format!(
                "Türetim kaydı “{}” katmanını iki kez anıyor.",
                input.layer
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_holds_by_its_parts() {
        let ok = DatasetInfo {
            title: Some("Parseller".into()),
            responsible: Some(DatasetParty {
                email: Some("harita@belediye.gov.tr".into()),
                ..DatasetParty::default()
            }),
            updated: Some("2026-10-25".into()),
            scale: Some(1000),
            ..DatasetInfo::default()
        };
        assert_eq!(dataset_problem(&ok), None);
        assert!(dataset_problem(&DatasetInfo::default()).is_some());
        let mut bad = ok.clone();
        bad.updated = Some("25.10.2026".into());
        assert!(dataset_problem(&bad).unwrap().contains("YYYY-AA-GG"));
        let mut bad = ok.clone();
        bad.responsible = Some(DatasetParty {
            email: Some("harita".into()),
            ..DatasetParty::default()
        });
        assert!(dataset_problem(&bad).is_some());
        let mut bad = ok;
        bad.keywords = vec!["kadastro".into(), "kadastro".into()];
        assert!(dataset_problem(&bad).is_some());
    }
}
