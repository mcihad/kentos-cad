//! Server jobs over HTTP (docs/adr/0542): asked for once whatever the retries, run by a worker, listed, read and their
//! file downloaded; cancelled; someone of another organisation finds the routes missing as for no project.

use axum::http::StatusCode;
use kentos_application::{admin, jobs};
use kentos_contracts::{ProjectInfo, ServerJob, ServerJobList, ServerJobState, TenantRole};
use kentos_postgres::testing::TestDb;
use sha2::{Digest, Sha256};

use super::tests::{app_with, get, json_req, not_found_body, send, signed_in};

#[tokio::test]
async fn a_job_is_asked_run_and_downloaded_over_http() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap()
        .to_string();
    admin::create_tenant(&db.owner, "diger", "Diğer", 2)
        .await
        .unwrap();
    for (login, t, role) in [
        ("ayse", "buro", TenantRole::ProjectManager),
        ("can", "diger", TenantRole::Owner),
    ] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, t, login, role, true)
            .await
            .unwrap();
    }
    let store = kentos_application::blobs::Blobs::new(
        std::env::temp_dir().join(format!("kentos-api-jobs-{}", uuid::Uuid::now_v7())),
    );
    let router = app_with(Some(db.app.clone()), store.clone());
    let (ayse, can) = (
        signed_in(&router, "ayse").await,
        signed_in(&router, "can").await,
    );
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Ada 3", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("/v1/tenants/{tenant}/projects"),
            &ayse,
            create,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let project = serde_json::from_slice::<ProjectInfo>(&body).unwrap().id;
    let base = format!("/v1/tenants/{tenant}/projects/{project}");

    let ask = serde_json::json!({ "kind": "project.export", "input": {}, "requestId": "istek-1" });
    let (status, _, body) = send(
        &router,
        json_req("POST", &format!("{base}/jobs"), &ayse, ask.clone()),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let job: ServerJob = serde_json::from_slice(&body).unwrap();
    assert_eq!(job.state, ServerJobState::Queued);
    let (_, _, body) = send(
        &router,
        json_req("POST", &format!("{base}/jobs"), &ayse, ask),
    )
    .await;
    assert_eq!(
        serde_json::from_slice::<ServerJob>(&body).unwrap().id,
        job.id
    );
    let bad = serde_json::json!({ "kind": "yok", "requestId": "istek-2" });
    let (status, _, _) = send(
        &router,
        json_req("POST", &format!("{base}/jobs"), &ayse, bad),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    jobs::work_once(
        &db.app,
        &store,
        uuid::Uuid::now_v7(),
        std::time::Duration::from_secs(30),
        &Default::default(),
    )
    .await
    .unwrap()
    .expect("a job ran");
    let one = format!("{base}/jobs/{}", job.id);
    let (_, _, body) = send(&router, get(&one, &ayse)).await;
    let done: ServerJob = serde_json::from_slice(&body).unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded);
    let artifact = done.artifact.unwrap();
    let (status, headers, body) = send(&router, get(&format!("{one}/artifact"), &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    let digest: String = Sha256::digest(&body)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(digest, artifact.sha256);
    assert_eq!(kentos_kcad::decode(&body).unwrap().name, "Ada 3");
    assert_eq!(
        headers["x-kentos-revision"].to_str().unwrap(),
        done.base_revision
    );
    // A download cut short goes on from where it stopped (`Range: bytes=N-`): the rest comes with 206, the same
    // entity tag, and the two parts make the file; a start past the end is refused.
    let cut = body.len() / 3;
    let mut ranged = get(&format!("{one}/artifact"), &ayse);
    ranged.headers_mut().insert(
        axum::http::header::RANGE,
        format!("bytes={cut}-").parse().unwrap(),
    );
    let (status, part_headers, rest) = send(&router, ranged).await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(part_headers["etag"], headers["etag"]);
    assert_eq!(
        part_headers["content-range"].to_str().unwrap(),
        format!("bytes {cut}-{}/{}", body.len() - 1, body.len())
    );
    assert_eq!([&body[..cut], &rest[..]].concat(), body);
    let mut past = get(&format!("{one}/artifact"), &ayse);
    past.headers_mut().insert(
        axum::http::header::RANGE,
        format!("bytes={}-", body.len()).parse().unwrap(),
    );
    let (status, _, _) = send(&router, past).await;
    assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
    let (_, _, body) = send(&router, get(&format!("{base}/jobs"), &ayse)).await;
    assert_eq!(
        serde_json::from_slice::<ServerJobList>(&body)
            .unwrap()
            .jobs
            .len(),
        1
    );

    // A queued job cancelled; its file is not there to download.
    let ask = serde_json::json!({ "kind": "project.export", "requestId": "istek-3" });
    let (_, _, body) = send(
        &router,
        json_req("POST", &format!("{base}/jobs"), &ayse, ask),
    )
    .await;
    let queued: ServerJob = serde_json::from_slice(&body).unwrap();
    let path = format!("{base}/jobs/{}", queued.id);
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("{path}/cancel"),
            &ayse,
            serde_json::json!({}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(
        serde_json::from_slice::<ServerJob>(&body).unwrap().state,
        ServerJobState::Cancelled
    );
    let (status, _, _) = send(&router, get(&format!("{path}/artifact"), &ayse)).await;
    assert_ne!(status, StatusCode::OK);

    // Someone of another organisation finds nothing, as for no project.
    let missing = not_found_body(
        &router,
        get(
            &format!("/v1/tenants/{tenant}/projects/{}", uuid::Uuid::now_v7()),
            &can,
        ),
    )
    .await;
    for uri in [
        format!("{base}/jobs"),
        one.clone(),
        format!("{one}/artifact"),
    ] {
        assert_eq!(not_found_body(&router, get(&uri, &can)).await, missing);
    }
    db.close().await;
}

/// A processor that makes a GeoPackage of a few words and says where it is on the way (docs/adr/0559).
struct Words;

impl kentos_application::processing_job::Processor for Words {
    fn process(
        &self,
        task: &kentos_application::processing_job::ProcessingTask,
        hooks: &kentos_application::processing_job::ProcessHooks,
    ) -> Result<
        kentos_application::processing_job::Processed,
        kentos_application::processing_job::ProcessFailure,
    > {
        hooks.progress.set(0.7, "Araç çalışıyor");
        assert_eq!(task.input.tool, "geometry.buffer");
        Ok(kentos_application::processing_job::Processed {
            summary: "3 tampon yapıldı".into(),
            sink: Some(b"GPKG-bytes".to_vec()),
            sink_rows: 3,
            ..Default::default()
        })
    }
}

#[tokio::test]
async fn a_processing_job_is_asked_checked_run_and_its_file_downloaded_over_http() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap()
        .to_string();
    admin::create_tenant(&db.owner, "diger", "Diğer", 2)
        .await
        .unwrap();
    for (login, t, role) in [
        ("ayse", "buro", TenantRole::ProjectManager),
        ("can", "diger", TenantRole::Owner),
    ] {
        admin::create_local_user(&db.owner, login, login, None, "dogru-parola-1")
            .await
            .unwrap();
        admin::set_membership(&db.owner, t, login, role, true)
            .await
            .unwrap();
    }
    let store = kentos_application::blobs::Blobs::new(
        std::env::temp_dir().join(format!("kentos-api-jobs-{}", uuid::Uuid::now_v7())),
    );
    let router = app_with(Some(db.app.clone()), store.clone());
    let (ayse, can) = (
        signed_in(&router, "ayse").await,
        signed_in(&router, "can").await,
    );
    let layer = serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] });
    let create = serde_json::json!({ "name": "Ada 3", "settings": { "srid": 5256, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
        "origin": { "x": 486500.0, "y": 4420200.0 }, "layers": [layer], "activeLayer": "cizim", "styles": { "items": [], "categories": [] } });
    let (_, _, body) = send(
        &router,
        json_req(
            "POST",
            &format!("/v1/tenants/{tenant}/projects"),
            &ayse,
            create,
        ),
    )
    .await;
    let project = serde_json::from_slice::<ProjectInfo>(&body).unwrap().id;
    let base = format!("/v1/tenants/{tenant}/projects/{project}/jobs");
    let ask = |request: &str, input: serde_json::Value| {
        serde_json::json!({ "kind": "project.processing", "input": input, "requestId": request })
    };

    // A local file in a value is refused where it is, in the request's own words.
    let (status, _, body) = send(
        &router,
        json_req(
            "POST",
            &base,
            &ayse,
            ask(
                "d-1",
                serde_json::json!({ "tool": "geometry.buffer", "parameters": { "dosya": { "path": "/etc/passwd" } } }),
            ),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let refused: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(refused["path"], "input.parameters.dosya.path");
    // An input that is not the kind's.
    let (status, _, _) = send(
        &router,
        json_req(
            "POST",
            &base,
            &ayse,
            ask("d-2", serde_json::json!({ "arac": "x" })),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Asked once whatever the retries; a worker with a processor runs it.
    let input = serde_json::json!({ "tool": "geometry.buffer", "label": "Tampon", "mode": "sink" });
    let (status, _, body) = send(
        &router,
        json_req("POST", &base, &ayse, ask("d-3", input.clone())),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let job: ServerJob = serde_json::from_slice(&body).unwrap();
    assert_eq!(job.kind, "project.processing");
    let (_, _, body) = send(&router, json_req("POST", &base, &ayse, ask("d-3", input))).await;
    assert_eq!(
        serde_json::from_slice::<ServerJob>(&body).unwrap().id,
        job.id
    );
    // A worker with no processor leaves it queued.
    assert!(
        jobs::work_once(
            &db.app,
            &store,
            uuid::Uuid::now_v7(),
            std::time::Duration::from_secs(30),
            &Default::default(),
        )
        .await
        .unwrap()
        .is_none()
    );
    let processor: std::sync::Arc<dyn kentos_application::processing_job::Processor> =
        std::sync::Arc::new(Words);
    jobs::work_once_with(
        &db.app,
        &store,
        uuid::Uuid::now_v7(),
        std::time::Duration::from_secs(30),
        &Default::default(),
        Some(processor),
    )
    .await
    .unwrap()
    .expect("the job ran");
    let one = format!("{base}/{}", job.id);
    let (_, _, body) = send(&router, get(&one, &ayse)).await;
    let done: ServerJob = serde_json::from_slice(&body).unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded, "{:?}", done.error);
    let result = done.result.clone().unwrap();
    assert_eq!(
        (
            result["tool"].as_str(),
            result["summary"].as_str(),
            result["added"].as_u64()
        ),
        (Some("geometry.buffer"), Some("3 tampon yapıldı"), Some(3))
    );
    // Its file is the processing job's GeoPackage, named by the project and the tool.
    let (status, headers, bytes) = send(&router, get(&format!("{one}/artifact"), &ayse)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&bytes[..], b"GPKG-bytes");
    let disposition = headers["content-disposition"].to_str().unwrap();
    assert!(disposition.contains("Ada 3 - Tampon.gpkg"), "{disposition}");
    // Another organisation finds the job as it finds no project.
    let (status, _, _) = send(&router, get(&one, &can)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    db.close().await;
}
