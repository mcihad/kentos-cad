//! Layer filters on the server (docs/adr/0211 §2), end to end with a
//! throwaway database: a project is created with a filtered layer and gives
//! its filter back; a tree whose filter breaks its rules is refused when a
//! project is created and in `project.changes`' layers, and a refused commit
//! changes nothing.

use axum::http::StatusCode;
use kentos_application::admin;
use kentos_contracts::{ApiError, ProjectInfo, TenantRole};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};

use super::tests::{app, get, json_req, send, signed_in};

#[tokio::test]
async fn a_layers_filter_is_checked_and_kept_over_http() {
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
    let tree = |filter: Value| {
        json!([
            { "id": "parsel", "name": "Parsel", "type": "layer", "visible": true, "locked": false, "expanded": true,
              "style": style, "children": [], "filter": filter },
        ])
    };
    let settings = json!({ "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 });
    let create = |layers: Value| {
        json!({ "name": "Süzgeçli parseller", "settings": settings, "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": layers,
            "activeLayer": "parsel", "styles": { "items": [], "categories": [] } })
    };
    let refused = |body: &[u8], words: &str| {
        let e: ApiError = serde_json::from_slice(body).unwrap();
        assert!(e.message.contains(words), "{}", e.message);
    };
    let base = format!("/v1/tenants/{tenant}/projects");

    // A filter that keeps nothing: no project.
    let (status, _, body) = send(
        &router,
        json_req("POST", &base, &ayse, create(tree(json!({})))),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "ne ifade ne nesne listesi");

    // A good filter is kept: the created project says it and so does its read.
    let good = json!({ "expression": "Nitelik = 'Arsa'", "objects": ["0192f5a1-7777-7000-8000-000000000601"] });
    let (status, _, body) = send(&router, json_req("POST", &base, &ayse, create(tree(good)))).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let info: ProjectInfo = serde_json::from_slice(&body).unwrap();
    let filter = info.layers[0].filter.as_ref().expect("the filter is kept");
    assert_eq!(filter.expression.as_deref(), Some("Nitelik = 'Arsa'"));
    assert_eq!(filter.objects.len(), 1);
    let project = info.id.clone();
    let read = |body: &[u8]| serde_json::from_slice::<ProjectInfo>(body).unwrap();
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    assert_eq!(read(&body).layers, info.layers);

    // The same object twice in the list, in project.changes: refused, the stored tree stays.
    let commit = |layers: Value, meta: &str| {
        json_req(
            "POST",
            &format!("{base}/{project}/commands"),
            &ayse,
            json!({ "commandName": "project.changes", "version": 1, "tenantId": tenant.to_string(), "projectId": project,
                "requestId": "suzgec", "idempotencyKey": uuid::Uuid::new_v4().to_string(), "expectedVersions": { "@project": meta },
                "input": { "features": [], "project": { "layers": layers } } }),
        )
    };
    let twice = json!({ "objects": ["0192f5a1-7777-7000-8000-000000000601", "0192f5a1-7777-7000-8000-000000000601"] });
    let (status, _, body) = send(&router, commit(tree(twice), &info.meta_version)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "iki kez var");
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    let now = read(&body);
    assert_eq!(now.layers, info.layers);
    assert_eq!(now.meta_version, info.meta_version);
}
