//! The cloud portal's dossiers as the desktop asks for them (docs/adr/9000 §5–8, docs/adr/9001), against a server that
//! answers in turn and keeps what each request said: the addresses with their queries, the program's header, the
//! bodies as the server reads them (camelCase, the status the person saw), a file's bytes checked against their
//! SHA-256, and a refusal in the server's own words.

use std::sync::{Arc, Mutex};

use kentos_cloud::{Cloud, DossierQuery};
use kentos_contracts::{
    DossierAction, DossierActionRequest, DossierDirection, DossierFileImport, DossierStatus,
    FromProject, MessagePost,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

/// One answer: its status, its body and the headers it comes with.
struct Answer {
    status: u16,
    body: Vec<u8>,
    headers: Vec<(&'static str, String)>,
}

fn json_answer(status: u16, body: Value) -> Answer {
    Answer {
        status,
        body: serde_json::to_vec(&body).unwrap(),
        headers: vec![("content-type", "application/json".into())],
    }
}

/// A server answering `answers` in turn; what each request said (its head and body).
async fn server(answers: Vec<Answer>) -> (Cloud, Arc<Mutex<Vec<(String, Vec<u8>)>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cloud = Cloud::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let seen = requests.clone();
    tokio::spawn(async move {
        for answer in answers {
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
            seen.lock().unwrap().push((head, data));
            let mut text = format!(
                "HTTP/1.1 {} Durum\r\ncontent-length: {}\r\n",
                answer.status,
                answer.body.len()
            );
            for (name, value) in &answer.headers {
                text.push_str(&format!("{name}: {value}\r\n"));
            }
            text.push_str("\r\n");
            stream.write_all(text.as_bytes()).await.unwrap();
            stream.write_all(&answer.body).await.unwrap();
        }
    });
    (cloud, requests)
}

const DOSSIER: &str = "01a1ea92-0b91-777f-8c49-0a95082ec501";
const FILE: &str = "01a1ea92-0b91-777f-8c49-0a95082ec5f1";
const TENANT: &str = "01a1ea91-fa5d-7693-b241-c3e515a0d28b";
const PROJECT: &str = "01a1ea92-0b41-71f3-bbbd-b66bcf53d5e3";

fn summary(status: &str) -> Value {
    json!({
        "id": DOSSIER, "direction": "transfer", "number": "2026/0007", "kind": "project_revision",
        "kindLabel": "Proje düzeltme", "title": "Belediye hizmet binası cephe düzenlemesi", "status": status,
        "statusLabel": "Yanıt bekleniyor", "tenantId": TENANT, "tenantSlug": "ornek-belediyesi",
        "tenantName": "Örnek Belediyesi", "professionalId": "u1", "professionalName": "Ayşe Yılmaz",
        "profession": "architect", "assigneeName": null, "round": 1, "site": {},
        "createdAt": "2026-11-17T17:33:21Z", "updatedAt": "2026-11-17T17:33:22Z",
        "sentAt": "2026-11-17T17:33:22Z", "dueOn": "2026-11-27", "clock": null, "awaitingMe": true
    })
}

fn detail(status: &str, actions: Value) -> Value {
    json!({
        "summary": summary(status), "description": "Ana cephede pencere oranları.", "ownerName": null,
        "side": "professional", "actions": actions, "canEdit": false, "canAddFiles": false, "canMessage": true,
        "canDelete": false, "canImport": true, "pendingRound": 2, "rounds": [], "pending": [], "messages": [],
        "events": [], "checklist": null, "professional": null,
        "organization": { "tenantId": TENANT, "slug": "ornek-belediyesi", "name": "Örnek Belediyesi",
            "province": "Örnek", "district": "Merkez", "contactEmail": null, "contactPhone": null },
        "assignees": []
    })
}

fn file_view() -> Value {
    json!({
        "id": FILE, "side": "professional", "purpose": "package", "round": 2, "state": "pending",
        "docType": "kentos_project", "docTypeLabel": "KentOS projesi (KCAD)", "name": "Cephe.kcad",
        "mediaType": "application/vnd.kentos.kcad", "size": 2048, "sha256": "ab".repeat(32),
        "addedByName": "Ayşe Yılmaz", "addedAt": "2026-11-17T18:00:00Z",
        "source": { "tenantId": TENANT, "projectId": PROJECT, "projectName": "Cephe", "revision": "3" },
        "isProject": true
    })
}

fn id(text: &str) -> Uuid {
    Uuid::parse_str(text).unwrap()
}

/// A list asks for what is chosen in the address, the organisation's under its id; the answer reads as the
/// contracts' types; every request says it is the desktop's.
#[tokio::test]
async fn the_lists_ask_with_their_query_and_read_the_servers_answer() {
    let list = json!({ "items": [summary("sent")], "total": 1 });
    let (cloud, seen) = server(vec![json_answer(200, list.clone()), json_answer(200, list)]).await;
    let mine = cloud
        .my_dossiers(&DossierQuery {
            direction: Some(DossierDirection::Transfer),
            awaiting: true,
            limit: Some(100),
            ..DossierQuery::default()
        })
        .await
        .expect("listed");
    assert_eq!(mine.total, 1);
    assert_eq!(mine.items[0].status, DossierStatus::Sent);
    assert_eq!(mine.items[0].number.as_deref(), Some("2026/0007"));
    let theirs = cloud
        .organization_dossiers(
            id(TENANT),
            &DossierQuery {
                statuses: vec![DossierStatus::Sent, DossierStatus::Delivered],
                text: Some("cephe".into()),
                ..DossierQuery::default()
            },
        )
        .await
        .expect("listed");
    assert_eq!(theirs.items.len(), 1);
    let seen = seen.lock().unwrap();
    assert!(
        seen[0]
            .0
            .starts_with("GET /v1/me/dossiers?direction=transfer&awaiting=true&limit=100 HTTP/1.1"),
        "{}",
        seen[0].0
    );
    assert!(
        seen[1].0.starts_with(&format!(
            "GET /v1/tenants/{TENANT}/dossiers?status=sent%2Cdelivered&q=cephe HTTP/1.1"
        )),
        "{}",
        seen[1].0
    );
    for (head, _) in seen.iter() {
        assert!(
            head.to_lowercase().contains("x-kentos-client: desktop"),
            "{head}"
        );
    }
}

/// A step goes with the status the person saw and comes back as the dossier after it; a message and the open
/// project's present state go as the server reads them.
#[tokio::test]
async fn a_step_a_message_and_the_open_project_go_as_the_server_reads_them() {
    let message = json!({
        "id": "m1", "round": 1, "side": "professional", "kind": "message", "authorName": "Ayşe Yılmaz",
        "body": "Cephe paftalarını yarın yüklüyorum.", "at": "2026-11-17T18:00:00Z", "attachments": []
    });
    let (cloud, seen) = server(vec![
        json_answer(200, detail("accepted", json!(["deliver", "cancel"]))),
        json_answer(201, message),
        json_answer(201, file_view()),
        // No content, as the server answers a removal.
        Answer {
            status: 204,
            body: Vec::new(),
            headers: Vec::new(),
        },
    ])
    .await;
    let after = cloud
        .dossier_step(
            id(DOSSIER),
            DossierActionRequest {
                action: DossierAction::Accept,
                expected_status: DossierStatus::Sent,
                note: Some("Üstleniyorum.".into()),
                assignee_id: None,
                attachments: Vec::new(),
            },
        )
        .await
        .expect("taken");
    assert_eq!(after.summary.status, DossierStatus::Accepted);
    assert_eq!(
        after.actions,
        [DossierAction::Deliver, DossierAction::Cancel]
    );
    let sent = cloud
        .dossier_message(
            id(DOSSIER),
            MessagePost {
                body: "Cephe paftalarını yarın yüklüyorum.".into(),
                attachments: Vec::new(),
            },
        )
        .await
        .expect("sent");
    assert_eq!(sent.author_name, "Ayşe Yılmaz");
    let added = cloud
        .dossier_add_project(
            id(DOSSIER),
            FromProject {
                tenant_id: TENANT.into(),
                project_id: PROJECT.into(),
                doc_type: None,
            },
        )
        .await
        .expect("added");
    assert!(added.is_project && added.state == "pending");
    cloud
        .dossier_remove_file(id(DOSSIER), id(FILE))
        .await
        .expect("taken out");
    let seen = seen.lock().unwrap();
    assert!(
        seen[0]
            .0
            .starts_with(&format!("POST /v1/dossiers/{DOSSIER}/actions HTTP/1.1")),
        "{}",
        seen[0].0
    );
    let step: Value = serde_json::from_slice(&seen[0].1).unwrap();
    assert_eq!(
        step,
        json!({ "action": "accept", "expectedStatus": "sent", "note": "Üstleniyorum.", "assigneeId": null,
                "attachments": [] })
    );
    assert!(
        seen[1]
            .0
            .starts_with(&format!("POST /v1/dossiers/{DOSSIER}/messages HTTP/1.1"))
    );
    let posted: Value = serde_json::from_slice(&seen[1].1).unwrap();
    assert_eq!(posted["body"], "Cephe paftalarını yarın yüklüyorum.");
    assert!(seen[2].0.starts_with(&format!(
        "POST /v1/dossiers/{DOSSIER}/files/from-project HTTP/1.1"
    )));
    let from: Value = serde_json::from_slice(&seen[2].1).unwrap();
    assert_eq!(
        from,
        json!({ "tenantId": TENANT, "projectId": PROJECT, "docType": null })
    );
    assert!(
        seen[3].0.starts_with(&format!(
            "DELETE /v1/dossiers/{DOSSIER}/files/{FILE} HTTP/1.1"
        )),
        "{}",
        seen[3].0
    );
    for (head, _) in seen.iter() {
        assert!(
            head.to_lowercase().contains("x-kentos-client: desktop"),
            "a change says it is the desktop's (the server's CSRF rule): {head}"
        );
    }
}

/// A file's bytes are taken only when they are the ones the server keeps (its SHA-256 as the entity tag); a KentOS
/// project file opens as a project whose ids the answer gives.
#[tokio::test]
async fn a_file_is_checked_and_a_project_file_opens_as_a_project() {
    let bytes = b"%PDF-1.7 cephe notlari".to_vec();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let file = Answer {
        status: 200,
        body: bytes.clone(),
        headers: vec![
            ("content-type", "application/pdf".into()),
            ("etag", format!("\"{sha}\"")),
        ],
    };
    let wrong = Answer {
        status: 200,
        body: bytes.clone(),
        headers: vec![("etag", format!("\"{}\"", "0".repeat(64)))],
    };
    let imported =
        json!({ "tenantId": TENANT, "projectId": PROJECT, "name": "Konut-412-7 (2026/0001)" });
    let (cloud, seen) = server(vec![
        file,
        wrong,
        json_answer(201, imported),
        json_answer(200, detail("sent", json!(["accept", "decline"]))),
    ])
    .await;
    let got = cloud
        .dossier_file(id(DOSSIER), id(FILE), None)
        .await
        .expect("read");
    assert_eq!(got.bytes, bytes);
    assert_eq!(got.sha256, sha);
    let refused = cloud
        .dossier_file(id(DOSSIER), id(FILE), None)
        .await
        .expect_err("another file than the server keeps");
    assert_eq!(refused.code, "corrupt");
    let made = cloud
        .dossier_import(id(DOSSIER), id(FILE), DossierFileImport { tenant_id: None })
        .await
        .expect("imported");
    assert_eq!(made.project_id, PROJECT);
    assert_eq!(made.name, "Konut-412-7 (2026/0001)");
    let page = cloud.dossier(id(DOSSIER)).await.expect("read");
    assert_eq!(
        page.actions,
        [DossierAction::Accept, DossierAction::Decline]
    );
    assert!(page.can_import && !page.can_add_files);
    let seen = seen.lock().unwrap();
    assert!(
        seen[0]
            .0
            .starts_with(&format!("GET /v1/dossiers/{DOSSIER}/files/{FILE} HTTP/1.1"))
    );
    assert!(seen[2].0.starts_with(&format!(
        "POST /v1/dossiers/{DOSSIER}/files/{FILE}/import HTTP/1.1"
    )));
    let into: Value = serde_json::from_slice(&seen[2].1).unwrap();
    assert_eq!(into, json!({ "tenantId": null }));
    assert!(
        seen[3]
            .0
            .starts_with(&format!("GET /v1/dossiers/{DOSSIER} HTTP/1.1"))
    );
}

/// A stale step is the server's conflict and comes back in its own words.
#[tokio::test]
async fn a_stale_step_says_the_servers_words() {
    let refused = json!({
        "error": "conflict",
        "message": "Dosya bu arada değişti (şimdi “Üstlenildi”); sayfayı yenileyip yeniden deneyin."
    });
    let (cloud, _) = server(vec![json_answer(409, refused)]).await;
    let failure = cloud
        .dossier_step(
            id(DOSSIER),
            DossierActionRequest {
                action: DossierAction::Accept,
                expected_status: DossierStatus::Sent,
                note: None,
                assignee_id: None,
                attachments: Vec::new(),
            },
        )
        .await
        .expect_err("refused");
    assert_eq!(failure.status, 409);
    assert_eq!(failure.code, "conflict");
    assert!(
        failure.message.contains("Dosya bu arada değişti"),
        "{failure}"
    );
}
