//! The people of a project's share dialog (docs/adr/0015, TODOS.md
//! CLOUD-12, CLOUD-16, CLOUD-21): who has access to it and why ([`list`]),
//! and whom it can be shared with ([`candidates`]). Both need
//! `project.share` and start from the caller's [`ProjectAccess`], so a
//! project the caller may not see answers 404 before anything is read.
//!
//! - The list shows everyone with a role or a grant: the owner, the
//!   organisation's owners and admins its policy lets in, and every grant,
//!   ended ones included. Each person's role now is worked out by
//!   [`standing`], the rules of `kentos.project_role` (migration 0004) that
//!   the server applies when that person opens the project; `tests/people.rs`
//!   keeps the two equal for every kind of person.
//! - A search finds only people the caller may see and share with: an
//!   organisation project's active members, or, for a project of a personal
//!   space, the active members of the caller's own organisations. It never
//!   reaches another organisation or an account the caller shares nothing
//!   with (finding anyone by e-mail comes with invitations, CLOUD-16/17).
//!   Row-level security scopes every query to the tenant it reads.

use std::collections::BTreeMap;

use kentos_contracts::{
    AccessBlock, AccessSource, GrantRole, ProjectAccessHolder, ProjectAccessList,
    ProjectPermission, ProjectRole, ShareCandidate, ShareCandidates, TenantKind,
};
use kentos_postgres::{Db, Scope};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::access::{ProjectAccess, not_found};
use crate::error::{AppError, AppResult};
use crate::projects::rfc3339;
use crate::tenancy;

/// Most people one search returns.
pub const CANDIDATES_MAX: i64 = 20;

/// What the database says about one person listed with a project.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub user: Uuid,
    /// The account is active (a disabled one cannot sign in).
    pub account_active: bool,
    /// Their role in the project's organisation (`owner` … `viewer`), if a member.
    pub member_role: Option<String>,
    /// The membership is active (not disabled or only invited).
    pub member_active: bool,
    pub seat: bool,
    pub grant: Option<GrantRole>,
    /// The grant's end has passed.
    pub expired: bool,
    /// The grant is a guest's: given to someone outside the organisation by
    /// an accepted invitation (docs/adr/0035).
    pub guest: bool,
}

/// A person's role in a project now and where it comes from, or why they
/// cannot use it: the rules of `kentos.project_role`, in its order.
///
/// - A personal space: its owner owns every project in it; anyone else
///   needs an unexpired grant, no membership.
/// - An organisation: an active membership with a seat first; then
///   ownership, the policy (owners and admins, a manager who may delete),
///   an unexpired grant. Someone who is not a member at all works with a
///   guest's grant while the organisation takes guests (`guests`).
///
/// `owner` is the personal space's person, or the organisation project's
/// owner. A disabled account cannot sign in, so it is listed as inactive
/// whatever it holds.
pub fn standing(
    kind: TenantKind,
    policy: bool,
    guests: bool,
    owner: Uuid,
    f: &Facts,
) -> Result<(ProjectRole, AccessSource), AccessBlock> {
    let granted = || match f.grant {
        Some(role) if !f.expired => Ok((ProjectRole::from(role), AccessSource::Grant)),
        Some(_) => Err(AccessBlock::Expired),
        None => Err(AccessBlock::NotMember),
    };
    if kind == TenantKind::Organization {
        match f.member_role {
            None if f.guest && f.grant.is_some() => {
                if !guests {
                    return Err(AccessBlock::GuestsOff);
                }
                if !f.account_active {
                    return Err(AccessBlock::Inactive);
                }
                return granted();
            }
            None => return Err(AccessBlock::NotMember),
            Some(_) if !f.member_active => return Err(AccessBlock::Inactive),
            Some(_) if !f.seat => return Err(AccessBlock::NoSeat),
            Some(_) => {}
        }
    }
    if !f.account_active {
        return Err(AccessBlock::Inactive);
    }
    if f.user == owner {
        return Ok((ProjectRole::Owner, AccessSource::Owner));
    }
    if kind == TenantKind::Organization
        && policy
        && matches!(f.member_role.as_deref(), Some("owner" | "admin"))
    {
        return Ok((ProjectRole::Manager, AccessSource::Policy));
    }
    granted()
}

type PersonRow = (
    Uuid,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
    Option<String>,
    Option<OffsetDateTime>,
    bool,
    bool,
);

/// Everyone with a role or a grant in the project in scope: the owner, the
/// grantees and (`$3`: an organisation whose policy is on) its owners and
/// admins, with their account, membership, seat and grant. Row-level
/// security shows the members of the tenant and the grantees of the project;
/// an owner who left the organisation comes without a name.
const PEOPLE_SELECT: &str = "with people as (
        select owner_user_id as user_id from kentos.project where tenant_id = $1 and id = $2
        union select user_id from kentos.project_grant where tenant_id = $1 and project_id = $2
        union select user_id from kentos.membership where $3 and tenant_id = $1 and role in ('owner', 'admin'))
     select x.user_id, u.display_name, u.email, u.status, m.role, m.status,
            exists (select 1 from kentos.seat_allocation s where s.tenant_id = $1 and s.user_id = x.user_id),
            g.role, g.expires_at, coalesce(g.expires_at <= now(), false), coalesce(g.guest, false)
       from people x
       left join kentos.app_user u on u.id = x.user_id
       left join kentos.membership m on m.tenant_id = $1 and m.user_id = x.user_id
       left join kentos.project_grant g on g.tenant_id = $1 and g.project_id = $2 and g.user_id = x.user_id";

/// Who may use a project and why, for those who may share it (a deleted one answers 410).
pub async fn list(db: &Db, access: &ProjectAccess) -> AppResult<ProjectAccessList> {
    access.live()?;
    access.require(ProjectPermission::Share)?;
    let mut tx = db.scoped(access.scope()).await?;
    let project: Option<(Uuid, Option<String>, String)> = sqlx::query_as(
        "select p.owner_user_id, u.display_name, p.storage from kentos.project p left join kentos.app_user u on u.id = p.owner_user_id
          where p.tenant_id = $1 and p.id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_optional(&mut *tx)
    .await?;
    let (owner_id, owner_name, storage) = project.ok_or_else(not_found)?;
    let (space_owner, admins_policy, guests): (Option<Uuid>, bool, bool) = sqlx::query_as(
        "select owner_user_id, admins_access_all_projects, allow_guests from kentos.tenant where id = $1",
    )
    .bind(access.tenant)
    .fetch_one(&mut *tx)
    .await?;
    let kind = access.tenant_kind;
    let policy = kind == TenantKind::Organization && admins_policy;
    let rows: Vec<PersonRow> = sqlx::query_as(PEOPLE_SELECT)
        .bind(access.tenant)
        .bind(access.project)
        .bind(policy)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    // In a personal space its person owns every project in it (`kentos.project_role`).
    let owner = match kind {
        TenantKind::Personal => space_owner.unwrap_or(owner_id),
        TenantKind::Organization => owner_id,
    };
    let mut people: Vec<ProjectAccessHolder> = rows
        .into_iter()
        .filter_map(
            |(
                user,
                name,
                email,
                account,
                member_role,
                member_status,
                seat,
                grant,
                expires,
                expired,
                guest,
            )| {
                let facts = Facts {
                    user,
                    account_active: account.as_deref() == Some("active"),
                    member_active: member_status.as_deref() == Some("active"),
                    member_role,
                    seat,
                    grant: grant.as_deref().and_then(GrantRole::from_name),
                    expired,
                    guest,
                };
                let now = standing(kind, policy, guests, owner, &facts);
                // Listed only for the policy, which does not apply to them now: not one of the project's people.
                if now.is_err() && user != owner && facts.grant.is_none() {
                    return None;
                }
                Some(ProjectAccessHolder {
                    user_id: user.to_string(),
                    display_name: name.unwrap_or_default(),
                    email,
                    role: now.ok().map(|(r, _)| r),
                    via: now.ok().map(|(_, v)| v),
                    blocked: now.err(),
                    grant: facts.grant,
                    expires_at: expires.map(rfc3339),
                    expired,
                    guest: guest && facts.grant.is_some(),
                })
            },
        )
        .collect();
    let owner_text = owner_id.to_string();
    people.sort_by_cached_key(|p| {
        (
            p.user_id != owner_text,
            fold(&p.display_name),
            p.user_id.clone(),
        )
    });
    Ok(ProjectAccessList {
        tenant_kind: kind,
        storage: crate::projects::storage_of(&storage),
        admins_access_all_projects: policy,
        owner_id: owner_text,
        owner_name: owner_name.unwrap_or_default(),
        people,
    })
}

/// Turkish letters and their plain forms, for searching names and e-mails:
/// "ayse" finds "Ayşe", "isik" finds "IŞIK". The database folds its side with
/// the same two strings (`translate`); both then lowercase ASCII only, so a
/// search does not depend on the database's locale.
pub(crate) const FOLD_FROM: &str = "ÇĞİIÖŞÜÂÎÛçğıöşüâîû";
pub(crate) const FOLD_TO: &str = "cgiiosuaiucgiosuaiu";

/// `text` as a search compares it (see [`FOLD_FROM`]).
pub fn fold(text: &str) -> String {
    text.chars()
        .map(|c| {
            FOLD_FROM
                .chars()
                .position(|f| f == c)
                .and_then(|i| FOLD_TO.chars().nth(i))
                .unwrap_or(c)
                .to_ascii_lowercase()
        })
        .collect()
}

/// The words of a search as LIKE patterns (`%word%`, with `%`, `_` and `\`
/// taken literally). At least two letters, at most 100 characters.
pub fn search_patterns(query: &str) -> AppResult<Vec<String>> {
    let words: Vec<String> = query.split_whitespace().map(fold).collect();
    let letters: usize = words.iter().map(|w| w.chars().count()).sum();
    if letters < 2 || query.chars().count() > 100 {
        return Err(AppError::invalid_at(
            "q",
            "Aramak için kişinin adından ya da e-postasından en az iki harf yazın (en çok 100 karakter).",
        ));
    }
    Ok(words
        .into_iter()
        .map(|w| {
            format!(
                "%{}%",
                w.replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        })
        .collect())
}

/// Active members of one tenant who match every pattern, but not `skip`.
const MEMBER_SEARCH: &str = "select u.id, u.display_name, u.email
   from kentos.membership m join kentos.app_user u on u.id = m.user_id
  where m.tenant_id = $1 and m.status = 'active' and u.status = 'active' and u.id <> all($2)
    and lower(translate(u.display_name || ' ' || coalesce(u.email, ''), $3, $4)) like all ($5::text[])
  order by lower(translate(u.display_name, $3, $4)), u.id
  limit $6";

/// People the caller may share the project with whose name or e-mail holds
/// every word of `query` (see the module's notes): at most
/// [`CANDIDATES_MAX`], by name. The caller and the owner are left out;
/// people who already have a grant are not (sharing again changes the role).
pub async fn candidates(
    db: &Db,
    access: &ProjectAccess,
    query: &str,
) -> AppResult<ShareCandidates> {
    access.live()?;
    access.require(ProjectPermission::Share)?;
    let patterns = search_patterns(query)?;
    let mut tx = db.scoped(access.scope()).await?;
    let owner: Uuid = sqlx::query_scalar(
        "select owner_user_id from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(not_found)?;
    tx.commit().await?;
    // Where to look: the project's organisation, or the caller's own organisations they may use now.
    let tenants: Vec<Uuid> = match access.tenant_kind {
        TenantKind::Organization => vec![access.tenant],
        TenantKind::Personal => tenancy::memberships(db, &access.actor)
            .await?
            .into_iter()
            .filter(|m| m.tenant_kind == TenantKind::Organization && m.active && m.seat)
            .filter_map(|m| Uuid::parse_str(&m.tenant_id).ok())
            .collect(),
    };
    let skip = [access.actor.user_id, owner];
    let mut found: BTreeMap<Uuid, ShareCandidate> = BTreeMap::new();
    for tenant in tenants {
        let mut tx = db
            .scoped(Scope {
                tenant: Some(tenant),
                user: Some(access.actor.user_id),
                project: None,
            })
            .await?;
        let rows: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(MEMBER_SEARCH)
            .bind(tenant)
            .bind(&skip[..])
            .bind(FOLD_FROM)
            .bind(FOLD_TO)
            .bind(&patterns)
            .bind(CANDIDATES_MAX)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;
        for (id, name, email) in rows {
            found.entry(id).or_insert(ShareCandidate {
                user_id: id.to_string(),
                display_name: name,
                email,
            });
        }
    }
    let mut candidates: Vec<ShareCandidate> = found.into_values().collect();
    candidates.sort_by_cached_key(|c| (fold(&c.display_name), c.user_id.clone()));
    candidates.truncate(CANDIDATES_MAX as usize);
    Ok(ShareCandidates { candidates })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(user: Uuid, role: &str) -> Facts {
        Facts {
            user,
            account_active: true,
            member_role: Some(role.into()),
            member_active: true,
            seat: true,
            grant: None,
            expired: false,
            guest: false,
        }
    }

    #[test]
    fn standing_follows_the_order_of_the_database_function() {
        let owner = Uuid::from_u128(1);
        let org = TenantKind::Organization;
        let person = Uuid::from_u128(2);
        // The owner, the policy's admin, a grant; the highest counts.
        assert_eq!(
            standing(org, true, true, owner, &member(owner, "project_manager")),
            Ok((ProjectRole::Owner, AccessSource::Owner))
        );
        let admin = Facts {
            grant: Some(GrantRole::Viewer),
            ..member(person, "admin")
        };
        assert_eq!(
            standing(org, true, true, owner, &admin),
            Ok((ProjectRole::Manager, AccessSource::Policy))
        );
        // The policy off: the admin works with the grant.
        assert_eq!(
            standing(org, false, true, owner, &admin),
            Ok((ProjectRole::Viewer, AccessSource::Grant))
        );
        // Membership, seat and account come before everything, the owner's too.
        for (facts, block) in [
            (
                Facts {
                    member_role: None,
                    ..member(owner, "owner")
                },
                AccessBlock::NotMember,
            ),
            (
                Facts {
                    member_active: false,
                    ..member(owner, "owner")
                },
                AccessBlock::Inactive,
            ),
            (
                Facts {
                    seat: false,
                    ..member(owner, "owner")
                },
                AccessBlock::NoSeat,
            ),
            (
                Facts {
                    account_active: false,
                    ..member(owner, "owner")
                },
                AccessBlock::Inactive,
            ),
        ] {
            assert_eq!(standing(org, true, true, owner, &facts), Err(block));
        }
        // An ended grant does not count.
        let ended = Facts {
            grant: Some(GrantRole::Editor),
            expired: true,
            ..member(person, "editor")
        };
        assert_eq!(
            standing(org, true, true, owner, &ended),
            Err(AccessBlock::Expired)
        );
        // A personal space: no membership needed, the space's person owns it.
        let guest = Facts {
            member_role: None,
            member_active: false,
            seat: false,
            grant: Some(GrantRole::Commenter),
            ..member(person, "")
        };
        assert_eq!(
            standing(TenantKind::Personal, false, true, owner, &guest),
            Ok((ProjectRole::Commenter, AccessSource::Grant))
        );
        assert_eq!(
            standing(
                TenantKind::Personal,
                false,
                true,
                owner,
                &member(owner, "owner")
            ),
            Ok((ProjectRole::Owner, AccessSource::Owner))
        );
        // Outside an organisation (docs/adr/0035): a guest's grant works while it takes guests;
        // a plain grant does not; nor does a guest's for a member whose membership is gone.
        let outsider = Facts {
            guest: true,
            ..guest.clone()
        };
        assert_eq!(
            standing(org, true, true, owner, &outsider),
            Ok((ProjectRole::Commenter, AccessSource::Grant))
        );
        assert_eq!(
            standing(org, true, false, owner, &outsider),
            Err(AccessBlock::GuestsOff)
        );
        assert_eq!(
            standing(org, true, true, owner, &guest),
            Err(AccessBlock::NotMember)
        );
        let left = Facts {
            member_role: Some("editor".into()),
            ..outsider.clone()
        };
        assert_eq!(
            standing(org, true, true, owner, &left),
            Err(AccessBlock::Inactive)
        );
        let ended_guest = Facts {
            expired: true,
            ..outsider
        };
        assert_eq!(
            standing(org, true, true, owner, &ended_guest),
            Err(AccessBlock::Expired)
        );
    }

    #[test]
    fn searches_fold_turkish_letters_and_take_wildcards_literally() {
        assert_eq!(
            fold("Ayşe IŞIK İpek Çağrı Öz Ünal"),
            "ayse isik ipek cagri oz unal"
        );
        assert_eq!(fold("ÂLİ"), "ali");
        assert_eq!(
            search_patterns("  Mehmet  dem ").unwrap(),
            ["%mehmet%", "%dem%"]
        );
        assert_eq!(search_patterns("a_b%").unwrap(), ["%a\\_b\\%%"]);
        assert_eq!(search_patterns("c\\").unwrap(), ["%c\\\\%"]);
        for short in ["", " ", "a", " ş "] {
            assert!(
                matches!(search_patterns(short), Err(AppError::Invalid { .. })),
                "{short:?}"
            );
        }
        assert!(search_patterns(&"x".repeat(101)).is_err());
        // The two fold tables pair up letter by letter.
        assert_eq!(FOLD_FROM.chars().count(), FOLD_TO.chars().count());
    }
}
