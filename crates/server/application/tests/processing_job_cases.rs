//! What a `project.processing` job decides without a database (docs/adr/0559), against the shared cases
//! `fixtures/server-jobs/v1/processing.json` that `scripts/fixtures/processing_job_cases.py` writes from the ADR's rules
//! without KentOS code: the request's faults, the memory estimate, the budget, and the delta (what a tool changed)
//! found by comparing two drawings. The cases describe the drawings compactly; this test builds the documents.

use kentos_application::processing_job::{budget_of, check_input, delta};
use kentos_contracts::{
    BlockDefinition, DOCUMENT_FORMAT, DOCUMENT_VERSION_2, DocumentSnapshotV1, DocumentSnapshotV2,
    Entity, EntityId, FeatureChange, LayerNode, ProcessingJobInput, processing_memory_estimate,
};
use serde_json::{Value, json};
use uuid::Uuid;

const CASES: &str = include_str!("../../../../fixtures/server-jobs/v1/processing.json");
const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("the cases read")
}

fn layer(id: &str) -> LayerNode {
    serde_json::from_value(json!({
        "id": id, "name": id, "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": []
    }))
    .expect("a layer")
}

fn uid(i: u64) -> String {
    format!("00000000-0000-4000-8000-{i:012x}")
}

fn entity_id(text: &str) -> EntityId {
    EntityId(Uuid::parse_str(text).expect("a uuid").into_bytes())
}

/// The compact drawing as a document: its layers, its objects as points, its blocks by name.
fn document(compact: &Value) -> DocumentSnapshotV2 {
    let sample = DocumentSnapshotV1::from_json(SAMPLE).expect("the sample reads");
    let mut settings = sample.settings.clone();
    settings.plot_scale = compact["plotScale"].as_f64().expect("a scale");
    let mut objects: Vec<Value> = compact["entities"].as_array().cloned().unwrap_or_default();
    if let Some(gen_) = compact.get("generate") {
        let from = gen_["from"].as_u64().unwrap_or(1);
        for i in 0..gen_["count"].as_u64().expect("a count") {
            objects.push(json!({
                "uid": uid(from + i), "slot": i + 1, "layer": gen_["layer"], "x": i as f64, "y": 0.0, "attrs": {}
            }));
        }
    }
    let (mut entities, mut uids) = (Vec::new(), Vec::new());
    for o in &objects {
        let entity: Entity = serde_json::from_value(json!({
            "kind": "point", "id": o["slot"], "layerId": o["layer"], "attrs": o["attrs"],
            "p": { "x": o["x"], "y": o["y"] }
        }))
        .expect("a point");
        entities.push(entity);
        uids.push(entity_id(o["uid"].as_str().expect("a uid")));
    }
    let blocks: Vec<BlockDefinition> = compact["blocks"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(i, name)| {
            serde_json::from_value(json!({
                "id": Uuid::from_u128(0x1000 + i as u128).to_string(),
                "name": name, "base": { "x": 0.0, "y": 0.0 }, "entities": []
            }))
            .expect("a block")
        })
        .collect();
    DocumentSnapshotV2 {
        format: DOCUMENT_FORMAT.to_owned(),
        version: DOCUMENT_VERSION_2,
        name: compact["name"].as_str().expect("a name").to_owned(),
        settings,
        origin: sample.origin,
        home_view: None,
        layers: compact["layers"]
            .as_array()
            .expect("layers")
            .iter()
            .map(|l| layer(l.as_str().expect("a layer id")))
            .collect(),
        active_layer: compact["activeLayer"].as_str().expect("active").to_owned(),
        entities,
        uids,
        styles: sample.styles.clone(),
        blocks,
        project_id: None,
        migrated_from: None,
        sheets: None,
        arrays: Vec::new(),
        constraints: Default::default(),
        requests: Vec::new(),
    }
}

fn input_of(value: &Value) -> ProcessingJobInput {
    serde_json::from_value(value.clone()).expect("an input")
}

#[test]
fn a_request_is_faulted_where_the_reference_says() {
    let cases = cases();
    let inputs = cases["inputs"].as_array().expect("inputs");
    assert!(inputs.len() >= 20);
    for case in inputs {
        let got = check_input(&input_of(&case["input"])).map(|(path, _)| path);
        let want = case["fault"].as_str().map(str::to_owned);
        assert_eq!(got, want, "{}", case["name"]);
    }
}

#[test]
fn the_memory_a_file_is_estimated_to_need_is_the_references() {
    let cases = cases();
    for case in cases["memory"].as_array().expect("memory") {
        let bytes = case["bytes"].as_u64().expect("bytes");
        assert_eq!(
            processing_memory_estimate(bytes),
            case["estimate"].as_u64().expect("estimate"),
            "{bytes}"
        );
    }
}

#[test]
fn a_budget_is_the_servers_or_a_smaller_one_asked_for() {
    let cases = cases();
    for case in cases["budgets"].as_array().expect("budgets") {
        let limits = kentos_application::jobs::JobLimits {
            cpu_seconds: case["limits"]["cpuSeconds"].as_u64().expect("cpu") as u32,
            memory_mb: case["limits"]["memoryMb"].as_u64().expect("memory") as u32,
            ..Default::default()
        };
        let mut request = case["request"].clone();
        request["tool"] = json!("x");
        let budget = budget_of(&input_of(&request), &limits);
        assert_eq!(
            u64::from(budget.cpu_seconds),
            case["budget"]["cpuSeconds"].as_u64().expect("cpu"),
            "{case}"
        );
        assert_eq!(
            budget.memory_bytes,
            case["budget"]["memoryBytes"].as_u64().expect("memory"),
            "{case}"
        );
    }
}

fn ids_of(
    delta: &kentos_application::processing_job::Delta,
    create: bool,
    update: bool,
    delete: bool,
) -> Vec<String> {
    delta
        .features
        .iter()
        .filter_map(|c| match c {
            FeatureChange::Create { id, .. } if create => Some(id.clone()),
            FeatureChange::Update { id, .. } if update => Some(id.clone()),
            FeatureChange::Delete { id } if delete => Some(id.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn what_a_tool_changed_is_found_as_the_reference_finds_it() {
    let cases = cases();
    let deltas = cases["deltas"].as_array().expect("deltas");
    assert!(deltas.len() >= 20);
    for case in deltas {
        let name = &case["name"];
        let before = document(&case["before"]);
        let after = document(&case["after"]);
        let want = &case["expect"];
        let got = delta(&before, &after);
        if let Some(code) = want["refused"].as_str() {
            let refusal = got.expect_err(&format!("{name} is refused"));
            assert_eq!(refusal.code, code, "{name}");
            continue;
        }
        let got = got.unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(got.added, want["added"].as_u64().expect("added"), "{name}");
        assert_eq!(
            got.changed,
            want["changed"].as_u64().expect("changed"),
            "{name}"
        );
        assert_eq!(
            got.removed,
            want["removed"].as_u64().expect("removed"),
            "{name}"
        );
        for (key, lists) in [
            ("created", (true, false, false)),
            ("updated", (false, true, false)),
            ("deleted", (false, false, true)),
        ] {
            if let Some(list) = want[key].as_array() {
                let want: Vec<String> = list
                    .iter()
                    .map(|v| v.as_str().expect("an id").to_owned())
                    .collect();
                assert_eq!(
                    ids_of(&got, lists.0, lists.1, lists.2),
                    want,
                    "{name}: {key}"
                );
            }
        }
        // The order a command takes them in: created, changed, removed.
        let order: Vec<u8> = got
            .features
            .iter()
            .map(|c| match c {
                FeatureChange::Create { .. } => 0,
                FeatureChange::Update { .. } => 1,
                FeatureChange::Delete { .. } => 2,
            })
            .collect();
        assert!(order.windows(2).all(|w| w[0] <= w[1]), "{name}");
        // The metadata the delta carries, by the parts of the patch it sets.
        let mut patch: Vec<&str> = Vec::new();
        if let Some(p) = &got.project {
            for (set, key) in [
                (p.name.is_some(), "name"),
                (p.settings.is_some(), "settings"),
                (p.layers.is_some(), "layers"),
                (p.active_layer.is_some(), "activeLayer"),
            ] {
                if set {
                    patch.push(key);
                }
            }
        }
        patch.sort_unstable();
        let want_patch: Vec<&str> = want["patch"]
            .as_array()
            .expect("patch")
            .iter()
            .map(|v| v.as_str().expect("a key"))
            .collect();
        assert_eq!(patch, want_patch, "{name}");
        // A created or changed object goes with no slot number: the server numbers nothing.
        for c in &got.features {
            if let FeatureChange::Create { entity, .. } | FeatureChange::Update { entity, .. } = c {
                assert_eq!(entity.base().id, 0, "{name}");
            }
        }
        assert_eq!(
            got.is_empty(),
            want["added"] == 0
                && want["changed"] == 0
                && want["removed"] == 0
                && want_patch.is_empty(),
            "{name}"
        );
    }
}
