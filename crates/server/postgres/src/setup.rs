//! `kentosd db-setup`: prepares a PostgreSQL server for KentOS with an admin
//! connection, and can be run again safely. It creates the two roles (the
//! owner that runs migrations, the server's role without BYPASSRLS), the
//! database owned by the owner role, and the extensions only a superuser may
//! install (postgis, pgcrypto). It never touches other databases on the
//! server. Passwords are random, kept in `.env.local`, and reused when the
//! file already has them.

use std::collections::BTreeMap;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};

use crate::{APP_ROLE, OWNER_ROLE};

pub const DEFAULT_DATABASE: &str = "kentos_cad";
pub const DATABASE_URL: &str = "KENTOS_DATABASE_URL";
pub const OWNER_URL: &str = "KENTOS_DATABASE_OWNER_URL";
const OWNER_PASSWORD: &str = "KENTOS_DATABASE_OWNER_PASSWORD";
const APP_PASSWORD: &str = "KENTOS_DATABASE_APP_PASSWORD";

/// What `setup` did, for the command's report.
#[derive(Debug, Default)]
pub struct Report {
    pub created_roles: Vec<String>,
    pub created_database: bool,
    pub vars: BTreeMap<String, String>,
}

/// A random password: 244 bits from two version-4 UUIDs, hex only (safe inside SQL quotes).
pub fn random_secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some('a'..='z' | '_'))
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && name.len() <= 63
}

fn valid_secret(s: &str) -> bool {
    s.len() >= 32 && s.chars().all(|c| c.is_ascii_hexdigit())
}

async fn one_connection(options: PgConnectOptions) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
}

/// Creates or updates a login role; returns whether it was created.
async fn ensure_role(
    admin: &PgPool,
    role: &str,
    attributes: &str,
    password: &str,
) -> Result<bool, sqlx::Error> {
    let exists: bool =
        sqlx::query_scalar("select exists (select 1 from pg_roles where rolname = $1)")
            .bind(role)
            .fetch_one(admin)
            .await?;
    // Role names are constants of this crate and passwords are hex (checked): nothing user-typed is spliced in.
    let verb = if exists { "alter" } else { "create" };
    sqlx::query(AssertSqlSafe(format!(
        "{verb} role {role} with login {attributes} password '{password}'"
    )))
    .execute(admin)
    .await?;
    Ok(!exists)
}

/// Installs the extensions and connection rights in a database, as the admin connected to it.
pub async fn prepare_database(admin_on_db: &PgPool, database: &str) -> Result<(), sqlx::Error> {
    assert!(valid_name(database), "veritabanı adı geçersiz: {database}");
    for statement in [
        "create extension if not exists postgis".to_string(),
        "create extension if not exists pgcrypto".to_string(),
        format!("revoke connect, temporary on database {database} from public"),
        format!("grant connect on database {database} to {OWNER_ROLE}, {APP_ROLE}"),
        // PostgreSQL 15+: the public schema belongs to the database owner; the server role only uses it.
        format!("grant usage on schema public to {APP_ROLE}"),
    ] {
        sqlx::query(AssertSqlSafe(statement))
            .execute(admin_on_db)
            .await?;
    }
    Ok(())
}

/// Prepares the server: roles, database, extensions. `existing` is the current `.env.local`.
pub async fn setup(
    admin_url: &str,
    database: &str,
    existing: &BTreeMap<String, String>,
) -> Result<Report, String> {
    if !valid_name(database) {
        return Err(format!(
            "Veritabanı adı “{database}” geçersiz: küçük harf, rakam ve _ kullanın."
        ));
    }
    let admin_options: PgConnectOptions = admin_url
        .parse()
        .map_err(|e| format!("Yönetici adresi okunamadı: {e}"))?;
    let admin = one_connection(admin_options.clone())
        .await
        .map_err(|e| format!("PostgreSQL'e yönetici olarak bağlanılamadı: {e}"))?;
    let is_super: bool =
        sqlx::query_scalar("select rolsuper from pg_roles where rolname = current_user")
            .fetch_one(&admin)
            .await
            .map_err(|e| e.to_string())?;
    if !is_super {
        return Err(
            "Kurulum süper kullanıcı ister (postgis eklentisini yalnızca o kurabilir).".into(),
        );
    }

    let secret = |key: &str| {
        existing
            .get(key)
            .filter(|s| valid_secret(s))
            .cloned()
            .unwrap_or_else(random_secret)
    };
    let owner_password = secret(OWNER_PASSWORD);
    let app_password = secret(APP_PASSWORD);
    let mut report = Report::default();
    let db_err = |e: sqlx::Error| e.to_string();
    if ensure_role(
        &admin,
        OWNER_ROLE,
        "nosuperuser nocreatedb nocreaterole",
        &owner_password,
    )
    .await
    .map_err(db_err)?
    {
        report.created_roles.push(OWNER_ROLE.into());
    }
    // The server's role: not an owner, and row-level security always applies to it.
    if ensure_role(
        &admin,
        APP_ROLE,
        "nosuperuser nocreatedb nocreaterole nobypassrls noinherit",
        &app_password,
    )
    .await
    .map_err(db_err)?
    {
        report.created_roles.push(APP_ROLE.into());
    }

    let owner: Option<String> = sqlx::query_scalar(
        "select pg_get_userbyid(datdba)::text from pg_database where datname = $1",
    )
    .bind(database)
    .fetch_optional(&admin)
    .await
    .map_err(db_err)?;
    match owner.as_deref() {
        None => {
            sqlx::query(AssertSqlSafe(format!(
                "create database {database} owner {OWNER_ROLE}"
            )))
            .execute(&admin)
            .await
            .map_err(db_err)?;
            report.created_database = true;
        }
        Some(OWNER_ROLE) => {}
        Some(other) => {
            return Err(format!(
                "“{database}” veritabanı zaten var ve sahibi {other}. Başka bir uygulamanın verisi olabilir; farklı bir ad seçin (--database)."
            ));
        }
    }
    admin.close().await;

    let on_db = one_connection(admin_options.clone().database(database))
        .await
        .map_err(db_err)?;
    prepare_database(&on_db, database).await.map_err(db_err)?;
    on_db.close().await;

    let host = admin_options.get_host().to_string();
    let port = admin_options.get_port();
    let url = |role: &str, password: &str| {
        format!("postgres://{role}:{password}@{host}:{port}/{database}")
    };
    report
        .vars
        .insert(DATABASE_URL.into(), url(APP_ROLE, &app_password));
    report
        .vars
        .insert(OWNER_URL.into(), url(OWNER_ROLE, &owner_password));
    report.vars.insert(APP_PASSWORD.into(), app_password);
    report.vars.insert(OWNER_PASSWORD.into(), owner_password);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_secrets_are_checked_before_reaching_sql() {
        assert!(valid_name("kentos_cad") && valid_name("_x1"));
        assert!(
            !valid_name("Kentos")
                && !valid_name("a-b")
                && !valid_name("x; drop")
                && !valid_name("")
        );
        let s = random_secret();
        assert_eq!(s.len(), 64);
        assert!(valid_secret(&s) && !valid_secret("kisa") && !valid_secret(&format!("{s}'")));
    }
}
