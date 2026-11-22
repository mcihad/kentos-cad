//! A database opened (docs/adr/9100 §6): its address read without a password, the password found where the host
//! keeps it, one pool of connections, each connection set the same way (`DateStyle` ISO so dates read as ISO text,
//! `standard_conforming_strings` so a backslash is a backslash, the program's name in `pg_stat_activity`). A failure
//! is said by its kind: the server not there, no answer in time, the login refused, anything else.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use kentos_data::{ConstraintKind, DataError, Reach};
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::{Executor, PgConnection};

use crate::catalog::Shape;
use crate::runtime;

/// Connections a database's pool holds at most: a view's reader, a table's pages, a transaction, a catalog question.
pub const MOST: u32 = 4;
/// How long a connection may take before the database counts as unreachable.
const CONNECT: Duration = Duration::from_secs(15);

/// An opened database.
pub struct PgSource {
    pub(crate) address: String,
    pub(crate) pool: PgPool,
    pub(crate) shapes: Arc<RwLock<HashMap<String, Shape>>>,
}

/// A `sqlx` error said as a data error: its kind by SQLSTATE or by how the connection failed.
pub(crate) fn error_of(e: sqlx::Error) -> DataError {
    match &e {
        sqlx::Error::Database(db) => {
            let code = db.code().map(|c| c.into_owned()).unwrap_or_default();
            let detail = db.message().to_owned();
            let constraint = db.constraint().map(str::to_owned);
            let kind = match code.as_str() {
                "23514" | "23P01" => Some(ConstraintKind::Check),
                "23505" => Some(ConstraintKind::Unique),
                "23503" => Some(ConstraintKind::Foreign),
                "23502" => Some(ConstraintKind::NotNull),
                _ => None,
            };
            if let Some(kind) = kind {
                let column = db
                    .try_downcast_ref::<sqlx::postgres::PgDatabaseError>()
                    .and_then(|p| p.column().map(str::to_owned));
                return DataError::Constraint {
                    kind,
                    name: constraint,
                    column,
                    detail,
                };
            }
            match code.as_str() {
                "42501" => DataError::Denied(detail),
                "42P01" => DataError::NoTable(detail),
                "42703" => DataError::Invalid(detail),
                "40001" | "40P01" => DataError::Conflict(detail),
                "28P01" | "28000" => DataError::Unreachable {
                    reach: Reach::Denied,
                    message: format!(
                        "Veritabanı girişi reddetti: {detail}. Kullanıcı adını ve parolayı denetleyin."
                    ),
                },
                "57014" => DataError::Cancelled,
                _ => DataError::Failed(detail),
            }
        }
        sqlx::Error::PoolTimedOut => DataError::Unreachable {
            reach: Reach::Timeout,
            message: "Veritabanı bağlantısı zamanında açılamadı: sunucu yoğun ya da ulaşılamıyor."
                .into(),
        },
        sqlx::Error::Io(io) => DataError::Unreachable {
            reach: Reach::Offline,
            message: format!(
                "Veritabanına ulaşılamadı ({io}): sunucu kapalı, adres yanlış ya da ağ engelli olabilir."
            ),
        },
        sqlx::Error::Tls(t) => DataError::Unreachable {
            reach: Reach::Failed,
            message: format!("Güvenli bağlantı kurulamadı (TLS): {t}"),
        },
        _ => DataError::Failed(e.to_string()),
    }
}

/// Whether an address carries a password (`user:secret@`, `?password=`): a project file must never hold one.
pub(crate) fn password_in(address: &str) -> bool {
    let rest = address.split_once("://").map_or(address, |(_, r)| r);
    let (head, query) = rest.split_once('?').unwrap_or((rest, ""));
    let user_has = head
        .split_once('@')
        .is_some_and(|(cred, _)| cred.contains(':'));
    user_has
        || query
            .split('&')
            .any(|kv| kv.to_ascii_lowercase().starts_with("password="))
}

/// The address's options with the password the host knows for it.
fn options(address: &str) -> Result<PgConnectOptions, DataError> {
    let mut o = PgConnectOptions::from_str(address)
        .map_err(|e| DataError::Invalid(format!("Veritabanı adresi okunamadı: {e}")))?;
    let known = kentos_data::secrets::password(
        o.get_host(),
        o.get_port(),
        o.get_database().unwrap_or_default(),
        o.get_username(),
    )
    .or_else(|| std::env::var("PGPASSWORD").ok().filter(|p| !p.is_empty()));
    if let Some(pw) = known {
        o = o.password(&pw);
    }
    Ok(o.application_name("KentOS"))
}

/// Every connection set the same way when it is made.
fn ready(conn: &mut PgConnection) -> futures_like::Boxed<'_> {
    Box::pin(async move {
        conn.execute(
            "SET DateStyle = 'ISO, YMD'; SET standard_conforming_strings = on; SET extra_float_digits = 3",
        )
        .await?;
        Ok(())
    })
}

mod futures_like {
    pub type Boxed<'c> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), sqlx::Error>> + Send + 'c>>;
}

impl PgSource {
    pub fn open(address: &str) -> Result<PgSource, DataError> {
        if password_in(address) {
            return Err(DataError::Invalid(
                "Adres parola taşımamalı: parolayı bağlantının kendisinde saklayın.".into(),
            ));
        }
        let o = options(address)?;
        let pool = runtime()?.block_on(async {
            tokio::time::timeout(
                CONNECT,
                PgPoolOptions::new()
                    .max_connections(MOST)
                    .min_connections(0)
                    .acquire_timeout(CONNECT)
                    .idle_timeout(Duration::from_secs(300))
                    .after_connect(|conn, _| ready(conn))
                    .connect_with(o),
            )
            .await
        });
        let pool = match pool {
            Err(_) => {
                return Err(DataError::Unreachable {
                    reach: Reach::Timeout,
                    message: format!(
                        "Veritabanı {} saniyede yanıt vermedi: sunucu kapalı, adres yanlış ya da ağ engelli olabilir.",
                        CONNECT.as_secs()
                    ),
                });
            }
            Ok(r) => r.map_err(error_of)?,
        };
        Ok(PgSource {
            address: address.to_owned(),
            pool,
            shapes: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}
