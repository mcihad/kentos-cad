//! Unsupported servers must see no write containing persistent dimension recipes.

use kentos_cloud::Cloud;
use kentos_contracts::{
    CommandEnvelope, DocumentSnapshotV2, ProjectChanges, ProjectCreate, ProjectPatch,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

const MODEL: &str = include_str!("../../../../fixtures/kcad/v2/dimension-sets.json");

fn create() -> ProjectCreate {
    let d: DocumentSnapshotV2 = serde_json::from_str(MODEL).unwrap();
    ProjectCreate {
        name: d.name,
        settings: d.settings,
        origin: d.origin,
        home_view: d.home_view,
        layers: d.layers,
        active_layer: d.active_layer,
        styles: d.styles,
        sheets: None,
        arrays: Vec::new(),
        dimension_sets: d.dimension_sets,
        casework: d.casework,
        constraints: Default::default(),
        description: None,
        project_type: None,
        tags: None,
        storage: None,
    }
}

fn envelope(input: Value) -> CommandEnvelope {
    CommandEnvelope {
        command_name: kentos_contracts::PROJECT_CHANGES.into(),
        version: 2,
        tenant_id: Uuid::now_v7().to_string(),
        project_id: Uuid::now_v7().to_string(),
        request_id: "capability-test".into(),
        idempotency_key: "capability-test".into(),
        expected_versions: Default::default(),
        input,
    }
}

async fn server(answers: Vec<(u16, Value)>) -> (Cloud, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cloud = Cloud::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    tokio::spawn(async move {
        for (status, body) in answers {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut byte = [0];
            while !bytes.ends_with(b"\r\n\r\n") {
                if stream.read(&mut byte).await.unwrap() == 0 {
                    break;
                }
                bytes.push(byte[0]);
            }
            let head = String::from_utf8(bytes).unwrap();
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut data = vec![0; length];
            stream.read_exact(&mut data).await.unwrap();
            seen.lock()
                .unwrap()
                .push(format!("{head}{}", String::from_utf8_lossy(&data)));
            let body = serde_json::to_vec(&body).unwrap();
            stream.write_all(format!("HTTP/1.1 {status} Answer\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
            stream.write_all(&body).await.unwrap();
            stream.shutdown().await.unwrap();
        }
    });
    (cloud, requests)
}

fn health(versions: Option<Vec<u32>>) -> Value {
    let mut h = json!({"status":"ok", "service":"test", "version":"test", "contracts":1});
    if let Some(versions) = versions {
        h["projectChangesVersions"] = json!(versions);
    }
    h
}

#[tokio::test]
async fn old_health_or_missing_endpoint_refuses_create_before_any_post() {
    for answer in [
        (200, health(None)),
        (200, health(Some(vec![1]))),
        (404, json!({"error":"not_found", "message":"old"})),
    ] {
        let (cloud, requests) = server(vec![answer]).await;
        let error = cloud
            .create_project(Uuid::now_v7(), create(), Uuid::now_v7())
            .await
            .unwrap_err();
        assert_eq!(error.code, "server_feature_unavailable");
        assert_eq!(error.path.as_deref(), Some("dimensionSets"));
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /v1/health "));
    }
}

#[tokio::test]
async fn supported_server_receives_recipe_create_after_probe() {
    let (cloud, requests) = server(vec![
        (200, health(Some(vec![1, 2]))),
        (400, json!({"error":"received", "message":"test receipt"})),
    ])
    .await;
    let error = cloud
        .create_project(Uuid::now_v7(), create(), Uuid::now_v7())
        .await
        .unwrap_err();
    assert_eq!(error.code, "received");
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].starts_with("POST "));
    assert!(requests[1].contains("\"dimensionSets\":[{"));
}

#[tokio::test]
async fn empty_clear_still_requires_v2_capability() {
    let input = ProjectChanges {
        features: Vec::new(),
        blocks: Vec::new(),
        project: Some(ProjectPatch {
            dimension_sets: Some(Vec::new()),
            ..ProjectPatch::default()
        }),
    };
    assert_eq!(input.command_version(), 2);
    let (cloud, requests) = server(vec![(200, health(None))]).await;
    let error = cloud
        .command::<Value>(envelope(serde_json::to_value(input).unwrap()))
        .await
        .unwrap_err();
    assert_eq!(error.code, "server_feature_unavailable");
    assert_eq!(requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn legacy_queued_v1_command_keeps_working_without_probe() {
    let input = ProjectChanges {
        features: Vec::new(),
        blocks: Vec::new(),
        project: None,
    };
    assert_eq!(input.command_version(), 1);
    let mut command = envelope(serde_json::to_value(input).unwrap());
    command.version = 1;
    let (cloud, requests) = server(vec![(200, json!({"ok":true}))]).await;
    assert_eq!(
        cloud.command::<Value>(command).await.unwrap(),
        json!({"ok":true})
    );
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("POST "));
}
