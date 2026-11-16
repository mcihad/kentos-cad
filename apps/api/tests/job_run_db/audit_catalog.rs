//! `project.audit` against the persistent catalog, the whole way (docs/adr/8059 §13): a release uploaded, scanned,
//! reviewed, approved and published in an organisation's catalog kept on the tests' database and object store; its
//! audit's Python environment prepared by the catalog's publishing hook (docs/adr/8057 §66), offline from a folder of
//! wheels; then a `project.audit` job asked as a user and taken by a worker whose real processor (`kentos-jobrun`)
//! gets the rule from that catalog and runs it in that environment, the report kept as the job's result. Needs the
//! tests' database, uv, a Python in KentOS's range and KentOS's library's wheel; without one it says so and passes.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Bytes;
use kentos_application::admin;
use kentos_application::blobs::Blobs;
use kentos_application::jobs::{self, JobLimits};
use kentos_application::processing_job::Processor;
use kentos_contracts::project_audit::ProjectAuditResult;
use kentos_contracts::{PROJECT_AUDIT_JOB, ServerJobCreate, ServerJobState};
use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
use kentos_extensions::manifest::Module;
use kentos_extensions::release::{Channel, GateKind, ReleaseState, Role, ScanOutcome};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

use super::processing_job::ChildProcessor;
use super::{parcels, project_of, support};
use crate::audit_env::EnvSettings;
use crate::extension_catalog::audits::{CatalogAudits, PrepareEnvironments};
use crate::extension_catalog::postgres::PgStore;
use crate::extension_catalog::store::Where;
use crate::extension_catalog::{Account, Accounts, Catalog, CatalogId, Move, ReleaseKey, grants};
use crate::support::audit_package::{Gone, built_tools, package, project_and_lock, python, wheel};

/// How long the publishing hook may take to prepare the environment (offline, from the folder: seconds).
const PREPARED_WITHIN: Duration = Duration::from_secs(180);

#[tokio::test]
async fn a_cloud_audit_runs_a_release_published_in_the_persistent_catalog_in_the_environment_publishing_made()
 {
    let Some(base) = python() else {
        eprintln!("Python yok (KENTOS_TEST_PYTHON ya da .run/py): sınama atlandı");
        return;
    };
    let found = kentos_python_env::uv::detect(None);
    let Some(uv) = found.path().filter(|_| found.ok()).map(Path::to_path_buf) else {
        eprintln!("uv yok ({}): sınama atlandı", found.words());
        return;
    };
    let Some((tools, work)) = built_tools() else {
        eprintln!(
            "KentOS'un Python kitaplığının tekerleği yok (tools/kentos-sdk.json): sınama atlandı"
        );
        return;
    };
    let Some(db) = TestDb::create().await else {
        return;
    };
    let work = work.with_file_name(format!("katalog-denetimi-sinama-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    let _gone = Gone(work.clone());
    // The release: a rule that imports a wheel its lock names, under the publisher Örnek Belediyesi.
    let wheels = work.join("tekerlekler");
    wheel(&base, &wheels, "kucuk", "1.0");
    let project = blueprint(&Ask {
        id: "tr.ornek.denetim".into(),
        name: "Denetim örneği".into(),
        summary: "Bir bulut denetim işinin kuralı.".into(),
        vertical: false,
        module: Module::Cbs,
        contributions: vec![Contribution::Audit],
    })
    .unwrap()["pyproject.toml"]
        .clone();
    let (pyproject, lock) =
        project_and_lock(&uv, &work, &project, &wheels, &[("kucuk", "1.0")], None);
    let (bytes, digest, manifest) = package(
        "tr.ornek.denetim",
        Some("ornek-belediye"),
        &pyproject,
        &lock,
    );
    let version = manifest.version.to_string();
    let settings = EnvSettings {
        dir: work.join("ortamlar"),
        base_python: Some(PathBuf::from(&base)),
        wheels: Some(wheels.clone()),
        index_url: None,
        uv: Some(uv),
        sdk_dirs: vec![tools],
        pythons: Default::default(),
    };
    // The organisation Büro and its project (Ayşe manages it), and the catalog's people, granted as the administrator
    // grants them: Ayşe publishes for Örnek Belediyesi there, Selin reviews, the scanner reports everywhere.
    let (access, _, _) = project_of(&db, "Parseller", &parcels()).await;
    let owner = &db.owner;
    for login in ["selin", "tarayici"] {
        admin::create_local_user(owner, login, login, None, "dogru-parola-123")
            .await
            .unwrap();
    }
    let buro = Where::In(CatalogId::Tenant("buro".into()));
    grants::grant(owner, &buro, "ayse", Role::Publisher)
        .await
        .unwrap();
    grants::grant(owner, &buro, "selin", Role::Reviewer)
        .await
        .unwrap();
    grants::grant(owner, &Where::Every, "tarayici", Role::Scanner)
        .await
        .unwrap();
    grants::publisher(owner, "ornek-belediye", Some("Örnek Belediyesi"))
        .await
        .unwrap();
    grants::member(owner, "ornek-belediye", "ayse", true)
        .await
        .unwrap();
    let account = |login: &'static str, via: Channel| async move {
        Account {
            id: admin::user_id(owner, login).await.unwrap().to_string(),
            via,
            fresh_sign_in: true,
        }
    };
    let ayse = account("ayse", Channel::Program).await;
    let selin = account("selin", Channel::Window).await;
    let scanner = account("tarayici", Channel::Program).await;
    // The catalog on the persistent store, its publishing hook the server's.
    let blobs = Blobs::new(work.join("nesneler"));
    let store = Arc::new(PgStore::new(db.app.clone(), blobs.clone()));
    let hook = Arc::new(PrepareEnvironments::new(settings.clone()));
    let catalog = Arc::new(Catalog::new(store, Accounts::Named).on_publish(hook));
    let tenant = CatalogId::Tenant("buro".into());
    let (uploaded, _) = catalog
        .upload(&tenant, &ayse, Bytes::from(bytes.clone()))
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(uploaded.content().package, digest);
    let key = ReleaseKey {
        catalog: tenant.clone(),
        extension: manifest.id.clone(),
        version: version.clone(),
    };
    for gate in GateKind::ALL {
        let report = Move::Scan {
            gate,
            content: digest.clone(),
            outcome: ScanOutcome::Passed,
        };
        catalog.act(&key, &scanner, &report).await.unwrap();
    }
    for (who, request) in [
        (&ayse, Move::Submit { note: None }),
        (&selin, Move::StartReview),
        (&selin, Move::Approve { note: None }),
    ] {
        catalog
            .act(&key, who, &request)
            .await
            .unwrap_or_else(|e| panic!("{e:?}"));
    }
    let published_at = Instant::now();
    let (published, _) = catalog.act(&key, &ayse, &Move::Publish).await.unwrap();
    assert_eq!(published.state(), ReleaseState::Published);
    // The move has its answer already; the hook prepares the environment and maps its Python, where jobs look.
    let map = settings.map();
    let mapped = loop {
        if let Some(python) = map.get(&manifest.id, &version) {
            break python;
        }
        assert!(
            published_at.elapsed() < PREPARED_WITHIN,
            "yayımlanan sürümün ortamı {} sn içinde hazırlanmadı",
            PREPARED_WITHIN.as_secs()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    let prepared = published_at.elapsed();
    // The job, asked as Ayşe and taken by a worker whose real processor reads the same catalog and map.
    let processor: Arc<dyn Processor> = Arc::new(
        ChildProcessor::with(support::child_exe(), work.join("isler")).with_audits(Arc::new(
            CatalogAudits::new(catalog.clone(), settings.map()),
        )),
    );
    let asked = ServerJobCreate {
        kind: PROJECT_AUDIT_JOB.into(),
        input: json!({ "extension": manifest.id, "rule": "adsiz" }),
        request_id: "katalogdan-denetim".into(),
        timeout_seconds: None,
        fresh: false,
    };
    let job = jobs::create(&db.app, &access, asked, &JobLimits::default())
        .await
        .unwrap();
    let began = Instant::now();
    let worked = jobs::work_once_with(
        &db.app,
        &blobs,
        Uuid::now_v7(),
        Duration::from_secs(60),
        &JobLimits::default(),
        Some(processor),
    )
    .await
    .unwrap()
    .expect("the worker took the job");
    let ran = began.elapsed();
    assert_eq!(worked.job.to_string(), job.id);
    let done = jobs::get(&db.app, &access, worked.job).await.unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded, "{done:?}");
    let result: ProjectAuditResult = serde_json::from_value(done.result.unwrap()).unwrap();
    assert_eq!(result.catalog, "tenant:buro");
    let rule = &result.report.rule;
    assert_eq!(
        (
            rule.extension.as_str(),
            rule.version.as_str(),
            rule.digest.as_str()
        ),
        (manifest.id.as_str(), version.as_str(), digest.as_str())
    );
    // The rule ran in the environment publishing made: it imported the wheel's module.
    assert!(!result.report.findings.is_empty());
    assert!(
        result
            .report
            .findings
            .iter()
            .all(|f| f.message.starts_with("kucuk 1.0: ")),
        "{:?}",
        result.report.findings
    );
    assert!(Path::new(&mapped).starts_with(&settings.dir), "{mapped}");
    eprintln!(
        "kalıcı katalogdan bulut denetimi: yayımlamadan ortam hazır {:.2} sn; iş {:.2} sn ({} bulgu: {})",
        prepared.as_secs_f64(),
        ran.as_secs_f64(),
        result.report.findings.len(),
        result.summary
    );
    db.close().await;
}
