//! What only the persistent store keeps (docs/adr/8059 §11, §13), on a test database (skipped without one): the
//! database itself freezes a sent content, a release's identity and its moves; the server's role adds moves and
//! packages but never changes or removes one; a write over a record that moved meanwhile is `Changed`, a second first
//! write `Exists`; an organisation's releases are not seen in another's scope; a package is written once; a record
//! whose moves are not the table's, or that skipped a step, does not load; the administrator's grants are what the
//! catalog reads. And how long a store's read and write take.

use std::time::Duration;

use axum::body::Bytes;
use kentos_extensions::release::{Actor, Channel, GateKind, ScanOutcome, Timestamp};
use kentos_postgres::{Scope, rescope};

use super::*;
use crate::extension_catalog::store::StoreError;

mod grants;
mod scope;
mod timing;

/// A statement as the server's role in `scope`, in a transaction of its own that is kept when it succeeds; the
/// database's error message when it fails.
async fn as_server(db: &TestDb, scope: Scope, sql: &str) -> Result<u64, String> {
    let mut tx = db.app.pool.begin().await.unwrap();
    rescope(&mut tx, scope).await.unwrap();
    let done = sqlx::query(sqlx::AssertSqlSafe(sql.to_owned()))
        .execute(&mut *tx)
        .await;
    match done {
        Ok(d) => {
            tx.commit().await.unwrap();
            Ok(d.rows_affected())
        }
        Err(sqlx::Error::Database(e)) => {
            Err(format!("{}: {}", e.code().unwrap_or_default(), e.message()))
        }
        Err(e) => Err(e.to_string()),
    }
}

async fn tenant_of(db: &TestDb, slug: &str) -> Uuid {
    sqlx::query_scalar("select id from kentos.tenant where slug = $1")
        .bind(slug)
        .fetch_one(&db.owner)
        .await
        .unwrap()
}

#[tokio::test]
async fn the_database_keeps_a_sent_content_a_release_s_identity_and_its_moves() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let bench = Bench::database(&db);
    let app = catalog(&bench).await;
    upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    let other = format!("sha256:{}", "e".repeat(64));
    let renew = format!(
        "update kentos.extension_release set package_digest = '{other}',
            record = jsonb_set(record, '{{content,package}}', to_jsonb('{other}'::text)) where version = '1.0.0'"
    );
    // A draft's content may be renewed (the flow's `replace`): in a transaction left unkept here.
    let mut tx = db.app.pool.begin().await.unwrap();
    let renewed = sqlx::query(sqlx::AssertSqlSafe(renew.clone()))
        .execute(&mut *tx)
        .await
        .unwrap();
    assert_eq!(renewed.rows_affected(), 1);
    tx.rollback().await.unwrap();
    // Sent for review: its content is fixed, whoever writes it.
    let content = package_of(&get(&app, Via::Web, &uri(PUBLIC, "1.0.0"), "ayse").await.1);
    let at = uri(PUBLIC, "1.0.0");
    scan_all(&app, &at, &content).await;
    let (status, _) = act(
        &app,
        Via::Python,
        "ayse",
        &at,
        json!({ "action": "submit" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let refused = as_server(&db, Scope::default(), &renew).await.unwrap_err();
    assert!(refused.contains("content_frozen"), "{refused}");
    // Nor its identity, nor its moves: none taken away, none rewritten.
    for (sql, why) in [
        (
            "update kentos.extension_release set author = 'baskasi',
                record = jsonb_set(record, '{author}', '\"baskasi\"')",
            "identity does not change",
        ),
        (
            "update kentos.extension_release set moves = moves - 1,
                record = jsonb_set(record, '{audit}', (record -> 'audit') - (moves - 1))",
            "moves are not taken away",
        ),
        (
            "update kentos.extension_release set record = jsonb_set(record, '{audit,0,reason}', '\"sonradan\"')",
            "move 0 of the record changed",
        ),
    ] {
        let refused = as_server(&db, Scope::default(), sql).await.unwrap_err();
        assert!(refused.contains(why), "{refused}");
    }
    // The columns are the record's: a state the record does not say is refused.
    let refused = as_server(
        &db,
        Scope::default(),
        "update kentos.extension_release set state = 'published'",
    )
    .await
    .unwrap_err();
    assert!(refused.starts_with("23514"), "{refused}");
    db.close().await;
}

#[tokio::test]
async fn the_server_s_role_adds_moves_and_packages_but_never_changes_or_removes_one() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let bench = Bench::database(&db);
    let app = catalog(&bench).await;
    upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    let denied = [
        "update kentos.extension_release_move set reason = 'sonradan'",
        "delete from kentos.extension_release_move",
        "delete from kentos.extension_release",
        "update kentos.extension_package set size = 1",
        "delete from kentos.extension_package",
        "insert into kentos.extension_publisher (id, name) values ('kendim', 'Kendim')",
        "delete from kentos.extension_catalog_grant",
    ];
    for sql in denied {
        let refused = as_server(&db, Scope::default(), sql).await.unwrap_err();
        assert!(refused.starts_with("42501"), "{sql}: {refused}");
    }
    let moves: i64 = sqlx::query_scalar("select count(*) from kentos.extension_release_move")
        .fetch_one(&db.owner)
        .await
        .unwrap();
    assert_eq!(moves, 1, "the upload's move, kept");
    db.close().await;
}

#[tokio::test]
async fn a_write_over_a_moved_record_is_changed_and_a_second_first_write_exists() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let bench = Bench::database(&db);
    let app = catalog(&bench).await;
    let (_, view) = upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    let content = package_of(&view);
    let store = bench.store();
    let read = store.release(&key("1.0.0")).await.unwrap().unwrap();
    let moves = read.audit().len();
    let scanner = Actor::new(&id("tarayici"), &[Role::Scanner], Channel::Program);
    let scanned = |gate| {
        let mut r = read.clone();
        r.record_scan(&scanner, gate, &content, ScanOutcome::Passed, Timestamp(1))
            .unwrap();
        r
    };
    // Two scanners' reports made on the same record: the first is kept, the second hears that the record moved.
    let (first, second) = (scanned(GateKind::Secrets), scanned(GateKind::Tests));
    store
        .write(&key("1.0.0"), Some(moves), &first)
        .await
        .unwrap();
    assert!(matches!(
        store.write(&key("1.0.0"), Some(moves), &second).await,
        Err(StoreError::Changed)
    ));
    assert!(matches!(
        store.write(&key("1.0.0"), None, &read).await,
        Err(StoreError::Exists)
    ));
    assert_eq!(store.release(&key("1.0.0")).await.unwrap(), Some(first));
    // A version that has no record has no record to move.
    assert!(matches!(
        store.write(&key("9.9.9"), Some(1), &second).await,
        Err(StoreError::Changed) | Err(StoreError::Failed(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn a_package_is_written_once_and_read_whole() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let root = std::env::temp_dir().join(format!("kentos-katalog-{}", Uuid::now_v7()));
    let store: Arc<dyn ReleaseStore> = Arc::new(PgStore::new(db.app.clone(), Blobs::new(&root)));
    let digest = format!("sha256:{}", "a".repeat(64));
    let (first, second) = (
        Bytes::from_static(b"ilk baytlar"),
        Bytes::from_static(b"ikinci"),
    );
    // The first writer, then seven more at once with other bytes of the same digest: one row, the first's object.
    store.put_package(&digest, first.clone()).await.unwrap();
    let mut writers = Vec::new();
    for _ in 0..7 {
        let (store, digest, bytes) = (store.clone(), digest.clone(), second.clone());
        writers.push(tokio::spawn(async move {
            store.put_package(&digest, bytes).await
        }));
    }
    for w in writers {
        w.await.unwrap().unwrap();
    }
    assert_eq!(store.package(&digest).await.unwrap(), Some(first.clone()));
    let (rows, size, sha256): (i64, i64, String) = sqlx::query_as(
        "select count(*) over (), size, sha256 from kentos.extension_package where digest = $1",
    )
    .bind(&digest)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    let hex: String = <sha2::Sha256 as sha2::Digest>::digest(&first)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!((rows, size, sha256), (1, first.len() as i64, hex));
    let key = Blobs::extension_package_key(&"a".repeat(64));
    assert_eq!(std::fs::read(root.join(&key)).unwrap(), first.to_vec());
    let staged = std::fs::read_dir(root.join("extension-packages/staging"))
        .map(|d| d.count())
        .unwrap_or(0);
    assert_eq!(staged, 0, "nothing left half written");
    let unknown = format!("sha256:{}", "b".repeat(64));
    assert_eq!(store.package(&unknown).await.unwrap(), None);
    // A row whose object went is the store's failure, never other bytes; a digest that is not one is not a key.
    std::fs::remove_file(root.join(&key)).unwrap();
    assert!(matches!(
        store.package(&digest).await,
        Err(StoreError::Failed(_))
    ));
    assert!(matches!(
        store.put_package("sha256:kisa", first).await,
        Err(StoreError::Failed(_))
    ));
    let _ = std::fs::remove_dir_all(&root);
    db.close().await;
}

#[tokio::test]
async fn a_record_that_is_not_its_moves_or_skipped_a_step_does_not_load() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let bench = Bench::database(&db);
    let app = catalog(&bench).await;
    upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    upload(&app, PUBLIC, "ayse", release("1.1.0")).await;
    // Behind the server's back, as the owner: a move's row rewritten, a record published without approval.
    sqlx::query(
        "update kentos.extension_release_move m set reason = 'sonradan yazıldı'
           from kentos.extension_release r where r.id = m.release_id and r.version = '1.0.0'",
    )
    .execute(&db.owner)
    .await
    .unwrap();
    sqlx::query(
        "update kentos.extension_release set state = 'published', record = jsonb_set(record, '{state}', '\"published\"')
          where version = '1.1.0'",
    )
    .execute(&db.owner)
    .await
    .unwrap();
    let store = bench.store();
    for (version, why) in [
        ("1.0.0", "hareket tablosu"),
        ("1.1.0", "record_inconsistent"),
    ] {
        match store.release(&key(version)).await {
            Err(StoreError::Failed(detail)) => assert!(detail.contains(why), "{version}: {detail}"),
            other => panic!("{version}: {other:?}"),
        }
    }
    assert!(matches!(
        store.releases(&CatalogId::Public).await,
        Err(StoreError::Failed(_))
    ));
    let (status, _) = get(&app, Via::Web, &uri(PUBLIC, "1.0.0"), "ayse").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    db.close().await;
}
