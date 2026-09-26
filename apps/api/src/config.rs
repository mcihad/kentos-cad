//! Server settings, from the environment first and `.env.local` second
//! (`KENTOS_ENV_FILE` names another file). Secrets are never logged.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use kentos_postgres::env;
use kentos_postgres::setup::{DATABASE_URL, OWNER_URL};

#[derive(Clone, Debug)]
pub struct OidcSettings {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    /// Accepted `aud` of bearer access tokens (default: the client id).
    pub audience: String,
    /// Button text on the sign-in page.
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub env_file: PathBuf,
    pub vars: BTreeMap<String, String>,
    pub database_url: Option<String>,
    pub owner_url: Option<String>,
    pub bind: String,
    pub port: u16,
    /// Where the browser reaches the app (`http://localhost:5173` in development).
    pub public_url: String,
    pub cookie_secure: bool,
    pub local_login: bool,
    pub oidc: Option<OidcSettings>,
    /// How long project events are kept (`KENTOS_EVENT_RETENTION_DAYS`, 1–3650 days, default 7).
    pub event_retention: Duration,
    /// How long a project moved to the trash stays restorable before it is
    /// removed for good (`KENTOS_TRASH_RETENTION_DAYS`, 1–3650 days, default
    /// 30; docs/adr/0028). Fixed for each project when it is moved there.
    pub trash_retention: Duration,
    /// Where file projects' objects are kept (`KENTOS_BLOB_DIR`; docs/adr/0031).
    /// Absent: `.run/blobs` beside the env file, for development; a server
    /// sets its own, on storage that is backed up with the database.
    pub blob_dir: PathBuf,
}

/// Days of event log kept unless `KENTOS_EVENT_RETENTION_DAYS` says otherwise.
pub const DEFAULT_RETENTION_DAYS: u64 = 7;

/// Days a project moved to the trash stays there unless `KENTOS_TRASH_RETENTION_DAYS` says otherwise.
pub const DEFAULT_TRASH_RETENTION_DAYS: u64 = 30;

/// `KENTOS_EVENT_RETENTION_DAYS`: whole days, at least one (a client away longer reopens the project).
pub fn retention(value: Option<&str>) -> Result<Duration, String> {
    days("KENTOS_EVENT_RETENTION_DAYS", value, DEFAULT_RETENTION_DAYS)
}

/// `KENTOS_TRASH_RETENTION_DAYS`: whole days, at least one.
pub fn trash_retention(value: Option<&str>) -> Result<Duration, String> {
    days(
        "KENTOS_TRASH_RETENTION_DAYS",
        value,
        DEFAULT_TRASH_RETENTION_DAYS,
    )
}

fn days(name: &str, value: Option<&str>, default: u64) -> Result<Duration, String> {
    let days = match value {
        None => default,
        Some(v) => v
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|d| (1..=3650).contains(d))
            .ok_or_else(|| format!("{name} 1 ile 3650 arasında bir gün sayısı olmalı: {v}"))?,
    };
    Ok(Duration::from_secs(days * 24 * 3600))
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let env_file =
            PathBuf::from(std::env::var("KENTOS_ENV_FILE").unwrap_or_else(|_| ".env.local".into()));
        let vars = env::read_file(&env_file)?;
        let get = |k: &str| env::lookup(&vars, k);
        let flag = |k: &str, default: bool| {
            get(k)
                .map(|v| matches!(v.as_str(), "1" | "true" | "yes"))
                .unwrap_or(default)
        };
        let port = match get("KENTOS_API_PORT") {
            Some(p) => p
                .parse()
                .map_err(|_| format!("KENTOS_API_PORT geçersiz: {p}"))?,
            None => 8787,
        };
        let oidc = match (get("KENTOS_OIDC_ISSUER"), get("KENTOS_OIDC_CLIENT_ID")) {
            (Some(issuer), Some(client_id)) => Some(OidcSettings {
                issuer: issuer.trim_end_matches('/').to_string(),
                audience: get("KENTOS_OIDC_AUDIENCE").unwrap_or_else(|| client_id.clone()),
                client_id,
                client_secret: get("KENTOS_OIDC_CLIENT_SECRET"),
                label: get("KENTOS_OIDC_LABEL").unwrap_or_else(|| "Kurum hesabıyla giriş".into()),
            }),
            (None, None) => None,
            _ => {
                return Err(
                    "OpenID için KENTOS_OIDC_ISSUER ile KENTOS_OIDC_CLIENT_ID birlikte verilmeli."
                        .into(),
                );
            }
        };
        Ok(Self {
            database_url: get(DATABASE_URL),
            owner_url: get(OWNER_URL),
            bind: get("KENTOS_API_BIND").unwrap_or_else(|| "127.0.0.1".into()),
            port,
            public_url: get("KENTOS_PUBLIC_URL")
                .unwrap_or_else(|| "http://localhost:5173".into())
                .trim_end_matches('/')
                .to_string(),
            cookie_secure: flag("KENTOS_COOKIE_SECURE", false),
            local_login: flag("KENTOS_LOCAL_LOGIN", true),
            oidc,
            event_retention: retention(get("KENTOS_EVENT_RETENTION_DAYS").as_deref())?,
            trash_retention: trash_retention(get("KENTOS_TRASH_RETENTION_DAYS").as_deref())?,
            blob_dir: get("KENTOS_BLOB_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    env_file
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."))
                        .join(".run/blobs")
                }),
            env_file,
            vars,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_event_retention_is_whole_days_within_bounds() {
        let day = 24 * 3600;
        assert_eq!(retention(None).unwrap().as_secs(), 7 * day);
        assert_eq!(retention(Some(" 30 ")).unwrap().as_secs(), 30 * day);
        for bad in ["0", "3651", "7.5", "yedi", "-1", ""] {
            assert!(retention(Some(bad)).is_err(), "{bad}");
            assert!(trash_retention(Some(bad)).is_err(), "{bad}");
        }
        // The trash keeps a project 30 days unless told otherwise; the error names the setting.
        assert_eq!(trash_retention(None).unwrap().as_secs(), 30 * day);
        assert_eq!(trash_retention(Some("90")).unwrap().as_secs(), 90 * day);
        assert!(
            trash_retention(Some("0"))
                .unwrap_err()
                .contains("KENTOS_TRASH_RETENTION_DAYS")
        );
    }
}
