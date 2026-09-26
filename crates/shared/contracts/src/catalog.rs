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
        CAD_CIRCLE_CREATE_VERSION, CAD_ENTITIES_DELETE, CAD_ENTITIES_DELETE_VERSION,
        CAD_ENTITIES_TRANSFORM, CAD_ENTITIES_TRANSFORM_VERSION, CAD_LINE_CREATE,
        CAD_LINE_CREATE_VERSION, CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION, CAD_POLYGON_CREATE,
        CAD_POLYGON_CREATE_VERSION, CAD_POLYLINE_CREATE, CAD_POLYLINE_CREATE_VERSION, CircleCreate,
        CircleCreated, CommitResult, EntitiesDelete, EntitiesDeleted, EntitiesTransform,
        EntitiesTransformed, LineCreate, LineCreated, PROJECT_ACCESS_REVOKE,
        PROJECT_ACCESS_REVOKE_VERSION, PROJECT_CHANGES, PROJECT_CHANGES_VERSION, PROJECT_SHARE,
        PROJECT_SHARE_VERSION, PointCreate, PointCreated, PolygonCreate, PolygonCreated,
        PolylineCreate, PolylineCreated, ProjectAccessChange, ProjectAccessRevoke, ProjectChanges,
        ProjectPermission, ProjectShare,
    };
    use serde_json::json;
    CommandCatalog {
        format: CATALOG_FORMAT.into(),
        version: CATALOG_VERSION,
        commands: vec![CommandDescriptor {
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
            summary: "Açık çizimde verilen katmana noktaları ve isteğe bağlı yay değerleriyle (kenar başına bir tane) açık bir çoklu çizgi ekler; tek geri alma adımıdır. \
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
        // The move, copy, rotate, scale and mirror tools' command (docs/adr/0037), held
        // together by fixtures/commands/v1/cad.entities.transform.json.
        CommandDescriptor {
            id: CAD_ENTITIES_TRANSFORM.into(),
            version: CAD_ENTITIES_TRANSFORM_VERSION,
            title: "Nesneleri dönüştür".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri taşır, bir merkez etrafında döndürür, bir merkeze göre ölçekler ya da iki noktalı bir eksene göre aynalar; yerinde ya da kopya olarak, tek geri alma adımında. \
                      Taşı, Kopyala, Döndür, Ölçekle ve Aynala araçları seçimi kimlik listesi olarak verip bu komutla yazar; adım aracın adını taşır. \
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
            ],
        }],
    }
}

#[cfg(all(test, feature = "schema"))]
mod tests {
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
