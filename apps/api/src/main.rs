//! `kentosd`: the KentOS server (CLAUDE.md §14, §18). `serve` runs the API
//! on 127.0.0.1 (`KENTOS_API_PORT`, default 8787) and, in the background,
//! removes project events older than the retention window
//! (`KENTOS_EVENT_RETENTION_DAYS`) and projects whose time in the trash is
//! over (`KENTOS_TRASH_RETENTION_DAYS`); the other subcommands set up the
//! database and administer tenants and accounts (see `cli::USAGE`). Without
//! a database configured, `serve` answers `/v1/health` only, so the drawing
//! app keeps working as before.

mod cli;
mod config;
mod http;
mod hub;
mod oidc;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use kentos_application::{events, lifecycle};
use kentos_postgres::Db;

use crate::config::Config;
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

async fn clean_store(db: Db, blobs: kentos_application::blobs::Blobs) {
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
                "dosya deposu temizlendi"
            ),
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(error = ?e, "dosya deposu temizlenemedi; bir sonraki turda yeniden denenecek")
            }
        }
    }
}

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
    let blobs = kentos_application::blobs::Blobs::new(&config.blob_dir);
    if let Some(db) = &database {
        tokio::spawn(prune_events(db.clone(), config.event_retention));
        tokio::spawn(purge_trash(db.clone()));
        tokio::spawn(clean_store(db.clone(), blobs.clone()));
    }
    let state = AppState {
        config: Arc::new(config),
        database,
        oidc,
        hub: hub::Hub::default(),
        logins: Default::default(),
        blobs,
    };
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("KentOS API {addr} adresini açamadı: {e}"))?;
    println!("KentOS API: http://{addr}/v1/health");
    axum::serve(listener, http::router(state))
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(|e| format!("KentOS API durdu: {e}"))
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
