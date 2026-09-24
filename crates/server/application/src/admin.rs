//! Administration from the command line (`kentosd tenant|user|member|project
//! …`), as the owner role: tenants, local accounts, memberships and seats,
//! and restoring a deleted project. There is no open sign-up (ADR 0007);
//! inviting from the interface comes later.
//! Passwords are hashed by PostgreSQL (`crypt` with a bcrypt salt, cost 12).

use kentos_contracts::TenantRole;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::identity::LOCAL_ISSUER;
use crate::tenancy::{role_from_db, role_to_db};

/// bcrypt reads at most 72 bytes; a longer password would be silently cut, so it is refused.
pub const PASSWORD_MIN_CHARS: usize = 10;
pub const PASSWORD_MAX_BYTES: usize = 72;

pub fn check_password(password: &str) -> AppResult<()> {
    if password.chars().count() < PASSWORD_MIN_CHARS {
        return Err(AppError::invalid(format!(
            "Parola en az {PASSWORD_MIN_CHARS} karakter olmalı."
        )));
    }
    if password.len() > PASSWORD_MAX_BYTES {
        return Err(AppError::invalid(format!(
            "Parola en çok {PASSWORD_MAX_BYTES} bayt olabilir (Türkçe harfler 2 bayt sayılır)."
        )));
    }
    Ok(())
}

pub fn check_login(login: &str) -> AppResult<()> {
    let ok = (3..=64).contains(&login.len())
        && login.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-' | b'@')
        });
    if ok {
        Ok(())
    } else {
        Err(AppError::invalid(
            "Giriş adı 3–64 karakter olmalı: küçük harf, rakam, . _ - @.",
        ))
    }
}

fn unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(d) if d.code().as_deref() == Some("23505"))
}

pub async fn create_tenant(owner: &PgPool, slug: &str, name: &str, seats: i32) -> AppResult<Uuid> {
    if name.trim().is_empty() || seats < 0 {
        return Err(AppError::invalid(
            "Kurum adı boş olamaz, koltuk sayısı negatif olamaz.",
        ));
    }
    let id = Uuid::now_v7();
    match sqlx::query(
        "insert into kentos.tenant (id, slug, name, seat_limit) values ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(slug)
    .bind(name.trim())
    .bind(seats)
    .execute(owner)
    .await
    {
        Ok(_) => Ok(id),
        Err(e) if unique_violation(&e) => Err(AppError::invalid(format!(
            "“{slug}” kısa adlı bir kurum zaten var."
        ))),
        Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23514") => {
            Err(AppError::invalid(
                "Kısa ad 2–63 karakter olmalı: küçük harf, rakam ve tire; harf ya da rakamla başlar.",
            ))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn create_local_user(
    owner: &PgPool,
    login: &str,
    name: &str,
    email: Option<&str>,
    password: &str,
) -> AppResult<Uuid> {
    check_login(login)?;
    check_password(password)?;
    if name.trim().is_empty() {
        return Err(AppError::invalid("Görünen ad boş olamaz."));
    }
    let id = Uuid::now_v7();
    let mut tx = owner.begin().await?;
    sqlx::query("insert into kentos.app_user (id, issuer, subject, display_name, email) values ($1, $2, $1::text, $3, $4)")
        .bind(id)
        .bind(LOCAL_ISSUER)
        .bind(name.trim())
        .bind(email)
        .execute(&mut *tx)
        .await?;
    let inserted = sqlx::query(
        "insert into kentos.local_credential (user_id, login, password_hash) values ($1, $2, public.crypt($3, public.gen_salt('bf', 12)))",
    )
    .bind(id)
    .bind(login)
    .bind(password)
    .execute(&mut *tx)
    .await;
    match inserted {
        Ok(_) => {
            tx.commit().await?;
            Ok(id)
        }
        Err(e) if unique_violation(&e) => Err(AppError::invalid(format!(
            "“{login}” giriş adı zaten kullanılıyor."
        ))),
        Err(e) => Err(e.into()),
    }
}

pub async fn set_password(owner: &PgPool, login: &str, password: &str) -> AppResult<()> {
    check_password(password)?;
    let done = sqlx::query(
        "update kentos.local_credential set password_hash = public.crypt($2, public.gen_salt('bf', 12)), updated_at = now() where lower(login) = lower($1)",
    )
    .bind(login)
    .bind(password)
    .execute(owner)
    .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::not_found(format!(
            "“{login}” giriş adlı hesap yok."
        )));
    }
    // A new password ends every open session of the account.
    sqlx::query(
        "update kentos.auth_session set revoked_at = now()
          where revoked_at is null and user_id = (select user_id from kentos.local_credential where lower(login) = lower($1))",
    )
    .bind(login)
    .execute(owner)
    .await?;
    Ok(())
}

async fn tenant_id(owner: &PgPool, slug: &str) -> AppResult<(Uuid, i32)> {
    sqlx::query_as("select id, seat_limit from kentos.tenant where slug = $1")
        .bind(slug)
        .fetch_optional(owner)
        .await?
        .ok_or_else(|| AppError::not_found(format!("“{slug}” kısa adlı kurum yok.")))
}

/// A local login or an account id.
pub async fn user_id(owner: &PgPool, who: &str) -> AppResult<Uuid> {
    if let Ok(id) = Uuid::parse_str(who) {
        return Ok(id);
    }
    sqlx::query_scalar("select user_id from kentos.local_credential where lower(login) = lower($1)")
        .bind(who)
        .fetch_optional(owner)
        .await?
        .ok_or_else(|| AppError::not_found(format!("“{who}” giriş adlı hesap yok.")))
}

/// Adds or changes a membership; with `seat`, also allocates a seat within the tenant's limit.
pub async fn set_membership(
    owner: &PgPool,
    tenant_slug: &str,
    who: &str,
    role: TenantRole,
    seat: bool,
) -> AppResult<()> {
    let (tenant, limit) = tenant_id(owner, tenant_slug).await?;
    let user = user_id(owner, who).await?;
    let mut tx = owner.begin().await?;
    sqlx::query(
        "insert into kentos.membership (tenant_id, user_id, role) values ($1, $2, $3)
         on conflict (tenant_id, user_id) do update set role = excluded.role, status = 'active'",
    )
    .bind(tenant)
    .bind(user)
    .bind(role_to_db(role))
    .execute(&mut *tx)
    .await?;
    if seat {
        // The tenant row lock serializes allocations, so the limit holds under concurrency.
        sqlx::query("select 1 from kentos.tenant where id = $1 for update")
            .bind(tenant)
            .execute(&mut *tx)
            .await?;
        let (used, mine): (i64, bool) = sqlx::query_as(
            "select count(*), coalesce(bool_or(user_id = $2), false) from kentos.seat_allocation where tenant_id = $1",
        )
        .bind(tenant)
        .bind(user)
        .fetch_one(&mut *tx)
        .await?;
        if !mine {
            if used >= i64::from(limit) {
                return Err(AppError::invalid(format!(
                    "“{tenant_slug}” kurumunun {limit} koltuğunun hepsi dolu."
                )));
            }
            sqlx::query("insert into kentos.seat_allocation (tenant_id, user_id) values ($1, $2)")
                .bind(tenant)
                .bind(user)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Debug)]
pub struct TenantLine {
    pub slug: String,
    pub name: String,
    pub seats_used: i64,
    pub seat_limit: i32,
    pub members: i64,
}

pub async fn list_tenants(owner: &PgPool) -> AppResult<Vec<TenantLine>> {
    let rows: Vec<(String, String, i64, i32, i64)> = sqlx::query_as(
        "select t.slug, t.name,
                (select count(*) from kentos.seat_allocation s where s.tenant_id = t.id), t.seat_limit,
                (select count(*) from kentos.membership m where m.tenant_id = t.id)
           from kentos.tenant t order by t.slug",
    )
    .fetch_all(owner)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(slug, name, seats_used, seat_limit, members)| TenantLine {
            slug,
            name,
            seats_used,
            seat_limit,
            members,
        })
        .collect())
}

#[derive(Debug)]
pub struct MemberLine {
    pub login: Option<String>,
    pub name: String,
    pub role: TenantRole,
    pub seat: bool,
    pub status: String,
}

pub async fn list_members(owner: &PgPool, tenant_slug: &str) -> AppResult<Vec<MemberLine>> {
    let (tenant, _) = tenant_id(owner, tenant_slug).await?;
    let rows: Vec<(Option<String>, String, String, bool, String)> = sqlx::query_as(
        "select c.login, u.display_name, m.role,
                exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id),
                m.status
           from kentos.membership m join kentos.app_user u on u.id = m.user_id
           left join kentos.local_credential c on c.user_id = u.id
          where m.tenant_id = $1 order by u.display_name",
    )
    .bind(tenant)
    .fetch_all(owner)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(login, name, role, seat, status)| {
            Some(MemberLine {
                login,
                name,
                role: role_from_db(&role)?,
                seat,
                status,
            })
        })
        .collect())
}

/// A deleted project, for the operator (`kentosd project deleted`).
#[derive(Debug)]
pub struct DeletedProject {
    pub id: Uuid,
    pub name: String,
    pub deleted_at: time::OffsetDateTime,
    pub deleted_by: String,
}

pub async fn deleted_projects(owner: &PgPool, tenant_slug: &str) -> AppResult<Vec<DeletedProject>> {
    let (tenant, _) = tenant_id(owner, tenant_slug).await?;
    let rows: Vec<(Uuid, String, time::OffsetDateTime, String)> = sqlx::query_as(
        "select p.id, p.name, p.deleted_at, u.display_name
           from kentos.project p join kentos.app_user u on u.id = p.deleted_by
          where p.tenant_id = $1 and p.deleted_at is not null order by p.deleted_at desc",
    )
    .bind(tenant)
    .fetch_all(owner)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, deleted_at, deleted_by)| DeletedProject {
            id,
            name,
            deleted_at,
            deleted_by,
        })
        .collect())
}

/// Undoes a deletion (lifecycle.rs): the project is listed and opens again,
/// with everything it had. Recorded in the audit log without an actor (the
/// operator works from the command line). Returns the project's name.
pub async fn restore_project(
    owner: &PgPool,
    tenant_slug: &str,
    project: Uuid,
) -> AppResult<String> {
    let (tenant, _) = tenant_id(owner, tenant_slug).await?;
    let mut tx = owner.begin().await?;
    let name: Option<String> = sqlx::query_scalar(
        "update kentos.project set deleted_at = null, deleted_by = null, updated_at = now()
          where tenant_id = $1 and id = $2 and deleted_at is not null returning name",
    )
    .bind(tenant)
    .bind(project)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(name) = name else {
        return Err(AppError::not_found(format!(
            "“{tenant_slug}” kurumunda silinmiş {project} projesi yok; silinmiş projeleri `kentosd project deleted --tenant {tenant_slug}` listeler."
        )));
    };
    sqlx::query(
        "insert into kentos.audit_event (tenant_id, project_id, action, detail) values ($1, $2, 'project.restore', $3)",
    )
    .bind(tenant)
    .bind(project)
    .bind(serde_json::json!({ "name": name, "by": "kentosd" }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords_and_logins_are_checked_before_the_database() {
        assert!(check_password("kisa").is_err());
        assert!(check_password("yeterince-uzun").is_ok());
        assert!(check_password(&"ş".repeat(37)).is_err()); // 74 bytes: bcrypt would cut it
        assert!(check_login("ayse.yilmaz").is_ok());
        assert!(
            check_login("Ayse").is_err()
                && check_login("a b").is_err()
                && check_login("ab").is_err()
        );
    }
}
