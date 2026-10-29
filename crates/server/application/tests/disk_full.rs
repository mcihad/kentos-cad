//! The disk-full drill (docs/adr/0739 §3): the object store on a small file system of its own, filled up, then every
//! writer of the server tried on it: a job's artifact, a checkpoint's snapshot, a part of an upload, a job's staged
//! file and a backup. Each one fails with `storage_full`, keeps nothing half written (the upload resumes at the length
//! it had) and leaves the rest as it was; once room is made the same requests succeed.
//!
//! The file system is a real one that fills: `scripts/ops/disk_full_drill.sh` mounts a small tmpfs in a private
//! mount namespace (no root needed) and runs this test there with `KENTOS_FULL_DIR` naming it. Real database
//! (`KENTOS_TEST_DB=required`), `pg_dump` (`KENTOS_PG_DUMP`).

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{catalog_envelope, envelope, member, new_project, open, scattered_points};
use kentos_application::access::ProjectAccess;
use kentos_application::blobs::Blobs;
use kentos_application::commands::{CatalogPolicy, run};
use kentos_application::jobs::{self, JobLimits};
use kentos_application::{AppError, backup, changes, files, job_inputs, upload_parts};
use kentos_contracts::{
    FileUploadBegin, JobInputBegin, PROJECT_CHECKPOINT_CREATE, ServerJobCreate, ServerJobState,
    TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const LEASE: Duration = Duration::from_secs(30);

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Fills the file system of `dir` with the file `name` until a write fails, then frees `room` bytes of it; the
/// filler's path.
fn fill(dir: &Path, name: &str, room: u64) -> PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    let block = vec![0x5au8; 64 * 1024];
    loop {
        match f.write(&block) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                assert!(kentos_application::blobs::is_full(&e), "{e}");
                break;
            }
        }
    }
    let _ = f.sync_all();
    let len = std::fs::metadata(&path).unwrap().len();
    f.set_len(len.saturating_sub(room)).unwrap();
    f.sync_all().unwrap();
    path
}

fn full(e: &AppError) -> bool {
    e.code() == "storage_full" && e.retry() == (true, Some(300))
}

/// The keys of the objects under `top` (`jobs`, `checkpoints`, `uploads`, `job-inputs`).
fn objects(store: &Blobs, top: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![store.root().join(top)];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            if e.file_type().unwrap().is_dir() {
                stack.push(e.path());
            } else {
                out.push(
                    e.path()
                        .strip_prefix(store.root())
                        .unwrap()
                        .display()
                        .to_string(),
                );
            }
        }
    }
    out.sort();
    out
}

async fn export(
    db: &TestDb,
    store: &Blobs,
    access: &ProjectAccess,
    request: &str,
) -> kentos_contracts::ServerJob {
    let limits = JobLimits::default();
    let job = jobs::create(
        &db.app,
        access,
        ServerJobCreate {
            kind: "project.export".into(),
            input: json!({}),
            request_id: request.into(),
            timeout_seconds: None,
            fresh: true,
        },
        &limits,
    )
    .await
    .unwrap();
    jobs::work_once(&db.app, store, Uuid::now_v7(), LEASE, &limits)
        .await
        .unwrap()
        .expect("a job");
    jobs::get(&db.app, access, Uuid::parse_str(&job.id).unwrap())
        .await
        .unwrap()
}

async fn checkpoint(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    name: &str,
) -> Result<(), AppError> {
    run(
        &db.app,
        store,
        &CatalogPolicy::default(),
        by,
        catalog_envelope(by, PROJECT_CHECKPOINT_CREATE, json!({ "name": name }), &[]),
    )
    .await
    .map(|_| ())
}

#[tokio::test]
#[ignore = "scripts/ops/disk_full_drill.sh runs it on a small file system of its own"]
async fn a_full_store_keeps_nothing_half_and_takes_the_same_requests_later() {
    let dir = PathBuf::from(
        std::env::var_os("KENTOS_FULL_DIR")
            .expect("KENTOS_FULL_DIR yok: scripts/ops/disk_full_drill.sh ile çalıştırın"),
    );
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = Blobs::new(dir.join("depo"));
    let backups = dir.join("yedekler");
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Dolu disk").await;
    let owner = open(&db, &pm, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, scattered_points(3_000, 1), &[]),
    )
    .await
    .unwrap();

    // With room: an export and a first backup, as before any trouble.
    let before = export(&db, &store, &owner, "once").await;
    assert_eq!(before.state, ServerJobState::Succeeded, "{before:?}");
    let size: u64 = before.artifact.as_ref().unwrap().size.parse().unwrap();
    let first = backup::take_into(&db.owner_url(), &store, &backups, 7)
        .await
        .unwrap();
    let jobs_before = objects(&store, "jobs");
    let checkpoints_before = objects(&store, "checkpoints");

    // The disk filled up to less room than one export needs.
    let filler = fill(&dir, "dolgu", size / 4);
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, scattered_points(10, 2), &[]),
    )
    .await
    .unwrap();

    // A job's artifact: the job fails `storage_full`, nothing of it in the store.
    let failed = export(&db, &store, &owner, "dolu").await;
    assert_eq!(failed.state, ServerJobState::Failed, "{failed:?}");
    let error = failed.error.unwrap();
    assert_eq!(error.code, "storage_full", "{}", error.message);
    assert!(error.message.contains("yer kalmadı"), "{}", error.message);
    assert_eq!(objects(&store, "jobs"), jobs_before);

    // A checkpoint's snapshot: refused, no object, no checkpoint.
    let refused = checkpoint(&db, &store, &owner, "Dolu").await.unwrap_err();
    assert!(full(&refused), "{refused:?}");
    assert_eq!(objects(&store, "checkpoints"), checkpoints_before);

    // A part of an upload: refused whole; the upload resumes at the length it had.
    let payload: Vec<u8> = (0..(size as usize)).map(|i| (i * 31 % 251) as u8).collect();
    let upload = files::begin(
        &db.app,
        &owner,
        FileUploadBegin {
            size: payload.len() as u32,
            sha256: sha(&payload),
        },
    )
    .await
    .unwrap();
    let upload = Uuid::parse_str(&upload.id).unwrap();
    let part = upload_parts::receive_part(&db.app, &store, &owner, upload, 0, &payload)
        .await
        .unwrap_err();
    assert!(full(&part), "{part:?}");
    let state = files::upload(&db.app, &store, &owner, upload)
        .await
        .unwrap();
    assert_eq!(state.received_bytes.as_deref(), Some("0"));

    // A job's staged file: refused, nothing kept, not received.
    let input = job_inputs::begin(
        &db.app,
        &owner,
        JobInputBegin {
            name: "parseller.gpkg".into(),
            size: payload.len() as u64,
            sha256: sha(&payload),
        },
        u64::MAX,
    )
    .await
    .unwrap();
    let input_id = Uuid::parse_str(&input.id).unwrap();
    let mut receiving = job_inputs::start_receive(&db.app, &store, &owner, input_id)
        .await
        .unwrap();
    let written = receiving.writer.write(&payload).await;
    let refused = match written {
        Err(e) => job_inputs::failed_write(&store, receiving, e).await,
        Ok(()) => job_inputs::finish_receive(&db.app, &store, &owner, receiving)
            .await
            .unwrap_err(),
    };
    assert!(full(&refused), "{refused:?}");
    assert!(objects(&store, "job-inputs").is_empty());
    assert!(
        !job_inputs::staged(&db.app, &owner, input_id)
            .await
            .unwrap()
            .received
    );

    // A backup: fails, its half folder goes, the earlier backup stays sound. (What room was left goes too: the
    // backup would link its old objects and need room only for the dump.)
    let rest = fill(&dir, "dolgu-2", 0);
    let why = backup::take_into(&db.owner_url(), &store, &backups, 7)
        .await
        .unwrap_err();
    assert!(!why.is_empty());
    let found = backup::found_in(&backups).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].name, first.name);
    assert!(backup::check_folder(&found[0].path).unwrap().sound());

    // Room made: the same requests succeed.
    std::fs::remove_file(&filler).unwrap();
    std::fs::remove_file(&rest).unwrap();
    let after = export(&db, &store, &owner, "yer-acildi").await;
    assert_eq!(after.state, ServerJobState::Succeeded, "{after:?}");
    checkpoint(&db, &store, &owner, "Dolu").await.unwrap();
    let state = upload_parts::receive_part(&db.app, &store, &owner, upload, 0, &payload).await;
    // The bytes are not a drawing: refused as one, but taken whole first (the store had room).
    assert!(matches!(state, Err(AppError::Invalid { .. })), "{state:?}");
    let second = backup::take_into(&db.owner_url(), &store, &backups, 7)
        .await
        .unwrap();
    assert!(second.carried.linked > 0, "{:?}", second.carried);
    assert!(second.removed.is_empty());
    assert_eq!(backup::found_in(&backups).unwrap().len(), 2);
}
