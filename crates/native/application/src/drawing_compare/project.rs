//! Sheet and view differences have their own identities; they are not model entities.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{DrawingProjectChange, view_key};
use kentos_domain::Document;
use serde_json::{Value, json};

type Records = BTreeMap<(&'static str, String), (String, Value)>;

fn pointer_part(id: &str) -> String {
    id.replace('~', "~0").replace('/', "~1")
}

fn records(doc: &Document) -> Records {
    let mut out = Records::new();
    for view in &doc.settings().named_views {
        out.insert(
            ("namedView", view_key(view)),
            (view.name.clone(), json!(view)),
        );
    }
    for template in &doc.settings().view_templates {
        out.insert(
            ("viewTemplate", template.id.clone()),
            (template.name.clone(), json!(template)),
        );
    }
    let Some(sheets) = doc.sheets() else {
        return out;
    };
    let mut book = sheets.book.clone();
    if let Some(list) = book.get("sheets").and_then(Value::as_array) {
        let mut order = Vec::new();
        for (index, sheet) in list.iter().enumerate() {
            let id = sheet
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("@{index}"));
            order.push(id.clone());
            let name = sheet
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&id)
                .to_owned();
            let mut data = sheet.clone();
            if let Some(items) = sheet.get("items").and_then(Value::as_array) {
                let mut other = Vec::new();
                let mut item_order = Vec::new();
                for (item_index, item) in items.iter().enumerate() {
                    let item_id = item
                        .get("id")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("@{item_index}"));
                    item_order.push(item_id.clone());
                    if item.pointer("/kind/type").and_then(Value::as_str) == Some("map") {
                        let item_name =
                            item.get("name").and_then(Value::as_str).unwrap_or(&item_id);
                        out.insert(
                            (
                                "viewport",
                                format!("{}/{}", pointer_part(&id), pointer_part(&item_id)),
                            ),
                            (format!("{name} / {item_name}"), item.clone()),
                        );
                    } else {
                        other.push(item.clone());
                    }
                }
                if let Some(map) = data.as_object_mut() {
                    map.insert("items".into(), json!(other));
                    map.insert("itemOrder".into(), json!(item_order));
                }
            }
            out.insert(("sheet", id), (name, data));
        }
        if let Some(map) = book.as_object_mut() {
            map.remove("sheets");
            map.insert("sheetOrder".into(), json!(order));
        }
    }
    // The book names picture content by digest. Never serialize the picture bytes for a comparison.
    let mut pictures: Vec<_> = sheets.pictures.iter().map(|p| p.sha256.as_str()).collect();
    pictures.sort_unstable();
    out.insert(
        ("sheetBook", "book".into()),
        (
            "Pafta kitabı".into(),
            json!({"book": book, "pictures": pictures}),
        ),
    );
    out
}

fn fields(a: &Value, b: &Value, prefix: &str, out: &mut Vec<String>) {
    if a == b {
        return;
    }
    if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
        let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
        for key in keys {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match (a.get(key), b.get(key)) {
                (Some(a), Some(b)) => fields(a, b, &path, out),
                _ => out.push(path),
            }
        }
    } else {
        out.push(prefix.to_owned());
    }
}

/// Deterministic differences, in category/id order. Entity counts remain independent.
pub fn diff(old: &Document, new: &Document) -> Vec<DrawingProjectChange> {
    let old = records(old);
    let new = records(new);
    let keys: BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let mut out = Vec::new();
    for key in keys {
        let (name, status, changed_fields) = match (old.get(key), new.get(key)) {
            (Some((_, a)), Some((name, b))) if a != b => {
                let mut changed = Vec::new();
                fields(a, b, "", &mut changed);
                (name, "changed", changed)
            }
            (None, Some((name, _))) => (name, "added", Vec::new()),
            (Some((name, _)), None) => (name, "removed", Vec::new()),
            _ => continue,
        };
        out.push(DrawingProjectChange {
            kind: key.0.into(),
            id: key.1.clone(),
            name: name.clone(),
            status: status.into(),
            fields: changed_fields,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::DocumentSnapshotV2;

    fn drawing(value: Value) -> Document {
        Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(value).unwrap())
            .unwrap()
    }

    fn sheet() -> Value {
        serde_json::from_str(include_str!("../../../../../fixtures/kcad/v2/sheets.json")).unwrap()
    }

    #[test]
    fn viewport_scale_is_reported_without_a_model_or_sheet_geometry_change() {
        let original = sheet();
        let mut revised = original.clone();
        revised["sheets"]["book"]["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(200);
        let old = drawing(original);
        let new = drawing(revised);
        let changes = diff(&old, &new);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, "viewport");
        assert_eq!(changes[0].id, "pafta-1/harita");
        assert_eq!(changes[0].fields, ["kind.view.scale"]);
        assert!(super::super::diff(&old, &new, 0.001).changed.is_empty());
    }

    #[test]
    fn sheet_page_and_title_block_changes_remain_sheet_changes() {
        let original = sheet();
        let mut revised = original.clone();
        revised["sheets"]["book"]["sheets"][0]["page"]["margins"]["left"] = json!(25000);
        revised["sheets"]["book"]["sheets"][0]["items"][2]["name"] = json!("Yeni antet");
        let changes = diff(&drawing(original), &drawing(revised));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, "sheet");
        assert_eq!(changes[0].fields, ["items", "page.margins.left"]);
    }

    #[test]
    fn viewport_add_remove_and_reorder_keep_their_stable_id() {
        let original = sheet();
        let mut revised = original.clone();
        revised["sheets"]["book"]["sheets"][0]["items"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        let changes = diff(&drawing(original.clone()), &drawing(revised));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].fields, ["itemOrder"]);
        let mut revised = original.clone();
        revised["sheets"]["book"]["sheets"][0]["items"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        let a = drawing(original);
        let b = drawing(revised);
        assert!(
            diff(&a, &b)
                .iter()
                .any(|c| c.kind == "viewport" && c.status == "removed")
        );
        assert!(
            diff(&b, &a)
                .iter()
                .any(|c| c.kind == "viewport" && c.status == "added")
        );
    }

    #[test]
    fn identical_book_is_empty_and_comparison_is_symmetric() {
        let old = drawing(sheet());
        assert!(diff(&old, &old).is_empty());
        let mut revised = sheet();
        revised["sheets"] = Value::Null;
        let new = drawing(revised);
        let removed = diff(&old, &new);
        let added = diff(&new, &old);
        assert_eq!(removed.len(), added.len());
        assert!(removed.iter().all(|c| c.status == "removed"));
        assert!(added.iter().all(|c| c.status == "added"));
    }

    #[test]
    fn named_view_identity_survives_rename_and_reports_its_recipe_scale() {
        let mut original = sheet();
        original["settings"]["namedViews"] = json!([{
            "id":"detail-1", "name":"Detay", "center":{"x":500000.,"y":4400000.},
            "width":10., "height":8., "twist":0., "recipe":{"version":1,"scale":20}
        }]);
        let mut revised = original.clone();
        revised["settings"]["namedViews"][0]["name"] = json!("Banyo detayı");
        revised["settings"]["namedViews"][0]["recipe"]["scale"] = json!(5);
        let changes = diff(&drawing(original), &drawing(revised));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].id, "detail-1");
        assert_eq!(changes[0].status, "changed");
        assert_eq!(changes[0].fields, ["name", "recipe.scale"]);
    }

    #[test]
    fn map_properties_have_specific_paths_and_never_change_model_counts() {
        for (key, value, field) in [
            ("viewLocked", json!(true), "kind.viewLocked"),
            ("layers", json!({"type":"visible"}), "kind.layers"),
            ("padding", json!(4000), "padding"),
            ("printable", json!(false), "printable"),
            ("rotation", json!(45000), "rotation"),
        ] {
            let original = sheet();
            let mut revised = original.clone();
            if field.starts_with("kind.") {
                revised["sheets"]["book"]["sheets"][0]["items"][0]["kind"][key] = value;
            } else {
                revised["sheets"]["book"]["sheets"][0]["items"][0][key] = value;
            }
            let changes = diff(&drawing(original), &drawing(revised));
            assert_eq!(changes.len(), 1, "{field}");
            assert_eq!(changes[0].kind, "viewport");
            assert!(
                changes[0].fields.iter().any(|f| f.starts_with(field)),
                "{changes:?}"
            );
        }
    }

    #[test]
    fn command_caps_project_changes_and_keeps_entity_counts_separate() {
        let mut revised = sheet();
        revised["sheets"]["book"]["sheets"][0]["name"] = json!("Yeni pafta");
        revised["sheets"]["book"]["sheets"][0]["items"][0]["kind"]["view"]["scale"] = json!(200);
        let doc = drawing(revised);
        let got = super::super::compare_in(
            &doc,
            &kentos_commands::DrawingCompare {
                against: kentos_commands::CompareAgainst::File {
                    path: std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../../fixtures/kcad/v2/sheets.kcad")
                        .canonicalize()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                },
                tolerance: None,
                limit: Some(1),
                match_search: None,
            },
            || false,
        )
        .unwrap_or_else(|_| panic!("comparison failed"));
        assert_eq!((got.changed, got.added, got.removed), (0, 0, 0));
        assert_eq!(got.project_changes.len(), 1);
        assert!(got.truncated);
    }

    #[test]
    fn viewport_ids_escape_separators_without_collisions() {
        assert_eq!(pointer_part("a/b~c"), "a~1b~0c");
        assert_ne!(
            format!("{}/{}", pointer_part("a/b"), pointer_part("c")),
            format!("{}/{}", pointer_part("a"), pointer_part("b/c"))
        );
    }
}
