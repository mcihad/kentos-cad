//! How long the event log is kept, against a real PostgreSQL: old events go
//! in batches, every tenant's, each project's horizon is recorded, and a
//! cursor older than what is left must reopen the project (`ResyncRequired`).

mod common;

use std::time::Duration;

use common::{a_point, envelope, member, new_project};
use kentos_application::tenancy::Access;
use kentos_application::{AppError, admin, changes, events, projects};
use kentos_contracts::TenantRole;
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

const WEEK: Duration = Duration::from_secs(7 * 24 * 3600);

/// Makes a project's events up to `seq` eight days old.
async fn age(db: &TestDb, project: Uuid, seq: i64) {
    sqlx::query("update kentos.outbox_event set created_at = now() - interval '8 days' where project_id = $1 and seq <= $2")
        .bind(project)
        .bind(seq)
        .execute(&db.owner)
        .await
        .unwrap();
}

/// Prunes one event at a time until nothing is old enough; returns how many went.
async fn prune_all(db: &TestDb) -> i64 {
    let mut total = 0;
    loop {
        let n = events::prune(&db.app, WEEK, 1).await.unwrap();
        total += n;
        if n == 0 {
            return total;
        }
    }
}

async fn commit_point(db: &TestDb, who: &Access, project: Uuid, x: f64) -> i64 {
    changes::commit(&db.app, who, envelope(who, project, a_point(x), &[]))
        .await
        .unwrap()
        .event_seq
        .parse()
        .unwrap()
}

#[tokio::test]
async fn old_events_go_and_older_cursors_must_reopen() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let other = member(&db, "diger", "baska", TenantRole::ProjectManager).await;
    let busy = new_project(&db, &pm, "Ada 101").await;
    let quiet = new_project(&db, &other, "Ada 7").await;
    let fresh = new_project(&db, &pm, "Ada 102").await;
    let mut seqs = Vec::new();
    for x in [1.0, 2.0, 3.0] {
        seqs.push(commit_point(&db, &pm, busy, x).await);
    }
    let q = commit_point(&db, &other, quiet, 9.0).await;
    // Two of the busy project's events and the quiet project's only one are older than a week.
    age(&db, busy, seqs[1]).await;
    age(&db, quiet, q).await;

    // A window under an hour is refused; the old events go one batch at a time, every tenant's.
    assert!(
        events::prune(&db.app, Duration::from_secs(1800), 10)
            .await
            .is_err()
    );
    assert_eq!(prune_all(&db).await, 3);
    assert_eq!(prune_all(&db).await, 0);

    // Busy: from the horizon on the log continues; from before it, reopen.
    let b = events::bounds(&db.app, &pm, busy).await.unwrap();
    assert_eq!((b.newest, b.pruned_through), (seqs[2], seqs[1]));
    assert!(matches!(
        events::after(&db.app, &pm, busy, 0, 10).await,
        Err(AppError::ResyncRequired(_))
    ));
    assert!(
        matches!(events::after(&db.app, &pm, busy, seqs[0], 10).await, Err(e) if e.code() == "resync_required")
    );
    let rest = events::after(&db.app, &pm, busy, seqs[1], 10)
        .await
        .unwrap();
    assert_eq!(
        rest.events
            .iter()
            .map(|e| e.seq.clone())
            .collect::<Vec<_>>(),
        vec![seqs[2].to_string()]
    );
    assert_eq!(
        projects::info(&db.app, &pm, busy)
            .await
            .unwrap()
            .event_cursor,
        seqs[2].to_string()
    );

    // Quiet: every event gone. Opening gives the horizon, never less (or it would be sent to reopen again and again).
    assert_eq!(
        projects::info(&db.app, &other, quiet)
            .await
            .unwrap()
            .event_cursor,
        q.to_string()
    );
    assert_eq!(events::latest(&db.app, &other, quiet).await.unwrap(), q);
    assert!(
        events::after(&db.app, &other, quiet, q, 10)
            .await
            .unwrap()
            .events
            .is_empty()
    );
    assert!(matches!(
        events::after(&db.app, &other, quiet, q - 1, 10).await,
        Err(AppError::ResyncRequired(_))
    ));

    // A project without events has nothing to resync from.
    assert_eq!(
        events::bounds(&db.app, &pm, fresh).await.unwrap(),
        events::Bounds {
            newest: 0,
            pruned_through: 0
        }
    );
    assert!(
        events::after(&db.app, &pm, fresh, 0, 10)
            .await
            .unwrap()
            .events
            .is_empty()
    );

    // New events follow a prune as before.
    let next = commit_point(&db, &pm, busy, 4.0).await;
    let rest = events::after(&db.app, &pm, busy, seqs[2], 10)
        .await
        .unwrap();
    assert_eq!(rest.events[0].seq, next.to_string());

    // Horizons are tenant data: each tenant sees only its own.
    let seen = |access: &Access| {
        let app = db.app.clone();
        let scope = access.scope();
        async move {
            let mut tx = app.scoped(scope).await.unwrap();
            let n: i64 = sqlx::query_scalar("select count(*) from kentos.outbox_horizon")
                .fetch_one(&mut *tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
            n
        }
    };
    assert_eq!((seen(&pm).await, seen(&other).await), (1, 1));
    db.close().await;
}
