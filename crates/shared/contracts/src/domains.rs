//! The project's value lists (docs/adr/0430, Değer listeleri): a coded list or a range kept once in the
//! project and linked from the fields of many layers. A linked field keeps a copy of its list's values
//! (or bounds), so every rule that reads a field reads it as before; the document writes the copies anew
//! whenever a list changes, in the same undo step. A field whose list is gone keeps its copy.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::fields::{FieldChoice, LayerField, LayerFieldKind, field_problem};
use crate::layer::LayerNode;

/// A project holds at most this many value lists.
pub const MOST_DOMAINS: usize = 200;
/// A list's id and name are at most this many characters; its description five hundred.
const MOST_NAME: usize = 64;
const MOST_DESCRIPTION: usize = 500;

/// A value list of the project: a coded list (`values`) or a range (`min`, `max`), never both.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct FieldDomain {
    /// Its stable id: what a field links to.
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub description: Option<String>,
    /// The kind of field it fits: a field linking it is of this kind.
    pub kind: LayerFieldKind,
    /// A coded list's values, in their order (labels, retired codes as a field's list has them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub values: Option<Vec<FieldChoice>>,
    /// A range's least value, in its canonical text; included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub min: Option<String>,
    /// A range's greatest value, in its canonical text; included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub max: Option<String>,
    /// `min` itself left out of the range (docs/adr/0438 §4); written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub min_exclusive: bool,
    /// `max` itself left out of the range; written only as `true`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "ts", ts(as = "Option<bool>", optional))]
    pub max_exclusive: bool,
    /// The unit a number list's values are in (`m²`; docs/adr/0438 §4), copied to the fields linking it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unit: Option<String>,
}

impl FieldDomain {
    /// Whether it is a coded list (else a range).
    pub fn coded(&self) -> bool {
        self.values.is_some()
    }

    /// `field` with this list's copy: its values or its bounds; the other two none.
    pub fn applied_to(&self, field: &LayerField) -> LayerField {
        LayerField {
            values: self.values.clone(),
            min: self.min.clone(),
            max: self.max.clone(),
            min_exclusive: self.min_exclusive,
            max_exclusive: self.max_exclusive,
            unit: self.unit.clone(),
            domain: Some(self.id.clone()),
            ..field.clone()
        }
    }

    /// A field of its own as the rules of a field's list see it.
    fn probe(&self) -> LayerField {
        LayerField {
            withheld: false,
            column: None,
            id: None,
            name: self.name.clone(),
            alias: None,
            kind: self.kind,
            length: None,
            scale: None,
            min: self.min.clone(),
            max: self.max.clone(),
            values: self.values.clone(),
            required: false,
            default: None,
            multiple: false,
            split: None,
            merge: None,
            domain: None,
            unique: false,
            min_exclusive: self.min_exclusive,
            max_exclusive: self.max_exclusive,
            unit: self.unit.clone(),
        }
    }
}

fn name_problem(text: &str, what: &str) -> Option<String> {
    let bad = text.trim().is_empty()
        || text.trim() != text
        || text.chars().count() > MOST_NAME
        || text.chars().any(char::is_control);
    bad.then(|| {
        format!(
            "Değer listesinin {what} boş olamaz, en çok {MOST_NAME} karakterdir, boşlukla başlayıp bitmez ve denetim karakteri içermez."
        )
    })
}

/// The first problem of one list, said in Turkish.
pub fn domain_problem(d: &FieldDomain) -> Option<String> {
    if let Some(p) = name_problem(&d.id, "kimliği") {
        return Some(p);
    }
    if let Some(p) = name_problem(&d.name, "adı") {
        return Some(p);
    }
    if let Some(t) = &d.description
        && (t.trim().is_empty() || t.chars().count() > MOST_DESCRIPTION)
    {
        return Some(format!(
            "“{}” listesinin açıklaması boş olamaz ve en çok {MOST_DESCRIPTION} karakterdir.",
            d.name
        ));
    }
    let ranged = d.min.is_some() || d.max.is_some();
    match (d.values.is_some(), ranged) {
        (true, true) => {
            return Some(format!(
                "“{}” listesi ya kod listesidir ya aralıktır; ikisi birden olamaz.",
                d.name
            ));
        }
        (false, false) => {
            return Some(format!(
                "“{}” listesinde ne kod var ne aralık; kod ekleyin ya da en azını veya en çoğunu verin.",
                d.name
            ));
        }
        _ => {}
    }
    if let Some(values) = &d.values
        && values.iter().all(|c| c.retired)
    {
        return Some(format!(
            "“{}” listesinin bütün kodları kullanımdan kalkmış; en az biri kullanımda olmalı.",
            d.name
        ));
    }
    field_problem(&d.probe()).map(|p| {
        p.replacen("alanının", "listesinin", 1)
            .replacen("alanında", "listesinde", 1)
    })
}

/// The first problem of the project's lists and where: the list's place in them.
pub fn domains_problem(domains: &[FieldDomain]) -> Option<(usize, String)> {
    if domains.len() > MOST_DOMAINS {
        return Some((
            MOST_DOMAINS,
            format!("Projede en çok {MOST_DOMAINS} değer listesi olur."),
        ));
    }
    for (i, d) in domains.iter().enumerate() {
        if let Some(p) = domain_problem(d) {
            return Some((i, p));
        }
        if domains[..i].iter().any(|o| o.id == d.id) {
            return Some((i, format!("“{}” kimlikli iki değer listesi var.", d.id)));
        }
        let folded = d.name.to_lowercase();
        if domains[..i].iter().any(|o| o.name.to_lowercase() == folded) {
            return Some((i, format!("“{}” adlı iki değer listesi var.", d.name)));
        }
    }
    None
}

/// Why `field` cannot link `domain`, said in Turkish: another kind, or a copy its own rules refuse (a
/// decimal's codes beyond its digits, a list's “;” in a field of several values).
pub fn link_problem(domain: &FieldDomain, field: &LayerField) -> Option<String> {
    let label = field.alias.as_deref().unwrap_or(&field.name);
    if field.kind != domain.kind {
        return Some(format!(
            "“{}” listesi {} alanına uyar; “{label}” {} alanı.",
            domain.name,
            domain.kind.label().to_lowercase(),
            field.kind.label().to_lowercase()
        ));
    }
    field_problem(&domain.applied_to(field))
}

/// `fields` with every linked field's copy written from its list (a list gone or not fitting: the copy
/// kept); whether anything changed.
pub fn follow_domains(fields: &[LayerField], domains: &[FieldDomain]) -> (Vec<LayerField>, bool) {
    let mut changed = false;
    let out = fields
        .iter()
        .map(|f| {
            let Some(d) = f
                .domain
                .as_ref()
                .and_then(|id| domains.iter().find(|d| d.id == *id))
            else {
                return f.clone();
            };
            if link_problem(d, f).is_some() {
                return f.clone();
            }
            let next = d.applied_to(f);
            changed |= next != *f;
            next
        })
        .collect();
    (out, changed)
}

/// The fields linking `id` in a layer tree: (layer id, field key), in the tree's order.
pub fn domain_uses(nodes: &[LayerNode], id: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    fn walk(nodes: &[LayerNode], id: &str, out: &mut Vec<(String, String)>) {
        for n in nodes {
            for f in &n.fields {
                if f.domain.as_deref() == Some(id) {
                    out.push((n.id.clone(), f.key().to_owned()));
                }
            }
            walk(&n.children, id, out);
        }
    }
    walk(nodes, id, &mut out);
    out
}

/// A fresh id for a list named `name`, not among `taken`: its letters folded to ASCII, `-` between
/// words; a number after it when taken.
pub fn domain_id_of(name: &str, taken: &[&str]) -> String {
    let mut stem = String::new();
    for c in name.chars() {
        let c = match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c => c.to_ascii_lowercase(),
        };
        if c.is_ascii_alphanumeric() {
            stem.push(c);
        } else if !stem.ends_with('-') && !stem.is_empty() {
            stem.push('-');
        }
    }
    let stem = stem.trim_end_matches('-');
    let stem: String = if stem.is_empty() {
        "liste".into()
    } else {
        stem.chars().take(56).collect()
    };
    if !taken.contains(&stem.as_str()) {
        return stem;
    }
    (2..)
        .map(|n| format!("{stem}-{n}"))
        .find(|c| !taken.contains(&c.as_str()))
        .unwrap_or(stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coded() -> FieldDomain {
        FieldDomain {
            id: "malzeme".into(),
            name: "Malzeme".into(),
            description: None,
            kind: LayerFieldKind::Text,
            values: Some(vec![
                FieldChoice::new("ah", "Ahşap"),
                FieldChoice::new("bt", "Betonarme"),
            ]),
            min: None,
            max: None,
            max_exclusive: false,
            min_exclusive: false,
            unit: None,
        }
    }

    #[test]
    fn a_list_is_coded_or_ranged() {
        assert_eq!(domain_problem(&coded()), None);
        let both = FieldDomain {
            min: Some("1".into()),
            ..coded()
        };
        assert!(domain_problem(&both).unwrap().contains("ikisi birden"));
        let none = FieldDomain {
            values: None,
            ..coded()
        };
        assert!(domain_problem(&none).unwrap().contains("ne kod var"));
        let range = FieldDomain {
            kind: LayerFieldKind::Integer,
            values: None,
            min: Some("0".into()),
            max: Some("40".into()),
            ..coded()
        };
        assert_eq!(domain_problem(&range), None);
        let upside = FieldDomain {
            min: Some("50".into()),
            ..range
        };
        assert!(
            domain_problem(&upside)
                .unwrap()
                .contains("listesinin en azı")
        );
    }

    #[test]
    fn ids_are_made_of_names() {
        assert_eq!(domain_id_of("Yapı malzemesi", &[]), "yapi-malzemesi");
        assert_eq!(
            domain_id_of("Yapı malzemesi", &["yapi-malzemesi"]),
            "yapi-malzemesi-2"
        );
        assert_eq!(domain_id_of("—", &[]), "liste");
    }
}
