//! The template library's server API (design §13): request and response
//! types the web, the desktop and the server share, generated to
//! TypeScript like the rest. Reading: `GET /v1/me/sheet-templates`,
//! `GET /v1/sheet-templates/{id}` (its `ETag` is the revision),
//! `GET …/{id}/access`, `GET …/{id}/access/candidates?q=`. Changing: the
//! product commands `sheet.template.create`, `.update`, `.delete`, `.share`,
//! `.unshare`, `.publish` (v1) through `POST /v1/tenants/{tenant}/commands`,
//! the tenant being the caller's personal space, or an organisation for its
//! library (design §13 “Kurum şablonları”: its owner, administrators and the
//! members who may create projects publish; the publisher and the
//! administrators edit and delete; every other active member with a seat
//! uses). The event `sheet_template.changed` reaches the owner and everyone
//! it is shared with, and for an organisation's template its members:
//! `GET /v1/me/sheet-templates/events?after=&wait=`, a cursor and a long poll
//! as a project's events (docs/adr/0044).
//!
//! With the `schema` feature the commands' input and output have JSON
//! Schemas and [`catalog_entries`] describes them as product commands
//! (docs/adr/0013); the server holds its handlers to that list.

use kentos_contracts::{ProjectType, Workspace};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use crate::template::{PaperChoice, Template};

#[cfg(feature = "schema")]
use schemars::JsonSchema;

/// The most a template's content may be, its pictures included.
pub const TEMPLATE_CONTENT_MAX: u32 = 8 * 1024 * 1024;
pub const TEMPLATE_NAME_MAX: usize = 120;
/// Templates a person may own.
pub const TEMPLATES_PER_PERSON_MAX: u32 = 500;

pub const COMMAND_CREATE: &str = "sheet.template.create";
pub const COMMAND_UPDATE: &str = "sheet.template.update";
pub const COMMAND_DELETE: &str = "sheet.template.delete";
pub const COMMAND_SHARE: &str = "sheet.template.share";
pub const COMMAND_UNSHARE: &str = "sheet.template.unshare";
pub const COMMAND_PUBLISH: &str = "sheet.template.publish";
pub const EVENT_CHANGED: &str = "sheet_template.changed";

/// What the caller may do with a template.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum TemplateRole {
    /// Its own (an organisation's template: the one who published it).
    Owner,
    /// An organisation's owner or administrator: edits and deletes the organisation's templates.
    Admin,
    /// Saves new revisions.
    Editor,
    /// Uses and duplicates it.
    Viewer,
}

/// An organisation a template's library is in: its id and name (the gallery's “Kurum: <ad>”).
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct CloudOrganization {
    pub tenant_id: String,
    pub name: String,
}

/// What sharing gives (ownership is not given in phase 1).
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum TemplateGrantRole {
    Viewer,
    Editor,
}

/// A template in a list: its metadata and the caller's role, without its content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub papers: Vec<PaperChoice>,
    pub workspaces: Vec<Workspace>,
    pub project_types: Vec<ProjectType>,
    pub revision: u32,
    /// Of the content (hex).
    pub sha256: String,
    /// The content's bytes.
    pub size: u32,
    pub role: TemplateRole,
    pub owner_id: String,
    /// Empty when the caller cannot see the owner.
    pub owner_name: String,
    /// The owner has shared it with someone.
    pub shared: bool,
    /// RFC 3339.
    pub created_at: String,
    pub updated_at: String,
    /// The organisation whose library it is in; none: a person's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub organization: Option<CloudOrganization>,
    /// The template it was published from (“Kuruma yayımla…”), when it was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub published_from: Option<String>,
}

/// An organisation's template library as the caller sees it (design §13 “Kurum şablonları”).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct OrganizationTemplates {
    pub tenant_id: String,
    pub name: String,
    /// The caller may publish into it: its owner, administrators and those who may create projects.
    pub can_publish: bool,
    pub templates: Vec<SheetTemplateSummary>,
}

/// An organisation whose library the account sees, as a device keeps it for its gallery (the
/// list's `organizations` without their templates): the “Kurumum” groups, and where “Kuruma
/// yayımla…” may go.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceOrganization {
    pub tenant_id: String,
    pub name: String,
    pub can_publish: bool,
}

impl DeviceOrganization {
    /// The organisations of a list, in its order (by name).
    pub fn of_list(list: &SheetTemplateList) -> Vec<DeviceOrganization> {
        list.organizations
            .iter()
            .map(|o| DeviceOrganization {
                tenant_id: o.tenant_id.clone(),
                name: o.name.clone(),
                can_publish: o.can_publish,
            })
            .collect()
    }
}

impl SheetTemplateList {
    /// Every template of the list: the account's own and those shared with it, then each
    /// organisation's library (a sync plans them all alike).
    pub fn every(&self) -> impl Iterator<Item = &SheetTemplateSummary> {
        self.templates
            .iter()
            .chain(self.organizations.iter().flat_map(|o| o.templates.iter()))
    }
}

/// `GET /v1/me/sheet-templates`: the caller's own and those shared with them; and the libraries
/// of the organisations the caller is an active member of, with a seat.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateList {
    pub templates: Vec<SheetTemplateSummary>,
    #[serde(default)]
    pub organizations: Vec<OrganizationTemplates>,
}

/// `GET /v1/sheet-templates/{id}`: the metadata and the latest content.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateDetail {
    pub summary: SheetTemplateSummary,
    pub content: Template,
}

/// Input of `sheet.template.create` v1: the metadata the catalogue shows comes from `content.meta`.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateCreate {
    /// The template file (`kentos.sheet.template/1`); the server gives it its id and revision.
    #[cfg_attr(feature = "schema", schemars(with = "serde_json::Value"))]
    pub content: Template,
}

/// Input of `sheet.template.update` v1: a new revision based on `expectedRevision` (a later one is a conflict).
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateUpdate {
    pub template_id: String,
    pub expected_revision: u32,
    /// The template file (`kentos.sheet.template/1`).
    #[cfg_attr(feature = "schema", schemars(with = "serde_json::Value"))]
    pub content: Template,
}

/// Input of `sheet.template.delete` v1.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateDelete {
    pub template_id: String,
    /// The revision the caller deletes; a later one is a conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub expected_revision: Option<u32>,
}

/// Input of `sheet.template.share` v1: only the owner shares, only with people of a common organisation (ADR 0024).
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateShare {
    pub template_id: String,
    pub user_id: String,
    pub role: TemplateGrantRole,
}

/// Input of `sheet.template.unshare` v1: the template leaves the person's “Benimle paylaşılanlar”; sheets made from it stay.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateUnshare {
    pub template_id: String,
    pub user_id: String,
}

/// Input of `sheet.template.publish` v1, sent to the organisation's command route: one's own
/// template copied into the organisation's library as a template of its own (a new id, revision
/// 1), its source kept (`publishedFrom`). Only those who may publish there do it.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplatePublish {
    /// The template to publish: the caller's own.
    pub template_id: String,
    /// The organisation: the command route's own.
    pub tenant_id: String,
}

/// What a command did; a retry with the same idempotency key gets the same one (`replayed`). Also the event's body.
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateChanged {
    pub template_id: String,
    pub revision: u32,
    #[serde(default)]
    pub deleted: bool,
    /// Whether anything changed (sharing the same role again does not).
    #[serde(default)]
    pub changed: bool,
    #[serde(default)]
    pub replayed: bool,
}

/// A person a template is shared with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateGrant {
    pub user_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub email: Option<String>,
    pub role: TemplateGrantRole,
    pub granted_by_name: String,
    /// RFC 3339.
    pub created_at: String,
}

/// `GET …/{id}/access` (the owner only).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateAccess {
    pub owner_id: String,
    pub owner_name: String,
    pub grants: Vec<SheetTemplateGrant>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateCandidate {
    pub user_id: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub email: Option<String>,
}

/// `GET …/{id}/access/candidates?q=`: active members of the caller's organisations whose name or e-mail holds every word (no open e-mail search).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateCandidates {
    pub candidates: Vec<SheetTemplateCandidate>,
}

/// A template's place in an account's cloud library, as a device keeps it
/// beside its copy (design §13; the web's `TemplateCloudState`): what the
/// sync plan reads (`LocalTemplate`) and what the gallery's badges say. A
/// copy of the cloud's is the device's cache of its last downloaded revision
/// (used offline); one waiting to be uploaded has revision 0.
///
/// The desktop keeps it in its template files (`kentos-sheet-ui`'s store);
/// the web keeps the same fields in IndexedDB (`TemplateCloudState`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceCloudState {
    /// The account whose library it is in (another account signed in on this device neither sees nor syncs it).
    pub account: String,
    /// The cloud revision this copy is based on; 0: not uploaded yet.
    pub revision: u32,
    /// Changed here since that revision.
    pub changed: bool,
    /// Deleted here: kept until the cloud hears of it.
    #[serde(default)]
    pub deleted: bool,
    /// The account's role in it: its own (`owner`), shared with it, or in an organisation's library.
    pub role: TemplateRole,
    /// Who owns it, for one shared with the account (an organisation's: who published it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_name: Option<String>,
    /// The organisation whose library it is in; none: a person's. Its commands go to that
    /// organisation's command route.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<CloudOrganization>,
    /// An organisation's template published from one of the account's: that template's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_from: Option<String>,
    /// Its owner shared it with someone.
    #[serde(default)]
    pub shared: bool,
    /// A copy a conflict kept (“… (bu cihazdaki kopya)”): the template it was a copy of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_of: Option<String>,
    /// The ids it had on this device before the cloud gave it its own (sheets made from it name the old one).
    #[serde(default)]
    pub former_ids: Vec<String>,
    /// Its revision on this device when the cloud gave it its id (the cloud's numbering starts again at 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub former_revision: Option<u32>,
}

impl DeviceCloudState {
    /// A template of this device going up: the account's own, not uploaded yet.
    pub fn to_upload(account: &str) -> DeviceCloudState {
        DeviceCloudState {
            account: account.to_owned(),
            revision: 0,
            changed: true,
            deleted: false,
            role: TemplateRole::Owner,
            owner_name: None,
            organization: None,
            published_from: None,
            shared: false,
            conflict_of: None,
            former_ids: Vec::new(),
            former_revision: None,
        }
    }

    /// The copy's state after the cloud's summary of it, keeping what only this device knows (its old ids, a conflict).
    pub fn of_summary(
        account: &str,
        s: &SheetTemplateSummary,
        prev: Option<&DeviceCloudState>,
    ) -> DeviceCloudState {
        DeviceCloudState {
            account: account.to_owned(),
            revision: s.revision,
            changed: false,
            deleted: false,
            role: s.role,
            owner_name: (s.role != TemplateRole::Owner && !s.owner_name.is_empty())
                .then(|| s.owner_name.clone()),
            organization: s.organization.clone(),
            published_from: s.published_from.clone(),
            shared: s.shared,
            conflict_of: prev.and_then(|p| p.conflict_of.clone()),
            former_ids: prev.map(|p| p.former_ids.clone()).unwrap_or_default(),
            former_revision: prev.and_then(|p| p.former_revision),
        }
    }
}

/// What happened to a template, as the people who hear it are told.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum SheetTemplateEventKind {
    Created,
    /// A new revision.
    Updated,
    Deleted,
    /// Shared with someone, or their role changed.
    Shared,
    /// A share taken away (the person it was taken from hears it too).
    Unshared,
}

/// `sheet_template.changed`: one change, for one of the people who should hear it. It names the template and revision, never content or people.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateEvent {
    /// The cursor (decimal text).
    pub seq: String,
    pub template_id: String,
    pub revision: u32,
    pub kind: SheetTemplateEventKind,
    /// Who made the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub actor: Option<String>,
}

/// `GET /v1/me/sheet-templates/events?after=&limit=&wait=`: the caller's events after a cursor; `next` is where to go on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct SheetTemplateEventPage {
    pub events: Vec<SheetTemplateEvent>,
    pub next: String,
}

/// The version of every template command.
pub const COMMAND_VERSION: u32 = 1;

/// The template commands as product commands (docs/adr/0013): their names,
/// what they need and do, and their input and output schemas derived from
/// the types above. The server runs exactly these.
#[cfg(feature = "schema")]
pub fn catalog_entries() -> Vec<kentos_contracts::CommandDescriptor> {
    use kentos_contracts::{
        CommandCost, CommandDescriptor, CommandEffect, CommandExample, CommandHost,
        CommandRequirement, CommandUndo,
    };
    fn schema<T: JsonSchema>() -> serde_json::Value {
        schemars::schema_for!(T).to_value()
    }
    let entry = |id: &str,
                 title: &str,
                 summary: &str,
                 input: serde_json::Value,
                 example: serde_json::Value| CommandDescriptor {
        id: id.into(),
        version: COMMAND_VERSION,
        title: title.into(),
        summary: summary.into(),
        aliases: Vec::new(),
        effect: CommandEffect::Project,
        hosts: vec![CommandHost::Server],
        headless: true,
        requires: vec![CommandRequirement::SignedIn],
        permissions: Vec::new(),
        undo: CommandUndo::None,
        cost: CommandCost::Interactive,
        input,
        output: schema::<SheetTemplateChanged>(),
        plan: None,
        examples: vec![CommandExample {
            title: title.into(),
            input: example,
            output: None,
        }],
    };
    let id = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f";
    vec![
        entry(
            COMMAND_CREATE,
            "Pafta şablonunu buluta kaydet",
            "Bir pafta şablonunu kişisel alanınıza ya da bir kurumun şablon kitaplığına ilk revizyonuyla kaydeder; kimliğini sunucu verir. İçerik resimleriyle en çok 8 MB, ad en çok 120 karakter; kişi başına en çok 500 şablon. \
             Komut şablonun gideceği alanın komut yoluna gönderilir: kişisel alan ya da kurum. Kurumda yalnız yayımlayabilenler kaydeder: kurum sahibi, yöneticiler ve proje açabilen üyeler. Aynı idempotency anahtarıyla tekrar aynı sonucu verir.",
            schema::<SheetTemplateCreate>(),
            serde_json::json!({ "content": { "schema": "kentos.sheet.template/1", "meta": { "id": "u:buro-paftasi", "name": "Büromun paftası" } } }),
        ),
        entry(
            COMMAND_UPDATE,
            "Pafta şablonunun yeni revizyonu",
            "Şablonun yeni revizyonunu kaydeder; sahibi ya da düzenleyici paylaşımı olan, kurum şablonunda yayımlayan ve kurum yöneticileri yapabilir. Kurum şablonunun komutu kurumun komut yoluna gönderilir. expectedRevision şablonun şimdiki revizyonu değilse 409 döner, hiçbir şey yazılmaz.",
            schema::<SheetTemplateUpdate>(),
            serde_json::json!({ "templateId": id, "expectedRevision": 3, "content": {} }),
        ),
        entry(
            COMMAND_DELETE,
            "Pafta şablonunu sil",
            "Şablonu herkesin listesinden kaldırır; yalnız sahibi yapabilir, kurum şablonunu yayımlayan ve kurum yöneticileri. expectedRevision verilmişse ve şablon o arada değişmişse 409 döner. Şablondan yapılmış paftalar kalır.",
            schema::<SheetTemplateDelete>(),
            serde_json::json!({ "templateId": id, "expectedRevision": 3 }),
        ),
        entry(
            COMMAND_SHARE,
            "Pafta şablonunu paylaş",
            "Şablonu bir kişiyle görüntüleyici ya da düzenleyici olarak paylaşır, ya da rolünü değiştirir; yalnız sahibi yapabilir ve yalnız ortak bir kurumunun etkin üyesiyle (ADR 0024). Aynı rol yeniden verilirse bir şey değişmez. Kurum şablonu tek tek paylaşılmaz: kurumun etkin, koltuklu bütün üyeleri onu görür.",
            schema::<SheetTemplateShare>(),
            serde_json::json!({ "templateId": id, "userId": "01925f3e-0000-7d2b-9e4f-0a1b2c3d4e5f", "role": "editor" }),
        ),
        entry(
            COMMAND_UNSHARE,
            "Pafta şablonunun paylaşımını kaldır",
            "Kişinin paylaşımını kaldırır; şablon onun “Benimle paylaşılanlar” listesinden çıkar, ondan yapılmış paftalar kalır. Yalnız sahibi yapabilir.",
            schema::<SheetTemplateUnshare>(),
            serde_json::json!({ "templateId": id, "userId": "01925f3e-0000-7d2b-9e4f-0a1b2c3d4e5f" }),
        ),
        CommandDescriptor {
            permissions: vec!["project.create".into()],
            ..entry(
                COMMAND_PUBLISH,
                "Pafta şablonunu kuruma yayımla",
                "Kendi pafta şablonunuzu bir kurumun şablon kitaplığına kopyalar: yeni kimlik, 1. revizyon, bulutun son revizyonunun içeriği; kaynağı publishedFrom olarak kalır. \
                 Kurumun komut yoluna gönderilir (tenantId o kurum); kurum sahibi, yöneticiler ve proje açabilen üyeler yapabilir. Kurumun etkin, koltuklu bütün üyeleri görür ve kullanır; yayımlayan ve kurum yöneticileri düzenler ve siler.",
                schema::<SheetTemplatePublish>(),
                serde_json::json!({ "templateId": id, "tenantId": "01925f3e-1111-7d2b-9e4f-0a1b2c3d4e5f" }),
            )
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(role: TemplateRole, shared: bool) -> SheetTemplateSummary {
        SheetTemplateSummary {
            id: "t".into(),
            name: "Belediye ifraz paftası".into(),
            description: String::new(),
            category: "kadastro".into(),
            tags: Vec::new(),
            papers: Vec::new(),
            workspaces: Vec::new(),
            project_types: Vec::new(),
            revision: 4,
            sha256: String::new(),
            size: 0,
            role,
            owner_id: "ayse".into(),
            owner_name: "Ayşe Yılmaz".into(),
            shared,
            created_at: String::new(),
            updated_at: String::new(),
            organization: None,
            published_from: None,
        }
    }

    /// A device's copy takes the cloud's revision, role and sharing, and keeps what only it knows.
    #[test]
    fn a_device_s_copy_follows_the_cloud_and_keeps_its_own() {
        let mut prev = DeviceCloudState::to_upload("bora");
        prev.conflict_of = Some("x".into());
        prev.former_ids = vec!["u-1".into()];
        prev.former_revision = Some(3);
        let c = DeviceCloudState::of_summary(
            "bora",
            &summary(TemplateRole::Editor, false),
            Some(&prev),
        );
        assert_eq!(
            (c.revision, c.changed, c.role),
            (4, false, TemplateRole::Editor)
        );
        assert_eq!(c.owner_name.as_deref(), Some("Ayşe Yılmaz"));
        assert_eq!(
            (
                c.conflict_of.as_deref(),
                c.former_ids.as_slice(),
                c.former_revision
            ),
            (Some("x"), ["u-1".to_owned()].as_slice(), Some(3))
        );
        // The account's own: no owner's name; shared with someone, said.
        let own = DeviceCloudState::of_summary("ayse", &summary(TemplateRole::Owner, true), None);
        assert_eq!((own.owner_name, own.shared), (None, true));
        // Kept as the web keeps it (camelCase), read back the same; an unknown field is refused.
        let json = serde_json::to_value(&c).expect("written");
        assert_eq!(json["ownerName"], "Ayşe Yılmaz");
        assert_eq!(json["conflictOf"], "x");
        assert_eq!(
            serde_json::from_value::<DeviceCloudState>(json).expect("read"),
            c
        );
        assert!(
            serde_json::from_str::<DeviceCloudState>(
                r#"{"account":"a","revision":0,"changed":true,"role":"owner","extra":1}"#
            )
            .is_err()
        );
    }

    /// An organisation's template: the copy knows the organisation (its commands go there) and
    /// the template it was published from; a list without organisations reads as none.
    #[test]
    fn an_organisation_s_template_keeps_its_organisation() {
        let mut s = summary(TemplateRole::Viewer, false);
        s.organization = Some(CloudOrganization {
            tenant_id: "buro".into(),
            name: "Harita Bürosu".into(),
        });
        s.published_from = Some("ayse-1".into());
        let c = DeviceCloudState::of_summary("bora", &s, None);
        assert_eq!(
            c.organization.as_ref().map(|o| o.name.as_str()),
            Some("Harita Bürosu")
        );
        assert_eq!(c.published_from.as_deref(), Some("ayse-1"));
        // Who published it is named to the others; the publisher's own copy names nobody.
        assert_eq!(c.owner_name.as_deref(), Some("Ayşe Yılmaz"));
        s.role = TemplateRole::Owner;
        assert_eq!(
            DeviceCloudState::of_summary("ayse", &s, None).owner_name,
            None
        );
        let json = serde_json::to_value(&c).expect("written");
        assert_eq!(json["organization"]["tenantId"], "buro");
        assert_eq!(json["publishedFrom"], "ayse-1");
        assert_eq!(
            serde_json::from_value::<DeviceCloudState>(json).expect("read"),
            c
        );
        // A person's template writes neither field.
        let own = serde_json::to_value(DeviceCloudState::to_upload("ayse")).expect("written");
        assert!(own.get("organization").is_none() && own.get("publishedFrom").is_none());
        let list: SheetTemplateList = serde_json::from_str(r#"{"templates":[]}"#).expect("read");
        assert!(list.organizations.is_empty());
        assert_eq!(
            serde_json::to_value(TemplateRole::Admin).expect("written"),
            "admin"
        );
    }
}
