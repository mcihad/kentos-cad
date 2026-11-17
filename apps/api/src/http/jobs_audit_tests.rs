//! `project.audit` over HTTP (docs/adr/8057 §42): asked like any job, run by a worker whose processor answers with the
//! job runner's report, kept as the result; a second identical request answers from the cache without running; a
//! version without a prepared interpreter ends with `tool_unavailable`. Needs the tests' database (skipped without).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::http::StatusCode;
use kentos_application::audit_catalog::{AuditFuture, AuditPackage, AuditSource};
use kentos_application::processing_job::{
    ProcessFailure, ProcessHooks, Processed, ProcessingTask, Processor, RunRequest,
};
use kentos_application::{admin, jobs};
use kentos_contracts::audit_job::{AuditReport, Finding, RuleRef, Severity};
use kentos_contracts::{ProjectInfo, ServerJob, ServerJobState, TenantRole};
use kentos_postgres::testing::TestDb;

use super::tests::{app_with, get, json_req, send, signed_in};

const DIGEST: &str = "sha256:abababababababababababababababababababababababababababababababab";

/// One published release, and an interpreter prepared for it or not.
struct OneRelease {
    python: Option<String>,
}

impl AuditSource for OneRelease {
    fn release<'a>(
        &'a self,
        tenant: &'a str,
        extension: &'a str,
        version: Option<&'a str>,
    ) -> AuditFuture<'a, Result<AuditPackage, String>> {
        Box::pin(async move {
            assert_eq!(
                (tenant, extension, version),
                ("buro", "tr.ornek.denetim", None)
            );
            Ok(AuditPackage {
                catalog: "tenant:buro".into(),
                version: "1.0.0".into(),
                digest: DIGEST.into(),
                bytes: b"kek-bytes".to_vec(),
            })
        })
    }

    fn python(&self, _: &str, _: &str) -> Option<String> {
        self.python.clone()
    }
}

/// A processor that answers an audit as the job runner would, counting its runs.
struct Audits {
    runs: AtomicUsize,
    source: Arc<OneRelease>,
}

impl Processor for Audits {
    fn process(
        &self,
        task: &ProcessingTask,
        _: &ProcessHooks,
    ) -> Result<Processed, ProcessFailure> {
        let RunRequest::Audit(run) = &task.request else {
            panic!("an audit was asked");
        };
        assert_eq!(
            task.package.as_deref(),
            Some(&b"kek-bytes"[..]),
            "the package beside the request"
        );
        assert_eq!(run.digest, DIGEST);
        self.runs.fetch_add(1, Ordering::SeqCst);
        let report = AuditReport {
            rule: RuleRef {
                extension: run.extension.clone(),
                version: run.version.clone(),
                digest: run.digest.clone(),
                rule: run.rule.clone(),
                handler: "denetim.logic:adsiz".into(),
                title: "Adsız nesneler".into(),
            },
            drawing: "7b7fea779c35a3d0".into(),
            facts: format!("sha256:{}", "c".repeat(64)),
            kentos: "0.1.0".into(),
            findings: vec![Finding {
                severity: Severity::Info,
                message: "Çizim: adsız bir nesne.".into(),
                feature_id: None,
                fix: None,
            }],
            dropped: 0,
            cut: false,
        };
        Ok(Processed {
            summary: "Adsız nesneler: 1 bilgi".into(),
            details: serde_json::to_value(report).unwrap(),
            ..Processed::default()
        })
    }

    fn audits(&self) -> Option<Arc<dyn AuditSource>> {
        Some(self.source.clone())
    }
}

#[tokio::test]
async fn an_audit_job_keeps_its_report_and_a_second_one_answers_from_the_cache() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let tenant = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap()
        .to_string();
    admin::create_local_user(&db.owner, "ayse", "ayse", None, "dogru-parola-1")
        .await
        .unwrap();
    admin::set_membership(&db.owner, "buro", "ayse", TenantRole::ProjectManager, true)
        .await
        .unwrap();
    let store = kentos_application::blobs::Blobs::new(
        std::env::temp_dir().join(format!("kentos-api-jobs-{}", uuid::Uuid::now_v7())),
    );
    let router = app_with(Some(db.app.clone()), store.clone());
    let ayse = signed_in(&router, "ayse").await;
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
    let ask = |request: &str| {
        serde_json::json!({ "kind": "project.audit", "requestId": request,
                            "input": { "extension": "tr.ornek.denetim", "rule": "adsiz" } })
    };
    // The interpreter is the server's: a request naming one is not the kind's input.
    let (status, _, _) = send(
        &router,
        json_req("POST", &base, &ayse, serde_json::json!({ "kind": "project.audit", "requestId": "x",
            "input": { "extension": "tr.ornek.denetim", "rule": "adsiz", "python": "/usr/bin/python3" } })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let processor = Arc::new(Audits {
        runs: AtomicUsize::new(0),
        source: Arc::new(OneRelease {
            python: Some("/srv/kentos/python/denetim-1.0.0/bin/python".into()),
        }),
    });
    let mut done = Vec::new();
    for request in ["d-1", "d-2"] {
        let (status, _, body) = send(&router, json_req("POST", &base, &ayse, ask(request))).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "{}",
            String::from_utf8_lossy(&body)
        );
        let job: ServerJob = serde_json::from_slice(&body).unwrap();
        let worker: Arc<dyn Processor> = processor.clone();
        jobs::work_once_with(
            &db.app,
            &store,
            uuid::Uuid::now_v7(),
            std::time::Duration::from_secs(30),
            &Default::default(),
            Some(worker),
        )
        .await
        .unwrap()
        .expect("the job ran");
        let (_, _, body) = send(&router, get(&format!("{base}/{}", job.id), &ayse)).await;
        let read: ServerJob = serde_json::from_slice(&body).unwrap();
        assert_eq!(read.state, ServerJobState::Succeeded, "{:?}", read.error);
        done.push(read);
    }
    let result = done[0].result.clone().unwrap();
    assert_eq!(
        (result["catalog"].as_str(), result["summary"].as_str()),
        (Some("tenant:buro"), Some("Adsız nesneler: 1 bilgi"))
    );
    assert_eq!(result["report"]["rule"]["digest"], DIGEST);
    assert_eq!(done[1].result, done[0].result, "the same report");
    assert_eq!(done[1].cached_from.as_deref(), Some(done[0].id.as_str()));
    assert_eq!(
        processor.runs.load(Ordering::SeqCst),
        1,
        "answered from the cache"
    );
    // A version the server prepared no interpreter for is not run.
    let bare = Arc::new(Audits {
        runs: AtomicUsize::new(0),
        source: Arc::new(OneRelease { python: None }),
    });
    let (_, _, body) = send(
        &router,
        json_req(
            "POST",
            &base,
            &ayse,
            serde_json::json!({ "kind": "project.audit", "requestId": "d-3", "fresh": true,
            "input": { "extension": "tr.ornek.denetim", "rule": "adsiz" } }),
        ),
    )
    .await;
    let job: ServerJob = serde_json::from_slice(&body).unwrap();
    let worker: Arc<dyn Processor> = bare.clone();
    jobs::work_once_with(
        &db.app,
        &store,
        uuid::Uuid::now_v7(),
        std::time::Duration::from_secs(30),
        &Default::default(),
        Some(worker),
    )
    .await
    .unwrap()
    .expect("the job ran");
    let (_, _, body) = send(&router, get(&format!("{base}/{}", job.id), &ayse)).await;
    let read: ServerJob = serde_json::from_slice(&body).unwrap();
    assert_eq!(read.state, ServerJobState::Failed);
    assert_eq!(
        read.error.as_ref().map(|e| e.code.as_str()),
        Some("tool_unavailable")
    );
    assert_eq!(bare.runs.load(Ordering::SeqCst), 0);
    db.close().await;
}
