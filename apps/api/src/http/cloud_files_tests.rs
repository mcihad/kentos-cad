//! Files in the cloud as a phone or an agent uses them (docs/adr/9000 §13-§15): a bearer session sends a photo in
//! parts and goes on after a wrong offset, the project keeps it as an attachment that downloads whole, in part, in
//! place or not at all when unchanged; a program passed off as a document is refused and the platform's log says so;
//! a project's drawing comes down as a picture with a tag; an upload nobody takes goes after a day.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kentos_application::admin;
use kentos_contracts::{AttachmentView, ProjectInfo, ScanState, TenantRole, TokenIssued, UploadView};
use kentos_postgres::testing::TestDb;
use sha2::{Digest, Sha256};

use super::tests::{app_with, send};

const MINIMAL: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/minimal.kcad");

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn with(method: &str, uri: &str, token: &str, body: Body, kind: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, kind)
        .body(body)
        .unwrap()
}

fn json(method: &str, uri: &str, token: &str, value: serde_json::Value) -> Request<Body> {
    with(method, uri, token, Body::from(value.to_string()), "application/json")
}

fn bytes(method: &str, uri: &str, token: &str, b: &[u8]) -> Request<Body> {
    with(method, uri, token, Body::from(b.to_vec()), "application/octet-stream")
}

fn get(uri: &str, token: &str, extra: &[(header::HeaderName, &str)]) -> Request<Body> {
    let mut r = Request::get(uri).header(header::AUTHORIZATION, format!("Bearer {token}"));
    for (k, v) in extra {
        r = r.header(k, *v);
    }
    r.body(Body::empty()).unwrap()
}

async fn token(app: &axum::Router, login: &str) -> String {
    let (status, _, body) = send(
        app,
        json(
            "POST",
            "/v1/auth/token",
            "",
            serde_json::json!({ "login": login, "password": "dogru-parola-1", "client": "mobile", "label": "Saha telefonu" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{}", String::from_utf8_lossy(&body));
    serde_json::from_slice::<TokenIssued>(&body).unwrap().token
}

/// An organisation where `ayse` manages projects, another `can` owns; their bearer tokens; a store of the test's own.
async fn offices(db: &TestDb) -> (axum::Router, uuid::Uuid, String, String, kentos_application::blobs::Blobs) {
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3).await.unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 2).await.unwrap();
    for (login, t, role) in [("ayse", "buro", TenantRole::ProjectManager), ("can", "diger", TenantRole::Owner)] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1").await.unwrap();
        admin::set_membership(&db.owner, t, login, role, true).await.unwrap();
    }
    let blobs = kentos_application::blobs::Blobs::new(
        std::env::temp_dir().join(format!("kentos-cloud-files-{}", uuid::Uuid::now_v7())),
    );
    let router = app_with(Some(db.app.clone()), blobs.clone());
    let (ayse, can) = (token(&router, "ayse").await, token(&router, "can").await);
    (router, tenant, ayse, can, blobs)
}

async fn new_project(router: &axum::Router, tenant: uuid::Uuid, token: &str, storage: &str) -> String {
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    // A file project is a CAD type's, a database project a CBS one's (docs/adr/9196 §2).
    let kind = if storage == "file" { "cad" } else { "gis" };
    let create = serde_json::json!({ "name": format!("Ada 7 ({storage})"), "projectType": kind, "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] },
        "storage": storage });
    let (status, _, body) =
        send(router, json("POST", &format!("/v1/tenants/{tenant}/projects"), token, create)).await;
    assert_eq!(status, StatusCode::CREATED, "{}", String::from_utf8_lossy(&body));
    serde_json::from_slice::<ProjectInfo>(&body).unwrap().id
}

/// Sends `content` as `name` through `/v1/uploads` in two parts; the received upload.
async fn uploaded(router: &axum::Router, token: &str, name: &str, content: &[u8]) -> Result<UploadView, (StatusCode, String)> {
    let (status, _, body) = send(
        router,
        json("POST", "/v1/uploads", token, serde_json::json!({ "name": name, "size": content.len(), "sha256": sha(content) })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{}", String::from_utf8_lossy(&body));
    let upload: UploadView = serde_json::from_slice(&body).unwrap();
    assert_eq!(upload.part_max, 32 * 1024 * 1024);
    let cut = content.len() / 2;
    let mut last = upload;
    for (offset, part) in [(0, &content[..cut]), (cut, &content[cut..])] {
        let (status, _, body) =
            send(router, bytes("PUT", &format!("/v1/uploads/{}?offset={offset}", last.id), token, part)).await;
        if status != StatusCode::OK {
            return Err((status, String::from_utf8_lossy(&body).into_owned()));
        }
        last = serde_json::from_slice(&body).unwrap();
    }
    Ok(last)
}

#[tokio::test]
async fn a_phone_sends_a_photo_in_parts_and_the_project_keeps_it() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (router, tenant, ayse, can, _) = offices(&db).await;
    let project = new_project(&router, tenant, &ayse, "database").await;
    let base = format!("/v1/tenants/{tenant}/projects/{project}");
    let mut photo = vec![0xff, 0xd8, 0xff, 0xe0];
    photo.extend((0..70_000u32).map(|i| (i % 251) as u8));

    // A part from the wrong place is refused and changes nothing; the upload says where to go on from.
    let (_, _, body) = send(
        &router,
        json("POST", "/v1/uploads", &ayse, serde_json::json!({ "name": "saha-fotografi.JPG", "size": photo.len(), "sha256": sha(&photo) })),
    )
    .await;
    let open: UploadView = serde_json::from_slice(&body).unwrap();
    let (status, _, _) = send(&router, bytes("PUT", &format!("/v1/uploads/{}?offset=0", open.id), &ayse, &photo[..40_000])).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, body) = send(&router, bytes("PUT", &format!("/v1/uploads/{}?offset=0", open.id), &ayse, &photo[40_000..])).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{}", String::from_utf8_lossy(&body));
    let (_, _, body) = send(&router, get(&format!("/v1/uploads/{}", open.id), &ayse, &[])).await;
    let half: UploadView = serde_json::from_slice(&body).unwrap();
    assert_eq!((half.received_bytes, half.received), (40_000, false));
    // Nobody else sees it.
    assert_eq!(send(&router, get(&format!("/v1/uploads/{}", open.id), &can, &[])).await.0, StatusCode::NOT_FOUND);
    let (status, _, body) =
        send(&router, bytes("PUT", &format!("/v1/uploads/{}?offset=40000", open.id), &ayse, &photo[40_000..])).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let done: UploadView = serde_json::from_slice(&body).unwrap();
    assert!(done.received);
    assert_eq!((done.scan, done.media_type.as_str()), (Some(ScanState::Unscanned), "image/jpeg"));

    // The project takes it once.
    let (status, _, body) = send(
        &router,
        json("POST", &format!("{base}/attachments"), &ayse, serde_json::json!({ "uploadId": done.id, "note": "Kuzey cephe" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{}", String::from_utf8_lossy(&body));
    let kept: AttachmentView = serde_json::from_slice(&body).unwrap();
    assert!(kept.viewable && kept.size == photo.len() as u64 && kept.note.as_deref() == Some("Kuzey cephe"));
    let (status, _, _) = send(
        &router,
        json("POST", &format!("{base}/attachments"), &ayse, serde_json::json!({ "uploadId": done.id })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "an upload is taken once");
    let (_, _, body) = send(&router, get(&format!("{base}/attachments"), &ayse, &[])).await;
    assert_eq!(serde_json::from_slice::<Vec<AttachmentView>>(&body).unwrap().len(), 1);

    // Whole, as a download: never shown in place unless asked, never sniffed.
    let at = format!("{base}/attachments/{}", kept.id);
    let (status, h, body) = send(&router, get(&at, &ayse, &[])).await;
    assert_eq!((status, body.as_ref()), (StatusCode::OK, photo.as_slice()));
    let tag = format!("\"{}\"", sha(&photo));
    assert_eq!(h[header::ETAG], tag.as_str());
    assert_eq!(h[header::CONTENT_TYPE], "application/octet-stream");
    assert!(h[header::CONTENT_DISPOSITION].to_str().unwrap().starts_with("attachment;"));
    assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    let (_, h, _) = send(&router, get(&format!("{at}?inline=1"), &ayse, &[])).await;
    assert_eq!(h[header::CONTENT_TYPE], "image/jpeg");
    assert!(h[header::CONTENT_DISPOSITION].to_str().unwrap().starts_with("inline;"));
    // Unchanged: 304; a part: 206.
    let (status, _, body) = send(&router, get(&at, &ayse, &[(header::IF_NONE_MATCH, &tag)])).await;
    assert_eq!((status, body.len()), (StatusCode::NOT_MODIFIED, 0));
    let (status, h, body) = send(&router, get(&at, &ayse, &[(header::RANGE, "bytes=0-3")])).await;
    assert_eq!((status, body.as_ref()), (StatusCode::PARTIAL_CONTENT, &photo[..4]));
    assert_eq!(h[header::CONTENT_RANGE], format!("bytes 0-3/{}", photo.len()).as_str());

    // Another organisation finds nothing; the manager removes it.
    assert_eq!(send(&router, get(&format!("{base}/attachments"), &can, &[])).await.0, StatusCode::NOT_FOUND);
    let (status, _, _) = send(&router, with("DELETE", &at, &ayse, Body::empty(), "application/json")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(send(&router, get(&at, &ayse, &[])).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_disguised_program_is_refused_and_an_untaken_upload_goes() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (router, _, ayse, _, blobs) = offices(&db).await;
    let (status, message) = uploaded(&router, &ayse, "rapor.pdf", b"MZ\x90\0\x03\0\0\0 bir program").await.unwrap_err();
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(message.contains("program"), "{message}");
    let logged: Vec<(String, String)> =
        sqlx::query_as("select target_label, detail->>'verdict' from kentos.platform_event where action = 'file.refused'")
            .fetch_all(&db.owner)
            .await
            .unwrap();
    assert_eq!(logged, vec![("rapor.pdf".to_owned(), "blocked".to_owned())]);

    // A received upload nobody takes is removed with its bytes after its day.
    let kept = uploaded(&router, &ayse, "notlar.txt", "Ada 7, parsel 5: köşe taşları yerinde.".as_bytes()).await.unwrap();
    assert_eq!(kept.media_type, "text/plain");
    sqlx::query("update kentos.upload set created_at = now() - interval '25 hours'")
        .execute(&db.owner)
        .await
        .unwrap();
    assert_eq!(kentos_application::uploads::expire(&db.app, &blobs).await.unwrap(), 1);
    assert_eq!(send(&router, get(&format!("/v1/uploads/{}", kept.id), &ayse, &[])).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_projects_drawing_comes_down_as_a_picture_with_a_tag() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (router, tenant, ayse, can, _) = offices(&db).await;
    // A file project: its newest revision, drawn once and kept.
    let project = new_project(&router, tenant, &ayse, "file").await;
    let base = format!("/v1/tenants/{tenant}/projects/{project}");
    assert_eq!(send(&router, get(&format!("{base}/preview.svg"), &ayse, &[])).await.0, StatusCode::NOT_FOUND);
    let (_, _, body) = send(
        &router,
        json("POST", &format!("{base}/uploads"), &ayse, serde_json::json!({ "size": MINIMAL.len(), "sha256": sha(MINIMAL) })),
    )
    .await;
    let upload: kentos_contracts::FileUpload = serde_json::from_slice(&body).unwrap();
    let (status, _, _) = send(&router, bytes("PUT", &format!("{base}/uploads/{}", upload.id), &ayse, MINIMAL)).await;
    assert_eq!(status, StatusCode::OK);
    let envelope = kentos_contracts::CommandEnvelope {
        command_name: "project.file.commit".into(),
        version: 1,
        tenant_id: tenant.to_string(),
        project_id: project.clone(),
        request_id: "istek-onizleme-1".into(),
        idempotency_key: uuid::Uuid::now_v7().to_string(),
        expected_versions: [("@file".to_string(), "0".to_string())].into_iter().collect(),
        input: serde_json::json!({ "uploadId": upload.id }),
    };
    let (status, _, body) =
        send(&router, json("POST", &format!("{base}/commands"), &ayse, serde_json::to_value(&envelope).unwrap())).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (status, h, body) = send(&router, get(&format!("{base}/preview.svg?width=640"), &ayse, &[])).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(h[header::CONTENT_TYPE], "image/svg+xml; charset=utf-8");
    let tag = h[header::ETAG].to_str().unwrap().to_owned();
    assert!(tag.starts_with("\"r1-") && tag.ends_with("-w640\""), "{tag}");
    assert!(body.starts_with(b"<svg"));
    let (status, _, again) = send(&router, get(&format!("{base}/preview.svg?width=640"), &ayse, &[])).await;
    assert_eq!((status, again), (StatusCode::OK, body), "the kept picture is the drawn one");
    let (status, _, _) = send(&router, get(&format!("{base}/preview.svg?width=640"), &ayse, &[(header::IF_NONE_MATCH, &tag)])).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert_eq!(send(&router, get(&format!("{base}/preview.svg"), &can, &[])).await.0, StatusCode::NOT_FOUND);

    // A database project: its objects as they are now.
    let project = new_project(&router, tenant, &ayse, "database").await;
    let (status, h, body) =
        send(&router, get(&format!("/v1/tenants/{tenant}/projects/{project}/preview.svg"), &ayse, &[])).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(h[header::ETAG].to_str().unwrap().starts_with("\"d"));
    assert_eq!(h["x-kentos-drawn"], "0");
}
