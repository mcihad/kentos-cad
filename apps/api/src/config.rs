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
    /// The networks of the organisation the map services' proxy may reach
    /// besides the public internet (`KENTOS_PROXY_ALLOW`: `10.20.0.0/16, fd00::/8`;
    /// docs/adr/0208 §13). Absent: none.
    pub proxy_allow: Option<String>,
    /// How many server-job workers `kentosd serve` runs beside the API (`KENTOS_WORKERS`, 0–16; docs/adr/0542).
    /// Absent: 1. `kentosd worker` runs workers alone.
    pub workers: u32,
    /// What server jobs may take (`KENTOS_JOB_LIMIT`: queued and running jobs of one organisation together,
    /// 1–10 000, default 50; `KENTOS_JOB_ARTIFACT_MAX_MB`: the biggest artifact of a job, 1–1 048 576 MB, default
    /// 2 048; docs/adr/0547).
    pub job_limits: kentos_application::jobs::JobLimits,
    /// How long a finished job and its artifact are kept (`KENTOS_JOB_RETENTION_DAYS`, 1–3650 days, default 14).
    pub job_retention: Duration,
}

/// Days of event log kept unless `KENTOS_EVENT_RETENTION_DAYS` says otherwise.
pub const DEFAULT_RETENTION_DAYS: u64 = 7;

/// Days a project moved to the trash stays there unless `KENTOS_TRASH_RETENTION_DAYS` says otherwise.
pub const DEFAULT_TRASH_RETENTION_DAYS: u64 = 30;

/// `KENTOS_JOB_RETENTION_DAYS`: whole days, at least one.
pub fn job_retention(value: Option<&str>) -> Result<Duration, String> {
    days(
        "KENTOS_JOB_RETENTION_DAYS",
        value,
        kentos_contracts::SERVER_JOB_RETENTION_DAYS_DEFAULT,
    )
}

/// `KENTOS_JOB_LIMIT`, `KENTOS_JOB_ARTIFACT_MAX_MB`, `KENTOS_JOB_CPU_SECONDS` and `KENTOS_JOB_MEMORY_MB`: what server
/// jobs may take (the last two are a `project.processing` job's budget, docs/adr/0559 §4).
pub fn job_limits(
    limit: Option<&str>,
    artifact_mb: Option<&str>,
    cpu_seconds: Option<&str>,
    memory_mb: Option<&str>,
) -> Result<kentos_application::jobs::JobLimits, String> {
    let defaults = kentos_application::jobs::JobLimits::default();
    let number = |name: &str, value: Option<&str>, max: u64, default: u64| -> Result<u64, String> {
        match value {
            None => Ok(default),
            Some(v) => v
                .trim()
                .parse::<u64>()
                .ok()
                .filter(|n| (1..=max).contains(n))
                .ok_or_else(|| format!("{name} 1 ile {max} arasında bir sayı olmalı: {v}")),
        }
    };
    let tenant_active = number(
        "KENTOS_JOB_LIMIT",
        limit,
        10_000,
        u64::from(defaults.tenant_active),
    )?;
    let mb = number(
        "KENTOS_JOB_ARTIFACT_MAX_MB",
        artifact_mb,
        1_048_576,
        defaults.artifact_max / (1024 * 1024),
    )?;
    let cpu = number(
        "KENTOS_JOB_CPU_SECONDS",
        cpu_seconds,
        604_800,
        u64::from(defaults.cpu_seconds),
    )?;
    let memory = number(
        "KENTOS_JOB_MEMORY_MB",
        memory_mb,
        4_194_304,
        u64::from(defaults.memory_mb),
    )?;
    Ok(kentos_application::jobs::JobLimits {
        tenant_active: u32::try_from(tenant_active).unwrap_or(u32::MAX),
        artifact_max: mb * 1024 * 1024,
        cpu_seconds: u32::try_from(cpu).unwrap_or(u32::MAX),
        memory_mb: u32::try_from(memory).unwrap_or(u32::MAX),
    })
}

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
            proxy_allow: get("KENTOS_PROXY_ALLOW"),
            workers: match get("KENTOS_WORKERS") {
                None => 1,
                Some(v) => v
                    .trim()
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n <= 16)
                    .ok_or_else(|| {
                        format!("KENTOS_WORKERS 0 ile 16 arasında bir sayı olmalı: {v}")
                    })?,
            },
            job_limits: job_limits(
                get("KENTOS_JOB_LIMIT").as_deref(),
                get("KENTOS_JOB_ARTIFACT_MAX_MB").as_deref(),
                get("KENTOS_JOB_CPU_SECONDS").as_deref(),
                get("KENTOS_JOB_MEMORY_MB").as_deref(),
            )?,
            job_retention: job_retention(get("KENTOS_JOB_RETENTION_DAYS").as_deref())?,
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

    #[test]
    fn the_job_settings_are_within_bounds_and_name_themselves() {
        let day = 24 * 3600;
        assert_eq!(job_retention(None).unwrap().as_secs(), 14 * day);
        assert_eq!(job_retention(Some("3")).unwrap().as_secs(), 3 * day);
        assert!(
            job_retention(Some("0"))
                .unwrap_err()
                .contains("KENTOS_JOB_RETENTION_DAYS")
        );
        let defaults = job_limits(None, None, None, None).unwrap();
        assert_eq!(
            (defaults.tenant_active, defaults.artifact_max),
            (50, 2 << 30)
        );
        let set = job_limits(Some(" 5 "), Some("64"), Some("90"), Some("512")).unwrap();
        assert_eq!((set.tenant_active, set.artifact_max), (5, 64 << 20));
        assert_eq!((defaults.cpu_seconds, defaults.memory_mb), (1_800, 4_096));
        assert_eq!((set.cpu_seconds, set.memory_mb), (90, 512));
        for (cpu, memory, name) in [
            (Some("0"), None, "KENTOS_JOB_CPU_SECONDS"),
            (Some("x"), None, "KENTOS_JOB_CPU_SECONDS"),
            (None, Some("0"), "KENTOS_JOB_MEMORY_MB"),
            (None, Some("4194305"), "KENTOS_JOB_MEMORY_MB"),
        ] {
            assert!(
                job_limits(None, None, cpu, memory)
                    .unwrap_err()
                    .contains(name),
                "{name}"
            );
        }
        for bad in ["0", "10001", "x", "-1", ""] {
            assert!(
                job_limits(Some(bad), None, None, None)
                    .unwrap_err()
                    .contains("KENTOS_JOB_LIMIT"),
                "{bad}"
            );
        }
        assert!(
            job_limits(None, Some("0"), None, None)
                .unwrap_err()
                .contains("KENTOS_JOB_ARTIFACT_MAX_MB")
        );
    }
}
