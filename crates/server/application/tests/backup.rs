//! A restore drill (docs/adr/0650): a backup of a server with a database project, its checkpoint, a file project's
//! revisions and checkpoint and a job's staged input is restored into a new database and store; the restored server
//! names every object it has, reads the same projects, and the check finds a missing and a changed object.
//! Needs `pg_dump` and `pg_restore` (`KENTOS_PG_DUMP`, `KENTOS_PG_RESTORE`); real database
//! (`KENTOS_TEST_DB=required` makes a missing one a failure).

mod common;

use common::{a_point, blobs, catalog_envelope, envelope, member, new_project, open};
use kentos_application::access::ProjectAccess;
use kentos_application::backup;
use kentos_application::blobs::Blobs;
use kentos_application::commands::{CatalogPolicy, CommandOutcome, run};
use kentos_application::{admin, changes, files, projects};
use kentos_contracts::{
    CommandEnvelope, DocumentSnapshotV1, FileUploadBegin, PROJECT_CHECKPOINT_CREATE,
    PROJECT_FILE_COMMIT, ProjectCreate, ProjectStorage, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MINIMAL: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/minimal.kcad");
const DRAWING: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/drawing.kcad");
const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

async fn file_project(db: &TestDb, who: &kentos_application::tenancy::Access) -> Uuid {
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let input = ProjectCreate {
        dimension_sets: Vec::new(),
        casework: Vec::new(),
        constraints: Default::default(),
        name: "Dosya".into(),
        settings: s.settings,
        origin: s.origin,
        home_view: s.home_view,
        layers: s.layers,
        active_layer: s.active_layer,
        styles: s.styles,
        sheets: None,
        arrays: Vec::new(),
        description: None,
        project_type: None,
        tags: None,
        storage: Some(ProjectStorage::File),
    };
    Uuid::parse_str(
        &projects::create(&db.app, who, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap()
}

/// Uploads `bytes` and commits them on `base`.
async fn save(db: &TestDb, store: &Blobs, by: &ProjectAccess, bytes: &[u8], base: &str) {
    let begun = files::begin(
        &db.app,
        by,
        FileUploadBegin {
            size: bytes.len() as u32,
            sha256: sha(bytes),
        },
    )
    .await
    .unwrap();
    let id = Uuid::parse_str(&begun.id).unwrap();
    let mut receiving = files::start_receive(&db.app, store, by, id).await.unwrap();
    receiving.writer.write(bytes).await.unwrap();
    files::finish_receive(&db.app, store, by, receiving)
        .await
        .unwrap();
    let commit = CommandEnvelope {
        command_name: PROJECT_FILE_COMMIT.into(),
        version: 1,
        tenant_id: by.tenant.to_string(),
        project_id: by.project.to_string(),
        request_id: format!("istek-{}", Uuid::new_v4()),
        idempotency_key: Uuid::new_v4().to_string(),
        expected_versions: [("@file".to_string(), base.to_string())]
            .into_iter()
            .collect(),
        input: json!({ "uploadId": id.to_string() }),
    };
    run(&db.app, store, &CatalogPolicy::default(), by, commit)
        .await
        .unwrap();
}

async fn checkpoint(db: &TestDb, store: &Blobs, by: &ProjectAccess, name: &str) {
    let out = run(
        &db.app,
        store,
        &CatalogPolicy::default(),
        by,
        catalog_envelope(by, PROJECT_CHECKPOINT_CREATE, json!({ "name": name }), &[]),
    )
    .await
    .unwrap();
    assert!(matches!(out, CommandOutcome::Checkpoint(_)), "{out:?}");
}

/// A server's rows, table by table, as the owner reads them (the restored server must read the same).
async fn rows(owner: &sqlx::PgPool) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    for t in [
        "project",
        "feature",
        "project_file_revision",
        "project_checkpoint",
        "job_input",
        "membership",
        "project_grant",
        "audit_event",
    ] {
        let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "select count(*) from kentos.{t}"
        )))
        .fetch_one(owner)
        .await
        .unwrap();
        out.push((t.to_owned(), n));
    }
    out
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &target);
        } else {
            std::fs::copy(e.path(), target).unwrap();
        }
    }
}

#[tokio::test]
async fn a_backup_is_restored_into_a_new_server() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    admin::create_tenant(&db.owner, "buro", "Büro", 4)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    // A database project with a few objects and a checkpoint of them.
    let live = new_project(&db, &pm, "Canlı").await;
    let live_p = open(&db, &pm, live).await;
    for x in [1.0, 2.0, 3.0] {
        changes::commit(&db.app, &live_p, envelope(&pm, live, a_point(x), &[]))
            .await
            .unwrap();
    }
    checkpoint(&db, &store, &live_p, "Teslim").await;
    // A file project with two revisions and a checkpoint naming the first.
    let file = file_project(&db, &pm).await;
    let file_p = open(&db, &pm, file).await;
    save(&db, &store, &file_p, MINIMAL, "0").await;
    checkpoint(&db, &store, &file_p, "İlk").await;
    save(&db, &store, &file_p, DRAWING, "1").await;
    // A job's staged input (docs/adr/0561): a row and its object.
    let input = Uuid::new_v4();
    let bytes = b"Ad,X,Y\nP1,1,2\n";
    let key = Blobs::job_input_key(live_p.tenant, live, input, &sha(bytes));
    let path = store.local_path(&key).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, bytes).unwrap();
    sqlx::query(
        "insert into kentos.job_input (tenant_id, project_id, id, created_by, name, size, sha256, blob_key, received_at)
         values ($1, $2, $3, $4, 'noktalar.csv', $5, $6, $7, now())",
    )
    .bind(live_p.tenant)
    .bind(live)
    .bind(input)
    .bind(pm.actor.user_id)
    .bind(bytes.len() as i64)
    .bind(sha(bytes))
    .bind(&key)
    .execute(&db.owner)
    .await
    .unwrap();

    // The server as it is: every object named is there.
    let now = backup::check(&db.owner, &store).await.unwrap();
    assert!(now.sound(), "{now:?}");
    assert_eq!(
        now.sound, 4,
        "two revisions, a snapshot and an input: {now:?}"
    );

    // The backup.
    let dir = std::env::temp_dir().join(format!("kentos-yedek-{}", Uuid::now_v7()));
    let taken = backup::take(&db.owner_url(), &store, &dir).await.unwrap();
    assert_eq!(taken.objects.len(), now.sound);
    let folder = backup::check_folder(&dir).unwrap();
    assert!(folder.sound() && folder.unnamed.is_empty(), "{folder:?}");
    // A copy for the independent check (scripts/ops/backup_check.py), before anything is broken below.
    if let Ok(out) = std::env::var("KENTOS_BACKUP_OUT") {
        let _ = std::fs::remove_dir_all(&out);
        copy_tree(&dir, std::path::Path::new(&out));
    }
    // A folder that is not empty is not written into.
    assert!(backup::take(&db.owner_url(), &store, &dir).await.is_err());

    // The restore: a new database from the dump, a new store from the copied objects.
    let restored = db.restored(&dir.join(backup::DUMP)).await;
    let store2 = Blobs::new(dir.join(backup::OBJECTS));
    let after = backup::check(&restored.owner, &store2).await.unwrap();
    assert!(after.sound() && after.unnamed.is_empty(), "{after:?}");
    assert_eq!(rows(&restored.owner).await, rows(&db.owner).await);
    // The same projects read the same through the server role.
    let live_r = open(&restored, &pm, live).await;
    let a = projects::features(&db.app, &live_p, None, 100)
        .await
        .unwrap();
    let b = projects::features(&restored.app, &live_r, None, 100)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&a.features).unwrap(),
        serde_json::to_value(&b.features).unwrap()
    );
    let file_r = open(&restored, &pm, file).await;
    for revision in [1, 2] {
        let (_, mut f) = files::download(&restored.app, &store2, &file_r, revision)
            .await
            .unwrap();
        let mut got = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut f, &mut got)
            .await
            .unwrap();
        assert_eq!(got, if revision == 1 { MINIMAL } else { DRAWING });
    }

    // A changed and a missing object are said.
    let first = &taken.objects[0];
    std::fs::write(store2.local_path(&first.key).unwrap(), b"bozuk").unwrap();
    let second = &taken.objects[1];
    std::fs::remove_file(store2.local_path(&second.key).unwrap()).unwrap();
    std::fs::write(
        dir.join(backup::OBJECTS).join("revisions").join("artik"),
        b"x",
    )
    .unwrap();
    let broken = backup::check(&restored.owner, &store2).await.unwrap();
    assert!(!broken.sound());
    assert_eq!(broken.wrong, vec![first.clone()]);
    assert_eq!(broken.missing, vec![second.clone()]);
    assert_eq!(broken.unnamed, vec!["revisions/artik".to_owned()]);
    // The backup's folder says the same, and a dump that changed.
    let folder = backup::check_folder(&dir).unwrap();
    assert!(!folder.sound());
    std::fs::write(dir.join(backup::DUMP), b"bozuk").unwrap();
    assert_eq!(backup::check_folder(&dir).unwrap().dump, Some(false));

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(store.root());
    restored.close().await;
    db.close().await;
}

/// How long a backup, its check and a restore take (docs/adr/0650, Ölçümler): 100 000 objects in a database project and
/// 20 staged inputs of 8 MiB in the store.
#[tokio::test]
#[ignore = "süre ölçümü: KENTOS_TEST_DB=required cargo test --release -p kentos-application --test backup backup_timing -- --ignored --nocapture"]
async fn backup_timing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let store = blobs();
    admin::create_tenant(&db.owner, "buro", "Büro", 4)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let live = new_project(&db, &pm, "Canlı").await;
    let live_p = open(&db, &pm, live).await;
    changes::commit(&db.app, &live_p, envelope(&pm, live, a_point(1.0), &[]))
        .await
        .unwrap();
    sqlx::query(
        "insert into kentos.feature (tenant_id, project_id, id, layer_id, kind, source_kind, srid, geom, cad_definition,
                                     properties, projection_version, created_by, updated_by)
         select tenant_id, project_id, gen_random_uuid(), layer_id, kind, source_kind, srid,
                public.st_translate(geom, i, i), cad_definition, jsonb_build_object('No', i::text),
                projection_version, created_by, updated_by
           from kentos.feature, generate_series(1, 100000) i where project_id = $1",
    )
    .bind(live)
    .execute(&db.owner)
    .await
    .unwrap();
    let bytes: Vec<u8> = (0..8u32 << 20)
        .map(|i| (i.wrapping_mul(2654435761u32) >> 24) as u8)
        .collect();
    for n in 0..20u8 {
        let mut b = bytes.clone();
        b[0] = n;
        let input = Uuid::new_v4();
        let key = Blobs::job_input_key(live_p.tenant, live, input, &sha(&b));
        let path = store.local_path(&key).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &b).unwrap();
        sqlx::query(
            "insert into kentos.job_input (tenant_id, project_id, id, created_by, name, size, sha256, blob_key, received_at)
             values ($1, $2, $3, $4, 'girdi.bin', $5, $6, $7, now())",
        )
        .bind(live_p.tenant)
        .bind(live)
        .bind(input)
        .bind(pm.actor.user_id)
        .bind(b.len() as i64)
        .bind(sha(&b))
        .bind(&key)
        .execute(&db.owner)
        .await
        .unwrap();
    }
    let dir = std::env::temp_dir().join(format!("kentos-yedek-{}", Uuid::now_v7()));
    let t = std::time::Instant::now();
    let taken = backup::take(&db.owner_url(), &store, &dir).await.unwrap();
    println!(
        "yedek: {:.2} s ({} nesne, döküm {:.1} MB)",
        t.elapsed().as_secs_f64(),
        taken.objects.len(),
        taken.dump_size as f64 / 1e6
    );
    let t = std::time::Instant::now();
    assert!(backup::check_folder(&dir).unwrap().sound());
    println!("yedeğin denetimi: {:.2} s", t.elapsed().as_secs_f64());
    let t = std::time::Instant::now();
    let restored = db.restored(&dir.join(backup::DUMP)).await;
    println!(
        "geri yükleme (pg_restore): {:.2} s",
        t.elapsed().as_secs_f64()
    );
    let t = std::time::Instant::now();
    let after = backup::check(&restored.owner, &Blobs::new(dir.join(backup::OBJECTS)))
        .await
        .unwrap();
    assert!(after.sound());
    println!(
        "geri yüklenen sunucunun denetimi: {:.2} s",
        t.elapsed().as_secs_f64()
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(store.root());
    restored.close().await;
    db.close().await;
}
