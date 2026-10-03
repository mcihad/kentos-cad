//! The sheet template library's sync from the desktop (docs/sheet/design.md
//! §13; the web's `app/sheet/templateSync.ts`, action for action and word for
//! word): the account's copies on this device against the cloud's list,
//! planned by the core (`kentos_sheet::sync::plan_sync`: pure, the rules of
//! design §13's table, no path losing data silently) and carried out here,
//! one action after another:
//!
//! - **download**: the revision's content, read and checked by the core,
//!   becomes this device's copy (the cache used offline); an organisation's
//!   template too, its copy knowing the organisation (design §13 “Kurum
//!   şablonları”: the same rules);
//! - **upload / create**: the copy goes up (`sheet.template.update` with the
//!   revision it is based on; `…create` for one never uploaded, which takes
//!   the id the cloud gives it, its old id kept for the sheets made from
//!   it); without the right to write any more, the changes stay here as a
//!   template of their own, “… (bu cihazdaki kopya)”;
//! - **uploadCopy**: a conflict: the copy changed here goes up as a new
//!   template “… (bu cihazdaki kopya)”, then the cloud's is downloaded;
//! - **removeLocal / keepLocal / deleteRemote / keepRemote**: as the plan
//!   says, with its words to the user.
//!
//! A command goes to its template's space: the personal space for a
//! person's, the organisation for one of its library (a conflict's copy of an
//! organisation's template goes up as the account's own). A command's
//! idempotency key is derived from what it sends, so a retry after an answer
//! that never came is the same request (the server replays it). A network failure stops the run and leaves the rest as it was; the
//! caller runs again later (the connection back, an event, a change here).
//! Where the copies are kept is the caller's ([`DeviceLibrary`]).

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{Me, TenantKind};
use kentos_sheet::cloud::{
    DeviceCloudState, DeviceOrganization, SheetTemplateSummary, TemplateRole,
};
use kentos_sheet::sync::{COPY_SUFFIX, LocalTemplate, RemoteTemplate, SyncAction, plan_sync};
use kentos_sheet::template::{Template, read_template};
use uuid::Uuid;

use crate::api::{Cloud, hex_sha256};
use crate::failure::ApiFailure;
use crate::sheet_templates as api;

/// What the sync says to the user (the web's `SYNC_TEXTS`).
pub mod texts {
    pub const NO_SPACE: &str =
        "Hesabın kişisel alanı bulunamadı; sunucu bu sürümle uyuşmuyor olabilir.";

    pub fn list_failed(why: &str) -> String {
        format!("Bulut şablonları okunamadı: {why}")
    }

    pub fn unshared(name: &str) -> String {
        format!(
            "“{name}” artık sizinle paylaşılmıyor; listenizden kalktı. Ondan yapılmış paftalar kalır."
        )
    }

    pub fn removed(name: &str) -> String {
        format!(
            "“{name}” bulutta silindi; bu cihazdaki kopyası da kalktı. Ondan yapılmış paftalar kalır."
        )
    }

    /// An organisation's template gone from the account's list: deleted there, or the account
    /// no longer in the organisation (the list cannot tell which).
    pub fn left_organisation(name: &str, organisation: &str) -> String {
        format!(
            "“{name}” artık “{organisation}” kurumunun şablonları arasında görünmüyor; bu cihazdaki kopyası da kalktı. Ondan yapılmış paftalar kalır."
        )
    }

    pub fn conflict(name: &str, copy: &str) -> String {
        format!(
            "“{name}” siz değiştirirken bulutta da değişmiş: ikisi de kaldı. Bulutun sürümü indirildi; sizin değişiklikleriniz “{copy}” adıyla ayrı bir şablon oldu."
        )
    }

    pub fn read_only(name: &str, copy: &str) -> String {
        format!(
            "“{name}” için düzenleme izniniz kalmamış: değişiklikleriniz “{copy}” adıyla bu cihazda ayrı bir şablon olarak kaldı."
        )
    }

    pub fn action_failed(name: &str, why: &str) -> String {
        format!("“{name}” eşitlenemedi: {why}")
    }
}

/// How many events a page holds at most (the server's `EVENTS_PAGE_MAX`, its default).
pub const EVENTS_PAGE: usize = 500;

/// A template kept on the device: its content and, for one of an account's cloud library, its place there.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceRecord {
    pub id: String,
    pub template: Template,
    pub cloud: Option<DeviceCloudState>,
}

/// Where the device keeps its templates (the desktop: `kentos-sheet-ui`'s store; the tests: memory).
pub trait DeviceLibrary: Send + Sync {
    /// Every template kept, in a stable order (the ones that do not read are left out).
    fn records(&self) -> Vec<DeviceRecord>;
    fn save(&self, record: &DeviceRecord) -> Result<(), String>;
    fn remove(&self, id: &str) -> Result<(), String>;
}

/// The signed-in account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub name: String,
}

impl Account {
    pub fn of(me: &Me) -> Account {
        Account {
            id: me.user.id.clone(),
            name: me.user.display_name.clone(),
        }
    }
}

/// The account's personal space, where its template commands go.
pub fn personal_of(me: &Me) -> Option<Uuid> {
    me.memberships
        .iter()
        .find(|m| m.tenant_kind == TenantKind::Personal)
        .and_then(|m| Uuid::parse_str(&m.tenant_id).ok())
}

/// How a sentence is said (the message log's level).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Info,
    Warn,
    Error,
    Success,
}

/// How a run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncEnd {
    Synced,
    /// The session ended (401): sign in again.
    SignedOut,
    /// The server could not be reached: the cached copies are used; run again later.
    Offline,
    /// A refusal that is not the network's (the server's words).
    Failed(String),
}

/// What one run did.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncReport {
    pub end: SyncEnd,
    /// What to tell the user, in order.
    pub said: Vec<(Tone, String)>,
    /// Ids the cloud gave the templates created from this device: local id → cloud id.
    pub created: Vec<(String, String)>,
    /// The device's templates changed (the gallery's lists follow).
    pub changed: bool,
    /// A 409 or a refused deletion on the way: another run should follow at once.
    pub again: bool,
    /// The organisations whose libraries the account sees, from the list; none: the list could not be read.
    pub organizations: Option<Vec<DeviceOrganization>>,
}

fn local(message: impl Into<String>) -> ApiFailure {
    ApiFailure::new(0, "local", message)
}

/// A command's idempotency key from what it sends: a retry is the same request.
pub fn key_of(what: &str) -> Uuid {
    let hex = hex_sha256(what.as_bytes());
    let mut b = [0u8; 16];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = hex
            .get(i * 2..i * 2 + 2)
            .and_then(|h| u8::from_str_radix(h, 16).ok())
            .unwrap_or(0);
    }
    Uuid::from_bytes(b)
}

/// The first 12 bytes of the content's SHA-256, in hex (the web's `digest`).
fn digest(t: &Template) -> String {
    let text = serde_json::to_string(t).unwrap_or_default();
    let hex = hex_sha256(text.as_bytes());
    hex.get(..24).unwrap_or(&hex).to_owned()
}

/// The content with the id and revision the cloud gave it (the cloud writes them into its own copy too).
fn with_meta(t: &Template, id: &str, revision: u32) -> Template {
    let mut t = t.clone();
    t.meta.id = id.to_owned();
    t.meta.revision = revision;
    t
}

/// The record's content, its id the record's.
fn template_of(r: &DeviceRecord, id: &str) -> Template {
    let mut t = r.template.clone();
    if t.meta.id != id {
        t.meta.id = id.to_owned();
    }
    t
}

fn uuid_of(id: &str) -> Result<Uuid, ApiFailure> {
    Uuid::parse_str(id).map_err(|_| local(format!("“{id}” bir bulut şablonu kimliği değil.")))
}

/// The organisation a copy's template is in (its commands go to that organisation's route).
fn organisation_of(r: &DeviceRecord) -> Option<Result<Uuid, ApiFailure>> {
    let o = r.cloud.as_ref()?.organization.as_ref()?;
    Some(Uuid::parse_str(&o.tenant_id).map_err(|_| {
        local(format!(
            "“{}” kurumunun kimliği okunamadı: {}",
            o.name, o.tenant_id
        ))
    }))
}

struct Run<'a> {
    cloud: &'a Cloud,
    personal: Option<Uuid>,
    me: &'a Account,
    lib: &'a dyn DeviceLibrary,
    report: SyncReport,
}

impl Run<'_> {
    fn say(&mut self, tone: Tone, text: String) {
        self.report.said.push((tone, text));
    }

    fn personal(&self) -> Result<Uuid, ApiFailure> {
        self.personal.ok_or_else(|| local(texts::NO_SPACE))
    }

    /// The command route of a copy's template: its organisation's, or the personal space.
    fn space_of(&self, r: &DeviceRecord) -> Result<Uuid, ApiFailure> {
        organisation_of(r).unwrap_or_else(|| self.personal())
    }

    fn save(&mut self, r: DeviceRecord) -> Result<(), ApiFailure> {
        self.lib
            .save(&r)
            .map_err(|e| local(format!("Şablon bu cihaza yazılamadı: {e}")))?;
        self.report.changed = true;
        Ok(())
    }

    fn remove(&mut self, id: &str) -> Result<(), ApiFailure> {
        self.lib
            .remove(id)
            .map_err(|e| local(format!("Şablon bu cihazdan silinemedi: {e}")))?;
        self.report.changed = true;
        Ok(())
    }

    /// The template's newest revision, read and checked by the core, as this device's copy.
    async fn download(
        &mut self,
        id: &str,
        prev: Option<&DeviceCloudState>,
    ) -> Result<(), ApiFailure> {
        let d = api::detail(self.cloud, uuid_of(id)?).await?;
        let json = serde_json::to_string(&d.content).map_err(|e| local(e.to_string()))?;
        let t = read_template(&json).map_err(|e| local(e.message))?;
        let cloud = DeviceCloudState::of_summary(&self.me.id, &d.summary, prev);
        self.save(DeviceRecord {
            id: id.to_owned(),
            template: t,
            cloud: Some(cloud),
        })
    }

    /// Carries out one action.
    async fn act(
        &mut self,
        a: &SyncAction,
        records: &BTreeMap<String, DeviceRecord>,
    ) -> Result<(), ApiFailure> {
        match a {
            SyncAction::Download(t) => {
                let prev = records.get(&t.id).and_then(|r| r.cloud.clone());
                self.download(&t.id, prev.as_ref()).await
            }
            SyncAction::Upload(u) => {
                let Some(r) = records.get(&u.id) else {
                    return Ok(());
                };
                let content = template_of(r, &u.id);
                let key = key_of(&format!(
                    "sheet-tpl-update-{}-{}-{}",
                    u.id,
                    u.expected_revision,
                    digest(&content)
                ));
                let answer = api::update(
                    self.cloud,
                    self.space_of(r)?,
                    uuid_of(&u.id)?,
                    u.expected_revision,
                    &content,
                    key,
                )
                .await;
                match answer {
                    Ok(res) => {
                        let mut cloud = r
                            .cloud
                            .clone()
                            .unwrap_or_else(|| DeviceCloudState::to_upload(&self.me.id));
                        cloud.revision = res.revision;
                        cloud.changed = false;
                        self.save(DeviceRecord {
                            id: u.id.clone(),
                            template: with_meta(&content, &u.id, res.revision),
                            cloud: Some(cloud),
                        })
                    }
                    // No right to write any more (the owner made it view-only): the changes stay here
                    // as a template of their own, and this device's copy goes back to the cloud's.
                    Err(e) if e.status == 403 => {
                        let name = r.template.meta.name.clone();
                        let copy = format!("{name}{COPY_SUFFIX}");
                        let id = Uuid::now_v7().to_string();
                        let mut own = content.clone();
                        own.meta.id = id.clone();
                        own.meta.name = copy.clone();
                        self.save(DeviceRecord {
                            id,
                            template: own,
                            cloud: None,
                        })?;
                        self.download(&u.id, r.cloud.as_ref()).await?;
                        self.say(Tone::Warn, texts::read_only(&name, &copy));
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            SyncAction::Create(c) => {
                let Some(r) = records.get(&c.id) else {
                    return Ok(());
                };
                let content = template_of(r, &c.id);
                let key = key_of(&format!("sheet-tpl-create-{}-{}", c.id, digest(&content)));
                // Made for an organisation's library on this device: it goes there.
                let res = api::create(self.cloud, self.space_of(r)?, &content, key).await?;
                let mut former = r
                    .cloud
                    .as_ref()
                    .map(|c| c.former_ids.clone())
                    .unwrap_or_default();
                former.push(c.id.clone());
                let cloud = DeviceCloudState {
                    revision: res.revision,
                    changed: false,
                    former_ids: former,
                    former_revision: Some(content.meta.revision),
                    organization: r.cloud.as_ref().and_then(|x| x.organization.clone()),
                    ..DeviceCloudState::to_upload(&self.me.id)
                };
                self.save(DeviceRecord {
                    id: res.template_id.clone(),
                    template: with_meta(&content, &res.template_id, res.revision),
                    cloud: Some(cloud),
                })?;
                if res.template_id != c.id {
                    self.remove(&c.id)?;
                }
                self.report.created.push((c.id.clone(), res.template_id));
                Ok(())
            }
            SyncAction::UploadCopy(c) => {
                let Some(r) = records.get(&c.id) else {
                    return Ok(());
                };
                let mut content = template_of(r, &c.id);
                content.meta.name = c.name.clone();
                let base = r.cloud.as_ref().map_or(0, |x| x.revision);
                let key = key_of(&format!(
                    "sheet-tpl-copy-{}-{base}-{}",
                    c.id,
                    digest(&content)
                ));
                let res = api::create(self.cloud, self.personal()?, &content, key).await?;
                let cloud = DeviceCloudState {
                    revision: res.revision,
                    changed: false,
                    conflict_of: Some(c.id.clone()),
                    ..DeviceCloudState::to_upload(&self.me.id)
                };
                self.save(DeviceRecord {
                    id: res.template_id.clone(),
                    template: with_meta(&content, &res.template_id, res.revision),
                    cloud: Some(cloud),
                })?;
                self.say(Tone::Warn, texts::conflict(&r.template.meta.name, &c.name));
                Ok(())
            }
            SyncAction::RemoveLocal(x) => {
                let Some(r) = records.get(&x.id) else {
                    return Ok(());
                };
                self.remove(&x.id)?;
                if let Some(c) = &r.cloud
                    && !c.deleted
                {
                    let name = &r.template.meta.name;
                    let text = match &c.organization {
                        Some(o) => texts::left_organisation(name, &o.name),
                        None if c.role == TemplateRole::Owner => texts::removed(name),
                        None => texts::unshared(name),
                    };
                    self.say(Tone::Info, text);
                }
                Ok(())
            }
            SyncAction::KeepLocal(n) => {
                let Some(r) = records.get(&n.id) else {
                    return Ok(());
                };
                // A template of this device only now (its cloud one is gone): kept as it is, said.
                self.save(DeviceRecord {
                    id: r.id.clone(),
                    template: r.template.clone(),
                    cloud: None,
                })?;
                self.say(Tone::Warn, n.message.clone());
                Ok(())
            }
            SyncAction::DeleteRemote(t) => {
                let key = key_of(&format!("sheet-tpl-delete-{}-{}", t.id, t.revision));
                let space = match records.get(&t.id) {
                    Some(r) => self.space_of(r)?,
                    None => self.personal()?,
                };
                let answer =
                    api::delete(self.cloud, space, uuid_of(&t.id)?, Some(t.revision), key).await;
                match answer {
                    Ok(_) => {}
                    // Not ours to delete (shared with us): it comes back on the next run.
                    Err(e) if e.status == 403 => self.report.again = true,
                    // Gone already: nothing to do.
                    Err(e) if e.status == 404 => {}
                    Err(e) => return Err(e),
                }
                self.remove(&t.id)
            }
            SyncAction::KeepRemote(n) => {
                self.say(Tone::Warn, n.message.clone());
                Ok(())
            }
        }
    }
}

/// The id a sync action is about.
pub fn action_id(a: &SyncAction) -> &str {
    match a {
        SyncAction::Download(t) | SyncAction::DeleteRemote(t) => &t.id,
        SyncAction::Upload(u) => &u.id,
        SyncAction::Create(c) | SyncAction::RemoveLocal(c) => &c.id,
        SyncAction::UploadCopy(c) => &c.id,
        SyncAction::KeepLocal(n) | SyncAction::KeepRemote(n) => &n.id,
    }
}

/// How a failure ends a run.
fn ended(e: &ApiFailure, reason: Option<String>) -> SyncEnd {
    if e.status == 401 {
        SyncEnd::SignedOut
    } else if e.transient() {
        SyncEnd::Offline
    } else {
        SyncEnd::Failed(reason.unwrap_or_else(|| e.message.clone()))
    }
}

/// The account's copies on this device, as the plan reads them.
fn local_templates(records: &BTreeMap<String, DeviceRecord>) -> Vec<LocalTemplate> {
    records
        .values()
        .filter_map(|r| {
            let c = r.cloud.as_ref()?;
            Some(LocalTemplate {
                id: r.id.clone(),
                name: r.template.meta.name.clone(),
                base_revision: (c.revision > 0).then_some(c.revision),
                dirty: c.changed || c.revision == 0,
                deleted: c.deleted,
            })
        })
        .collect()
}

/// One sync run: the cloud's list, the plan, every action in turn. `busy` is
/// told when an action for a template starts and ends (the gallery's
/// “Eşitleniyor”). A run that cannot reach the server stops where it is
/// (`Offline`); what it did so far stays done.
pub async fn sync_once(
    cloud: &Cloud,
    personal: Option<Uuid>,
    me: &Account,
    lib: &dyn DeviceLibrary,
    busy: &(dyn Fn(&str, bool) + Send + Sync),
) -> SyncReport {
    let mut run = Run {
        cloud,
        personal,
        me,
        lib,
        report: SyncReport {
            end: SyncEnd::Synced,
            said: Vec::new(),
            created: Vec::new(),
            changed: false,
            again: false,
            organizations: None,
        },
    };
    let list = match api::list(cloud).await {
        Ok(l) => {
            run.report.organizations = Some(DeviceOrganization::of_list(&l));
            l.every().cloned().collect::<Vec<_>>()
        }
        Err(e) => {
            run.report.end = ended(&e, Some(texts::list_failed(&e.message)));
            return run.report;
        }
    };
    let records: BTreeMap<String, DeviceRecord> = lib
        .records()
        .into_iter()
        .filter(|r| r.cloud.as_ref().is_some_and(|c| c.account == me.id))
        .map(|r| (r.id.clone(), r))
        .collect();
    let summaries: BTreeMap<&str, &SheetTemplateSummary> =
        list.iter().map(|s| (s.id.as_str(), s)).collect();
    let remote: Vec<RemoteTemplate> = list
        .iter()
        .map(|s| RemoteTemplate {
            id: s.id.clone(),
            name: s.name.clone(),
            revision: s.revision,
            deleted: false,
        })
        .collect();
    let plan = plan_sync(&local_templates(&records), &remote);
    let acted: BTreeSet<&str> = plan.actions.iter().map(action_id).collect();
    for a in &plan.actions {
        let id = action_id(a).to_owned();
        busy(&id, true);
        let done = run.act(a, &records).await;
        busy(&id, false);
        let Err(e) = done else {
            continue;
        };
        if e.transient() || e.status == 401 {
            run.report.end = ended(&e, None);
            return run.report;
        }
        if e.status == 409 {
            run.report.again = true;
        } else {
            let name = records
                .get(&id)
                .map(|r| r.template.meta.name.clone())
                .or_else(|| summaries.get(id.as_str()).map(|s| s.name.clone()))
                .unwrap_or_else(|| id.clone());
            run.say(Tone::Error, texts::action_failed(&name, &e.message));
        }
    }
    // What the list says of the others (their role, owner, sharing) is kept with them; they are clean (else the plan acts).
    for r in records.values() {
        let (Some(s), Some(c)) = (summaries.get(r.id.as_str()), r.cloud.as_ref()) else {
            continue;
        };
        if c.deleted || c.changed || acted.contains(r.id.as_str()) {
            continue;
        }
        let next = DeviceCloudState::of_summary(&me.id, s, Some(c));
        if next.role != c.role
            || next.owner_name != c.owner_name
            || next.shared != c.shared
            || next.organization != c.organization
            || next.published_from != c.published_from
        {
            let saved = run.save(DeviceRecord {
                id: r.id.clone(),
                template: r.template.clone(),
                cloud: Some(next),
            });
            if let Err(e) = saved {
                run.say(
                    Tone::Error,
                    texts::action_failed(&r.template.meta.name, &e.message),
                );
            }
        }
    }
    run.report
}

/// The account's newest template event cursor: its pages read to the end, so
/// nothing between it and the sync that follows is missed.
pub async fn newest_cursor(cloud: &Cloud) -> Result<String, ApiFailure> {
    let mut after = "0".to_owned();
    loop {
        let page = api::events(cloud, &after, false).await?;
        let full = page.events.len() >= EVENTS_PAGE;
        after = page.next;
        if !full {
            return Ok(after);
        }
    }
}

/// Templates in memory: a device for the tests.
#[derive(Default)]
pub struct MemoryLibrary {
    records: std::sync::Mutex<BTreeMap<String, DeviceRecord>>,
}

impl MemoryLibrary {
    pub fn get(&self, id: &str) -> Option<DeviceRecord> {
        self.records.lock().ok()?.get(id).cloned()
    }
}

impl DeviceLibrary for MemoryLibrary {
    fn records(&self) -> Vec<DeviceRecord> {
        self.records
            .lock()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    fn save(&self, record: &DeviceRecord) -> Result<(), String> {
        let mut m = self.records.lock().map_err(|e| e.to_string())?;
        m.insert(record.id.clone(), record.clone());
        Ok(())
    }

    fn remove(&self, id: &str) -> Result<(), String> {
        let mut m = self.records.lock().map_err(|e| e.to_string())?;
        m.remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, cloud: Option<DeviceCloudState>) -> DeviceRecord {
        let mut t = kentos_sheet::template::system_template("sys:genel-a4-dikey")
            .cloned()
            .expect("a system template");
        t.meta.id = id.to_owned();
        t.meta.name = format!("Şablon {id}");
        DeviceRecord {
            id: id.to_owned(),
            template: t,
            cloud,
        }
    }

    #[test]
    fn a_key_is_the_same_for_the_same_request_only() {
        assert_eq!(key_of("a"), key_of("a"));
        assert_ne!(key_of("a"), key_of("b"));
        // Long enough for the server (8–200 letters).
        assert_eq!(key_of("a").to_string().len(), 36);
    }

    #[test]
    fn the_plan_reads_the_account_s_copies_as_the_web_does() {
        let up = DeviceCloudState::to_upload("ayse");
        let mut synced = DeviceCloudState::to_upload("ayse");
        synced.revision = 3;
        synced.changed = false;
        let mut changed = synced.clone();
        changed.changed = true;
        let records: BTreeMap<String, DeviceRecord> = [
            record("a", Some(up)),
            record("b", Some(synced)),
            record("c", Some(changed)),
            record("d", None),
        ]
        .into_iter()
        .map(|r| (r.id.clone(), r))
        .collect();
        let local = local_templates(&records);
        let of = |id: &str| {
            local
                .iter()
                .find(|l| l.id == id)
                .map(|l| (l.base_revision, l.dirty))
        };
        // Not uploaded yet: no base, dirty; synced: its base, clean; changed: its base, dirty; this device's: not planned.
        assert_eq!(of("a"), Some((None, true)));
        assert_eq!(of("b"), Some((Some(3), false)));
        assert_eq!(of("c"), Some((Some(3), true)));
        assert_eq!(of("d"), None);
    }

    #[test]
    fn a_memory_device_keeps_and_forgets() {
        let lib = MemoryLibrary::default();
        lib.save(&record("x", None)).expect("saved");
        assert_eq!(lib.records().len(), 1);
        assert!(lib.get("x").is_some());
        lib.remove("x").expect("removed");
        assert!(lib.records().is_empty());
    }
}
