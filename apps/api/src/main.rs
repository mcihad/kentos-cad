//! `kentosd`: the KentOS server (CLAUDE.md §14, §18). `serve` runs the API
//! on 127.0.0.1 (`KENTOS_API_PORT`, default 8787) and, in the background,
//! removes project events older than the retention window
//! (`KENTOS_EVENT_RETENTION_DAYS`) and projects whose time in the trash is
//! over (`KENTOS_TRASH_RETENTION_DAYS`); the other subcommands set up the
//! database and administer tenants and accounts (see `cli::USAGE`). Without
//! a database configured, `serve` answers `/v1/health` only, so the drawing
//! app keeps working as before.

mod audit_env;
mod cli;
mod config;
mod delivery;
mod extension_catalog;
mod http;
mod hub;
mod oidc;
mod processing_job;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use kentos_application::audit_catalog::AuditSource;
use kentos_application::blobs::Blobs;
use kentos_application::{events, lifecycle};
use kentos_postgres::Db;

use crate::config::Config;
use crate::extension_catalog::audits::{CatalogAudits, PrepareEnvironments};
use crate::extension_catalog::postgres::PgStore;
use crate::extension_catalog::{Accounts, Catalog};
use crate::http::AppState;

/// The event log is pruned a minute after the start, then every hour, in batches of this many events.
const PRUNE_FIRST: Duration = Duration::from_secs(60);
const PRUNE_EVERY: Duration = Duration::from_secs(3600);
const PRUNE_BATCH: i32 = 2000;

/// Removes events older than `keep`. Every batch is one short statement on a
/// pooled connection, so requests are never held up behind it; a failure is
/// logged and the next round tries again. Clients whose cursor fell behind
/// are told to reopen (`resyncRequired`, events.rs).
async fn prune_events(db: Db, keep: Duration) {
    let mut every =
        tokio::time::interval_at(tokio::time::Instant::now() + PRUNE_FIRST, PRUNE_EVERY);
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        every.tick().await;
        let mut removed = 0;
        loop {
            match events::prune(&db, keep, PRUNE_BATCH).await {
                Ok(n) => {
                    removed += n;
                    if n < i64::from(PRUNE_BATCH) {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "eski olaylar budanamadı; bir sonraki turda yeniden denenecek");
                    break;
                }
            }
        }
        if removed > 0 {
            tracing::info!(silinen = removed, "eski olaylar budandı");
        }
    }
}

/// The object store of file projects is cleaned (docs/adr/0031) two minutes after
/// the start, then every hour: uploads nobody committed, stray upload files, and
/// the objects of projects removed for good.
const CLEAN_FIRST: Duration = Duration::from_secs(120);
const CLEAN_EVERY: Duration = Duration::from_secs(3600);

async fn clean_store(db: Db, blobs: Blobs, job_retention: Duration) {
    let mut every =
        tokio::time::interval_at(tokio::time::Instant::now() + CLEAN_FIRST, CLEAN_EVERY);
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        every.tick().await;
        match kentos_application::files::cleanup(&db, &blobs, kentos_application::files::SETTLED)
            .await
        {
            Ok(done) if done != Default::default() => tracing::info!(
                suresi_dolan = done.expired,
                sahipsiz = done.swept,
                silinen_proje = done.purged,
                kontrol_noktasi = done.checkpoints,
                "dosya deposu temizlendi"
            ),
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = ?e, "dosya deposu temizlenemedi; bir sonraki turda yeniden denenecek")
            }
        }
        // Finished server jobs and their artifacts past the retention (docs/adr/0547).
        match kentos_application::job_upkeep::expire(&db, &blobs, job_retention, JOB_EXPIRE_BATCH).await {
            Ok(done) if done != Default::default() => tracing::info!(
                is = done.jobs,
                sonuc_dosyasi = done.objects,
                "süresi dolan sunucu işleri silindi"
            ),
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = ?e, "eski sunucu işleri silinemedi; bir sonraki turda yeniden denenecek")
            }
        }
    }
}

/// Finished jobs removed at a time by the retention sweep.
const JOB_EXPIRE_BATCH: i32 = 200;

/// Projects whose time in the trash is over are removed for good (docs/adr/0028) a
/// minute and a half after the start, then every hour, this many at a time.
const PURGE_FIRST: Duration = Duration::from_secs(90);
const PURGE_EVERY: Duration = Duration::from_secs(3600);
const PURGE_BATCH: i32 = 20;

/// The trash's retention: removes for good the projects whose `purge_after`
/// passed (fixed when each was moved there). Each project is its own short
/// transaction in the database; a failure is logged and the next round tries
/// again. Open editors of such a project had stopped when it was trashed.
async fn purge_trash(db: Db) {
    let mut every =
        tokio::time::interval_at(tokio::time::Instant::now() + PURGE_FIRST, PURGE_EVERY);
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        every.tick().await;
        let mut removed = 0;
        loop {
            match lifecycle::purge_expired(&db, PURGE_BATCH).await {
                Ok(n) => {
                    removed += n;
                    if n < i64::from(PURGE_BATCH) {
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "süresi dolan çöp kutusu projeleri silinemedi; bir sonraki turda yeniden denenecek");
                    break;
                }
            }
        }
        if removed > 0 {
            tracing::info!(
                silinen = removed,
                "süresi dolan çöp kutusu projeleri kalıcı olarak silindi"
            );
        }
    }
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}

async fn serve(config: Config) -> Result<(), String> {
    let database = match &config.database_url {
        Some(url) => {
            let db = Db::connect(url, 16, Duration::from_secs(5))
                .await
                .map_err(|e| format!("Veritabanına bağlanılamadı: {e}"))?;
            if !db
                .schema_ready()
                .await
                .map_err(|e| format!("Şema denetlenemedi: {e}"))?
            {
                return Err(
                    "Veritabanı şeması bu sürüme göre eski; önce `kentosd migrate` çalıştırın."
                        .into(),
                );
            }
            Some(db)
        }
        None => {
            tracing::warn!(
                "KENTOS_DATABASE_URL yok: yalnızca /v1/health yanıt verir (`kentosd db-setup` ile kurun)"
            );
            None
        }
    };
    let ip: std::net::IpAddr = config
        .bind
        .parse()
        .map_err(|_| format!("KENTOS_API_BIND geçersiz: {}", config.bind))?;
    let addr = SocketAddr::from((ip, config.port));
    let oidc = match &config.oidc {
        Some(settings) => Some(Arc::new(oidc::Oidc::new(
            settings.clone(),
            &config.public_url,
        )?)),
        None => None,
    };
    let blobs = Blobs::new(&config.blob_dir);
    let proxy = http::proxy::Proxy::new(http::proxy::Reach::parse(config.proxy_allow.as_deref())?)?;
    let catalog = extension_catalog(&config, database.as_ref(), &blobs)?;
    if let Some(db) = &database {
        // The religious holidays the legal deadlines are counted over (docs/adr/9000 §19); the fixed ones are code.
        if let Err(e) = kentos_application::portal::holidays::load(db).await {
            tracing::warn!("Tatil takvimi okunamadı; yalnız sabit tatiller ve hafta sonu sayılır: {e}");
        }
        tokio::spawn(prune_events(db.clone(), config.event_retention));
        tokio::spawn(purge_trash(db.clone()));
        tokio::spawn(clean_store(db.clone(), blobs.clone(), config.job_retention));
        // Notifications by e-mail and SMS (docs/adr/9000 §17), when the server is set up to send them.
        if let Some(delivery) = config.delivery.clone() {
            tokio::spawn(delivery.run(db.clone()));
        }
        let audits = audit_source(&config, catalog.as_ref());
        spawn_workers(db, &blobs, config.workers, config.job_limits, audits);
    }
    let signups_per_hour = config.signups_per_hour;
    let state = AppState {
        config: Arc::new(config),
        database,
        oidc,
        hub: hub::Hub::default(),
        logins: Default::default(),
        signups: Arc::new(http::signup_guard::SignupGuard::new(signups_per_hour)),
        blobs,
        proxy: Some(Arc::new(proxy)),
        // Off unless the setting opens it (docs/adr/8059 §13): its routes answer 404 then.
        extension_catalog: catalog,
    };
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("KentOS API {addr} adresini açamadı: {e}"))?;
    println!("KentOS API: http://{addr}/v1/health");
    // The connection's address reaches the handlers (the sign-up's limit per address, docs/adr/9000 §20).
    axum::serve(listener, http::router(state).into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(|e| format!("KentOS API durdu: {e}"))
}

/// The extension catalog on its persistent store when `KENTOS_EXTENSION_CATALOG=on` (docs/adr/8059 §13): the routes
/// and the workers' audit source read the same one, and a release published there gets its audit's environment
/// (docs/adr/8057 §66). Its accounts are the server's sessions, so it takes no person's decision until it asks for a
/// fresh sign-in.
fn extension_catalog(
    config: &Config,
    database: Option<&Db>,
    blobs: &Blobs,
) -> Result<Option<Arc<Catalog>>, String> {
    if !config.extension_catalog {
        return Ok(None);
    }
    let db = database.ok_or(
        "KENTOS_EXTENSION_CATALOG=on veritabanı ister: KENTOS_DATABASE_URL yok (`kentosd db-setup`).",
    )?;
    let store = Arc::new(PgStore::new(db.clone(), blobs.clone()));
    let hook = Arc::new(PrepareEnvironments::new(config.audit_envs.clone()));
    tracing::info!(
        "eklenti kataloğu açık (kalıcı depo); kişinin kararları taze kimlik doğrulaması gelene dek verilmez"
    );
    Ok(Some(Arc::new(
        Catalog::new(store, Accounts::Sessions)
            .on_publish(hook)
            .wheels(config.audit_envs.wheels_dir()),
    )))
}

/// Where `project.audit` jobs take their rules from: the catalog and the environments' map, read again at every job.
fn audit_source(config: &Config, catalog: Option<&Arc<Catalog>>) -> Option<Arc<dyn AuditSource>> {
    catalog.map(|c| {
        Arc::new(CatalogAudits::new(c.clone(), config.audit_envs.map())) as Arc<dyn AuditSource>
    })
}

/// How long a worker's claim lasts without a beat, and how long an idle worker waits before looking again.
const WORKER_LEASE: Duration = Duration::from_secs(60);
const WORKER_IDLE: Duration = Duration::from_secs(2);

/// Server-job workers (docs/adr/0542): each claims, runs and finishes one job at a time; `project.audit` jobs take
/// their rules from `audits` (none: the catalog is off, and they say so).
fn spawn_workers(
    db: &Db,
    blobs: &Blobs,
    count: u32,
    limits: kentos_application::jobs::JobLimits,
    audits: Option<Arc<dyn AuditSource>>,
) -> Vec<tokio::task::JoinHandle<()>> {
    // The tools of `project.processing` jobs run in a child of this program (docs/adr/0559 §3); without it the
    // workers leave those jobs to another that has it.
    let processor: Option<Arc<dyn kentos_application::processing_job::Processor>> =
        match processing_job::ChildProcessor::new() {
            Ok(p) => Some(Arc::new(match audits {
                Some(source) => p.with_audits(source),
                None => p,
            })),
            Err(e) => {
                tracing::warn!(error = %e, "araç işlemi yok; işçiler project.processing işi almayacak");
                None
            }
        };
    (0..count)
        .map(|_| {
            let owner = uuid::Uuid::now_v7();
            tokio::spawn(kentos_application::jobs::run_worker(
                db.clone(),
                blobs.clone(),
                owner,
                WORKER_LEASE,
                WORKER_IDLE,
                limits,
                processor.clone(),
                Arc::new(std::sync::atomic::AtomicBool::new(false)),
                move |done| match done {
                    Ok(w) => tracing::info!(worker = %owner, job = %w.job, state = ?w.state, "sunucu işi bitti"),
                    Err(e) => tracing::warn!(worker = %owner, error = %e, "sunucu işi alınamadı; yeniden denenecek"),
                },
            ))
        })
        .collect()
}

/// `kentosd worker`: server-job workers alone, without the API (`KENTOS_WORKERS`, at least one).
async fn worker(config: Config) -> Result<(), String> {
    let url = config
        .database_url
        .clone()
        .ok_or("KENTOS_DATABASE_URL yok: işler veritabanında; `kentosd db-setup` ile kurun.")?;
    let db = Db::connect(&url, 4 + config.workers.max(1) * 2, Duration::from_secs(10))
        .await
        .map_err(|e| format!("veritabanına bağlanılamadı: {e}"))?;
    if !db.schema_ready().await.map_err(|e| e.to_string())? {
        return Err("Veritabanının şeması bu sürümün değil: `kentosd migrate` çalıştırın.".into());
    }
    let blobs = Blobs::new(&config.blob_dir);
    let catalog = extension_catalog(&config, Some(&db), &blobs)?;
    let audits = audit_source(&config, catalog.as_ref());
    let handles = spawn_workers(
        &db,
        &blobs,
        config.workers.max(1),
        config.job_limits,
        audits,
    );
    println!("KentOS sunucu işleri: {} çalışan", handles.len());
    shutdown().await;
    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("KENTOS_LOG")
                .unwrap_or_else(|_| "info,sqlx=warn,tower_http=info".into()),
        )
        .with_target(false)
        .init();
    let result = async {
        let args = cli::Args::parse(std::env::args().skip(1))?;
        let config = Config::load()?;
        if args.words.is_empty() || args.words == ["serve"] {
            serve(config).await
        } else if args.words == ["worker"] {
            worker(config).await
        } else {
            cli::run(&config, &args).await
        }
    }
    .await;
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
