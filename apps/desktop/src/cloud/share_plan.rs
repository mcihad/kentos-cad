//! What the Paylaş window says and offers, apart from its drawing (the web's
//! app/cloud/sharePlan.ts with the rules of sharing.ts and invitations.ts;
//! docs/adr/0111): the roles and where a person's access comes from, the
//! rows of Kişiler, the finder's words, the invitations' order, lines and
//! questions, the product commands' inputs and how a failure is said.
//! fixtures/cloud/v1/share.json holds every answer; both platforms play it
//! (`share_plan_tests.rs`). Dates are the device's local time.

use kentos_cloud::ApiFailure;
#[cfg(test)]
use kentos_contracts::InvitationAccepted;
use kentos_contracts::{
    AccessBlock, AccessSource, GrantRole, InvitationChange, InvitationState, ProjectAccessHolder,
    ProjectAccessList, ProjectInvitation, ProjectRole, ProjectStorage, TenantKind,
};
use kentos_expression::js::collate::compare_tr;
use kentos_expression::js::text::upper_tr;
use serde_json::{Value, json};

use super::local_time::{self, Zone};
use super::words::epoch;

// ── The window's words (the web's `SHARE_TEXTS`) ─────────────────────────

pub(crate) const TITLE: &str = "Projeyi paylaş";
pub(crate) const TAB_PEOPLE: &str = "Kişiler";
pub(crate) const TAB_INVITES: &str = "Davetler";
pub(crate) const CLOSE: &str = "Kapat";

/// Kişiler's words.
pub(crate) mod people {
    pub(crate) const ADD: &str = "Kişi ekle";
    pub(crate) const ROLE: &str = "Rol";
    pub(crate) const UNTIL: &str = "Bitiş (isteğe bağlı)";
    pub(crate) const SHARE: &str = "Paylaş";
    pub(crate) const TITLE: &str = "Erişimi olanlar";
    pub(crate) const LOADING: &str = "Erişimi olanlar yükleniyor…";
    pub(crate) const YOU: &str = " (siz)";
    pub(crate) const REMOVE: &str = "Kaldır";
    pub(crate) const BAD_DATE: &str =
        "Bitiş tarihi okunamadı: takvimden bir gün seçin ya da alanı boş bırakın.";
    pub(crate) const READ_FAILED: &str = "Erişim listesi okunamadı";
    pub(crate) const SHARE_FAILED: &str = "Paylaşılamadı";
    pub(crate) const CHANGE_FAILED: &str = "Rol değiştirilemedi";
    pub(crate) const REVOKE_FAILED: &str = "Erişim kaldırılamadı";
    pub(crate) const GUEST_ROLE: &str = "Misafirin rolü davetle verilir. Değiştirmek için erişimini kaldırıp yeni rolle yeniden davet edin.";

    pub(crate) fn remove_label(name: &str) -> String {
        format!("{name} erişimini kaldır")
    }
    /// The web's name for a row's role field (its screen reader's); the
    /// desktop has none, the fixture keeps the words in step.
    #[cfg(test)]
    pub(crate) fn role_label(name: &str) -> String {
        format!("{name} için rol")
    }
    pub(crate) fn count(n: usize) -> String {
        format!("{n} kişi erişebiliyor")
    }
    pub(crate) fn adding(name: &str) -> String {
        format!("{name} ekleniyor…")
    }
    pub(crate) fn changing(name: &str) -> String {
        format!("{name} için rol değiştiriliyor…")
    }
    pub(crate) fn revoking(name: &str) -> String {
        format!("{name} için erişim kaldırılıyor…")
    }
    pub(crate) fn storage(title: &str) -> String {
        format!("Saklama: {title}. ")
    }
}

/// Kişi ekle's finder.
pub(crate) mod find {
    pub(crate) const PLACEHOLDER: &str = "Ad ya da e-posta yazın";
    pub(crate) const FAILED: &str = "Kişi aranamadı";

    pub(crate) fn has(role: &str) -> String {
        format!("şu an {role}")
    }
}

/// Davetler's words.
pub(crate) mod invites {
    pub(crate) const EMAIL: &str = "E-postayla davet et";
    pub(crate) const PLACEHOLDER: &str = "ad@kurum.gov.tr";
    pub(crate) const ROLE: &str = "Rol";
    pub(crate) const WAIT: &str = "Geçerlilik";
    pub(crate) const SEND: &str = "Davet et";
    pub(crate) const TITLE: &str = "Davetler";
    pub(crate) const LOADING: &str = "Davetler yükleniyor…";
    pub(crate) const EMPTY: &str = "Bekleyen ya da son 30 günde sonuçlanmış davet yok. Kurum dışından biriyle çalışmak için yukarıdan e-postayla davet edin.";
    pub(crate) const NO_RIGHT: &str =
        "Davetleri görmek ve göndermek için bu projede paylaşım yetkiniz olmalı (project.share).";
    pub(crate) const REVOKE: &str = "Geri al";
    pub(crate) const CREATED: &str =
        "Davet oluşturuldu. Bağlantıyı kopyalayıp davet ettiğiniz kişiye iletin.";
    pub(crate) const READ_FAILED: &str = "Davetler okunamadı";
    pub(crate) const SEND_FAILED: &str = "Davet gönderilemedi";
    pub(crate) const REVOKE_FAILED: &str = "Davet geri alınamadı";
    pub(crate) const LINK_LABEL: &str = "Davet bağlantısı";
    pub(crate) const COPY: &str = "Kopyala";
    pub(crate) const COPIED: &str = "Kopyalandı";
    pub(crate) const COPIED_SAY: &str =
        "Bağlantı panoya kopyalandı; davet ettiğiniz kişiye iletin.";
    pub(crate) const NO_LINK: &str =
        "Ancak bağlantı bu yanıtta yok; sunucu onu yalnız ilk yanıtta verir.";
    pub(crate) const NO_LINK_WARN: &str = "Bağlantıyı almak için aynı adrese yeniden davet gönderin; bu davetin bağlantısı artık çalışmaz.";

    pub(crate) fn count(waiting: usize) -> String {
        format!("{waiting} bekliyor")
    }
    pub(crate) fn revoke_label(email: &str) -> String {
        format!("{email} davetini geri al")
    }
    pub(crate) fn inviting(email: &str) -> String {
        format!("“{email}” davet ediliyor…")
    }
    pub(crate) fn revoking(email: &str) -> String {
        format!("“{email}” için davet geri alınıyor…")
    }
}

// ── Roles, blocks, storage (sharing.ts) ──────────────────────────────────

/// A role's name.
pub(crate) fn role_label(role: ProjectRole) -> &'static str {
    match role {
        ProjectRole::Owner => "Sahip",
        ProjectRole::Manager => "Yönetici",
        ProjectRole::Editor => "Düzenleyici",
        ProjectRole::Commenter => "Yorumcu",
        ProjectRole::Viewer => "Görüntüleyici",
    }
}

/// A grant's role name.
pub(crate) fn grant_label(role: GrantRole) -> &'static str {
    role_label(project_role(role))
}

fn project_role(role: GrantRole) -> ProjectRole {
    match role {
        GrantRole::Manager => ProjectRole::Manager,
        GrantRole::Editor => ProjectRole::Editor,
        GrantRole::Commenter => ProjectRole::Commenter,
        GrantRole::Viewer => ProjectRole::Viewer,
    }
}

/// The roles a share gives, weakest first (ownership is transferred, not shared).
pub(crate) const GRANT_ROLES: [GrantRole; 4] = [
    GrantRole::Viewer,
    GrantRole::Commenter,
    GrantRole::Editor,
    GrantRole::Manager,
];

/// The roles an invitation gives: managing and sharing stay with members.
pub(crate) const INVITE_ROLES: [GrantRole; 3] =
    [GrantRole::Viewer, GrantRole::Commenter, GrantRole::Editor];

/// What each role may do, in one line (docs/adr/0015's table).
pub(crate) fn role_hint(role: GrantRole) -> &'static str {
    match role {
        GrantRole::Viewer => "Projeyi açar, görür, indirir ve geçmişine bakar; değiştiremez.",
        GrantRole::Commenter => "Görüntüleyicinin yapabildikleri ve yorum.",
        GrantRole::Editor => "Nesneleri çizer, değiştirir ve siler.",
        GrantRole::Manager => {
            "Düzenleyicinin yapabildikleri; ad, ayar ve katmanları değiştirir, projeyi paylaşır."
        }
    }
}

/// Why someone cannot use the project.
pub(crate) fn block_text(block: AccessBlock) -> &'static str {
    match block {
        AccessBlock::Expired => "Paylaşımın süresi doldu",
        AccessBlock::NotMember => "Kurumun üyesi değil",
        AccessBlock::Inactive => "Hesabı ya da kurum üyeliği etkin değil",
        AccessBlock::NoSeat => "Kurumda koltuğu yok",
        AccessBlock::GuestsOff => "Misafir; kurum dışarıdan misafir kabul etmiyor",
    }
}

/// Where a project keeps its content: its name and what that means.
pub(crate) fn storage_text(storage: ProjectStorage) -> (&'static str, &'static str) {
    match storage {
        ProjectStorage::Database => (
            "Yönetilen PostGIS veritabanı",
            "Nesneler sunucudaki veritabanında tek tek saklanır; her kayıt tek işlemde yazılır ve erişimi olan herkes hemen görür. Paylaşım alıcıya veritabanı hesabı ya da parolası vermez.",
        ),
        ProjectStorage::File => (
            "Dosya (KCAD revizyonları)",
            "Proje sunucuda değişmez .kcad revizyonları olarak saklanır; her kayıt yeni bir revizyondur ve dayandığı revizyonla karşılaştırılır, arada başkası kaydettiyse üzerine yazılmaz. Paylaşım alıcıya dosya deposuna ayrı bir erişim vermez.",
        ),
    }
}

/// A date as the interface writes it, in the device's time: “31.12.2026”.
pub(crate) fn date_text(iso: &str, zone: &Zone) -> String {
    local_time::day(iso, zone).unwrap_or_default()
}

/// The end of a day picked for a share's end (`YYYY-MM-DD`): 23:59:59 in
/// the device's time, as the server's RFC 3339 (“2026-12-31T20:59:59.000Z”);
/// none for nonsense or a day its month does not have (30 February).
pub(crate) fn end_of_day(date: &str, zone: &Zone) -> Option<String> {
    let b = date.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> {
        let part = date.get(from..to)?;
        part.bytes()
            .all(|d| d.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=days).contains(&day) {
        return None;
    }
    let local = local_time::seconds_of(year, month, day) + 86_399;
    Some(local_time::iso_utc(local_time::from_local(local, zone)))
}

/// Where a person's access comes from, or why they cannot use the project, in one line.
pub(crate) fn source_text(p: &ProjectAccessHolder, zone: &Zone) -> String {
    let grant = p.grant.map_or(String::new(), |g| {
        let until = p.expires_at.as_deref().map_or(String::new(), |e| {
            format!(", {} tarihine kadar", date_text(e, zone))
        });
        format!("paylaşım: {}{until}", grant_label(g))
    });
    if let Some(block) = p.blocked {
        return [block_text(block).to_owned(), grant]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
    }
    match p.via {
        Some(AccessSource::Owner) => "Proje sahibi".to_owned(),
        Some(AccessSource::Policy) => ["Kurum politikası: kurum yöneticisi".to_owned(), grant]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
        _ => {
            // A guest: outside the organisation, through an accepted invitation (docs/adr/0035).
            let kind = if p.guest {
                "Misafir (davetle)"
            } else {
                "Paylaşım"
            };
            match &p.expires_at {
                Some(e) => format!("{kind}, {} tarihine kadar", date_text(e, zone)),
                None => kind.to_owned(),
            }
        }
    }
}

/// One person's row in Kişiler: sharing.ts's `PersonRow` with the view's
/// letters, second line, the role's note and the lock's reason.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PersonRow {
    pub user_id: String,
    pub name: String,
    pub email: Option<String>,
    /// The role now, or “Erişemiyor”.
    pub role_label: String,
    /// Where it comes from, or why not.
    pub source: String,
    pub blocked: bool,
    /// The signed-in account (its own access does not change here).
    pub you: bool,
    pub owner: bool,
    pub grant: Option<GrantRole>,
    pub expires_at: Option<String>,
    pub guest: bool,
    /// Its grant's role can be changed here.
    pub can_change: bool,
    /// Its grant can be taken away here.
    pub can_revoke: bool,
    pub initials: String,
    /// Under the name: the e-mail and where the access comes from.
    pub sub: String,
    /// The role cannot be changed because it is a guest's: the role's tip.
    pub role_tip: Option<&'static str>,
    /// No Kaldır: why (the lock's tip).
    pub fixed: Option<&'static str>,
}

/// The avatar's letters: the first two words' initials, or “?”.
pub(crate) fn initials(name: &str) -> String {
    let letters: String = name
        .split(kentos_expression::js::text::is_space)
        .filter(|w| !w.is_empty())
        .take(2)
        .filter_map(|w| w.chars().next())
        .map(|c| upper_tr(&c.to_string()))
        .collect();
    if letters.is_empty() {
        "?".to_owned()
    } else {
        letters
    }
}

/// Why a row's access cannot be taken away here, for a row without Kaldır.
fn fixed_reason(you: bool, owner: bool, grant: bool) -> &'static str {
    if you {
        "Kendi erişiminizi buradan değiştiremezsiniz; proje sahibine ya da başka bir yöneticiye başvurun."
    } else if owner {
        "Proje sahibinin erişimi paylaşımla değişmez."
    } else if !grant {
        "Kurum politikasından gelen erişim paylaşımla değişmez."
    } else {
        "Bu erişimi değiştirme yetkiniz yok."
    }
}

const OUTSIDE: &str = "Kurum dışından biri “Davetler”den e-postayla davet edilir ve misafir olur.";

/// The note under the people: who else can reach the project.
pub(crate) fn policy_text(kind: TenantKind, admins_all: bool) -> String {
    match (kind, admins_all) {
        (TenantKind::Personal, _) => "Kişisel alanınızdaki bu proje yalnız paylaştığınız ve davet ettiğiniz kişilere açıktır.".to_owned(),
        (_, true) => format!("Kurumun politikası açık: kurum sahibi ve yöneticileri paylaşılmamış kurum projelerine de yönetici olarak erişir. Paylaşım kurumun üyeleriyledir; {OUTSIDE}"),
        (_, false) => format!("Kurumun politikası kapalı: kurum yöneticileri de yalnız kendileriyle paylaşılan projelere erişir. Paylaşım kurumun üyeleriyledir; {OUTSIDE}"),
    }
}

/// Kişiler once the server has listed the people.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PeopleView {
    /// The storage line: its bold start, then what it means.
    pub storage: (String, &'static str),
    pub rows: Vec<PersonRow>,
    pub count: String,
    pub policy: String,
}

/// The people as Kişiler lists them: the owner first, then by name in
/// Turkish order (the web's `personRows` and `peopleView`).
pub(crate) fn people_view(
    list: &ProjectAccessList,
    me: &str,
    may_share: bool,
    zone: &Zone,
) -> PeopleView {
    let mut rows: Vec<PersonRow> = list
        .people
        .iter()
        .map(|p| {
            let you = p.user_id == me;
            let owner = p.user_id == list.owner_id;
            let other = may_share && !you && !owner && p.grant.is_some();
            let name = if p.display_name.is_empty() {
                "Adı görünmeyen hesap".to_owned()
            } else {
                p.display_name.clone()
            };
            let source = source_text(p, zone);
            // Changing an ended grant's role would give it back without an end: shared again instead.
            let can_change = other && !p.expired && p.via == Some(AccessSource::Grant) && !p.guest;
            let sub = [p.email.clone().unwrap_or_default(), source.clone()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            PersonRow {
                user_id: p.user_id.clone(),
                initials: initials(&name),
                name,
                email: p.email.clone(),
                role_label: p.role.map_or("Erişemiyor", role_label).to_owned(),
                source,
                blocked: p.role.is_none(),
                you,
                owner,
                grant: p.grant,
                expires_at: p.expires_at.clone(),
                guest: p.guest,
                can_change,
                can_revoke: other,
                sub,
                role_tip: (!(can_change && p.grant.is_some()) && p.guest)
                    .then_some(people::GUEST_ROLE),
                fixed: (!other).then(|| fixed_reason(you, owner, p.grant.is_some())),
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        b.owner
            .cmp(&a.owner)
            .then_with(|| compare_tr(&a.name, &b.name))
    });
    let (title, detail) = storage_text(list.storage);
    PeopleView {
        storage: (people::storage(title), detail),
        count: people::count(rows.iter().filter(|r| !r.blocked).count()),
        rows,
        policy: policy_text(list.tenant_kind, list.admins_access_all_projects),
    }
}

/// Why Paylaş is off, or empty when it is on.
pub(crate) fn share_tip(may_share: bool, chosen: bool) -> &'static str {
    if !may_share {
        "Bu projede paylaşım yetkiniz yok (project.share)."
    } else if chosen {
        ""
    } else {
        "Önce “Kişi ekle” alanında bir kişi arayıp listeden seçin."
    }
}

/// What a share did: nothing new, a role changed, or someone added.
pub(crate) fn shared_text(name: &str, role: GrantRole, changed: bool, had: bool) -> String {
    let role = grant_label(role);
    if !changed {
        format!("{name} zaten {role} rolündeydi; değişen bir şey yok.")
    } else if had {
        format!("{name} artık {role}.")
    } else {
        format!("{name} projeye {role} olarak eklendi.")
    }
}

/// A grant's role changed in its row.
pub(crate) fn role_changed_text(name: &str, role: GrantRole) -> String {
    format!("{name} artık {}.", grant_label(role))
}

/// A grant taken away.
pub(crate) fn revoked_text(name: &str) -> String {
    format!("{name} artık projeye erişemiyor.")
}

/// A question the window asks before it acts.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Question {
    pub title: &'static str,
    pub message: String,
    pub details: Vec<String>,
    pub action: &'static str,
    pub cancel: &'static str,
}

/// Before taking someone's access away: a guest comes back by invitation, a member by a new share.
pub(crate) fn revoke_question(name: &str, project: &str, guest: bool) -> Question {
    Question {
        title: "Erişimi kaldır",
        message: format!("{name}, “{project}” projesine artık erişemesin mi?"),
        details: vec![
            "Projeyi şu anda açık tutuyorsa kaydı hemen durur; gönderilmemiş değişiklikleri kendi cihazında kalır.".to_owned(),
            "Daha önce indirdiği kopyalar ve ekranında gördükleri geri alınamaz.".to_owned(),
            if guest {
                "Kurum dışından olduğu için erişimini yeniden davetle geri verebilirsiniz."
            } else {
                "Yeniden paylaşarak erişimini geri verebilirsiniz."
            }
            .to_owned(),
        ],
        action: "Erişimi kaldır",
        cancel: "Vazgeç",
    }
}

// ── Kişi ekle ────────────────────────────────────────────────────────────

/// Nobody found for `query`: what the workspace allows, and for a whole
/// e-mail the offer to invite it instead.
pub(crate) fn nobody_found(query: &str, personal: bool) -> (String, Option<String>) {
    let text = if personal {
        format!(
            "“{query}” ile eşleşen kimse yok. Kurumlarınızın dışından biriyle “Davetler”den e-postayla paylaşabilirsiniz."
        )
    } else {
        format!(
            "“{query}” ile eşleşen etkin bir kurum üyesi yok. Kurum dışından biri “Davetler”den e-postayla davet edilir ve misafir olur."
        )
    };
    let invite = email_problem(query)
        .is_none()
        .then(|| format!("“{}” adresine e-postayla davet gönder…", query.trim()));
    (text, invite)
}

/// A found person who can use the project already: their role now.
pub(crate) fn candidate_note(role: Option<ProjectRole>) -> Option<String> {
    role.map(|r| find::has(role_label(r)))
}

/// The finder searches once this much is typed, spaces aside.
pub(crate) fn searches_for(query: &str) -> bool {
    query
        .chars()
        .filter(|c| !kentos_expression::js::text::is_space(*c))
        .count()
        >= 2
}

// ── Davetler (invitations.ts) ────────────────────────────────────────────

/// How long an invitation waits unless told otherwise, and at most (days).
pub(crate) const INVITE_DAYS: u32 = 14;
pub(crate) const INVITE_MAX_DAYS: u32 = 90;
/// The waits the form offers.
pub(crate) const INVITE_DAY_CHOICES: [u32; 7] = [1, 3, 7, 14, 30, 60, 90];

/// A wait's name in the Geçerlilik list.
pub(crate) fn day_text(days: u32) -> String {
    if days == INVITE_DAYS {
        format!("{days} gün (varsayılan)")
    } else {
        format!("{days} gün")
    }
}

/// An invitation's state.
pub(crate) fn state_label(state: InvitationState) -> &'static str {
    match state {
        InvitationState::Pending => "Bekliyor",
        InvitationState::Accepted => "Kabul edildi",
        InvitationState::Revoked => "Geri alındı",
        InvitationState::Expired => "Süresi doldu",
    }
}

/// Why `email` is not an address the server takes (its `check_email`).
pub(crate) fn email_problem(email: &str) -> Option<&'static str> {
    let e = kentos_expression::js::text::trim(email).to_lowercase();
    if e.is_empty() {
        return Some("Davet edilecek kişinin e-posta adresini yazın.");
    }
    let at = e.find('@');
    let domain = at.map_or("", |i| &e[i + 1..]);
    let ok = kentos_expression::js::text::utf16_len(&e) <= 254
        && !e
            .chars()
            .any(|c| kentos_expression::js::text::is_space(c) || c.is_control())
        && at.is_some_and(|i| i > 0)
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.');
    (!ok).then_some("Davet için geçerli bir e-posta adresi yazın (ör. ad@kurum.gov.tr).")
}

/// The `expiresAt` of an invitation that waits `days` days from `now_ms`
/// (milliseconds since 1970): none for the default, the longest ten
/// minutes inside the limit (a clock a little ahead is not refused).
pub(crate) fn invite_expiry(days: u32, now_ms: i64) -> Option<String> {
    if days == INVITE_DAYS {
        return None;
    }
    const DAY_MS: i64 = 86_400_000;
    let wait = (i64::from(days) * DAY_MS).min(i64::from(INVITE_MAX_DAYS) * DAY_MS - 600_000);
    Some(local_time::iso_utc_ms(now_ms + wait))
}

/// Why Davet et is off, or empty when it is on.
pub(crate) fn invite_tip(may_share: bool, email: &str) -> &'static str {
    if !may_share {
        "Bu projede davet yetkiniz yok (project.share)."
    } else if email.trim().is_empty() {
        "Önce davet edilecek e-posta adresini yazın."
    } else {
        ""
    }
}

/// The invitations as the window lists them: the waiting ones first, each part newest first.
pub(crate) fn sort_invitations(list: &[ProjectInvitation]) -> Vec<ProjectInvitation> {
    let mut out = list.to_vec();
    out.sort_by(|a, b| {
        let waiting = |i: &ProjectInvitation| i.state == InvitationState::Pending;
        waiting(b)
            .cmp(&waiting(a))
            .then_with(|| epoch(&b.created_at).cmp(&epoch(&a.created_at)))
    });
    out
}

/// The count beside “Davetler”: the waiting ones; nothing while the list is empty.
pub(crate) fn invites_count(list: &[ProjectInvitation]) -> String {
    if list.is_empty() {
        return String::new();
    }
    invites::count(
        list.iter()
            .filter(|i| i.state == InvitationState::Pending)
            .count(),
    )
}

/// One line under an invitation's e-mail.
pub(crate) fn invitation_sub(i: &ProjectInvitation, zone: &Zone) -> String {
    let who = if i.created_by_name.is_empty() {
        "görünmüyor"
    } else {
        &i.created_by_name
    };
    let by = format!("Davet eden: {who}, {}", date_text(&i.created_at, zone));
    match i.state {
        InvitationState::Pending => {
            format!(
                "{by} · {} tarihine kadar bekler",
                date_text(&i.expires_at, zone)
            )
        }
        InvitationState::Accepted => {
            let taker = i
                .accepted_by_name
                .as_deref()
                .filter(|n| !n.is_empty())
                .unwrap_or("görünmüyor");
            let when = i
                .accepted_at
                .as_deref()
                .map_or(String::new(), |a| format!(", {}", date_text(a, zone)));
            format!("{by} · Kabul eden: {taker}{when}")
        }
        InvitationState::Expired => {
            format!(
                "{by} · Süresi {} tarihinde doldu",
                date_text(&i.expires_at, zone)
            )
        }
        InvitationState::Revoked => by,
    }
}

/// What to ask before inviting an address: its waiting invitation is
/// replaced, and someone who can use the project already keeps a stronger role.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InviteQuestion {
    pub waiting: Option<ProjectInvitation>,
    /// Who can use the project already: their name and role's name.
    pub holder: Option<(String, &'static str)>,
}

pub(crate) fn invite_question(
    email: &str,
    invitations: &[ProjectInvitation],
    access: Option<&ProjectAccessList>,
) -> Option<InviteQuestion> {
    let e = kentos_expression::js::text::trim(email).to_lowercase();
    let waiting = invitations
        .iter()
        .find(|i| i.state == InvitationState::Pending && i.email == e)
        .cloned();
    let holder = access.and_then(|a| {
        a.people.iter().find_map(|p| {
            let role = p.role?;
            (p.email.as_deref().map(str::to_lowercase).as_deref() == Some(e.as_str())).then(|| {
                let name = if p.display_name.is_empty() {
                    e.clone()
                } else {
                    p.display_name.clone()
                };
                (name, role_label(role))
            })
        })
    });
    (waiting.is_some() || holder.is_some()).then_some(InviteQuestion { waiting, holder })
}

/// The question itself.
pub(crate) fn invite_ask(address: &str, q: &InviteQuestion, zone: &Zone) -> Question {
    let mut details = Vec::new();
    if let Some(w) = &q.waiting {
        details.push(format!("{} tarihli bekleyen davet geri alınır; onun bağlantısı artık çalışmaz. Yeni bağlantıyı yeniden iletmeniz gerekir.", date_text(&w.created_at, zone)));
    }
    if let Some((name, role)) = &q.holder {
        details.push(format!("{name} projeye zaten {role} olarak erişebiliyor. Davet ancak daha güçlü bir rol verir; rolü düşürmez."));
    }
    Question {
        title: if q.waiting.is_some() {
            "Bekleyen davet var"
        } else {
            "Zaten erişebiliyor"
        },
        message: format!("“{address}” için yeni bir davet gönderilsin mi?"),
        details,
        action: "Yeni davet gönder",
        cancel: "Vazgeç",
    }
}

/// The note under the invitations: what the invited get.
pub(crate) fn invite_rules(personal: bool) -> &'static str {
    if personal {
        "Davet edilen, bağlantıyı açıp davetin gönderildiği e-postanın hesabıyla girince projeye paylaşımla erişir. Bağlantıyı siz iletirsiniz; KentOS e-posta göndermez."
    } else {
        "Davet edilen, bağlantıyı açıp davetin gönderildiği e-postanın hesabıyla girince projeye erişir: kurumun üyesiyse paylaşımla, değilse misafir olarak (yalnız bu projeyi görür; rolü en çok Düzenleyici; kurum misafir almıyorsa kabul edilmez). Bağlantıyı siz iletirsiniz; KentOS e-posta göndermez."
    }
}

/// What the panel says of a new invitation: with its link (shown once) the
/// warning that it is shown only now; without it how to get one.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Invited {
    pub what: String,
    pub warn: String,
    pub link: bool,
}

pub(crate) fn invited_link(change: &InvitationChange, days: u32, zone: &Zone) -> Invited {
    let i = &change.invitation;
    let what = format!(
        "“{}” davet edildi: {}, {} tarihine kadar bekler.",
        i.email,
        grant_label(i.role),
        date_text(&i.expires_at, zone)
    );
    if change.token.is_none() {
        return Invited {
            what: format!("{what} {}", invites::NO_LINK),
            warn: invites::NO_LINK_WARN.to_owned(),
            link: false,
        };
    }
    Invited {
        what,
        warn: format!(
            "Bu bağlantı yalnız şimdi gösterilir: sunucu onu saklamaz, pencere kapanınca yeniden gösterilemez. Kopyalayıp davet ettiğiniz kişiye kendiniz iletin; {days} gün içinde, bir kez kullanılabilir. Kaybederseniz yeniden davet edin (eski bağlantı çalışmaz olur)."
        ),
        link: true,
    }
}

/// Before withdrawing a waiting invitation.
pub(crate) fn invitation_revoke_question(email: &str) -> Question {
    Question {
        title: "Daveti geri al",
        message: format!("“{email}” adresine gönderilen davet geri alınsın mı?"),
        details: vec![
            "Davetin bağlantısı artık çalışmaz; açan kişi projeye erişemez.".to_owned(),
            "Kişiye ayrıca haber vermeniz gerekmez; isterseniz daha sonra yeniden davet edebilirsiniz.".to_owned(),
        ],
        action: "Daveti geri al",
        cancel: "Vazgeç",
    }
}

/// An invitation's avatar: the address's first letter.
pub(crate) fn invitation_initial(email: &str) -> String {
    email
        .chars()
        .next()
        .map_or_else(|| "?".to_owned(), |c| upper_tr(&c.to_string()))
}

/// The link the inviter sends: the web app's address with the token (the
/// server's public address on the desktop, docs/adr/0111).
pub(crate) fn invitation_link(token: &str, base: &str) -> String {
    format!("{base}?davet={token}")
}

/// How the account reaches a project an invitation opened: the web's
/// invitation page says it; the desktop opens no invitation links, the
/// fixture keeps the words in step.
#[cfg(test)]
pub(crate) fn accepted_how(a: &InvitationAccepted) -> &'static str {
    if a.role == ProjectRole::Owner {
        "Projenin sahibisiniz; davet bir şey değiştirmedi."
    } else if a.guest {
        "Misafir olarak: kurumun dışındasınız ve yalnız bu projeyi görürsünüz."
    } else if a.tenant_kind == TenantKind::Personal {
        "Paylaşımla: proje bir kişisel alanda."
    } else {
        "Kurum üyesi olarak paylaşımla."
    }
}

// ── The product commands' inputs ─────────────────────────────────────────

/// `project.share` v1: gives `user` a role, or changes theirs; `expires_at` ends it by itself.
pub(crate) fn share_input(user: &str, role: GrantRole, expires_at: Option<&str>) -> Value {
    match expires_at {
        Some(e) => json!({ "userId": user, "role": role, "expiresAt": e }),
        None => json!({ "userId": user, "role": role }),
    }
}

/// `project.access.revoke` v1.
pub(crate) fn revoke_input(user: &str) -> Value {
    json!({ "userId": user })
}

/// `project.invite` v1: the address trimmed; the server's 14 days when no end is given.
pub(crate) fn invite_input(email: &str, role: GrantRole, expires_at: Option<&str>) -> Value {
    let email = email.trim();
    match expires_at {
        Some(e) => json!({ "email": email, "role": role, "expiresAt": e }),
        None => json!({ "email": email, "role": role }),
    }
}

/// `project.invitation.revoke` v1.
pub(crate) fn invitation_revoke_input(id: &str) -> Value {
    json!({ "invitationId": id })
}

// ── Failures and lines ───────────────────────────────────────────────────

/// A failed request as the window says it: what failed, then the cause and what to do.
pub(crate) fn failure_text(e: &ApiFailure, failed: &str) -> String {
    if e.code == "local" {
        return format!("{failed}: {}", e.message);
    }
    if e.status == 0 {
        return format!("{failed}: sunucuya ulaşılamadı. Bağlantınızı denetleyip yeniden deneyin.");
    }
    if e.status == 401 {
        return format!("{failed}: oturumunuz sona erdi. Yeniden giriş yapıp tekrar deneyin.");
    }
    if e.code == "not_found" && e.message == "Proje bulunamadı." {
        return format!(
            "{failed}: proje bulunamadı; silinmiş ya da size erişimi kaldırılmış olabilir. Pencereyi kapatıp proje listesini yenileyin."
        );
    }
    if e.retryable || matches!(e.status, 408 | 502 | 503 | 504) {
        return format!("{failed}: sunucu şu an yanıt vermiyor. Birazdan yeniden deneyin.");
    }
    format!("{failed}: {}", e.message)
}

/// A change to the people as the log writes it: the project, then the status line's text.
pub(crate) fn share_log(project: &str, text: &str) -> String {
    format!("“{project}”: {text}")
}

pub(crate) fn invited_log(project: &str, email: &str, role: GrantRole) -> String {
    format!(
        "“{project}”: “{email}” {} olarak davet edildi.",
        grant_label(role)
    )
}

pub(crate) fn invitation_revoked_log(project: &str, email: &str) -> String {
    format!("“{project}”: “{email}” için davet geri alındı.")
}

pub(crate) fn invitation_revoked_say(email: &str) -> String {
    format!("“{email}” için davet geri alındı; bağlantısı artık çalışmaz.")
}
