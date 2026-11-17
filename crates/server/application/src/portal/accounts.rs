//! Who the signed-in person is on the portal, signing up as a professional, and one's own profile and password
//! (docs/adr/9000 §2). Signing up opens a local account and a professional profile waiting for an administrator's
//! check; until it is verified the person reads the portal but applies nowhere and is sent nothing.

use kentos_contracts::{
    OrganizationCategory, PasswordChange, PortalMe, PortalMembership, Profession, ProfessionalEdit,
    Signup, SignupInfo, UserView,
};
use kentos_postgres::{Db, Scope};
use uuid::Uuid;

use super::rows::{PROFESSIONAL_SELECT, ProfessionalRow};
use super::{answer, catalog, email_at, optional_at, text_at};
use crate::admin::{check_login, check_password};
use crate::error::AppResult;
use crate::identity::Actor;
use crate::tenancy::role_from_db;

pub(crate) fn user_scope(actor: &Actor) -> Scope {
    Scope {
        tenant: None,
        user: Some(actor.user_id),
        project: None,
    }
}

pub(crate) fn category_of(text: &str) -> OrganizationCategory {
    match text {
        "municipality" => OrganizationCategory::Municipality,
        "office" => OrganizationCategory::Office,
        _ => OrganizationCategory::Other,
    }
}

pub(crate) fn category_code(category: OrganizationCategory) -> &'static str {
    match category {
        OrganizationCategory::Municipality => "municipality",
        OrganizationCategory::Office => "office",
        OrganizationCategory::Other => "other",
    }
}

type MembershipRow = (
    Uuid,
    String,
    String,
    String,
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
);

/// `GET /v1/portal/me`.
pub async fn me(db: &Db, actor: &Actor) -> AppResult<PortalMe> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let platform_admin: bool = sqlx::query_scalar("select kentos.is_platform_admin()")
        .fetch_one(&mut *tx)
        .await?;
    let login: Option<String> = sqlx::query_scalar("select kentos.own_login()")
        .fetch_one(&mut *tx)
        .await?;
    let professional: Option<ProfessionalRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{PROFESSIONAL_SELECT} where p.user_id = $1"
    )))
    .bind(actor.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let rows: Vec<MembershipRow> = sqlx::query_as(
        "select t.id, t.slug, t.name, t.category, m.role, m.status, t.status,
                exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id),
                t.province, t.district
           from kentos.membership m join kentos.tenant t on t.id = m.tenant_id
          where m.user_id = $1 and t.kind = 'organization'
          order by t.name",
    )
    .bind(actor.user_id)
    .fetch_all(&mut *tx)
    .await?;
    let unread: i64 = sqlx::query_scalar(
        "select count(*) from kentos.notification where user_id = $1 and read_at is null",
    )
    .bind(actor.user_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let organizations = rows
        .into_iter()
        .filter_map(
            |(id, slug, name, category, role, status, tenant_status, seat, province, district)| {
                Some(PortalMembership {
                    tenant_id: id.to_string(),
                    tenant_slug: slug,
                    tenant_name: name,
                    category: category_of(&category),
                    role: role_from_db(&role)?,
                    usable: status == "active" && tenant_status == "active" && seat,
                    province,
                    district,
                })
            },
        )
        .collect();
    Ok(PortalMe {
        user: UserView {
            id: actor.user_id.to_string(),
            display_name: actor.display_name.clone(),
            email: actor.email.clone(),
            method: actor.method,
        },
        login,
        platform_admin,
        professional: professional.map(|p| p.view()),
        organizations,
        unread: u32::try_from(unread).unwrap_or(u32::MAX),
    })
}

/// `GET /v1/portal/signup`.
pub async fn signup_info(db: &Db) -> AppResult<SignupInfo> {
    let open: bool = sqlx::query_scalar("select signup_open from kentos.platform_setting")
        .fetch_one(&db.pool)
        .await?;
    Ok(SignupInfo { open })
}

/// A chamber registration number: letters, digits, spaces, `-` and `/`.
fn chamber_no(value: &str) -> AppResult<String> {
    let value = text_at("chamberNo", "Oda sicil numarası", value, 1, 40)?;
    if value
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '/' | '.'))
    {
        Ok(value)
    } else {
        Err(crate::AppError::invalid_at(
            "chamberNo",
            "Oda sicil numarasında yalnız harf, rakam, boşluk, - ve / bulunur.",
        ))
    }
}

/// A professional's own account and profile, as signing up gives them (or an administrator opens them, `by`).
/// Returns the new account.
pub async fn sign_up(db: &Db, input: &Signup, by: Option<&Actor>) -> AppResult<Uuid> {
    let login = input.login.trim().to_lowercase();
    check_login(&login).map_err(|e| e.at("login"))?;
    check_password(&input.password).map_err(|e| e.at("password"))?;
    let name = text_at("displayName", "Ad soyad", &input.display_name, 3, 120)?;
    let email = email_at("email", Some(&input.email), true)?;
    let chamber = chamber_no(&input.chamber_no)?;
    let office = optional_at("officeName", "Büro adı", input.office_name.as_deref(), 200)?;
    let phone = optional_at("phone", "Telefon", input.phone.as_deref(), 40)?;
    let province = optional_at("province", "İl", input.province.as_deref(), 60)?;
    let id = Uuid::now_v7();
    let mut tx = db
        .scoped(Scope {
            tenant: None,
            user: by.map(|a| a.user_id),
            project: None,
        })
        .await?;
    let code: Option<String> = sqlx::query_scalar(
        "select kentos.register_professional($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(id)
    .bind(&login)
    .bind(&input.password)
    .bind(&name)
    .bind(&email)
    .bind(catalog::profession_code(input.profession))
    .bind(&chamber)
    .bind(&office)
    .bind(&phone)
    .bind(&province)
    .fetch_one(&mut *tx)
    .await?;
    answer(code)?;
    tx.commit().await?;
    Ok(id)
}

/// `PUT /v1/portal/me/professional`: the parts of one's profile one changes oneself (the profession and registration
/// number are what an administrator checked, and stay as checked).
pub async fn edit_own(db: &Db, actor: &Actor, input: &ProfessionalEdit) -> AppResult<()> {
    let office = optional_at("officeName", "Büro adı", input.office_name.as_deref(), 200)?;
    let phone = optional_at("phone", "Telefon", input.phone.as_deref(), 40)?;
    let province = optional_at("province", "İl", input.province.as_deref(), 60)?;
    let mut tx = db.scoped(user_scope(actor)).await?;
    let done = sqlx::query(
        "update kentos.professional set office_name = $2, phone = $3, province = $4, listed = $5 where user_id = $1",
    )
    .bind(actor.user_id)
    .bind(&office)
    .bind(&phone)
    .bind(&province)
    .bind(input.listed)
    .execute(&mut *tx)
    .await?;
    if done.rows_affected() == 0 {
        return Err(crate::AppError::not_found("Meslek mensubu kaydınız yok."));
    }
    tx.commit().await?;
    Ok(())
}

/// `POST /v1/portal/me/password`: every session of the account closes; the caller is given a new one.
pub async fn change_password(db: &Db, actor: &Actor, input: &PasswordChange) -> AppResult<()> {
    check_password(&input.new).map_err(|e| e.at("new"))?;
    let mut tx = db.scoped(user_scope(actor)).await?;
    let code: Option<String> = sqlx::query_scalar("select kentos.change_own_password($1, $2)")
        .bind(&input.current)
        .bind(&input.new)
        .fetch_one(&mut *tx)
        .await?;
    answer(code)?;
    tx.commit().await?;
    Ok(())
}

/// The profession's name and chamber, for sentences.
pub fn profession_words(p: Profession) -> (&'static str, &'static str) {
    (catalog::profession_label(p), catalog::chamber(p))
}
