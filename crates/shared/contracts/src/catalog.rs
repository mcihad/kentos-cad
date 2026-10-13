//! The product command catalog (docs/adr/0013): the typed, versioned
//! operations every host reaches the same way (the web app, the desktop app,
//! the server, the CLI, Python and AI). The web app's interface commands
//! (`core/commands.ts`: open a window, pick a tool) are not here; one may end
//! in a product command.
//!
//! Input and output schemas are JSON Schema derived from the contract types
//! themselves (`schemars`, the `schema` feature), so a type and its schema
//! cannot drift. A command is listed only once a host handles it.
//!
//! `cargo test -p kentos-contracts` compares the catalog with
//! `apps/web/src/contracts/generated/commandCatalog.json` and fails on a
//! difference; after a deliberate change, `KENTOS_WRITE_CATALOG=1` rewrites
//! the file (read the diff before committing).

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub const CATALOG_FORMAT: &str = "kentos.commands";
pub const CATALOG_VERSION: u32 = 1;

/// Every product command, as `catalog()` builds it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandCatalog {
    #[cfg_attr(feature = "ts", ts(type = "\"kentos.commands\""))]
    pub format: String,
    #[cfg_attr(feature = "ts", ts(type = "1"))]
    pub version: u32,
    pub commands: Vec<CommandDescriptor>,
    /// What every code of the commands' answers means (docs/adr/0253 §7).
    #[serde(default)]
    pub codes: Vec<CatalogCode>,
    /// The codes any call may answer with, whatever its command (the protocol's).
    #[serde(default)]
    pub protocol: Vec<String>,
    /// What the schemas' `x-unit`s mean (docs/adr/0256 §3): every number of
    /// every command's input, output and plan names one.
    #[serde(default)]
    pub units: Vec<CatalogUnit>,
}

/// A unit the schemas' `x-unit` names and what it means.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogUnit {
    pub unit: String,
    /// Turkish, for people and for an agent reading a number.
    pub meaning: String,
}

/// A code of the commands' answers and what it means (`codes::CODES`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CatalogCode {
    pub code: String,
    /// `failed`, `conflict`, `needsInput`, `warning`, `failedOrWarning`, or
    /// `host`: the host's own failure, not a command's answer.
    pub kind: String,
    /// Whether calling again may succeed (a conflict's).
    pub retryable: bool,
    /// Turkish.
    pub meaning: String,
}

/// One product command (TODOS.md CMD-01).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandDescriptor {
    /// Stable name `alan.nesne.eylem`, lowercase ASCII (`cad.polygon.create`); never reused.
    pub id: String,
    /// Version of the input and output schema; an incompatible change is a new version.
    pub version: u32,
    /// Turkish name (interface, help, AI tool title).
    pub title: String,
    /// One or two Turkish sentences: what it does and what it needs.
    pub summary: String,
    /// Command line and search names.
    pub aliases: Vec<String>,
    pub effect: CommandEffect,
    /// Where a handler exists.
    pub hosts: Vec<CommandHost>,
    /// Runs from explicit input alone, without an interface or its implicit state (CMD-07).
    pub headless: bool,
    pub requires: Vec<CommandRequirement>,
    /// Project permissions it needs (docs/adr/0015); `summary` says which applies when.
    pub permissions: Vec<String>,
    pub undo: CommandUndo,
    pub cost: CommandCost,
    /// JSON Schema of the input.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
    /// JSON Schema of the result.
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub output: serde_json::Value,
    /// JSON Schema of what `plan` answers: the change it would make, with
    /// nothing written (TODOS.md CMD-04). Absent for a command only the
    /// server runs: the server executes, it does not plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub plan: Option<serde_json::Value>,
    /// The object kinds it takes, the codes it refuses with and warns with
    /// (docs/adr/0253 §7); absent for a command only the server runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub capabilities: Option<crate::capabilities::CommandCapabilities>,
    pub examples: Vec<CommandExample>,
}

/// What a command changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandEffect {
    /// Nothing.
    Query,
    /// Only the local session: camera, selection, active tool.
    View,
    /// The open document; undoable.
    Document,
    /// Project state on the server: saving, sharing, cloud commits.
    Project,
    /// Tenants and users.
    Admin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandHost {
    Web,
    Desktop,
    Server,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandRequirement {
    /// An open document.
    Document,
    /// An open cloud project.
    CloudProject,
    /// A signed-in account.
    SignedIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandUndo {
    None,
    /// One local undo step.
    Step,
    /// Undone in the cloud by an inverse command, checked like any other (TX-03).
    Inverse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", ts(export))]
pub enum CommandCost {
    Instant,
    /// The user waits for it.
    Interactive,
    /// Queued; the answer is a job id (202).
    Job,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandExample {
    pub title: String,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
    pub output: Option<serde_json::Value>,
}

#[cfg(feature = "schema")]
fn schema<T: schemars::JsonSchema>() -> serde_json::Value {
    schemars::schema_for!(T).to_value()
}

/// The catalog, with schemas derived from the contract types.
#[cfg(feature = "schema")]
pub fn catalog() -> CommandCatalog {
    use crate::{
        ArcCreate, ArcCreated, CAD_ARC_CREATE, CAD_ARC_CREATE_VERSION, CAD_CIRCLE_CREATE,
        CAD_CIRCLE_CREATE_VERSION, CAD_ENTITIES_ARRAY, CAD_ENTITIES_ARRAY_VERSION,
        CAD_ENTITIES_DELETE, CAD_ENTITIES_DELETE_VERSION, CAD_ENTITIES_EDIT,
        CAD_ENTITIES_EDIT_VERSION, CAD_ENTITIES_TRANSFORM, CAD_ENTITIES_TRANSFORM_VERSION,
        CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION, CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION,
        CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION, CAD_POLYLINE_CREATE,
        CAD_POLYLINE_CREATE_VERSION, CircleCreate, CircleCreated, CommitResult, EntitiesArray,
        EntitiesArrayed, EntitiesDelete, EntitiesDeleted, EntitiesEdit, EntitiesEdited,
        EntitiesTransform, EntitiesTransformed, LineCreate, LineCreated, PROJECT_ACCESS_REVOKE,
        PROJECT_ACCESS_REVOKE_VERSION, PROJECT_CHANGES, PROJECT_CHANGES_VERSION, PROJECT_SHARE,
        PROJECT_SHARE_VERSION, PointCreate, PointCreated, PolygonCreate, PolygonCreated,
        PolylineCreate, PolylineCreated, ProjectAccessChange, ProjectAccessRevoke, ProjectChanges,
        ProjectPermission, ProjectShare,
    };
    use serde_json::json;
    let mut commands = vec![CommandDescriptor {
            id: PROJECT_CHANGES.into(),
            version: PROJECT_CHANGES_VERSION,
            title: "Proje değişiklikleri".into(),
            summary: "Bir bulut projesine nesne ekler, değiştirir, siler ve proje bilgisini günceller; hepsi tek işlemde yazılır ya da hiçbiri yazılmaz. \
                      Her değişen nesnenin ve proje bilgisinin beklenen sürümü zarfın expectedVersions alanındadır; biri değişmişse 409 döner, hiçbir şey yazılmaz. \
                      Nesneler feature.write, proje bilgisi project.edit ister. Aynı idempotency anahtarıyla tekrar aynı sonucu verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![
                ProjectPermission::FeatureWrite.name().into(),
                ProjectPermission::Edit.name().into(),
            ],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<ProjectChanges>(),
            output: schema::<CommitResult>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Yeni bir nokta ekle".into(),
                input: json!({
                    "features": [{
                        "op": "create",
                        "id": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "entity": {
                            "kind": "point",
                            "id": 1,
                            "layerId": "kot",
                            "attrs": { "ad": "P1" },
                            "p": { "x": 423510.25, "y": 4512300.5 }
                        }
                    }]
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: PROJECT_SHARE.into(),
            version: PROJECT_SHARE_VERSION,
            title: "Projeyi paylaş".into(),
            summary: "Bir kişiye projede rol verir ya da rolünü değiştirir: görüntüleyici, yorumcu, düzenleyici ya da yönetici; istenirse bir bitiş zamanıyla. \
                      Kurum projesinde kişi kurumun üyesi olmalıdır. project.share ister; değişiklik denetime ve projenin olay günlüğüne yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Share.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<ProjectShare>(),
            output: schema::<ProjectAccessChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir meslektaşı düzenleyici yap".into(),
                input: json!({ "userId": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "role": "editor" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: PROJECT_ACCESS_REVOKE.into(),
            version: PROJECT_ACCESS_REVOKE_VERSION,
            title: "Proje erişimini kaldır".into(),
            summary: "Bir kişinin projedeki paylaşımını kaldırır; açık bağlantıları kapanır ve sonraki istekleri reddedilir. \
                      Önceden indirilmiş kopyalar geri alınamaz. project.share ister."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Share.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<ProjectAccessRevoke>(),
            output: schema::<ProjectAccessChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Paylaşımı kaldır".into(),
                input: json!({ "userId": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f" }),
                output: None,
            }],
        },
        // The project catalog and lifecycle (docs/adr/0028). Every one runs on the
        // server under the caller's access to the project (docs/adr/0015), writes its
        // audit record and answers a retry with the same idempotency key once.
        CommandDescriptor {
            id: crate::PROJECT_CREATE.into(),
            version: crate::PROJECT_CREATE_VERSION,
            title: "Proje oluştur".into(),
            summary: "Bir çalışma alanında (kişisel alan ya da kurum) boş bir bulut projesi açar: adı, ayarları, katman ağacı ve stilleriyle; isteğe bağlı açıklama, tür ve etiketlerle. \
                      Proje açanındır; başkaları paylaşımla eklenir. Çalışma alanında project.create hakkı ister (kurumda proje yöneticisi ve üstü); projeye bağlı bir izin istemez. \
                      Zarfın tenantId'si çalışma alanıdır, projectId boş kalır. Aynı idempotency anahtarıyla tekrar aynı projeyi verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProjectCreate>(),
            output: schema::<crate::ProjectInfo>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Boş bir ifraz projesi aç".into(),
                input: json!({
                    "name": "Ada 101 ifrazı",
                    "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000 },
                    "origin": { "x": 486500.0, "y": 4420200.0 },
                    "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                                 "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
                    "activeLayer": "cizim",
                    "styles": { "items": [], "categories": [] },
                    "projectType": "subdivision",
                    "tags": ["Kadıköy", "2026"]
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_RENAME.into(),
            version: crate::PROJECT_RENAME_VERSION,
            title: "Projeyi yeniden adlandır".into(),
            summary: "Projenin adını erişimi olan herkes için değiştirir; açık editörler yeni adı hemen görür. project.edit ister. \
                      Arşivlenmiş ya da çöp kutusundaki proje yeniden adlandırılamaz. expectedVersions[\"@catalog\"] verilmişse ve proje bilgileri o sürümde değilse hiçbir şey yazılmaz (409)."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Edit.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProjectRename>(),
            output: schema::<crate::ProjectCatalogChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Projeye yeni bir ad ver".into(),
                input: json!({ "name": "Ada 101 (revize)" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_METADATA_UPDATE.into(),
            version: crate::PROJECT_METADATA_UPDATE_VERSION,
            title: "Proje bilgilerini değiştir".into(),
            summary: "Projenin katalog bilgilerini değiştirir: ad, açıklama, tür ve etiketler; verilmeyen alan olduğu gibi kalır. \
                      Tür yalnız katalogda düzen içindir: mevzuata uygunluk ya da resmî onay anlamına gelmez, bir modül açmaz, saklama biçimini değiştirmez. \
                      project.edit ister; arşivlenmiş ya da çöp kutusundaki projede yapılamaz. expectedVersions[\"@catalog\"] verilmişse ve proje bilgileri o sürümde değilse hiçbir şey yazılmaz (409)."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Edit.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProjectMetadataUpdate>(),
            output: schema::<crate::ProjectCatalogChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Türü ve etiketleri değiştir, açıklama ekle".into(),
                input: json!({
                    "projectType": "landReadjustment",
                    "description": "Kadıköy 18. madde uygulaması; dağıtım cetveli ayrı dosyada.",
                    "tags": ["Kadıköy", "DOP %40"]
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_DUPLICATE.into(),
            version: crate::PROJECT_DUPLICATE_VERSION,
            title: "Projenin kopyasını oluştur".into(),
            summary: "Projeyi yeni kimlikli yeni bir projeye kopyalar: ayarlar, katman ağacı, stiller, açıklama, tür, etiketler ve bütün nesneler; her nesne kalıcı kimliğini korur. \
                      Geçmiş (komut günlüğü, olaylar, denetim kayıtları), paylaşımlar, favoriler ve arşiv durumu kopyalanmaz; kopya çağıranındır ve yalnız onun (ve kurum politikasının) erişimindedir. \
                      Kaynakta project.download, kopyanın gideceği çalışma alanında project.create ister. Çöp kutusundaki proje kopyalanamaz."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Download.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProjectDuplicate>(),
            output: schema::<crate::ProjectDuplicated>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kişisel alana bir kopya".into(),
                input: json!({ "name": "Ada 101 (deneme)", "tenantId": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_ARCHIVE.into(),
            version: crate::PROJECT_ARCHIVE_VERSION,
            title: "Projeyi arşivle".into(),
            summary: "Projeyi arşivler: salt okunur olur, Arşivlenmişler listesinde durur; açık editörlerin kaydı durur, gönderilmemiş değişiklikleri cihazlarında kalır. \
                      Paylaşımı değiştirilebilir, kopyası oluşturulabilir. project.edit ister; tersi project.unarchive'dır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Edit.name().into()],
            undo: CommandUndo::Inverse,
            cost: CommandCost::Interactive,
            input: schema::<crate::EmptyInput>(),
            output: schema::<crate::ProjectCatalogChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Biten projeyi arşivle".into(),
                input: json!({}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_UNARCHIVE.into(),
            version: crate::PROJECT_UNARCHIVE_VERSION,
            title: "Projeyi arşivden çıkar".into(),
            summary: "Arşivlenmiş projeyi yeniden düzenlenebilir yapar. Açık tutanlar projeyi yeniden açınca kaydetmeye başlar; cihazlarında saklanan değişiklikler geri gelir. \
                      project.edit ister; tersi project.archive'dır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Edit.name().into()],
            undo: CommandUndo::Inverse,
            cost: CommandCost::Interactive,
            input: schema::<crate::EmptyInput>(),
            output: schema::<crate::ProjectCatalogChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Arşivden çıkar".into(),
                input: json!({}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_TRASH.into(),
            version: crate::PROJECT_TRASH_VERSION,
            title: "Projeyi çöp kutusuna taşı".into(),
            summary: "Projeyi erişimi olan herkes için çöp kutusuna taşır: listelerden kalkar, açılamaz ve değiştirilemez (410); açık editörlerin kaydı durur. \
                      Hiçbir şey silinmez: proje sahibi ya da kurum yöneticisi saklama süresi dolana kadar geri yükleyebilir, sonra proje kalıcı olarak silinir. \
                      project.delete ister (proje sahibi; kurum politikası açıksa kurum sahibi ve yöneticisi); tersi project.restore'dur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Delete.name().into()],
            undo: CommandUndo::Inverse,
            cost: CommandCost::Interactive,
            input: schema::<crate::EmptyInput>(),
            output: schema::<crate::ProjectCatalogChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çöp kutusuna taşı".into(),
                input: json!({}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_RESTORE.into(),
            version: crate::PROJECT_RESTORE_VERSION,
            title: "Projeyi çöp kutusundan geri yükle".into(),
            summary: "Çöp kutusundaki projeyi her şeyiyle geri getirir: listelerde görünür ve açılır; arşivlenmişse arşivde kalır. \
                      project.delete ister; tersi project.trash'tir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Delete.name().into()],
            undo: CommandUndo::Inverse,
            cost: CommandCost::Interactive,
            input: schema::<crate::EmptyInput>(),
            output: schema::<crate::ProjectCatalogChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Geri yükle".into(),
                input: json!({}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_PURGE.into(),
            version: crate::PROJECT_PURGE_VERSION,
            title: "Projeyi kalıcı olarak sil".into(),
            summary: "Çöp kutusundaki projeyi nesneleri, paylaşımları, komut günlüğü ve olaylarıyla birlikte kalıcı olarak siler; geri alınamaz, denetim kaydı kalır. \
                      Açık onay ister: confirmName projenin şimdiki adıyla aynı olmalıdır. Önce çöp kutusuna taşınmamış proje silinmez. project.delete ister. \
                      Önceden indirilmiş kopyalar ve sunucu yedekleri bu komutla silinmez."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Delete.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProjectPurge>(),
            output: schema::<crate::ProjectPurged>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Adını yazarak kalıcı olarak sil".into(),
                input: json!({ "confirmName": "Ada 101 (eski)" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_FAVORITE.into(),
            version: crate::PROJECT_FAVORITE_VERSION,
            title: "Favorilere ekle ya da çıkar".into(),
            summary: "Projeyi çağıranın favorilerine ekler (favorite: true) ya da çıkarır; yalnız o kişinin listesini değiştirir, projeye ve başkalarına dokunmaz. project.read ister."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Read.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ProjectFavorite>(),
            output: schema::<crate::ProjectCatalogChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Favorilere ekle".into(),
                input: json!({ "favorite": true }),
                output: None,
            }],
        },
        // A file project's save (docs/adr/0031): the verified upload becomes the next revision.
        CommandDescriptor {
            id: crate::PROJECT_FILE_COMMIT.into(),
            version: crate::PROJECT_FILE_COMMIT_VERSION,
            title: "Dosya revizyonunu kaydet".into(),
            summary: "Dosya olarak saklanan projede doğrulanmış bir yüklemeyi (POST …/uploads, PUT …/uploads/{yükleme}) projenin yeni revizyonu yapar. \
                      expectedVersions[\"@file\"] dosyanın dayandığı revizyondur (ilk kayıtta \"0\"); proje o arada değiştiyse hiçbir şey yazılmaz, çakışma döner. \
                      Yetki kayıt anında yeniden sorulur. feature.write ister; revizyonlar değişmez, eski revizyon silinmez."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::FeatureWrite.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::FileCommit>(),
            output: schema::<crate::FileCommitted>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "İlk revizyonu kaydet".into(),
                input: json!({ "uploadId": "0199a1b2-3c4d-7e5f-8a9b-0c1d2e3f4a5b" }),
                output: None,
            }],
        },
        // Into the other storage mode, as a new project (docs/adr/0039).
        CommandDescriptor {
            id: crate::PROJECT_CONVERT.into(),
            version: crate::PROJECT_CONVERT_VERSION,
            title: "Öbür saklama biçimine dönüştür".into(),
            summary: "Projenin şimdiki hâlinden öbür saklama biçiminde yeni bir proje açar; kaynak değişmez. \
                      Dosya projesinin en yeni revizyonu veritabanı projesine aktarılır (PostGIS'e aktar; nesneler kalıcı kimlikleriyle); \
                      veritabanı projesinin tek anlık görüntüsü dosya projesinin 1. revizyonu olur. Yeni proje çağıranındır. \
                      Kaynakta project.download, hedef çalışma alanında proje açma hakkı ister."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Download.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProjectConvert>(),
            output: schema::<crate::ProjectDuplicated>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Dosya projesini PostGIS'e aktar".into(),
                input: json!({ "to": "database" }),
                output: None,
            }],
        },
        // A file's drawing into a new database project (docs/adr/0036).
        CommandDescriptor {
            id: crate::PROJECT_IMPORT.into(),
            version: crate::PROJECT_IMPORT_VERSION,
            title: "Dosyayı projeye aktar".into(),
            summary: "Doğrulanmış bir .kcad yüklemesini (POST …/uploads, PUT …/uploads/{yükleme}) henüz kaydı olmayan bir veritabanı projesine tek işlemde aktarır: \
                      dosyanın ayarları, katmanları, stilleri ve her nesne kalıcı kimliğiyle. Sunucunun almadığı ilk nesne bütün dosyayı yerini söyleyerek reddeder; \
                      o zaman hiçbir şey yazılmaz. feature.write ve project.edit ister."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![
                ProjectPermission::FeatureWrite.name().into(),
                ProjectPermission::Edit.name().into(),
            ],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProjectImport>(),
            output: schema::<crate::ProjectImported>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Yerel çizimi yeni projeye aktar".into(),
                input: json!({ "uploadId": "0199a1b2-3c4d-7e5f-8a9b-0c1d2e3f4a5b" }),
                output: None,
            }],
        },
        // Invitations by link (docs/adr/0035).
        CommandDescriptor {
            id: crate::PROJECT_INVITE.into(),
            version: crate::PROJECT_INVITE_VERSION,
            title: "Projeye davet et".into(),
            summary: "Bir e-posta adresine projeye davet bağlantısı açar: rol en çok düzenleyici, bitiş verilmezse 14 gün, en çok 90 gün. \
                      Belirteç yalnız ilk yanıtta döner; bağlantıyı davet eden iletir. Aynı adrese bekleyen davet yenisiyle değişir. \
                      Daveti yalnız o adresin doğrulanmış hesabı kabul eder; kurum dışından biri misafir olur. project.share ister."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Share.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProjectInvite>(),
            output: schema::<crate::InvitationChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Belediyedeki mühendisi görüntüleyici olarak davet et".into(),
                input: json!({ "email": "muhendis@belediye.gov.tr", "role": "viewer" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_INVITATION_REVOKE.into(),
            version: crate::PROJECT_INVITATION_REVOKE_VERSION,
            title: "Daveti geri al".into(),
            summary: "Bekleyen bir daveti geri alır; bağlantısı artık çalışmaz. Kabul edilmiş davetin verdiği erişim project.access.revoke ile kaldırılır. \
                      project.share ister."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::Share.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::InvitationRevoke>(),
            output: schema::<crate::InvitationChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bekleyen daveti geri al".into(),
                input: json!({ "invitationId": "0199a1b2-3c4d-7e5f-8a9b-0c1d2e3f4a5b" }),
                output: None,
            }],
        },
        // Checkpoints: named points of a project's history (docs/adr/0034).
        CommandDescriptor {
            id: crate::PROJECT_CHECKPOINT_CREATE.into(),
            version: crate::PROJECT_CHECKPOINT_CREATE_VERSION,
            title: "Kontrol noktası oluştur".into(),
            summary: "Projenin şimdiki hâline ad verir. Veritabanı projesinde o anın tek revizyonlu .kcad görüntüsü nesne deposunda saklanır; \
                      dosya projesinde bir revizyon (verilmezse en yenisi) adlandırılır, bir şey kopyalanmaz. feature.write ister; \
                      arşivdeki ya da çöpteki projede alınmaz."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::FeatureWrite.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::CheckpointCreate>(),
            output: schema::<crate::CheckpointChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Teslim öncesi".into(),
                input: json!({ "name": "Belediyeye teslim", "note": "Ada 101 ifraz dosyası" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_CHECKPOINT_DELETE.into(),
            version: crate::PROJECT_CHECKPOINT_DELETE_VERSION,
            title: "Kontrol noktasını sil".into(),
            summary: "Bir kontrol noktasını kaldırır: veritabanı projesinde saklanan görüntüsü de gider; dosya projesinde adlandırdığı revizyon kalır. \
                      Kontrol noktasını oluşturan kişi (feature.write ile) ya da project.edit sahibi siler."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![ProjectPermission::FeatureWrite.name().into()],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::CheckpointDelete>(),
            output: schema::<crate::CheckpointChange>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kontrol noktasını kaldır".into(),
                input: json!({ "checkpointId": "0199a1b2-3c4d-7e5f-8a9b-0c1d2e3f4a5b" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::PROJECT_CHECKPOINT_RESTORE.into(),
            version: crate::PROJECT_CHECKPOINT_RESTORE_VERSION,
            title: "Yeni proje olarak geri yükle".into(),
            summary: "Bir kontrol noktasından ya da dosya projesinin bir revizyonundan yeni bir proje açar; kaynak proje değişmez. \
                      Dosya projesinin noktası, o revizyonla başlayan bir dosya projesi olur; veritabanı projesinin kontrol noktası, \
                      dosyasından içe aktarılan bir veritabanı projesi olur (nesneler kalıcı kimlikleriyle). Yeni proje çağıranındır; \
                      geçmiş ve paylaşımlar gelmez. Kaynakta project.history ve project.download, hedef çalışma alanında proje açma hakkı ister."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Project,
            hosts: vec![CommandHost::Server],
            headless: true,
            requires: vec![CommandRequirement::SignedIn, CommandRequirement::CloudProject],
            permissions: vec![
                ProjectPermission::History.name().into(),
                ProjectPermission::Download.name().into(),
            ],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::CheckpointRestore>(),
            output: schema::<crate::ProjectDuplicated>(),
            plan: None,
            capabilities: None,
            examples: vec![CommandExample {
                title: "Teslim hâlinden yeni proje".into(),
                input: json!({ "checkpointId": "0199a1b2-3c4d-7e5f-8a9b-0c1d2e3f4a5b", "name": "Ada 101 (teslim)" }),
                output: None,
            }],
        },
        // The first document command (docs/adr/0022): the web's and the desktop's own
        // handlers, held together by fixtures/commands/v1/cad.polygon.create.json.
        CommandDescriptor {
            id: CAD_POLYGON_CREATE.into(),
            version: CAD_POLYGON_CREATE_VERSION,
            title: "Kapalı alan oluştur".into(),
            summary: "Açık çizimde verilen katmana köşeleri, isteğe bağlı yay değerleri ve delikleriyle bir kapalı alan ekler; tek geri alma adımıdır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<PolygonCreate>(),
            output: schema::<PolygonCreated>(),
            plan: Some(schema::<crate::PolygonPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dikdörtgen bir alan".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 },
                            { "x": 423500.0, "y": 4512312.5 }
                        ]
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Doğu kenarı yarım daire, ortasında delik olan renkli alan; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 },
                            { "x": 423500.0, "y": 4512312.5 }
                        ],
                        "bulges": [0.0, 1.0, 0.0, 0.0],
                        "holes": [{
                            "pts": [
                                { "x": 423505.0, "y": 4512304.0 },
                                { "x": 423509.0, "y": 4512304.0 },
                                { "x": 423509.0, "y": 4512308.0 },
                                { "x": 423505.0, "y": 4512308.0 }
                            ]
                        }],
                        "color": "#E5484D",
                        "attrs": { "Ad": "Avlu" },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // The line and polyline tools' commands (docs/adr/0027), held together by
        // fixtures/commands/v1/cad.line.create.json and cad.polyline.create.json.
        CommandDescriptor {
            id: CAD_LINE_CREATE.into(),
            version: CAD_LINE_CREATE_VERSION,
            title: "Çizgi oluştur".into(),
            summary: "Açık çizimde verilen katmana iki uç noktası arasında düz bir çizgi ekler; tek geri alma adımıdır. \
                      Çizgi aracı zincirin her parçası için bu komutu bir kez çalıştırır: her parça ayrı nesne ve ayrı adımdır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<LineCreate>(),
            output: schema::<LineCreated>(),
            plan: Some(schema::<crate::LinePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir çizgi".into(),
                    input: json!({
                        "layerId": "yapi",
                        "a": { "x": 423500.0, "y": 4512300.0 },
                        "b": { "x": 423520.0, "y": 4512312.5 }
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Renkli ve öznitelikli bir çizgi; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "a": { "x": 423500.0, "y": 4512300.0 },
                        "b": { "x": 423500.0, "y": 4512330.0 },
                        "color": "#E5484D",
                        "attrs": { "Tür": "İstinat duvarı" },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: CAD_POLYLINE_CREATE.into(),
            version: CAD_POLYLINE_CREATE_VERSION,
            title: "Çoklu çizgi oluştur".into(),
            summary: "Açık çizimde verilen katmana noktaları ve isteğe bağlı yay değerleriyle (kenar başına bir tane) bir çoklu çizgi ekler; tek geri alma adımıdır. \
                      closed: true son noktayı ilkine bağlar (kapalı çizgi; alan değildir). widths kenar başına başlangıç ve bitiş genişliğidir, \
                      çizimin uzunluğunda; kâğıdın çizgi kalınlığı (lineWeight) ayrıdır (ADR 0258). \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<PolylineCreate>(),
            output: schema::<PolylineCreated>(),
            plan: Some(schema::<crate::PolylinePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Üç noktalı çoklu çizgi".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 }
                        ]
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "İkinci kenarı çeyrek daire yayı olan renkli çoklu çizgi; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423530.0, "y": 4512310.0 }
                        ],
                        "bulges": [0.0, 0.41421356237309503],
                        "color": "#E5484D",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "25 cm genişliğinde kapalı duvar ekseni: kapanan kenar da genişlikli".into(),
                    input: json!({
                        "layerId": "duvar",
                        "pts": [
                            { "x": 0.0, "y": 0.0 },
                            { "x": 6.0, "y": 0.0 },
                            { "x": 6.0, "y": 4.5 },
                            { "x": 0.0, "y": 4.5 }
                        ],
                        "closed": true,
                        "widths": [[0.25, 0.25], [0.25, 0.25], [0.25, 0.25], [0.25, 0.25]]
                    }),
                    output: None,
                },
            ],
        },
        // The erase tool's command (docs/adr/0029): the selection made explicit, held
        // together by fixtures/commands/v1/cad.entities.delete.json.
        CommandDescriptor {
            id: CAD_ENTITIES_DELETE.into(),
            version: CAD_ENTITIES_DELETE_VERSION,
            title: "Nesneleri sil".into(),
            summary: "Açık çizimden kalıcı kimlikleriyle verilen nesneleri siler; tek geri alma adımıdır, geri alma onları aynı kimlikleriyle yerlerine getirir. \
                      Silinecekler girdide açıkça verilir, seçim okunmaz: Sil aracı ve Delete tuşu seçimi ya da tıklanan nesneyi buraya yazar. \
                      Kilitli katmandaki nesneler silinmez: başkaları varsa uyarıyla yerinde kalır, hepsi kilitliyse hiçbir şey silinmez. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey silinmez, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<EntitiesDelete>(),
            output: schema::<EntitiesDeleted>(),
            plan: Some(schema::<crate::EntitiesDeletePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki nesneyi sil".into(),
                    input: json!({
                        "uids": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"
                        ]
                    }),
                    output: Some(json!({
                        "removed": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"
                        ],
                        "locked": [],
                        "revision": "39"
                    })),
                },
                CommandExample {
                    title: "Bir nesneyi yalnız planlandığı sürümde sil".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "expectedRevision": "38"
                    }),
                    output: None,
                },
            ],
        },
        // The point, circle and arc tools' commands (docs/adr/0032), held together by
        // fixtures/commands/v1/cad.{point,circle,arc}.create.json.
        CommandDescriptor {
            id: CAD_POINT_CREATE.into(),
            version: CAD_POINT_CREATE_VERSION,
            title: "Nokta oluştur".into(),
            summary: "Açık çizimde verilen katmana bir nokta nesnesi ekler; isteğe bağlı kotu ve yanında görünen yazısıyla. Tek geri alma adımıdır. \
                      Nokta aracı her nokta için bu komutu bir kez çalıştırır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<PointCreate>(),
            output: schema::<PointCreated>(),
            plan: Some(schema::<crate::PointPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir nokta".into(),
                    input: json!({
                        "layerId": "nokta",
                        "p": { "x": 423510.25, "y": 4512300.5 }
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Kotlu ve adlı bir nokta; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "kot",
                        "p": { "x": 423510.25, "y": 4512300.5 },
                        "z": 812.35,
                        "label": "812.35",
                        "attrs": { "Tür": "Kot noktası" },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: CAD_CIRCLE_CREATE.into(),
            version: CAD_CIRCLE_CREATE_VERSION,
            title: "Daire oluştur".into(),
            summary: "Açık çizimde verilen katmana merkezi ve yarıçapıyla bir daire ekler; tek geri alma adımıdır. \
                      Daire aracı yöntemlerinin (merkez ve yarıçap ya da çap, iki nokta, üç nokta, iki nesneye teğet ve yarıçap, üç nesneye teğet) dairesini ortak geometri çekirdeğiyle bulur ve bu komutla yazar. \
                      Yarıçap sıfırdan büyük olmalıdır. Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<CircleCreate>(),
            output: schema::<CircleCreated>(),
            plan: Some(schema::<crate::CirclePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yarıçapı 12,5 m olan bir daire".into(),
                    input: json!({
                        "layerId": "yapi",
                        "c": { "x": 423510.0, "y": 4512300.0 },
                        "r": 12.5
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Renkli bir daire; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "c": { "x": 423510.0, "y": 4512300.0 },
                        "r": 3.0,
                        "color": "#E5484D",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: CAD_ARC_CREATE.into(),
            version: CAD_ARC_CREATE_VERSION,
            title: "Yay oluştur".into(),
            summary: "Açık çizimde verilen katmana merkezi, yarıçapı ve saat yönünün tersine başlangıç ile bitiş açısıyla (radyan) bir yay ekler; tek geri alma adımıdır. \
                      Yay aracı yöntemlerinin (üç nokta; başlangıç, merkez ve bitiş, açı ya da kiriş; başlangıç, bitiş ve merkez, açı, yön ya da yarıçap; önce merkez; son nesneye teğet devam) yayını ortak geometri çekirdeğiyle bulur ve bu komutla yazar. \
                      Yarıçap sıfırdan büyük olmalıdır; açılar verildiği gibi saklanır, eşit açılar tam turdur. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<ArcCreate>(),
            output: schema::<ArcCreated>(),
            plan: Some(schema::<crate::ArcPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Doğudan kuzeye çeyrek yay".into(),
                    input: json!({
                        "layerId": "yapi",
                        "c": { "x": 423510.0, "y": 4512300.0 },
                        "r": 10.0,
                        "a0": 0.0,
                        "a1": std::f64::consts::FRAC_PI_2
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Batıdan güneye çeyrek yay, eksi bitiş açısıyla; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "c": { "x": 423510.0, "y": 4512300.0 },
                        "r": 4.0,
                        "a0": std::f64::consts::PI,
                        "a1": -std::f64::consts::FRAC_PI_2,
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // The move, copy, rotate, scale, mirror and align tools' command (docs/adr/0037,
        // 0047), held together by fixtures/commands/v1/cad.entities.transform.json.
        CommandDescriptor {
            id: CAD_ENTITIES_TRANSFORM.into(),
            version: CAD_ENTITIES_TRANSFORM_VERSION,
            title: "Nesneleri dönüştür".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri taşır, bir merkez etrafında döndürür, bir merkeze göre ölçekler, iki noktalı bir eksene göre aynalar ya da kaynak noktalarını hedef noktalarına hizalar; yerinde ya da kopya olarak, tek geri alma adımında. \
                      Hizalamada birinci kaynak noktası birinci hedefe gider; ikinci çift verilirse nesneler kaynak doğrultusu hedef doğrultusuna oturacak kadar döner, scale ile boyları da eşitlenir. \
                      Taşı, Kopyala, Döndür, Ölçekle, Aynala ve Hizala araçları seçimi kimlik listesi olarak verip bu komutla yazar; adım aracın adını taşır. \
                      Dönüşüm geometri çekirdeğindedir: her nesne türü (yay, daire, elips, eğri, yazı, ölçü, tarama) aynı kuralla döner; yaylar saat yönünün tersine kalır, aynalanan yazı okunur kalır. \
                      Yerinde değişen nesne yuvasını, kalıcı kimliğini ve öbür alanlarını korur; kopya bütün alanları alır, yeni bir kalıcı kimlik alır. \
                      Kilitli katmandaki nesne ne değişir ne kopyalanır: öbürleri uyarıyla yazılır, hepsi kilitliyse hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<EntitiesTransform>(),
            output: schema::<EntitiesTransformed>(),
            plan: Some(schema::<crate::EntitiesTransformPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki nesneyi 12,5 m doğuya, 7,25 m güneye taşıma".into(),
                    input: json!({
                        "uids": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"
                        ],
                        "transform": { "kind": "move", "dx": 12.5, "dy": -7.25 }
                    }),
                    output: Some(json!({
                        "changed": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"
                        ],
                        "created": [],
                        "locked": [],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Bir nesnenin bir nokta etrafında 90° döndürülmüş kopyası; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "transform": {
                            "kind": "rotate",
                            "center": { "x": 423510.0, "y": 4512300.0 },
                            "angle": std::f64::consts::FRAC_PI_2
                        },
                        "copy": true,
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bir yapıyı yolun kenarına hizalama: köşesi hedef noktaya, cephesi yol doğrultusuna".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "transform": {
                            "kind": "align",
                            "source": { "x": 423510.0, "y": 4512300.0 },
                            "target": { "x": 423540.0, "y": 4512320.0 },
                            "source2": { "x": 423530.0, "y": 4512300.0 },
                            "target2": { "x": 423552.0, "y": 4512336.0 }
                        }
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "created": [],
                        "locked": [],
                        "revision": "38"
                    })),
                },
            ],
        },
        // The edge, corner and object modify tools' command (docs/adr/0047), held together
        // by fixtures/commands/v1/cad.entities.edit.json.
        CommandDescriptor {
            id: CAD_ENTITIES_EDIT.into(),
            version: CAD_ENTITIES_EDIT_VERSION,
            title: "Nesneleri düzenle".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelere yeni geometri verir, onları parçalarla değiştirir, onlardan yeni nesneler yapar ya da onları siler; hepsi tek geri alma adımında, işlemin adıyla. \
                      Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil ve Esnet araçları geometriyi ortak geometri çekirdeğiyle bulur, önizlemede gösterdiklerini bu komutla yazar; \
                      Öznitelikler panelinin geometri satırları ve yerinde yazı düzenleyicisi de properties işlemiyle (adımı Değiştir), alan araçları da (Alan birleştir, kesiştir, çıkar, böl, Alana çevir, Çizgiye çevir) kendi adlarıyla bu komutla yazar. \
                      Seçili nesnenin tutamacı sürüklenince grip işlemiyle (adımı Tutamaçla düzenle), tutamacın menüsü Ortasına köşe ekle ve Köşeyi sil için vertexAdd ve vertexRemove, Düz kenar yap ve Yaya dönüştür için straightEdge ve arcEdge işlemleriyle yazar. \
                      Parçala, Yönü çevir, Sadeleştir ve Çizimi temizle split, reverse, simplify ve cleanup işlemleriyle, Tüm köşeleri yuvarla ve Tüm köşelere pah fillet ve chamfer işlemleriyle yazar. \
                      update nesnenin geometrisini değiştirir, öbür alanları kalır; replace nesneyi yerinde ve kimliğiyle başka bir nesne yapar, katmanı ve rengi kalır, öznitelikleri ve etiketi keepData ile kalır; \
                      add bir nesneden yeni nesne yapar, katmanını, rengini ve kalınlığını alır; add'e verilen katman, renk, kalınlık, öznitelik ve etiket yeni nesnenin kendisinindir (Patlat bir bloğun nesnelerini kendi katman, renk ve verileriyle böyle çıkarır); remove nesneyi siler. Bir nesne tek bir değişiklikle değişir. Yazının metni boş olamaz; kapalı alanın halkası en az 3 köşelidir, kenarlarından biri yaysa 2. \
                      Kilitli katmandaki nesne değişmez, ondan nesne yapılmaz: böyle bir nesne verilirse hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<EntitiesEdit>(),
            output: schema::<EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Budama: çizginin kalan iki parçası; ilki çizginin kendisidir".into(),
                    input: json!({
                        "operation": "trim",
                        "changes": [
                            {
                                "kind": "replace",
                                "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                                "geometry": {
                                    "kind": "line",
                                    "a": { "x": 423500.0, "y": 4512300.0 },
                                    "b": { "x": 423508.0, "y": 4512300.0 }
                                }
                            },
                            {
                                "kind": "add",
                                "from": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                                "geometry": {
                                    "kind": "line",
                                    "a": { "x": 423512.0, "y": 4512300.0 },
                                    "b": { "x": 423520.0, "y": 4512300.0 }
                                }
                            }
                        ]
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "created": ["01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"],
                        "removed": [],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Uzatma: çizginin ucu sınıra kadar; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "operation": "extend",
                        "changes": [{
                            "kind": "update",
                            "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "geometry": {
                                "kind": "line",
                                "a": { "x": 423500.0, "y": 4512300.0 },
                                "b": { "x": 423530.0, "y": 4512300.0 }
                            }
                        }],
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // The array tools' command (docs/adr/0047), held together by
        // fixtures/commands/v1/cad.entities.array.json.
        CommandDescriptor {
            id: CAD_ENTITIES_ARRAY.into(),
            version: CAD_ENTITIES_ARRAY_VERSION,
            title: "Nesneleri diziye kopyala".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerin kopyalarını satır ve sütunlara ya da bir merkezin çevresine dizer; bütün kopyalar tek geri alma adımındadır. \
                      Dizi: satır × sütun yer (2 ile 10 000 arası), sütunlar dx, satırlar dy aralıkla; birden çok yeri olan yönün aralığı sıfır olamaz. \
                      Kutupsal dizi: merkezin çevresinde adet kadar öğe (2 ile 1000 arası, asıllar dahil), fill derecelik açıya; 360 tam turu eşit böler, eksi saat yönündedir. rotate ile kopyalar merkez etrafında döner, yoksa yönünü koruyup kopyalanan nesnelerin kutusunun ortasıyla yer değiştirir. \
                      Yol boyunca dizi: kalıcı kimliğiyle verilen yol (çizgi, yay, daire ya da çoklu çizgi) boyunca adet kadar yer (2 ile 10 000 arası, asıllar yolun başında); spacing verilirse yerler yolun başından o aralıkla, verilmezse yolun başından sonuna (kapalı yolda çevresine) eşit dağılır; align ile kopyalar yolun doğrultusuyla döner. Yol okunur, değişmez. \
                      Dizi, Kutupsal dizi ve Yol boyunca dizi araçları seçimi kimlik listesi olarak verip bu komutla yazar; adım aracın adını taşır. Yerleşim geometri çekirdeğindedir: her nesne türü aynı kuralla kopyalanır. \
                      Kopya aslının bütün alanlarını ve yeni bir kalıcı kimlik alır. Kilitli katmandaki nesnenin kopyası yapılmaz: öbürleri uyarıyla yazılır, hepsi kilitliyse hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<EntitiesArray>(),
            output: schema::<EntitiesArrayed>(),
            plan: Some(schema::<crate::EntitiesArrayPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir parseli 2 satır × 3 sütun diziye kopyalama: sütunlar 25 m, satırlar 40 m arayla".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "layout": { "kind": "grid", "rows": 2, "cols": 3, "dx": 25.0, "dy": 40.0 }
                    }),
                    output: Some(json!({
                        "created": [
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7082",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7083",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7084",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7085"
                        ],
                        "locked": [],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Bir direği bir meydanın çevresine 8 kez, tam turda; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "layout": {
                            "kind": "polar",
                            "center": { "x": 423510.0, "y": 4512300.0 },
                            "count": 8,
                            "fill": 360.0,
                            "rotate": true
                        },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // Ötele's semantic command (docs/adr/0246), held together by
        // fixtures/commands/v1/cad.entities.offset.json. The web is paused
        // (docs/adr/0228): the desktop's host alone runs it for now.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_OFFSET.into(),
            version: crate::CAD_ENTITIES_OFFSET_VERSION,
            title: "Nesneleri ötele".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerin paralel kopyalarını yapar; kopyalar ve kaynakların silinmesi tek geri alma adımındadır, adı “Ötele”. \
                      Girdi sonucun geometrisi değil, istenen öteleme: distance ile mesafe (metre, sıfırdan büyük) ve side noktası (çizgide ve yolda noktaya en yakın kenarın o yanı, daire, yay ve elipste nokta dışarıdaysa büyür, içerideyse küçülür) ya da through ile kopyanın geçeceği nokta (Noktadan geç: mesafe noktanın nesneye uzaklığıdır). \
                      bothSides (İki yana) öbür yana da aynı mesafede bir kopya yapar; deleteSource (Kaynağı sil) kopyası yapılan kaynakları aynı adımda siler; layerId kopyaların katmanıdır, verilmezse her kopya kaynağının katmanına gider. \
                      Çizgi, çoklu çizgi (yaylarıyla, çok parçalısı parça parça), kapalı alan, daire, yay, elips ve yardımcı çizgi ile ışın ötelenir; öbür türler not_offsettable ile reddedilir. Kopyayı ortak geometri çekirdeği hesaplar: Ötele aracı, Python ve ajan aynı nesneyi aynı mesafeyle öteleyince aynı kopyayı bulur. \
                      Kopya kaynağının katmanını, rengini ve çizgi kalınlığını alır; özniteliklerini, etiketini, simgesini ve eklerini almaz; köşeler kaynağın köşelerinin kotunu ve ölçüsünü (M) taşır. \
                      Yapılamayan kopya (merkezini geçen daire, içe dönen yay) offset_failed uyarısıyla atlanır; hiçbir kopya yapılamazsa hiçbir şey yazılmaz. Kilitli katmandaki nesne ötelenmez: hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. Plan yazmadan kopyaları gösterir; planın sürümüyle yazma planı yazar. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesOffset>(),
            output: schema::<crate::EntitiesOffsetMade>(),
            plan: Some(schema::<crate::EntitiesOffsetPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir yol kenarını 0,2 m doğuya ötelemek".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "by": { "kind": "distance", "distance": 0.2, "side": { "x": 423520.0, "y": 4512300.0 } }
                    }),
                    output: Some(json!({
                        "created": ["01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"],
                        "removed": [],
                        "distances": [0.2],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Bir parsel sınırını iki yana 1,5 m, kopyalar başka katmana, kaynak silinir; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "by": { "kind": "distance", "distance": 1.5, "side": { "x": 423510.0, "y": 4512310.0 } },
                        "bothSides": true,
                        "deleteSource": true,
                        "layerId": "yol",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bir daireyi bir noktadan geçecek biçimde ötelemek (Noktadan geç)".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "by": { "kind": "through", "point": { "x": 423515.0, "y": 4512300.0 } }
                    }),
                    output: None,
                },
            ],
        },
        // Yeniden bağla (docs/adr/0249 §3), held together by
        // fixtures/commands/v1/cad.dimensions.reassociate.json. The web is paused
        // (docs/adr/0228): the desktop's host alone runs it for now.
        CommandDescriptor {
            id: crate::CAD_DIMENSIONS_REASSOCIATE.into(),
            version: crate::CAD_DIMENSIONS_REASSOCIATE_VERSION,
            title: "Ölçüyü yeniden bağla".into(),
            summary: "İlişkili ölçünün noktalarını nesnelerin yerlerine bağlar ya da serbest bırakır; tek geri alma adımı “Ölçüyü yeniden bağla”. \
                      Her bağ ölçünün bir noktası (a, b ya da c) ve yeni yeridir: nesnenin kalıcı kimliği ve yeri (node, start, end, center, vertex, mid; parça, halka ve sırasıyla). to null noktayı serbest bırakır: nokta yerinde kalır, hiçbir şeyi izlemez. \
                      Bağlanan nokta yerinin şimdiki koordinatına taşınır ve bundan sonra nesneyi izler: nesne taşınır, döner ya da köşesi çekilirse ölçü de gider; köşe silinirse bağ kopar, nokta yerinde kalır. Kopmuş bağa yer verilirse yeniden canlanır. \
                      Sonuç ölçünün yeni değerini (value, unit) söyler. Hiçbir nokta değişmiyorsa hiçbir şey yazılmaz. Kilitli katmandaki ölçü değişmez. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DimensionsReassociate>(),
            output: schema::<crate::DimensionsReassociated>(),
            plan: Some(schema::<crate::DimensionsReassociatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Ölçünün iki ucunu bir duvarın iki köşesine bağla".into(),
                    input: json!({
                        "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001",
                        "points": [
                            { "point": "a", "to": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "place": { "kind": "vertex", "part": 0, "ring": 0, "index": 0 } } },
                            { "point": "b", "to": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "place": { "kind": "vertex", "part": 0, "ring": 0, "index": 1 } } }
                        ]
                    }),
                    output: Some(json!({ "changed": 2, "value": 12.5, "unit": "length", "revision": "41" })),
                },
                CommandExample {
                    title: "Bir ucu serbest bırak".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "points": [{ "point": "b", "to": null }] }),
                    output: None,
                },
            ],
        },
        // Ölçülendirme by what it measures (docs/adr/0255), held together by
        // fixtures/commands/v1/cad.dimensions.create.json; the desktop's alone while the web is paused.
        CommandDescriptor {
            id: crate::CAD_DIMENSIONS_CREATE.into(),
            version: crate::CAD_DIMENSIONS_CREATE_VERSION,
            title: "Ölçü ekle".into(),
            summary: "measure'ın ölçtüğünü at'nin yerine ölçü olarak yazar; layerId'ye cad.entities.create'in kurallarıyla tek adımda, adı “Ekle”. \
                      Ölçülen noktalar koordinat (kind at) ya da nesnenin yeri (kind place: kalıcı kimlik ve yer: node, start, end, center, vertex, mid) olarak verilir; kenarlar nesnenin kimliği ve tıklanan yerle (of, first, second: uid, near), o yere en yakın düz kenar, yay ya da daire ölçülür. \
                      Türler: aligned (Hizalı), linear (Doğrusal; angle 0 ΔY, 90 ΔX, yoksa at'ye göre), ordinate (Koordinat; çizgisi at'de biter), azimuth (Semt), slope (Eğim; za, zb ya da yerin köşesinin kotu), angle (iki düz kenar), angleVertex (köşe ve iki kol noktası), angleArc (yayın kendi açısı), radius, diameter, arcLength (between ile yayın iki nokta arası parçası), jogged (Kırıklı yarıçap: shownCentre ve on). \
                      at ölçü çizgisinin, yayının ya da çizgisinin geçtiği yerdir; offset verilirse onun yerine yazılan uzaklıktır (yarıçap, çap ve koordinat almaz: invalid_offset). dimensionStyle projenin ölçü stilinin adıdır (yoksa Standart); height verilmezse stilin yüksekliği, Standart'ta projenin Ölçü yüksekliği. \
                      Hizalı, Doğrusal, Koordinat, Semt ve Eğim'de yer olarak verilen noktalar bağlanır: nesnesi değişince ölçü izler (ilişkili ölçü). Ölçülendirme aracı ölçüyü bu komutla yazar: araç, Python ve ajan aynı girdiye aynı ölçüyü alır. \
                      Ölçü oluşmuyorsa (noktalar çakışıyor, kenarlar paralel, yay boş) no_dimension; nesnede aranan türde kenar yoksa not_measurable. Çıktı ve plan cad.entities.create'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DimensionsCreate>(),
            output: schema::<crate::EntitiesCreated>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Duvarın iki köşesi arasına hizalı ölçü, 2 m dışarıda; köşelere bağlı".into(),
                    input: json!({
                        "measure": {
                            "kind": "aligned",
                            "a": { "kind": "place", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "place": { "kind": "vertex", "part": 0, "ring": 0, "index": 0 } },
                            "b": { "kind": "place", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "place": { "kind": "vertex", "part": 0, "ring": 0, "index": 1 } }
                        },
                        "at": { "x": 500005.0, "y": 4400002.0 },
                        "layerId": "olcu"
                    }),
                    output: Some(json!({ "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4401"], "ids": [71], "revision": "67" })),
                },
                CommandExample {
                    title: "Dairenin yarıçapı, kuzeydoğuya doğru".into(),
                    input: json!({
                        "measure": { "kind": "radius", "of": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003", "near": { "x": 500020.0, "y": 4400010.0 } } },
                        "at": { "x": 500030.0, "y": 4400020.0 },
                        "layerId": "olcu",
                        "dimensionStyle": "Mimari"
                    }),
                    output: None,
                },
            ],
        },
        // Buda, Uzat, Köşe yuvarla and Pah by their meaning (docs/adr/0250), held together by
        // fixtures/commands/v1/cad.entities.{trim,extend,fillet,chamfer}.json; the desktop's alone while the web is paused.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_TRIM.into(),
            version: crate::CAD_ENTITIES_TRIM_VERSION,
            title: "Nesneleri buda".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri tıklandıkları yerde sınırlarına kadar budar; tek geri alma adımı “Buda”. \
                      Her seçim (picks) bir nesne ve onun üstünde ya da yakınında bir noktadır: o noktanın iki yanındaki sınırların arasındaki parça kalkar. Aynı nesnenin seçimleri sırayla, önceki seçimlerin bıraktığı parçalar üstünde işlenir (Çit). \
                      boundaries sınırları söyler: all (varsayılan; bütün görünen nesneler: görünür katmanda, katman süzgecinden geçen), area (köşeleri verilen dikdörtgene değen görünen nesneler; aracın görünümü) ya da objects (verilen nesneler, Sınır seç). Nesne kendini sınırlamaz; her nesne komuttan önceki çizimle sınırlanır. \
                      Nesnenin yerini kalan ilk parça alır, öbür parçalar ondan yeni nesnelerdir; tek parça kalırsa özellikleri korunur (kapalı alan değilse). Adalı alan budanmaz. \
                      Bir şey yapamayan seçim trim_failed uyarısıyla atlanır; hiçbiri bir şey yapamazsa hiçbir şey yazılmaz. Parçaları paylaşılan geometri çekirdeği bulur: Buda aracı, Python ve ajan aynı isteğe aynı parçaları alır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesTrim>(),
            output: schema::<crate::EntitiesTrimmed>(),
            plan: Some(schema::<crate::EntitiesTrimPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir duvar çizgisini iki kesen arasında buda".into(),
                    input: json!({ "picks": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "at": { "x": 500005.0, "y": 4400000.0 } }] }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4003"], "removed": [], "revision": "52" })),
                },
                CommandExample {
                    title: "Yalnız seçilen sınırlarla, görünümün dikdörtgeninde değil".into(),
                    input: json!({ "picks": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "at": { "x": 500005.0, "y": 4400000.0 } }], "boundaries": { "kind": "objects", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"] } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_EXTEND.into(),
            version: crate::CAD_ENTITIES_EXTEND_VERSION,
            title: "Nesneleri uzat".into(),
            summary: "Kalıcı kimlikleriyle verilen açık çizgileri, çoklu çizgileri ve yayları tıklanan yere yakın uçlarından ilk sınıra kadar uzatır; tek geri alma adımı “Uzat”. \
                      Seçimler ve sınırlar cad.entities.trim'deki gibidir; aynı nesnenin iki seçimi iki ucunu uzatabilir, bir uç bir kez uzar. Uç hiçbir sınıra ulaşmıyorsa seçim extend_failed uyarısıyla atlanır; hiçbir uç uzamazsa hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesTrim>(),
            output: schema::<crate::EntitiesTrimmed>(),
            plan: Some(schema::<crate::EntitiesTrimPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir çizginin sağ ucunu ilk sınıra uzat".into(),
                    input: json!({ "picks": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "at": { "x": 500009.0, "y": 4400000.0 } }] }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "created": [], "removed": [], "revision": "53" })),
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_FILLET.into(),
            version: crate::CAD_ENTITIES_FILLET_VERSION,
            title: "Köşe yuvarla".into(),
            summary: "Bir köşeyi verilen yarıçapla yuvarlar; tek geri alma adımı “Köşe yuvarla”. \
                      corner köşeyi söyler: vertex (çoklu çizginin ya da alanın düz kenarlar arasındaki köşesi, kimliği ve köşe sırası), lines (iki çizgi, her biri tıklandığı tarafı korur) ya da all (verilen çoklu çizgi ve alanların bütün köşeleri; sığmayan köşe atlanır ve sayılır). \
                      trim (Kırp, varsayılan açık) kenarları yaya kadar kırpar; iki çizginin yayı ilk çizgiden yeni nesnedir. Kırp kapalıyken kenarlar kalır, yalnız yay eklenir. Sıfır yarıçap iki çizgiyi buluştukları noktada birleştirir. \
                      Hesap paylaşılan geometri çekirdeğinindir; araç, Python ve ajan aynı köşeyi aynı yarıçapla aynı biçimde yuvarlar. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesFillet>(),
            output: schema::<crate::EntitiesCornered>(),
            plan: Some(schema::<crate::EntitiesCornerPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir parselin üçüncü köşesini 2 m yarıçapla yuvarla".into(),
                    input: json!({ "corner": { "kind": "vertex", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "index": 2 }, "radius": 2.0 }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "created": [], "done": 1, "skipped": 0, "revision": "54" })),
                },
                CommandExample {
                    title: "İki çizgiyi 1 m yarıçapla birleştir".into(),
                    input: json!({ "corner": { "kind": "lines", "first": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "at": { "x": 500004.0, "y": 4400000.0 } }, "second": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "at": { "x": 500010.0, "y": 4400004.0 } } }, "radius": 1.0 }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_CHAMFER.into(),
            version: crate::CAD_ENTITIES_CHAMFER_VERSION,
            title: "Pah kır".into(),
            summary: "Bir köşeyi pah kırar: ilk kenar boyunca distance, öbürü boyunca second (verilmezse aynı) metre; tek geri alma adımı “Pah”. \
                      corner, trim ve all cad.entities.fillet'teki gibidir; iki çizginin pahı ilk çizgiden yeni bir çizgidir. İki mesafe de sıfırsa iki çizgi buluştukları noktada birleşir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesChamfer>(),
            output: schema::<crate::EntitiesCornered>(),
            plan: Some(schema::<crate::EntitiesCornerPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir alanın bütün köşelerine 0,5 m pah".into(),
                    input: json!({ "corner": { "kind": "all", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"] }, "distance": 0.5 }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "created": [], "done": 4, "skipped": 0, "revision": "55" })),
                },
            ],
        },
        // Esnet, Kır, Birleştir, Böl, Alan oluştur and Tarama by their meaning (docs/adr/0254),
        // held together by fixtures/commands/v1/cad.entities.{stretch,break,join,divide,boundary,hatch}.json;
        // the desktop's alone while the web is paused.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_STRETCH.into(),
            version: crate::CAD_ENTITIES_STRETCH_VERSION,
            title: "Nesneleri esnet".into(),
            summary: "Pencerenin (window) içindeki köşeleri dx, dy kadar taşır, öbürlerini bırakır; tek geri alma adımı “Esnet”. Yay bükümünü korur. \
                      uids verilirse yalnız onlar (seçim); penceresinde köşesi olmayan outside_window uyarısıyla atlanır. Verilmezse pencereye değen görünen nesneler. Kilitli katmandakiler locked_skipped uyarısıyla atlanır; hiçbiri kalmazsa nothing_to_stretch. \
                      topology verilirse (Topolojik düzenleme) taşınan köşeleri paylaşan görünen komşular aynı adımda izler; points noktaları da köşe sayar. Kilitli ya da geçersiz kalacak komşu uyarıyla değişmez. \
                      Yer değiştirme sıfırsa hiçbir şey yazılmaz. Yeni şekilleri paylaşılan geometri çekirdeği bulur: Esnet aracı, Python ve ajan aynı isteğe aynı şekilleri alır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesStretch>(),
            output: schema::<crate::EntitiesStretched>(),
            plan: Some(schema::<crate::EntitiesStretchPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir duvarın ucunu 2 m doğuya çek".into(),
                    input: json!({ "window": { "min": { "x": 500018.0, "y": 4399998.0 }, "max": { "x": 500022.0, "y": 4400002.0 } }, "dx": 2.0, "dy": 0.0 }),
                    output: Some(json!({ "stretched": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "followed": [], "revision": "61" })),
                },
                CommandExample {
                    title: "Yalnız seçilen parselin köşesi, ortak sınırlı komşularıyla".into(),
                    input: json!({ "window": { "min": { "x": 500039.0, "y": 4400029.0 }, "max": { "x": 500041.0, "y": 4400031.0 } }, "dx": 0.5, "dy": 0.5, "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "topology": {} }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_BREAK.into(),
            version: crate::CAD_ENTITIES_BREAK_VERSION,
            title: "Nesneyi kır".into(),
            summary: "Kalıcı kimliğiyle verilen nesnenin first ile second arasındaki kısmını siler; second yoksa nesneyi first'te ikiye böler. Tek geri alma adımı “Kır”. \
                      Çizgi, çoklu çizgi, alan, yay, daire, elips, yardımcı çizgi ve ışın kırılır; öbürleri not_breakable. Kilitli katmandaki nesne kırılmaz. \
                      Nesnenin yerini ilk parça alır, öbür parçalar ondan yeni nesnelerdir; tek parça kalırsa özellikleri korunur (kapalı alan değilse). Parçaları paylaşılan geometri çekirdeği bulur; bulamadığında break_failed çekirdeğin sözüyle döner. \
                      Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesBreak>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir çizginin iki noktası arasını sil".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "first": { "x": 500004.0, "y": 4400000.0 }, "second": { "x": 500006.0, "y": 4400000.0 } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4003"], "removed": [], "revision": "62" })),
                },
                CommandExample {
                    title: "Bir çizgiyi bir noktada ikiye böl".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "first": { "x": 500005.0, "y": 4400000.0 } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_JOIN.into(),
            version: crate::CAD_ENTITIES_JOIN_VERSION,
            title: "Nesneleri birleştir".into(),
            summary: "Kalıcı kimlikleriyle verilen çizgi, yay ve açık çoklu çizgilerden uç uca buluşanları her zincir için tek nesne yapar; kapanan zincir kapalı alan olur. Tek geri alma adımı “Birleştir”. \
                      Zincir ilk nesnesinin yerini, kimliğini ve verisini alır, öbürleri silinir. tolerance uçların aralığıdır (metre, varsayılan 0,001). Kilitli katmandakiler locked_skipped uyarısıyla dışarıda kalır; hiç zincir kurulamazsa nothing_to_join. \
                      chain verilirse (Zincir) uids tek nesnedir, tohum: ona uç uca bağlı nesneler chain'in nesneleri arasında (all, area ya da objects) serbest uca, kavşağa, kilitli nesneye ya da başa dönene kadar onunla birleşir; tohum yerini, kimliğini ve verisini verir. \
                      Zincirleri paylaşılan geometri çekirdeği bulur. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesJoin>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Uç uca iki çizgiyi bir çoklu çizgi yap".into(),
                    input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"] }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "created": [], "removed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "revision": "63" })),
                },
                CommandExample {
                    title: "Zincir: tıklanan çizgiye bağlı bütün görünen çizgiler".into(),
                    input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "chain": { "kind": "all" } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_DIVIDE.into(),
            version: crate::CAD_ENTITIES_DIVIDE_VERSION,
            title: "Nesneyi böl".into(),
            summary: "Kalıcı kimliğiyle verilen nesnenin üstüne noktalar koyar: division eşit parçalar (parts, 2–10 000) ya da aralık (step, metre). Noktalar nesnenin başından, fromEnd ile sonundan ölçülür; layerId'ye cad.entities.create'in kurallarıyla yazılır. Tek geri alma adımı “Böl”. \
                      Çizgi, çoklu çizgi, alan, yay, daire, elips ve eğri bölünür; öbürleri not_divisible. Aralık nesneden uzunsa no_points, 10 000'den çok nokta too_many_points. Noktaların yerini paylaşılan geometri çekirdeği bulur (elipsinkiler eğrinin üstünde). \
                      color, lineWeight, attrs, label ve symbol noktaların görünüşü ve taşıdıklarıdır. Çıktı ve plan cad.entities.create'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesDivide>(),
            output: schema::<crate::EntitiesCreated>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir yolu dört eşit parçaya böl".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "division": { "kind": "parts", "count": 4 }, "layerId": "nokta" }),
                    output: Some(json!({ "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4101", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4102", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4103"], "ids": [51, 52, 53], "revision": "64" })),
                },
                CommandExample {
                    title: "Sondan başlayarak her 20 metrede bir nokta".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "division": { "kind": "step", "length": 20.0 }, "fromEnd": true, "layerId": "nokta" }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_BOUNDARY.into(),
            version: crate::CAD_ENTITIES_BOUNDARY_VERSION,
            title: "İçine tıklayarak alan".into(),
            summary: "at noktasını çevreleyen ve sınırların çizgilerinin kapattığı bölgeyi layerId'de alan yapar; içindeki kapalı gruplar delik olur (islands, varsayılan açık). Tek geri alma adımı “Alan oluştur”. \
                      boundaries sınırları söyler: all (varsayılan; görünen bütün nesneler), area (dikdörtgene değen görünen nesneler; aracın görünümü) ya da objects (verilen nesneler). boundaryLayer verilirse yalnız o katmanın çizgileri sayılır. Bölge yoksa not_enclosed. \
                      Bölgeyi paylaşılan geometri çekirdeği bulur; alan cad.entities.create'in kurallarıyla yazılır (color, lineWeight, attrs, label, symbol). Çıktı ve plan cad.entities.create'inkidir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesBoundary>(),
            output: schema::<crate::EntitiesCreated>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Çizgilerle çevrili odanın içine tıkla".into(),
                    input: json!({ "at": { "x": 500002.0, "y": 4400001.5 }, "layerId": "alan" }),
                    output: Some(json!({ "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4201"], "ids": [60], "revision": "65" })),
                },
                CommandExample {
                    title: "Yalnız sınır katmanının çizgileriyle, adalar yok sayılarak".into(),
                    input: json!({ "at": { "x": 500002.0, "y": 4400001.5 }, "layerId": "alan", "boundaryLayer": "sinir", "islands": false }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_HATCH.into(),
            version: crate::CAD_ENTITIES_HATCH_VERSION,
            title: "Tara".into(),
            summary: "region'ın bölgelerini pattern ile tarar, her bölgeye bir tarama; layerId'ye cad.entities.create'in kurallarıyla tek adımda yazar, adı “Tarama”. \
                      region point: at'yi çevreleyen bölge. way object (varsayılan, kapalı nesne): noktayı çevreleyen en küçük kapalı nesne, içindeki ya da kenarını aşan küçük kapalı nesneler adadır; associative (varsayılan açık) taramayı nesnelerine bağlar. way lines (çizgiler): sınırların (boundaries, boundaryLayer) çizgilerinin kapattığı yüz; bir şeye bağlanmaz. texts açıksa bölgedeki yazılar ve bloklar kutularıyla boş kalır. \
                      region objects (Çoklu tara): her kapalı nesne, çok parçalı alanın her parçası kendi içinden taranır; kapalı olmayan not_closed uyarısıyla atlanır. \
                      pattern: name (Dolu, Çizgili, Çapraz, ANSI31, ISO02W100 …, degradeler), scale (kâğıtta; çizim ölçeğiyle çarpılır), angle (derece), secondColor ve inverted (degrade). Bilinmeyen ad unknown_pattern; desen bölge için çok sıksa pattern_too_dense; bölge yoksa not_enclosed. \
                      Bölgeleri paylaşılan geometri çekirdeği bulur: Tarama ve Çoklu tara araçları, Python ve ajan aynı isteğe aynı halkaları alır. Çıktı ve plan cad.entities.create'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesHatch>(),
            output: schema::<crate::EntitiesCreated>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parselin içine tıklayıp ANSI31 ile tara; binalar ada".into(),
                    input: json!({ "region": { "kind": "point", "at": { "x": 500010.0, "y": 4400010.0 } }, "pattern": { "name": "ANSI31", "scale": 1.0 }, "layerId": "tarama" }),
                    output: Some(json!({ "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4301"], "ids": [70], "revision": "66" })),
                },
                CommandExample {
                    title: "Seçilen üç alanı dolu tara".into(),
                    input: json!({ "region": { "kind": "objects", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4003"] }, "pattern": { "name": "Dolu" }, "layerId": "tarama", "color": "#5FBF77" }),
                    output: None,
                },
            ],
        },
        // The drawing tools' command for objects without one of their own (docs/adr/0057),
        // held together by fixtures/commands/v1/cad.entities.create.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_CREATE.into(),
            version: crate::CAD_ENTITIES_CREATE_VERSION,
            title: "Nesneleri ekle".into(),
            summary: "Verilen nesneleri bir katmana yazar; hepsi tek geri alma adımındadır, adı “Ekle” ya da aracın ya da hesabın adı (Paralel çizgi, Dik in, Dik çık, Böl, Tarama, Alan oluştur, Poligon hesabı, Kutupsal alım, Önden kestirme, Geriden kestirme). \
                      Her nesne geometrisiyle verilir, cad.entities.edit'teki gibi: nokta, çizgi, çoklu çizgi, kapalı alan, daire, yay, elips, eğri, yardımcı çizgi, ışın, yazı, ölçü, tarama, blok yerleştirmesi (çizimde tanımlı bir blok, ölçeği pozitif); renk, öznitelik ve etiket isteğe bağlıdır. \
                      Elips, Eğri, Yardımcı çizgi, Işın, Halka, Paralel çizgi, Dik in, Dik çık ve Böl araçları geometriyi ortak geometri çekirdeğiyle bulur, önizlemede gösterdiklerini bu komutla yazar; Yazı, Ölçülendirme, Tarama, Parsel oluştur ve İçine tıklayarak alan da. \
                      Hesap menüsünün pencereleri (Poligon hesabı, Kutupsal alım, Önden ve Geriden kestirme) bulduğu noktaları adları, etiketleri ve öznitelikleriyle bu komutla çizime ekler. \
                      Kilitli katmana hiçbir şey yazılmaz; gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesCreate>(),
            output: schema::<crate::EntitiesCreated>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir elips: merkezi, büyük yarı ekseni (merkezden bir uca) ve küçük eksenin büyüğe oranı".into(),
                    input: json!({
                        "layerId": "yapi",
                        "objects": [{
                            "geometry": {
                                "kind": "ellipse",
                                "c": { "x": 423510.0, "y": 4512300.0 },
                                "major": { "x": 12.0, "y": 0.0 },
                                "ratio": 0.5,
                                "t0": 0.0,
                                "t1": 0.0
                            }
                        }]
                    }),
                    output: Some(json!({
                        "created": ["01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"],
                        "ids": [41],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Paralel çizgi: eksenin iki yanı 5 m arayla ve eksen, tek adımda; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yol",
                        "operation": "parallel",
                        "objects": [
                            { "geometry": { "kind": "polyline", "pts": [{ "x": 423500.0, "y": 4512305.0 }, { "x": 423600.0, "y": 4512305.0 }] } },
                            { "geometry": { "kind": "polyline", "pts": [{ "x": 423500.0, "y": 4512295.0 }, { "x": 423600.0, "y": 4512295.0 }] } },
                            { "geometry": { "kind": "polyline", "pts": [{ "x": 423500.0, "y": 4512300.0 }, { "x": 423600.0, "y": 4512300.0 }] } }
                        ],
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Poligon hesabının iki yeni noktası: adları etiket ve öznitelik olarak, tek adımda “Poligon hesabı”".into(),
                    input: json!({
                        "layerId": "poligon",
                        "operation": "traverse",
                        "objects": [
                            { "geometry": { "kind": "point", "p": { "x": 423512.304, "y": 4512291.268 } }, "label": "P1", "attrs": { "Ad": "P1", "Tür": "Poligon noktası" } },
                            { "geometry": { "kind": "point", "p": { "x": 423548.119, "y": 4512310.442 } }, "label": "P2", "attrs": { "Ad": "P2", "Tür": "Poligon noktası" } }
                        ]
                    }),
                    output: None,
                },
            ],
        },
        // Öznitelikler's layer, colour, symbol and attribute rows and the symbol commands'
        // command, held together by fixtures/commands/v1/cad.entities.set.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_SET.into(),
            version: crate::CAD_ENTITIES_SET_VERSION,
            title: "Nesnelerin özelliklerini değiştir".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerin katmanını, rengini, sembolünü, özniteliklerini ya da etiketini değiştirir; hepsi tek geri alma adımında, adı işlemin: Katman değiştir, Renk değiştir, Sembol ata (null ile Sembolü kaldır), Değiştir (öznitelik), Etiket değiştir. \
                      Verilmeyen özellik değişmez. Renk, sembol ve etiket null ile kaldırılır: nesne katmanının rengiyle ve stiliyle çizilir. Öznitelik adıyla verilir ya da null ile silinir; adı geçmeyenler kalır. \
                      Öznitelikler panelinin katman, renk, sembol ve öznitelik satırları, Sembol ver ve Sembolü kaldır bu komutla yazar. \
                      Kilitli katmandaki nesne değişmez, kilitli katmana nesne taşınmaz: böyle bir nesne ya da katman verilirse hiçbir şey yazılmaz. Gizli katmana uyarıyla taşınır. \
                      Zaten istendiği gibi olan nesne değişmez; hiçbiri değişmiyorsa hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesSetProperties>(),
            output: schema::<crate::EntitiesPropertiesSet>(),
            plan: Some(schema::<crate::EntitiesSetPropertiesPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Katman değiştir: iki nesne Yol katmanına".into(),
                    input: json!({
                        "uids": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"
                        ],
                        "layerId": "yol",
                        "operation": "layer"
                    }),
                    output: Some(json!({
                        "changed": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"
                        ],
                        "ids": [12, 41],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Renk katmana göre: nesnenin kendi rengi kaldırılır".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "color": null,
                        "operation": "color"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Parsel numarası ve onu gösteren etiket birlikte; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "attrs": { "Parsel": "8" },
                        "label": "8",
                        "operation": "attributes",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // Blok oluştur and the Bloklar panel (docs/adr/0144), held together by
        // fixtures/commands/v1/cad.blocks.define.json and cad.blocks.edit.json.
        CommandDescriptor {
            id: crate::CAD_BLOCKS_DEFINE.into(),
            version: crate::CAD_BLOCKS_DEFINE_VERSION,
            title: "Blok tanımla".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerden, verilen adla ve taban noktasıyla yeni bir blok tanımı yapar; tek geri alma adımı, adı Blok tanımla. \
                      Nesneler olduğu gibi, koordinatları, katmanları, renkleri ve verileriyle kopyalanır; blok iç içe olabilir, en çok 16 düzey. Ad boş olamaz ve çizimde bir kez bulunur (büyük küçük harf Türkçe kurallarıyla ayırt edilmez). \
                      replace ile nesneler silinir, yerine yeni bloğun taban noktasına, layerId katmanına bir yerleştirmesi konur (ölçek 1, dönüş yok); kilitli katmandaki nesne tanıma girer ama silinmez, böyle bir nesne varsa replace reddedilir. \
                      Blok oluştur aracı bu komutla yazar; blok cad.entities.create'in insert geometrisiyle yerleştirilir, Patlat onu cad.entities.edit ile açar. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::BlocksDefine>(),
            output: schema::<crate::BlockDefined>(),
            plan: Some(schema::<crate::BlocksDefinePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Rögar: bir çember ve yazısı, taban noktası çemberin merkezi".into(),
                    input: json!({
                        "name": "Rögar",
                        "base": { "x": 423510.0, "y": 4512300.0 },
                        "uids": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"
                        ]
                    }),
                    output: Some(json!({
                        "block": "01925f3e-7c1c-7b22-8d33-4c5e6f708192",
                        "removed": [],
                        "revision": "39"
                    })),
                },
                CommandExample {
                    title: "Seçilenleri blokla değiştir: nesneler gider, yerine bloğun bir yerleştirmesi gelir".into(),
                    input: json!({
                        "name": "Aydınlatma direği",
                        "base": { "x": 423520.0, "y": 4512305.0 },
                        "uids": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"
                        ],
                        "replace": true,
                        "layerId": "aydinlatma"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_BLOCKS_EDIT.into(),
            version: crate::CAD_BLOCKS_EDIT_VERSION,
            title: "Blok tanımını değiştir".into(),
            summary: "Bir blok tanımını kimliğiyle değiştirir; tek geri alma adımı, adı işlemin: rename yeni ad (Blok değiştir), redefine içindekileri kalıcı kimlikleriyle verilen nesnelerden yeniden kurar (Blok değiştir), rebase taban noktasını taşır (Blok değiştir), remove kullanılmayan tanımı siler (Blok sil), purge kullanılmayan bütün tanımları siler (Blokları temizle), attributes tanımın öznitelik listesini bütün yazar (Blok değiştir; etiket boş olamaz, listede bir kez bulunur; boş liste öznitelikleri kaldırır). \
                      Tanım değişince bütün yerleştirmeleri yenisini gösterir. Çizimde ya da başka bir tanımda yerleştirmesi olan tanım silinmez; tanım kendini içeremez, en çok 16 düzey iç içe olabilir. \
                      redefine'da replace ile nesneler silinir, yerine tanımın bir yerleştirmesi konur. Bir şey değiştirmeyen istek hiçbir şey yazmaz. \
                      Bloklar paneli bu komutla yazar. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::BlocksEdit>(),
            output: schema::<crate::BlocksEdited>(),
            plan: Some(schema::<crate::BlocksEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yeniden adlandır".into(),
                    input: json!({
                        "operation": "rename",
                        "block": "01925f3e-7c1c-7b22-8d33-4c5e6f708192",
                        "name": "Kanalizasyon rögarı"
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1c-7b22-8d33-4c5e6f708192"],
                        "removed": [],
                        "deleted": [],
                        "revision": "40"
                    })),
                },
                CommandExample {
                    title: "Kullanılmayan bütün blokları temizle".into(),
                    input: json!({ "operation": "purge" }),
                    output: None,
                },
                CommandExample {
                    title: "Öznitelikler: rögarın numarası ve kotu, yazıları taban noktasının sağında".into(),
                    input: json!({
                        "operation": "attributes",
                        "block": "01925f3e-7c1c-7b22-8d33-4c5e6f708192",
                        "attributes": [
                            { "tag": "NO", "prompt": "Rögar numarası", "value": "R-?", "p": { "x": 0.9, "y": 0.15 }, "height": 0.5, "rotation": 0.0 },
                            { "tag": "KOT", "prompt": "Kapak kotu", "p": { "x": 0.9, "y": -0.55 }, "height": 0.4, "rotation": 0.0 }
                        ]
                    }),
                    output: None,
                },
            ],
        },
        // Harita servisi, Servisten veri al and the layers' menus (docs/adr/0208 §15),
        // held together by fixtures/commands/v1/cad.layers.service.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_SERVICE.into(),
            version: crate::CAD_LAYERS_SERVICE_VERSION,
            title: "Servis katmanı".into(),
            summary: "Katman ağacına bir harita servisinden çizilen katman ekler (add: XYZ ve TMS, WMTS, WMS, OGC API Tiles, ArcGIS REST, Google'ın 2B karoları, vektör karolar), nesneleri bir servisten alınan katman ekler (addFeed: WFS, OGC API Features, ArcGIS katmanı, GeoJSON adresi; alanlarıyla), birini değiştirir (update: ad, servis ya da kaynak) ya da bir servis katmanını siler (remove); tek geri alma adımı, adı işlemin. \
                      parent katmanın gireceği grup (ya da yanına gireceği katman), index gruptaki yeri: ilki en üstte çizilir, verilmezse en sona, her şeyin altına girer. \
                      connections projenin bağlantılarına eklenir ya da aynı kimliklisinin yerini alır; bağlantı yalnız doğrulamanın türünü ve gizli olmayan adları taşır, gizli değerler çizime yazılmaz. \
                      Servis katmanı nesne tutmaz; servis ya da kaynak projenin bağlantılarından birini adlandırır. \
                      Nesneler servisten alındıktan sonra cad.entities.create ile yazılır; aynı işlem grubunda tek adım olur. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersService>(),
            output: schema::<crate::LayersServiced>(),
            plan: Some(schema::<crate::LayersServicePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "OpenStreetMap altlığı en alta".into(),
                    input: json!({
                        "operation": "add",
                        "name": "OpenStreetMap",
                        "service": {
                            "kind": "xyz",
                            "url": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
                            "maxZoom": 19,
                            "attribution": "© OpenStreetMap katkıda bulunanlar",
                            "preset": "osm-standard"
                        }
                    }),
                    output: Some(json!({ "layer": "layer-7", "revision": "41" })),
                },
                CommandExample {
                    title: "Belediyenin WMS'i, anahtarı adreste giden bir bağlantıyla".into(),
                    input: json!({
                        "operation": "add",
                        "name": "İmar planı",
                        "index": 0,
                        "service": {
                            "kind": "wms",
                            "url": "https://kbs.ornek-belediye.bel.tr/geoserver/wms",
                            "layers": ["imar:plan"],
                            "srid": 5254,
                            "version": "1.3.0",
                            "format": "image/png",
                            "transparent": true,
                            "connection": "belediye"
                        },
                        "connections": [{
                            "id": "belediye",
                            "name": "Belediye KBS",
                            "origin": "https://kbs.ornek-belediye.bel.tr",
                            "auth": "query",
                            "names": ["apikey"]
                        }]
                    }),
                    output: None,
                },
                CommandExample {
                    title: "WFS'ten alınacak parseller için veri katmanı".into(),
                    input: json!({
                        "operation": "addFeed",
                        "name": "Parseller (WFS)",
                        "feed": {
                            "kind": "wfs",
                            "url": "https://kbs.ornek-belediye.bel.tr/geoserver/wfs",
                            "name": "kadastro:parsel",
                            "version": "2.0.0",
                            "srid": 5254,
                            "key": "parsel_id"
                        }
                    }),
                    output: None,
                },
            ],
        },
        // Ağlar (docs/adr/0209 §11), held together by fixtures/commands/v1/cad.network.define.json.
        CommandDescriptor {
            id: crate::CAD_NETWORK_DEFINE.into(),
            version: crate::CAD_NETWORK_DEFINE_VERSION,
            title: "Ağ tanımı".into(),
            summary: "Projenin ağlarına bir ağ tanımı yazar (set: aynı kimlikli ağın yerini alır, yoksa sona eklenir) ya da birini siler (remove). \
                      Ağ, katmanlardan her analizde kurulan grafın tanımıdır: kenar katmanları (çizgi, çoklu çizgi ve yaylar; isteğe bağlı süzgeç ifadesi), düğüm katmanları (bağlantı, kaynak ya da vana; süzgeç ve kapalı ifadesi), bağlanma (ends: uçlarda, bir uç başka kenara değiyorsa orada; vertices: bütün köşelerde de) ve tolerans (m), yön (both, digitized ya da field: alanın ileri, geri ve kapalı değerleri), uzunluktan başka maliyetler (speed: hız alanından dakika, varsayılan hızla; field: kenarın bütününün maliyeti) ve kapalı kenarların ifadesi. \
                      Ağ proje ayarıdır: yazmak çizimi kirli yapar, geri alma adımı değildir. Çizimde olmayan katmanı adlandıran ağ yazılır, unknown_layer uyarısıyla. \
                      Analizler (en kısa yol, hizmet alanı, en yakın tesis, maliyet matrisi, şebeke izleme) çizimi değiştirmez; sonuçları cad.entities.create ile yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::NetworkDefine>(),
            output: schema::<crate::NetworkDefined>(),
            plan: Some(schema::<crate::NetworkDefined>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yol ağı: tek yönlü sokaklar ve hızla süre".into(),
                    input: json!({
                        "operation": "set",
                        "network": {
                            "id": "ag-1",
                            "name": "Yollar",
                            "kind": "road",
                            "edges": [{ "layer": "yol-ekseni" }],
                            "connect": "ends",
                            "tolerance": 0.01,
                            "direction": { "kind": "field", "field": "yon", "forward": ["FT"], "backward": ["TF"], "closed": ["N"] },
                            "costs": [{ "name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0 }]
                        }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Su şebekesi: akışla çizilmiş borular, vanalar ve depo".into(),
                    input: json!({
                        "operation": "set",
                        "network": {
                            "id": "ag-2",
                            "name": "İçme suyu",
                            "kind": "utility",
                            "edges": [{ "layer": "su-hatti" }],
                            "junctions": [
                                { "layer": "su-vana", "role": "valve", "closed": "durum = 'kapalı'" },
                                { "layer": "su-depo", "role": "source" }
                            ],
                            "connect": "ends",
                            "tolerance": 0.01,
                            "direction": { "kind": "digitized" }
                        }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Ağı sil".into(),
                    input: json!({ "operation": "remove", "id": "ag-2" }),
                    output: Some(json!({ "networks": [], "revision": "12" })),
                },
            ],
        },
        // Adres verisi (docs/adr/0229 §14), held together by fixtures/commands/v1/cad.addresses.define.json.
        // The web is paused (docs/adr/0228): the desktop's host alone runs it for now.
        CommandDescriptor {
            id: crate::CAD_ADDRESSES_DEFINE.into(),
            version: crate::CAD_ADDRESSES_DEFINE_VERSION,
            title: "Adres verisi".into(),
            summary: "Projenin adres verilerine bir tanım yazar (set: aynı kimliklinin yerini alır, yoksa sona eklenir) ya da birini siler (remove). \
                      Tanım, adres aramasının her aramada çizimden kurduğu dizinin kaynağıdır: kapı noktaları (nokta katmanı; yol ve kapı no alanları zorunlu, mahalle, ilçe, il, posta kodu, bina adı ve adres kodu isteğe bağlı), yollar (çizgi katmanı; ad zorunlu, mahalle ve ilçe isteğe bağlı) ve bölgeler (alan katmanları; mahalle, ilçe ve il düzeyinden en çok birer, ad zorunlu, mahallede ilçe isteğe bağlı); en düşük eşleşme puanı (50–100) ve Konumdan adres'in uzaklığı (metre). \
                      Adres verisi proje ayarıdır: yazmak çizimi kirli yapar, geri alma adımı değildir. Çizimde olmayan katmanı adlandıran tanım yazılır, unknown_layer uyarısıyla. \
                      Adres arama ve konumdan adres çizimi değiştirmez; sonuçları cad.entities.create ile yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::AddressesDefine>(),
            output: schema::<crate::AddressesDefined>(),
            plan: Some(schema::<crate::AddressesDefined>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Numarataj: kapı noktaları, yol orta hatları ve mahalle sınırları".into(),
                    input: json!({
                        "operation": "set",
                        "locator": {
                            "id": "adres-1",
                            "name": "Numarataj",
                            "doors": {
                                "layer": "kapi",
                                "street": "YOL_ADI",
                                "door": "KAPI_NO",
                                "neighbourhood": "MAHALLE",
                                "district": "ILCE",
                                "code": "UAVT"
                            },
                            "streets": { "layer": "yol-ekseni", "name": "YOL_ADI" },
                            "areas": [{ "layer": "mahalle", "level": "neighbourhood", "name": "AD" }],
                            "minScore": 85,
                            "reach": 100.0
                        }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Adres verisini sil".into(),
                    input: json!({ "operation": "remove", "id": "adres-1" }),
                    output: Some(json!({ "locators": [], "revision": "8" })),
                },
            ],
        },
        // Zaman ayarları (docs/adr/0210 §11), held together by fixtures/commands/v1/cad.layers.time.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_TIME.into(),
            version: crate::CAD_LAYERS_TIME_VERSION,
            title: "Katmanın zamanı".into(),
            summary: "Katmanın zaman ayarını yazar ya da kaldırır (time verilmezse ya da null ise): nesnelerin başlangıcı (anlık katmanda anı) ve bitişi hangi özniteliklerdedir, kimlik alanı (iki zamanı karşılaştırmanın anahtarı) ve Birikimli (nesne başlangıcından sonra hep görünür). \
                      Değerler öznitelik metnidir: YYYY-AA-GG ya da GG.AA.YYYY, isteğe bağlı saat (T ya da boşlukla SS:DD[:ss[.kesir]]) ve saat dilimi (Z, +SS:DD …); boş başlangıç ya da bitiş açık uçtur. \
                      Zaman sürgüsü, Yeni sürüm oluştur ve Sona erdir bu ayarı kullanır; sürgü bir görünümdür, çizimi değiştirmez. \
                      Katman ağacının değişikliğidir: tek geri alma adımı “Zaman ayarları”; ayar aynıysa hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersTime>(),
            output: schema::<crate::LayersTimed>(),
            plan: Some(schema::<crate::LayersTimePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parsellerin geçerlilik aralığı, parsel numarası anahtar".into(),
                    input: json!({
                        "layer": "parsel",
                        "time": { "start": "gecerlilik_baslangic", "end": "gecerlilik_bitis", "key": "parsel_no" }
                    }),
                    output: Some(json!({ "layer": "parsel", "changed": true, "revision": "27" })),
                },
                CommandExample {
                    title: "Olaylar: yalnız tarih, birikimli".into(),
                    input: json!({ "layer": "ariza", "time": { "start": "tarih", "cumulative": true } }),
                    output: None,
                },
                CommandExample {
                    title: "Zamanı kaldır".into(),
                    input: json!({ "layer": "parsel" }),
                    output: None,
                },
            ],
        },
        // Senaryo oluştur and Senaryoyu uygula (docs/adr/0210 §9, §11), held together by
        // fixtures/commands/v1/cad.scenarios.edit.json.
        CommandDescriptor {
            id: crate::CAD_SCENARIOS_EDIT.into(),
            version: crate::CAD_SCENARIOS_EDIT_VERSION,
            title: "Senaryo".into(),
            summary: "Bir senaryo oluşturur (create) ya da birini mevcut duruma uygular (apply); tek geri alma adımı, adı işlemin. \
                      Senaryo ağacın en üstünde, senaryo olarak işaretli bir gruptur: verilen ana katmanların (senaryo dışındaki katmanların) her biri için aynı adlı, aynı stil, alanlar, zaman ayarı ve kenetle bir katman açılır, kaynağının yerine geçer (replaces); copyObjects açıksa (varsayılan) nesneleri yeni kimliklerle kopyalanır. Kilitli kaynağın nesneleri kopyalanmaz (layer_locked). \
                      apply: senaryonun bir ana katmanın yerine geçen her katmanı, o katmanın nesnelerini kendi nesneleriyle değiştirir (nesneler kimliklerini korur) ve silinir; grup senaryo olmaktan çıkar, öbür katmanlarıyla sıradan bir grup olarak kalır (katmanı kalmazsa silinir). \
                      Bir senaryoyu göstermek (grubunu görünür, yerine geçtiği ana katmanları gizli yapmak) bir görünümdür, komut değildir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ScenariosEdit>(),
            output: schema::<crate::ScenariosEdited>(),
            plan: Some(schema::<crate::ScenariosEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yol genişletme önerisi: yollar ve parseller kopyalanır".into(),
                    input: json!({
                        "operation": "create",
                        "name": "Yol genişletme A",
                        "layers": ["yol", "parsel"],
                        "note": "12 m'lik yol önerisi"
                    }),
                    output: Some(json!({
                        "scenario": "layer-21",
                        "layers": [{ "base": "yol", "layer": "layer-22" }, { "base": "parsel", "layer": "layer-23" }],
                        "kept": [],
                        "objects": 48,
                        "removed": 0,
                        "revision": "31"
                    })),
                },
                CommandExample {
                    title: "Seçilen öneriyi mevcut duruma uygula".into(),
                    input: json!({ "operation": "apply", "scenario": "layer-21" }),
                    output: None,
                },
            ],
        },
        // Katman süzgeci (docs/adr/0211 §5), held together by fixtures/commands/v1/cad.layers.filter.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_FILTER.into(),
            version: crate::CAD_LAYERS_FILTER_VERSION,
            title: "Katman süzgeci".into(),
            summary: "Katmanın süzgecini yazar ya da kaldırır (filter verilmezse ya da null ise): yalnız süzgeçten geçen nesneler çizilir, seçilir, kenetlenir, Öznitelik tablosunda, İşlemler'de ve Veride ara'da görünür. \
                      Süzgeç bir ifadedir (İfadeyle seç'in dili: Nitelik = 'Arsa' ve $alan > 500; doğru olan geçer, yanlış, boş ya da hatalı olan geçmez; $sıra ve $ölçek kullanılamaz) ya da nesnelerin kalıcı kimlik listesidir (Seçimden süzgeç) ya da ikisi birden (ikisinden de geçmeli). \
                      Süzgeç görünümdür: nesneler silinmez, dosya alışverişi ve komutlar bütün nesnelerle çalışır. \
                      Sonuç süzgeçten geçen ve katmanın bütün nesne sayısıdır. Katman ağacının değişikliğidir: tek geri alma adımı “Katman süzgeci”; süzgeç aynıysa hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersFilter>(),
            output: schema::<crate::LayersFiltered>(),
            plan: Some(schema::<crate::LayersFilterPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yalnız büyük arsalar".into(),
                    input: json!({ "layer": "parsel", "filter": { "expression": "Nitelik = 'Arsa' ve $alan > 500" } }),
                    output: Some(json!({ "layer": "parsel", "changed": true, "passed": 12, "total": 68, "revision": "28" })),
                },
                CommandExample {
                    title: "Yalnız bu iki nesne (seçimden)".into(),
                    input: json!({
                        "layer": "parsel",
                        "filter": { "objects": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2002"] }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Süzgeci kaldır".into(),
                    input: json!({ "layer": "parsel" }),
                    output: None,
                },
            ],
        },
        // Katman çizimi (docs/adr/0226 §3, §4), held together by fixtures/commands/v1/cad.layers.compose.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_COMPOSE.into(),
            version: crate::CAD_LAYERS_COMPOSE_VERSION,
            title: "Katman çizimi".into(),
            summary: "Katmanın çizime nasıl katıldığını yazar: saydamlığı (opacity, 0–1), karışım kipi (blend: normal, multiply, screen, overlay, darken, lighten, colorDodge, colorBurn, hardLight, softLight, difference, exclusion, addition) ve maskesi (mask: katmanın yerleşen etiketleri labels ve işaretleri markers, size mm kadar büyütülüp köşeleri yuvarlanarak targets'taki katmanlarda boşaltılır). \
                      Katmanın çizimi verilen olur: verilmeyen alan varsayılandır (opak, normal, maskesiz); hiçbiri verilmezse katman çizimi kalkar. Saydamlığı 1'den küçük, kipi normal olmayan ya da maskelenen katman ayrı bir resme çizilip çizimin o ana kadarki resmine bu kurallarla birleştirilir. \
                      Katman ağacının değişikliğidir: tek geri alma adımı “Katman çizimi”; aynıysa hiçbir şey yazılmaz. Kilitli katman da alır. Maskelenen katman çizimin başka bir katmanı olmalıdır (kendisi ya da grup değil); sonradan silinen maskelenen katman atlanır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersCompose>(),
            output: schema::<crate::LayersComposed>(),
            plan: Some(schema::<crate::LayersComposePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Gölgeli kabartma çoğaltarak, yarı saydam".into(),
                    input: json!({ "layer": "kabartma", "opacity": 0.6, "blend": "multiply" }),
                    output: Some(json!({ "layer": "kabartma", "changed": true, "revision": "18" })),
                },
                CommandExample {
                    title: "Yol adları alttaki çizgileri boşaltır".into(),
                    input: json!({ "layer": "yol-adi", "mask": { "labels": true, "size": 1.0, "targets": ["yol", "parsel"] } }),
                    output: None,
                },
                CommandExample {
                    title: "Katman çizimini kaldır".into(),
                    input: json!({ "layer": "kabartma" }),
                    output: None,
                },
            ],
        },
        // Bilgi kartı ve eylemler (docs/adr/0225 §4, §5), held together by fixtures/commands/v1/cad.layers.info.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_INFO.into(),
            version: crate::CAD_LAYERS_INFO_VERSION,
            title: "Bilgi kartı ve eylemler".into(),
            summary: "Katmanın eylemlerini ve bilgi kartını yazar. Eylem nesnenin açtığı adres ya da dosyadır: adı ve hedefi (http, https ya da mailto adresi ya da dosya yolu); kart Bilgi aracının nesnede gösterdiğidir: başlık, gösterilecek alanlar (boşsa hepsi), metin, ekler ve eylemler (attachments ve actions false ise gizlenir). \
                      Hedef, başlık ve metin şablondur: {…} arası nesne üstünde değerlendirilen bir ifadedir (İfadeyle seç'in dili: {Ada}, {[Tapu alanı]}, {yuvarla($alan, 2)}); {{ ve }} düz ayraçtır; tek başına {$enlem} ve {$boylam} nesnenin WGS 84'teki yeridir. Adreste değerler yüzde kodlanır. \
                      actions verilmezse eylem yoktur, card verilmezse ya da null ise varsayılan kart gösterilir. Katman ağacının değişikliğidir: tek geri alma adımı “Bilgi kartı”; aynıysa hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersInfo>(),
            output: schema::<crate::LayersInformed>(),
            plan: Some(schema::<crate::LayersInfoPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parselin kartı ve iki eylemi".into(),
                    input: json!({
                        "layer": "parsel",
                        "actions": [
                            { "name": "Parsel sorgu", "target": "https://parselsorgu.example/{Ada}/{Parsel}" },
                            { "name": "Haritada aç", "target": "https://www.google.com/maps?q={$enlem},{$boylam}" }
                        ],
                        "card": { "title": "Parsel {Ada}/{Parsel}", "fields": ["Malik", "Nitelik"], "text": "Alan: {yuvarla($alan, 2)} m²" }
                    }),
                    output: Some(json!({ "layer": "parsel", "changed": true, "revision": "31" })),
                },
                CommandExample {
                    title: "Varsayılan kart, eylemsiz".into(),
                    input: json!({ "layer": "parsel" }),
                    output: None,
                },
            ],
        },
        // Etiketler (docs/adr/0212 §5), held together by fixtures/commands/v1/cad.layers.labels.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_LABELS.into(),
            version: crate::CAD_LAYERS_LABELS_VERSION,
            title: "Etiketler".into(),
            summary: "Katmanın etiketlemesini yazar: label tek etiketin stilidir (metin şablonu ya da ifadesi, boy, renk, hale, zemin, gölge, çağrı çizgisi; noktada çevresinde ya da üstünde, çizgide paralel, kıvrık, yatay ya da eş yükselti, alanda yatay, eğik, çevre boyunca, sınır, parsel ya da köşe; yığma, kısaltma sözlüğü, küçültme, dışarıda; öncelik 0–10, çakışma, yinelenenler), labels katmanın nasıl etiketlendiğidir: single (tek etiket), rules (sıralı sınıflar: adı, koşulu ve kendi stili; koşulu tutan her sınıf nesneyi etiketler) ya da off (etiketsiz), ve obstacle ile nesneleri öbür etiketlere engeldir (ağırlık 1–10; interior ya da boundary). \
                      Verilmeyen değişmez, null kaldırır (label null: türün varsayılan etiketi; labels null: tek etiket). İfadeler İfadeyle seç'in dilidir; $sıra ve $ölçek kullanılamaz. \
                      Katman ağacının değişikliğidir: tek geri alma adımı “Etiketler”; hepsi aynıysa hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersLabels>(),
            output: schema::<crate::LayersLabelled>(),
            plan: Some(schema::<crate::LayersLabelsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parsel numarası ve maliki, kurallı".into(),
                    input: json!({
                        "layer": "parsel",
                        "labels": {
                            "mode": "rules",
                            "classes": [
                                { "name": "No", "style": { "placement": "center", "size": 11, "weight": 600, "text": "Parsel", "area": "parcel",
                                                          "stack": { "mode": "ifNeeded", "chars": 6 }, "outside": true, "callout": { "kind": "straight" }, "priority": 7 } },
                                { "name": "Malik", "when": "Malik <> ''", "style": { "placement": "center", "size": 9, "italic": true, "text": "Malik", "area": "free",
                                                                                    "inside": true, "priority": 3,
                                                                                    "abbreviate": { "words": [{ "word": "Arazisi", "short": "Ar." }] } } }
                            ]
                        }
                    }),
                    output: Some(json!({ "layer": "parsel", "changed": true, "revision": "29" })),
                },
                CommandExample {
                    title: "Yol adları kıvrık, birleşik ve yinelenerek".into(),
                    input: json!({
                        "layer": "yol",
                        "label": { "placement": "along", "size": 11, "line": "curved", "mergeLines": true, "repeat": 600, "duplicates": 250 }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Binalar etiketsiz, etiketlere engel".into(),
                    input: json!({ "layer": "bina", "labels": { "mode": "off", "obstacle": { "weight": 7 } } }),
                    output: None,
                },
            ],
        },
        // Etiket (docs/adr/0212 §5), held together by fixtures/commands/v1/cad.labels.pin.json.
        CommandDescriptor {
            id: crate::CAD_LABELS_PIN.into(),
            version: crate::CAD_LABELS_PIN_VERSION,
            title: "Etiketi sabitle".into(),
            summary: "Nesnelerin etiketlerini elle sabitler, döndürür, gizler ya da serbest bırakır: her değişiklik bir nesnenin bir etiketidir (class: kurallı katmanın sınıfının adı; yoksa nesnenin ilk etiketi), pin etiketin ortasının nesnenin çapasından uzaklığı (at, metre), açısı (rotation, derece, saat yönünün tersine) ya da gizliliğidir (hidden: true); pin null etiketi serbest bırakır, motor yeniden yerleştirir. \
                      Sabit etiket her zaman yerinde çizilir, öbür etiketler ondan kaçar; nesne taşınınca, kopyalanınca etiket onunla gider. \
                      Tek geri alma adımı “Etiket”; hiçbir iğne değişmiyorsa hiçbir şey yazılmaz. Kilitli katmandaki nesnenin etiketi değişmez. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LabelsPin>(),
            output: schema::<crate::LabelsPinned>(),
            plan: Some(schema::<crate::LabelsPinPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parselin numarasını taşı ve döndür".into(),
                    input: json!({
                        "pins": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2001", "class": "No", "pin": { "class": "No", "at": { "x": -8.0, "y": 6.0 }, "rotation": 20.0 } }]
                    }),
                    output: Some(json!({ "changed": 1, "revision": "30" })),
                },
                CommandExample {
                    title: "Nokta adını gizle".into(),
                    input: json!({ "pins": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2002", "pin": { "hidden": true } }] }),
                    output: None,
                },
                CommandExample {
                    title: "Etiketi serbest bırak".into(),
                    input: json!({ "pins": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2001", "class": "No", "pin": null }] }),
                    output: None,
                },
            ],
        },
        // Katman stili (docs/adr/0213 §5), held together by fixtures/commands/v1/cad.layers.renderer.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_RENDERER.into(),
            version: crate::CAD_LAYERS_RENDERER_VERSION,
            title: "Katman stili".into(),
            summary: "Katmanın işleyicisini yazar ya da kaldırır (renderer verilmezse ya da null ise katmanın basit görünüşü): single (tek sembol), categorized, graduated, rules ve tematik işleyiciler: \
                      unclassed (sürekli renk: değerin [min, max]'taki payı rampada bir renk, sembolün ana rengi), proportional (orantılı sembol: değer sembolün boyu; Alan, Yarıçap ya da Flannery), \
                      bivariate (iki değişkenli renk: iki değerin sınıfları n × n ızgarada bir renk), dotDensity (nokta yoğunluğu: alanın içinde değer / dotValue kadar nokta), chart (pasta, çubuk ya da yığılmış çubuk), \
                      heatmap (ısı haritası: noktaların dördüncü dereceden (quartic) çekirdekle yoğunluğu, rampa; en büyük değer sabit ya da dinamik), cluster (kümeleme: yakın noktalar tek işaret ve sayısı), \
                      displacement (yayma: üst üste binen noktalar halka, iç içe halkalar ya da ızgarada) ve inverted (ters alan: alanların dışı boyanır). \
                      Semboller kitaplıktan { ref } ya da yerinde yazılır; ifadeler İfadeyle seç'in dilidir. Katmanın görünüşüdür: kilitli katman da alır; tek geri alma adımı “Katman stili”; aynıysa hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersRenderer>(),
            output: schema::<crate::LayersRendered>(),
            plan: Some(schema::<crate::LayersRendererPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Mahalleler nüfusa göre sürekli renk".into(),
                    input: json!({
                        "layer": "mahalle",
                        "renderer": {
                            "type": "unclassed", "expr": "Nüfus", "min": 0, "max": 40000,
                            "ramp": ["#FFF5B8", "#FDB863", "#E66101", "#A50F15"],
                            "symbols": { "fill": { "type": "fill", "layers": [
                                { "type": "simpleFill", "color": "#000000" },
                                { "type": "simpleLine", "color": "#FFFFFF", "width": 0.2 }
                            ] } }
                        }
                    }),
                    output: Some(json!({ "layer": "mahalle", "changed": true, "revision": "30" })),
                },
                CommandExample {
                    title: "Olayların ısı haritası".into(),
                    input: json!({
                        "layer": "olay",
                        "renderer": { "type": "heatmap", "radius": 24, "unit": "px", "quality": 2,
                                      "ramp": ["#2B83BA00", "#2B83BA", "#ABDDA4", "#FFFFBF", "#FDAE61", "#D7191C"] }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Yakın noktaları kümele".into(),
                    input: json!({ "layer": "olay", "renderer": { "type": "cluster", "distance": 40, "unit": "px", "grow": true } }),
                    output: None,
                },
                CommandExample {
                    title: "Basit görünüşe dön".into(),
                    input: json!({ "layer": "mahalle", "renderer": null }),
                    output: None,
                },
            ],
        },
        // The layer tree (docs/adr/0248 §1), held together by fixtures/commands/v1/cad.layers.edit.json.
        // The web is paused (docs/adr/0228): the desktop's host alone runs these five for now.
        CommandDescriptor {
            id: crate::CAD_LAYERS_EDIT.into(),
            version: crate::CAD_LAYERS_EDIT_VERSION,
            title: "Katman düzenle".into(),
            summary: "Katman ağacını tek geri alma adımıyla değiştirir (operation): add yeni katman ya da grup ekler (kind; name verilmezse “Yeni katman”, “Yeni grup”, ağaçta bir tane olacak biçimde; parent bir grup ya da grubuna girilecek katman, verilmezse en üst; index gruptaki yeri, üstte 0; color, lineType, lineWeight, visible, locked; activate yeni katmanı etkin yapar), \
                      style katmanın ya da grubun rengini (#rrggbb ya da fg, fg-dim, ink, paper), çizgi tipini ve kalınlığını (0–100 mm) yazar, kilitli düğüm de alır; move düğümü bir gruba (parent; verilmezse en üst) ve yerine (index) taşır; \
                      remove katmanı ya da grubu altındakiler ve üzerlerindeki nesnelerle siler (son katman, etkin katman ve kilitli düğüm silinmez); duplicate katmanı stili, görünürlüğü, alanları ve nesnelerinin kopyalarıyla aynı gruba “<ad> kopyası” olarak kopyalar; \
                      merge kaynak katmanların (layers) nesnelerini hedef katmana (layer) taşır ve kaynakları siler, etkin katman kaynaksa hedef etkin olur; fields katmanın alanlarını yazar ve renames ile nesnelerinin anahtarlarını yeniden adlandırır; \
                      purge boş katmanları, boş kalan grupları ve hiçbir yerde kullanılmayan blok tanımlarını siler (layers ve blocks verilirse yalnız onları; kalanların kullandığı kalır, kept'te sayılır). \
                      Hiçbir şey değişmezse hiçbir şey yazılmaz (changed false). Adım adı işlemin adıdır: Katman ekle, Grup ekle, Katman rengi, Çizgi tipi, Çizgi kalınlığı, Katman stili, Katmanı taşı, Katman sil, Katmanı kopyala, Katmanları birleştir, Alanlar, Kullanılmayanları temizle. \
                      Ad, görünürlük, kilit ve etkin katman cad.layers.set'tedir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersEdit>(),
            output: schema::<crate::LayersEdited>(),
            plan: Some(schema::<crate::LayersEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Ofis şablonunun bir katmanı: Duvar, kırmızı, 0,5 mm, etkin".into(),
                    input: json!({ "operation": "add", "name": "Duvar", "color": "#FF0000", "lineWeight": 0.5, "activate": true }),
                    output: Some(json!({ "layer": "layer-12", "changed": true, "objects": 0, "removed": [], "blocks": [], "kept": 0, "revision": "9" })),
                },
                CommandExample {
                    title: "Mimari grubuna taşı, en üste".into(),
                    input: json!({ "operation": "move", "layer": "layer-12", "parent": "mimari", "index": 0 }),
                    output: None,
                },
                CommandExample {
                    title: "Eski katmanları Bina katmanına birleştir".into(),
                    input: json!({ "operation": "merge", "layer": "bina", "layers": ["bina-eski", "yapi"] }),
                    output: Some(json!({ "layer": "bina", "changed": true, "objects": 48, "removed": ["bina-eski", "yapi"], "blocks": [], "kept": 0, "revision": "12" })),
                },
                CommandExample {
                    title: "Kullanılmayanları temizle".into(),
                    input: json!({ "operation": "purge" }),
                    output: None,
                },
            ],
        },
        // The tree's own changes (docs/adr/0248 §2), held together by fixtures/commands/v1/cad.layers.set.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_SET.into(),
            version: crate::CAD_LAYERS_SET_VERSION,
            title: "Katman ağacı".into(),
            summary: "Katmanlar panelinin göz, kilit, ad ve etkin katman değişikliklerini yapar (operation): rename (layer, name), visible ve locked (layer, value; grubun gizliliği ve kilidi altındakilerin hepsine geçer), \
                      active (layer: nesne alan bir katman; grup ve servis katmanı olmaz), isolate (layers: bu düğümler, üstlerindeki gruplar ve altlarındakiler görünür, gerisi gizlenir) ve showAll (hepsi görünür). \
                      Bunlar ağacın kendi değişiklikleridir: çizimi kirli yapar ve sürümü değiştirir ama geri alma adımı değildir; etkin katman dosyada saklanır ama düzenleme sayılmaz. Değişmeyen hiçbir şey yazmaz (changed false). \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersSet>(),
            output: schema::<crate::LayersChanged>(),
            plan: Some(schema::<crate::LayersChanged>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bina katmanını kilitle".into(),
                    input: json!({ "operation": "locked", "layer": "bina", "value": true }),
                    output: Some(json!({ "changed": true, "active": "parsel", "revision": "14" })),
                },
                CommandExample {
                    title: "Yalnız Kadastro grubu görünsün".into(),
                    input: json!({ "operation": "isolate", "layers": ["kadastro"] }),
                    output: None,
                },
            ],
        },
        // Katman durumları (docs/adr/0248 §3), held together by fixtures/commands/v1/cad.layers.states.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_STATES.into(),
            version: crate::CAD_LAYERS_STATES_VERSION,
            title: "Katman durumları".into(),
            summary: "Projenin adlı katman durumlarını yönetir (operation): save ağacın şimdiki görünürlüğünü, istenirse kilitlerini (locks) ve katmanların stillerini (styles, verilmezse açık) yeni bir durum olarak kaydeder (name verilmezse “Durum n”); \
                      update durumu (state) şimdiki ağaçtan yeniden kaydeder; rename durumu yeniden adlandırır; remove siler; apply durumu uygular: adını verdiği ve ağaçta hâlâ olan düğümlerin görünürlüğü ve (saklandıysa) kilidi ağacın kendi değişikliği olarak, \
                      katmanların stilleri tek geri alma adımında (“Katman durumu: <ad>”) yazılır; ağaçta olmayan düğümler missing'de sayılır. Durumlar proje ayarıdır: kaydetmek, güncellemek, adlandırmak ve silmek çizimi kirli yapar, geri alma adımı değildir. \
                      Adlar baştaki ve sondaki boşluklar sayılmadan bir kez kullanılır. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersStates>(),
            output: schema::<crate::LayerStatesDone>(),
            plan: Some(schema::<crate::LayerStatesDone>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Pafta görünümünü kilitleriyle kaydet".into(),
                    input: json!({ "operation": "save", "name": "Pafta", "locks": true }),
                    output: Some(json!({ "state": "durum-1", "changed": true, "visible": 0, "locked": 0, "styles": 0, "missing": 0, "revision": "15" })),
                },
                CommandExample {
                    title: "Pafta görünümüne dön".into(),
                    input: json!({ "operation": "apply", "state": "durum-1" }),
                    output: None,
                },
            ],
        },
        // Yazı ve ölçü stilleri (docs/adr/0248 §4), held together by fixtures/commands/v1/cad.styles.edit.json.
        CommandDescriptor {
            id: crate::CAD_STYLES_EDIT.into(),
            version: crate::CAD_STYLES_EDIT_VERSION,
            title: "Yazı ve ölçü stilleri".into(),
            summary: "Projenin adlı yazı (kind text) ya da ölçü (kind dimension) stillerinden birini değiştirir (operation): create yeni stili tablonun sonuna ekler (kimliği boşsa yeni bir UUIDv7 verilir), \
                      update stilin değerlerini değiştirir: o stildeki yazılar (ölçüler) eski değeri hâlâ taşıyan alanlarında yenisini alır, elle değiştirilenler kalır; rename stilin adını değiştirir, nesneler değişmez; \
                      remove stili siler, nesneleri görünüşlerini korur ve bağlarını yitirir. Adlar büyük küçük harf ayırmadan bir kez kullanılır, Standart başka stile verilmez. \
                      Tablo proje ayarıdır: değiştirmek çizimi kirli yapar, geri alma adımı değildir; izleyen nesneler tek geri alma adımında (“Yazı stili”, “Ölçü stili”) yazılır, kilitli katmandakiler değişmez ve locked'da sayılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::StylesEdit>(),
            output: schema::<crate::StylesEdited>(),
            plan: Some(schema::<crate::StylesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Ada no yazı stili: Arimo, kalın, 3,5 mm".into(),
                    input: json!({
                        "kind": "text",
                        "operation": "create",
                        "text": { "id": "", "name": "Ada no", "font": "arimo", "bold": true, "height": 3.5 }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Mimari ölçü stilini sil".into(),
                    input: json!({ "kind": "dimension", "operation": "remove", "style": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0201" }),
                    output: Some(json!({ "style": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0201", "changed": true, "followed": 12, "locked": 0, "revision": "21" })),
                },
            ],
        },
        // Proje ayarları by key (docs/adr/0248 §5), held together by fixtures/commands/v1/cad.settings.set.json.
        CommandDescriptor {
            id: crate::CAD_SETTINGS_SET.into(),
            version: crate::CAD_SETTINGS_SET_VERSION,
            title: "Proje ayarları".into(),
            summary: "Projenin kendi ayarlarından verilenleri değiştirir, verilmeyenler kalır: name (çizimin adı), projectType (cad ya da gis), srid (kayıttaki bir EPSG kodu ya da yerel proje için 0; atanır, koordinatlar dönüştürülmez; tanımlı sistemin yerini alır, sistem birimi metre yapar), \
                      secondSrid (kayıttaki, projeninkinden başka bir EPSG kodu ya da yok için 0), drawingUnit (yalnız sistemsiz yerel projede mm, cm ya da m), lengthDecimals ve areaDecimals (0–4), areaUnit (m2, donum, ha), angleUnit (grad, deg), \
                      plotScale (1 ile 1 000 000 arası tam sayı payda; eski genel yükseklikteki yazılar, kılavuzlar, ölçüler ve tablolar yenisini tek geri alma adımında, “Yazı yüksekliklerini uydur”, alır) ve drawingFont. \
                      Ayarlar projenin düzenlemesidir, geri alma adımı değildir. Kullanıcının ve cihazın ayarları (tema, grafik, kısayollar) projenin değildir; bilinmeyen anahtar reddedilir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SettingsSet>(),
            output: schema::<crate::SettingsChanged>(),
            plan: Some(schema::<crate::SettingsChanged>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Mimari proje: CAD, 1:100, yerel ve milimetre".into(),
                    input: json!({ "projectType": "cad", "srid": 0, "drawingUnit": "mm", "plotScale": 100 }),
                    output: Some(json!({ "changed": ["projectType", "srid", "drawingUnit", "plotScale"], "followed": 0, "locked": 0, "revision": "3" })),
                },
                CommandExample {
                    title: "TUREF / TM33 ata, WGS 84'ü ikinci sistem yap".into(),
                    input: json!({ "srid": 5256, "secondSrid": 4326 }),
                    output: None,
                },
            ],
        },
        // A local project's unit, keeping the sizes or the numbers (docs/adr/0256 §1), held together
        // by fixtures/commands/v1/cad.units.change.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_UNITS_CHANGE.into(),
            version: crate::CAD_UNITS_CHANGE_VERSION,
            title: "Birimi değiştir".into(),
            summary: "Yerel projenin çizim birimini from'dan to'ya değiştirir (mm, cm, m); from çizimin şimdiki birimi olmalıdır. geometry ne olacağını söyler: \
                      keep boyları korur, yalnız okuma ve yazma birimi değişir (4000 mm'lik duvar 4 m okunur, hiçbir nesne değişmez); \
                      scale sayıları korur, çizim iki birimin oranıyla 0,0'a göre ölçeklenir (4000 mm 4000 m okunur: ×1000; cm'den mm'ye ÷10). \
                      scale'de nesnelerin koordinatları, uzunlukları, yarıçapları, kotları, yazı, ölçü, kılavuz ve tablo yükseklikleri ve boyları, paragraf kutusu, eğri boyunca yazının eğrisi, tarama aralığı ve desen ölçeği, resim ve raster çerçeveleri, blok tanımları, ana görünüm ölçeklenir; \
                      açılar, oranlar, yerleştirmenin kendi ölçeği, kâğıt üstü boylar ve ölçüler (M) kalır. Nokta bulutlarının noktaları, pafta kitabı ve projenin toleransları değişmez; uyarı söyler. \
                      Telde geometri her durumda metredir. Birim ve ölçeklenen çizim tek geri alma adımıdır (“Birimi değiştir”). Koordinat sistemli projenin birimi sistemindir (unit_needs_local); scale'de kilitli katmanda nesne varsa hiçbir şey yazılmaz (layer_locked). \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::UnitsChange>(),
            output: schema::<crate::UnitsChanged>(),
            plan: Some(schema::<crate::UnitsChanged>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Milimetreden metreye, boylar korunarak".into(),
                    input: json!({ "from": "mm", "to": "m", "geometry": "keep" }),
                    output: Some(json!({ "from": "mm", "to": "m", "geometry": "keep", "factor": 1.0, "objects": 0, "blocks": 0, "revision": "8" })),
                },
                CommandExample {
                    title: "Metre sanılan milimetrelik çizim: sayılar korunur, çizim binde birine iner".into(),
                    input: json!({ "from": "m", "to": "mm", "geometry": "scale" }),
                    output: Some(json!({ "from": "m", "to": "mm", "geometry": "scale", "factor": 0.001, "objects": 412, "blocks": 6, "revision": "9" })),
                },
            ],
        },
        // A DXF into the drawing, its unit decided by one rule (docs/adr/0257), held together by
        // fixtures/commands/v1/file.import.dxf.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::FILE_IMPORT_DXF.into(),
            version: crate::FILE_IMPORT_DXF_VERSION,
            title: "DXF içe aktar".into(),
            summary: "path'teki DXF dosyasını açık çizime alır, masaüstünün DXF içe aktar penceresi gibi: her kaynak katmanın nesneleri çizimde aynı adlı katmana (büyük küçük harf ve Türkçe işaretler bir), yoksa dosyanın görünüşüyle yeni bir katmana gider; \
                      yeni katmanlar dosyanın adındaki gruba girer (group ile başka bir grup, boş metinle ağacın üstü). layers bir kaynak katmanı var olan bir katmana yönlendirir, skipLayers kaynak katmanları dışarıda bırakır. \
                      Bloklar tanım olarak (çizimde aynı adlı blok varsa yeni adla, renamed söyler), explodeBlocks ile patlatılarak gelir; dosyanın yazı ve ölçü stilleri projeye eklenir. \
                      Birim (unit): fileUnit verilmişse değerler o birimdedir; verilmemişse yerel projede dosyanın $INSUNITS'te bildirdiği birim, koordinat sistemli projede metre kullanılır (başka bildirilen birim unit_ignored ile söylenir). \
                      Birim bildirmeyen ya da tanınmayan kod bildiren dosyanın sayıları ölçeklenmez: projenin biriminde (sistemli projede metre) sayılır ve unit_assumed söyler; birimi başka bloklar block_units_differ ile söylenir. \
                      Telde geometri metredir. Bütün alınanlar tek geri alma adımıdır (“DXF: ad.dxf”); plan dosyayı okur, hiçbir şey yazmaz ve aynısını söyler. Çıktıdaki sha256 okunan dosyanındır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["dxfin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportDxf>(),
            output: schema::<crate::DxfImported>(),
            plan: Some(schema::<crate::DxfImported>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Milimetrelik plan metrelik projeye".into(),
                    input: json!({ "path": "/plan/kat-plani.dxf" }),
                    output: Some(json!({
                        "file": "kat-plani.dxf", "bytes": 48213, "sha256": "9f2c4c0e1b7a3d5e8f6a2b4c6d8e0f1a3b5c7d9e1f2a4b6c8d0e2f4a6b8c0d2e",
                        "unit": { "declared": 4, "used": 4, "name": "milimetre", "source": "declared", "factor": 0.001, "assumed": false },
                        "closedPolylines": "line",
                        "objects": 312, "targets": { "DUVAR": "import-duvar", "0": "cizim" },
                        "layers": [{ "id": "import-duvar", "name": "DUVAR" }], "group": "import-kat-plani-dxf",
                        "blocks": 2, "textStyles": 1, "dimensionStyles": 0, "report": {}, "revision": "12"
                    })),
                },
                CommandExample {
                    title: "Birim bildirmeyen dosya, santimetre olduğu bilinerek, ağacın üstüne".into(),
                    input: json!({ "path": "/plan/eski.dxf", "fileUnit": "cm", "group": "", "skipLayers": ["DEFPOINTS"] }),
                    output: None,
                },
            ],
        },
        // The sheet book by the sheet core's own operations (docs/adr/0252 §2), held together by
        // fixtures/commands/v1/cad.sheets.edit.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_SHEETS_EDIT.into(),
            version: crate::CAD_SHEETS_EDIT_VERSION,
            title: "Pafta kitabı".into(),
            summary: "Projenin pafta kitabını pafta çekirdeğinin kendi işlemleriyle değiştirir (kentos.sheet/1; pafta tasarımcısının yaptıkları): ops sırayla ve tek değişiklik olarak uygulanır. \
                      Her işlem op ile adlanan bir nesnedir: addSheet (sayfası, öğeleriyle bir pafta), removeSheet, renameSheet, setPage, addItems (harita, antet, ölçek çubuğu, kuzey oku, yazı …), setItemProps, removeItems, applyTemplate, saveVariables ve ötekiler. \
                      Kâğıt üstündeki uzunluklar mikrometre (A3 yatay 420000 × 297000), açılar derecenin binde biridir; haritanın görünümü projenin koordinatlarında merkez ve ölçek paydasıyla verilir. \
                      Kitap kâğıdındır: değiştirmek çizimi kirli yapar, çizimin geri alma adımı değildir; sonuçtaki inverse değişikliği geri alan işlemlerdir (bu komuta yeniden verilir). Sonuç kitabın paftalarını boyları ve haritalarının ölçekleriyle sayar. \
                      Çekirdeğin okumadığı ya da reddettiği işlem hiçbir şey yazdırmaz; ret çekirdeğin kodu ve ops[i] yoluyla döner. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SheetsEdit>(),
            output: schema::<crate::SheetsEdited>(),
            plan: Some(schema::<crate::SheetsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "A3 yatay pafta, 1:50 haritayla".into(),
                    input: json!({ "ops": [{
                        "op": "addSheet",
                        "sheet": {
                            "id": "kat-plani", "name": "Kat planı",
                            "page": { "paper": "a3", "orientation": "landscape",
                                      "size": { "width": 420000, "height": 297000 },
                                      "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } },
                            "items": [{
                                "id": "harita", "name": "Harita",
                                "frame": { "left": 20000, "top": 10000, "width": 390000, "height": 277000 },
                                "kind": { "type": "map", "view": { "type": "fixed", "center": { "x": 2.0, "y": 1.5 }, "scale": 50, "rotation": 0 } }
                            }]
                        }
                    }] }),
                    output: Some(json!({
                        "label": "Pafta ekle: Kat planı",
                        "inverse": [{ "op": "removeSheet", "id": "kat-plani" }],
                        "sheets": [{ "id": "kat-plani", "name": "Kat planı", "width": 420.0, "height": 297.0, "items": 1,
                                     "maps": [{ "item": "harita", "name": "Harita", "scale": 50 }] }],
                        "revision": "12"
                    })),
                },
                CommandExample {
                    title: "Paftayı sil".into(),
                    input: json!({ "ops": [{ "op": "removeSheet", "id": "kat-plani" }] }),
                    output: None,
                },
            ],
        }];
    commands.extend(protocol_commands());
    for d in &mut commands {
        if d.capabilities.is_none() {
            d.capabilities = crate::capabilities::capabilities_of(&d.id);
        }
    }
    CommandCatalog {
        format: CATALOG_FORMAT.into(),
        version: CATALOG_VERSION,
        commands,
        codes: crate::codes::CODES
            .iter()
            .map(|c| CatalogCode {
                code: c.code.into(),
                kind: match c.kind {
                    crate::codes::CodeKind::Failed => "failed",
                    crate::codes::CodeKind::Conflict => "conflict",
                    crate::codes::CodeKind::NeedsInput => "needsInput",
                    crate::codes::CodeKind::Warning => "warning",
                    crate::codes::CodeKind::FailedOrWarning => "failedOrWarning",
                    crate::codes::CodeKind::Host => "host",
                }
                .into(),
                retryable: c.retryable,
                meaning: c.meaning.into(),
            })
            .collect(),
        protocol: crate::capabilities::PROTOCOL_CODES
            .iter()
            .map(|c| (*c).to_owned())
            .collect(),
        units: crate::units::UNITS
            .iter()
            .map(|(unit, meaning)| CatalogUnit {
                unit: (*unit).into(),
                meaning: (*meaning).into(),
            })
            .collect(),
    }
}

/// The protocol's commands (docs/adr/0253 §8, §10): the change set, and the
/// files a drawing is written to.
#[cfg(feature = "schema")]
fn protocol_commands() -> Vec<CommandDescriptor> {
    use serde_json::json;
    vec![
        CommandDescriptor {
            id: crate::CAD_CHANGESET_APPLY.into(),
            version: crate::CAD_CHANGESET_APPLY_VERSION,
            title: "Değişiklik kümesi".into(),
            summary: "Birden çok komutu tek iş olarak yazar: steps sırayla, aynı adımın içinde çalışır; biri reddederse hepsi geri alınır, proje ayarları, stil kitaplığı ve pafta kitabı da. \
                      Sonraki adım önceki bir adımın çıktısını tam değer olan bir metinle adlar: \"$ad\" o adımın uid'i, \"$ad[2]\" created[2]'si, \"$ad.layer\" layer'ı (ad, adımın as'i). \
                      Bağlı yazılar, taramalar ve ölçüler son adımdan sonra bir kez izler, çizimin bütünlüğü o zaman denetlenir. Nesneler, katmanlar ve bloklar label adlı tek geri alma adımıdır; ayarlar, stiller ve pafta kitabı geri alma adımı değildir, özet hangisinin değiştiğini söyler. \
                      İç içe küme ve dosyaya yazan komut girmez. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["degisiklik kumesi".into(), "changeset".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ChangesetApply>(),
            output: schema::<crate::ChangesetApplied>(),
            plan: Some(schema::<crate::ChangesetApplied>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Yeni katmana iki köşeli bir alan ve onu öteleyen bir kopya".into(),
                input: json!({
                    "label": "Parsel ve çekme hattı",
                    "steps": [
                        { "command": "cad.layers.edit", "as": "katman",
                          "input": { "operation": "add", "name": "Çekme hattı" } },
                        { "command": "cad.polygon.create", "as": "parsel",
                          "input": { "layerId": "0", "pts": [ { "x": 0.0, "y": 0.0 }, { "x": 20.0, "y": 0.0 }, { "x": 20.0, "y": 30.0 }, { "x": 0.0, "y": 30.0 } ] } },
                        { "command": "cad.entities.offset",
                          "input": { "uids": ["$parsel"], "by": { "kind": "distance", "distance": 5.0, "side": { "x": 10.0, "y": 15.0 } }, "layerId": "$katman.layer" } }
                    ]
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_EXPORT_DXF.into(),
            version: crate::FILE_EXPORT_DXF_VERSION,
            title: "DXF dışa aktar".into(),
            summary: "Çizimi AutoCAD 2007 DXF dosyası olarak yazar: bütün nesneler ya da layers'ın katmanlarındakiler, katmanlarıyla; DXF'in tutamadıkları KentOS verisiyle. \
                      Dosya yarım kalmaz: geçici adla yazılır, diske indirilir, geri okunup denetlenir, sonra tek adımda yerine konur; eski dosya ancak yenisi tamsa gider. overwrite verilmezse var olan dosya reddedilir (file_exists). \
                      plan dosyayı yazmadan yazılacak olanı söyler (nesneler, katmanlar, baytlar, yazıcının raporu). Çizimi değiştirmez ve geri alma adımı değildir: çizimin geri alması dosyayı geri almaz."
                .into(),
            aliases: vec!["dxf".into(), "dxfout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportDxf>(),
            output: schema::<crate::DxfExported>(),
            plan: Some(schema::<crate::DxfExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bütün çizim".into(),
                input: json!({ "path": "/tmp/kat-plani.dxf", "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_EXPORT_PDF.into(),
            version: crate::FILE_EXPORT_PDF_VERSION,
            title: "PDF olarak yazdır".into(),
            summary: "Projenin pafta kitabını PDF olarak yazar: sheets'teki paftalar (yoksa hepsi, kitabın sırasıyla), her biri bir sayfa; projenin sistemi transverse Mercator ise GeoPDF. \
                      Pencere olmadan çizilemeyen (haritaların yazıları) notes'ta, pafta çekirdeğinin bulguları findings'te söylenir. Dosya DXF'teki gibi yarım kalmadan yazılır; overwrite verilmezse var olan dosya reddedilir. \
                      plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir: çizimin geri alması dosyayı geri almaz."
                .into(),
            aliases: vec!["pdf".into(), "yazdir".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Job,
            input: schema::<crate::ExportPdf>(),
            output: schema::<crate::PdfExported>(),
            plan: Some(schema::<crate::PdfExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kitabın bütün paftaları".into(),
                input: json!({ "path": "/tmp/paftalar.pdf" }),
                output: None,
            }],
        },
    ]
}

#[cfg(all(test, feature = "schema"))]
mod tests {
    /// docs/adr/0256 §3: every number of every command's input, output and
    /// plan says its unit, on itself or on the field holding it; and each
    /// unit is one the catalog explains.
    #[test]
    fn every_number_names_its_unit() {
        use serde_json::Value;
        use std::collections::BTreeSet;

        /// The types that take their unit from the field holding them: a
        /// missing unit is the holder's, as the holder's field is annotated.
        const HELD: [&str; 2] = ["Vec2", "Bounds"];

        #[allow(clippy::too_many_arguments)]
        fn walk(
            s: &Value,
            defs: &serde_json::Map<String, Value>,
            unit: Option<&str>,
            at: &str,
            held: Option<&str>,
            seen: &mut BTreeSet<(String, Option<String>, Option<String>)>,
            missing: &mut BTreeSet<String>,
            wrong: &mut BTreeSet<String>,
        ) {
            let Some(map) = s.as_object() else {
                return;
            };
            if map.contains_key("const") || map.contains_key("enum") {
                return;
            }
            let own = map.get("x-unit").and_then(Value::as_str);
            if let Some(u) = own
                && !crate::units::is_unit(u)
            {
                wrong.insert(format!("{at}: {u}"));
            }
            let unit = own.or(unit);
            let numeric = |t: &Value| matches!(t.as_str(), Some("number" | "integer"));
            let is_number = match map.get("type") {
                Some(Value::Array(ts)) => ts.iter().any(numeric),
                Some(t) => numeric(t),
                None => false,
            };
            if is_number && unit.is_none() {
                missing.insert(held.unwrap_or(at).to_owned());
            }
            if let Some(r) = map.get("$ref").and_then(Value::as_str) {
                let name = r.rsplit('/').next().unwrap_or_default().to_owned();
                let held = if HELD.contains(&name.as_str()) {
                    Some(held.unwrap_or(at))
                } else {
                    None
                };
                if seen.insert((
                    name.clone(),
                    unit.map(str::to_owned),
                    held.map(str::to_owned),
                )) && let Some(def) = defs.get(&name)
                {
                    walk(def, defs, unit, &name, held, seen, missing, wrong);
                }
            }
            if let Some(Value::Object(props)) = map.get("properties") {
                for (k, v) in props {
                    walk(
                        v,
                        defs,
                        unit,
                        &format!("{at}.{k}"),
                        held,
                        seen,
                        missing,
                        wrong,
                    );
                }
            }
            for k in ["items", "additionalProperties"] {
                if let Some(v) = map.get(k) {
                    walk(v, defs, unit, at, held, seen, missing, wrong);
                }
            }
            for k in ["anyOf", "oneOf", "allOf", "prefixItems"] {
                for v in map.get(k).and_then(Value::as_array).into_iter().flatten() {
                    walk(v, defs, unit, at, held, seen, missing, wrong);
                }
            }
        }

        let (mut missing, mut wrong) = (BTreeSet::new(), BTreeSet::new());
        for d in super::catalog().commands {
            for (part, schema) in [
                ("input", Some(&d.input)),
                ("output", Some(&d.output)),
                ("plan", d.plan.as_ref()),
            ] {
                let Some(schema) = schema else {
                    continue;
                };
                let empty = serde_json::Map::new();
                let defs = schema
                    .get("$defs")
                    .and_then(Value::as_object)
                    .unwrap_or(&empty);
                let mut seen = BTreeSet::new();
                walk(
                    schema,
                    defs,
                    None,
                    &format!("{}:{part}", d.id),
                    None,
                    &mut seen,
                    &mut missing,
                    &mut wrong,
                );
            }
        }
        assert!(
            wrong.is_empty(),
            "units the catalog does not explain: {wrong:#?}"
        );
        assert!(
            missing.is_empty(),
            "{} numbers without a unit (x-unit): {missing:#?}",
            missing.len()
        );
    }

    use super::*;
    use crate::{
        EntitiesDelete, EntitiesDeleted, LineCreate, LineCreated, PolygonCreate, PolygonCreated,
        PolylineCreate, PolylineCreated, ProjectAccessRevoke, ProjectChanges, ProjectPermission,
        ProjectShare,
    };
    use std::path::PathBuf;

    fn catalog_file() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../apps/web/src/contracts/generated/commandCatalog.json")
    }

    #[test]
    fn ids_are_unique_lowercase_and_examples_fit_their_input_type() {
        let c = catalog();
        let mut seen = std::collections::BTreeSet::new();
        for d in &c.commands {
            assert!(
                seen.insert((&d.id, d.version)),
                "{} v{} twice",
                d.id,
                d.version
            );
            assert!(
                d.id.split('.').count() >= 2
                    && d.id
                        .split('.')
                        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_lowercase())),
                "{}: alan.nesne.eylem, lowercase ASCII",
                d.id
            );
            assert!(!d.hosts.is_empty(), "{}: listed without a host", d.id);
            assert!(
                d.input.get("$schema").is_some() && d.output.get("$schema").is_some(),
                "{}: schemas",
                d.id
            );
            // A command an app runs plans too; one only the server runs does not.
            let local = d
                .hosts
                .iter()
                .any(|h| matches!(h, CommandHost::Web | CommandHost::Desktop));
            assert_eq!(
                d.plan.as_ref().is_some_and(|p| p.get("$schema").is_some()),
                local,
                "{}: a plan schema exactly when an app runs it",
                d.id
            );
            // Permissions are the fixed names of docs/adr/0015.
            for p in &d.permissions {
                assert!(
                    ProjectPermission::ALL.iter().any(|k| k.name() == p),
                    "{}: unknown permission {p}",
                    d.id
                );
            }
            for e in &d.examples {
                // Each command's example must parse as its own input type.
                let parsed = match d.id.as_str() {
                    crate::PROJECT_CHANGES => {
                        serde_json::from_value::<ProjectChanges>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_SHARE => {
                        serde_json::from_value::<ProjectShare>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_ACCESS_REVOKE => {
                        serde_json::from_value::<ProjectAccessRevoke>(e.input.clone()).map(|_| ())
                    }
                    // The catalog and lifecycle commands (docs/adr/0028).
                    crate::PROJECT_CREATE => {
                        serde_json::from_value::<crate::ProjectCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_RENAME => {
                        serde_json::from_value::<crate::ProjectRename>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_METADATA_UPDATE => {
                        serde_json::from_value::<crate::ProjectMetadataUpdate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::PROJECT_DUPLICATE => {
                        serde_json::from_value::<crate::ProjectDuplicate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::PROJECT_ARCHIVE
                    | crate::PROJECT_UNARCHIVE
                    | crate::PROJECT_TRASH
                    | crate::PROJECT_RESTORE => {
                        serde_json::from_value::<crate::EmptyInput>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_PURGE => {
                        serde_json::from_value::<crate::ProjectPurge>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_FAVORITE => {
                        serde_json::from_value::<crate::ProjectFavorite>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::PROJECT_FILE_COMMIT => {
                        serde_json::from_value::<crate::FileCommit>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_CONVERT => {
                        serde_json::from_value::<crate::ProjectConvert>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_IMPORT => {
                        serde_json::from_value::<crate::ProjectImport>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_INVITE => {
                        serde_json::from_value::<crate::ProjectInvite>(e.input.clone()).map(|_| ())
                    }
                    crate::PROJECT_INVITATION_REVOKE => {
                        serde_json::from_value::<crate::InvitationRevoke>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::PROJECT_CHECKPOINT_CREATE => {
                        serde_json::from_value::<crate::CheckpointCreate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::PROJECT_CHECKPOINT_DELETE => {
                        serde_json::from_value::<crate::CheckpointDelete>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::PROJECT_CHECKPOINT_RESTORE => {
                        serde_json::from_value::<crate::CheckpointRestore>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_POLYGON_CREATE => {
                        serde_json::from_value::<PolygonCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LINE_CREATE => {
                        serde_json::from_value::<LineCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_POLYLINE_CREATE => {
                        serde_json::from_value::<PolylineCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_DELETE => {
                        serde_json::from_value::<EntitiesDelete>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_POINT_CREATE => {
                        serde_json::from_value::<crate::PointCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_CIRCLE_CREATE => {
                        serde_json::from_value::<crate::CircleCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ARC_CREATE => {
                        serde_json::from_value::<crate::ArcCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_TRANSFORM => {
                        serde_json::from_value::<crate::EntitiesTransform>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ENTITIES_EDIT => {
                        serde_json::from_value::<crate::EntitiesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_ARRAY => {
                        serde_json::from_value::<crate::EntitiesArray>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_OFFSET => {
                        serde_json::from_value::<crate::EntitiesOffset>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DIMENSIONS_REASSOCIATE => {
                        serde_json::from_value::<crate::DimensionsReassociate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_DIMENSIONS_CREATE => {
                        serde_json::from_value::<crate::DimensionsCreate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ENTITIES_TRIM | crate::CAD_ENTITIES_EXTEND => {
                        serde_json::from_value::<crate::EntitiesTrim>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_FILLET => {
                        serde_json::from_value::<crate::EntitiesFillet>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_CHAMFER => {
                        serde_json::from_value::<crate::EntitiesChamfer>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ENTITIES_STRETCH => {
                        serde_json::from_value::<crate::EntitiesStretch>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ENTITIES_BREAK => {
                        serde_json::from_value::<crate::EntitiesBreak>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_JOIN => {
                        serde_json::from_value::<crate::EntitiesJoin>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_DIVIDE => {
                        serde_json::from_value::<crate::EntitiesDivide>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_BOUNDARY => {
                        serde_json::from_value::<crate::EntitiesBoundary>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ENTITIES_HATCH => {
                        serde_json::from_value::<crate::EntitiesHatch>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_CREATE => {
                        serde_json::from_value::<crate::EntitiesCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_SET => {
                        serde_json::from_value::<crate::EntitiesSetProperties>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_BLOCKS_DEFINE => {
                        serde_json::from_value::<crate::BlocksDefine>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_BLOCKS_EDIT => {
                        serde_json::from_value::<crate::BlocksEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_SERVICE => {
                        serde_json::from_value::<crate::LayersService>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_NETWORK_DEFINE => {
                        serde_json::from_value::<crate::NetworkDefine>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ADDRESSES_DEFINE => {
                        serde_json::from_value::<crate::AddressesDefine>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_LAYERS_TIME => {
                        serde_json::from_value::<crate::LayersTime>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SCENARIOS_EDIT => {
                        serde_json::from_value::<crate::ScenariosEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_FILTER => {
                        serde_json::from_value::<crate::LayersFilter>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_COMPOSE => {
                        serde_json::from_value::<crate::LayersCompose>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_INFO => {
                        serde_json::from_value::<crate::LayersInfo>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_LABELS => {
                        serde_json::from_value::<crate::LayersLabels>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LABELS_PIN => {
                        serde_json::from_value::<crate::LabelsPin>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_RENDERER => {
                        serde_json::from_value::<crate::LayersRenderer>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_EDIT => {
                        serde_json::from_value::<crate::LayersEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_SET => {
                        serde_json::from_value::<crate::LayersSet>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_STATES => {
                        serde_json::from_value::<crate::LayersStates>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STYLES_EDIT => {
                        serde_json::from_value::<crate::StylesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SETTINGS_SET => {
                        serde_json::from_value::<crate::SettingsSet>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SHEETS_EDIT => {
                        serde_json::from_value::<crate::SheetsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_UNITS_CHANGE => {
                        serde_json::from_value::<crate::UnitsChange>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_DXF => {
                        serde_json::from_value::<crate::ImportDxf>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_CHANGESET_APPLY => {
                        serde_json::from_value::<crate::ChangesetApply>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_DXF => {
                        serde_json::from_value::<crate::ExportDxf>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_PDF => {
                        serde_json::from_value::<crate::ExportPdf>(e.input.clone()).map(|_| ())
                    }
                    other => panic!("{other}: add its input type to this test"),
                };
                parsed.unwrap_or_else(|err| panic!("{}: {}: {err}", d.id, e.title));
                // And an example's output its output type.
                if let Some(output) = &e.output {
                    let parsed = match d.id.as_str() {
                        crate::CAD_POLYGON_CREATE => {
                            serde_json::from_value::<PolygonCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_LINE_CREATE => {
                            serde_json::from_value::<LineCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_POLYLINE_CREATE => {
                            serde_json::from_value::<PolylineCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_ENTITIES_DELETE => {
                            serde_json::from_value::<EntitiesDeleted>(output.clone()).map(|_| ())
                        }
                        crate::CAD_POINT_CREATE => {
                            serde_json::from_value::<crate::PointCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_CIRCLE_CREATE => {
                            serde_json::from_value::<crate::CircleCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ARC_CREATE => {
                            serde_json::from_value::<crate::ArcCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_ENTITIES_TRANSFORM => {
                            serde_json::from_value::<crate::EntitiesTransformed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_EDIT => {
                            serde_json::from_value::<crate::EntitiesEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_ARRAY => {
                            serde_json::from_value::<crate::EntitiesArrayed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_OFFSET => {
                            serde_json::from_value::<crate::EntitiesOffsetMade>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_DIMENSIONS_REASSOCIATE => {
                            serde_json::from_value::<crate::DimensionsReassociated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_TRIM | crate::CAD_ENTITIES_EXTEND => {
                            serde_json::from_value::<crate::EntitiesTrimmed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_FILLET | crate::CAD_ENTITIES_CHAMFER => {
                            serde_json::from_value::<crate::EntitiesCornered>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_STRETCH => {
                            serde_json::from_value::<crate::EntitiesStretched>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_BREAK | crate::CAD_ENTITIES_JOIN => {
                            serde_json::from_value::<crate::EntitiesEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_DIVIDE
                        | crate::CAD_ENTITIES_BOUNDARY
                        | crate::CAD_ENTITIES_HATCH
                        | crate::CAD_DIMENSIONS_CREATE => {
                            serde_json::from_value::<crate::EntitiesCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_CREATE => {
                            serde_json::from_value::<crate::EntitiesCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_SET => {
                            serde_json::from_value::<crate::EntitiesPropertiesSet>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_BLOCKS_DEFINE => {
                            serde_json::from_value::<crate::BlockDefined>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_BLOCKS_EDIT => {
                            serde_json::from_value::<crate::BlocksEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_SERVICE => {
                            serde_json::from_value::<crate::LayersServiced>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_NETWORK_DEFINE => {
                            serde_json::from_value::<crate::NetworkDefined>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ADDRESSES_DEFINE => {
                            serde_json::from_value::<crate::AddressesDefined>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_TIME => {
                            serde_json::from_value::<crate::LayersTimed>(output.clone()).map(|_| ())
                        }
                        crate::CAD_SCENARIOS_EDIT => {
                            serde_json::from_value::<crate::ScenariosEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_FILTER => {
                            serde_json::from_value::<crate::LayersFiltered>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_COMPOSE => {
                            serde_json::from_value::<crate::LayersComposed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_INFO => {
                            serde_json::from_value::<crate::LayersInformed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_LABELS => {
                            serde_json::from_value::<crate::LayersLabelled>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LABELS_PIN => {
                            serde_json::from_value::<crate::LabelsPinned>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_RENDERER => {
                            serde_json::from_value::<crate::LayersRendered>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_EDIT => {
                            serde_json::from_value::<crate::LayersEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_SET => {
                            serde_json::from_value::<crate::LayersChanged>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_STATES => {
                            serde_json::from_value::<crate::LayerStatesDone>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_STYLES_EDIT => {
                            serde_json::from_value::<crate::StylesEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_SETTINGS_SET => {
                            serde_json::from_value::<crate::SettingsChanged>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_SHEETS_EDIT => {
                            serde_json::from_value::<crate::SheetsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_UNITS_CHANGE => {
                            serde_json::from_value::<crate::UnitsChanged>(output.clone())
                                .map(|_| ())
                        }
                        crate::FILE_IMPORT_DXF => {
                            serde_json::from_value::<crate::DxfImported>(output.clone()).map(|_| ())
                        }
                        other => panic!("{other}: add its output type to this test"),
                    };
                    parsed.unwrap_or_else(|err| panic!("{}: {}: output: {err}", d.id, e.title));
                }
            }
        }
    }

    #[test]
    fn generated_catalog_file_is_current() {
        let text = format!(
            "{}\n",
            serde_json::to_string_pretty(&catalog()).expect("the catalog serializes")
        );
        let path = catalog_file();
        if std::env::var("KENTOS_WRITE_CATALOG").as_deref() == Ok("1") {
            std::fs::write(&path, &text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            return;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            on_disk == text,
            "{} is out of date: run `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-contracts catalog`, read the diff, commit it",
            path.display()
        );
    }
}
