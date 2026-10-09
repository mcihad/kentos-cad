//! Temporal layers and scenarios on the server (docs/adr/0210 §2), end to end
//! with a throwaway database: a project is created with a temporal layer and
//! a scenario standing for a base layer, and gives them back; a tree that
//! breaks their rules is refused when a project is created and in
//! `project.changes`' layers, and a refused commit changes nothing.

use axum::http::StatusCode;
use kentos_application::admin;
use kentos_contracts::{ApiError, ProjectInfo, TenantRole};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};

use super::tests::{app, get, json_req, send, signed_in};

#[tokio::test]
async fn a_projects_times_and_scenarios_are_checked_and_kept_over_http() {
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
    let layer = |id: &str, name: &str, extra: Value| {
        let mut n = json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": style, "children": [] });
        n.as_object_mut()
            .unwrap()
            .extend(extra.as_object().cloned().unwrap_or_default());
        n
    };
    let tree = |end: &str, replaces: &str| {
        json!([
            { "id": "alt-a", "name": "Öneri A", "type": "group", "visible": false, "locked": false, "expanded": true,
              "style": style, "scenario": { "note": "12 m'lik yol" },
              "children": [layer("alt-a-yol", "Yol", json!({ "replaces": replaces }))] },
            layer("yol", "Yol", json!({})),
            layer("parsel", "Parsel", json!({ "time": { "start": "baslangic", "end": end, "key": "parsel_no" } })),
        ])
    };
    let settings = json!({ "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 });
    let create = |layers: Value| {
        json!({ "name": "Zamanlı parseller", "settings": settings, "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": layers,
            "activeLayer": "parsel", "styles": { "items": [], "categories": [] } })
    };
    let refused = |body: &[u8], words: &str| {
        let e: ApiError = serde_json::from_slice(body).unwrap();
        assert!(e.message.contains(words), "{}", e.message);
    };
    let base = format!("/v1/tenants/{tenant}/projects");

    // A time whose end is its start: no project.
    let (status, _, body) = send(
        &router,
        json_req("POST", &base, &ayse, create(tree("baslangic", "yol"))),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "başlangıç ve bitiş aynı alan olamaz");

    // A good tree is kept: the created project says it and so does its read.
    let (status, _, body) = send(
        &router,
        json_req("POST", &base, &ayse, create(tree("bitis", "yol"))),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let info: ProjectInfo = serde_json::from_slice(&body).unwrap();
    let parcels = info.layers.iter().find(|n| n.id == "parsel").unwrap();
    assert_eq!(
        parcels.time.as_ref().map(|t| t.start.as_str()),
        Some("baslangic")
    );
    let scenario = &info.layers[0];
    assert_eq!(
        scenario.scenario.as_ref().and_then(|s| s.note.as_deref()),
        Some("12 m'lik yol")
    );
    assert_eq!(scenario.children[0].replaces.as_deref(), Some("yol"));
    let project = info.id.clone();
    let read = |body: &[u8]| serde_json::from_slice::<ProjectInfo>(body).unwrap();
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    assert_eq!(read(&body).layers, info.layers);

    // A scenario layer standing for itself, in project.changes: refused, the stored tree stays.
    let commit = |layers: Value, meta: &str| {
        json_req(
            "POST",
            &format!("{base}/{project}/commands"),
            &ayse,
            json!({ "commandName": "project.changes", "version": 1, "tenantId": tenant.to_string(), "projectId": project,
                "requestId": "zaman", "idempotencyKey": uuid::Uuid::new_v4().to_string(), "expectedVersions": { "@project": meta },
                "input": { "features": [], "project": { "layers": layers } } }),
        )
    };
    let (status, _, body) = send(
        &router,
        commit(tree("bitis", "alt-a-yol"), &info.meta_version),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    refused(&body, "kendi yerine geçemez");
    let (_, _, body) = send(&router, get(&format!("{base}/{project}"), &ayse)).await;
    let now = read(&body);
    assert_eq!(now.layers, info.layers);
    assert_eq!(now.meta_version, info.meta_version);
}
