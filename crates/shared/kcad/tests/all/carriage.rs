//! Every project-level field is carried by the file (docs/adr/0265 §1, its ek in docs/adr/0430): the writer picks the oldest
//! schema that holds a drawing, so a key it writes must be one that schema's reader reads. The fixtures test reads each
//! sample as a whole; a drawing that has one feature alone can pick a schema too old for it (a project with value lists
//! and no form, a field linked to a list and no id), and only a test that carries each feature on its own sees that.
//!
//! The samples are the fixtures' own (`fixtures/kcad/v2/*.json`, written by the independent writer): for every key of
//! the settings, of the snapshot, of a layer and of a layer field that any of them has, the first sample's value is put
//! alone into the smallest drawing, written, read back and held to what was written. The destructuring below names every
//! field of the contract's types: a field added without a name here does not compile, one named without a fixture fails.

use kentos_kcad::contracts::{DocumentSnapshotV2, LayerField, LayerNode, ProjectSettings};
use serde_json::{Value, json};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

/// The fixtures that are drawings: file name and content.
fn drawings() -> Vec<(String, Value)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(DIR).expect("fixtures").flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let Ok(v) = serde_json::from_slice::<Value>(&std::fs::read(&path).expect("read")) else {
            continue;
        };
        if v.get("format").and_then(Value::as_str) == Some("kentos.document") {
            out.push((path.file_name().unwrap().to_string_lossy().into_owned(), v));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn minimal() -> Value {
    drawings()
        .into_iter()
        .find(|(n, _)| n == "minimal.json")
        .expect("minimal.json")
        .1
}

enum Carried {
    /// Written, read back and the same.
    Same,
    /// The writer refused the drawing on its own rules: nothing to compare.
    Refused,
}

/// `doc` written and read back; the part `of` names (`settings`, `layers`, …) is held to what was written.
fn carry(doc: &Value, of: &str) -> Result<Carried, String> {
    let snap: DocumentSnapshotV2 =
        serde_json::from_value(doc.clone()).map_err(|e| format!("not a snapshot: {e}"))?;
    let Ok(bytes) = kentos_kcad::encode(&snap) else {
        return Ok(Carried::Refused);
    };
    let back = kentos_kcad::decode(&bytes)
        .map_err(|e| format!("the reader refuses what the writer wrote: {}", e.message))?;
    let (a, b) = (
        serde_json::to_value(&snap).expect("json"),
        serde_json::to_value(&back).expect("json"),
    );
    if a.get(of) == b.get(of) {
        Ok(Carried::Same)
    } else {
        Err(format!("read back differs in {of}"))
    }
}

/// Every key of `parts(v)` of every drawing, with the first value found.
fn first_values(parts: impl Fn(&Value) -> Vec<(String, Value)>) -> Vec<(String, Value, String)> {
    let mut seen = std::collections::BTreeMap::new();
    for (file, v) in drawings() {
        for (k, val) in parts(&v) {
            seen.entry(k).or_insert((val, file.clone()));
        }
    }
    seen.into_iter().map(|(k, (v, f))| (k, v, f)).collect()
}

fn object(v: &Value, key: &str) -> Vec<(String, Value)> {
    v.get(key)
        .and_then(Value::as_object)
        .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default()
}

fn nodes(list: &Value, out: &mut Vec<Value>) {
    for n in list.as_array().into_iter().flatten() {
        out.push(n.clone());
        nodes(&n["children"], out);
    }
}

// ── The settings ────────────────────────────────────────────────────────────────────────────────────────────────

/// Every field of the settings, named: a new field does not compile until it is named here.
#[allow(dead_code)]
fn settings_fields(s: &ProjectSettings) -> Vec<&'static str> {
    let ProjectSettings {
        srid: _,
        length_decimals: _,
        area_decimals: _,
        area_unit: _,
        angle_unit: _,
        plot_scale: _,
        workspace: _,
        drawing_font: _,
        line_types: _,
        hatch_patterns: _,
        sources: _,
        source_bases: _,
        line_type_scale: _,
        drawing_unit: _,
        second_srid: _,
        custom_crs: _,
        second_custom_crs: _,
        datum_transforms: _,
        survey: _,
        layer_states: _,
        text_styles: _,
        dimension_styles: _,
        leader_styles: _,
        wall_styles: _,
        arch: _,
        topology: _,
        annotation: _,
        connections: _,
        networks: _,
        variables: _,
        address_locators: _,
        ucs: _,
        named_ucs: _,
        named_views: _,
        forms: _,
        domains: _,
        relations: _,
        join_views: _,
        linear_events: _,
        length_style: _,
        xref_paths: _,
        heights: _,
        coordinate_epoch: _,
    } = s;
    vec![
        "srid",
        "lengthDecimals",
        "areaDecimals",
        "areaUnit",
        "angleUnit",
        "plotScale",
        "workspace",
        "drawingFont",
        "lineTypes",
        "hatchPatterns",
        "sources",
        "sourceBases",
        "lineTypeScale",
        "drawingUnit",
        "secondSrid",
        "customCrs",
        "secondCustomCrs",
        "datumTransforms",
        "survey",
        "layerStates",
        "textStyles",
        "dimensionStyles",
        "leaderStyles",
        "wallStyles",
        "arch",
        "topology",
        "annotation",
        "connections",
        "networks",
        "variables",
        "addressLocators",
        "ucs",
        "namedUcs",
        "namedViews",
        "forms",
        "domains",
        "relations",
        "joinViews",
        "linearEvents",
        "lengthStyle",
        "xrefPaths",
        "heights",
        "coordinateEpoch",
    ]
}

#[test]
fn every_settings_field_has_a_fixture_and_is_carried_alone() {
    let names =
        settings_fields(&serde_json::from_value(minimal()["settings"].clone()).expect("settings"));
    let found = first_values(|v| object(v, "settings"));
    let mut failed = Vec::new();
    for name in names {
        let Some((_, value, file)) = found.iter().find(|(k, _, _)| k == name) else {
            failed.push(format!("{name}: no fixture has it"));
            continue;
        };
        let alone = {
            let mut doc = minimal();
            doc["settings"][name] = value.clone();
            doc
        };
        let outcome = match carry(&alone, "settings") {
            Ok(Carried::Refused) => {
                // The writer's own rules need what the fixture has beside it (a bound layer's source, a drawing unit):
                // the key goes in with its fixture's whole settings, the objects and the layers.
                let whole = drawings()
                    .into_iter()
                    .find(|(n, _)| n == file)
                    .expect("fixture")
                    .1;
                carry(&whole, "settings")
            }
            other => other,
        };
        if let Err(why) = outcome {
            failed.push(format!("{name} (as in {file}): {why}"));
        }
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
}

// ── The snapshot ────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn every_part_of_the_snapshot_is_carried_alone() {
    // The keys the snapshot's own fields name; `settings` and `layers` have their own tests, the objects need layers.
    let found = first_values(|v| {
        v.as_object()
            .map(|o| o.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    });
    let mut failed = Vec::new();
    for (key, value, file) in &found {
        if matches!(
            key.as_str(),
            "format"
                | "version"
                | "name"
                | "settings"
                | "origin"
                | "layers"
                | "activeLayer"
                | "entities"
                | "uids"
                | "styles"
        ) {
            continue;
        }
        let mut doc = minimal();
        doc[key] = value.clone();
        let whole = drawings()
            .into_iter()
            .find(|(n, _)| n == file)
            .expect("fixture")
            .1;
        let outcome = match carry(&doc, key) {
            Ok(Carried::Refused) => carry(&whole, key),
            other => other,
        };
        if let Err(why) = outcome {
            failed.push(format!("{key} (as in {file}): {why}"));
        }
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
    // The contract's own list: a field added to the snapshot is named here before it is carried.
    let _ = |s: DocumentSnapshotV2| {
        let DocumentSnapshotV2 {
            format: _,
            version: _,
            name: _,
            settings: _,
            origin: _,
            home_view: _,
            layers: _,
            active_layer: _,
            entities: _,
            uids: _,
            styles: _,
            blocks: _,
            project_id: _,
            migrated_from: _,
            sheets: _,
            arrays: _,
            constraints: _,
            requests: _,
        } = s;
    };
}

// ── A layer and its fields ──────────────────────────────────────────────────────────────────────────────────────

#[test]
fn every_layer_key_is_carried_alone() {
    let mut all = Vec::new();
    for (_, v) in drawings() {
        nodes(&v["layers"], &mut all);
    }
    let mut seen = std::collections::BTreeMap::new();
    for n in &all {
        for (k, val) in n.as_object().into_iter().flatten() {
            seen.entry(k.clone()).or_insert((val.clone(), n.clone()));
        }
    }
    let mut failed = Vec::new();
    for (key, (value, node)) in &seen {
        if matches!(
            key.as_str(),
            "id" | "name" | "type" | "visible" | "locked" | "expanded" | "style" | "children"
        ) {
            continue;
        }
        let mut doc = minimal();
        let mut layer = doc["layers"][0].clone();
        layer[key] = value.clone();
        doc["layers"] = json!([layer]);
        let outcome = match carry(&doc, "layers") {
            Ok(Carried::Refused) => {
                // A key that needs its kind (a scenario is a group) or its fields: the node as it is, alone.
                let mut doc = minimal();
                let mut alone = node.clone();
                alone["children"] = json!([]);
                doc["layers"] = json!([alone]);
                doc["activeLayer"] = node["id"].clone();
                carry(&doc, "layers")
            }
            other => other,
        };
        if let Err(why) = outcome {
            failed.push(format!("layer.{key}: {why}"));
        }
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
    let _ = |n: LayerNode| n.id;
}

#[test]
fn every_layer_field_key_is_carried_alone() {
    let mut all = Vec::new();
    for (_, v) in drawings() {
        nodes(&v["layers"], &mut all);
    }
    let mut seen = std::collections::BTreeMap::new();
    for n in &all {
        for f in n["fields"].as_array().into_iter().flatten() {
            for (k, val) in f.as_object().into_iter().flatten() {
                seen.entry(k.clone())
                    .or_insert((val.clone(), f.clone(), n.clone()));
            }
        }
    }
    let mut failed = Vec::new();
    // The contract's list of a field's keys: a field added is named here before it is carried.
    let _ = |f: LayerField| {
        let LayerField {
            id: _,
            name: _,
            alias: _,
            kind: _,
            length: _,
            scale: _,
            min: _,
            max: _,
            values: _,
            required: _,
            default: _,
            multiple: _,
            split: _,
            merge: _,
            domain: _,
            unique: _,
            min_exclusive: _,
            max_exclusive: _,
            unit: _,
        } = f;
    };
    for key in [
        "id",
        "alias",
        "length",
        "scale",
        "min",
        "max",
        "values",
        "required",
        "default",
        "multiple",
        "split",
        "merge",
        "domain",
        "unique",
        "minExclusive",
        "maxExclusive",
        "unit",
    ] {
        let Some((value, field, node)) = seen.get(key) else {
            failed.push(format!("field.{key}: no fixture has it"));
            continue;
        };
        let with = |field: Value, settings: Option<&Value>| {
            let mut doc = minimal();
            let mut layer = doc["layers"][0].clone();
            layer["fields"] = json!([field]);
            doc["layers"] = json!([layer]);
            if let Some(s) = settings {
                doc["settings"] = s.clone();
            }
            doc
        };
        let mut alone = json!({ "name": "Alan", "kind": field["kind"] });
        alone[key] = value.clone();
        let outcome = match carry(&with(alone, None), "layers") {
            Ok(Carried::Refused) => {
                // The key beside the field it was written with (a range needs its kind) and its project's lists.
                let owner = drawings()
                    .into_iter()
                    .find(|(_, v)| {
                        let mut ns = Vec::new();
                        nodes(&v["layers"], &mut ns);
                        ns.iter()
                            .any(|n| n["id"] == node["id"] && n["fields"] == node["fields"])
                    })
                    .map(|(_, v)| v["settings"].clone());
                carry(&with(field.clone(), owner.as_ref()), "layers")
            }
            other => other,
        };
        if let Err(why) = outcome {
            failed.push(format!("field.{key}: {why}"));
        }
    }
    assert!(failed.is_empty(), "\n{}", failed.join("\n"));
}

// ── A project saved and opened again ────────────────────────────────────────────────────────────────────────────

/// The writer and the verifying read the apps' Save runs (`encode_verified`): a drawing is saved, and what comes back is
/// what was saved, in the settings, the layers and the objects.
fn saved_and_opened(snap: &DocumentSnapshotV2) -> DocumentSnapshotV2 {
    let bytes =
        kentos_kcad::encode_verified(snap).expect("Save writes it and reads it back the same");
    kentos_kcad::decode(&bytes).expect("opens")
}

#[test]
fn a_project_with_lists_relations_and_forms_is_saved_and_opened_again() {
    // The forms' sample with the value lists of the lists' sample and a field linked to one of them: all three.
    let lists = drawings()
        .into_iter()
        .find(|(n, _)| n == "domains.json")
        .expect("domains.json")
        .1;
    let mut doc = drawings()
        .into_iter()
        .find(|(n, _)| n == "forms.json")
        .expect("forms.json")
        .1;
    doc["settings"]["domains"] = lists["settings"]["domains"].clone();
    doc["layers"][0]["fields"][0]["domain"] = json!("durum");
    let snap: DocumentSnapshotV2 = serde_json::from_value(doc).expect("a snapshot");
    assert!(
        !snap.settings.forms.is_empty()
            && !snap.settings.relations.is_empty()
            && !snap.settings.domains.is_empty()
    );
    let back = saved_and_opened(&snap);
    assert_eq!(
        serde_json::to_value(&back).unwrap(),
        serde_json::to_value(&snap).unwrap()
    );
}

#[test]
fn the_profile_projects_are_saved_and_opened_again() {
    // fixtures/profiles/v1: a municipality's project (value lists, relations, linked fields), another institution's
    // names for it and the park project: each as the apps hold it (the v1 sample migrated), saved and opened.
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/profiles/v1/"
    );
    let mut saved = 0;
    for name in ["belediye.json", "sivas.json", "park-r1.json"] {
        let text = std::fs::read_to_string(format!("{dir}{name}"))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let v1 = kentos_kcad::contracts::DocumentSnapshotV1::from_json(&text)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let v2 = kentos_kcad::contracts::migrate_v1(v1).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let back = saved_and_opened(&v2);
        assert_eq!(
            serde_json::to_value(&back.settings).unwrap(),
            serde_json::to_value(&v2.settings).unwrap(),
            "{name}"
        );
        assert_eq!(
            serde_json::to_value(&back.layers).unwrap(),
            serde_json::to_value(&v2.layers).unwrap(),
            "{name}"
        );
        saved += 1;
    }
    assert_eq!(saved, 3);
}
