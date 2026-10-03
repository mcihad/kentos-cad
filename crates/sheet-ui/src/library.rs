//! The template library as the gallery shows it (docs/sheet/design.md §11,
//! §13; the web's `product/sheet/templates.ts`, `app/sheet/providers.ts`,
//! `cloudLibrary.ts` and `ui/sheet/galleryPlan.ts`, word for word): where a
//! card comes from, its badges, what the gallery offers for it and why it
//! cannot now, and what the gallery and the share window ask of the host's
//! cloud. The network is the host's (the desktop: `kentos-cloud`'s
//! `sheet_library`); this crate keeps the device's copies (`store.rs`) and
//! says what they are.

use std::collections::BTreeSet;

use kentos_sheet::cloud::{
    DeviceOrganization, SheetTemplateAccess, SheetTemplateCandidates, TemplateGrantRole,
    TemplateRole,
};

use crate::store::StoredTemplate;

/// What the library and the gallery say (the web's `LIBRARY_TEXTS`, `PROVIDER_TEXTS`, galleryPlan's reasons).
pub mod texts {
    pub const SIGN_IN: &str = "Bulut şablonları için önce bulut hesabınızla giriş yapın.";
    pub const OFFLINE: &str = "Sunucuya şu an ulaşılamıyor: bulut şablonlarının son indirilen sürümleri kullanılıyor; bağlantı gelince eşitlenir.";
    pub const SYNCING: &str = "Şablonlar eşitleniyor…";
    pub const NOT_OWNER: &str = "Yalnız şablonun sahibi paylaşır ve siler.";
    pub const SHARE_NEEDS_SYNC: &str = "Paylaşmak için önce buluta eşitleyin.";
    pub const SHARE_OWNER_ONLY: &str = "Yalnız şablonun sahibi paylaşır.";
    pub const SHARED_NO_DELETE: &str =
        "Benimle paylaşılan şablon silinmez; sahibi paylaşımı kaldırırsa listeden kalkar.";
    /// Kurumum, signed out.
    pub const ORGANISATION_SIGN_IN: &str = "Kurum şablonları için bulut hesabınızla giriş yapın.";
    /// Kurumum, signed in without an organisation.
    pub const NO_ORGANISATION: &str = "Bir kurumun etkin üyesi değilsiniz: kurum şablonlarını kurumun etkin, koltuklu üyeleri görür.";
    pub const ORGANISATION_NO_DELETE: &str =
        "Kurum şablonunu yalnız yayımlayan ve kurum yöneticileri siler.";
    pub const PUBLISH_NEEDS_SYNC: &str = "Kuruma yayımlamak için önce buluta eşitleyin.";
    pub const PUBLISH_NO_ORGANISATION: &str = "Yayımlayabileceğiniz bir kurum yok: kurum sahibi, yöneticiler ve proje açabilen üyeler yayımlar.";
    pub const PUBLISH_WAIT_SYNC: &str =
        "Bu cihazdaki değişiklikler eşitlenince yayımlayın: kuruma bulutun son sürümü kopyalanır.";

    /// “Kurum: Harita Bürosu”: an organisation's template's badge.
    pub fn organisation(name: &str) -> String {
        format!("Kurum: {name}")
    }

    pub fn published(name: &str, organisation: &str) -> String {
        format!(
            "“{name}” “{organisation}” kurumunda yayımlandı: kurumun etkin üyeleri görür ve kullanır."
        )
    }

    pub fn publish_failed(why: &str) -> String {
        format!("Kuruma yayımlanamadı: {why}")
    }

    /// The organisation's copy of the template is replaced by this one's content (a new revision there).
    pub fn republished(name: &str, organisation: &str) -> String {
        format!(
            "“{name}” “{organisation}” kurumundaki kopyasının yeni revizyonu olarak eşitleniyor."
        )
    }
    pub const SHARED_SIGN_IN: &str =
        "Sizinle paylaşılan şablonlar için bulut hesabınızla giriş yapın.";

    /// “Şablonlar eşitlendi · 14:05”: the host gives the local time.
    pub fn synced(at: &str) -> String {
        format!("Şablonlar eşitlendi · {at}")
    }

    pub fn uploaded(name: &str) -> String {
        format!("“{name}” buluta eşitlendi: web ve masaüstünde aynı; artık paylaşılabilir.")
    }

    pub fn not_uploaded(name: &str) -> String {
        format!("“{name}” şimdi eşitlenemedi; bağlantı gelince kendiliğinden gider.")
    }

    pub fn deleted(name: &str) -> String {
        format!("“{name}” silindi; bulut hesabınızdan da kalkıyor. Ondan yapılmış paftalar kalır.")
    }

    pub fn device_refused(name: &str, why: &str) -> String {
        format!("Bu cihazdaki “{name}” şablonu okunamadı: {why} Kayıt olduğu gibi korunuyor.")
    }
}

/// The signed-in account.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryAccount {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
}

/// How the sync stands (the web's `SyncStatus`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SyncStatus {
    #[default]
    SignedOut,
    /// The server could not be reached: the cached copies are used; it runs again on reconnect.
    Offline,
    Syncing,
    /// At this local time (“14:05”).
    Synced(String),
    /// A refusal that is not the network's (the server's words).
    Failed(String),
}

/// What the host says of the account's cloud library: it changes with the
/// sign-in, the connection and the sync.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Library {
    /// Whose cloud copies the lists show: the signed-in account's, or (not
    /// signed in yet, without a connection) the last one's, so they can be
    /// used; none after a sign-out.
    pub viewer: Option<String>,
    /// The signed-in account; none: the cloud's actions ask to sign in.
    pub account: Option<LibraryAccount>,
    pub status: SyncStatus,
    /// The templates a sync action is under way for (“Eşitleniyor”).
    pub busy: BTreeSet<String>,
    /// The server answers (the host's connection).
    pub online: bool,
    /// The organisations whose libraries the account sees, from the last list the host read
    /// (their names group Kurumum; where `can_publish`, “Kuruma yayımla…” may go).
    pub organizations: Vec<DeviceOrganization>,
}

impl Library {
    /// Why the account's library cannot be changed at all (signed out); none when it can,
    /// offline too (a change waits for the connection).
    pub fn account_why(&self) -> Option<&'static str> {
        self.account.is_none().then_some(texts::SIGN_IN)
    }

    /// Why the cloud's actions cannot run now (not signed in, the server away); none when they can.
    pub fn why(&self) -> Option<&'static str> {
        if self.account.is_none() {
            return Some(texts::SIGN_IN);
        }
        if self.status == SyncStatus::Offline || !self.online {
            return Some(texts::OFFLINE);
        }
        None
    }

    /// The line the gallery shows for the library now; empty when there is nothing to say.
    pub fn line(&self) -> String {
        match &self.status {
            SyncStatus::SignedOut => String::new(),
            SyncStatus::Offline => texts::OFFLINE.to_owned(),
            SyncStatus::Syncing => texts::SYNCING.to_owned(),
            SyncStatus::Failed(reason) => reason.clone(),
            SyncStatus::Synced(at) => texts::synced(at),
        }
    }
}

/// Where a card comes from (the web's `TemplateSource`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardSource {
    /// The program's own, read-only.
    System,
    /// Made here, only on this device.
    Device,
    /// The account's own in the cloud.
    Cloud,
    /// Shared with the account.
    Shared,
    /// An organisation's library's (design §13 “Kurum şablonları”).
    Organisation,
}

impl CardSource {
    /// Where a template kept here comes from.
    pub fn of(r: &StoredTemplate) -> CardSource {
        match &r.cloud {
            None => CardSource::Device,
            Some(c) if c.organization.is_some() => CardSource::Organisation,
            Some(c) if c.role == TemplateRole::Owner => CardSource::Cloud,
            Some(_) => CardSource::Shared,
        }
    }
}

/// A card's badges (the web's `TemplateBadge`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Badge {
    System,
    Device,
    Synced,
    Syncing,
    Unsynced,
    Conflict,
    Shared,
    Viewer,
    Editor,
    /// An organisation's: written “Kurum: <ad>” ([`texts::organisation`]).
    Organisation,
    /// An organisation's template the account published.
    Published,
    /// An organisation's template the account administers.
    Admin,
    Newer,
}

impl Badge {
    pub fn label(self) -> &'static str {
        match self {
            Badge::System => "Sistem",
            Badge::Device => "Bu cihazda",
            Badge::Synced => "Eşitlendi",
            Badge::Syncing => "Eşitleniyor",
            Badge::Unsynced => "Değişti, eşitlenmedi",
            Badge::Conflict => "Çakışma",
            Badge::Shared => "Paylaşıldı",
            Badge::Viewer => "Görüntüleyebilir",
            Badge::Editor => "Düzenleyebilir",
            Badge::Organisation => "Kurum",
            Badge::Published => "Yayımladınız",
            Badge::Admin => "Yöneticisiniz",
            Badge::Newer => "Yeni sürüm var",
        }
    }

    /// What it means, for its tooltip.
    pub fn tip(self) -> &'static str {
        match self {
            Badge::System => "Uygulamayla gelir; salt okunurdur.",
            Badge::Device => {
                "Yalnız bu cihazda; bulutta kopyası yok. Paylaşmak için önce buluta eşitleyin."
            }
            Badge::Synced => "Bulut hesabınızdakiyle aynı; web ve masaüstünde kullanılır.",
            Badge::Syncing => "Bulutla şimdi eşitleniyor.",
            Badge::Unsynced => "Bu cihazda değişti; bağlantı gelince buluta gider.",
            Badge::Conflict => {
                "Siz değiştirirken bulutta da değişmişti: bu, sizin sürümünüz; bulutunki ayrıca duruyor."
            }
            Badge::Shared => "Başkalarıyla paylaştınız.",
            Badge::Viewer => "Sizinle paylaşıldı: kullanır ve kopyalarsınız.",
            Badge::Editor => "Sizinle paylaşıldı: yeni sürümünü de kaydedebilirsiniz.",
            Badge::Organisation => {
                "Kurumunuzun şablonu: kurumun etkin üyeleri görür ve kullanır; yayımlayan ve kurum yöneticileri düzenler ve siler."
            }
            Badge::Published => "Bu kurum şablonunu siz yayımladınız: düzenler ve silersiniz.",
            Badge::Admin => "Kurum yöneticisi olarak düzenler ve silersiniz.",
            Badge::Newer => "Bu şablonun daha yeni bir sürümü var.",
        }
    }

    /// A badge that says something is wrong or waiting (drawn in the warning tone).
    pub fn warns(self) -> bool {
        matches!(self, Badge::Unsynced | Badge::Conflict | Badge::Newer)
    }
}

/// A template kept here: only on this device, or its place in the cloud and the account's role (the web's `libraryBadges`).
pub fn badges_of(r: &StoredTemplate, busy: &BTreeSet<String>) -> Vec<Badge> {
    let Some(c) = &r.cloud else {
        return vec![Badge::Device];
    };
    let mut out = Vec::new();
    if c.organization.is_some() {
        // An organisation's: its badge, and what the account is in it (a member only uses it).
        out.push(Badge::Organisation);
        match c.role {
            TemplateRole::Owner => out.push(Badge::Published),
            TemplateRole::Admin => out.push(Badge::Admin),
            TemplateRole::Editor | TemplateRole::Viewer => {}
        }
    } else {
        match c.role {
            TemplateRole::Viewer => out.push(Badge::Viewer),
            TemplateRole::Editor | TemplateRole::Admin => out.push(Badge::Editor),
            TemplateRole::Owner => {}
        }
    }
    out.push(if busy.contains(&r.id) {
        Badge::Syncing
    } else if c.changed || c.revision == 0 {
        Badge::Unsynced
    } else {
        Badge::Synced
    });
    if c.conflict_of.is_some() {
        out.push(Badge::Conflict);
    }
    if c.role == TemplateRole::Owner && c.shared && c.organization.is_none() {
        out.push(Badge::Shared);
    }
    out
}

/// The templates a viewer sees: this device's own, and the copies of one
/// account's library (deleted ones wait unseen) (the web's `visibleRecords`).
pub fn visible<'a>(
    records: &'a [StoredTemplate],
    viewer: Option<&str>,
) -> impl Iterator<Item = &'a StoredTemplate> {
    records.iter().filter(move |r| match &r.cloud {
        None => true,
        Some(c) => viewer == Some(c.account.as_str()) && !c.deleted,
    })
}

impl crate::designer::Designer {
    /// The template a sheet was made from, as this device knows it (the web's
    /// `TemplateShelf.known`): its name and revision, by its id among the system's and the
    /// templates kept here for the account (its own and those shared with it), or by an id it
    /// had before the cloud gave it its own — its revision then counted on from the one it had.
    pub fn template_of(
        &self,
        origin: &kentos_sheet::model::TemplateOrigin,
    ) -> Option<(String, u32)> {
        let id = origin.template_id.as_str();
        if let Some(t) = kentos_sheet::template::system_templates()
            .iter()
            .find(|t| t.meta.id == id)
        {
            return Some((t.meta.name.clone(), t.meta.revision));
        }
        let kept: Vec<&StoredTemplate> =
            visible(&self.user_templates, self.library.viewer.as_deref()).collect();
        if let Some(r) = kept.iter().find(|r| r.id == id) {
            return Some((r.template.meta.name.clone(), r.template.meta.revision));
        }
        kept.iter().find_map(|r| {
            let c = r.cloud.as_ref()?;
            c.former_ids.iter().any(|f| f == id).then(|| {
                (
                    r.template.meta.name.clone(),
                    (c.former_revision.unwrap_or(1) + r.template.meta.revision).saturating_sub(1),
                )
            })
        })
    }

    /// “Yeni sürüm var”: a newer revision of the sheet's template is kept here. The sheet does
    /// not change by itself (applying it comes in a later phase).
    pub fn template_newer(&self, sheet: &kentos_sheet::model::Sheet) -> bool {
        sheet.origin.as_ref().is_some_and(|o| {
            self.template_of(o)
                .is_some_and(|(_, revision)| revision > o.revision)
        })
    }
}

/// What the gallery offers for a card (the web's `TemplateAction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateAction {
    Use,
    Duplicate,
    Edit,
    Share,
    /// “Kuruma yayımla…”: one's own template copied into an organisation's library.
    Publish,
    Sync,
    Delete,
}

impl TemplateAction {
    /// The actions under a card's details, in order (Kullan is the window's own button).
    pub const DETAILS: [TemplateAction; 6] = [
        TemplateAction::Duplicate,
        TemplateAction::Edit,
        TemplateAction::Share,
        TemplateAction::Publish,
        TemplateAction::Sync,
        TemplateAction::Delete,
    ];

    /// What it does, for its tooltip (the web's `ACTION_TIP`).
    pub fn tip(self) -> &'static str {
        match self {
            TemplateAction::Use => {
                "Seçili şablondan yeni bir pafta yapar; seçilen kâğıda kısıtlarıyla yerleşir."
            }
            TemplateAction::Duplicate => {
                "Şablonun bir kopyasını şablonlarınıza (Benim, bu cihazda) alır."
            }
            TemplateAction::Edit => {
                "Şablonu pafta olarak açar; Şablon olarak kaydet ile geri yazılır. Sistem şablonu değişmez: kopyası kaydedilir."
            }
            TemplateAction::Share => {
                "Şablonu kurumunuzdaki kişilerle paylaşır: görüntüleyebilir ya da düzenleyebilir."
            }
            TemplateAction::Publish => {
                "Şablonunuzu kurumun şablon kitaplığına kopyalar: kurumun etkin üyeleri görür ve kullanır. Kurumdaki kopyasını sonra buradan güncelleyebilirsiniz."
            }
            TemplateAction::Sync => {
                "Bu cihazdaki şablonu bulut hesabınıza eşitler: web ve masaüstünde aynı olur, paylaşılabilir. Eşitlenmiş şablonlar kendiliğinden eşitlenir; bu, hemen eşitler."
            }
            TemplateAction::Delete => "Şablonu siler; ondan yapılmış paftalar kalır.",
        }
    }
}

/// An action as the gallery shows it for a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionView {
    /// Offered at all for this card (a system template has no Sil, Eşitle or Paylaş).
    pub shown: bool,
    pub label: &'static str,
    /// Why it cannot run now; none when it can.
    pub reason: Option<&'static str>,
}

/// Why the cloud cannot be used now, as the gallery's actions read it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Can {
    /// The cloud's actions (sync, share): not signed in, or the server away ([`Library::why`]).
    pub cloud: Option<&'static str>,
    /// Changing the account's library at all: not signed in ([`Library::account_why`]).
    pub account: Option<&'static str>,
    /// Publishing into an organisation: the cloud's reason, or no organisation it may publish into.
    pub publish: Option<&'static str>,
}

impl Library {
    /// What the gallery can do with the cloud now.
    pub fn can(&self) -> Can {
        let cloud = self.why();
        Can {
            cloud,
            account: self.account_why(),
            publish: cloud.or_else(|| {
                (!self.organizations.iter().any(|o| o.can_publish))
                    .then_some(texts::PUBLISH_NO_ORGANISATION)
            }),
        }
    }
}

/// What the gallery offers for a card and why it cannot now (the web's
/// `actionsOf`): system templates are read-only (used, copied, edited as a
/// copy); the user's own are edited, synced, deleted and published into an
/// organisation; one only on this device cannot be shared or published before
/// it is synced; a shared one is used and copied (edited too by its editor),
/// and its owner decides the rest; an organisation's is used and copied by
/// every member, edited and deleted by the one who published it and the
/// organisation's administrators, and never shared one by one.
pub fn action(
    source: CardSource,
    role: Option<TemplateRole>,
    can: Can,
    a: TemplateAction,
) -> ActionView {
    let system = source == CardSource::System;
    let device = source == CardSource::Device;
    let cloud_card = source == CardSource::Cloud;
    let shared = source == CardSource::Shared;
    let org = source == CardSource::Organisation;
    let editor = shared && role == Some(TemplateRole::Editor);
    // An organisation's template the account published or administers.
    let manages = org && matches!(role, Some(TemplateRole::Owner | TemplateRole::Admin));
    let cloud = can.cloud;
    match a {
        TemplateAction::Use => ActionView {
            shown: true,
            label: "Kullan",
            reason: None,
        },
        TemplateAction::Duplicate => ActionView {
            shown: true,
            label: if system || shared || org {
                "Şablonlarıma kopyala"
            } else {
                "Çoğalt"
            },
            reason: None,
        },
        TemplateAction::Edit => ActionView {
            shown: true,
            label: if system || (shared && !editor) || (org && !manages) {
                "Kopyasını düzenle"
            } else {
                "Düzenle"
            },
            reason: None,
        },
        TemplateAction::Share => ActionView {
            shown: !system && !org,
            label: "Paylaş…",
            reason: if shared {
                Some(texts::SHARE_OWNER_ONLY)
            } else if device {
                Some(texts::SHARE_NEEDS_SYNC)
            } else {
                cloud
            },
        },
        TemplateAction::Publish => ActionView {
            shown: device || cloud_card,
            label: "Kuruma yayımla…",
            reason: if device {
                Some(texts::PUBLISH_NEEDS_SYNC)
            } else {
                can.publish
            },
        },
        TemplateAction::Sync => ActionView {
            shown: device || cloud_card || shared || org,
            label: if device {
                "Buluta eşitle"
            } else {
                "Şimdi eşitle"
            },
            reason: cloud,
        },
        TemplateAction::Delete => ActionView {
            shown: !system,
            label: "Sil",
            reason: if shared {
                Some(texts::SHARED_NO_DELETE)
            } else if org && !manages {
                Some(texts::ORGANISATION_NO_DELETE)
            } else if cloud_card || org {
                can.account
            } else {
                None
            },
        },
    }
}

/// What the gallery and the share window ask of the host's cloud.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryRequest {
    /// A sync now (Eşitle, a change saved here, a deletion).
    Sync,
    /// “Buluta eşitle”: the template was marked to go up (`DeviceCloudState::to_upload`);
    /// a sync now, then [`texts::uploaded`] or [`texts::not_uploaded`].
    Upload { id: String, name: String },
    /// Who has the template (its owner only).
    Access { id: String },
    /// Whom it may be shared with: members of the account's organisations whose name or e-mail holds every word.
    Candidates { id: String, query: String },
    /// Shares it with a person, or changes their role (`change`: in their row of the list).
    Share {
        id: String,
        user: String,
        person: String,
        role: TemplateGrantRole,
        change: bool,
    },
    /// Takes a person's access away.
    Unshare {
        id: String,
        user: String,
        person: String,
    },
    /// “Kuruma yayımla”: the account's template `id` copied into the organisation `organization`
    /// (`sheet.template.publish`); a sync follows, bringing the organisation's copy.
    Publish {
        id: String,
        name: String,
        organization: String,
        organization_name: String,
    },
}

/// The host's answers to the share window.
#[derive(Clone, Debug, PartialEq)]
pub enum LibraryEvent {
    Access {
        id: String,
        result: Result<SheetTemplateAccess, String>,
    },
    Candidates {
        id: String,
        query: String,
        result: Result<SheetTemplateCandidates, String>,
    },
    /// A share or an unshare was answered: what to say, or why it failed.
    Shared {
        id: String,
        result: Result<String, String>,
    },
    /// The templates kept here changed (a sync): the lists are read again.
    Changed,
    /// The sync's state, the sign-in or the connection changed.
    State(Library),
    /// Ids the cloud gave the templates created from this device: the gallery's choice follows them.
    Created(Vec<(String, String)>),
    /// “Buluta eşitle” could not take this template up now (it goes when the connection comes back).
    NotUploaded(String),
    /// “Kuruma yayımla” was answered: what to say and the organisation's new template's id, or
    /// why it failed.
    Published {
        id: String,
        result: Result<(String, Option<String>), String>,
    },
}

#[cfg(test)]
mod tests {
    use kentos_sheet::cloud::DeviceCloudState;

    use super::*;

    fn record(id: &str, cloud: Option<DeviceCloudState>) -> StoredTemplate {
        let mut t = kentos_sheet::template::system_template("sys:genel-a4-dikey")
            .expect("a system template")
            .clone();
        t.meta.id = id.to_owned();
        StoredTemplate {
            id: id.to_owned(),
            saved: 1,
            template: t,
            cloud,
        }
    }

    fn cloud(account: &str, revision: u32, role: TemplateRole) -> DeviceCloudState {
        DeviceCloudState {
            revision,
            changed: false,
            role,
            ..DeviceCloudState::to_upload(account)
        }
    }

    #[test]
    fn the_badges_are_the_web_s() {
        let none = BTreeSet::new();
        assert_eq!(badges_of(&record("a", None), &none), [Badge::Device]);
        let up = record("b", Some(DeviceCloudState::to_upload("ayse")));
        assert_eq!(badges_of(&up, &none), [Badge::Unsynced]);
        let mut shared = cloud("ayse", 2, TemplateRole::Owner);
        shared.shared = true;
        assert_eq!(
            badges_of(&record("c", Some(shared)), &none),
            [Badge::Synced, Badge::Shared]
        );
        let mut conflict = cloud("ayse", 1, TemplateRole::Owner);
        conflict.conflict_of = Some("x".into());
        assert_eq!(
            badges_of(&record("d", Some(conflict)), &none),
            [Badge::Synced, Badge::Conflict]
        );
        let busy: BTreeSet<String> = ["e".to_owned()].into();
        assert_eq!(
            badges_of(
                &record("e", Some(cloud("ayse", 4, TemplateRole::Editor))),
                &busy
            ),
            [Badge::Editor, Badge::Syncing]
        );
        assert_eq!(Badge::Unsynced.label(), "Değişti, eşitlenmedi");
    }

    #[test]
    fn a_viewer_sees_this_device_s_and_its_own_account_s() {
        let mut gone = cloud("ayse", 2, TemplateRole::Owner);
        gone.deleted = true;
        let records = vec![
            record("a", None),
            record("b", Some(cloud("ayse", 1, TemplateRole::Owner))),
            record("c", Some(cloud("bora", 1, TemplateRole::Owner))),
            record("d", Some(gone)),
        ];
        let ids = |viewer| {
            visible(&records, viewer)
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(Some("ayse")), ["a", "b"]);
        assert_eq!(ids(None), ["a"]);
    }

    #[test]
    fn the_actions_and_their_reasons_are_the_web_s() {
        let can = |cloud| Can {
            cloud,
            account: None,
            publish: cloud,
        };
        let action = |s, cloud, a| action(s, None, can(cloud), a);
        let share = |s, cloud| action(s, cloud, TemplateAction::Share);
        assert!(!share(CardSource::System, None).shown);
        assert_eq!(
            share(CardSource::Device, None).reason,
            Some(texts::SHARE_NEEDS_SYNC)
        );
        assert_eq!(
            share(CardSource::Shared, None).reason,
            Some(texts::SHARE_OWNER_ONLY)
        );
        assert_eq!(
            share(CardSource::Cloud, Some(texts::OFFLINE)).reason,
            Some(texts::OFFLINE)
        );
        assert_eq!(share(CardSource::Cloud, None).reason, None);
        let sync = action(CardSource::Device, None, TemplateAction::Sync);
        assert_eq!((sync.shown, sync.label), (true, "Buluta eşitle"));
        assert_eq!(
            action(CardSource::Cloud, None, TemplateAction::Sync).label,
            "Şimdi eşitle"
        );
        assert!(action(CardSource::Shared, None, TemplateAction::Sync).shown);
        assert_eq!(
            action(CardSource::Shared, None, TemplateAction::Duplicate).label,
            "Şablonlarıma kopyala"
        );
        assert_eq!(
            action(CardSource::System, None, TemplateAction::Edit).label,
            "Kopyasını düzenle"
        );
        assert_eq!(
            action(CardSource::Shared, None, TemplateAction::Delete).reason,
            Some(texts::SHARED_NO_DELETE)
        );
        // A device template is deleted here at once, whatever the cloud.
        assert_eq!(
            action(
                CardSource::Device,
                Some(texts::SIGN_IN),
                TemplateAction::Delete
            )
            .reason,
            None
        );
        // An editor edits the shared template itself; a viewer a copy.
        let edit = |role| {
            super::action(
                CardSource::Shared,
                Some(role),
                can(None),
                TemplateAction::Edit,
            )
            .label
        };
        assert_eq!(edit(TemplateRole::Editor), "Düzenle");
        assert_eq!(edit(TemplateRole::Viewer), "Kopyasını düzenle");
        // A cloud template is deleted offline too (the cloud hears of it later); not signed out.
        let delete =
            |can| super::action(CardSource::Cloud, None, can, TemplateAction::Delete).reason;
        assert_eq!(
            delete(Can {
                cloud: Some(texts::OFFLINE),
                account: None,
                publish: Some(texts::OFFLINE)
            }),
            None
        );
        assert_eq!(
            delete(Can {
                cloud: Some(texts::SIGN_IN),
                account: Some(texts::SIGN_IN),
                publish: Some(texts::SIGN_IN)
            }),
            Some(texts::SIGN_IN)
        );
    }

    /// An organisation's template (design §13 “Kurum şablonları”): its card source and badges;
    /// every member uses it and copies it to their own; the one who published it and the
    /// administrators edit and delete it; nobody shares it one by one. One's own template is
    /// published into an organisation once it is in the cloud and the account may publish there.
    #[test]
    fn an_organisation_s_template_follows_its_rules() {
        let org = |role| {
            let mut c = cloud("bora", 2, role);
            c.organization = Some(kentos_sheet::cloud::CloudOrganization {
                tenant_id: "buro".into(),
                name: "Harita Bürosu".into(),
            });
            c
        };
        let none = BTreeSet::new();
        let r = record("k", Some(org(TemplateRole::Viewer)));
        assert_eq!(CardSource::of(&r), CardSource::Organisation);
        assert_eq!(badges_of(&r, &none), [Badge::Organisation, Badge::Synced]);
        assert_eq!(
            badges_of(&record("k", Some(org(TemplateRole::Owner))), &none),
            [Badge::Organisation, Badge::Published, Badge::Synced]
        );
        assert_eq!(
            badges_of(&record("k", Some(org(TemplateRole::Admin))), &none),
            [Badge::Organisation, Badge::Admin, Badge::Synced]
        );
        assert_eq!(texts::organisation("Harita Bürosu"), "Kurum: Harita Bürosu");
        let ready = Can::default();
        let act = |role, a| action(CardSource::Organisation, Some(role), ready, a);
        assert_eq!(
            act(TemplateRole::Viewer, TemplateAction::Duplicate).label,
            "Şablonlarıma kopyala"
        );
        assert_eq!(
            act(TemplateRole::Viewer, TemplateAction::Edit).label,
            "Kopyasını düzenle"
        );
        assert_eq!(
            act(TemplateRole::Admin, TemplateAction::Edit).label,
            "Düzenle"
        );
        assert_eq!(
            act(TemplateRole::Owner, TemplateAction::Edit).label,
            "Düzenle"
        );
        assert_eq!(
            act(TemplateRole::Viewer, TemplateAction::Delete).reason,
            Some(texts::ORGANISATION_NO_DELETE)
        );
        assert_eq!(
            act(TemplateRole::Admin, TemplateAction::Delete).reason,
            None
        );
        assert!(!act(TemplateRole::Owner, TemplateAction::Share).shown);
        assert!(!act(TemplateRole::Owner, TemplateAction::Publish).shown);
        assert!(act(TemplateRole::Viewer, TemplateAction::Sync).shown);
        // Kuruma yayımla: one's own, in the cloud, with an organisation to publish into.
        let mut l = Library {
            account: Some(LibraryAccount::default()),
            online: true,
            ..Library::default()
        };
        let publish = |l: &Library, s| action(s, None, l.can(), TemplateAction::Publish);
        assert_eq!(
            publish(&l, CardSource::Cloud).reason,
            Some(texts::PUBLISH_NO_ORGANISATION)
        );
        l.organizations = vec![DeviceOrganization {
            tenant_id: "buro".into(),
            name: "Harita Bürosu".into(),
            can_publish: true,
        }];
        assert_eq!(publish(&l, CardSource::Cloud).reason, None);
        assert_eq!(publish(&l, CardSource::Cloud).label, "Kuruma yayımla…");
        assert_eq!(
            publish(&l, CardSource::Device).reason,
            Some(texts::PUBLISH_NEEDS_SYNC)
        );
        assert!(!publish(&l, CardSource::Shared).shown && !publish(&l, CardSource::System).shown);
        l.online = false;
        assert_eq!(publish(&l, CardSource::Cloud).reason, Some(texts::OFFLINE));
    }

    #[test]
    fn the_line_and_the_reason_follow_the_sync() {
        let mut l = Library::default();
        assert_eq!((l.why(), l.line().as_str()), (Some(texts::SIGN_IN), ""));
        l.account = Some(LibraryAccount {
            id: "ayse".into(),
            name: "Ayşe".into(),
            email: None,
        });
        l.online = true;
        l.status = SyncStatus::Synced("14:05".into());
        assert_eq!(l.why(), None);
        assert_eq!(l.line(), "Şablonlar eşitlendi · 14:05");
        l.status = SyncStatus::Offline;
        assert_eq!(l.why(), Some(texts::OFFLINE));
        assert_eq!(l.line(), texts::OFFLINE);
    }
}
