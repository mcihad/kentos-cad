//! Portable drawing-standard documents carried by a project. The order is
//! significant: an earlier office standard has precedence over a later one.
//! The command layer validates the full standard rules before associating one.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// An exact versioned standard document retained in the drawing file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ProjectStandardDocument {
    /// UTF-8 JSON document, with its format, document version, identity and
    /// office revision. Keeping the complete text allows exact re-export.
    pub text: String,
}

pub const MAX_PROJECT_STANDARD_DOCUMENTS: usize = 16;
pub const MAX_PROJECT_STANDARD_DOCUMENT_BYTES: usize = 1_048_576;
pub const MAX_PROJECT_STANDARDS_BYTES: usize = 4 * 1_048_576;

/// Structural validation at the file boundary. Rule semantics belong to the
/// commands crate; a project reader must never silently discard a rule.
pub fn project_standards_problem(documents: &[ProjectStandardDocument]) -> Option<String> {
    if documents.len() > MAX_PROJECT_STANDARD_DOCUMENTS {
        return Some(format!(
            "En çok {MAX_PROJECT_STANDARD_DOCUMENTS} standart bağlanabilir"
        ));
    }
    let mut size = 0usize;
    let mut ids = Vec::<String>::new();
    for document in documents {
        let Some(next_size) = size.checked_add(document.text.len()) else {
            return Some("Standart belgelerinin toplam boyutu geçersiz".into());
        };
        size = next_size;
        if document.text.is_empty() || document.text.len() > MAX_PROJECT_STANDARD_DOCUMENT_BYTES {
            return Some("Standart belgesi boş veya 1 MiB sınırından büyük".into());
        }
        let value: serde_json::Value = match serde_json::from_str(&document.text) {
            Ok(value) => value,
            Err(_) => return Some("Standart belgesi geçerli JSON değil".into()),
        };
        let Some(object) = value.as_object() else {
            return Some("Standart belgesi bir nesne olmalı".into());
        };
        if object.get("format").and_then(|v| v.as_str()) != Some("kentos.drawing-standard")
            || !matches!(object.get("version").and_then(|v| v.as_u64()), Some(1 | 2))
            || !object.get("standard").is_some_and(|v| v.is_object())
            || (object.get("version").and_then(|v| v.as_u64()) == Some(2)
                && !object.get("resources").is_some_and(|v| v.is_object()))
        {
            return Some("Standart belgesinin biçimi veya sürümü desteklenmiyor".into());
        }
        let (Some(id), Some(revision)) = (
            object.get("id").and_then(|v| v.as_str()),
            object.get("revision").and_then(|v| v.as_str()),
        ) else {
            return Some("Standart kimliği veya revizyonu eksik".into());
        };
        if id.trim().is_empty()
            || revision.trim().is_empty()
            || id.len() > 128
            || revision.len() > 128
        {
            return Some("Standart kimliği veya revizyonu geçersiz".into());
        }
        if ids.iter().any(|existing| existing == id) {
            return Some(format!("“{id}” standart kimliği iki kez bağlanamaz"));
        }
        ids.push(id.to_owned());
    }
    if size > MAX_PROJECT_STANDARDS_BYTES {
        return Some("Standart belgelerinin toplamı 4 MiB sınırını aşıyor".into());
    }
    None
}

/// A sheet template revision a bound office standard approved, recorded in the drawing by
/// `cad.standards.install` (docs/adr/1100 §3): the project's own note of which template new sheets
/// are made from. Recording it changes no sheet; a sheet keeps the origin it was made from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export))]
pub struct ApprovedSheetTemplate {
    /// The template's id (`sys:cad-mimari`, a user's or an office's).
    pub template_id: String,
    #[cfg_attr(feature = "schema", schemars(extend("x-unit" = "version")))]
    pub revision: u32,
    /// The standard that approved it and its office revision.
    pub standard: String,
    pub standard_revision: String,
}

pub const MAX_APPROVED_SHEET_TEMPLATES: usize = 256;

/// Structural validation at the file boundary: at most 256, each id 1–128 bytes without edge
/// spaces or control characters and unique (case aside), revision at least 1, the standard's id and
/// revision 1–128 bytes.
pub fn approved_sheet_templates_problem(items: &[ApprovedSheetTemplate]) -> Option<String> {
    if items.len() > MAX_APPROVED_SHEET_TEMPLATES {
        return Some(format!("En çok {MAX_APPROVED_SHEET_TEMPLATES} onaylı pafta şablonu kaydedilir"));
    }
    let short = |t: &str| {
        !t.trim().is_empty() && t.trim() == t && t.len() <= 128 && !t.chars().any(char::is_control)
    };
    let mut ids = std::collections::BTreeSet::new();
    for item in items {
        if !short(&item.template_id) || item.revision == 0 {
            return Some("Onaylı pafta şablonunun kimliği 1–128 bayt ve revizyonu en az 1 olmalı".into());
        }
        if !short(&item.standard) || !short(&item.standard_revision) {
            return Some("Onaylı pafta şablonunun standart kimliği ve revizyonu 1–128 bayt olmalı".into());
        }
        if !ids.insert(item.template_id.to_lowercase()) {
            return Some(format!("“{}” onaylı pafta şablonu iki kez kayıtlı", item.template_id));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_sheet_templates_are_unique_and_short() {
        let item = |id: &str, revision| ApprovedSheetTemplate {
            template_id: id.into(),
            revision,
            standard: "ofis-a".into(),
            standard_revision: "2026.1".into(),
        };
        assert_eq!(approved_sheet_templates_problem(&[item("sys:cad-mimari", 1), item("ofis:a3", 2)]), None);
        assert!(approved_sheet_templates_problem(&[item("A", 1), item("a", 2)]).is_some());
        assert!(approved_sheet_templates_problem(&[item(" a", 1)]).is_some());
        assert!(approved_sheet_templates_problem(&[item("a", 0)]).is_some());
        let mut blank = item("a", 1);
        blank.standard_revision = String::new();
        assert!(approved_sheet_templates_problem(&[blank]).is_some());
    }

    fn document(id: &str) -> ProjectStandardDocument {
        ProjectStandardDocument {
            text: serde_json::json!({"format":"kentos.drawing-standard", "version":1,
                "id":id, "revision":"2026.1", "standard":{"name":"Ofis"}})
            .to_string(),
        }
    }

    #[test]
    fn validates_identity_and_preserves_the_order_of_distinct_packages() {
        let a = document("a");
        let b = document("b");
        assert_eq!(project_standards_problem(&[b.clone(), a.clone()]), None);
        assert!(project_standards_problem(&[a.clone(), a]).is_some());
        assert!(
            project_standards_problem(&[ProjectStandardDocument { text: "{".into() }]).is_some()
        );
        assert!(project_standards_problem(&[document("")]).is_some());
        let mut version_two: serde_json::Value =
            serde_json::from_str(&document("v2").text).unwrap();
        version_two["version"] = 2.into();
        version_two["resources"] = serde_json::json!({"lineTypes":[], "hatchPatterns":[]});
        assert_eq!(
            project_standards_problem(&[ProjectStandardDocument {
                text: version_two.to_string(),
            }]),
            None
        );
        version_two.as_object_mut().unwrap().remove("resources");
        assert!(
            project_standards_problem(&[ProjectStandardDocument {
                text: version_two.to_string(),
            }])
            .is_some()
        );
    }
}
