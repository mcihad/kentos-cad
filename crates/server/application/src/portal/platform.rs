//! The platform administrators' use cases (docs/adr/9000 §3): the platform at a glance, organisations (opened with
//! their first owner, seats, suspension), accounts (opened, closed, a new password), professionals (checked against
//! their chamber registration and verified, or not), signing up opened or closed, and the record of all of it. Every
//! change goes through a security-definer function that checks the caller and records the change (migration 0043).

use kentos_contracts::{
    MemberView, NewOwner, PlatformEventView, PlatformSettings, PlatformSummary, PlatformTenant,
    PlatformTenantCreate, PlatformTenantDetail, PlatformTenantEdit, PlatformUser,
    PlatformUserCreate, PlatformUserEdit, ProfessionalReview, ProfessionalStatus, ProfessionalView,
};
use kentos_postgres::Db;
use serde::Deserialize;
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use super::accounts::{category_code, category_of, user_scope};
use super::rows::{PROFESSIONAL_SELECT, ProfessionalRow};
use super::{answer, catalog, email_at, optional_at, rfc3339, text_at};
use crate::admin::{check_login, check_password};
use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::tenancy::role_from_db;

/// A transaction for a platform administrator, or the refusal.
async fn admin_tx(db: &Db, actor: &Actor) -> AppResult<Transaction<'static, Postgres>> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let admin: bool = sqlx::query_scalar("select kentos.is_platform_admin()")
        .fetch_one(&mut *tx)
        .await?;
    if !admin {
        return Err(AppError::forbidden(
            "Bu sayfa yalnız sistem yöneticilerine açık.",
        ));
    }
    Ok(tx)
}

fn count(n: i64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Figures {
    municipalities: i64,
    organisations: i64,
    suspended_organisations: i64,
    accounts: i64,
    disabled_accounts: i64,
    professionals: i64,
    pending_professionals: i64,
    seats_used: i64,
    seat_limit: i64,
    open_dossiers: i64,
    dossiers_this_month: i64,
    projects: i64,
    active_sessions: i64,
}

/// `GET /v1/platform/summary`.
pub async fn summary(db: &Db, actor: &Actor) -> AppResult<PlatformSummary> {
    let mut tx = admin_tx(db, actor).await?;
    let Json(f): Json<Figures> = sqlx::query_scalar("select kentos.platform_summary()")
        .fetch_one(&mut *tx)
        .await?;
    let signup_open: bool = sqlx::query_scalar("select signup_open from kentos.platform_setting")
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(PlatformSummary {
        municipalities: count(f.municipalities),
        organisations: count(f.organisations),
        suspended_organisations: count(f.suspended_organisations),
        accounts: count(f.accounts),
        disabled_accounts: count(f.disabled_accounts),
        professionals: count(f.professionals),
        pending_professionals: count(f.pending_professionals),
        seats_used: count(f.seats_used),
        seat_limit: count(f.seat_limit),
        open_dossiers: count(f.open_dossiers),
        dossiers_this_month: count(f.dossiers_this_month),
        projects: count(f.projects),
        active_sessions: count(f.active_sessions),
        signup_open,
    })
}

#[derive(sqlx::FromRow)]
struct TenantRow {
    id: Uuid,
    slug: String,
    name: String,
    category: String,
    province: Option<String>,
    district: Option<String>,
    status: String,
    seat_limit: i32,
    seats_used: i64,
    members: i64,
    projects: i64,
    dossiers: i64,
    accepts_applications: bool,
    created_at: OffsetDateTime,
}

impl TenantRow {
    fn view(self) -> PlatformTenant {
        PlatformTenant {
            id: self.id.to_string(),
            slug: self.slug,
            name: self.name,
            category: category_of(&self.category),
            province: self.province,
            district: self.district,
            status: self.status,
            seat_limit: u32::try_from(self.seat_limit).unwrap_or(0),
            seats_used: count(self.seats_used),
            members: count(self.members),
            projects: count(self.projects),
            dossiers: count(self.dossiers),
            accepts_applications: self.accepts_applications,
            created_at: rfc3339(self.created_at),
        }
    }
}

/// `GET /v1/platform/tenants`: every organisation, by name.
pub async fn tenants(db: &Db, actor: &Actor) -> AppResult<Vec<PlatformTenant>> {
    let mut tx = admin_tx(db, actor).await?;
    let rows: Vec<TenantRow> = sqlx::query_as("select * from kentos.platform_tenants()")
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(rows.into_iter().map(TenantRow::view).collect())
}

/// `GET /v1/platform/tenants/{id}`: one organisation and its members.
pub async fn tenant(db: &Db, actor: &Actor, id: Uuid) -> AppResult<PlatformTenantDetail> {
    let mut tx = admin_tx(db, actor).await?;
    let row: Option<TenantRow> =
        sqlx::query_as("select * from kentos.platform_tenants() where id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let tenant = row
        .ok_or_else(|| AppError::not_found("Kurum bulunamadı."))?
        .view();
    let members = members_of(&mut tx, id).await?;
    tx.commit().await?;
    Ok(PlatformTenantDetail { tenant, members })
}

type MemberRow = (Uuid, String, Option<String>, String, String, bool);

/// An organisation's members, owners first; for whoever row security lets see them (its admins, platform
/// administrators).
pub(crate) async fn members_of(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
) -> AppResult<Vec<MemberView>> {
    let rows: Vec<MemberRow> = sqlx::query_as(
        "select m.user_id, u.display_name, u.email, m.role, m.status,
                exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
           from kentos.membership m join kentos.app_user u on u.id = m.user_id
          where m.tenant_id = $1
          order by kentos.role_rank(m.role) desc, u.display_name",
    )
    .bind(tenant)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(user, name, email, role, status, seat)| {
            Some(MemberView {
                user_id: user.to_string(),
                display_name: name,
                email,
                role: role_from_db(&role)?,
                status,
                seat,
            })
        })
        .collect())
}

fn slug(value: &str) -> AppResult<String> {
    let s = value.trim().to_lowercase();
    let ok = (2..=63).contains(&s.len())
        && s.bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(s)
    } else {
        Err(AppError::invalid_at(
            "slug",
            "Kısa ad 2–63 karakter olmalı: küçük harf (Türkçe harf olmadan), rakam ve tire; harf ya da rakamla başlar.",
        ))
    }
}

fn seats(n: u32) -> AppResult<i32> {
    if n <= 100_000 {
        Ok(i32::try_from(n).unwrap_or(100_000))
    } else {
        Err(AppError::invalid_at(
            "seats",
            "Koltuk sayısı en çok 100 000 olabilir.",
        ))
    }
}

/// A new account through `kentos.platform_create_account`, in the open transaction.
async fn open_account(
    tx: &mut Transaction<'static, Postgres>,
    field: &str,
    login: &str,
    password: &str,
    name: &str,
    email: Option<&str>,
) -> AppResult<Uuid> {
    let login = login.trim().to_lowercase();
    check_login(&login).map_err(|e| e.at(format!("{field}login")))?;
    check_password(password).map_err(|e| e.at(format!("{field}password")))?;
    let name = text_at(&format!("{field}displayName"), "Ad soyad", name, 3, 120)?;
    let email = email_at(&format!("{field}email"), email, false)?;
    let id = Uuid::now_v7();
    let code: Option<String> =
        sqlx::query_scalar("select kentos.platform_create_account($1, $2, $3, $4, $5)")
            .bind(id)
            .bind(&login)
            .bind(password)
            .bind(&name)
            .bind(&email)
            .fetch_one(&mut **tx)
            .await?;
    answer(code).map_err(|e| match e.path() {
        Some("login") => e.at(format!("{field}login")),
        _ => e,
    })?;
    Ok(id)
}

/// `POST /v1/platform/tenants`: an organisation with its first owner (an account picked, or opened now).
pub async fn create_tenant(
    db: &Db,
    actor: &Actor,
    input: &PlatformTenantCreate,
) -> AppResult<Uuid> {
    let slug = slug(&input.slug)?;
    let name = text_at("name", "Kurum adı", &input.name, 2, 200)?;
    let province = optional_at("province", "İl", input.province.as_deref(), 60)?;
    let district = optional_at("district", "İlçe", input.district.as_deref(), 60)?;
    let seat_limit = seats(input.seats)?;
    let mut tx = admin_tx(db, actor).await?;
    let owner = match &input.owner {
        None => None,
        Some(NewOwner {
            user_id: Some(id), ..
        }) => Some(super::id_at("owner.userId", id)?),
        Some(NewOwner {
            login: Some(login),
            password,
            display_name,
            email,
            ..
        }) => Some(
            open_account(
                &mut tx,
                "owner.",
                login,
                password.as_deref().unwrap_or_default(),
                display_name.as_deref().unwrap_or_default(),
                email.as_deref(),
            )
            .await?,
        ),
        Some(_) => {
            return Err(AppError::invalid_at(
                "owner",
                "Kurumun ilk sahibi için bir hesap seçin ya da yeni hesabın giriş adını verin.",
            ));
        }
    };
    if owner.is_some() && seat_limit < 1 {
        return Err(AppError::invalid_at(
            "seats",
            "Kurumun sahibine bir koltuk gerekir: en az 1 koltuk verin.",
        ));
    }
    let id = Uuid::now_v7();
    let code: Option<String> =
        sqlx::query_scalar("select kentos.platform_create_tenant($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(id)
            .bind(&slug)
            .bind(&name)
            .bind(category_code(input.category))
            .bind(&province)
            .bind(&district)
            .bind(seat_limit)
            .bind(owner)
            .fetch_one(&mut *tx)
            .await?;
    answer(code)?;
    tx.commit().await?;
    Ok(id)
}

/// `PATCH /v1/platform/tenants/{id}`.
pub async fn update_tenant(
    db: &Db,
    actor: &Actor,
    id: Uuid,
    input: &PlatformTenantEdit,
) -> AppResult<()> {
    let name = text_at("name", "Kurum adı", &input.name, 2, 200)?;
    let province = optional_at("province", "İl", input.province.as_deref(), 60)?;
    let district = optional_at("district", "İlçe", input.district.as_deref(), 60)?;
    let seat_limit = seats(input.seats)?;
    if !matches!(input.status.as_str(), "active" | "suspended") {
        return Err(AppError::invalid_at(
            "status",
            "Durum “active” ya da “suspended” olmalı.",
        ));
    }
    let mut tx = admin_tx(db, actor).await?;
    let code: Option<String> =
        sqlx::query_scalar("select kentos.platform_update_tenant($1, $2, $3, $4, $5, $6, $7)")
            .bind(id)
            .bind(&name)
            .bind(category_code(input.category))
            .bind(&province)
            .bind(&district)
            .bind(seat_limit)
            .bind(&input.status)
            .fetch_one(&mut *tx)
            .await?;
    answer(code)?;
    tx.commit().await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    login: Option<String>,
    display_name: String,
    email: Option<String>,
    status: String,
    created_at: OffsetDateTime,
    signup: bool,
    platform_admin: bool,
    profession: Option<String>,
    professional_status: Option<String>,
    organisations: i64,
}

/// `GET /v1/platform/users?q=`: accounts, newest first, by name, e-mail or sign-in name.
pub async fn users(db: &Db, actor: &Actor, query: Option<&str>) -> AppResult<Vec<PlatformUser>> {
    let mut tx = admin_tx(db, actor).await?;
    let rows: Vec<UserRow> = sqlx::query_as("select * from kentos.platform_users($1, 200)")
        .bind(query.map(str::trim).unwrap_or_default())
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|r| PlatformUser {
            id: r.id.to_string(),
            login: r.login,
            display_name: r.display_name,
            email: r.email,
            status: r.status,
            created_at: rfc3339(r.created_at),
            signup: r.signup,
            platform_admin: r.platform_admin,
            profession: r.profession.as_deref().and_then(catalog::profession_of),
            professional_status: r
                .professional_status
                .as_deref()
                .and_then(catalog::professional_status_of),
            organizations: count(r.organisations),
        })
        .collect())
}

/// `POST /v1/platform/users`.
pub async fn create_user(db: &Db, actor: &Actor, input: &PlatformUserCreate) -> AppResult<Uuid> {
    let mut tx = admin_tx(db, actor).await?;
    let id = open_account(
        &mut tx,
        "",
        &input.login,
        &input.password,
        &input.display_name,
        input.email.as_deref(),
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// `PATCH /v1/platform/users/{id}`: closes or opens the account, gives it a new password; both close its sessions.
pub async fn edit_user(
    db: &Db,
    actor: &Actor,
    id: Uuid,
    input: &PlatformUserEdit,
) -> AppResult<()> {
    if let Some(p) = &input.password {
        check_password(p).map_err(|e| e.at("password"))?;
    }
    if let Some(s) = &input.status
        && !matches!(s.as_str(), "active" | "disabled")
    {
        return Err(AppError::invalid_at(
            "status",
            "Durum “active” ya da “disabled” olmalı.",
        ));
    }
    let mut tx = admin_tx(db, actor).await?;
    if let Some(status) = &input.status {
        let code: Option<String> =
            sqlx::query_scalar("select kentos.platform_set_user_status($1, $2)")
                .bind(id)
                .bind(status)
                .fetch_one(&mut *tx)
                .await?;
        answer(code)?;
    }
    if let Some(password) = &input.password {
        let code: Option<String> =
            sqlx::query_scalar("select kentos.platform_reset_password($1, $2)")
                .bind(id)
                .bind(password)
                .fetch_one(&mut *tx)
                .await?;
        answer(code)?;
    }
    tx.commit().await?;
    Ok(())
}

/// `GET /v1/platform/professionals?status=`: those waiting for a check first, then the newest.
pub async fn professionals(
    db: &Db,
    actor: &Actor,
    status: Option<ProfessionalStatus>,
) -> AppResult<Vec<ProfessionalView>> {
    let mut tx = admin_tx(db, actor).await?;
    let rows: Vec<ProfessionalRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{PROFESSIONAL_SELECT} where ($1::text is null or p.status = $1)
          order by case p.status when 'pending' then 0 else 1 end, p.created_at desc"
    )))
    .bind(status.map(catalog::professional_status_code))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows.iter().map(ProfessionalRow::view).collect())
}

/// `POST /v1/platform/professionals/{id}/review`. Turning one down or suspending says why.
pub async fn review(
    db: &Db,
    actor: &Actor,
    user: Uuid,
    input: &ProfessionalReview,
) -> AppResult<()> {
    let note = optional_at("note", "Açıklama", Some(&input.note), 1000)?;
    if matches!(
        input.status,
        ProfessionalStatus::Rejected | ProfessionalStatus::Suspended
    ) && note.is_none()
    {
        return Err(AppError::invalid_at(
            "note",
            "Kaydı onaylamamanın ya da askıya almanın nedenini yazın: kişi bildirimde bunu okur.",
        ));
    }
    let mut tx = admin_tx(db, actor).await?;
    let code: Option<String> =
        sqlx::query_scalar("select kentos.platform_review_professional($1, $2, $3)")
            .bind(user)
            .bind(catalog::professional_status_code(input.status))
            .bind(note.unwrap_or_default())
            .fetch_one(&mut *tx)
            .await?;
    answer(code)?;
    tx.commit().await?;
    Ok(())
}

type EventRow = (
    i64,
    OffsetDateTime,
    String,
    String,
    String,
    String,
    Json<serde_json::Value>,
);

/// What the record's line says.
fn event_text(action: &str, actor: &str, label: &str, detail: &serde_json::Value) -> String {
    let flag = |k: &str| {
        detail
            .get(k)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    };
    match action {
        "tenant.create" => format!("{actor}, “{label}” kurumunu açtı."),
        "tenant.update" => {
            let mut parts = Vec::new();
            if let Some(s) = detail.get("seats").and_then(serde_json::Value::as_i64) {
                parts.push(format!("koltuk {s}"));
            }
            match detail.get("status").and_then(serde_json::Value::as_str) {
                Some("suspended") => parts.push("askıya alındı".into()),
                Some("active") => parts.push("yeniden etkin".into()),
                _ => {}
            }
            if parts.is_empty() {
                format!("{actor}, “{label}” kurumunun bilgilerini değiştirdi.")
            } else {
                format!(
                    "{actor}, “{label}” kurumunu değiştirdi: {}.",
                    parts.join(", ")
                )
            }
        }
        "user.create" => format!("{actor}, “{label}” hesabını açtı."),
        "user.disable" => format!("{actor}, “{label}” hesabını kapattı."),
        "user.enable" => format!("{actor}, “{label}” hesabını yeniden açtı."),
        "user.password" => format!("{actor}, “{label}” hesabına yeni parola verdi."),
        "user.platform_admin" => format!("“{label}” sistem yöneticisi yapıldı ({actor})."),
        "user.platform_admin_removed" => {
            format!("“{label}” artık sistem yöneticisi değil ({actor}).")
        }
        "professional.register" if flag("byAdmin") => {
            format!("{actor}, “{label}” için meslek mensubu kaydı açtı.")
        }
        "professional.register" => format!("“{label}” meslek mensubu olarak kayıt oldu."),
        "professional.verified" => format!("{actor}, “{label}” kaydını doğruladı."),
        "professional.rejected" => format!("{actor}, “{label}” kaydını onaylamadı."),
        "professional.suspended" => format!("{actor}, “{label}” kaydını askıya aldı."),
        "professional.pending" => format!("{actor}, “{label}” kaydını yeniden incelemeye aldı."),
        "setting.signup" if flag("open") => format!("{actor}, meslek mensubu kaydını açtı."),
        "setting.signup" => format!("{actor}, meslek mensubu kaydını kapattı."),
        "file.refused" => {
            let why = detail.get("reason").and_then(serde_json::Value::as_str).unwrap_or("türü alınmıyor");
            if detail.get("verdict").and_then(serde_json::Value::as_str) == Some("infected") {
                format!("{actor}, zararlı yazılım taşıyan “{label}” dosyasını gönderdi; dosya alınmadı ({why}).")
            } else {
                format!("{actor}, “{label}” dosyasını gönderdi; alınmadı: {why}")
            }
        }
        other => format!("{actor}: {other} ({label})"),
    }
}

/// `GET /v1/platform/events`: the newest of the record first.
pub async fn events(db: &Db, actor: &Actor, limit: u32) -> AppResult<Vec<PlatformEventView>> {
    let mut tx = admin_tx(db, actor).await?;
    let rows: Vec<EventRow> = sqlx::query_as(
        "select id, at, actor_name, action, target_kind, target_label, detail
           from kentos.platform_event order by id desc limit $1",
    )
    .bind(i64::from(limit.clamp(1, 500)))
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, at, actor_name, action, target_kind, target_label, Json(detail))| {
                PlatformEventView {
                    id: id.to_string(),
                    at: rfc3339(at),
                    text: event_text(&action, &actor_name, &target_label, &detail),
                    actor_name,
                    action,
                    target_kind,
                    target_label,
                }
            },
        )
        .collect())
}

/// `GET /v1/platform/settings`.
pub async fn settings(db: &Db, actor: &Actor) -> AppResult<PlatformSettings> {
    let mut tx = admin_tx(db, actor).await?;
    let signup_open: bool = sqlx::query_scalar("select signup_open from kentos.platform_setting")
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(PlatformSettings { signup_open })
}

/// `PUT /v1/platform/settings`.
pub async fn set_settings(db: &Db, actor: &Actor, input: &PlatformSettings) -> AppResult<()> {
    let mut tx = admin_tx(db, actor).await?;
    let code: Option<String> = sqlx::query_scalar("select kentos.platform_set_signup($1)")
        .bind(input.signup_open)
        .fetch_one(&mut *tx)
        .await?;
    answer(code)?;
    tx.commit().await?;
    Ok(())
}
