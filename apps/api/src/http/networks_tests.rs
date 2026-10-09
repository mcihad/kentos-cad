//! A project's networks on the server (docs/adr/0209 §2), end to end with a
//! throwaway database: a project is created with its networks and gives them
//! back; a network that breaks the contract's rules is refused at the field it
//! is in, when a project is created and in `project.changes`' settings, and a
//! refused commit changes nothing; settings without networks take them away.

use axum::http::StatusCode;
use kentos_application::admin;
use kentos_contracts::{ApiError, ProjectInfo, TenantRole};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};

use super::tests::{app, get, json_req, send, signed_in};

#[tokio::test]
async fn a_projects_networks_are_checked_and_kept_over_http() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    admin::create_local_user(&db.owner, "ayse", "Ayşe Yılmaz", None, "dogru-parola-1")
        .await
        .unwrap();
    admin::set_membership(&db.owner, "buro", "ayse", TenantRole::ProjectManager, true)
        .await
        .unwrap();
    let router = app(Some(db.app.clone()));
    let ayse = signed_in(&router, "ayse").await;

    let network = |id: &str, name: &str, tolerance: f64| {
        json!({ "id": id, "name": name, "kind": "road", "edges": [{ "layer": "yol" }], "connect": "ends", "tolerance": tolerance,
            "direction": { "kind": "both" }, "costs": [{ "name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0 }] })
    };
    let settings = |networks: Value| {
        json!({ "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
            "networks": networks })
    };
    let layer = json!({ "id": "yol", "name": "Yol", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = |networks: Value| {
        json!({ "name": "Yol ağı", "settings": settings(networks), "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer],
            "activeLayer": "yol", "styles": { "items": [], "categories": [] } })
    };
    let refused = |body: &[u8], path: &str| {
        let e: ApiError = serde_json::from_slice(body).unwrap();
        assert_eq!(e.path.as_deref(), Some(path), "{}", e.message);
        assert!(
            e.message.starts_with("Projenin ağları geçersiz: "),
            "{}",
            e.message
        );
    };
    let base = format!("/v1/tenants/{tenant}/projects");

    // A tolerance out of its range: no project.
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &base,
            &ayse,
            create(json!([network("yollar", "Yollar", 0.0)])),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "settings.networks");

    // A good network is kept: the created project says it and so does its read.
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &base,
            &ayse,
            create(json!([network("yollar", "Yollar", 0.01)])),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let info: ProjectInfo = serde_json::from_slice(&body).unwrap();
    assert_eq!(info.settings.networks.len(), 1);
    assert_eq!(info.settings.networks[0].name, "Yollar");
    assert_eq!(info.settings.networks[0].costs[0].name, "Süre");
    let project = info.id.clone();
    let read = |body: &[u8]| serde_json::from_slice::<ProjectInfo>(body).unwrap();
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    assert_eq!(read(&body).settings.networks, info.settings.networks);

    let commit = |networks: Value, meta: &str| {
        json_req(
            "POST",
            &format!("{base}/{project}/commands"),
            &ayse,
            json!({ "commandName": "project.changes", "version": 1, "tenantId": tenant.to_string(), "projectId": project,
                "requestId": "aglar", "idempotencyKey": uuid::Uuid::new_v4().to_string(), "expectedVersions": { "@project": meta },
                "input": { "features": [], "project": { "settings": settings(networks) } } }),
        )
    };
    // Two networks of one name (letters' case aside) in project.changes: refused, the stored one stays.
    let twice = json!([
        network("yollar", "Yollar", 0.01),
        network("ana-yollar", "YOLLAR", 0.01)
    ]);
    let (status, _, body) = send(&router, commit(twice, &info.meta_version)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "project.settings.networks");
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    let now = read(&body);
    assert_eq!(now.settings.networks, info.settings.networks);
    assert_eq!(now.meta_version, info.meta_version);

    // Settings without networks take them away.
    let (status, _, body) = send(&router, commit(json!([]), &info.meta_version)).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    assert!(read(&body).settings.networks.is_empty());
}
