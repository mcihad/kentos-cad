//! How long the persistent store's reads and writes take on the tests' database (docs/adr/8059 §13): a store that
//! times the calls of ten releases' flows.

use std::sync::Mutex;
use std::time::Instant;

use super::*;
use crate::extension_catalog::store::{Access, StoreFuture};

/// A store that times its reads and writes, the persistent one inside.
struct Timed {
    inner: Arc<dyn ReleaseStore>,
    reads: Mutex<Vec<Duration>>,
    writes: Mutex<Vec<Duration>>,
    lists: Mutex<Vec<Duration>>,
}

impl ReleaseStore for Timed {
    fn releases<'a>(&'a self, catalog: &'a CatalogId) -> StoreFuture<'a, Vec<Release>> {
        Box::pin(async move {
            let began = Instant::now();
            let done = self.inner.releases(catalog).await;
            self.lists.lock().unwrap().push(began.elapsed());
            done
        })
    }

    fn release<'a>(&'a self, key: &'a ReleaseKey) -> StoreFuture<'a, Option<Release>> {
        Box::pin(async move {
            let began = Instant::now();
            let done = self.inner.release(key).await;
            self.reads.lock().unwrap().push(began.elapsed());
            done
        })
    }

    fn write<'a>(
        &'a self,
        key: &'a ReleaseKey,
        moves: Option<usize>,
        release: &'a Release,
    ) -> StoreFuture<'a, ()> {
        Box::pin(async move {
            let began = Instant::now();
            let done = self.inner.write(key, moves, release).await;
            self.writes.lock().unwrap().push(began.elapsed());
            done
        })
    }

    fn put_package<'a>(&'a self, digest: &'a str, bytes: Bytes) -> StoreFuture<'a, ()> {
        self.inner.put_package(digest, bytes)
    }

    fn package<'a>(&'a self, digest: &'a str) -> StoreFuture<'a, Option<Bytes>> {
        self.inner.package(digest)
    }

    fn access<'a>(&'a self, catalog: &'a CatalogId, account: &'a str) -> StoreFuture<'a, Access> {
        self.inner.access(catalog, account)
    }

    fn trusted<'a>(&'a self, catalog: &'a CatalogId, publisher: &'a str) -> StoreFuture<'a, bool> {
        self.inner.trusted(catalog, publisher)
    }
}

/// The median and the 95th percentile, in milliseconds.
fn spread(times: &Mutex<Vec<Duration>>) -> (usize, f64, f64) {
    let mut t = times.lock().unwrap().clone();
    t.sort();
    let at = |q: f64| t[((t.len() - 1) as f64 * q).round() as usize].as_secs_f64() * 1000.0;
    (t.len(), at(0.5), at(0.95))
}

#[tokio::test]
async fn how_long_a_release_s_read_and_write_take() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let bench = Bench::database(&db);
    grants(&bench).await;
    let timed = Arc::new(Timed {
        inner: bench.store(),
        reads: Mutex::default(),
        writes: Mutex::default(),
        lists: Mutex::default(),
    });
    let app = app_with(Some(Catalog::new(timed.clone(), Accounts::Named)));
    // Ten versions from upload to publication: twelve moves each, every one read and written over its record.
    for minor in 0..10 {
        let version = format!("1.{minor}.0");
        in_review(&app, &version).await;
        let at = uri(PUBLIC, &version);
        act(&app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
        let (status, view) = act(&app, Via::Mcp, "ayse", &at, json!({ "action": "publish" })).await;
        assert_eq!((status, state_of(&view)), (StatusCode::OK, "published"));
    }
    let (writes, write_median, write_p95) = spread(&timed.writes);
    let (reads, read_median, read_p95) = spread(&timed.reads);
    let (lists, list_median, list_p95) = spread(&timed.lists);
    eprintln!(
        "kalıcı depo: {writes} yazma, ortanca {write_median:.2} ms, %95 {write_p95:.2} ms; {reads} okuma, ortanca \
         {read_median:.2} ms, %95 {read_p95:.2} ms; {lists} liste (en çok 10 sürüm), ortanca {list_median:.2} ms, \
         %95 {list_p95:.2} ms"
    );
    assert_eq!(writes, 10 * 12);
    assert!(
        write_p95 < 1000.0 && read_p95 < 1000.0,
        "a store call over a second"
    );
    db.close().await;
}
