//! Layer compositions on the server (docs/adr/0226 §3, §4), end to end with
//! a throwaway database: a project is created with a layer's opacity, mode
//! and mask and gives them back; a composition on a group is refused in
//! `project.changes`' layers and a refused commit changes nothing; a mask
//! whose target another editor deleted is kept, not refused.

use axum::http::StatusCode;
use kentos_application::admin;
use kentos_contracts::{ApiError, BlendMode, ProjectInfo, TenantRole};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};

use super::tests::{app, get, json_req, send, signed_in};

#[tokio::test]
async fn a_layers_composition_is_checked_and_kept_over_http() {
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

    let style = json!({ "color": "ink", "lineType": "continuous", "lineWeight": 0.25 });
    let tree = |compose: Value| {
        json!([
            { "id": "altlik", "name": "Altlık", "type": "layer", "visible": true, "locked": false, "expanded": true,
              "style": style, "children": [] },
            { "id": "parsel", "name": "Parsel", "type": "layer", "visible": true, "locked": false, "expanded": true,
              "style": style, "children": [], "compose": compose },
        ])
    };
    let settings = json!({ "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 });
    let create = |layers: Value| {
        json!({ "name": "Katman çizimi", "settings": settings, "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": layers,
            "activeLayer": "parsel", "styles": { "items": [], "categories": [] } })
    };
    let refused = |body: &[u8], words: &str| {
        let e: ApiError = serde_json::from_slice(body).unwrap();
        assert!(e.message.contains(words), "{}", e.message);
    };
    let base = format!("/v1/tenants/{tenant}/projects");

    // A mask without a source: no project.
    let blind = json!({ "mask": { "size": 1.0, "targets": ["altlik"] } });
    let (status, _, body) =
        send(&router, json_req("POST", &base, &ayse, create(tree(blind)))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "maskenin kaynağı yok");

    // A good composition is kept: the created project says it and so does its read.
    let good = json!({ "opacity": 0.6, "blend": "multiply", "mask": { "labels": true, "size": 1.0, "targets": ["altlik"] } });
    let (status, _, body) = send(&router, json_req("POST", &base, &ayse, create(tree(good)))).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let info: ProjectInfo = serde_json::from_slice(&body).unwrap();
    let compose = info.layers[1]
        .compose
        .as_ref()
        .expect("the composition is kept");
    assert_eq!(compose.opacity, Some(0.6));
    assert_eq!(compose.blend, Some(BlendMode::Multiply));
    let project = info.id.clone();
    let read = |body: &[u8]| serde_json::from_slice::<ProjectInfo>(body).unwrap();
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    assert_eq!(read(&body).layers, info.layers);

    // A composition on a group, in project.changes: refused, the stored tree stays.
    let grouped = json!([
        { "id": "altlik", "name": "Altlık", "type": "group", "visible": true, "locked": false, "expanded": true,
          "style": style, "children": [], "compose": { "opacity": 0.5 } },
        { "id": "parsel", "name": "Parsel", "type": "layer", "visible": true, "locked": false, "expanded": true,
          "style": style, "children": [] },
    ]);
    let commit = |layers: Value, meta: &str| {
        json_req(
            "POST",
            &format!("{base}/{project}/commands"),
            &ayse,
            json!({ "commandName": "project.changes", "version": 1, "tenantId": tenant.to_string(), "projectId": project,
                "requestId": uuid::Uuid::new_v4().to_string(), "idempotencyKey": uuid::Uuid::new_v4().to_string(),
                "expectedVersions": { "@project": meta }, "input": { "features": [], "project": { "layers": layers } } }),
        )
    };
    let (status, _, body) = send(&router, commit(grouped, &info.meta_version)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "katman çizimi yalnız katmanın olur");
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    let now = read(&body);
    assert_eq!(now.layers, info.layers);
    assert_eq!(now.meta_version, info.meta_version);

    // Another editor deleted the mask's target: the tree is kept as it is.
    let orphan = json!([
        { "id": "parsel", "name": "Parsel", "type": "layer", "visible": true, "locked": false, "expanded": true,
          "style": style, "children": [], "compose": { "mask": { "labels": true, "size": 1.0, "targets": ["altlik"] } } },
    ]);
    let (status, _, body) = send(&router, commit(orphan, &info.meta_version)).await;
    assert!(status.is_success(), "{}", String::from_utf8_lossy(&body));
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    let mask = read(&body).layers[0]
        .compose
        .clone()
        .and_then(|c| c.mask)
        .expect("the mask is kept");
    assert_eq!(mask.targets, ["altlik"]);
}
