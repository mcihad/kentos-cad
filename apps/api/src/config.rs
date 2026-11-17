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
    /// The scan of uploaded files (docs/adr/9000 §14): `KENTOS_AV_ADDRESS` (an antivirus daemon, `tcp://host:port` or
    /// `unix:///path`), `KENTOS_AV_REQUIRED` (refuse what it cannot scan) and `KENTOS_AV_MAX_BYTES`. Absent: the type
    /// rules alone.
    pub scanner: kentos_application::scan::Scanner,
    /// Notifications by e-mail and SMS (docs/adr/9000 §17; crate::delivery): `KENTOS_MAIL_TRANSPORT`,
    /// `KENTOS_SMS_TRANSPORT` and their settings. Absent: the bell alone.
    pub delivery: Option<crate::delivery::Delivery>,
    /// E-signing a dossier's files (docs/adr/9004): `KENTOS_IMZA_SIGNING_KEY` (or the file `KENTOS_IMZA_SIGNING_KEY_FILE`)
    /// is the server's Ed25519 key, never shown; `KENTOS_IMZA_ISSUER` and `KENTOS_IMZA_AUDIENCE` the names the signing
    /// service checks. Absent: the signing routes answer 409.
    pub signing: Option<std::sync::Arc<kentos_application::portal::signing::Signer>>,
    /// The accounts one address may open by signing up in an hour (`KENTOS_SIGNUP_PER_HOUR`, 5; docs/adr/9000 §20).
    pub signups_per_hour: u32,
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
    /// 1–10 000, default 50; `KENTOS_JOB_USER_LIMIT`: of them, one user's, 1–10 000, default 10, docs/adr/0739 §1;
    /// `KENTOS_JOB_ARTIFACT_MAX_MB`: the biggest artifact of a job, 1–1 048 576 MB, default 2 048; docs/adr/0547;
    /// `KENTOS_JOB_INPUT_MAX_MB`: the biggest file staged for an import job, default 2 048).
    pub job_limits: kentos_application::jobs::JobLimits,
    /// How long a finished job and its artifact are kept (`KENTOS_JOB_RETENTION_DAYS`, 1–3650 days, default 14).
    pub job_retention: Duration,
    /// The audit rules' Python environments (docs/adr/8057 §66), read by [`audit_envs`]:
    /// - `KENTOS_AUDIT_ENV_DIR`: the environments, uv's cache, the wheel folder and the interpreter map; absent:
    ///   `.run/denetim-ortamlari` beside the env file, for development.
    /// - `KENTOS_AUDIT_BASE_PYTHON`: KentOS's Python they are made from, a CPython in KentOS's range; absent: the one
    ///   uv installed in that folder.
    /// - `KENTOS_AUDIT_WHEELS`: the wheel folder the administrator fills; absent: `tekerlekler` in that folder.
    /// - `KENTOS_AUDIT_INDEX_URL`: an index, http or https: the network while preparing, where the administrator
    ///   allows it; absent: offline.
    /// - `KENTOS_AUDIT_UV`: uv; absent: the installation's own, else `PATH`.
    /// - `KENTOS_AUDIT_SDK_DIR`: where KentOS's library's wheel is; absent: the installation's beside the program.
    /// - `KENTOS_AUDIT_PYTHONS`: a JSON object `{"eklenti@sürüm": "/…/bin/python"}`, under the file the server writes.
    ///
    /// Paths are absolute.
    pub audit_envs: crate::audit_env::EnvSettings,
    /// The extension catalog (docs/adr/8059 §13): `KENTOS_EXTENSION_CATALOG=on` opens it on its persistent store
    /// (the database and the object store); absent or `off`, its routes answer 404 and `project.audit` jobs say the
    /// catalog is off. Open, it takes uploads and every move but a person's decision, which needs a fresh sign-in the
    /// server does not ask for yet.
    pub extension_catalog: bool,
}

/// `KENTOS_EXTENSION_CATALOG`: `on` or `off` (the default); anything else is a mistake, not a silent `off`.
pub fn extension_catalog(value: Option<&str>) -> Result<bool, String> {
    match value.map(str::trim) {
        None | Some("off") => Ok(false),
        Some("on") => Ok(true),
        Some(other) => Err(format!(
            "KENTOS_EXTENSION_CATALOG on ya da off olmalı: {other}"
        )),
    }
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

/// `KENTOS_JOB_LIMIT`, `KENTOS_JOB_ARTIFACT_MAX_MB`, `KENTOS_JOB_CPU_SECONDS`, `KENTOS_JOB_MEMORY_MB` and
/// `KENTOS_JOB_INPUT_MAX_MB`: what server jobs may take (CPU and memory are a worker-run job's budget, docs/adr/0559 §4;
/// the last is the biggest file staged for a `project.import` job, docs/adr/0561 §1).
pub fn job_limits(
    limit: Option<&str>,
    artifact_mb: Option<&str>,
    cpu_seconds: Option<&str>,
    memory_mb: Option<&str>,
    input_mb: Option<&str>,
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
    let input = number(
        "KENTOS_JOB_INPUT_MAX_MB",
        input_mb,
        1_048_576,
        defaults.input_max / (1024 * 1024),
    )?;
    Ok(kentos_application::jobs::JobLimits {
        tenant_active: u32::try_from(tenant_active).unwrap_or(u32::MAX),
        user_active: defaults.user_active,
        artifact_max: mb * 1024 * 1024,
        cpu_seconds: u32::try_from(cpu).unwrap_or(u32::MAX),
        memory_mb: u32::try_from(memory).unwrap_or(u32::MAX),
        input_max: input * 1024 * 1024,
    })
}

/// `KENTOS_JOB_USER_LIMIT`: how many queued and running jobs one user may have in an organisation (docs/adr/0739 §1).
pub fn job_user_limit(value: Option<&str>) -> Result<u32, String> {
    match value {
        None => Ok(kentos_contracts::SERVER_JOB_USER_ACTIVE_DEFAULT),
        Some(v) => v
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|n| (1..=10_000).contains(n))
            .ok_or_else(|| {
                format!("KENTOS_JOB_USER_LIMIT 1 ile 10000 arasında bir sayı olmalı: {v}")
            }),
    }
}

/// The audit environments' settings ([`Config::audit_envs`]) from `get`, the env file at `env_file`.
pub fn audit_envs(
    get: impl Fn(&str) -> Option<String>,
    env_file: &std::path::Path,
) -> Result<crate::audit_env::EnvSettings, String> {
    let path = |name: &str| -> Result<Option<PathBuf>, String> {
        match get(name) {
            None => Ok(None),
            Some(v) if std::path::Path::new(v.trim()).is_absolute() => {
                Ok(Some(PathBuf::from(v.trim())))
            }
            Some(v) => Err(format!("{name} mutlak bir yol olmalı: {v}")),
        }
    };
    let index_url = match get("KENTOS_AUDIT_INDEX_URL") {
        None => None,
        Some(v) if v.trim().starts_with("https://") || v.trim().starts_with("http://") => {
            Some(v.trim().to_owned())
        }
        Some(v) => {
            return Err(format!(
                "KENTOS_AUDIT_INDEX_URL http ya da https adresi olmalı: {v}"
            ));
        }
    };
    let pythons = match get("KENTOS_AUDIT_PYTHONS") {
        None => kentos_application::audit_catalog::AuditPythons::default(),
        Some(v) => kentos_application::audit_catalog::AuditPythons::parse(&v)
            .map_err(|e| format!("KENTOS_AUDIT_PYTHONS: {e}"))?,
    };
    Ok(crate::audit_env::EnvSettings {
        dir: path("KENTOS_AUDIT_ENV_DIR")?.unwrap_or_else(|| {
            env_file
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(".run/denetim-ortamlari")
        }),
        base_python: path("KENTOS_AUDIT_BASE_PYTHON")?,
        wheels: path("KENTOS_AUDIT_WHEELS")?,
        index_url,
        uv: path("KENTOS_AUDIT_UV")?,
        sdk_dirs: match path("KENTOS_AUDIT_SDK_DIR")? {
            Some(dir) => vec![dir],
            None => kentos_python_env::sdk::dirs(),
        },
        pythons,
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
        let signups_per_hour = match get("KENTOS_SIGNUP_PER_HOUR") {
            None => 5,
            Some(v) => v
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|n| (1..=10_000).contains(n))
                .ok_or_else(|| format!("KENTOS_SIGNUP_PER_HOUR 1 ile 10000 arasında bir sayı olmalı: {v}"))?,
        };
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
        let scanner = kentos_application::scan::Scanner::configured(|k| get(k))?;
        let mut config = Self {
            scanner,
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
            job_limits: kentos_application::jobs::JobLimits {
                user_active: job_user_limit(get("KENTOS_JOB_USER_LIMIT").as_deref())?,
                ..job_limits(
                    get("KENTOS_JOB_LIMIT").as_deref(),
                    get("KENTOS_JOB_ARTIFACT_MAX_MB").as_deref(),
                    get("KENTOS_JOB_CPU_SECONDS").as_deref(),
                    get("KENTOS_JOB_MEMORY_MB").as_deref(),
                    get("KENTOS_JOB_INPUT_MAX_MB").as_deref(),
                )?
            },
            job_retention: job_retention(get("KENTOS_JOB_RETENTION_DAYS").as_deref())?,
            audit_envs: audit_envs(get, &env_file)?,
            extension_catalog: extension_catalog(get("KENTOS_EXTENSION_CATALOG").as_deref())?,
            env_file,
            vars,
            delivery: None,
            signing: None,
            signups_per_hour,
        };
        let delivery = crate::delivery::Delivery::configured(|k| env::lookup(&config.vars, k), &config.public_url)?;
        config.delivery = delivery;
        config.signing = signing(|k| env::lookup(&config.vars, k))?;
        Ok(config)
    }
}

/// The server's e-signing key, from the environment or a file (a file keeps it out of the process list and the
/// environment of child processes).
fn signing(
    get: impl Fn(&str) -> Option<String>,
) -> Result<Option<std::sync::Arc<kentos_application::portal::signing::Signer>>, String> {
    let key = match get("KENTOS_IMZA_SIGNING_KEY_FILE").filter(|p| !p.trim().is_empty()) {
        Some(path) => Some(
            std::fs::read_to_string(path.trim())
                .map_err(|e| format!("KENTOS_IMZA_SIGNING_KEY_FILE okunamadı ({path}): {e}"))?,
        ),
        None => get("KENTOS_IMZA_SIGNING_KEY"),
    };
    let Some(key) = key.filter(|k| !k.trim().is_empty()) else {
        return Ok(None);
    };
    let signer = kentos_application::portal::signing::Signer::from_config(
        &key,
        get("KENTOS_IMZA_ISSUER").as_deref(),
        get("KENTOS_IMZA_AUDIENCE").as_deref(),
    )?;
    Ok(Some(std::sync::Arc::new(signer)))
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

    /// The audit environments' settings: absolute paths, an http(s) index, the setting's map read; the folder beside
    /// the env file when none is named; every wrong one names itself.
    #[test]
    fn the_audit_environment_settings_are_read_and_name_themselves() {
        let env_file = std::path::Path::new("/srv/kentos/.env.local");
        let none = audit_envs(|_| None, env_file).unwrap();
        assert_eq!(
            none.dir,
            PathBuf::from("/srv/kentos/.run/denetim-ortamlari")
        );
        assert_eq!(
            none.wheels_dir(),
            PathBuf::from("/srv/kentos/.run/denetim-ortamlari/tekerlekler")
        );
        assert_eq!(
            (none.base_python, none.index_url, none.uv),
            (None, None, None)
        );
        let set = |name: &'static str, value: &'static str| {
            move |k: &str| (k == name).then(|| value.to_owned())
        };
        let given = audit_envs(set("KENTOS_AUDIT_ENV_DIR", "/veri/ortamlar"), env_file).unwrap();
        assert_eq!(given.dir, PathBuf::from("/veri/ortamlar"));
        assert_eq!(
            audit_envs(
                set(
                    "KENTOS_AUDIT_INDEX_URL",
                    "https://paketler.kurum.example/simple"
                ),
                env_file
            )
            .unwrap()
            .index_url
            .as_deref(),
            Some("https://paketler.kurum.example/simple")
        );
        assert_eq!(
            audit_envs(set("KENTOS_AUDIT_SDK_DIR", "/opt/kentos/tools"), env_file)
                .unwrap()
                .sdk_dirs,
            [PathBuf::from("/opt/kentos/tools")]
        );
        let mapped = audit_envs(
            set("KENTOS_AUDIT_PYTHONS", r#"{"a@1": "/a/bin/python"}"#),
            env_file,
        )
        .unwrap();
        assert_eq!(mapped.map().get("a", "1").as_deref(), Some("/a/bin/python"));
        for (name, value) in [
            ("KENTOS_AUDIT_ENV_DIR", "goreli/yol"),
            ("KENTOS_AUDIT_BASE_PYTHON", "python3"),
            ("KENTOS_AUDIT_WHEELS", "tekerlekler"),
            ("KENTOS_AUDIT_UV", "uv"),
            ("KENTOS_AUDIT_SDK_DIR", "tools"),
            ("KENTOS_AUDIT_INDEX_URL", "ftp://paketler"),
            ("KENTOS_AUDIT_PYTHONS", "[]"),
        ] {
            assert!(
                audit_envs(set(name, value), env_file)
                    .unwrap_err()
                    .contains(name),
                "{name}"
            );
        }
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
        let defaults = job_limits(None, None, None, None, None).unwrap();
        assert_eq!(
            (defaults.tenant_active, defaults.artifact_max),
            (50, 2 << 30)
        );
        let set = job_limits(
            Some(" 5 "),
            Some("64"),
            Some("90"),
            Some("512"),
            Some("128"),
        )
        .unwrap();
        assert_eq!((defaults.input_max, set.input_max), (2 << 30, 128 << 20));
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
                job_limits(None, None, cpu, memory, None)
                    .unwrap_err()
                    .contains(name),
                "{name}"
            );
        }
        for bad in ["0", "10001", "x", "-1", ""] {
            assert!(
                job_limits(Some(bad), None, None, None, None)
                    .unwrap_err()
                    .contains("KENTOS_JOB_LIMIT"),
                "{bad}"
            );
            assert!(
                job_user_limit(Some(bad))
                    .unwrap_err()
                    .contains("KENTOS_JOB_USER_LIMIT"),
                "{bad}"
            );
        }
        assert_eq!(job_user_limit(None), Ok(10));
        assert_eq!(job_user_limit(Some(" 3 ")), Ok(3));
        assert!(
            job_limits(None, Some("0"), None, None, None)
                .unwrap_err()
                .contains("KENTOS_JOB_ARTIFACT_MAX_MB")
        );
        for bad in ["0", "1048577", "x"] {
            assert!(
                job_limits(None, None, None, None, Some(bad))
                    .unwrap_err()
                    .contains("KENTOS_JOB_INPUT_MAX_MB"),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_extension_catalog_opens_only_when_the_setting_says_on() {
        assert_eq!(extension_catalog(None), Ok(false));
        assert_eq!(extension_catalog(Some("off")), Ok(false));
        assert_eq!(extension_catalog(Some(" on ")), Ok(true));
        for bad in ["1", "true", "acik", ""] {
            assert!(
                extension_catalog(Some(bad))
                    .unwrap_err()
                    .contains("KENTOS_EXTENSION_CATALOG"),
                "{bad}"
            );
        }
    }
}
