//! A user's share of the job queue and the jobs' result cache (docs/adr/0739 §1, §2): one user does not fill the
//! organisation's queue; an export built from the same project state, input and reach takes the artifact of the
//! earlier job (a second key of the same bytes, its own to remove) for any user whose reach is the same; a write, a
//! fresh request or a gone object builds it again. Real database (`KENTOS_TEST_DB=required` makes a missing one a
//! failure).

mod common;

use std::time::Duration;

use common::{
    a_point, blobs, catalog_envelope, envelope, member, new_project, open, scattered_points, share,
};
use kentos_application::commands::{CatalogPolicy, run};
use kentos_application::jobs::{self, JobLimits};
use kentos_application::{AppError, backup, changes};
use kentos_contracts::{
    GrantRole, PROJECT_CHECKPOINT_CREATE, ServerJob, ServerJobCreate, ServerJobState, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

const LEASE: Duration = Duration::from_secs(30);

fn export(request: &str, fresh: bool) -> ServerJobCreate {
    ServerJobCreate {
        kind: "project.export".into(),
        input: json!({}),
        request_id: request.into(),
        timeout_seconds: None,
        fresh,
    }
}

fn id_of(job: &ServerJob) -> Uuid {
    Uuid::parse_str(&job.id).unwrap()
}

async fn bytes_of(
    db: &TestDb,
    store: &kentos_application::blobs::Blobs,
    access: &kentos_application::access::ProjectAccess,
    job: Uuid,
) -> Vec<u8> {
    let (_, mut file) = jobs::artifact(&db.app, store, access, job).await.unwrap();
    let mut out = Vec::new();
    file.read_to_end(&mut out).await.unwrap();
    out
}

/// Asks for an export and runs the queue until it is done; the job as its asker reads it then.
async fn exported(
    db: &TestDb,
    store: &kentos_application::blobs::Blobs,
    access: &kentos_application::access::ProjectAccess,
    request: &str,
    fresh: bool,
) -> ServerJob {
    let limits = JobLimits::default();
    let job = jobs::create(&db.app, access, export(request, fresh), &limits)
        .await
        .unwrap();
    let worked = jobs::work_once(&db.app, store, Uuid::now_v7(), LEASE, &limits)
        .await
        .unwrap()
        .expect("a job");
    assert_eq!(worked.job, id_of(&job));
    let done = jobs::get(&db.app, access, id_of(&job)).await.unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded, "{done:?}");
    done
}

#[tokio::test]
async fn a_user_has_a_share_of_the_queue() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let other = member(&db, "buro", "ikinci", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Kuyruk payı").await;
    let mine = open(&db, &pm, project).await;
    share(&db, &mine, &other.actor, GrantRole::Editor).await;
    let theirs = open(&db, &other, project).await;
    let limits = JobLimits {
        tenant_active: 4,
        user_active: 2,
        ..JobLimits::default()
    };
    let first = jobs::create(&db.app, &mine, export("a-1", false), &limits)
        .await
        .unwrap();
    jobs::create(&db.app, &mine, export("a-2", false), &limits)
        .await
        .unwrap();
    // The third is past the user's share; the words say whose and that the others' go on.
    let refused = jobs::create(&db.app, &mine, export("a-3", false), &limits).await;
    match refused {
        Err(AppError::Quota(m)) => {
            assert!(m.contains("işleriniz") && m.contains("(2)"), "{m}")
        }
        other => panic!("{other:?}"),
    }
    // A retry of an asked one gets it back whatever the share.
    let again = jobs::create(&db.app, &mine, export("a-1", false), &limits)
        .await
        .unwrap();
    assert_eq!(again.id, first.id);
    // Another user of the organisation has their own share, up to the organisation's cap.
    jobs::create(&db.app, &theirs, export("b-1", false), &limits)
        .await
        .unwrap();
    // The organisation's cap is reached while that user still has a share: its words are the organisation's.
    let tight = JobLimits {
        tenant_active: 3,
        ..limits
    };
    match jobs::create(&db.app, &theirs, export("b-2", false), &tight).await {
        Err(AppError::Quota(m)) => assert!(m.contains("Kurumunuzun") && m.contains("(3)"), "{m}"),
        other => panic!("{other:?}"),
    }
    jobs::create(&db.app, &theirs, export("b-2", false), &limits)
        .await
        .unwrap();
    // Both full: the user's own share is said, only their own job ending makes room for them.
    match jobs::create(&db.app, &theirs, export("b-3", false), &limits).await {
        Err(AppError::Quota(m)) => assert!(m.contains("işleriniz") && m.contains("diğer"), "{m}"),
        other => panic!("{other:?}"),
    }
    // A job cancelled gives its place back.
    jobs::cancel(&db.app, &mine, id_of(&first)).await.unwrap();
    jobs::create(&db.app, &mine, export("a-5", false), &limits)
        .await
        .unwrap();
}

#[tokio::test]
async fn an_export_takes_the_artifact_of_the_same_state() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let one = member(&db, "buro", "birinci", TenantRole::Viewer).await;
    let two = member(&db, "buro", "ikinci", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Önbellek").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &one.actor, GrantRole::Editor).await;
    share(&db, &owner, &two.actor, GrantRole::Editor).await;
    let first_editor = open(&db, &one, project).await;
    let second_editor = open(&db, &two, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_500.0), &[]),
    )
    .await
    .unwrap();
    let store = blobs();

    // Built once.
    let built = exported(&db, &store, &first_editor, "e-1", false).await;
    assert_eq!(built.cached_from, None);
    let built_bytes = bytes_of(&db, &store, &first_editor, id_of(&built)).await;
    // The same state again, by the same user: taken, the same bytes, a key of its own.
    let taken = exported(&db, &store, &first_editor, "e-2", false).await;
    assert_eq!(taken.cached_from.as_deref(), Some(built.id.as_str()));
    assert_eq!(
        taken.artifact.as_ref().map(|a| &a.sha256),
        built.artifact.as_ref().map(|a| &a.sha256)
    );
    assert_eq!(
        taken.result, built.result,
        "the answer is the earlier one's"
    );
    assert_eq!(
        bytes_of(&db, &store, &first_editor, id_of(&taken)).await,
        built_bytes
    );
    // Another user with the same reach takes it too: an export is the same file for both. (From the newest job of the
    // key, here the one that took it: each holds the same bytes under a key of its own.)
    let theirs = exported(&db, &store, &second_editor, "e-3", false).await;
    assert_eq!(theirs.cached_from.as_deref(), Some(taken.id.as_str()));
    assert_eq!(
        bytes_of(&db, &store, &second_editor, id_of(&theirs)).await,
        built_bytes
    );
    // Asked fresh: built again.
    let fresh = exported(&db, &store, &first_editor, "e-4", true).await;
    assert_eq!(fresh.cached_from, None);
    assert_eq!(
        bytes_of(&db, &store, &first_editor, id_of(&fresh)).await,
        built_bytes
    );
    // Each job's object is its own: the first job's removed (as the retention would), the others still read and a
    // later job takes one of them.
    let first_key = format!(
        "jobs/{}/{}/{}-{}",
        owner.tenant,
        project,
        built.id,
        built.artifact.as_ref().unwrap().sha256
    );
    store.remove(&first_key).await.unwrap();
    assert_eq!(
        bytes_of(&db, &store, &first_editor, id_of(&taken)).await,
        built_bytes
    );
    let later = exported(&db, &store, &first_editor, "e-5", false).await;
    assert!(later.cached_from.is_some() && later.cached_from.as_deref() != Some(built.id.as_str()));
    // A write: a new state, built again.
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_600.0), &[]),
    )
    .await
    .unwrap();
    let after = exported(&db, &store, &first_editor, "e-6", false).await;
    assert_eq!(after.cached_from, None);
    assert_ne!(
        after.artifact.as_ref().map(|a| &a.sha256),
        built.artifact.as_ref().map(|a| &a.sha256)
    );
    // An export that must refuse (another revision expected) is never taken from the cache.
    let limits = JobLimits::default();
    let refusing = jobs::create(
        &db.app,
        &first_editor,
        ServerJobCreate {
            input: json!({ "expectedRevision": "1" }),
            ..export("e-7", false)
        },
        &limits,
    )
    .await
    .unwrap();
    jobs::work_once(&db.app, &store, Uuid::now_v7(), LEASE, &limits)
        .await
        .unwrap();
    let refused = jobs::get(&db.app, &first_editor, id_of(&refusing))
        .await
        .unwrap();
    assert_eq!(refused.state, ServerJobState::Failed);
    assert_eq!(
        refused.error.map(|e| e.code).as_deref(),
        Some("revision_conflict")
    );
    // The owner designs forms: another reach, so the editors' file is not theirs.
    let owners = exported(&db, &store, &owner, "e-8", false).await;
    assert_eq!(owners.cached_from, None);
}

/// The timings of docs/adr/0739 (release build, the test database): an export of 50 000 points built, taken from the
/// cache and built fresh; a backup of the store copied, then the next one linked.
#[tokio::test]
#[ignore = "release timing: cargo test --release -p kentos-application --test server_job_cache timing -- --ignored --nocapture"]
async fn timing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Ölçüm").await;
    let owner = open(&db, &pm, project).await;
    for salt in 0..10 {
        changes::commit(
            &db.app,
            &owner,
            envelope(&pm, project, scattered_points(5_000, salt), &[]),
        )
        .await
        .unwrap();
    }
    let store = blobs();
    for name in ["Birinci", "İkinci", "Üçüncü", "Dördüncü"] {
        run(
            &db.app,
            &store,
            &CatalogPolicy::default(),
            &owner,
            catalog_envelope(
                &owner,
                PROJECT_CHECKPOINT_CREATE,
                json!({ "name": name }),
                &[],
            ),
        )
        .await
        .unwrap();
    }

    let t = std::time::Instant::now();
    let built = exported(&db, &store, &owner, "o-1", false).await;
    let build = t.elapsed();
    let t = std::time::Instant::now();
    let taken = exported(&db, &store, &owner, "o-2", false).await;
    let take = t.elapsed();
    assert_eq!(taken.cached_from.as_deref(), Some(built.id.as_str()));
    let t = std::time::Instant::now();
    let fresh = exported(&db, &store, &owner, "o-3", true).await;
    let again = t.elapsed();
    assert_eq!(fresh.cached_from, None);
    let size = built.artifact.as_ref().unwrap().size.clone();

    let root = std::env::temp_dir().join(format!("kentos-yedek-olcum-{}", Uuid::now_v7()));
    let t = std::time::Instant::now();
    let first = backup::take_into(&db.owner_url(), &store, &root, 7)
        .await
        .unwrap();
    let copied = t.elapsed();
    let t = std::time::Instant::now();
    let second = backup::take_into(&db.owner_url(), &store, &root, 7)
        .await
        .unwrap();
    let linked = t.elapsed();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(store.root());
    println!(
        "dışa aktarma ({size} bayt, 50 000 nokta): yapıldı {build:.1?}, önbellekten {take:.1?}, yeniden {again:.1?}"
    );
    let bytes: i64 = second.manifest.objects.iter().map(|o| o.size).sum();
    println!(
        "yedek: ilk {copied:.1?} ({} kopya, {} bağ), sonraki {linked:.1?} ({} kopya, {} bağ); nesneler {bytes} bayt, döküm {} bayt",
        first.carried.copied,
        first.carried.linked,
        second.carried.copied,
        second.carried.linked,
        second.manifest.dump_size
    );
    assert!(
        second.carried.linked > 0 && second.carried.copied == 0,
        "{:?}",
        second.carried
    );
}
