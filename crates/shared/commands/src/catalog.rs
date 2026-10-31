//! The product command catalog (docs/adr/0013), built over the commands' types: the descriptors are
//! `kentos_contracts::catalog`'s.
//!
//! `cargo test -p kentos-commands` compares the catalog with
//! `apps/web/src/contracts/generated/commandCatalog.json` and fails on a
//! difference; after a deliberate change, `KENTOS_WRITE_CATALOG=1` rewrites
//! the file (read the diff before committing). The file holds the catalog's
//! shared form (`CommandCatalog::shared`, docs/adr/0731): each definition once,
//! in its top-level `$defs`.

pub use kentos_contracts::catalog::*;

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
                      Dönüşüm geometri çekirdeğindedir: her nesne türü (yay, daire, elips, eğri, yazı, ölçü, tarama) aynı kuralla döner; yaylar saat yönünün tersine kalır. Aynanın text alanı yazıların ne olacağıdır (AutoCAD'in MIRRTEXT'i): readable (varsayılan) yazıyı aynadaki yerinde okunur, baş aşağı olmadan, hizası o yeri dolduracak biçimde yazar; mirrored yazıyı çizimle döndürür; yazıların metni ve bloğun öznitelik değerleri hiç değişmez, bloğun kendi yazıları her zaman okunurdur. \
                      Yerinde değişen nesne yuvasını, kalıcı kimliğini ve öbür alanlarını korur; kopya bütün alanları alır, yeni bir kalıcı kimlik alır. \
                      layerId (Katmana kopyala) yalnız kopyayla verilir: kopyalar o katmana gider, katman yeni nesnenin katmanı gibi denetlenir, adım Katmana kopyala. \
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
                      Kopyala'nın Dizi'si (line): yer değiştirme (dx, dy) boyunca adet kadar yer (2 ile 10 000 arası, asıllar ilk yerde), k'ıncı kopya k yer değiştirme ötede; fit ile son kopya yer değiştirmenin ucunda, öbürleri onu eşit paylaşır; adım Kopyala. \
                      Dizi, Kutupsal dizi ve Yol boyunca dizi araçları seçimi kimlik listesi olarak verip bu komutla yazar; adım aracın adını taşır. Yerleşim geometri çekirdeğindedir: her nesne türü aynı kuralla kopyalanır. \
                      Kopya aslının bütün alanlarını ve yeni bir kalıcı kimlik alır. Kilitli katmandaki nesnenin kopyası yapılmaz: öbürleri uyarıyla yazılır, hepsi kilitliyse hiçbir şey yazılmaz. \
                      associative ile (ilişkili dizi, docs/adr/0300) nesneler “Dizi n” bloğu olur ve her yere bloğun bir yerleştirmesi konur; dizi çizimde kalır (çıktının array'i) ve cad.arrays.edit ile düzenlenir. \
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
        // Diziyi düzenle (docs/adr/0301), held together by fixtures/commands/v1/cad.arrays.edit.json.
        CommandDescriptor {
            id: crate::CAD_ARRAYS_EDIT.into(),
            version: crate::CAD_ARRAYS_EDIT_VERSION,
            title: "Diziyi düzenle".into(),
            summary: "Kimliğiyle verilen ilişkili diziyi (cad.entities.array'in associative'i) düzenler (AutoCAD'in ARRAYEDIT'i); tek geri alma adımı “Diziyi düzenle”. \
                      Öğeler yerleşimin sırasıyla 0'dan numaralanır (ızgarada satır satır: satır r, sütun c r × sütun + c). \
                      edit layout: yeni satır, sütun, adet, aralık, açı ya da yol; her öğe yeniden yerleşir. Öğe numarasını (iki ızgarada satırını ve sütununu) korur; elle silinen silinmiş, elle değişen (kept) yerinde kalır; yeni yerlere yerleştirme eklenir, kalkan yerlerin yerleştirmeleri silinir. \
                      edit placement: ilk yerin p, scale, rotation ve mirror'ı; silinmemiş ve değişmemiş öğeler izler. \
                      edit replace: items öğeleri block adlı bloğu yerleştirir ve korunur. edit remove: items öğelerinin yerleştirmeleri silinir, öğeler boş kalır. edit reset: items (yoksa hepsi) yerleşimin verdiği gibi olur: silinen geri gelir, değişen dizinin bloğuyla yerine döner. \
                      Dizinin katmanı ve dokunulan her yerleştirme kilitsiz olmalı: hepsi ya da hiçbiri. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ArraysEdit>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Direk dizisini 3 satır × 6 sütuna çıkar".into(),
                    input: json!({ "array": "0192f5a3-3333-7000-8000-000000000001", "edit": { "kind": "layout", "layout": { "kind": "grid", "rows": 3, "cols": 6, "dx": 30.0, "dy": 25.0 } } }),
                    output: Some(json!({ "changed": [], "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4101"], "removed": [], "revision": "66" })),
                },
                CommandExample {
                    title: "Elle silinen ve değiştirilen öğeleri yerine getir".into(),
                    input: json!({ "array": "0192f5a3-3333-7000-8000-000000000001", "edit": { "kind": "reset" } }),
                    output: None,
                },
            ],
        },
        // Kısıtlar (docs/adr/0335), held together by fixtures/commands/v1/cad.constraints.edit.json.
        CommandDescriptor {
            id: crate::CAD_CONSTRAINTS_EDIT.into(),
            version: crate::CAD_CONSTRAINTS_EDIT_VERSION,
            title: "Kısıtlar".into(),
            summary: "Çizimin geometrik ve ölçü kısıtlarını (AutoCAD'in GEOMCONSTRAINT ve DIMCONSTRAINT'i) ekler, kaldırır, sürücü ya da başvuru yapar, öbür çözümüne çevirir ya da çizimi yeniden çözer; kısıtlar ve çözümün oynattığı nesneler tek geri alma adımı “Kısıtlar”. \
                      edit add: kısıtlar sırasıyla eklenir; kural ve parçaları (nesnenin kalıcı kimliği ve parçası: nokta yeri, düz kenar ya da daire). Yön (teğetin yanı, kenara uzaklığın yanı, açının 180°'den küçük olanı, yatay ve düşey uzaklığın küçükten büyüğe) sense: given demedikçe çizimden okunur. Ölçü kısıtının parameter'ı boşsa bir sonraki boş ad (d1, ang1, rad1, dia1); value bir ifadedir (çizim biriminde, açı projenin açı biriminde; 3.5, 120cm, d1 / 2), yoksa çizimin ölçtüğü; reference yalnız ölçer. Ötekileri tekrarlayan kısıt eklenmez (constraint_redundant). \
                      edit remove: kimlikleriyle kısıtlar kalkar, ölçünün parametresi başka parametre okumuyorsa onunla. edit mode: ölçüler sürücü ya da başvuru olur. edit flip: kısıtın öbür çözümü. edit solve: çizim kısıtlarıyla yeniden çözülür. \
                      Kilitli katmandaki nesne kımıldamaz. Çelişkide (constraint_conflict, çelişenler uids'te; mimari bağlar parçaları, değerleri ve hangisinin kaldırılınca geçireceğiyle bonds'ta), tekil kısıtta ya da çözüm bulunamadığında hiçbir şey yazılmaz. Bir şekli ters çeviren çözüm (shape_flipped) allowFlip olmadan yazılmaz. Çıktı eklenen ve kaldırılan kısıtları, oynayan nesneleri ve çözücünün teşhisini (durum, serbestlik, fazla, çelişen, tekil, kopuk kısıtlar, serbest nesneler) verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ConstraintsEdit>(),
            output: schema::<crate::ConstraintsEdited>(),
            plan: Some(schema::<crate::ConstraintsEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Alt kenarı yatay yap".into(),
                    input: json!({ "edit": { "kind": "add", "constraints": [{ "rule": { "kind": "horizontal",
                        "a": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "element": { "kind": "segment", "part": 0, "ring": 0, "index": 0 } } } }] } }),
                    output: Some(json!({ "added": ["0192f5a6-2000-7000-8000-000000000301"], "removed": [],
                        "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"],
                        "diagnosis": { "status": "under", "dof": 3, "free": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "iterations": 1 },
                        "revision": "13" })),
                },
                CommandExample {
                    title: "Genişliği 8 m olan ölçü kısıtı".into(),
                    input: json!({ "edit": { "kind": "add", "constraints": [{ "rule": { "kind": "distance",
                        "a": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "element": { "kind": "point", "place": { "kind": "start" } } },
                        "b": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "element": { "kind": "point", "place": { "kind": "end" } } },
                        "parameter": "genislik" }, "value": "8" }] } }),
                    output: None,
                },
            ],
        },
        // Parametreler (docs/adr/0336 §1), held together by fixtures/commands/v1/cad.parameters.edit.json.
        CommandDescriptor {
            id: crate::CAD_PARAMETERS_EDIT.into(),
            version: crate::CAD_PARAMETERS_EDIT_VERSION,
            title: "Parametreler".into(),
            summary: "Çizimin parametrelerini (AutoCAD'in parametre yöneticisi) adlarıyla değiştirir ve çizimi yeni değerlerle çözer; parametreler ve çözümün oynattığı nesneler tek geri alma adımı “Parametreler”. \
                      edits sırasıyla: add kullanıcı parametresi (ad, tür length, angle ya da number, ifade, isteğe bağlı min, max, açıklama); set ifade, sınırlar (clear kaldırır) ve açıklama, verilmeyenler kalır; rename yeni ad, onu okuyan ifadeler ve ölçü kısıtı izler; remove başka parametrenin okumadığı kullanıcı parametresi. \
                      Değerler parametrenin biriminde: uzunluk çizim biriminde, açı projenin açı biriminde. Yalnız ölçen ölçünün parametresine ifade verilmez (parameter_measured). Döngü, aralık dışı değer, çelişki, tekillik ve ters dönme cad.constraints.edit'teki gibi reddedilir, hiçbir şey yazılmaz. Çıktı her parametrenin değerini, oynayan nesneleri ve çözücünün teşhisini verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ParametersEdit>(),
            output: schema::<crate::ParametersEdited>(),
            plan: Some(schema::<crate::ParametersEditPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Genişliği 9 m yap".into(),
                input: json!({ "edits": [{ "kind": "set", "name": "genislik", "expression": "9" }] }),
                output: None,
            }],
        },
        // Kısıtları oku (docs/adr/0335 §1), held together by fixtures/commands/v1/cad.constraints.query.json.
        CommandDescriptor {
            id: crate::CAD_CONSTRAINTS_QUERY.into(),
            version: crate::CAD_CONSTRAINTS_QUERY_VERSION,
            title: "Kısıtları oku".into(),
            summary: "Çizimin kısıtlarını (uids verilirse yalnız o nesneleri ananları, ids verilirse yalnız o kısıtları, architectural ile yalnız duvar çizgisi ve pervaz bağlarını: Bağı göster) Türkçe adları, ölçü kısıtının bugünkü değeri (measured: metre ya da radyan), tuttuğu değer (kept) ve tutup tutmadığıyla (holds), parametreleri ve çizimin çözücüdeki teşhisini (durum, serbestlik, fazla, çelişen, tekil, kopuk kısıtlar, serbest nesneler) verir; hiçbir şey yazmaz."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ConstraintsQuery>(),
            output: schema::<crate::ConstraintsListed>(),
            plan: Some(schema::<crate::ConstraintsListed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimin bütün kısıtları".into(),
                input: json!({}),
                output: None,
            }],
        },
        // Patlat by its meaning (docs/adr/0302), held together by fixtures/commands/v1/cad.entities.explode.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_EXPLODE.into(),
            version: crate::CAD_ENTITIES_EXPLODE_VERSION,
            title: "Nesneleri patlat".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri bir düzey parçalar (AutoCAD'in EXPLODE'u); tek geri alma adımı “Patlat”. \
                      Çoklu çizgi ve alan kenarlarına (çizgi ve yay; delikler ve parçalar da), daire, yay, elips, eliptik yay, eğri ve NURBS eğrisi köşeleri üstünde olan bir çoklu çizgiye (daire ve tam elips kapalı), ölçü çizgilerine, oklarına ve yazısına, desenli tarama çizgilerine, kılavuz çizgisine, okuna ve notuna, tablo çizgilerine ve yazılarına, bloğun yerleştirmesi tanımın nesnelerine açılır. \
                      Parça nesnenin katmanını, rengini ve kalınlığını alır; bloğun nesnesi çizimde varsa kendi katmanını, rengini, kalınlığını, özniteliklerini ve etiketini (yoksa yerleştirmeninkini) alır. \
                      İlişkili dizinin (cad.entities.array'in associative'i) bir öğesi bütün diziyi patlatır: her öğe açılır, dizi aynı adımda kalkar. \
                      Eğrinin nokta sıklığı density'dir: count (eşit bölüm sayısı), step (bölümün en büyük uzunluğu) ya da deviation (kirişlerin eğriden en büyük uzaklığı); verilmezse daire, yay ve elipste kirişler eğriye 0,01 m'den, eğri ve NURBS'te 0,1 mm'den yakın. Bölümler eğri boyunca eşit uzunluktadır; bir eğri en çok 100 000 köşe olur, ötesi invalid_density. \
                      Patlamayan nesne (çizgi, nokta, yazı, dolu tarama …) ve kilitli katmandaki nesne uyarıyla atlanır; hiçbiri patlamazsa not_explodable, hepsi kilitliyse layer_locked. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesExplode>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir parsel alanını kenar çizgilerine ayır".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"] }),
                output: Some(json!({ "changed": [], "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4101", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4102", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4103", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4104"], "removed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "revision": "67" })),
            }, CommandExample {
                title: "Bir daireyi 24 köşeli kapalı çoklu çizgiye çevir".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "density": { "kind": "count", "segments": 24 } }),
                output: Some(json!({ "changed": [], "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4105"], "removed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "revision": "68" })),
            }],
        },
        // Eğriyi düzenle (docs/adr/0303), held together by fixtures/commands/v1/cad.curves.edit.json.
        CommandDescriptor {
            id: crate::CAD_CURVES_EDIT.into(),
            version: crate::CAD_CURVES_EDIT_VERSION,
            title: "Eğriyi düzenle".into(),
            summary: "Kalıcı kimliğiyle verilen çoklu çizgiyi ya da NURBS eğrisini değişecek olanla düzenler (AutoCAD'in PEDIT'i ve SPLINEDIT'i); tek geri alma adımı “Eğriyi düzenle”, nesne katmanını, görünüşünü ve verisini korur. \
                      Çoklu çizgide close (Kapat), open (Aç: kapanış kenarının bükümü ve genişliği gider), decurve (Düzleştir: yaylar düz olur) ve toSpline (Eğriye çevir: köşelerden geçen ya da köşeleri denetim noktası olan NURBS eğrisi, derece yoksa 3). \
                      Geçiş noktalı NURBS eğrisinde addFit, moveFit, removeFit ve tangents (uç teğetleri): eğri noktalardan yeniden kurulur, açık eğri teğetlerini korur. \
                      Her NURBS eğrisinde moveControl ve weight (kapalı eğride sarılan kopya da; geçiş noktaları gider), removeControl, insertKnot (açık eğride, aralığın içinde, derece kereden az olan yere; eğri değişmez), close ve open, toPolyline (her kiriş eğriye tolerance'tan yakın çoklu çizgi). \
                      Retler: not_a_curve (tür ya da düzenleme uymuyor), multi_part, no_fit_points, invalid_index, invalid_knot, invalid_weight, invalid_tolerance, too_few_points, coincident_points, invalid_nurbs. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::CurvesEdit>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Eğriye bir geçiş noktası ekle".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001", "edit": { "kind": "addFit", "index": 2, "at": { "x": 18, "y": 70 } } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001"], "created": [], "removed": [], "revision": "68" })),
                },
                CommandExample {
                    title: "Çoklu çizgiyi köşelerinden geçen kübik eğriye çevir".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5002", "edit": { "kind": "toSpline", "through": true } }),
                    output: None,
                },
            ],
        },
        // Özellik kopyala (docs/adr/0304), held together by fixtures/commands/v1/cad.entities.match.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_MATCH.into(),
            version: crate::CAD_ENTITIES_MATCH_VERSION,
            title: "Özellik kopyala".into(),
            summary: "Kalıcı kimliğiyle verilen kaynak nesnenin seçilen özelliklerini öbür nesnelere verir (AutoCAD'in MATCHPROP'u); tek geri alma adımı “Özellik kopyala”. \
                      Özellikler (properties; verilmezse hepsi): layer, color, lineType, lineTypeScale, lineWeight (yalnız çizgiyle çizilenler arasında), transparency, symbol; text (yazının yüzü, yüksekliği ve genişlik çarpanı, yazılara), dimension (ölçünün görünüşü, değer yüksekliği ve zemini, ölçülere), hatch (taramanın deseni, taramalara; bölgeleri kalır), leader (kılavuzun oku, ok boyu, not yüksekliği ve zemini, kılavuzlara). \
                      Kaynakta olmayan değer (katmanın rengi, sembol yok) hedeftekini kaldırır. Kaynak hedeflerin arasındaysa atlanır, kilitli katmandaki hedef uyarıyla atlanır; katmana taşıma cad.entities.set'in kurallarıyla (kilitli, gizli, alanlar). \
                      Retler: invalid_uid, no_entities, no_properties, entity_not_found, layer_locked. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesMatch>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir parselin katmanını ve rengini öbür parsellere ver".into(),
                    input: json!({ "source": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6001", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6003"], "properties": ["layer", "color"] }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6003"], "created": [], "removed": [], "revision": "69" })),
                },
                CommandExample {
                    title: "Bir yazının yüzünü ve yüksekliğini başka yazılara ver".into(),
                    input: json!({ "source": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6011", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6012"], "properties": ["text"] }),
                    output: None,
                },
            ],
        },
        // Çizimi temizle (docs/adr/0305), held together by fixtures/commands/v1/cad.entities.cleanup.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_CLEANUP.into(),
            version: crate::CAD_ENTITIES_CLEANUP_VERSION,
            title: "Çizimi temizle".into(),
            summary: "Toleransla boş ve yinelenen nesneleri siler, aynı doğrudaki çizgileri birleştirir, yolların gereksiz köşelerini atar (AutoCAD'in OVERKILL'i); tek geri alma adımı “Çizimi temizle”. \
                      Kapsam uids ya da verilmezse görünen ve kilitsiz katmanlardaki bütün nesneler. tolerance (metre, 0 ile 10; verilmezse 0,000001) içinde: boş nesne (çizgi, yol ve yay tolerans kadar kısa, daire tolerans kadar küçük, sözsüz yazı) gider; aynı türden, noktaları tolerans içinde, öbür alanları, ignore dışındaki kendi özellikleri ve verisi (öznitelik, etiket, ekler) eşit nesne yinelenendir ve gider, ilki kalır. \
                      overlaps (varsayılan açık) aynı doğrudaki üst üste binen çizgileri ilkine katar, endToEnd uç uca olanları da; kotlu ya da ölçülü çizgi katılmaz. straightVertices (varsayılan açık) yolun komşularının arasındaki doğrudaki köşelerini, her zaman yinelenen köşelerini atar; kendi kotu ya da ölçüsü olan, genişliğin değiştiği köşe kalır. \
                      Rapor sayıları ve kaybı verir (gidenin kalandan en uzak noktası). Retler: invalid_uid, no_entities, not_finite, invalid_tolerance, entity_not_found, layer_locked. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesCleanup>(),
            output: schema::<crate::EntitiesCleaned>(),
            plan: Some(schema::<crate::EntitiesCleanupPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bütün çizimi milimetre toleransla temizle".into(),
                    input: json!({ "tolerance": 0.001 }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7001"], "removed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7003"], "report": { "empty": 0, "duplicates": 1, "merged": 1, "merging": 1, "vertices": 0, "paths": 0, "loss": 0.0004 }, "revision": "70" })),
                },
                CommandExample {
                    title: "Seçilenlerde rengi farklı yinelenenleri de sil".into(),
                    input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7011", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7012"], "ignore": ["color"], "overlaps": false }),
                    output: None,
                },
            ],
        },
        // Panoya kopyala and Taban noktasıyla kopyala (docs/adr/0309), held together by
        // fixtures/commands/v1/cad.entities.copy.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_COPY.into(),
            version: crate::CAD_ENTITIES_COPY_VERSION,
            title: "Panoya kopyala".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerin kopyasını kullandıklarıyla birlikte verir (AutoCAD'in COPYCLIP'i ve COPYBASE'i); çizim değişmez. \
                      Kopya (Clip) nesneleri çizimdeki sıralarıyla, kaynaktaki kimliklerini, bulundukları katmanları gruplarıyla ve görünüşleriyle, yerleştirdikleri blokları (iç içe olanlar da), adını andıkları yazı ve ölçü stillerini, çizdikleri kitaplık öğelerini, kaynağın birimini ve taban noktasını taşır. \
                      Taban noktası base'dir; verilmezse nesnelerin kutusunun sol alt köşesi. cad.entities.paste kopyayı aynı ya da başka bir çizime yapıştırır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesCopy>(),
            output: schema::<crate::Clip>(),
            plan: Some(schema::<crate::Clip>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İki nesneyi bir köşe noktasından kopyala".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "base": { "x": 100.0, "y": 200.0 } }),
                output: None,
            }],
        },
        // Yapıştır, Özgün koordinatlara yapıştır and Blok olarak yapıştır (docs/adr/0309), held together by
        // fixtures/commands/v1/cad.entities.paste.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_PASTE.into(),
            version: crate::CAD_ENTITIES_PASTE_VERSION,
            title: "Yapıştır".into(),
            summary: "cad.entities.copy'nin verdiği kopyayı bu çizime yapıştırır (AutoCAD'in PASTECLIP, PASTEORIG ve PASTEBLOCK'u); tek geri alma adımı “Yapıştır”. \
                      Kopyanın taban noktası at'a gelir; at verilmezse nesneler kopyalandıkları yerde kalır. \
                      Her nesne bu çizimde kendi katmanının yolundaki katmana gider (adlar büyük küçük harf ve Türkçe harf farkı gözetmeden); olmayan yol kaynağın görünüşüyle ve gruplarıyla açılır. \
                      Bu çizimde aynı adlı blok varsa bu çizimin tanımı kullanılır (report.kept), yoksa eklenir (report.blocks); stiller de adıyla, olmayanlar eklenir (proje ayarı). conflicts: rename ile aynı adlı blok ve stil kopyanınkiyle “Ad (2)” gibi boş bir adla gelir, nesneler onu kullanır (report.renamed, names_renamed uyarısı). Nesneler yeni kimlik alır; kopyanın dışına bağlar düşer. \
                      asBlock ile nesneler o adla (boş bir ad: “Ad (2)” …) tek blok olur, tabanı kopyanın taban noktası, etkin katmanda at'a yerleştirilir. \
                      Uzunluklar her çizimde metre olduğundan başka birimli çizimden gelen nesneler boylarını korur (units_differ uyarısı). Retler: no_entities, invalid_clip, not_finite, empty_name, invalid_revision, revision_conflict, not_a_layer, layer_locked."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesPaste>(),
            output: schema::<crate::EntitiesPasted>(),
            plan: Some(schema::<crate::EntitiesPastePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir çizgiyi başka bir noktaya yapıştır".into(),
                input: json!({
                    "clip": {
                        "objects": [{ "kind": "line", "id": 1, "layerId": "cizim", "attrs": {}, "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 10.0, "y": 0.0 } }],
                        "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"],
                        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
                        "base": { "x": 0.0, "y": 0.0 }
                    },
                    "at": { "x": 50.0, "y": 20.0 }
                }),
                output: None,
            }],
        },
        // Denetle (docs/adr/0306), held together by fixtures/commands/v1/cad.drawing.audit.json.
        CommandDescriptor {
            id: crate::CAD_DRAWING_AUDIT.into(),
            version: crate::CAD_DRAWING_AUDIT_VERSION,
            title: "Denetle".into(),
            summary: "Çizimin bozuk ilişkilerini ve kayıp tanımlarını bulur, istenirse onarır; kullanılmayan adlı kaynakları raporlar, istenirse stilleri temizler (AutoCAD'in AUDIT ve PURGE'ü). \
                      Bulunanlar: nesnesi olmayan bağlı yazı (brokenLink), sınır nesnesi olmayan ilişkili tarama (brokenHatch), boş bıraktığı yazısı çizimde olmayan tarama (missingCutout) ve kendi bağlı etiketini boş bırakan tarama (hatchTextCycle; döngü), nesnesi olmayan canlı ölçü bağı (brokenDimension), projede olmayan yazı ya da ölçü stili (missingTextStyle, missingDimensionStyle). \
                      fix ile onarım tek geri alma adımıdır, “Denetle”: bağ kopar, ilişki kalkar, yazı taramanın listesinden çıkar ve tarama yeniden kesilir, ölçü bağı “nesne yok” nedeniyle kopar, stil Standart olur, görünüş kalır; kilitli katmandaki nesne onarılmaz, uyarılır. \
                      Kullanılmayanlar: boş katmanlar ve gruplar, yerleştirilmeyen bloklar (cad.layers.edit'in purge'ünün bulduğu, kilitli olmayanlar), hiçbir nesnenin adını anmadığı yazı ve ölçü stilleri; purge (textStyles, dimensionStyles) stilleri siler, bu bir proje ayarıdır. \
                      Retler: invalid_revision; expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DrawingAudit>(),
            output: schema::<crate::DrawingAudited>(),
            plan: Some(schema::<crate::DrawingAudited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yalnız denetle: bulunanları ve kullanılmayanları raporla".into(),
                    input: json!({}),
                    output: None,
                },
                CommandExample {
                    title: "Onar ve kullanılmayan stilleri temizle".into(),
                    input: json!({ "fix": true, "purge": ["textStyles", "dimensionStyles"] }),
                    output: None,
                },
            ],
        },
        // İki revizyonu karşılaştır (docs/adr/0576), held together by fixtures/commands/v1/cad.drawing.compare.json.
        CommandDescriptor {
            id: crate::CAD_DRAWING_COMPARE.into(),
            version: crate::CAD_DRAWING_COMPARE_VERSION,
            title: "Revizyonları karşılaştır".into(),
            summary: "Çizimin şimdiki hâliyle aynı çizimin başka bir hâli arasında ne değiştiğini söyler; çizimi değiştirmez. \
                      Öbür hâl (against) açık çizimin geri alma geçmişinde steps adım gerisi (history) ya da bir çizim dosyasıdır (file: mutlak yol, .kcad v2 ya da v1 JSON). \
                      Nesneler kalıcı kimlikleriyle eşlenir: şimdi olup öbüründe olmayan eklenmiş (added), öbüründe olup şimdi olmayan silinmiş (removed), ikisinde olup ayrılan değişmiş (changed). \
                      Değişim, veri karşılaştırmanın kuralıyla (ADR 0179) bulunur: geometri tolerance (metre; varsayılan 0,001) içinde aynıysa ve öznitelikler eşitse aynı; katman bir öznitelik gibi sayılır (fields'ta layer). \
                      Cevap sayıları (same, changed, added, removed) ve kimlik listelerini verir; her liste limit'te kesilir ve truncated bunu söyler. \
                      Retler: invalid_tolerance, invalid_limit, no_such_step (geri alma geçmişi o kadar adım tutmuyor), invalid_file_path, file_not_found, import_unreadable."
                .into(),
            aliases: vec!["revizyonkarsilastir".into(), "farkbul".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::DrawingCompare>(),
            output: schema::<crate::DrawingCompared>(),
            plan: Some(schema::<crate::DrawingCompared>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Geri alma geçmişinde 5 adım geriyle karşılaştır".into(),
                    input: json!({ "against": { "kind": "history", "steps": 5 } }),
                    output: None,
                },
                CommandExample {
                    title: "İndirilmiş bir revizyonla karşılaştır, 1 cm toleransla".into(),
                    input: json!({ "against": { "kind": "file", "path": "/veri/mahalle (revizyon 12).kcad" }, "tolerance": 0.01 }),
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
                      Çizgi, çoklu çizgi (yaylarıyla, çok parçalısı parça parça), kapalı alan, daire, yay, elips ve yardımcı çizgi ile ışın ötelenir; diğer türler not_offsettable ile reddedilir. Kopyayı ortak geometri çekirdeği hesaplar: Ötele aracı, Python ve ajan aynı nesneyi aynı mesafeyle öteleyince aynı kopyayı bulur. \
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
        // Öne getir and Arkaya gönder (docs/adr/0260 §5), held together by
        // fixtures/commands/v1/cad.entities.order.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_ENTITIES_ORDER.into(),
            version: crate::CAD_ENTITIES_ORDER_VERSION,
            title: "Çizim sırasını değiştir".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri katmanlarının çizim sırasında öne (front, Öne getir) ya da arkaya (back, Arkaya gönder) alır; tek geri alma adımıdır. \
                      Nesnenin sırası order alanıdır (yoksa 0); eşitlerde belgenin sırası geçer. Her katmanda öbür nesnelerin en büyük (öne) ya da en küçük (arkaya) sırası bulunur, nesneler onun ötesindeki sıraları kendi aralarındaki sırayla alır. \
                      Çizim sırası katmanı aşmaz: katmanların sırası ağacın sırasıdır. Zaten önde (arkada) olan katman değişmez; hiçbir şey değişmezse hiçbir şey yazılmaz. \
                      Kilitli katmandaki nesne varsa hiçbir şey yazılmaz (layer_locked). \
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
            input: schema::<crate::EntitiesOrder>(),
            output: schema::<crate::EntitiesOrdered>(),
            plan: Some(schema::<crate::EntitiesOrderPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir taramayı katmanının arkasına göndermek".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "to": "back"
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "ids": [12],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "İki yazıyı öne getirmek; yalnız planlandığı sürümde".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"],
                        "to": "front",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // Ölçekli açıklamalar (docs/adr/0323 §4), held together by
        // fixtures/commands/v1/cad.annotations.scales.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_ANNOTATIONS_SCALES.into(),
            version: crate::CAD_ANNOTATIONS_SCALES_VERSION,
            title: "Açıklamanın ölçekleri".into(),
            summary: "Kalıcı kimlikleriyle verilen yazı, ölçü ve kılavuzları ölçekli yapar ve ölçeklerini düzenler; tek geri alma adımıdır. \
                      Ölçekli nesne katıldığı her ölçekte kâğıtta aynı yükseklikte çizilir: kendi geometrisi annotative.at ölçeğindeki (yapıldığında projenin çizim ölçeği) temsilidir, diğer ölçeklerde ölçek/at kadar büyür, \
                      o ölçeğin ayrı değerleriyle (yazının ve notun yeri ve doğrultusu, ölçünün çizgisinin uzaklığı ve değerinin yeri); katılmadığı ölçekte çizilmez. Paftanın haritası her nesneyi kendi ölçeğinde çizer. \
                      make (Ölçekli yap; scales eklenir), unmake (tek yükseklik, nesnenin kendisi), add ve remove (scales; kendi ölçeği kaldırılmaz: own_scale), \
                      set (scale ölçeğinde point, rotation; ölçüde offset, textShift), reset (scales ya da bütün diğer ölçeklerde ayrı değer kalmaz). \
                      Ölçekli olmayan ya da olamayan nesne (eğri boyunca yazı, bağlı yazı, diğer türler) not_annotative uyarısıyla atlanır, hiçbiri kalmazsa ret; kilitli katmandakiler layer_locked uyarısıyla atlanır. \
                      Çizim ölçeği değişince ölçekli nesnenin kendi geometrisi yeni ölçeğin temsili olur, eski ölçeği diğer ölçeklerine geçer; ölçekli olmayan nesneler eskisi gibi izler. \
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
            input: schema::<crate::AnnotationsScales>(),
            output: schema::<crate::AnnotationsScaled>(),
            plan: Some(schema::<crate::AnnotationsScalesPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir yazıyı 1:100'de ve 1:50'de aynı kâğıt yüksekliğinde çizdirmek".into(),
                    input: json!({
                        "operation": "make",
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "scales": [50.0]
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "ids": [12],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "1:50 paftasında yazının yerini ayırmak".into(),
                    input: json!({
                        "operation": "set",
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "scale": 50.0,
                        "point": { "x": 500012.5, "y": 4400020.0 }
                    }),
                    output: None,
                },
            ],
        },
        // Denetim noktalı and Geçiş noktalı eğri's command (docs/adr/0259 §3), held together by
        // fixtures/commands/v1/cad.nurbs.create.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_NURBS_CREATE.into(),
            version: crate::CAD_NURBS_CREATE_VERSION,
            title: "NURBS eğrisi oluştur".into(),
            summary: "Açık çizimde verilen katmana bir NURBS eğrisi ekler; tek geri alma adımıdır. \
                      by.kind control (Denetim noktalı): derece, denetim noktaları, isteğe bağlı ağırlıklar ve düğümler; düğüm verilmezse uçlara sıkıştırılmış eşit aralıklı düğümler kurulur, eğri ilk ve son denetim noktasından geçer. \
                      by.kind fit (Geçiş noktalı): eğrinin sırayla geçeceği noktalar ve derece (verilmezse 3; nokta sayısından küçük olur); komut kiriş uzunluğuyla parametreler, ortalanmış düğümler ve çözülen denklem sistemiyle eğriyi kurar (Piegl ve Tiller'ın global interpolasyonu), noktalar eğrinin geçiş noktaları olarak saklanır. \
                      closed: true eğriyi kendi üstüne düzgünce kapatır (periyodik): denetim noktalarında ilk derece kadar nokta sona sarılır, geçiş noktalarında son nokta ilkine bağlanır; kapalı eğriye düğüm verilmez. \
                      Eğriyi ortak geometri çekirdeği kurar: araç, Python ve ajan aynı noktalarla aynı eğriyi bulur. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. Plan yazmadan eğriyi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::NurbsCreate>(),
            output: schema::<crate::NurbsCreated>(),
            plan: Some(schema::<crate::NurbsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dört noktadan geçen kübik eğri (dere kıyısı)".into(),
                    input: json!({
                        "layerId": "dere",
                        "by": { "kind": "fit", "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.0 },
                            { "x": 423545.0, "y": 4512305.0 },
                            { "x": 423570.0, "y": 4512318.0 }
                        ] }
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Ağırlıklı ikinci dereceden eğri: tam bir çeyrek daire".into(),
                    input: json!({
                        "layerId": "yol",
                        "by": { "kind": "control", "degree": 2,
                            "ctrl": [ { "x": 10.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }, { "x": 0.0, "y": 10.0 } ],
                            "weights": [1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0] }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Altı denetim noktalı kapalı kübik eğri; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "peyzaj",
                        "by": { "kind": "control", "degree": 3, "ctrl": [
                            { "x": 0.0, "y": 0.0 }, { "x": 8.0, "y": -2.0 }, { "x": 14.0, "y": 3.0 },
                            { "x": 12.0, "y": 10.0 }, { "x": 4.0, "y": 11.0 }, { "x": -2.0, "y": 6.0 }
                        ] },
                        "closed": true,
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // Duvar, Çizgiden duvar and the wall's own fields (docs/adr/0339 §5), held together by
        // fixtures/commands/v1/cad.walls.{create,convert,edit}.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_WALLS_CREATE.into(),
            version: crate::CAD_WALLS_CREATE_VERSION,
            title: "Duvar oluştur".into(),
            summary: "Açık çizimde verilen katmana duvarlar ekler; tek geri alma adımı “Duvar”. \
                      Her duvar çizildiği yolla verilir: köşeler, kenar başına büküm (yay), kapalı ya da açık. \
                      Genişlik (m), hiza (left: yol duvarın sol yüzü, center: ortası, right: sağ yüzü; yolun gidişine göre) ve öteleme (yol, hiza çizgisinden sola) duvarın gövdesinin yolun neresinde durduğunu söyler. \
                      style projenin ya da KentOS'un duvar stilidir (kimliği ya da adı): bileşenleri (sıva, tuğla, yalıtım …), uçları ve şematik dolgusu duvara geçer; KentOS'un stili aynı adımda projeye eklenir. Bileşenlerin hepsinin kalınlığı belliyse genişlik onların toplamıdır, biri kalınlıksızsa genişliğin kalanını o alır. \
                      Duvarın yüzleri, uçları, bileşen çizgileri ve taramaları yoldan çizilir, saklanmaz; ayrıntısı çizimin ölçeğine göredir. Yayı duvarın yüzünden dar olan duvar yazılmaz (wall_collapses). \
                      arch duvarın katını (projenin katlarından), evresini, yapım durumunu ve disiplinini taşır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz. Plan yazmadan duvarları gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::WallsCreate>(),
            output: schema::<crate::WallsCreated>(),
            plan: Some(schema::<crate::WallsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir odanın dış duvarı: kapalı yol, içi solda, KentOS'un dış duvar stiliyle".into(),
                    input: json!({
                        "layerId": "duvar",
                        "walls": [{
                            "pts": [ { "x": 0.0, "y": 0.0 }, { "x": 6.0, "y": 0.0 }, { "x": 6.0, "y": 4.0 }, { "x": 0.0, "y": 4.0 } ],
                            "closed": true,
                            "justify": "left",
                            "style": "kentos-dis-duvar-25",
                            "arch": { "level": "zemin", "phase": "new" }
                        }]
                    }),
                    output: Some(json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "ids": [12],
                        "stylesAdded": ["kentos-dis-duvar-25"],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Yaylı bir bölme duvarı: 10 cm, ortası yolda".into(),
                    input: json!({
                        "layerId": "duvar",
                        "walls": [{
                            "pts": [ { "x": 2.0, "y": 1.0 }, { "x": 5.0, "y": 1.0 } ],
                            "bulges": [0.25],
                            "width": 0.1
                        }]
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_WALLS_RECTANGLE.into(),
            version: crate::CAD_WALLS_RECTANGLE_VERSION,
            title: "Dikdörtgen duvar".into(),
            summary: "Verilen dikdörtgen boyunca tek kapalı duvar oluşturur; mevcut Duvar ve Dikdörtgen araçlarını değiştirmez. \
                      frame iki karşı köşe, köşe ve boyutlar, alan ve bir kenar ya da yönlü kenarla verilir; metre ve radyan kullanır. \
                      reference inner (varsayılan, İç net) dikdörtgenin iç boşluğunu, outer (Dış ölçü) dış yüzünü, center (Eksen) duvarın orta çizgisini ölçer. \
                      İlk köşe korunur, yol saat yönünün tersine düzenlenir; tıklama çeyreği iç/dış anlamını değiştirmez. \
                      width duvar kalınlığıdır; style ve bileşenleri cad.walls.create kurallarıyla çözümlenir. Dış ya da eksen referansında kalınlık sonrası iç boy kalmazsa invalid_wall ile reddedilir. \
                      Sonuç sıradan tek kapalı duvardır; otomatik mahal veya etiket oluşturmaz. Plan yazmadan gerçek duvarı ve alınacak stilleri gösterir. \
                      Katman ve görünüş girdide açıktır, kilitli katmana yazılmaz; expectedRevision çakışmasında hiçbir şey değişmez. Tek komut günlüğü ve geri alma adımıdır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::WallsRectangle>(),
            output: schema::<crate::WallsCreated>(),
            plan: Some(schema::<crate::WallsPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İç net 4 × 3 m, 200 mm duvar".into(),
                input: json!({
                    "layerId": "duvar",
                    "frame": {"kind": "size", "corner": {"x": 0.0, "y": 0.0},
                              "length": 4.0, "width": 3.0, "toward": {"x": 4.0, "y": 3.0}},
                    "reference": "inner", "width": 0.2
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_WALLS_PARTITION.into(),
            version: crate::CAD_WALLS_PARTITION_VERSION,
            title: "Ölçülü bölme duvar".into(),
            summary: "Düz ya da yaylı bir kaynak duvar kenarından açık net yüzler arası veya eksen mesafesiyle bölme üretir: düz kenarda paralel düz, yaylı kenarda aynı merkezli yay. source kalıcı duvar kimliği ve sıfır tabanlı kenarıdır; side kaynak yolunun sol/sağıdır (saat yönü tersine yayda sol merkeze doğrudur). extent source kaynak uçlarını (yayda merkezden izdüşümleriyle), between seçilen iki düz ya da yaylı duvar kenarının merkez çizgilerini (kaynağın ortasına en yakın buluşma), length bölmenin kendi ekseni boyunca anchor ve boy/yönü kullanır. Tüm uzunluklar metredir. wall normal WallsCreate katman/stil/görünüş/öznitelik/revision kurallarını taşır; tam bir NewWall, boş pts ile verilir, yol, büküm ve eksen hizası komutla hesaplanır. Kaynak ve sınırlar okunur, değiştirilmez veya silinmez; kilitli referanslar okunabilir. Yayın merkezine ulaşan bölme, ulaşılmayan ve aynı merkezli sınır reddedilir. Yazmayan plan gövde, kaynak yüz (yayda sourceFaceBulge) ve net/eksen mesafelerini gösterir. Tek sıradan duvar, tek geri alma ve journal adımıdır.".into(),
            aliases: vec![], effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop], headless: true,
            requires: vec![CommandRequirement::Document], permissions: vec![],
            undo: CommandUndo::Step, cost: CommandCost::Instant,
            input: schema::<crate::WallsPartition>(), output: schema::<crate::WallsCreated>(),
            plan: Some(schema::<crate::WallPartitionPlan>()), capabilities: None, examples: vec![],
        },
        CommandDescriptor {
            id: crate::CAD_WALLS_FROM_LINES.into(),
            version: crate::CAD_WALLS_FROM_LINES_VERSION,
            title: "Çizgiden duvar".into(),
            summary: "Çizimin çizgilerinden duvar yapar; tek geri alma adımı “Çizgiden duvar”. \
                      mode each (Tek tek): her çizgi, açık ya da kapalı çoklu çizgi (yayları da), kapalı alanın dış halkası, yay ve daire boyunca verilen genişlik, hiza ve stille bir duvar. \
                      mode pairs (Çiftler): düz çizgiler (çizgi, iki köşeli çoklu çizgi) eşlenir, en yakını önce: doğrultuları angle dereceden (verilmezse 1°) az farklı, aralarındaki uzaklık en çok maxWidth (verilmezse 0,6 m) ve izdüşümleri örtüşen iki çizgi; duvar örtüşen kısımda ikisinin ortasından, ilk çizginin yönünde, aralarındaki uzaklık genişliğinde, ortası yolda. Eşi bulunmayan çizgi söylenir (wall_unpaired). \
                      mode contours (Kapalı kontur): yalnız kapalı çoklu çizgi, alan (bütün parçalar/delikler) veya daire; dış halka CCW, delik CW düzenlenir. justify left İç net, right Dış ölçü, center Eksen; yön değişse de aynı fiziksel referans. Kaynak öznitelik ve görünümü duvarlara geçer. Geçersiz/açık/kilitli tek kaynak bütün işlemi reddeder; Eksen referansındaki zıt taraflı ortak sınırlar mevcut 0,000001 m eşleme toleransıyla tek duvar ağına dönüşür. Ortak parçanın öznitelik/görünümü ilk kaynaktandır ve wall_shared_boundary uyarısıyla söylenir. İç net/Dış ölçüde ortak sınır veya uyumsuz asimetrik stil, yalnız nokta teması ve kesişen/üst üste odalar atomik reddedilir. Aynı katman ve aynı semantik katta aynı analitik konturdan tekrar duvar üretilmez. arch verilirse bütün yeni duvarlar o mimari bilgiyi alır; arch.level projede tanımlı bir kat kimliği olmalıdır. Verilmezse katsızdır; kaynak CAD öznitelikleri kat sayılmaz. Farklı kat duvarları birleşmez; katsızlar yalnız katsızlarla birleşir. Kot/M taşıyan konturda 2B kayıp yapılmaz, ret verilir. \
                      Kaynak çizgiler keepSources verilmezse silinir; duvarın bütünüyle örtmediği kaynak kalır ve söylenir (source_kept). Each/Pairs modlarında duvar türünden olmayan nesne bırakılır (not_a_wall_path). \
                      Duvarlar layerId verilmişse o katmana, verilmemişse her biri kaynağının katmanına yazılır. Plan yazmadan duvarları ve silinecek kaynakları gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::WallsFromLines>(),
            output: schema::<crate::WallsFromLinesDone>(),
            plan: Some(schema::<crate::WallsFromLinesPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Taranmış bir planın paralel çizgileri 30 cm'ye kadar duvar olur".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"],
                        "mode": "pairs",
                        "maxWidth": 0.3
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bir çoklu çizgi boyunca 20 cm'lik gazbeton duvar; çizgi kalsın".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "style": "Gazbeton 20",
                        "keepSources": true
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_WALLS_EDIT.into(),
            version: crate::CAD_WALLS_EDIT_VERSION,
            title: "Duvarı düzenle".into(),
            summary: "Kalıcı kimlikleriyle verilen duvarların kendi alanlarını değiştirir; tek geri alma adımı “Duvarı düzenle”, hepsi ya da hiçbiri. \
                      operation.kind set: genişlik, hiza, öteleme, stil (kimliği ya da adı; null stilsiz tek gövde), uçlar, şematik dolgu, yükseklik, kat (projenin katlarından), evre, yapım durumu ve disiplin; verilmeyenler kalır, null olanlar kalkar. \
                      Stilin bileşenleri duvara geçer; bileşenlerin sığmadığı genişlik ya da yayı yeni genişlikte çizilemeyen duvar yazılmaz. \
                      split (Böl): her duvar yolunun at noktasına en yakın yerinde ikiye; nokta duvarın iki yüzü arasında olmalı (point_off_wall), uçta olmaz (split_at_end); kapalı duvar orada açılır. join (Birleştir): uçları 1 mm içinde buluşan, alanları aynı iki ya da daha çok duvar tek duvar olur, ilk verilenin kimliğiyle (ters yöndeki gövdesi yerinde çevrilir; walls_not_touching, walls_differ); uçları buluşursa kapanır. \
                      extend (Uzat): her duvarın ucu (end, verilmezse to duvarına yakın olanı) son kenarının doğrultusunda to duvarının yoluna dek; trim (Buda): her duvar to duvarının yolunu kestiği yerde kısalır, keep noktasına en yakın parça (verilmezse uzun olanı) kalır (wall_misses, wall_closed). flip (Çevir): yol ters döner, gövde yolun öbür yanına aynalanır, uçların birleşimi yer değiştirir. \
                      replaceStyle (Stili değiştir): from stilindeki duvarlar (uids boşsa çizimin bütün duvarları) to stilini alır (style_not_used). joins (Birleşim): başlangıcın ve sonun birleşimi (auto, none, miter, butt, through) ve bileşen temizliği (cleanup); resetJoins ikisini de kurala döndürür. \
                      shift (Kaydır, docs/adr/0710): tek duvarın düz kenarı edge (verilmezse 0) kendine dik by metre kayar (yolun soluna artı, sağına eksi); yanındaki köşeler bitişik kenarlar boyunca, yol ucu ona uç uca bağlı duvarın doğrultusunda (o duvarın ucu da aynı noktaya), saplandığı duvar boyunca ya da serbestse dik kayar; kenara T ile saplanan duvarların uçları kendi doğrultularında izler; açıklıklar yerinde kalır, izleyen duvarlar çıktının joined'ındadır. distance (Mesafe): side (left ya da right) yanındaki en yakın paralel duvara net mesafe distance olacak kadar Kaydır (cad.walls.distances aynı mesafeyi okur), o duvar sabit. Yay kenar, aynı doğrultudaki bitişik kenar ya da uç uca duvar, köşede birden çok duvar, kenarı yok eden ya da ters çeviren kaydırma invalid_wall; sıfır kaydırma nothing_to_set; izleyen duvar kilitli katmandaysa layer_locked. \
                      Birleşimler duvarlardan çizilir, aynı adımda izler; düzenlemeden sonra birleşmeyen duvarlar uyarıda onaran öneriyle söylenir (wall_join_overlap, wall_join_crowded, wall_join_acute, wall_join_unresolved). Plan yazmadan duvarları gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::WallsEdit>(),
            output: schema::<crate::WallsEdited>(),
            plan: Some(schema::<crate::WallsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Duvarı 30 cm yap, sağ yüzü yolda kalsın".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "operation": { "kind": "set", "width": 0.3, "justify": "right" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "İki duvar 1. katta, yıkılacak".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"],
                        "operation": { "kind": "set", "level": "kat-1", "phase": "demolish" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Duvarı kapı yerinde ikiye böl".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "operation": { "kind": "split", "at": { "x": 3.2, "y": 0.0 } }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bölme duvarını dış duvara uzat ki T birleşsin".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "operation": { "kind": "extend", "to": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Köşede bu duvar öbürünün yüzünde dursun (Alın)".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "operation": { "kind": "joins", "end": "butt" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Çizimdeki bütün Gazbeton 20 duvarlarını İç duvar 15 yap".into(),
                    input: json!({
                        "uids": [],
                        "operation": { "kind": "replaceStyle", "from": "Gazbeton 20", "to": "İç duvar 15" }
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_WALLS_JOINS.into(),
            version: crate::CAD_WALLS_JOINS_VERSION,
            title: "Duvar birleşimleri".into(),
            summary: "Verilen duvarların (uids boşsa çizimin bütün duvarlarının) çevrelerindeki duvarlarla birleşimlerini söyler: köşe (corner: gönye ya da alın), düz devam (inline), T (tee: duvarın ucu öbürünün içinde) ve kesişme (cross); her birleşimin yeri ve duvarları (uçlarıyla). \
                      Buluşup birleşmeyen duvarlar issues'tadır: aynı doğrultuda üst üste (wall_join_overlap), ikiden çok uç bir doğruda değil (wall_join_crowded), köşe çok dar (wall_join_acute), çizgiler kesilemedi (wall_join_unresolved); her birinin ne olduğu, nasıl onarılacağı ve fix: olduğu gibi cad.walls.edit'e verilecek bir girdi (bir ucun birleşimi). \
                      T'de bağlanan duvarın bileşenleri öncelikleri (1 en güçlü) konağınkinden güçlü oldukça geçer; temizlik kapalıysa yalnız yüzler buluşur. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::WallsJoins>(),
            output: schema::<crate::WallsJoined>(),
            plan: Some(schema::<crate::WallsJoined>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimin bütün duvarlarında birleşmeyenler".into(),
                input: json!({}),
                output: None,
            }],
        },
        // Duvarın geçici ölçüleri (docs/adr/0710), held together by fixtures/commands/v1/cad.walls.distances.json.
        CommandDescriptor {
            id: crate::CAD_WALLS_DISTANCES.into(),
            version: crate::CAD_WALLS_DISTANCES_VERSION,
            title: "Duvarın mesafeleri".into(),
            summary: "Kalıcı kimlikleri verilen duvarların düz kenarı edge'de (verilmezse 0) iki yanındaki net mesafeleri okur (docs/adr/0710): yolun soluna (left) ve sağına (right) kenarın yüzünden, aynı kattaki (duvarın kendi öbür kenarları da) ona paralel ve boyunca örtüşen en yakın düz duvar kenarının bakan yüzüne dik uzaklık; uid, edge ve face o duvarı ve yüzünü, from ve to örtüşmenin ortasında iki yüzdeki noktaları verir (metre). Paralel duvarı olmayan yan yoktur. Yay kenar ya da olmayan kenar invalid_wall. Belgeye yazmaz; kenarı kaydırmak için cad.walls.edit'in distance ya da shift işlemi."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::WallsDistancesInput>(),
            output: schema::<crate::WallsDistances>(),
            plan: Some(schema::<crate::WallsDistances>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Doğu duvarının iç ve dış yanındaki mesafeler".into(),
                input: json!({ "uids": ["0192f5a1-6000-7000-8000-000000000202"] }),
                output: None,
            }],
        },
        // Kolonun geçici ölçüleri (docs/adr/0712), held together by fixtures/commands/v1/cad.columns.distances.json.
        CommandDescriptor {
            id: crate::CAD_COLUMNS_DISTANCES.into(),
            version: crate::CAD_COLUMNS_DISTANCES_VERSION,
            title: "Kolonun mesafeleri".into(),
            summary: "Kalıcı kimlikleri verilen kolonların dört yanındaki net mesafeleri okur (docs/adr/0712): her yandan (side: 0 merkezin altı, 1 sağı, 2 üstü, 3 solu, kolonun dönüşüyle; yuvarlak kolonda çevreleyen karenin yanları) dışarıya, aynı kattaki ona paralel ve boyunca örtüşen en yakın yüze dik uzaklık: bir duvarın bakan yüzü (target wall, edge yolun kenarı, face yüzü) ya da başka bir kolonun yanı (target column, edge onun yanı). from ve to örtüşmenin ortasında iki yüzdeki noktalardır (metre); paralel yüzü olmayan yan yoktur. Akslara bağlı kolon tied ile söylenir: yeri aksların kesişimidir. Belgeye yazmaz; kolonu kaydırmak için cad.arch.edit'in distance işlemi."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ColumnsDistancesInput>(),
            output: schema::<crate::ColumnsDistances>(),
            plan: Some(schema::<crate::ColumnsDistances>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Odadaki kolonun dört yanındaki duvarlara mesafeler".into(),
                input: json!({ "uids": ["0192f5a1-6000-7000-8000-000000000204"] }),
                output: None,
            }],
        },
        // Kapı, Pencere, Boşluk and Açıklığı düzenle (docs/adr/0341 §4), held together by
        // fixtures/commands/v1/cad.openings.{create,edit}.json. The web is paused (docs/adr/0228).
        // Mahal, Mahali düzenle and Metraj (docs/adr/0342 §4), held together by
        // fixtures/commands/v1/cad.spaces.{create,edit,quantities}.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_SPACES_NUMBER.into(), version: crate::CAD_SPACES_NUMBER_VERSION,
            title: "Mahalleri numarala".into(),
            summary: "Açık 1–10000 farklı mahal UUID'sini first tam ilk numarasından başlayarak ardışık numaralar. first en çok 256 karakterdir, ASCII rakamla biter, baş/son boşluk ve kontrol karakteri içermez; Z-009→Z-010. Son rakam dizisi sayıya/f64'e çevrilmez. order=input (varsayılan) UUID listesi/tıklama sırasını korur. order=direction için from ve to SI metre noktalarıdır; mevcut etiket ankrajının yön izdüşümü, eşitlikte sol normal izdüşümü, sonra UUID sıralanır. Bu bir satır/sütun ya da en yakın komşu rotası değildir. Yalnız number değişir; UUID, ad, kullanım, geometri, kaynaklar ve etiket konumları korunur, etiket/çizelge mevcut follower ile tek undo adımında yenilenir. Seçim dışındaki mahalda aynı tam numara varsa (gizli/başka kat dahil) space_number_conflict bütün yazmayı reddeder; otomatik atlama yoktur. Seçimdeki eski yinelenenler düzeltilebilir; açıklık numaraları ayrı alandır. Kilitli mahal, tekrar UUID ve geçersiz yön atomik rettir. Plan SpacesEdited.spaces/kept alanlarını numaralandırma sırasıyla verir, yazmaz; aynı girdinin tekrarı boş undo üretmez. expectedRevision eski önizlemeden yazmayı engeller.".into(),
            aliases: vec![], effect: CommandEffect::Document, hosts: vec![CommandHost::Desktop], headless: true,
            requires: vec![CommandRequirement::Document], permissions: vec![], undo: CommandUndo::Step, cost: CommandCost::Instant,
            input: schema::<crate::SpacesNumber>(), output: schema::<crate::SpacesEdited>(), plan: Some(schema::<crate::SpacesEdited>()), capabilities: None,
            examples: vec![CommandExample { title: "Tıklama sırasıyla zemin kat numaraları".into(),
                input: json!({"uids":["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],"first":"Z-001"}), output: None }],
        },
        CommandDescriptor {
            id:crate::CAD_SPACES_SEPARATORS.into(),version:crate::CAD_SPACES_SEPARATORS_VERSION,
            title:"Mahal ayırıcı".into(),
            summary:"Sıfır kalınlıklı açık mahal sınırları. operation.create: layerId ve paths (pts metre, bulges kenar başına tan(açı/4), closed, arch açık kat/evre); 1–10000 yol, toplam en çok 100000 köşe, yol başına 2–10000 köşe. Duvar malzemesi/metrajı oluşturmaz; net/eksen/brüt ayırıcıyı ötelemez. operation.set uygun tek parçalı, düzlemsel, genişliksiz çoklu çizgilere açık spaceSeparator rolü ve tam arch bilgisi verir; clear yalnız rolü kaldırır, geometriyi ve görünüşü korur. Set/clear operation.uids kalıcı nesne kimlikleridir; düz çizgi dahil diğer türler reddedilir. Sıradan çizgi, katman adı, renk veya kesikli görünüş kendiliğinden sınır değildir. Kaynak değişiminde mahal/etiket/çizelge aynı undo adımında güncellenir; açık kaynak UUID listesi ayırıcıyı yalnız listede ise kullanır. Plan yazmaz, execute tek adım, expectedRevision eski önizlemenin yazılmasını engeller. KCAD 106 rolü korur; eski dosyalar değiştirilmez.".into(),
            aliases:vec![],effect:CommandEffect::Document,hosts:vec![CommandHost::Desktop],headless:true,
            requires:vec![CommandRequirement::Document],permissions:vec![],undo:CommandUndo::Step,cost:CommandCost::Instant,
            input:schema::<crate::SpacesSeparators>(),output:schema::<crate::SpaceSeparatorsChanged>(),
            plan:Some(schema::<crate::SpaceSeparatorsPlan>()),capabilities:None,
            examples:vec![CommandExample {title:"Duvarsız bölme".into(),input:json!({"operation":{"kind":"create","layerId":"cizim","paths":[{"pts":[{"x":5.,"y":0.},{"x":5.,"y":8.}]}]}}),output:None}],
        },
        CommandDescriptor {
            id: crate::CAD_SPACES_DISCOVER.into(),
            version: crate::CAD_SPACES_DISCOVER_VERSION,
            title: "Toplu mahal bul".into(),
            summary: "Bir katın ya da bütün çizimin duvar ağındaki kapalı boşlukları bulur; duvar gövdeleri mahal değildir. level açık kat kimliğidir; verilmezse eski Mahal aracındaki Tüm katlar kuralı kullanılır. bounds iki karşı köşedir: odanın doğrulanmış iç temsil noktası bölgedeyse oda önerilir; duvarlar kesilmez. method net/centerline/gross; verilmezse projenin yöntemi. Plan her önerinin geometrisini, existing (aynı odayı kullanan mevcut mahal UUID'leri) ve conflicts (elle çizilmiş/kopuk mahalle kısmi çakışma) listelerini gösterir. Mevcut mahalın kimliği, adı, numarası ve sınırı değiştirilmez. Execute yalnız yeni mahalleri tek Mahal undo adımında yazar; bir çakışma varsa hiçbir mahal yazmaz. Yeniden çalıştırmak aynı odaları üretmez; yeni oda yoksa boş sonuç döner, undo adımı eklenmez. Açıklıklar mevcut duvar kuralında kapalı sayılır; fiziksel duvar kopukluğu kapanmaz. columns: ignore (varsayılan/eski çağrı; kolonlar düşülmez), net (yalnız Net yönteminde) veya all (her yöntemde) kalıcı kolon tercihi; aynı katın gerçek dikdörtgen/analitik daire izleri birleşik düşülür. Tam kaplanan aday atlanır, okunamayan kolon kaynağı bütün yazmayı reddeder. Bölge temsil noktası duvar yüzüne aittir; kolon düşümü bu oda kimliğini değiştirmez. Sanal sınır yalnız açık spaceSeparator rolündeki çoklu çizgilerden gelir; sıfır kalınlıklıdır ve kaynak katı/kapsamı/UUID listesine uyar. Kaynak duvar ve oda başına 10000 kota vardır. layerId hedef katmandır; attrs yeni mahallerin tipli öznitelikleridir. expectedRevision eski önizlemeden yazmayı engeller. Geometri SI metre olarak kalır.".into(),
            aliases: vec![], effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop], headless: true,
            requires: vec![CommandRequirement::Document], permissions: vec![],
            undo: CommandUndo::Step, cost: CommandCost::Instant,
            input: schema::<crate::SpacesDiscover>(),output: schema::<crate::SpacesCreated>(),
            plan: Some(schema::<crate::SpacesDiscovered>()),capabilities: None,
            examples: vec![CommandExample{title:"Zemin kattaki yeni mahaller".into(),input:json!({"layerId":"mahal","level":"zemin"}),output:None}],
        },
        CommandDescriptor {
            id: crate::CAD_SPACES_CREATE.into(),
            version: crate::CAD_SPACES_CREATE_VERSION,
            title: "Mahal oluştur".into(),
            summary: "Mahaller (odalar) ekler; tek geri alma adımı “Mahal”. \
                      Her mahal ya duvarlardan bulunur (seed: duvarlarla çevrili bir iç nokta; sınırı duvarların yüzlerinden, kapı boşlukları kapalı sayılarak, delikleriyle bulunur ve duvar değişince belge yeniden bulur) ya da sınırı çizilir (parts: dış halka ve delikler, birden çok parça olabilir). \
                      method sınırın neyi gösterdiğidir: net (iç yüzler), centerline (duvar eksenleri: her kenar duvarın yarı kalınlığı kadar duvara kayar), gross (dış yüzler: tam kalınlık); verilmezse projenin yöntemi (settings.arch.spaceMethod), o da yoksa net. \
                      columns: ignore (yoksa eski davranış), net (yalnız net alanda) veya all (her yöntemde) fiziksel kolon düşümüdür; yalnız seed ile ilişkili mahalda verilir. Aynı katın gerçek kolonları dolgu/görünürlükten bağımsız düşülür; örtüşmeler bir kez sayılır, duvar veya kolon değişince aynı adımda yeniden bulunur. Tam kaplanma veya okunamayan kaynak invalid_space; mevcut oda son sınırı ve noFreeArea/invalidBoundarySource nedeni ile kalır. \
                      number, name, usage, arch (kat, evre), height (yoksa katın), finishes (döşeme, duvar, tavan kaplaması) ve katmanın tipli alanlarıyla öznitelikler. Alan, çevre ve hacim sınırdan ölçülür; yöntemin bir mevzuata uyduğu söylenmez. \
                      arch.level açıkça verilirse sınır yalnız aynı kat kimliğinin duvarlarından bulunur ve izlenir; eşit kot farklı katları birleştirmez. Kat belirtilmeyen eski çağrılar bütün çizimdeki duvarları kullanmayı sürdürür. \
                      Kapanmayan duvarlarda mahal yazılmaz (space_not_enclosed). Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. Plan yazmadan mahalleri sınırlarıyla gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SpacesCreate>(),
            output: schema::<crate::SpacesCreated>(),
            plan: Some(schema::<crate::SpacesPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Duvarlarla çevrili odanın içine tıklanmış gibi: net alanlı Salon".into(),
                    input: json!({
                        "layerId": "mahal",
                        "spaces": [{ "seed": { "x": 3.0, "y": 2.0 }, "number": "Z-01", "name": "Salon", "usage": "Oturma" }]
                    }),
                    output: Some(json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61"], "ids": [21], "revision": "40" })),
                },
                CommandExample {
                    title: "Çizilmiş sınırlı bir depo, duvar ekseninden".into(),
                    input: json!({
                        "layerId": "mahal",
                        "spaces": [{
                            "parts": [{ "outer": { "pts": [ { "x": 0.0, "y": 0.0 }, { "x": 3.0, "y": 0.0 }, { "x": 3.0, "y": 2.0 }, { "x": 0.0, "y": 2.0 } ] } }],
                            "method": "centerline", "name": "Depo"
                        }]
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SPACES_EDIT.into(),
            version: crate::CAD_SPACES_EDIT_VERSION,
            title: "Mahali düzenle".into(),
            summary: "Kalıcı kimlikleri verilen mahalleri değiştirir; tek geri alma adımı “Mahali düzenle”, hepsi ya da hiçbiri. \
                      operation: set (numara, ad, kullanım, yöntem, columns kolon tercihi, kat, evre, yükseklik, kaplamalar; null alanı kaldırır; duvarlardan bulunan mahalin yöntemi değişince sınırı yeniden bulunur), unit bina programının bölümü, null bölümden çıkarır, programda olmayan unit_not_found), merge (mahaller birleşir: keep verilen mahalin, yoksa ilkinin kimliği sürer, numarası ve bölümü onun, number başka numara verir, null kaldırır; öbürleri silinir, numaraları, kaplamaları ve etiketleri onlarla gider, cevabın decisions'ı yazmadan önce söyler; bölümleri farklı mahaller space_units_merged uyarısı; birleşen sınır çizilmiş olur; değmeyen mahaller spaces_not_touching; keep birleşenlerden değilse invalid_input), split (line: iki ya da daha çok nokta; mahal çizgiyle bölünür, keep noktasını içeren parça, yoksa en büyük kimliğini korur; yeni parçalar büyüklük sırasıyla numbers numaralarını alır, verilmezse numarasız kalır (aynı numara iki kez yazılmaz), bölümü alır, finishes false değilse kaplamaları; decisions her parçanın kimliğini, numarasını, alanını, kaplamalarını ve içinde duran donatıları söyler; bölmeyen çizgi split_missed, parça sayısıyla uymayan numbers ya da hiçbir parçada olmayan keep invalid_input), rebound (sınır duvarlardan yeniden bulunur: seed verilirse ondan, yoksa kendi tohumundan, o da yoksa etiket noktasından; çizilmiş mahal duvarlardan bulunan olur). \
                      Kat değişimi de duvar sınırını yeniden bulur; seçilen kat tohumu kapatmıyorsa son sınır korunur ve lostEnclosure gösterilir. Kolon tercihi değişimi de aynı yeniden hesabı yapar; yalnız ilişkili seed mahalda etkin olabilir. Böl/Birleştir sonucu çizilmiş sınır ve kolon tercihi ignore olur; o anki delikler korunur. Plan ve tek geri alma aynı mahal/sınır değişimini kapsar. \
                      Cevap kept (kimliği korunanlar), added (yeni mahaller) ve removed (silinenler) ile söyler. Kilitli katmandaki mahal bütün değişikliği reddeder. Plan yazmadan mahalleri olacakları gibi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SpacesEdit>(),
            output: schema::<crate::SpacesEdited>(),
            plan: Some(schema::<crate::SpacesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki mahali birleştir".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e62"],
                        "operation": { "kind": "merge" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Mahali bir çizgiyle ikiye böl".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61"],
                        "operation": { "kind": "split", "line": [ { "x": 3.0, "y": -1.0 }, { "x": 3.0, "y": 5.0 } ] }
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SLABS_CREATE.into(),
            version: crate::CAD_SLABS_CREATE_VERSION,
            title: "Döşeme oluştur".into(),
            summary: "Döşeme, balkon ya da teras ekler; tek geri alma adımı “Döşeme”. \
                      Her döşemenin sınırı ya çizilir (parts: dış halka ve delikler, birden çok parça olabilir: L kontur, avlu, ıslak hacim deliği) ya da duvarlardan bulunur (seed: bir iç nokta; boundary outline duvarların dış yüzlerine kadar yapının konturu, room noktayı çevreleyen odanın iç yüzleri). Kat verilmişse yalnız o katın duvarları, verilmemişse bütün duvarlar kullanılır; bulunan sınır duvarlara bağlı kalmaz. \
                      edgeOffset bütün kenarları aynı mesafede öteler (artı dışa: duvarların altına ya da çıkma, eksi içe); halka ters döner, kendini ya da başka halkayı keserse yazılmaz (slab_offset_failed). \
                      role (floor döşeme, balcony balkon, terrace teras), thickness (yoksa 15 cm), offset (üst yüzün katın kotuna göre farkı; ıslak hacimde eksi), arch (kat, evre), material ve katmanın tipli alanlarıyla öznitelikler. \
                      edge serbest kenarlar boyunca korkuluk (railing, iki ince çizgi) ya da parapet (dolu bant) çizer; en verilmezse türünün (5 cm, 15 cm). Serbest kenar, ortasından aynı katın hiçbir duvar yüzü geçmeyen kenardır; belge her adımda duvarlara göre yeniden yazar. \
                      excluded metrajda listeler ama toplamaz. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. Plan yazmadan döşemeleri sınırları ve kenarlarıyla gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SlabsCreate>(),
            output: schema::<crate::SlabsCreated>(),
            plan: Some(schema::<crate::SlabsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Zemin katın döşemesi: duvarların dış yüzlerinden, 20 cm".into(),
                    input: json!({
                        "layerId": "doseme",
                        "slabs": [{ "seed": { "x": 3.0, "y": 2.0 }, "thickness": 0.2, "arch": { "level": "zemin" } }]
                    }),
                    output: Some(json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e71"], "ids": [31], "revision": "41" })),
                },
                CommandExample {
                    title: "Korkuluklu balkon: çizilmiş sınır".into(),
                    input: json!({
                        "layerId": "doseme",
                        "slabs": [{
                            "parts": [{ "outer": { "pts": [ { "x": 0.0, "y": 6.0 }, { "x": 4.0, "y": 6.0 }, { "x": 4.0, "y": 7.5 }, { "x": 0.0, "y": 7.5 } ] } }],
                            "role": "balcony", "edge": { "kind": "railing" }
                        }]
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SLABS_EDIT.into(),
            version: crate::CAD_SLABS_EDIT_VERSION,
            title: "Döşemeyi düzenle".into(),
            summary: "Kalıcı kimlikleri verilen döşemeleri değiştirir; tek geri alma adımı “Döşemeyi düzenle”, hepsi ya da hiçbiri. \
                      operation: set (tür, kalınlık, kot farkı, kat, evre, malzeme, kenar, metraj dışı; null alanı kaldırır, edge null kenarı kaldırır), offset (distance: bütün kenarlar ötelenir, artı dışa), subtract (parts ya da seed/boundary ile bulunan alan döşemeden çıkarılır: delik, girinti; döşemeye değmeyen ya da hepsini götüren alan slab_cut_missed), add (aynı alan döşemeye eklenir). \
                      Kenarın parçaları belgede duvarlara göre yeniden yazılır. Kilitli katmandaki döşeme bütün değişikliği reddeder. Plan yazmadan döşemeleri olacakları gibi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SlabsEdit>(),
            output: schema::<crate::SlabsEdited>(),
            plan: Some(schema::<crate::SlabsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Islak hacmin alanını döşemeden çıkar".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e71"],
                        "operation": { "kind": "subtract", "seed": { "x": 1.0, "y": 1.0 }, "boundary": "room" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Döşemeyi 5 cm aşağı al, parapet ver".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e71"],
                        "operation": { "kind": "set", "offset": -0.05, "edge": { "kind": "parapet" } }
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SPACES_QUANTITIES.into(),
            version: crate::CAD_SPACES_QUANTITIES_VERSION,
            title: "Metraj".into(),
            summary: "Çizimin mahallerini, duvarlarını ve açıklıklarını kaynak geometriden ölçer; çizimi değiştirmez. \
                      Mahal: numara, ad, kullanım, kat, yöntem, alan (halkaların alanı, delikler düşülür), çevre, yükseklik (kendi, yoksa katın), hacim; duvarlardan bulunanda üç yöntemin alanı ayrı (byMethod). Etkin columns tercihi her yöntem hesabına uygulanır. Her kopuk sınırda, kolon düşümü kapalı ve kaynak listesi olmayan eski mahaller dahil, boundaryBreak nedenini ve rule son geçerli sınır olduğunu söyler; alan/çevre/hacim ve toplamlar son geçerli geometriyi içerir. Geçerli satırlarda boundaryBreak yoktur. \
                      Duvar: stil, kat, uzunluk (taban çizgisi), kalınlık, yükseklik, brüt yüzey (uzunluk × yükseklik), açıklıkların alanı, net yüzey, hacim (net yüzey × kalınlık), açıklıklarının kimlikleri. \
                      Açıklık: biçim, tür, numara, duvarı, genişlik, yükseklik (lento − denizlik), alan. Her satır kuralını (rule) söyler; toplamlar satırların sırasıyla toplanır. \
                      Oda (boşluk olmayan mahal): süpürgelik (baseboard), halkalarının uzunluğundan kenarları boyunca duran geçişlerin (kapı, döşemeye inen boşluk; düz kenarda) genişlikleri düşülerek. Döşeme (slabs): tür, kat, malzeme, kalınlık, kot farkı, üst kot (katın kotu + fark), alan (delikler düşülür), aynı kattaki boşluk mahallerinin kestiği alan (voidArea), net alan, çevre, hacim (net alan × kalınlık), kenar türü ve uzunluğu; excluded döşeme listelenir, toplanmaz. Geçişler (transitions): iki yanında döşeme kaplaması bilinen ve farklı iki oda olan geçişler, genişlikleri ve kaplamalarıyla. \
                      Rampa (ramps, docs/adr/0723 §3): kat, genişlik, kot farkı, çizildiği eğim (kot farkının büyüklüğü ÷ eğimli uzunluk; eğimli kolu yoksa yok), eğimli uzunluk (orta çizgi eksi sahanlıklar), orta çizginin uzunluğu, sahanlık sayısı, plan alanı (dönüşlerde gönyeli şerit), başlangıç ve bitiş kotu, kata bağlıysa farkı (baseOffset), korkuluk çizgilerinin uzunluğu; toplamlarda rampArea ve rampRailingLength. \
                      levels, phases, usages ve layers verilen kat, evre, kullanım ve katmanlarla süzer (usages verilince yalnız mahaller)."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SpacesQuantities>(),
            output: schema::<crate::SpacesQuantified>(),
            plan: Some(schema::<crate::SpacesQuantified>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Zemin katın metrajı".into(),
                input: json!({ "levels": ["zemin"] }),
                output: None,
            }],
        },
        // Aks, Kolon, Merdiven, Rampa and Mimari nesneyi düzenle (docs/adr/0343 §4), held together by
        // fixtures/commands/v1/cad.{grids,columns,stairs,ramps}.create.json and cad.arch.edit.json. The web is paused.
        CommandDescriptor {
            id: crate::CAD_GRIDS_CREATE.into(),
            version: crate::CAD_GRIDS_CREATE_VERSION,
            title: "Aks".into(),
            summary: "Akslar ekler; tek geri alma adımı “Aks”. grids: her biri iki nokta (pts; bulges ile yaylı), etiket (label), balonlar (bubbles: both, start, end, none) ve balon çapı (bubbleSize, kâğıtta mm; yoksa 8). \
                      series: paralel akslar: ilki from → to, her sonraki spacings'teki aralık kadar side (left, right) yana; ilk etiket start. \
                      Etiket verilmezse bir öncekinin sıradakidir: harfler A … H, J … N, P … Z, AA (I ve O atlanır), sayılar 1, 2 …; çağrının ilki çizimin son aksından sonrakini, aks yoksa A alır. \
                      radial (docs/adr/0725 §1): eğrisel aks ağı: center'dan radyal akslar, ilki angle doğrultusunda, sonrakiler turns'teki açılar kadar (radyan; hepsi aynı yöne, toplamı tam turdan az), center çevresinde yaylar, ilki radius'ta, sonrakiler rings kadar dışarıda; her aks extension (öntanımlı 1.5 m) kadar uzar: radyal ilk yaydan o kadar içeriden (merkezi geçmeden) son yaydan o kadar dışarıya, yay kendi boyunca dış radyallerden öteye (kalan turun dörtte birini geçmeden); etiketler radialStart (1) ve arcStart (A); columns kesişimlere yay yay akslara bağlı kolonlar, eni radyal aksı boyunca.                       ends (aksın, serinin, ağların): kesik aks, yalnız uçlarında kâğıtta bu kadar mm (0'dan büyük, en çok 1000).                       Aksın çizimi (kesikli çizgi, balonlar, etiket) alanlarından türer. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::GridsCreate>(),
            output: schema::<crate::GridsCreated>(),
            plan: Some(schema::<crate::GridsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "A–D aksları 6 m arayla, 1–3 aksları 5 m arayla".into(),
                    input: json!({ "layerId": "aks", "series": { "from": { "x": 0.0, "y": -2.0 }, "to": { "x": 0.0, "y": 12.0 }, "spacings": [6.0, 6.0, 4.5], "side": "right", "start": "A" } }),
                    output: Some(json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e62", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e63", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e64"], "ids": [21, 22, 23, 24], "revision": "40" })),
                },
                CommandExample {
                    title: "Yalnız başında balonlu tek aks".into(),
                    input: json!({ "layerId": "aks", "grids": [{ "pts": [{ "x": -2.0, "y": 0.0 }, { "x": 20.0, "y": 0.0 }], "label": "1", "bubbles": "start" }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_COLUMNS_CREATE.into(),
            version: crate::CAD_COLUMNS_CREATE_VERSION,
            title: "Kolon".into(),
            summary: "Kolonlar ekler; tek geri alma adımı “Kolon”. columns: her biri merkez (at) ya da iki aks (grids: kalıcı kimlikler; kolon kesişimlerinde durur, at verilirse ona en yakın kesişimde), biçim (shape: rect, round, lShape), en (width), boy (depth; yoksa en), L kolonun kolları (legs: en boyunca kolun boy yönündeki ve boy boyunca kolun en yönündeki kalınlığı; yalnız L'de, gerekli, o yanından ince; docs/adr/0725 §3), hiza (anchor: center, left, right, bottom, top, bottomLeft, bottomRight, topLeft, topRight; at ve aks kesişimi kutunun bu noktasıdır, §2), dönüş (rotation, radyan, hiza noktası çevresinde), tür (type), etiket (tag), dolu (fill). \
                      crossings: verilen aksların her çiftinin, ikisinin de içinde kalan kesişimlerinde birer kolon; aynı ölçülerle. Etiketsiz kolonlar tagStart'tan artarak etiketlenir (S1, S2 …). \
                      Akslara bağlı kolonun yerini belge izler: aks taşınınca kolon kesişimde kalır, konum yazısı (place) iki aksın etiketlerinden “B-3” olur; aks silinir ya da akslar kesişmezse broken: gridGone. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ColumnsCreate>(),
            output: schema::<crate::ColumnsCreated>(),
            plan: Some(schema::<crate::ColumnsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bütün aks kesişimlerine 30 × 50 kolon".into(),
                    input: json!({ "layerId": "kolon", "crossings": { "grids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e62", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e65"], "width": 0.3, "depth": 0.5, "fill": true }, "tagStart": "S1" }),
                    output: None,
                },
                CommandExample {
                    title: "Tek yuvarlak kolon".into(),
                    input: json!({ "layerId": "kolon", "columns": [{ "at": { "x": 3.0, "y": 4.0 }, "shape": "round", "width": 0.4, "tag": "S9" }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_STAIRS_CREATE.into(),
            version: crate::CAD_STAIRS_CREATE_VERSION,
            title: "Merdiven".into(),
            summary: "Merdivenler ekler; tek geri alma adımı “Merdiven”. Her biri yürüme çizgisi (pts: 2 nokta düz, 3 L, 4 U; yolun yönü çıkış yönüdür), genişlik, rıht sayısı (risers), rıht yüksekliği (riserHeight), basamak derinliği (tread), dönüşteki sahanlığın boyu (landing; yoksa genişlik), kesme çizgisinin rıhtı (cut), basamak numaraları (numbering). \
                      Basamak çizgileri başlangıçtan tread aralıkla, sahanlıkları atlayarak çizilir. Uymayanlar uyarıdır, ret değildir: yol rıhtlara yetmiyorsa stair_short, uzunsa stair_long; 2 × rıht + basamak 0,60–0,65 m dışındaysa stair_step_rule (yalnız bilgi); katın yüksekliği biliniyorsa rıht sayısı × rıht onu tutmuyorsa stair_rise_level. \
                      Hiçbir söz yönetmeliğe uyum söylemez. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::StairsCreate>(),
            output: schema::<crate::StairsCreated>(),
            plan: Some(schema::<crate::StairsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "U merdiven: 18 rıht, 17 cm rıht, 28 cm basamak".into(),
                    input: json!({ "layerId": "merdiven", "stairs": [{ "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 3.0, "y": 0.0 }, { "x": 3.0, "y": 2.0 }, { "x": 0.0, "y": 2.0 }], "width": 1.0, "risers": 18, "riserHeight": 0.17, "tread": 0.28, "cut": 9 }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_RAMPS_CREATE.into(),
            version: crate::CAD_RAMPS_CREATE_VERSION,
            title: "Rampa".into(),
            summary: "Rampalar ekler; tek geri alma adımı “Rampa”. Her biri orta çizgi (pts: 2–4 nokta), genişlik, toplam yükseklik (rise), eğim (slope, yüzde; yoksa yükseklik ve eğimli uzunluktan), başlangıç kotu (base; yoksa 0) ya da katına göre farkı (baseOffset, docs/adr/0723 §2: başlangıç katın kotu artı fark, kat kotu değişince izler; katı ister, base ile birlikte verilmez), dönüşteki sahanlığın boyu (landing; yoksa genişlik), fit (son kol eğime göre biter, docs/adr/0705), maxRun (en uzun eğimli kol: aşan kola düz köşe ve ara sahanlık, docs/adr/0723 §1), korkuluk (railings). \
                      Uçlarında kot yazısı (+0.00 gibi), eğimli kollarında ok. Verilen eğim yükseklik bölü eğimli uzunluktan 0,1 puandan çok farklıysa ramp_slope_mismatch, sahanlıklar bütün uzunluğu alıyorsa ramp_no_run uyarısı; ret değildir. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::RampsCreate>(),
            output: schema::<crate::RampsCreated>(),
            plan: Some(schema::<crate::RampsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "10 m'lik %5 rampa".into(),
                    input: json!({ "layerId": "rampa", "ramps": [{ "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }], "width": 1.5, "rise": 0.5, "slope": 5.0 }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ARCH_EDIT.into(),
            version: crate::CAD_ARCH_EDIT_VERSION,
            title: "Mimari nesneyi düzenle".into(),
            summary: "Kalıcı kimlikleri verilen aks, kolon, merdiven ve rampaları değiştirir; tek geri alma adımı “Mimari nesneyi düzenle”, hepsi ya da hiçbiri. \
                      operation: set (türün alanları: aksın label, bubbles, bubbleSize, ends (null bütün aks); kolonun shape (L olmayan biçim kolları kaldırır), width, depth, legs, anchor (yeri kalır, gövdesi onun çevresinde kayar), rotation, type, tag, fill; merdivenin width, risers, riserHeight, tread, landing, cut, numbering; rampanın width, rise, slope, base (kat bağını kaldırır), baseOffset (katına bağlar; null bağı kaldırır, kot yerinde kalır; base ile birlikte verilmez), landing; hepsinin level, phase; null alanı kaldırır; nesnenin türünde olmayan alan field_not_for_kind), renumber (akslar ya da kolonlar, tek tür, verildiği sırayla start'tan; aks etiketi değişince bağlı kolonların konum yazısı da), attach (kolonları iki aksın kesişimine bağlar, merkezlerine en yakın kesişime), detach (bağı koparır, kolon yerinde kalır), distance (docs/adr/0712: tek kolon, yanı side — 0 merkezin altı, 1 sağı, 2 üstü, 3 solu, dönüşüyle — baktığı paralel yüzden distance metre olacak kadar o yana dik kayar, karşıdaki yüz sabit; cad.columns.distances aynı mesafeyi okur. Akslara bağlı kolon column_on_grids, kolon olmayan not_a_column, o yanda paralel yüz yoksa ya da eksi mesafe invalid_column, sıfır kaydırma nothing_to_set). \
                      Kilitli katmandaki nesne bütün değişikliği reddeder. Merdiven ve rampanın denetimleri uyarıdır. Plan yazmadan nesneleri olacakları gibi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ArchEdit>(),
            output: schema::<crate::ArchEdited>(),
            plan: Some(schema::<crate::ArchEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Aksları yeniden numarala".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e62"], "operation": { "kind": "renumber", "start": "A" } }),
                    output: None,
                },
                CommandExample {
                    title: "Merdivenin rıht sayısını değiştir".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e66"], "operation": { "kind": "set", "risers": 17, "riserHeight": 0.18 } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_TAGS_CREATE.into(),
            version: crate::CAD_TAGS_CREATE_VERSION,
            title: "Etiket".into(),
            summary: "Nesnelere etiket koyar; tek geri alma adımı “Etiket”. Her etiket nesnesinin kalıcı kimliğiyle (of) verilir: açıklık, mahal, kolon, aks, duvar, merdiven, rampa ya da blok; başka nesne not_taggable. \
                      content şablondur: {number}, {type}, {form}, {width}, {height}, {sill}, {head}, {name}, {usage}, {area}, {perimeter}, {level}, {phase}, {tag}, {place}, {label}, {style}, {risers}, {slope}, {floorElevation}, {ceilingElevation}, {netArea} ve {attr:Ad}, satırlar \\n ile, en çok 6 satır; verilmezse türün şablonu (açıklıkta {number}, mahalde {number}, {name} ve {area} üç satır, kolonda {tag}, aksta {label}, duvarda {style}, merdivende {risers}, rampada {slope}, blokta {attr:NO}). \
                      Mahalın floorElevation değeri bilinen katın kotu; ceilingElevation kat kotu + açık mahal yüksekliğidir. Katın döşemeden döşemeye yüksekliği tavan sayılmaz; eksik kat/yükseklik boş yazılır. Kot yazımı diğer uzunluklar gibi çizim birimi ve basamağındadır. netArea mahalın aynı kat/kapsam/kaynak ve kolon tercihiyle iç yüzler alanıdır; area seçilmiş yöntem alanı olarak kalır. Kopuk sınır veya elle çizilmiş brüt/eksen mahalında netArea boş kalır. \
                      shape çerçevedir (circle, hexagon, rectangle, rounded, none; verilmezse açıklıkta altıgen, mahalde dikdörtgen, ötekilerde daire), size yazının kâğıttaki yüksekliği (mm, verilmezse 2,5), leader çerçeveden nesnenin bağ noktasına çizgi. at verilmezse mahalde bağ noktası, ötekilerde onun 4 yazı yüksekliği üstü. \
                      all verilen türlerin etiketsiz ve görünen nesnelerine etiket koyar. all.refresh yok/null/false mevcut etiketleri korur. true yalnız layerId katmanındaki uygun mevcut etiketlere açıkça verilen all.content/shape/size alanlarını uygular; boş alan mevcut değeri korur. Kimlik, elle konum, kol ve diğer özellikler korunur; başka etiket katmanlarına dokunulmaz ve orada etiketi bulunan kaynak için ikinci etiket yaratılmaz. all.layers kaynak CAD katmanlarını, all.phases kaynak evrelerini, all.levels mimari kat kimliklerini süzer. levels yok/null tüm katlar, [null] yalnız atanmamış, dolu kimlik listesi verilen katlar, [] hiçbir kaynaktır; birleşim ve tekrar desteklenir. Dolu kimlik projede yoksa level_not_found/all.levels[i] tüm çağrıyı reddeder. Açıklık katını duvardan alır; host kayıpsa/duvar değilse açık kat ve atanmamış kapsamına girmez. Bloklar kat atanmamış sayılır. Bu süzgeç görünüş/pafta bağlamı değildir. Açık tags girdisindeki kaynaklar all tarafından yeniden işlenmez. Plan tags yeni, refreshed değişen mevcut etiketlerdir; çıktı uids/ids yalnız yeni etiketler, isteğe bağlı refreshed yenilenen UUID'lerdir. Değişmeyenler yazılmaz; yeni+yenilenen en çok10.000 ve birlikte tek undo'dur. \
                      Etiketin yazısı ve bağ noktası belgenindir: nesne değişince aynı adımda yeniden yazılır, nesne taşınınca etiket de taşınır, nesne silinince etiket son yazısıyla targetGone kalır. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::TagsCreate>(),
            output: schema::<crate::TagsCreated>(),
            plan: Some(schema::<crate::TagsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kapıya numara etiketi".into(),
                    input: json!({ "layerId": "etiket", "tags": [{ "of": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61" }] }),
                    output: None,
                },
                CommandExample {
                    title: "Bütün mahalleri etiketle".into(),
                    input: json!({ "layerId": "etiket", "all": { "kinds": ["space"] } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_TAGS_EDIT.into(),
            version: crate::CAD_TAGS_EDIT_VERSION,
            title: "Etiketi düzenle".into(),
            summary: "Kalıcı kimlikleri verilen etiketlerin şablonunu (content; null türün şablonu), çerçevesini (shape), yazı yüksekliğini (size, kâğıtta mm; null 2,5) ya da kolunu (leader) değiştirir; yazıları nesnelerinden yeniden yazılır. Tek geri alma adımı “Etiketi düzenle”, hepsi ya da hiçbiri; kilitli katmandaki etiket bütün değişikliği reddeder. Plan yazmadan etiketleri olacakları gibi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::TagsEdit>(),
            output: schema::<crate::TagsEdited>(),
            plan: Some(schema::<crate::TagsEdited>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Etiketi dikdörtgen çerçeveye al, numara ve tür yaz".into(),
                input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e71"], "change": { "shape": "rectangle", "content": "{number}\n{type}" } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_TAGS_CHECK.into(),
            version: crate::CAD_TAGS_CHECK_VERSION,
            title: "Etiketleri denetle".into(),
            summary: "Açıklıkları, mahalleri ve etiketleri denetler; çizimi değiştirmez. Bulgular: tag_duplicate_number (aynı numara birden çok açıklıkta ya da mahalde), tag_missing_type (türü olmayan açıklık), tag_host_broken (duvarı olmayan, duvarının dışındaki ya da çakışan açıklık), tag_target_gone (nesnesi olmayan etiket), tag_untagged (etiketsiz açıklık ya da mahal). \
                      missingSpaceNumbers=true ayrıca etiketi olsa da numarası olmayan her mahalı tag_missing_space_number ile yinelenen numaralardan sonra raporlar; adı ve katı mesajda, UUID ve noktası bulgudadır. Yok/null/false eski bulguları ve sırasını korur. Boş numara alanın yokluğudur; boş/yalnız boşluk metin mevcut mahal sözleşmesinde reddedilir. Tam numara metinleri karşılaştırılır; Z-001 ve Z-1 farklı, açıklık/mahal ayrı gruplardır. Gizli/kilitli ve diğer katlar dahil; yalnız layers açıkça verilirse o katmanlarla sınırlar. Diğer bulgular yukarıdaki kod sırasıyla, her grup çizimdeki sırayladır. Salt okunurdur; otomatik numaralama, düzeltme veya undo üretmez."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::TagsCheck>(),
            output: schema::<crate::TagsChecked>(),
            plan: Some(schema::<crate::TagsChecked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bütün çizimi denetle".into(),
                input: json!({}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_VIEWMARKS_CREATE.into(),
            version: crate::CAD_VIEWMARKS_CREATE_VERSION,
            title: "Görünüm işareti".into(),
            summary: "Kesit, görünüş ve detay işaretleri ekler; tek geri alma adımı “Görünüm işareti”. Her işaret biçimiyle (form: section, elevation, detail) ve noktalarıyla verilir: kesitin kesme çizgisi 2–32 nokta (flip bakış yönünü çevirir), görünüş tek nokta ve bakış yönü (angle, derece, doğudan saat yönünün tersine; yoksa 90), detay merkezi ve sınırının yarıçapı (radius), balonun yeri (at; yoksa sağ üstte iki yarıçap uzakta). \
                      mark balonun üst yarısındaki numaradır (en çok 16 harf); target işaretin baktığı yerdir: adlı görünüm ({view}) ya da paftadaki görünüm penceresi ({sheet, item}); projede olmayan hedef target_not_found. Balonun alt yarısına hedefin numarası yazılır (paftanın adı ya da görünümün adı; hedef yoksa “—”), mark boşsa üst yarıya pencerenin paftadaki sırası; yazılar belgenindir: pafta ya da görünüm yeniden adlandırılınca aynı anda yeniden yazılır, hedef silinince işaret son yazısıyla targetGone kalır. \
                      size yazının kâğıttaki yüksekliğidir (mm, yoksa 2,5). Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ViewMarksCreate>(),
            output: schema::<crate::ViewMarksCreated>(),
            plan: Some(schema::<crate::ViewMarksPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "A-A kesiti, A3 paftasının ilk penceresine".into(),
                    input: json!({ "layerId": "isaret", "marks": [{ "form": "section", "points": [{ "x": -1.0, "y": 2.0 }, { "x": 9.0, "y": 2.0 }], "mark": "A", "target": { "sheet": "pafta-1", "item": "harita-1" } }] }),
                    output: None,
                },
                CommandExample {
                    title: "Mutfağın detayı".into(),
                    input: json!({ "layerId": "isaret", "marks": [{ "form": "detail", "points": [{ "x": 4.0, "y": 3.0 }], "radius": 1.2, "mark": "1" }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_VIEWMARKS_EDIT.into(),
            version: crate::CAD_VIEWMARKS_EDIT_VERSION,
            title: "Görünüm işaretini düzenle".into(),
            summary: "Kalıcı kimlikleri verilen görünüm işaretlerini değiştirir; tek geri alma adımı “Görünüm işaretini düzenle”, hepsi ya da hiçbiri. operation: set (mark, size: kâğıtta mm, null 2,5; flip; angle yalnız görünüşte; radius yalnız detayda), retarget (target: adlı görünüm ya da paftadaki pencere; projede yoksa target_not_found), detach (hedefi bırakır, alt yarıya “—” yazılır). \
                      Yazıları hedeflerinden yeniden yazılır. Kilitli katmandaki işaret bütün değişikliği reddeder. Plan yazmadan işaretleri olacakları gibi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ViewMarksEdit>(),
            output: schema::<crate::ViewMarksEdited>(),
            plan: Some(schema::<crate::ViewMarksEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kesiti adlı görünüme bağla".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e81"], "operation": { "kind": "retarget", "target": { "view": "v-1a2b3c4d" } } }),
                    output: None,
                },
                CommandExample {
                    title: "Bakış yönünü çevir".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e81"], "operation": { "kind": "set", "flip": true } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_VIEWMARKS_REFERENCES.into(),
            version: crate::CAD_VIEWMARKS_REFERENCES_VERSION,
            title: "Başvurular".into(),
            summary: "Çizimin görünüm işaretlerini ve bakabilecekleri hedefleri listeler; çizimi değiştirmez. Her işaret: kimliği, biçimi, numarası, hedefi, yazısı, kopukluğu, ilk noktası (çizimdeki sırayla). Her hedef: adlı görünümler projenin sırasıyla, sonra pafta kitabının görünüm pencereleri kitabın sırasıyla; adı (paftada “Pafta › Pencere”), alt yarıya yazılacak numarası, pencerenin paftadaki sırası ve ona bakan işaretler."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ViewMarksReferences>(),
            output: schema::<crate::ViewMarksReferenced>(),
            plan: Some(schema::<crate::ViewMarksReferenced>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İşaretler ve hedefler".into(),
                input: json!({}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_SPACES_FINISH.into(),
            version: crate::CAD_SPACES_FINISH_VERSION,
            title: "Döşeme ve tavan".into(),
            summary: "Mahallere döşeme deseni ya da tavan ızgarası yazar: her mahalin her parçasına mahale bağlı bir tarama, Kaplama özniteliğiyle (Döşeme, Tavan); mahal duvarlarıyla değişince tarama da izler. Tek geri alma adımı “Döşeme ve tavan”. \
                      surface: floor ise pattern kitaplığın ya da projenin desen adıdır (yoksa mahalin döşeme kaplamasının adıyla aynı adlı desen, o da yoksa ANSI31; bilinmeyen ad unknown_pattern), scale desenin ölçeği; desen mahalin sol alt köşesinden başlar. surface: ceiling ise grid ızgaradır: dx ve dy aralıklar (m), angle derece, origin center (mahalin ortası, karşılıklı duvarlarda eşit kesik) ya da corner. Döşemeye ızgara, tavana desen invalid_finish. \
                      Aynı yüzün eski taraması yerine geçer (finish_replaced). layerId verilmezse A-ZEMN-DSN ya da A-TAVN-IZG, yoksa aynı adımda açılır. Plan yazmadan taramaları olacakları gibi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SpacesFinish>(),
            output: schema::<crate::SpacesFinished>(),
            plan: Some(schema::<crate::SpacesFinished>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Salona NET deseni".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e91"], "surface": "floor", "pattern": "NET", "scale": 2.0 }),
                    output: None,
                },
                CommandExample {
                    title: "60 × 60 asma tavan".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e91"], "surface": "ceiling", "grid": { "dx": 0.6, "dy": 0.6 } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_TEMPLATES_APPLY.into(),
            version: crate::CAD_TEMPLATES_APPLY_VERSION,
            title: "Mimari şablonu".into(),
            summary: "Mimari çalışma paketinin eksik katman, yazı/ölçü stili, blok, duvar stili, kat ve paftalarını ekler. \
                      settingsMode=keep birim ve çizim ölçeğini korur (GUI varsayılanı); settingsMode=template gösterilen paket ayarını (cm / 1:50, unit verilirse o gösterim birimi) uygular. Koordinat sistemli projede birim metre kalır; template kipi bu projede unit=m ister, başka birim unit_needs_local olur. Model koordinatlarını yeniden ölçeklemez. \
                      Açık kiplerde mevcut tanımlar, fontlar ve katman durumları korunur; planın review alanı önce/sonra birim-ölçek/mahal yöntemi ile kimlik/ad eşleşmesinde korunacak parçaları verir. Tüm paket ayarlar ve paftalar dahil tek undo adımıdır; hata kısmi yazmaz. keep ile unit birlikte template_settings_conflict olur. \
                      settingsMode alanı yoksa eski v1 davranışı sürer: boş çizime cm (unit yalnız mm/cm), nesneli çizimde birimi koruma uyarısı, her çizime 1:50; ayarlar ve pafta kitabı bu eski çağrıda undo dışında. Eski otomasyonu korumak içindir; yeni çağrılar açık kip kullanmalı. \
                      Paket: mimari-ofis standardı, KentOS blok kitaplığı ve duvar stilleri, iki kat, A-101/A-301 1:50 ve A-501 1:20 A3 paftalar. Çıktı eklenen adları sayar; plan yazmaz. expectedRevision değişmişse conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::TemplatesApply>(),
            output: schema::<crate::TemplatesApplied>(),
            plan: Some(schema::<crate::TemplatesApplied>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Mevcut birim ve ölçeği koruyarak mimari paketi ekle".into(),
                    input: json!({ "name": "mimari", "settingsMode": "keep" }),
                    output: None,
                },
                CommandExample {
                    title: "Eski v1 otomasyon çağrısı, milimetre".into(),
                    input: json!({ "name": "mimari", "unit": "mm" }),
                    output: None,
                },
            ],
        },
        // Veri profilleri (docs/adr/0541), held together by fixtures/commands/v1/cad.profiles.{apply,check}.json.
        CommandDescriptor {
            id: crate::CAD_PROFILES_APPLY.into(),
            version: crate::CAD_PROFILES_APPLY_VERSION,
            title: "Veri profilini uygula".into(),
            summary: "Bir veri profilinin (yerleşik tucbs-temel ya da profile ile verilen) çizimde olmayan parçalarını ekler; var olanı korur. \
                      Profil veridir: sınıfları katman yollarıyla (grupları ve katmanı adlarıyla, Türkçe büyük küçük harf ayrımsız bulunur), geometrileri, alanları, kimlik alanları ve kayıtlı alanlarıyla; projenin değer listeleri, sınıflar arası ilişkiler ve adres verisi. \
                      Eksik gruplar ve katmanlar alanlarıyla açılır, var olan katmana eksik alanları eklenir; türü farklı alan (profile_field_differs), farklı değer listesi (profile_domain_differs) korunur, kilitli katman ve yolunda katman olmayan sınıf atlanır (profile_layer_skipped). \
                      Katmanlar ve alanları tek geri alma adımıdır (“Veri profili”); değer listeleri, ilişkiler ve adres verisi proje ayarıdır. Profil resmî bir uygulama şeması değildir; temeli profilin basis'inde yazılıdır. Plan yazmadan aynısını söyler. \
                      Profil kurumun kendi adlarıyla da uygulanır (docs/adr/0558): name yerleşik olmayan bir ad verilirse profil çizimin “profiller” klasöründeki dosyasından (drawing çizimin dosyası) okunur; mapping (ya da projenin eşlemesinin adı, mappingName) verilirse profil o eşlemeden okunur: kurumun katman yolları, alan adları ve kodlarıyla bulunur ve eklenir, hiçbir şey kendiliğinden yeniden adlandırılmaz (unknown_mapping, invalid_profile_mapping). \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["veriprofili".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesApply>(),
            output: schema::<crate::ProfilesApplied>(),
            plan: Some(schema::<crate::ProfilesApplied>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "TUCBS temel belediye profilini uygula".into(),
                input: json!({ "name": "tucbs-temel" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_PROFILES_CHECK.into(),
            version: crate::CAD_PROFILES_CHECK_VERSION,
            title: "Veri profilini denetle".into(),
            summary: "Çizimin verisinin bir veri profilinden nerede ayrıldığını söyler; hiçbir şeyi değiştirmez, plan aynı cevabı verir. \
                      Bulgular sırasıyla: projede olmayan değer listeleri; sınıf sınıf eksik katman, eksik alan, türü farklı alan, başka geometrideki nesneler, boş zorunlu alanlar, alanın kuralına uymayan değerler (değer başına), birden çok nesnedeki kimlikler, \
                      kayıtlı alanı geometrik alanından profilin toleransından çok ayrılan nesneler (iki alan da söylenir; hiçbiri öbürünün yerine yazılmaz); sonra projede olmayan ilişkiler ve üst sınıfta olmayan nesneyi anan bağlar, en sonda adres verisi. \
                      Her bulgu türü, sınıfı, katmanı, konusu, değeri, nesnelerin kalıcı kimlikleri (en çok 1000) ve sayısı, Veri profilini uygula'nın düzeltip düzeltmediği ve Türkçe iletisiyle gelir; sınıfların nesne ve bulgu sayıları da. clean bulgu olmadığını söyler, mevzuata uygunluğu değil. \
                      Profil, Veri profilini uygula gibi, çizimin klasöründeki bir dosyadan (name, drawing) ve bir eşlemeden (mapping, mappingName) okunabilir: bulgular kurumun adlarıyla gelir, raporda mapping eşlemenin adını söyler (docs/adr/0558)."
                .into(),
            aliases: vec!["profildenetle".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesCheck>(),
            output: schema::<crate::ProfilesReport>(),
            plan: Some(schema::<crate::ProfilesReport>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimi TUCBS temel belediye profiliyle denetle".into(),
                input: json!({ "name": "tucbs-temel" }),
                output: None,
            }],
        },
        // A profile of an institution's own, its releases and mappings (docs/adr/0558), kept in the folder or inside the
        // drawing (docs/adr/0735), held together by fixtures/commands/v1/cad.profiles.{import,list,migrate,remove}.json.
        CommandDescriptor {
            id: crate::CAD_PROFILES_IMPORT.into(),
            version: crate::CAD_PROFILES_IMPORT_VERSION,
            title: "Veri profili içe aktar".into(),
            summary: "Kurumun kendi veri profilini ya da eşlemesini projeye alır: bir JSON dosyası (source: mutlak yol ya da çizimin klasöründen), profilin kendisi (profile) ya da eşlemenin kendisi (mapping), yalnız biri. \
                      Dosya biçimiyle tanınır: kentos.data-profile bir veri profili, kentos.profile-mapping bir eşlemedir; başka biçim unknown_profile_format, 1'den başka biçim sürümü unsupported_profile_version, okunmayan JSON ya da tutmayan profil invalid_profile, tutmayan eşleme invalid_profile_mapping ile reddedilir (eşleme profiliyle denetlenir: sınıflar, alanlar, kodlar profilde olmalı, kurumdaki adlar çakışmamalı). \
                      store: folder (varsayılan) ise profil çizimin dosyasının yanındaki “profiller” klasörüne <ad>.profil.json, eşleme <ad>.eslem.json olarak yazılır (ad küçük harf, rakam ve tire; klasör yoksa açılır): çizimin dosyası yoksa invalid_file_path. Dosya geçici adla yazılıp geri okunur, sonra yerine konur: bütün ya da hiç. \
                      store: drawing ise aynı metin çizimin kendi dosyasına, projenin ayarlarındaki profileFiles'a konur (en çok 256 profil ve eşleme, her biri en çok 4 MiB): çizim nereye giderse, bulut projesi de, onunla gider; çizimin dosyası gerekmez. Bu ayarın düzenlemesidir, geri alma adımı değildir. Aynı ad ve tür hem çizimde hem klasörde varsa komutlar çizimin içindekini okur (profile_shadowed). KentOS'un kendi profilinin adı alınamaz (profile_name_taken). \
                      Aynı adlı profil var ve daha yeni sürümdeyse profile_downgrade, aynı sürümdeyse klasörde file_exists, çizimde profile_exists; daha eski sürümdeyse değiştirilir ve söylenir (profile_replaced); overwrite her birinin üzerine yazar. Plan yazmadan aynısını söyler. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Profilin sürümü kendi release sayısıdır (biçim sürümü version değil) ve migrations'ı bir sürümden sonrakine veriyi taşıyan adımlardır; Veri profili göçü onları çalıştırır."
                .into(),
            aliases: vec!["profilal".into(), "profiliceaktar".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesImport>(),
            output: schema::<crate::ProfileImported>(),
            plan: Some(schema::<crate::ProfileImported>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kurumun profilini çizimin klasöründen projeye al".into(),
                    input: json!({
                        "source": "kurum-profili.json",
                        "drawing": "/projeler/belediye/imar.kcad"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Kurumun profilini çizimin kendi dosyasına al".into(),
                    input: json!({
                        "source": "/kurum/profiller/kurum-profili.json",
                        "store": "drawing"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_PROFILES_LIST.into(),
            version: crate::CAD_PROFILES_LIST_VERSION,
            title: "Veri profillerini listele".into(),
            summary: "KentOS'un yerleşik veri profillerini, çizimin kendi dosyasının taşıdıklarını (profileFiles) ve çizimin “profiller” klasöründeki profil ve eşleme dosyalarını listeler; hiçbir şeyi değiştirmez. \
                      Önce yerleşik profiller kendi sırasıyla, sonra çizimin içindekiler, sonra klasörün dosyaları, her biri adıyla (aynı adda önce profil). Her satır: türü (profil ya da eşleme), kaynağı (builtin, drawing ya da project: klasör), komutlara verilen adı, başlığı, profilin sürümü (release) ve sınıf ve göç sayısı; eşlemede hangi profil için olduğu ve anan sınıf sayısı; klasör dosyasında yolu. \
                      Okunmayan ya da tutmayan dosya listeden düşmez, problem alanında nedeniyle gelir (profilin adı dosyanın adıyla aynı olmalı; eşleme profiliyle denetlenir, profili bulunamayan eşleme denetlenmeden listelenir). Aynı ad ve tür çizimde de varsa klasörün dosyası shadowed gelir: komutlar çizimin içindekini okur. Çizimin dosyası yoksa (drawing) klasör okunmaz; folder çizimin klasörünü söyler."
                .into(),
            aliases: vec!["profiller".into(), "profillistesi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesList>(),
            output: schema::<crate::ProfilesListed>(),
            plan: Some(schema::<crate::ProfilesListed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Yerleşik ve projenin profillerini listele".into(),
                input: json!({ "drawing": "/projeler/belediye/imar.kcad" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_PROFILES_MIGRATE.into(),
            version: crate::CAD_PROFILES_MIGRATE_VERSION,
            title: "Veri profili göçü".into(),
            summary: "Projenin verisini bir profilin from sürümünden to sürümüne (verilmezse profilin kendi sürümüne) profilin kendi göç adımlarıyla taşır. Profil profile ile ya da adıyla (yerleşik ya da projenin dosyası) verilir. \
                      Göç bir sürümden sonrakine gider ve profilin migrations listesinde yazılıdır; from'dan to'ya kesintisiz bir yol yoksa migration_gap, to from'dan büyük değilse ya da profilin sürümünü aşıyorsa invalid_migration. Adımlar sırasıyla, her biri çizimin o anki hâline göre: \
                      renameLayer (katman yolundaki son ad değişir; aynı yerde o ad varsa layer_refused), renameField (alan adı ve nesnelerin değerleri taşınır; formların, ilişkilerin ve ifadelerin anması izler), changeFieldType (alanın yeni tanımı; her değer yeni kuraldan geçip kanonik metniyle yazılır; uymayan onFail'e göre olduğu gibi kalır ya da boşaltılır ve değer değer raporlanır), mapValues (eski değerler yenisi olur; birkaçı birleşebilir, bir eski değer başka alanın değerine göre ikiye ayrılabilir; çok değerli alanda her kod), retireCode (projenin değer listesinde kod kullanımdan kalkar; replaceWith ile ona bağlı alanların o kodlu değerleri başka kod olur). \
                      Katmanı, alanı ya da değer listesi çizimde olmayan adım atlanır ve söylenir (migration_step_skipped); zaten uygulanmış adım sessiz atlanır, kilitli katman atlanır (profile_layer_skipped). Hepsi tek geri alma adımıdır (“Profil göçü”, değer listeleri dahil); yalnız katman adları adımın dışındadır: katmanın adı çizimin bayrağıdır, hiçbir katman yeniden adlandırması geri alınmaz (docs/adr/0020), geri alma alanları, değerleri ve listeleri geri verir. layer_refused'ta hiçbir şey yazılmaz. Göç yeni sürümün getirdiklerini eklemez: Veri profilini uygula ekler, Veri profilini denetle verinin hâlâ ayrıldığı yeri söyler; önce göç, sonra uygula. \
                      Cevapta adım adım ne yapıldığı, kaç nesneye yazıldığı ve uymayan değerler (katman, alan, değer, nesnelerin kalıcı kimlikleri en çok 1000, sayı) vardır. Plan yazmadan aynısını söyler. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["profilgoc".into(), "profilgocu".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProfilesMigrate>(),
            output: schema::<crate::ProfilesMigrated>(),
            plan: Some(schema::<crate::ProfilesMigrated>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kurumun profilini 1. sürümden 2. sürüme taşı".into(),
                input: json!({ "name": "kurum-profili", "from": 1, "to": 2 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_PROFILES_REMOVE.into(),
            version: crate::CAD_PROFILES_REMOVE_VERSION,
            title: "Veri profilini çizimden kaldır".into(),
            summary: "Çizimin kendi dosyasının taşıdığı (profileFiles, Veri profili içe aktar'ın store: drawing'i) bir veri profilini ya da eşlemeyi çizimden çıkarır: kind (profile ya da mapping) ve name ile. \
                      “profiller” klasöründeki dosyaya dokunmaz: klasörde aynı ad ve türde dosya varsa komutlar bundan sonra onu okur, cevabın folderPath'i onu söyler (drawing çizimin dosyası, klasör onun yanındadır). \
                      Çizimin taşımadığı profil unknown_profile, eşleme unknown_mapping ile reddedilir (ileti klasördeki aynı adlı dosyayı da söyler). Projenin bir eşlemesi kaldırılan profili adlandırıyor ve profilin başka kopyası kalmıyorsa söylenir (profile_still_named). \
                      Projenin ayarının düzenlemesidir, geri alma adımı değildir; plan yazmadan aynısını söyler. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["profilkaldir".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesRemove>(),
            output: schema::<crate::ProfileRemoved>(),
            plan: Some(schema::<crate::ProfileRemoved>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimin taşıdığı kurum profilini çizimden kaldır".into(),
                input: json!({ "kind": "profile", "name": "kurum-profili" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_SHEETS_SET.into(),
            version: crate::CAD_SHEETS_SET_VERSION,
            title: "Pafta seti".into(),
            summary: "Projenin pafta kitabını pafta seti olarak listeler; çizimi ve kitabı değiştirmez. details=true tam kitap, sistem şablonları, standardRatios ve güncel namedViews listesini verir; pictures=true gömülü resim baytlarını ekler. Canlı adlı görünümün çerçeve ölçeği ve köşeleri güncel kaynaktan hesaplanır; book içindeki capture değişmez. Paftalar kitabın sırasıyla, adlarının numarasıyla serilere ayrılır (A-101 → A-1 Planlar; A-2 Görünüşler, A-3 Kesitler, A-5 Detaylar; numarasız Diğer, en sonda). Her harita çerçevesi: kimliği, adı, ölçeği, gösterdiği zemin (dört köşesi doğu ve kuzey, metre; kutusu) ve ona bakan görünüm işaretleri; atlas gibi sabit görünümü olmayan çerçevenin zemini yoktur. \
                      uids verilince etkilenen çerçeveler: nesnelerden birinin kutusu çerçevenin zemin kutusuyla kesişen (shows), nesnelerden bir görünüm işaretinin hedefi olan (marked) ve zemini bilinmeyen (unknown) çerçeveler, kitabın sırasıyla; paftaları yeniden basılacaklardır. Kutu kesişimidir: fazladan pafta listelenebilir, eksik listelenmez."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SheetsSet>(),
            output: schema::<crate::SheetsSetResult>(),
            plan: Some(schema::<crate::SheetsSetResult>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Pafta seti".into(),
                    input: json!({}),
                    output: None,
                },
                CommandExample {
                    title: "Bir duvarın etkilediği paftalar".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e81"] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_NAVIGATOR_READ.into(),
            version: crate::CAD_NAVIGATOR_READ_VERSION,
            title: "Proje gezgini".into(),
            summary: "Projenin katlarını, adlı görünümlerini, paftalarını ve çizelgelerini aralarındaki bağlarla listeler; çizimi ve kitabı değiştirmez. Her kat: kimliği, adı, kotu, üzerindeki nesnelerin sayısı (açıklık duvarının, etiket nesnesinin katında), tarifi o katı çizen görünümler ve o katı çizen pafta haritaları. Her görünüm: tarifinin katı, sürümü ve ölçeği, ona bağlı haritalar. Her pafta: serisi ve haritaları (bağlı görünüm, kayıp görünüm, kat, ölçek; cad.sheets.set'inkiler). Her çizelge: tablonun kimliği, neyi listelediği, başlığı ve listelediği nesne sayısı. unleveled katsız nesnelerin sayısıdır. \
                      uid verilince nesnenin nerede göründüğü: katı, onu çizen görünümler (alanının kutusu nesnenin kutusuyla kesişen, tarifi katını, evresini ve türünü dışarıda bırakmayan), onu çizen haritalar (cad.sheets.set'in shows'u), onu listeleyen çizelgeler ve onu adlandıran etiketler. Kutu kesişimidir: fazladan listelenebilir, eksik listelenmez."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::NavigatorRead>(),
            output: schema::<crate::NavigatorReadResult>(),
            plan: Some(schema::<crate::NavigatorReadResult>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Proje gezgini".into(),
                    input: json!({}),
                    output: None,
                },
                CommandExample {
                    title: "Bir kapının göründüğü yerler".into(),
                    input: json!({ "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e81" }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_XREFS_EDIT.into(),
            version: crate::CAD_XREFS_EDIT_VERSION,
            title: "Dış başvuru".into(),
            summary: "Başka bir çizim dosyasını dış başvuru olarak bağlar ve yönetir; her işlem tek geri alma adımı. operation.kind: attach (path mutlak .kcad ya da .pdf yolu; page PDF'in sayfası, 1'den, yoksa 1; mode attach ya da overlay; name yoksa dosyanın adı; at, scale, rotation derece; layerId yerleştirmenin katmanı): kaynağın nesneleri (resim, raster ve tablo dışında) bir blok tanımı olur, katmanları “Dış başvuru: ad” grubunun altına gelir, bir yerleştirme konur; reload kaynağı yeniden okur (yerleştirmeler aynı, kaynağın nesnelerine bağlı ölçüler ve kılavuz uçları aynı nesneleri izler, silineninki kopar); unload içeriği boşaltır; repath yeni yoldaki dosyayı bağlar ve yükler; detach tanımı, getirdiklerini, bütün yerleştirmelerini ve katman grubunu siler; bind (mode bind: katmanlar grupta kalır; insert: aynı yoldaki katmanlarla birleşir) sıradan bloğa çevirir; paths projenin arama yollarını yazar (ayar; kaynak yolunda ve göreli yolunda yoksa bu klasörlerde aranır); underlay PDF altlığının görünüşünü yazar (fade %0–80 soldurma, monochrome tek renk, plot false yazdırılmaz). \
                      PDF altlığı: sayfanın çizgileri, dolguları ve yazıları kâğıt metresinde (1 pt = 0,0254/72 m) sol alt köşesinden bir çizim olarak okunur, katmanları PDF'in isteğe bağlı içerik grupları; resimler ve gölgelemeler alınmaz (pdf_left_out); kenet ve ölçü onun çizgilerine olur ama sonuç kâğıttan okunmuştur, kanıtlanmış CAD geometrisi değildir (underlay_measure). \
                      Kaynağın attach dış başvuruları iç içe gelir, overlay olanlar gelmez; döngü xref_cycle. drawing açık çizimin dosya yolu: göreli yol ve döngü için. İç içe gelen dış başvuru kendi başına değiştirilemez (nested_xref). Plan yazmadan ne olacağını söyler."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::XrefsEdit>(),
            output: schema::<crate::XrefsEdited>(),
            plan: Some(schema::<crate::XrefsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Mimari planı dış başvuru olarak ekle".into(),
                    input: json!({ "operation": { "kind": "attach", "path": "/proje/mimari/plan.kcad", "layerId": "0" } }),
                    output: None,
                },
                CommandExample {
                    title: "Yeniden yükle".into(),
                    input: json!({ "operation": { "kind": "reload", "name": "plan" } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_XREFS_LIST.into(),
            version: crate::CAD_XREFS_LIST_VERSION,
            title: "Dış başvurular".into(),
            summary: "Çizimin dış başvurularını blok sırasıyla listeler; çizimi değiştirmez. Her biri: adı, blok kimliği, türü, bulunduğu yol (yolunda yoksa drawing'in klasöründen göreli yolu), durumu (loaded, unloaded, missing: dosya yok, changed: dosyanın özeti yüklenenden başka, Yeniden yükle alır), onu getiren, yerleştirme ve nesne sayısı, katman grubu, nasıl bulunduğu (path, relative, searchPath, none) ve bulunamadıysa denenen yollar; affected: true her birinin yerleştirmelerini gösteren paftaları ve adlı görünümleri de verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::XrefsList>(),
            output: schema::<crate::XrefsListed>(),
            plan: Some(schema::<crate::XrefsListed>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dış başvurular ve durumları".into(),
                    input: json!({ "drawing": "/proje/ofis.kcad" }),
                    output: None,
                },
                CommandExample {
                    title: "Etkilenen paftalar ve görünümlerle".into(),
                    input: json!({ "drawing": "/proje/ofis.kcad", "affected": true }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_RASTERS_GEOREF.into(),
            version: crate::CAD_RASTERS_GEOREF_VERSION,
            title: "Raster oturt".into(),
            summary: "Rasteri kontrol noktalarıyla çizime oturtur ve nasıl oturtulduğunu rasterin kaydına yazar (tek geri alma adımı “Raster oturt”). points: kaynağın pikseli (sütun, satır; piksel köşeleri, sol üst köşe 0, 0) ve hedefi (srid'in sisteminde; verilmezse projenin; başka sistem önce projeninkine çevrilir); check: dönüşüme girmeyen denetim noktası, hatası doğruluğu ölçer; off: kullanılmayan. method: helmert ve affine rasterin dönüşümünü değiştirir; projective, poly2, poly3 ve thinPlate rasteri kuzeye bakan ızgaraya yeniden örnekler (pixel verilmezse dönüşümün kontrol noktalarındaki ortalama ölçeği; resampling bilinear ya da nearest): bağlı kaynağın çıktısı output'a ya da kaynağın yanına “<ad>-oturtulmus.tif” olarak yazılır, gömülü kaynağınki kitaplığa gömülür; kaynak dosya değişmez. Yeniden örneklenmiş rasterin noktaları kaydındaki kaynağın pikselleridir; yeniden oturtma kaynaktan yapılır. made: ev sahibinin planın ızgarasıyla kendi yazdığı çıktı (dosya ya da kitaplıktaki raster); ızgarası planınki değilse ret. Kayıt: yöntem, noktalar, hedeflerin sistemi, kaynak ve SHA-256 özeti, m0, denetim noktalarının karesel ortalama ve en büyük hatası, çıktı ızgarası ve örnekleme. Denetim noktası yoksa no_check_points uyarısı. Plan yazmadan artıkları, hataları ve rasterin yeni hâlini verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::RastersGeoref>(),
            output: schema::<crate::RastersGeoreffed>(),
            plan: Some(schema::<crate::RastersGeoreffed>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Taranmış paftayı dört kontrol ve iki denetim noktasıyla afin oturt".into(),
                    input: json!({
                        "uid": "018f6c1e-0000-7000-8000-000000000001",
                        "method": "affine",
                        "points": [
                            { "name": "K1", "pixel": [80.5, 60.5], "target": [487240.25, 4420771.4] },
                            { "name": "K2", "pixel": [1940.5, 60.5], "target": [488170.25, 4420808.6] },
                            { "name": "K3", "pixel": [1940.5, 1440.5], "target": [488211.65, 4420118.6] },
                            { "name": "K4", "pixel": [80.5, 1440.5], "target": [487281.65, 4420081.4] },
                            { "name": "D1", "pixel": [1000.5, 750.5], "target": [487722.75, 4420445.0], "check": true },
                            { "name": "D2", "pixel": [400.5, 1100.5], "target": [487433.25, 4420258.0], "check": true }
                        ]
                    }),
                    output: None,
                },
                CommandExample {
                    title: "İnce plakayla yeniden örnekle, 0,5 m piksel, en yakın örnekleme".into(),
                    input: json!({
                        "uid": "018f6c1e-0000-7000-8000-000000000001",
                        "method": "thinPlate",
                        "points": [
                            { "pixel": [0.0, 0.0], "target": [487100.0, 4420300.0] },
                            { "pixel": [400.0, 0.0], "target": [487300.0, 4420316.0] },
                            { "pixel": [400.0, 300.0], "target": [487315.0, 4420166.0] },
                            { "pixel": [0.0, 300.0], "target": [487115.0, 4420150.0] },
                            { "pixel": [200.0, 150.0], "target": [487208.0, 4420233.0], "check": true }
                        ],
                        "pixel": 0.5,
                        "resampling": "nearest",
                        "drawing": "/proje/pafta.kcad"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_RASTERS_MOSAIC.into(),
            version: crate::CAD_RASTERS_MOSAIC_VERSION,
            title: "Sanal mozaik".into(),
            summary: "Birçok GeoTIFF dosyasını tek raster gibi gösterilen ve çözümlenen sanal mozaik yapar: path'e GDAL'ın VRT dosyası yazılır (piksel kopyalanmaz; GDAL ve QGIS de açar). \
                      sources kuzeyi yukarıda, GeoTIFF etiketleriyle yerinde, aynı koordinat sisteminde, bant sayısında ve veri türünde dosyalardır (mutlak ya da drawing'in klasöründen göreli). \
                      Örtüşen yerde üstteki: order given (sources'un sırası, ilki üstte), finest (küçük piksel üstte) ya da newest (TIFF DateTime'ı yeni olan üstte; tarihsizler en altta, mosaic_undated). \
                      Bir dosyanın nodata'sı alttakini gösterir; hiçbir dosyanın örtmediği yer mozaiğin nodata'sıdır (nodata, yoksa sırayla ilk nodata'lı dosyanınki). \
                      Mozaiğin pikseli resolution highest (en ince), lowest (en kaba) ya da average (ortalama), ya da pixelSize; kapsamı dosyaların birleşimi (gdalbuildvrt'nin kuralı). Pikseli başka olan dosya en yakın pikselden okunur (mosaic_resampled). \
                      Dosya yarım kalmaz: geçici adla yazılır, geri okunup denetlenir, sonra yerine konur; var olan path overwrite verilmezse reddedilir (file_exists). \
                      layer verilirse mozaik o katmana path'e bağlı raster olarak eklenir (tek geri alma adımı “Sanal mozaik”; sistemi projeninkinden başkaysa crs_mismatch); verilmezse çizim değişmez. plan yazmadan mozaiği söyler."
                .into(),
            aliases: vec!["sanalmozaik".into(), "vrt".into(), "buildvrt".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::RastersMosaic>(),
            output: schema::<crate::RastersMosaicked>(),
            plan: Some(schema::<crate::RastersMosaicked>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Üç ortofotoyu yeni tarihli üstte mozaik yap ve katmana ekle".into(),
                    input: json!({
                        "sources": ["orto/2019.tif", "orto/2021.tif", "orto/2023.tif"],
                        "path": "orto/mozaik.vrt",
                        "order": "newest",
                        "layer": "altlik",
                        "drawing": "/proje/imar.kcad"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Yükseklik paftalarını 5 m pikselle birleştir".into(),
                    input: json!({
                        "sources": ["/veri/dem/a.tif", "/veri/dem/b.tif"],
                        "path": "/veri/dem/dem.vrt",
                        "pixelSize": 5.0,
                        "nodata": -9999.0
                    }),
                    output: None,
                },
            ],
        },
        // Rasteri yeniden izdüşür (docs/adr/0644 §3), held together by fixtures/commands/v1/cad.rasters.reproject.json.
        CommandDescriptor {
            id: crate::CAD_RASTERS_REPROJECT.into(),
            version: crate::CAD_RASTERS_REPROJECT_VERSION,
            title: "Rasteri yeniden izdüşür".into(),
            summary: "Başka koordinat sistemindeki bir raster dosyasını (file; GeoTIFF ya da dünya dosyalı PNG ve JPEG, mutlak ya da drawing'in klasöründen göreli) projenin sistemine çevirir: pikselleri projenin sisteminde kuzeyi yukarıda bir ızgaraya yeniden örneklenir, output'a GeoTIFF yazılır (yoksa dosyanın yanına <ad>-epsg<kod>.tif) ve raster o dosyaya bağlı olarak katmana eklenir (layer; yoksa dosyanın adıyla yeni katman en üstte), tek geri alma adımı “Rasteri yeniden izdüşür”. \
                      Dosya sistem bildirmiyorsa srid verilir (crs_unknown); bildirdiğinden başka srid uyarıyla onun yerine sayılır (crs_overridden). Dönüşüm projenin datum seçimleriyle, Koordinat dönüştür'ün yoluyla yapılır; yolu ve doğruluğu cevabın way'indedir. \
                      Izgara kaynağın kenarlarının (her kenarda 20 adım) projedeki görüntüsünü örter, köşesi piksel boyunun (cell; yoksa kaynağın orta pikselinin projede kapladığı alanın karekökü, altı anlamlı basamak) katındadır. sample nearest kaynağın merkezin altındaki pikselini alır (sınıf ve kalite bantları kodlarını korur), bilinear çevresindeki dördün ağırlıklı ortalamasını, değersiz pikselleri dışarıda bırakarak. Değerler korunur: 8 bitlik raster RGBA'ya çevrilmez. \
                      Dosya yarım kalmaz: geçici adla yazılır, sonra yerine konur; var olan output overwrite verilmezse reddedilir (file_exists). Kaynağın kendisinin üzerine yazılmaz. plan yazmadan ızgarayı söyler."
                .into(),
            aliases: vec!["yenidenizdusur".into(), "rasterizdusur".into(), "gdalwarp".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::RastersReproject>(),
            output: schema::<crate::RastersReprojected>(),
            plan: Some(schema::<crate::RastersReprojected>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "UTM 36'daki uydu bandını TM36 projesine 10 m pikselle al".into(),
                    input: json!({
                        "file": "veri/B04.tif",
                        "cell": 10.0,
                        "drawing": "/proje/imar.kcad"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Sistemsiz eski paftayı ED50 UTM 36 diye çift doğrusal çevir".into(),
                    input: json!({
                        "file": "/veri/pafta.tif",
                        "srid": 23036,
                        "sample": "bilinear",
                        "output": "/veri/pafta-tm36.tif",
                        "layer": "altlik"
                    }),
                    output: None,
                },
            ],
        },
        // GeoPackage tile pyramids as rasters (docs/adr/0556), held together by
        // fixtures/commands/v1/cad.rasters.gpkgread.json and cad.rasters.gpkgwrite.json.
        CommandDescriptor {
            id: crate::CAD_RASTERS_GPKGREAD.into(),
            version: crate::CAD_RASTERS_GPKGREAD_VERSION,
            title: "GeoPackage karolarını oku".into(),
            summary: "Bir GeoPackage'ın karo (raster) tablosunu okur: tables dosyadaki her karo tablosunu (karo matrisi kümesinin kutusu, veri kutusu, EPSG kodu, her zoom düzeyinin matrisi, karo boyu, pikseli, karo sayısı ve karolarının biçimi) söyler. \
                      output verilirse seçilen zoom düzeyi (zoom; yoksa en yükseği) ve alan (area, tablonun sisteminde [en küçük x, en küçük y, en büyük x, en büyük y]; yoksa tablonun veri kutusu, o da yoksa karo matrisi kümesinin kutusu, düzeyin ızgarasına kesilir) tek raster olarak output'a kuzeyi yukarıda, tablonun sisteminde, dört bantlı (kırmızı, yeşil, mavi, alfa) 8 bitlik GeoTIFF yazılır; dosyada karosu olmayan yer saydamdır (alfa 0, tiles_missing). \
                      Pencerenin sınırı GDAL'ın GeoPackage sürücüsünün kuralıyla pikselin tamına yuvarlanır (başlangıç (min − küme başlangıcı) / piksel'in yuvarlağı, boy (en büyük − en küçük) / piksel + 0,5'in tamı) ve ızgaradan taşan yer kesilir. Karolar PNG olmalı ve matrisin karo boyunda olmalı (8 bit gri, gri+alfa, RGB, RGBA ve paletli); JPEG, WebP ve 8 bitten fazla PNG okunmaz (tile_format_unsupported); raster en çok 25 milyon piksel olur (gpkg_tiles_too_large). \
                      Tablo EPSG sistemini bildirmiyorsa srid 0 olur (crs_undeclared). layer verilirse raster output'a bağlı olarak o katmana eklenir (tek geri alma adımı “GeoPackage karoları”; sistemi projeninkinden başkaysa crs_mismatch); verilmezse çizim değişmez. Dosya yarım kalmaz: geçici adla yazılır, geri okunup denetlenir, sonra yerine konur; var olan output overwrite verilmezse reddedilir (file_exists). plan yazmadan söyler."
                .into(),
            aliases: vec!["gpkgkaro".into(), "gpkgraster".into(), "karooku".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::RastersGpkgRead>(),
            output: schema::<crate::RastersGpkgReadDone>(),
            plan: Some(schema::<crate::RastersGpkgReadDone>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dosyadaki karo tablolarını ve zoom düzeylerini listele".into(),
                    input: json!({ "path": "/veri/ortofoto.gpkg" }),
                    output: None,
                },
                CommandExample {
                    title: "En ince düzeyin bir alanını GeoTIFF yap ve altlık katmanına ekle".into(),
                    input: json!({
                        "path": "/veri/ortofoto.gpkg",
                        "table": "orto",
                        "area": [487000.0, 4420000.0, 487500.0, 4420400.0],
                        "output": "/veri/orto-parca.tif",
                        "layer": "altlik"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_RASTERS_GPKGWRITE.into(),
            version: crate::CAD_RASTERS_GPKGWRITE_VERSION,
            title: "Rasteri GeoPackage karosu olarak yaz".into(),
            summary: "Çizimdeki bir rasteri (uid; 8 bitlik, kuzeyi yukarıda, paletsiz, dosyası ya da kitaplık kaynağı okunabilen) path'e GeoPackage karo tablosu (table; yoksa tiles) olarak yazar: 256 × 256 PNG karolar, en ince zoom düzeyi rasterin kendi pikseli, her kaba düzey yarı piksel çözünürlüklü (2 × 2 ortalaması; ızgaranın dışı saydam); karo ızgarasının başlangıcı rasterin sol üst köşesidir ve karo matrisi kümesinin kutusu en kaba düzeyin tek karosunu kaplar, veri kutusu rasterin kendi kutusudur. \
                      Karolar rasterin bantlarındandır: 1 bant gri, 2 bant gri ve alfa, 3 ve 4 bant RGBA (alfa rasterde yoksa içeride 255, ızgara dışında 0). Rasterin sistemi (EPSG kodu ve WKT'si; sistemsizse tanımsız kartezyen) dosyaya yazılır. nodata değeri karoda taşınamaz (nodata_lost). Raster 25 milyon pikseli aşarsa gpkg_tiles_too_large. \
                      mode new: yeni GeoPackage (1.3; var olan dosyanın üzerine yalnız overwrite ile yazılır, geçici adla yazılır, geri okunup yerine konur); add: var olan GeoPackage'a tek işlemde tablo eklenir (aynı adlı tablo table_exists, hata olursa dosya olduğu gibi kalır). plan yazmadan yazılacak olanı söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["gpkgkarouret".into(), "karoyaz".into(), "rastergpkg".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::RastersGpkgWrite>(),
            output: schema::<crate::RastersGpkgWritten>(),
            plan: Some(schema::<crate::RastersGpkgWritten>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ortofotoyu karo tablosu olarak yeni bir GeoPackage'a yaz".into(),
                input: json!({
                    "uid": "6f1c0a52-3b7e-4d0a-9a55-0d4f6b1f2c11",
                    "path": "/veri/ortofoto.gpkg",
                    "table": "orto"
                }),
                output: None,
            }],
        },
        // A temporal layer's state as valid and as known (docs/adr/0537), held together by
        // fixtures/commands/v1/cad.time.query.json. The web is paused.
        CommandDescriptor {
            id: crate::CAD_TIME_QUERY.into(),
            version: crate::CAD_TIME_QUERY_VERSION,
            title: "Zamanda sorgu".into(),
            summary: "Zamansal bir katmanın (layer) hangi nesnelerinin bir geçerlilik zamanında ve bilinen bir günde gösterildiğini söyler; çizimi değiştirmez. \
                      Geçerlilik zamanı bir an (at) ya da [from, to) aralığıdır, katmanın zaman ayarının kuralıyla (aralıklı, anlık, birikimli); zamansız nesne her zaman gösterilir. \
                      Bilinen gün (knownAt) yalnız o güne dek kaydedilmiş ve henüz yerine başkası geçmemiş sürümleri bırakır: katmanın recordedStart ve recordedEnd alanlarıyla (.kcad şema 90); kayıt zamanı tutmayan katmanda not_recorded. \
                      “O gün geçerli olan” ile “o gün sistemde bilinen” ayrı sorulur. An bir tarih ya da tarih ve saattir, bölgesiyle (Z, +03:00); bölgesiz saat UTC'dir; cevap anları UTC yazar. \
                      Cevap gösterilenlerin kalıcı kimliklerini çizimin sırasıyla ve öbürlerinin nedenlerini sayar: notValid (geçerlilik zamanı göstermiyor), notKnown (geçerli ama o gün bilinmiyor), timeless, unrecorded, unreadable. \
                      Soru çizimin değişmez bir durumuna bağlanır: revision geri alma geçmişinde n adım önceki çizimi (history) ya da bir çizim dosyasını (file) sorar; cevap durumu (state) ve katmanın o durumdaki içerik özetini (stamp: nesnelerin türleri, geometrileri ve öznitelikleri kalıcı kimlikleriyle) verir. Özeti expectedStamp olarak veren soru katman o durumda başkaysa state_changed çakışmasıyla döner: aynı özetle iki cevap aynı veridendir."
                .into(),
            aliases: vec!["zamandasor".into(), "bilinendurum".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::TimeQuery>(),
            output: schema::<crate::TimeQueried>(),
            plan: Some(schema::<crate::TimeQueried>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "1 Haziran 2024'te geçerli parseller, 1 Ocak 2025'te bilindiği gibi".into(),
                    input: json!({ "layer": "parsel", "at": "2024-06-01", "knownAt": "2025-01-01" }),
                    output: None,
                },
                CommandExample {
                    title: "2023 içinde geçerli olanlar".into(),
                    input: json!({ "layer": "parsel", "from": "2023-01-01", "to": "2024-01-01" }),
                    output: None,
                },
                CommandExample {
                    title: "Bir dosyada saklanan durumda, özetiyle sabitlenmiş".into(),
                    input: json!({
                        "layer": "parsel",
                        "at": "2024-06-01",
                        "revision": { "kind": "file", "path": "/arsiv/parsel-2023.kcad" },
                        "expectedStamp": "9a3f6d0c2b8e4f7a1c5d9e2b6f0a3c7d1e4b8f2a5c9d3e6f0b4a7c1d5e8f2b6a"
                    }),
                    output: None,
                },
            ],
        },
        // Sensor observations (docs/adr/0646 §3–§5), held together by
        // fixtures/commands/v1/cad.sensors.{ingest,latest}.json. The web is paused.
        CommandDescriptor {
            id: crate::CAD_SENSORS_INGEST.into(),
            version: crate::CAD_SENSORS_INGEST_VERSION,
            title: "Sensör gözlemlerini al".into(),
            summary: "Bir sensör katmanına (layer: zaman ayarı kimlik alanını, key, adlandıran zamansal katman) gözlemleri tek geri alma adımında (“Sensör gözlemleri”) geometrisiz kayıt olarak yazar. \
                      Kaynak tam olarak biri: observations (asset, at, value, quality satırları), file (SensorThings Observations cevabının mutlak yolu) ya da url (SensorThings Observations adresi; @iot.nextLink en çok pages sayfa izlenir, kalan sayfa more ile söylenir); file ve url'nin gözlemleri tek varlığındır (asset). \
                      Varlık katmanın key alanına, olay zamanı (SensorThings'te phenomenonTime, aralıkta başlangıcı) start alanına UTC olarak, alım zamanı (receivedAt; yoksa şimdi) recordedStart'a, değer valueField'a, kalite qualityField'a yazılır; değerler katmanın alanlarının kuralından geçer. \
                      Yinelenen gözlem (aynı varlığın aynı andaki, katmanın şimdiki sürümlerinde ya da partide) duplicates ile: first eskisi kalır, last yenisi geçer (recordedEnd tutan katmanda eski sürümün kayıt bitişi alım zamanı olur, tutmayanda eskinin değeri yerinde değişir), all hepsi yazılır. \
                      Geç gelen gözlem (varlığın en son anından önceki) late ile: accept yazılır ve sayılır (lateField'a evet), reject yazılmaz. \
                      Yazılamayan satır (boş varlık, okunmayan an, alanın kuralına uymayan değer, reddedilen geç gözlem) rejected'da sırası, kodu ve nedeniyle söylenir; öbürleri yazılır."
                .into(),
            aliases: vec!["sensoral".into(), "gozlemal".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::SensorsIngest>(),
            output: schema::<crate::SensorsIngested>(),
            plan: Some(schema::<crate::SensorsIngestPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki gözlem, aynı andaki gözlemin yenisi geçer".into(),
                    input: json!({
                        "layer": "olcum",
                        "valueField": "deger",
                        "qualityField": "kalite",
                        "duplicates": "last",
                        "observations": [
                            { "asset": "S1", "at": "2024-06-01T10:20:00Z", "value": 13.5, "quality": "iyi" },
                            { "asset": "S2", "at": "2024-06-01T10:20:00Z", "value": 7.25 }
                        ]
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bir SensorThings veri akışının gözlemleri, geç gelenler reddedilerek".into(),
                    input: json!({
                        "layer": "olcum",
                        "valueField": "deger",
                        "url": "https://sensors.example.org/v1.1/Datastreams(8)/Observations",
                        "asset": "S8",
                        "late": "reject"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SENSORS_LATEST.into(),
            version: crate::CAD_SENSORS_LATEST_VERSION,
            title: "Sensörlerin son gözlemi".into(),
            summary: "Bir sensör katmanının (layer) her varlığının bir andaki (at; yoksa şimdi) son gözlemini, değerini (valueField), kalitesini (qualityField) ve yaşını verir; çizimi değiştirmez. \
                      Son gözlem, anı sorulan andan geç olmayan en geç gözlemdir; eşitlikte alımı geç olan, sonra çizimde sonra gelen. Bilinen gün (knownAt) yalnız o gün bilinen sürümleri alır; verilmezse yerine başkası geçmemiş sürümler. \
                      Yaşı staleAfter saniyeden büyük gözlem stale'dir: akışı kesilen varlığın son değeri güncel gibi gösterilmez. expectedAssets verilen varlıklardan gözlemi olmayanlar missing'de söylenir. Varlıklar doğal sırayla."
                .into(),
            aliases: vec!["sonolcum".into(), "sensordurumu".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SensorsLatest>(),
            output: schema::<crate::SensorsLatestAnswer>(),
            plan: Some(schema::<crate::SensorsLatestAnswer>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Öğlen her sensörün son değeri, bir saatten eskisi eskimiş".into(),
                input: json!({
                    "layer": "olcum",
                    "valueField": "deger",
                    "at": "2024-06-01T12:00:00Z",
                    "staleAfter": 3600,
                    "expectedAssets": ["S1", "S2", "S3"]
                }),
                output: None,
            }],
        },
        // A point cloud's source metadata and its points by a lasting id (docs/adr/0536), held together by
        // fixtures/commands/v1/cad.pointclouds.{describe,query}.json. The web is paused.
        CommandDescriptor {
            id: crate::CAD_POINTCLOUDS_DESCRIBE.into(),
            version: crate::CAD_POINTCLOUDS_DESCRIBE_VERSION,
            title: "Nokta bulutunun bilgisi".into(),
            summary: "Çizimin nokta bulutlarının (uids; yoksa hepsi, çizimin sırasıyla) her kaynağının başlığını okur ve söyler; çizimi değiştirmez. \
                      Kaynak: türü (las, laz, copc, text) ve sürümü, nokta biçimi ve kaydın boyu, nokta sayısı, kutusu, ölçek ve öteleme; yatay sistem (EPSG ve dosyanın yazdığı ad) ve düşey sistem (WKT'nin VERT_CS ya da VERTCRS adı, GeoTIFF anahtarı 4096'nın EPSG:n'si; yazmayan dosyada yok: kotların dayanağı bilinmez); başlığın oluşturulma günü (YYYY-MM-DD), noktaların GPS zamanı (standard ya da week), sistem ve yazılım, kaynak kimliği, GUID; her noktanın boyutları LAS'ın sırasıyla ve ek baytların adları ve türleri, dönüşe göre nokta sayıları, COPC'nin düğümleri. \
                      classes ile her noktanın sınıfı sayılır (kaynak bütün okunur; bir çağrıda en çok 200 milyon nokta, fazlası too_many_cloud_points). Metin bulutunun başlığı yoktur: sayısı ve kutusu bulutun kaydından. \
                      Okunamayan kaynak (bulunamadı, adresteki bulut, bozuk dosya) why ile söylenir ve uyarılır (cloud_source_unreadable); öbürleri okunur. Dosya projenin öbür bağları gibi aranır (kayıtlı yol, çizimin klasörü, arama yolları); drawing açık çizimin dosyasıdır."
                .into(),
            aliases: vec!["bulutbilgisi".into(), "lasbilgi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::PointCloudsDescribe>(),
            output: schema::<crate::PointCloudsDescribed>(),
            plan: Some(schema::<crate::PointCloudsDescribed>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bütün bulutların kaynakları".into(),
                    input: json!({}),
                    output: None,
                },
                CommandExample {
                    title: "Bir bulutun sınıfları sayılarak".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61"], "classes": true }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_POINTCLOUDS_QUERY.into(),
            version: crate::CAD_POINTCLOUDS_QUERY_VERSION,
            title: "XYZ sor".into(),
            summary: "Bir yerin (at) planda radius içindeki en yakın noktasını kaynağın tam çözünürlüğünde okur (çizimin ayrıntı düzeyi değil); çizimi değiştirmez. \
                      Bulutlar uids ile verilir, verilmezse görünen bulutlar bakılır. COPC'de yalnız arama karesine değen düğümler okunur, LAS ve LAZ bütün okunur. \
                      Nokta kalıcı kimliğiyle söylenir: bulutun kalıcı kimliği (cloud), kaynağın buluttaki sırası (source) ve noktanın kaynağın sırasındaki yeri (index: LAS ve LAZ'da dosyanın sırası, COPC'de düğümler anahtarlarıyla); aynı dosya her okunuşta aynı kimliği verir. \
                      Cevap noktanın koordinatlarını, uzaklığını, sınıfını, yoğunluğunu, dönüşünü, kaynak kimliğini, GPS zamanını ve rengini, bakılan nokta sayısını (read) verir; eşit uzaklıkta önce bakılan kalır. Metin bulutu ve okunamayan kaynak uyarıyla atlanır (cloud_source_unreadable)."
                .into(),
            aliases: vec!["xyzsor".into(), "noktasor".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::PointCloudsQuery>(),
            output: schema::<crate::PointCloudsQueried>(),
            plan: Some(schema::<crate::PointCloudsQueried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir yerin 0,5 m içindeki en yakın noktası".into(),
                input: json!({ "at": { "x": 500150.0, "y": 4400120.0 }, "radius": 0.5 }),
                output: None,
            }],
        },
        // A point cloud's statistics in an area and profile along a line at the source's resolution, a mesh's metadata and
        // the face and values at a place and time (docs/adr/0552), held together by
        // fixtures/commands/v1/cad.pointclouds.{region,profile}.json and cad.meshes.{describe,query}.json.
        CommandDescriptor {
            id: crate::CAD_POINTCLOUDS_REGION.into(),
            version: crate::CAD_POINTCLOUDS_REGION_VERSION,
            title: "Nokta bulutu alan istatistiği".into(),
            summary: "Bir alanın (ring: planda en az üç köşeli halka, kapalı sayılır; holes: delikler) içindeki nokta bulutu noktalarının istatistiğini kaynağın tam çözünürlüğünde okur (çizimin ayrıntı düzeyi değil); çizimi değiştirmez. \
                      Sınırdaki nokta alanın içindedir, bir deliğin kesin içindeki dışındadır. Bulutlar uids ile verilir, verilmezse görünen bulutlar bakılır. COPC'de yalnız alanın kutusuna değen düğümler, LAS ve LAZ'da kutusu alanın kutusuna değen dosya bütün okunur. \
                      Cevap bulut başına ve toplam: nokta sayısı, içerdeki noktaların kutusu, yükseklik ve yoğunluk değerlerinin en küçüğü, en büyüğü ve ortalaması (toplam tam), sınıf başına nokta sayısı; alanın büyüklüğü (halka eksi delikler; halka basit sayılır), alan başına nokta (density) ve bakılan nokta sayısı (read). Bir çağrıda en çok 200 milyon nokta okunur (too_many_cloud_points). Metin bulutu ve okunamayan kaynak uyarıyla atlanır (cloud_source_unreadable)."
                .into(),
            aliases: vec!["bulutalan".into(), "bulutistatistik".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::PointCloudsRegion>(),
            output: schema::<crate::PointCloudsRegioned>(),
            plan: Some(schema::<crate::PointCloudsRegioned>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir parselin içindeki noktaların istatistiği".into(),
                input: json!({
                    "ring": [
                        { "x": 500100.0, "y": 4400100.0 }, { "x": 500140.0, "y": 4400100.0 },
                        { "x": 500140.0, "y": 4400130.0 }, { "x": 500100.0, "y": 4400130.0 }
                    ]
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_POINTCLOUDS_PROFILE.into(),
            version: crate::CAD_POINTCLOUDS_PROFILE_VERSION,
            title: "Nokta bulutu profili".into(),
            summary: "Bir yolun (path: planda en az iki köşe) çevresindeki koridorda (width: yolun iki yanında toplam genişlik; uçlar yuvarlak) nokta bulutu noktalarını istasyonlara (step: yol boyunca adım) ayırır, kaynağın tam çözünürlüğünde okur (çizimin ayrıntı düzeyi değil); çizimi değiştirmez. \
                      Noktanın istasyonu, yolun en yakın doğru parçasındaki ayağının yol boyunca uzaklığıdır (eşit uzaklıkta önceki parça); ofseti yolun solunda artıdır. İstasyon yolun başından sayılır, [from, to) aralıklıdır, son istasyon yolun ucunu da içerir. \
                      Cevap yolun uzunluğunu, koridordaki nokta sayısını, her istasyonun (boş olanlar dahil) nokta sayısını ve yükseklik değerlerinin en küçüğünü, en büyüğünü ve ortalamasını, bakılan nokta sayısını (read) verir; points ile noktalar da (kalıcı kimlik, istasyon, ofset, yer, sınıf) istasyona göre sıralı listelenir. En çok 100 000 istasyon (too_many_stations) ve 100 000 listelenen nokta (too_many_listed_points). COPC'de yalnız koridorun bir parçasının kutusuna değen düğümler okunur. Bulutlar uids ile verilir, verilmezse görünen bulutlar bakılır."
                .into(),
            aliases: vec!["bulutprofil".into(), "bulutkesit".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::PointCloudsProfile>(),
            output: schema::<crate::PointCloudsProfiled>(),
            plan: Some(schema::<crate::PointCloudsProfiled>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir yolun 4 m genişliğindeki koridorunda 10 m'lik istasyonlar".into(),
                input: json!({
                    "path": [{ "x": 500100.0, "y": 4400100.0 }, { "x": 500160.0, "y": 4400100.0 }],
                    "width": 4.0,
                    "step": 10.0
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_MESHES_DESCRIBE.into(),
            version: crate::CAD_MESHES_DESCRIBE_VERSION,
            title: "Meshin bilgisi".into(),
            summary: "Çizimin mesh rasterlerinin (uids; yoksa hepsi, çizimin sırasıyla) dosyasının başlığını ve ağını okur ve söyler; çizimi değiştirmez. \
                      Mesh: düğüm, yüz, kenar ve üçgen sayısı, kutusu, yüzlerinin toplam alanı, kenarlarının ortalama uzunluğu, koordinat sistemi (EPSG, coğrafi mi) ve veri setleri: değişkeni (vektörün x bileşeni; y bileşeni), adı ve birimi, düğümde mi yüzde mi, boyutları (zaman boyutu ayrıca: adım sayısı, ilk ve son an ve nasıl yazıldıkları); rasterin gösterdiği veri seti ve adımları. \
                      Mesh, NetCDF dosyasındaki UGRID ağıdır (2DM ve DAT Mesh ekle'de böyle bir dosyaya çevrilir). Okunamayan dosya (bulunamadı, bozuk) why ile söylenir ve uyarılır (mesh_source_unreadable); öbürleri okunur. Dosya projenin öbür bağları gibi aranır; drawing açık çizimin dosyasıdır. Klasik NetCDF'in yanında NetCDF-4 (HDF5), GRIB ve Zarr da okunur; adresteki dosya ev sahibinin ağıyla."
                .into(),
            aliases: vec!["meshbilgi".into(), "agbilgisi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::MeshesDescribe>(),
            output: schema::<crate::MeshesDescribed>(),
            plan: Some(schema::<crate::MeshesDescribed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bütün meshlerin bilgisi".into(),
                input: json!({}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_MESHES_QUERY.into(),
            version: crate::CAD_MESHES_QUERY_VERSION,
            title: "Mesh sor".into(),
            summary: "Bir yerin (at) hangi mesh yüzünde olduğunu ve veri setlerinin o yerdeki değerlerini söyler; çizimi değiştirmez. \
                      Yüz kesin bulunur (kapalı alanı yeri tutan üçgenlerin en küçük sıralısı); kalıcı kimliği meshin kalıcı kimliği (mesh), dosyadaki ağın adı (topology) ve yüzün dosyadaki sırasıdır (face); düğümleri ve alanı da söylenir. Değer: düğümdeki veri setinde üçgende doğrusal ara değer, yüzdeki veri setinde yüzün değeri, vektörde büyüklüğü; değer yoksa ya da yüz kapalıysa (maske 0) değer yok. \
                      time (1970'ten beri milisaniye) verilirse her veri setinin ondan önceki ya da onda olan son zaman adımının değeri okunur, ilk adımdan öncesinde değer yok; verilmezse rasterin gösterdiği adımlar. series ile zaman boyutlu her veri setinin her adımdaki değeri de verilir. Meshler uids sırasıyla (yoksa görünenler) bakılır, yeri tutan ilki cevaplar. Dosyadan yalnız gereken değerler okunur (read); bir çağrıda en çok 2 milyon değer (too_many_mesh_values)."
                .into(),
            aliases: vec!["meshsor".into(), "agsor".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::MeshesQuery>(),
            output: schema::<crate::MeshesQueried>(),
            plan: Some(schema::<crate::MeshesQueried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir yerin yüzü ve değerleri".into(),
                input: json!({ "at": { "x": 500012.0, "y": 4420010.0 } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_MESHES_REGION.into(),
            version: crate::CAD_MESHES_REGION_VERSION,
            title: "Mesh alan istatistiği".into(),
            summary: "Bir alanın (ring, holes) içindeki mesh yüzlerini ve veri setlerinin o yüzlerdeki istatistiğini söyler; çizimi değiştirmez. \
                      Yüz alandadır: merkezi (alanının ağırlık merkezi; içbükey yüzün merkezi yüzün dışına düşerse ilk üçgeninin ağırlık merkezi) halkanın kapalı alanında ve hiçbir deliğin kesin içinde değilse. Bir yüzün değeri o merkezde Mesh sor'un verdiği değerdir (düğümdeki veri setinde üçgende doğrusal ara değer, yüzdeki veri setinde yüzün değeri, vektörde büyüklüğü). \
                      Her mesh için yüz sayısı (count), alanları toplamı (area), faces ile yüzlerin sırası (en çok 100 000) ve her veri seti için değeri olan yüz sayısı, en küçük, en büyük, yüz alanlarıyla ağırlıklı ortalama (mean) ve değer × alan toplamı (integral). time ve variables Mesh sor'unki gibidir. Meshler uids sırasıyla (yoksa görünenler); okunamayan dosya uyarıyla atlanır (mesh_source_unreadable). Klasik NetCDF'in yanında NetCDF-4 (HDF5), GRIB ve Zarr da okunur; adresteki dosya ev sahibinin ağıyla. Bir çağrıda en çok 2 milyon değer (too_many_mesh_values)."
                .into(),
            aliases: vec!["meshalan".into(), "agalan".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::MeshesRegion>(),
            output: schema::<crate::MeshesRegioned>(),
            plan: Some(schema::<crate::MeshesRegioned>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir alandaki yüzlerin istatistiği".into(),
                input: json!({ "ring": [
                    { "x": 500000.0, "y": 4420000.0 }, { "x": 500020.0, "y": 4420000.0 },
                    { "x": 500020.0, "y": 4420010.0 }, { "x": 500000.0, "y": 4420010.0 }
                ] }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_MESHES_PROFILE.into(),
            version: crate::CAD_MESHES_PROFILE_VERSION,
            title: "Mesh profili".into(),
            summary: "Bir yol boyunca (path) mesh veri setlerinin değerlerini istasyon istasyon söyler; çizimi değiştirmez. \
                      İstasyonlar yolun başından step aralıkla (0, step, 2·step …) ve yolun sonudur; her istasyonun yeri, yattığı yüz (meshin dışında yok) ve her veri setinin oradaki değeri Mesh sor'un verdiği gibidir (değer yoksa null). time ve variables Mesh sor'unki gibidir. Meshler uids sırasıyla (yoksa görünenler); okunamayan dosya uyarıyla atlanır (mesh_source_unreadable). Bir çağrıda en çok 100 000 istasyon (too_many_stations) ve 2 milyon değer (too_many_mesh_values)."
                .into(),
            aliases: vec!["meshprofil".into(), "agprofil".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::MeshesProfile>(),
            output: schema::<crate::MeshesProfiled>(),
            plan: Some(schema::<crate::MeshesProfiled>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir yol boyunca değerler".into(),
                input: json!({ "path": [ { "x": 500000.0, "y": 4420005.0 }, { "x": 500020.0, "y": 4420005.0 } ], "step": 5.0 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_STAC_SEARCH.into(),
            version: crate::CAD_STAC_SEARCH_VERSION,
            title: "STAC'ta ara".into(),
            summary: "Bir STAC API'sinde (endpoint: kökü, örneğin https://earth-search.aws.element84.com/v1) alanı ve zamanı kaplayan görüntü öğelerini arar; çizimi değiştirmez. bbox [batı, güney, doğu, kuzey] derece (WGS 84), datetime RFC 3339 an ya da aralık (başlangıç/bitiş, açık uç ..; 2024-07-01 gibi bir gün kendi günüdür), collections koleksiyonlar, ids öğe kimlikleri, maxCloudCover en çok bulut yüzdesi (eo:cloud_cover; sunucudan istenir ve gelende yeniden denetlenir, bulut oranı olmayan öğe kalır), limit sayfa boyu (1–100, varsayılan 10). \
                      Cevap: gönderilen istek (request: POST /search, gövde anahtarları alfabetik), öğeler (kataloğun adresi, koleksiyon, kimlik, çekim zamanı, kutu, bulut oranı, platform, gsd, EPSG, lisans ve varlıkları: anahtar, imzasız adres, roller, bantlar, rasterın veri türü, nodata, ölçek ve öteleme, proj:shape ve proj:transform), sunucunun gönderdiği sayı (returned), süzülenler (dropped), toplam (matched) ve sonraki sayfa (next: sunucunun next bağlantısı; verilirse o sayfa alınır, adresi uç noktanın sunucusunda olmalı). \
                      Bir varlığın kalıcı kimliği kataloğun adresi + koleksiyon + öğe + varlık anahtarıdır; imzalı (geçici) adresin sorgusu hiçbir yerde gösterilmez ya da yazılmaz (signed: true). Ağ ev sahibinindir: masaüstü ve MCP ağa gider; ağı olmayan ev sahibi stac_unavailable der."
                .into(),
            aliases: vec!["stac".into(), "stacara".into(), "uydugoruntusu".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::StacSearch>(),
            output: schema::<crate::StacSearched>(),
            plan: Some(schema::<crate::StacSearched>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Sentinel-2 L2A'da bir alan için temmuzdaki en çok %20 bulutlu görüntüler".into(),
                    input: json!({
                        "endpoint": "https://earth-search.aws.element84.com/v1",
                        "bbox": [36.9, 39.7, 37.1, 39.8],
                        "datetime": "2024-07-01/2024-07-20",
                        "collections": ["sentinel-2-l2a"],
                        "maxCloudCover": 20,
                        "limit": 2
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Aynı aramanın sonraki sayfası".into(),
                    input: json!({
                        "endpoint": "https://earth-search.aws.element84.com/v1",
                        "collections": ["sentinel-2-l2a"],
                        "maxCloudCover": 20,
                        "next": { "method": "POST", "href": "https://earth-search.aws.element84.com/v1/search", "body": { "next": "2024-07-08T08:29:15.791000Z,S2B_37SCD_20240708_0_L2A,sentinel-2-l2a" }, "merge": true }
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_STAC_PREPARE.into(),
            version: crate::CAD_STAC_PREPARE_VERSION,
            title: "STAC verisini hazırla".into(),
            summary: "Aramanın döndürdüğü bir öğenin (item) seçilen varlıklarını (assets: red, nir, swir16 …; her biri tek bantlı COG) çizime adresleriyle bağlı raster olarak ekler: tek yeni grup (group; yoksa “STAC: öğenin kimliği”) ve her varlık için anahtarının adını taşıyan bir katman; tek geri alma adımı “STAC verisini hazırla”. Raster ızgarasını katalogdan alır (proj:shape, proj:transform, proj:epsg; varlıkta ya da öğede) ve dosya açılmaz; sistemi projeninkinden başkaysa crs_mismatch (dönüşüm yok). nodata ve veri türü kataloğundur. \
                      qa kalite bandıdır (asset + isteğe bağlı mask: values ya da bits; verilmezse platformun kuralı: Sentinel-2 scl sınıfları 0, 1, 3, 8, 9, 10; Landsat qa_pixel bitleri 0–4): ayrıca eklenir ve veri bantlarından değildir; maske bu komutta uygulanmaz, kaydedilir. search aramanın kaydıdır (endpoint, bbox, datetime, collections, maxCloudCover). \
                      Cevaptaki hazırlama kaydı (kentos.stac-preparation 1) aramayı, öğeyi, çekim zamanını, bulut oranını, her bandın adını, adresini (imzasız), veri türünü, nodata'sını, ölçeğini ve ötelemesini, ızgarasını, kalite bandını ve maske kuralını, katman ve nesne kimliklerini taşır; reflectance bantların ortak ölçek ve ötelemesidir (Spektral indis'in Ölçek ve Öteleme'si). Aynı kayıt her rasterin özniteliklerinde de durur (stac.catalog, stac.collection, stac.item, stac.asset, stac.href, stac.scale, stac.offset …). \
                      Adresi s3:// olan varlık stac_href_not_http ile reddedilir; imzalı adresin imzası yazılmaz (stac_signed_href_stripped): imza gerektiren dosya açılmaz. plan yazmadan kaydı söyler."
                .into(),
            aliases: vec!["stachazirla".into(), "uydugoruntusuhazirla".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::StacPrepare>(),
            output: schema::<crate::StacPrepared>(),
            plan: Some(schema::<crate::StacPrepared>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir Sentinel-2 öğesinin kırmızı ve yakın kızılötesi bantlarını, SCL kalite bandıyla ekle".into(),
                input: json!({
                    "item": { "catalog": "https://earth-search.aws.element84.com/v1", "collection": "sentinel-2-l2a", "id": "S2A_37SCD_20240713_0_L2A", "assets": [] },
                    "assets": ["red", "nir"],
                    "qa": { "asset": "scl" },
                    "group": "STAC: S2A_37SCD_20240713"
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_IMAGES_EDIT.into(),
            version: crate::CAD_IMAGES_EDIT_VERSION,
            title: "Resmin kaynağı".into(),
            summary: "Resimlerin kaynağını değiştirir; her işlem tek geri alma adımı, bütün ya da hiç; çerçeve, dönüş, kırpma ve saydamlık değişmez. operation.kind: embed (Göm: uids'in, verilmezse çizimin bütün bağlı resimlerinin dosyası okunur, PNG ya da JPEG ve en çok 32 MB; baytları projenin kitaplığına içeriğinden kimlikli resim olarak girer, resim gömülü olur); relink (Yeniden bağla: uids'in resimleri file dosyasını gösterir, mutlak ya da drawing'in klasöründen göreli; gömülü resim bağlı olur). Bağlı resmin dosyası kayıtlı yolda, drawing'in klasöründen göreli ve projenin arama yollarında aranır. Plan yazmadan ne olacağını söyler."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ImagesEdit>(),
            output: schema::<crate::ImagesEdited>(),
            plan: Some(schema::<crate::ImagesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bütün bağlı resimleri göm".into(),
                    input: json!({ "operation": { "kind": "embed" }, "drawing": "/proje/ofis.kcad" }),
                    output: None,
                },
                CommandExample {
                    title: "Resmi başka dosyaya bağla".into(),
                    input: json!({ "operation": { "kind": "relink", "uids": ["018f6c1e-0000-7000-8000-000000000001"], "file": "fotograflar/cephe.jpg" }, "drawing": "/proje/ofis.kcad" }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_INSERTS_CLIP.into(),
            version: crate::CAD_INSERTS_CLIP_VERSION,
            title: "Kırp".into(),
            summary: "Blok ve dış başvuru yerleştirmelerini kırpar (AutoCAD'in XCLIP'i); tek geri alma adımı “Kırp”. operation.kind: set (boundary çizimde: rectangle a, b köşeleriyle ya da polygon points; inverted dışını gösterir; frame hide, show: yalnız ekranda çerçeve, plot: çerçeve içerikle çizilir ve yazdırılır), remove (sınır kalkar), frame (çerçevenin görünüşü), invert (içi ve dışı yer değiştirir). Sınır yerleştirmenin tanımında saklanır: yerleştirme taşınınca, döndürülünce, ölçeklenince onunla gider. Dolu alanlar alan olarak, çizgiler ve kapalı eğrilerin sınırları yol olarak kesilir; yazı, nokta ve tarama kutusunun ortasına göre ya bütün ya hiç. Kilitli katmandakiler atlanır ve sayılır. Retler: no_entities, invalid_uid, entity_not_found, not_an_insert, layer_locked, invalid_clipping (en az 3, en çok 10 000 köşe, alanı olan, kendini kesmeyen), not_clipped, invalid_revision, revision_conflict; uyarılar layer_locked, clip_empty. Plan yazmadan ne olacağını söyler."
                .into(),
            aliases: vec!["XCLIP".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::InsertsClip>(),
            output: schema::<crate::InsertsClipped>(),
            plan: Some(schema::<crate::InsertsClipped>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dikdörtgenle kırp".into(),
                    input: json!({ "uids": ["018f6c1e-0000-7000-8000-000000000001"], "operation": { "kind": "set", "boundary": { "kind": "rectangle", "a": { "x": 0, "y": 0 }, "b": { "x": 10, "y": 6 } } } }),
                    output: None,
                },
                CommandExample {
                    title: "Çokgenin dışını göster, çerçeveyi yazdır".into(),
                    input: json!({ "uids": ["018f6c1e-0000-7000-8000-000000000001"], "operation": { "kind": "set", "boundary": { "kind": "polygon", "points": [{ "x": 0, "y": 0 }, { "x": 8, "y": 0 }, { "x": 4, "y": 5 }] }, "inverted": true, "frame": "plot" } }),
                    output: None,
                },
                CommandExample {
                    title: "Kırpmayı kaldır".into(),
                    input: json!({ "uids": ["018f6c1e-0000-7000-8000-000000000001"], "operation": { "kind": "remove" } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ASSETS_CHECK.into(),
            version: crate::CAD_ASSETS_CHECK_VERSION,
            title: "Bağlı dosyalar".into(),
            summary: "Çizimin bağlı olduğu bütün dosyaları tek listede verir; çizimi değiştirmez: dış başvurular (arama yollarıyla), bağlı resimler, rasterler, nokta bulutları, bağlı tablolar, ekler ve bağlı katmanların kaynakları. Her biri: türü, kimin (uid, block, source), adı, kayıtlı yolu, bulunduğu yer ve nasıl (path, relative, searchPath, none; adres ve veritabanına bakılmaz), durumu (ok, changed: kayıtlı SHA-256'dan başka, missing, unloaded, remote, database), denenen yollar. drawing açık çizimin dosyası; kinds yalnız bu türler; digests: false özet hesaplamaz."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::AssetsCheck>(),
            output: schema::<crate::AssetsChecked>(),
            plan: Some(schema::<crate::AssetsChecked>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bütün bağlı dosyalar".into(),
                    input: json!({ "drawing": "/proje/ofis.kcad" }),
                    output: None,
                },
                CommandExample {
                    title: "Yalnız dış başvurular ve resimler, özetsiz".into(),
                    input: json!({ "drawing": "/proje/ofis.kcad", "kinds": ["xref", "image"], "digests": false }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SCHEDULES_CREATE.into(),
            version: crate::CAD_SCHEDULES_CREATE_VERSION,
            title: "Çizelge".into(),
            summary: "Çizimin açıklıklarından, mahallerinden, kolonlarından ya da duvarlarından (of: openings, spaces, columns, walls) bir çizelge tablosu yerleştirir; sol üst köşesi at. Tek geri alma adımı “Çizelge”. \
                      columns sütunlardır (field: number, type, form, width, height, sill, head, wall, level, phase, count, area, perimeter, volume, name, usage, tag, place, length, thickness, attr; title başlık, attr öznitelik adı); verilmezse türün sütunları. filter kat, evre, biçim (yalnız açıklıkta) ve katmanla süzer; sort sütunlara göre sıralar (yoksa çizimin sırası); group aynı satırları birleştirir (adet, alan, çevre, hacim ve uzunluk toplanır); total toplam satırı ekler. \
                      Çizelgenin satırları her adımda belgece yeniden yazılır: kapı değişince satırı da değişir. Uzunluklar projenin biriminde ve basamağında, alanlar alan biriminde. height yazı yüksekliği (m; verilmezse projenin tablo yüksekliği çizim ölçeğinde). Katman açıkça verilir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SchedulesCreate>(),
            output: schema::<crate::SchedulesCreated>(),
            plan: Some(schema::<crate::SchedulesPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kapı ve pencere çizelgesi, numaraya göre".into(),
                    input: json!({ "layerId": "cizelge", "at": { "x": 20.0, "y": 10.0 }, "of": "openings", "sort": [{ "column": 0 }], "total": true }),
                    output: None,
                },
                CommandExample {
                    title: "Zemin katın mahal çizelgesi".into(),
                    input: json!({ "layerId": "cizelge", "at": { "x": 20.0, "y": 0.0 }, "of": "spaces", "filter": { "levels": ["zemin"] }, "total": true }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SCHEDULES_EDIT.into(),
            version: crate::CAD_SCHEDULES_EDIT_VERSION,
            title: "Çizelgeyi düzenle".into(),
            summary: "Bir çizelgeyi değiştirir; tek geri alma adımı “Çizelgeyi düzenle”. operation: cell (row tablonun satırı, 0 başlıktır; column sütun; value hücrenin yazısı) satırın nesnelerinin kendi alanına yazılır: numara, tür, genişlik, denizlik, lento, ad, kullanım, etiket, kalınlık ve öznitelik düzenlenir, uzunluk projenin biriminde okunur, boş yazı isteğe bağlı alanı kaldırır; \
                      adet, alan, çevre, hacim, yükseklik, uzunluk, duvar, kat, evre, konum ve biçim hesaplanır (cell_computed); birden çok nesneli gruplu satırın numarası yazılmaz (schedule_group_number). set sütunları, süzgeci (null kaldırır), sıralamayı, grubu ve toplamı değiştirir. \
                      Satırın nesnelerinden biri kilitli katmandaysa hiçbir şey yazılmaz. Plan yazmadan çizelgeyi olacağı gibi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SchedulesEdit>(),
            output: schema::<crate::SchedulesEdited>(),
            plan: Some(schema::<crate::SchedulesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İkinci satırın numarasını yaz".into(),
                    input: json!({ "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e70", "operation": { "kind": "cell", "row": 2, "column": 0, "value": "K02" } }),
                    output: None,
                },
                CommandExample {
                    title: "Türe göre grupla".into(),
                    input: json!({ "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e70", "operation": { "kind": "set", "columns": [{ "field": "type" }, { "field": "width" }, { "field": "count" }], "group": true } }),
                    output: None,
                },
            ],
        },
        // A schedule's columns and rows read back (docs/adr/0733 §6).
        CommandDescriptor {
            id: crate::CAD_SCHEDULES_QUERY.into(),
            version: crate::CAD_SCHEDULES_QUERY_VERSION,
            title: "Çizelgeyi oku".into(),
            summary: "Bir çizelgenin (uid) sütunlarını ve satırlarını okur, yazmaz. columns: başlık (birimiyle), alan, editable (hücresi satırın nesnelerine yazılır: numara, tür, genişlik, denizlik, lento, ad, kullanım, etiket, kalınlık, kaplamalar, öznitelik) ya da hesaplanır, summed (gruplu satır, ara toplam ve toplam toplar). \
                      rows: tablodaki sırası (cad.schedules.edit'in row'u), rolü (heading başlık, data nesnelerin satırı, header grup başlığı, footer ara toplam, total genel toplam), hücreleri ve veri satırının nesneleri (kalıcı kimlikler, çizimin sırasıyla): düzenlenen bir hücre bu nesnelere yazılır. Başlık ve ara toplam satırının nesnesi yoktur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SchedulesQuery>(),
            output: schema::<crate::SchedulesQueried>(),
            plan: Some(schema::<crate::SchedulesQueried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Hangi hücre hangi kapılara yazılır".into(),
                input: json!({ "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e70" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_OPENINGS_CREATE.into(),
            version: crate::CAD_OPENINGS_CREATE_VERSION,
            title: "Açıklık ekle".into(),
            summary: "Duvarlara kapı, pencere ya da boşluk koyar; tek geri alma adımı, ilkinin biçimiyle “Kapı”, “Pencere” ya da “Boşluk”. \
                      Her açıklık duvarıyla (host: kalıcı kimliği) ve yeriyle verilir: at (duvarın taban çizgisi boyunca başından açıklığın ortasına, m), ya da point (duvarın yanında bir nokta: ortası taban çizgisinin ona en yakın yerinde), ya da point ile distance ve from (açıklığın yakın pervazı duvarın o ucundan, köşeden distance m; from start, end ya da near: noktaya yakın uç), ya da point ile middle (noktanın yanındaki kenarın ortası). \
                      type KentOS'un açıklık türüdür (kimliği ya da adı: Kapı 90, Kapı 80, İki kanatlı kapı 140, Sürme kapı 160, Pencere 120, Sabit pencere 100, Boşluk 100); verilmezse biçiminin ilki. form, width, leaves (1 ya da 2), operation (swing kanatlı, slide sürme, fixed sabit), hand (menteşe yolun başında ya da sonunda), side (kanadın açıldığı yan; verilmezse noktanın yanı, o da yoksa sol), swing (açılma açısı, derece) ve number türün değerlerinin üstüne yazılır. \
                      distanceReference uzaklığın yakın pervaza (jamb, varsayılan), merkeze (center) veya hand ile seçilen menteşeye (hinge) olduğunu söyler; distance gerekir. span iki pervaz noktasıdır: host yoluna izdüşümlerinin yay uzunluğu genişlik, yol üzerindeki ortası konum olur; at/distance/middle/width ile birlikte verilmez. point ayrıca açılma yanını seçebilir. \
                      Açıklık duvarın tek bir kenarında (düz ya da yay), uçlarının arasında olmalıdır (opening_off_wall) ve aynı duvardaki başka açıklığa binmez (openings_overlap). Pervazları duvardan çizilir, duvarın çizgileri onların arasında açılır; duvar taşınınca, uzayınca ya da kalınlaşınca açıklık onunla gider. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz. Plan yazmadan açıklıkları pervazlarıyla gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::OpeningsCreate>(),
            output: schema::<crate::OpeningsCreated>(),
            plan: Some(schema::<crate::OpeningsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Duvarın başından 1,5 m'de 90'lık bir kapı, odaya doğru açılan".into(),
                    input: json!({
                        "layerId": "kapi",
                        "openings": [{
                            "host": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "at": 1.5,
                            "type": "Kapı 90",
                            "side": "left",
                            "number": "K1"
                        }]
                    }),
                    output: Some(json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"],
                        "ids": [14],
                        "revision": "39"
                    })),
                },
                CommandExample {
                    title: "Tıklanan yerin yakın köşesinden 40 cm'de bir pencere".into(),
                    input: json!({
                        "layerId": "pencere",
                        "openings": [{
                            "host": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "point": { "x": 5.2, "y": 0.4 },
                            "distance": 0.4,
                            "type": "kentos-pencere-120"
                        }]
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_OPENINGS_EDIT.into(),
            version: crate::CAD_OPENINGS_EDIT_VERSION,
            title: "Açıklığı düzenle".into(),
            summary: "Kalıcı kimlikleri verilen kapı, pencere ve boşlukları değiştirir; tek geri alma adımı “Açıklığı düzenle”, hepsi ya da hiçbiri. \
                      operation: set (alanlar: genişlik, biçim, kanat, açılış, menteşe, yan, açı, açılım yayı, kasa, denizlik, lento, numara; null alanı kaldırır), move (by: duvar boyunca kaydırma, sona doğru artı; at: yeni konum), flip (what: side açılış yanı, hand menteşe, both ikisi), rehost (to: başka duvarın kimliği; point verilirse ortası oraya en yakın yerde, verilmezse pervazlarının ortasına en yakın yerde; aynalanmış gibi düşerse menteşe ve yan çevrilir), replaceType (to: KentOS'un türü; uids boşsa from türündeki bütün açıklıklar), renumber (prefix ve start: K1, K2 …; duvarların sırasıyla ve her duvar boyunca; uids boşsa form biçimindeki ya da bütün açıklıklar), copy (offsets: her uzaklıkta duvar boyunca bir kopya, numarasız), distance (docs/adr/0701: side start/end, face left/right, distance metre: pervazın o yöndeki en yakın referansa — yüzün köşesi ya da ucu, aynı kenardaki başka açıklığın pervazı, yüzü kesen başka duvar — net mesafesi bu olur; referans sabit, açıklık kenarında kalır; mesafeyi önce cad.openings.distances okur), array (tek kaynak, korunur; layout.count yalnız yeni kopyaların adedi 1–10000; spacing: distance net pervaz arası veya centers merkez aralığı, direction start/end; fit: startMargin/endMargin net uç payları arasında eşit dağıtım, 1 adet ortalanır; uzunluklar metre, analitik duvar yolu boyunca; numbering isteğe bağlı prefix/start/step, yalnız yeni kopyalar, aynı biçimdeki mevcut açıklıkla yinelenen numara tüm işlemi reddeder). \
                      Duvarın dışına ya da köşesine taşan (opening_off_wall), başka açıklığa binen (openings_overlap) açıklık yazılmaz; duvarı olmayan açıklık duvar boyunca taşınmaz ve kopyalanmaz (opening_host_gone), önce rehost'la bir duvara konur. Kilitli katmandaki açıklık bütün değişikliği reddeder. Plan yazmadan açıklıkları olacakları gibi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::OpeningsEdit>(),
            output: schema::<crate::OpeningsEdited>(),
            plan: Some(schema::<crate::OpeningsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kapının açılışını öbür yana çevir".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"],
                        "operation": { "kind": "flip", "what": "side" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Kaynağı koruyup 1 metre net aralıkla üç kapı çoğalt".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"],
                        "operation": {"kind": "array", "layout": {"kind": "spacing", "count": 3,
                            "distance": 1.0, "reference": "clear", "direction": "end"},
                            "numbering": {"prefix": "K", "start": 2, "step": 1}}
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bütün kapıları K1'den numarala".into(),
                    input: json!({
                        "operation": { "kind": "renumber", "prefix": "K", "start": 1, "form": "door" }
                    }),
                    output: None,
                },
            ],
        },
        // Yeniden bağla (docs/adr/0249 §3), held together by
        // fixtures/commands/v1/cad.dimensions.reassociate.json. The web is paused
        // (docs/adr/0228): the desktop's host alone runs it for now.
        // Kapı ve pencere tipleri (docs/adr/0772), held together by fixtures/commands/v1/cad.openingtypes.edit.json and
        // cad.openings.query.json (scripts/fixtures/opening_types_cases.py).
        CommandDescriptor {
            id: crate::CAD_OPENING_TYPES_EDIT.into(),
            version: crate::CAD_OPENING_TYPES_EDIT_VERSION,
            title: "Kapı ve pencere tipleri".into(),
            summary: "Projenin kapı ve pencere tiplerini (ofisin K01'i) yönetir (docs/adr/0772); tek geri alma adımı “Açıklık tipi”, tipi izleyen açıklıklar aynı adımda. operation: define (name, id isteğe bağlı, copyOf: projenin ya da KentOS'un tipi, form, change; sürüm 1), update (type: kimlik ya da ad, change; sürüm bir artar, tipi izleyen açıklıklar yeni değerleri alır), rename (type, name; kimlik değişmez), remove (type; izleyen açıklık varsa opening_type_in_use, detach: true açıklıkları değerleriyle tipten ayırır), import (library: kentos.opening-types 1 dosyasının içeriği, tiplerin kaynağı kitaplığın adı; onConflict: refuse, skip, replace — yalnız aynı kimlik ve kaynak, yeni sürüm —, rename), export (library adı, types; dosyanın içeriği library çıktısında, çizim değişmez). change: width (kaba boşluk) ya da nominal (by: rough kaba boşluk, unit kasa dış eni, leaf kanat eni; value: kaba boşluk bundan, montaj boşluğu ve kasadan hesaplanır), leaves, operation, swing, showSwing, frame, sill, head, depth, build (sizeBy, fit montaj boşluğu, leafThickness, split eşit olmayan kanatlarda birinci kanadın payı, frameDepth, frameOffset, threshold eşik, casing pervaz, sillBoard denizlik çıkıntısı, mullions, transoms, useZone kullanım alanı), unset (yapının alanlarını kaldırır). Tipi izleyen açıklığın istisnası (overrides) olan alan korunur (report.kept, opening_type_kept); kilitli katmandaki açıklık eski sürümde kalır (report.skipped, opening_type_skipped); yeni değerle duvarına sığmayan açıklık bütün değişikliği reddeder (opening_type_refused) ya da onRefused: skip ile eski sürümde kalır (report.refused). Plan yazmadan tabloyu ve her açıklığın sonucunu gösterir (etkilenenlerin önizlemesi)."
                .into(),
            aliases: vec!["KAPITIPI".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::OpeningTypesEdit>(),
            output: schema::<crate::OpeningTypesEdited>(),
            plan: Some(schema::<crate::OpeningTypesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "90 cm kanatlı K01 kapı tipi".into(),
                    input: json!({
                        "operation": {"kind": "define", "name": "K01", "copyOf": "kentos-kapi-90",
                            "change": {"nominal": {"by": "leaf", "value": 0.9}, "frame": 0.05,
                                "build": {"fit": 0.01, "leafThickness": 0.04}}}
                    }),
                    output: None,
                },
                CommandExample {
                    title: "K01'in kasasını 6 cm yap; izleyen kapılar alır".into(),
                    input: json!({"operation": {"kind": "update", "type": "K01", "change": {"frame": 0.06}}}),
                    output: None,
                },
            ],
        },
        // Malzemeler (docs/adr/0785), held together by fixtures/commands/v1/cad.materials.edit.json
        // (scripts/fixtures/materials_cases.py).
        CommandDescriptor {
            id: crate::CAD_MATERIALS_EDIT.into(),
            version: crate::CAD_MATERIALS_EDIT_VERSION,
            title: "Malzemeler".into(),
            summary: "Projenin yapı malzemelerini (kalıcı kimlik, ad, kategori, metrajda alan ya da hacim, kesitin taraması ve kalemi) yönetir ve duvar stili bileşenlerini, duvar bileşenlerini ve döşemeleri onlara bağlar (docs/adr/0785); tek geri alma adımı “Malzeme”, bağlı stiller, duvarlar ve döşemeler aynı adımda. operation: define (name, id isteğe bağlı: adından; change: category structure/insulation/finish/membrane/other, quantity area/volume, hatch {pattern, scale, angle, color}, cut {color, lineWeight, lineType}), update (material: kimlik ya da ad, change; name yeniden adlandırır, kimlik değişmez; bağlı bileşenlerin material metni, taraması ve kalemi, döşemelerin material metni yeni değerleri alır), remove (material; bağlı bir şey varsa material_in_use ve bağlı duvar ve döşemelerin kimlikleri), assign (material: kimlik ya da ad, yoksa bağdan ayırır; targets: wallStyle {style, component}, wall {uid, component}, slab {uid}; bileşen sırası soldan 0'dan), adopt (bağsız her metin malzemeden bir proje malzemesi açar ve bağlar; aynı metnin taraması ya da kalemi farklı bileşenler bağlanmaz, görünüş değişmez: material_adopt_left). Kilitli katmandaki duvar ve döşeme eski değerleriyle kalır (report.locked, material_locked); bağlanacak nesne kilitliyse layer_locked. Plan yazmadan tabloyu ve etkilenenleri (report: styles, walls, slabs) gösterir. Malzeme lejandı ve Metraj bağlı bileşenleri malzemenin kimliğiyle gruplar. Ofis malzeme kitaplığı (docs/adr/0791): import (library: kentos.materials 1 dosyası ya da name: KentOS kitaplığı kentos-temel; materials: kimlik ya da ad, yoksa hepsi; onConflict: refuse varsayılan material_exists, keep projeninkini korur material_kept, replace aynı kimlikliyi kitaplığınkiyle değiştirir ve bağlıları izler; başka kimlikli aynı ad keep dışında reddedilir; report.taken, replaced, kept), export (library: adı, materials: yoksa hepsi; çizim değişmez, cevabın library'si dosyanın içeriği). Bozuk ya da bilinmeyen kitaplık invalid_library."
                .into(),
            aliases: vec!["MALZEME".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::MaterialsEdit>(),
            output: schema::<crate::MaterialsEdited>(),
            plan: Some(schema::<crate::MaterialsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Tuğla malzemesi, taşıyıcı ve hacimle".into(),
                    input: json!({"operation": {"kind": "define", "name": "Tuğla", "change": {"category": "structure",
                        "hatch": {"pattern": "ANSI31", "scale": 1.0, "angle": 0.0}}}}),
                    output: None,
                },
                CommandExample {
                    title: "Dış duvar stilinin çekirdeğini tuğlaya bağla".into(),
                    input: json!({"operation": {"kind": "assign", "material": "tugla",
                        "targets": [{"kind": "wallStyle", "style": "Dış duvar 25", "component": 1}]}}),
                    output: None,
                },
                CommandExample {
                    title: "KentOS temel malzeme kitaplığından taşyünü ve EPS'i al, aynısı varsa projeninkini koru".into(),
                    input: json!({"operation": {"kind": "import", "name": "kentos-temel",
                        "materials": ["tasyunu", "eps"], "onConflict": "keep"}}),
                    output: None,
                },
                CommandExample {
                    title: "Projenin malzemelerini ofis kitaplığı olarak ver".into(),
                    input: json!({"operation": {"kind": "export", "library": "Ofis malzemeleri"}}),
                    output: None,
                },
            ],
        },
        // Oda modülleri (docs/adr/0775), held together by fixtures/commands/v1/cad.modules.edit.json and
        // cad.modules.query.json (scripts/fixtures/module_cases.py); several storeys and a repeat's loose objects
        // (docs/adr/0796) by cad.modules.edit.storeys.json and cad.modules.query.storeys.json
        // (scripts/fixtures/module_storeys_cases.py).
        CommandDescriptor {
            id: crate::CAD_MODULES_EDIT.into(),
            version: crate::CAD_MODULES_EDIT_VERSION,
            title: "Oda modülü".into(),
            summary: "Bir banyoyu ya da odayı bir kez çizip bağlı örnekler olarak yeniden kullanır (docs/adr/0775); her yazma tek geri alma adımı “Oda modülü”. operation: define (name, id isteğe bağlı, uids: tek kattaki düz duvarlar, açıklıklar, mahaller, donatılar, ölçüler, etiketler ve yazılar; base taban noktası; nesneler yerinde kalır ve ilk örnek olur; açıklığın duvarı, mahalin kaynakları, ölçünün noktaları ya da etiketin nesnesi seçimin dışındaysa module_outside), place (module, at, rotation radyan, mirror kendi y ekseninde aynalı, level hedef kat, phase evre, numbering: prefix ve suffix mahal, açıklık numaralarına; örnek içi duvar, mahal kaynağı, ölçü ve etiket bağları örneğin kendi nesnelerine bağlanır), update (module, from: kaynağı veren örnek, instances: güncellenecekler, boş: hepsi; tanımın sürümü bir artar, bağlı örnekler yeni tanımı alır, yerel istisna ve açıklamaların görünüşteki yeri korunur, kilitli katmandaki örnek eski sürümde kalır: module_instance_skipped), except ve restore (uids: örneğin nesneleri; yerel istisna yap, kaynağa döndür), detach (module, instance: örnek modülden ayrılır, nesneler kalır), rename, remove (tanım silinir; örneklerin nesneleri son hâliyle kalır, bağları kopuk: sourceGone). addMembers (module, instance, uids: örnekte çizilen yeni nesneler tanıma katılır; öbür bağlı örneklere kendi matrisleriyle yerleşir; docs/adr/0779), removeMembers (uids: örneğin nesneleri anahtarlarıyla tanımdan çıkar; kendisi sıradan nesne olarak kalır, öbür örneklerdeki karşılıkları silinir, istisna ya da kilitli olanlar serbest kalır: module_member_released; kalan bir üye ona bağlıysa module_member_needed), repeat (from: kaynak kat, modules: süzgeç, to: hedef katlar, her birinde numbering, unit ve örneğe özel satırlar instances: module, instance, numbering, unit; kattaki bağlı örnekler aynı yer, dönüş ve aynayla her hedef kata yerleşir; boş kat module_repeat_empty, zaten yerleşmiş module_repeat_taken; loose: kattaki modülsüz duvar, açıklık, mahal, donatı, ölçü ve etiketler de düz kopya olarak her hedefe gider, birbirlerine ve tekrarlanan örneklerin nesnelerine bağları kopyalara çevrilir, numaraları hedefin numbering'iyle (yoksa katın mahal öneki değişir); kilitli, başka türden, tekrarlanmayan örneğin ya da dışarıya bağlı olan kalır: report.left, module_repeat_left; cevabın copied'ı; docs/adr/0796). Birden çok kattaki nesneler çok katlı modül olur (taban en alçak kat; duvar, mahal ve donatının storey'i taban katının kaç kat üstünde olduğu; katlar kotlarına göre sıralanır); place ve repeat üst katları hedefin üstündeki katlara koyar, yetmeyen kat module_storey_missing; katlı ile katsız nesne birlikte module_levels. place ve repeat'in unit'i bina programının bölümü: örneğin mahalleri o bölümde olur (daire modülü). Elle değişmiş (istisna yapılmamış) örnek nesnesi güncellemede kaynağın değeriyle yazılır ve söylenir: module_drift_overwritten. Plan yazmadan yapılacakları (created, changed, placed, report.updated, skipped, conflicts, changedKeys, addedKeys, removedKeys, released, deleted) gösterir. Tasarım alternatifi kapsam dışıdır."
                .into(),
            aliases: vec!["MODUL".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ModulesEdit>(),
            output: schema::<crate::ModulesEdited>(),
            plan: Some(schema::<crate::ModulesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Banyoyu 1. katta aynalı yerleştir".into(),
                    input: json!({"operation": {"kind": "place", "module": "Banyo", "at": {"x": 12, "y": 0},
                        "mirror": true, "level": "kat1", "numbering": {"prefix": "D2-"}}}),
                    output: None,
                },
                CommandExample {
                    title: "Zemin kattaki daireleri 1. ve 2. kata tekrarla".into(),
                    input: json!({"operation": {"kind": "repeat", "from": "zemin", "to": [
                        {"level": "kat1", "numbering": {"prefix": "1-"}},
                        {"level": "kat2", "numbering": {"prefix": "2-"}}]}}),
                    output: None,
                },
                CommandExample {
                    title: "Zemin katı modülsüz nesneleriyle 1. kata tekrarla".into(),
                    input: json!({"operation": {"kind": "repeat", "from": "zemin", "loose": true, "to": [
                        {"level": "kat1", "numbering": {"prefix": "1-"}}]}}),
                    output: None,
                },
                CommandExample {
                    title: "Birinci örnekten kaynağı güncelle".into(),
                    input: json!({"operation": {"kind": "update", "module": "banyo", "from": "banyo-1"}}),
                    output: None,
                },
            ],
        },
        // Tasarım seçenekleri (docs/adr/0788), held together by fixtures/commands/v1/cad.options.edit.json and
        // cad.options.query.json (scripts/fixtures/option_cases.py).
        CommandDescriptor {
            id: crate::CAD_OPTIONS_EDIT.into(),
            version: crate::CAD_OPTIONS_EDIT_VERSION,
            title: "Tasarım seçeneği".into(),
            summary: "Planın bir parçası için sınırlı alternatifler (mutfak A/B) kurar ve yönetir (docs/adr/0788); her yazma tek geri alma adımı “Tasarım seçeneği”. Çizim, paftalar, metraj ve çizelgeler ana modeli ve her kümenin birincil seçeneğini gösterir; görünüş tarifi (cad.views.edit recipe.options) başka bir seçeneği seçebilir. Aynı kümenin seçenekleri birbirini görmez: duvarları birleşmez, açıklıkları çakışmaz, mahal sınırını ana model ve kendi seçeneğinin duvarlarında bulur. operation: define (name, id ve option isteğe bağlı, uids: hiçbir seçenekte olmayan nesneler; ilk seçenek birincil), copy (set, option, name, primary: seçeneğin nesnelerinin bağımsız kopyası aynı yerde, aralarındaki bağlar kopyalara çevrilir), addMembers (set, option, uids: ana modelden seçeneğe), removeMembers (set, uids: seçenekten ana modele), primary (set, option), accept (set, option yoksa birincil: seçenek ana model olur, öbür seçeneklerin nesneleri silinir, küme kalkar), delete (set, option: birincil olmayan seçenek nesneleriyle silinir), rename (set, option isteğe bağlı, name). Ana modelin bir nesnesi seçeneğin nesnesine ya da bir seçeneğin nesnesi başka bir seçeneğinkine bağlıysa option_leaning. Otomatik birleştirme yoktur. Evre seçenek kimliği değildir."
                .into(),
            aliases: vec!["SECENEK".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::OptionsEdit>(),
            output: schema::<crate::OptionsEdited>(),
            plan: Some(schema::<crate::OptionsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Mutfağın nesnelerinden seçenek kümesi".into(),
                    input: json!({"operation": {"kind": "define", "name": "Mutfak",
                        "uids": ["0192f5a1-7880-7000-8000-000000000005"]}}),
                    output: None,
                },
                CommandExample {
                    title: "A'nın kopyasından B seçeneği".into(),
                    input: json!({"operation": {"kind": "copy", "set": "mutfak", "option": "a", "name": "B"}}),
                    output: None,
                },
                CommandExample {
                    title: "B'yi kabul et".into(),
                    input: json!({"operation": {"kind": "accept", "set": "mutfak", "option": "b"}}),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_OPTIONS_QUERY.into(),
            version: crate::CAD_OPTIONS_QUERY_VERSION,
            title: "Tasarım seçeneklerini oku".into(),
            summary: "Projenin tasarım seçeneği kümelerini okur (docs/adr/0788); çizimi değiştirmez. set ile tek küme; her seçeneğin nesne sayısı, türlere göre sayıları, mahallerinin alanı (çizim biriminin karesi), duvarlarının boyu, silinmiş nesneleri. uid ile nesnenin kümesi ve seçeneği. compare (set, a, b, search, tolerance) iki seçeneği Veri karşılaştır'ın kuralıyla konumdan eşler: aynı sayısı, yalnız b'de (added), yalnız a'da (removed), değişen çiftler, alan ve duvar boyu farkı (b − a), sayısı farklı türler."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::OptionsQuery>(),
            output: schema::<crate::OptionsQueried>(),
            plan: Some(schema::<crate::OptionsQueried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Mutfağın A ve B seçeneklerini karşılaştır".into(),
                input: json!({"compare": {"set": "mutfak", "a": "a", "b": "b"}}),
                output: None,
            }],
        },
        // Türetilmiş çizim: oda iç görünüşü (docs/adr/0799), held together by fixtures/commands/v1/cad.elevations.edit.json
        // and cad.elevations.query.json (scripts/fixtures/room_elevation_cases.py).
        CommandDescriptor {
            id: crate::CAD_ELEVATIONS_EDIT.into(),
            version: crate::CAD_ELEVATIONS_EDIT_VERSION,
            title: "Oda iç görünüşü".into(),
            summary: "Bir mahalin iç görünüşlerini modelin bildiklerinden türetir (docs/adr/0799): mahalin dış halkalarının her düz yanı (0'dan, parça parça numaralı) odanın içinden bakılarak çizilir; yüzü o yanda olan aynı kattaki duvarlar, yüksekliği (mahalinki, yoksa duvarların en alçağı: duvarın kendisi ya da katın), duvarlardaki açıklıklar denizlik ve lento kotlarıyla, açıklık numaraları, başlık ve iki ölçü. Nesneler İç görünüşler grubunda yeni bir katmanda, katmanın türetim kaydıyla (mahal, yanlar, başlangıç, her nesnenin rolü ve kimliği, çizilenin özeti); tek geri alma adımı “Oda iç görünüşü”. Modelin bilmediği çizilmez, leftOut'ta listelenir: yaylı yan (curvedSide), mahalin boşluğu (hole), odadaki kolon, merdiven, rampa ve blok (column, stair, ramp, fixture), baca (flue), kopuk açıklık (brokenOpening); üst kotu olmayan açıklığın pervazları kesikli (headUnknown). operation: derive (space, sides isteğe bağlı, at: başlangıç, yoksa mahalin kutusunun 3 m sağı; gap: yanların arası, yoksa 1 m; name: katmanın adı), refresh (layer: modelin şimdiki hâliyle yeniden çizer, nesneler rollerine göre aynı kimlikle kalır), release (layer: türetim kaydını kaldırır, çizim kalır). Kesit/görünüş işareti (cad.viewmarks.create) yalnız başvurudur; işaret koymak çizim üretmez."
                .into(),
            aliases: vec!["ICGORUNUS".into(), "INTERIORELEVATION".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ElevationsEdit>(),
            output: schema::<crate::ElevationsEdited>(),
            plan: Some(schema::<crate::ElevationsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Salonun bütün iç görünüşleri".into(),
                    input: json!({"operation": {"kind": "derive", "space": "0192f5a1-7880-7000-8000-000000000005"}}),
                    output: None,
                },
                CommandExample {
                    title: "Yalnız A ve C yanları, çizimin sağında".into(),
                    input: json!({"operation": {"kind": "derive", "space": "0192f5a1-7880-7000-8000-000000000005",
                        "sides": [0, 2], "at": {"x": 20.0, "y": 0.0}}}),
                    output: None,
                },
                CommandExample {
                    title: "Kapı taşındıktan sonra yeniden üret".into(),
                    input: json!({"operation": {"kind": "refresh", "layer": "ic-gorunus-1"}}),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ELEVATIONS_QUERY.into(),
            version: crate::CAD_ELEVATIONS_QUERY_VERSION,
            title: "İç görünüşleri oku".into(),
            summary: "Çizimin türetilmiş oda iç görünüşlerini katman ağacının sırasıyla okur (docs/adr/0799); çizimi değiştirmez. Her biri için katmanı, mahali ve adı, istenen yanlar, durum: current (yeniden çizilse aynı ve nesnelerinin hepsi yerinde), stale (model değişti ya da nesnelerinden bazıları silindi), gone (mahali yok); silinmiş nesneler (missing), model değişmemişken elle değiştirilmiş nesneler (edited) ve türetildiği çizim sürümü. layer ile tek katman, uid ile nesnenin üyesi olduğu iç görünüş."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ElevationsQuery>(),
            output: schema::<crate::ElevationsQueried>(),
            plan: Some(schema::<crate::ElevationsQueried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bütün iç görünüşler ve güncellikleri".into(),
                input: json!({}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_MODULES_QUERY.into(),
            version: crate::CAD_MODULES_QUERY_VERSION,
            title: "Oda modüllerini oku".into(),
            summary: "Projenin oda modüllerini örnekleriyle okur (docs/adr/0775); çizimi değiştirmez. module ile tek modül, uid ile nesnenin hangi modülün hangi örneğinde ve hangi anahtarla olduğu. Her örnek: sürümü, stale (tanımdan eski), nesne sayısı, yerel istisnaları, missing (nesnesi silinmiş anahtarlar), drifted (elle değişmiş, istisna olmayan nesneler; docs/adr/0779), broken (sourceGone), unit (bina programının bölümü), katı, aynalı olup olmadığı; çok katlı modülde storeys (kat sayısı) ve örneğin levels'ı (durduğu katlar, taban önce; docs/adr/0796); uid ile drifted o nesnenin elle değişip değişmediği."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ModulesQuery>(),
            output: schema::<crate::ModulesQueried>(),
            plan: Some(schema::<crate::ModulesQueried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Banyo modülünün örnekleri".into(),
                input: json!({"module": "Banyo"}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_OPENINGS_QUERY.into(),
            version: crate::CAD_OPENINGS_QUERY_VERSION,
            title: "Açıklıkları oku".into(),
            summary: "Çizimin kapı, pencere ve öbür açıklıklarını tipleri ve genişlikleriyle okur (docs/adr/0772); çizimi değiştirmez. Süzgeçler: uids (seçim kapsamı), type (projenin ya da KentOS'un tipi, kimlik ya da ad: Aynı tiptekileri seç), level (duvarın katı; null katsız), form. Her satır: uid, form, number, type ve typeName, typeVersion (izlediği tipin aldığı sürümü), stale (tipinden eski), overrides (örnek istisnaları), level, host ve sizes: rough kaba boşluk (açıklığın width'i), unit kasa dış eni (kaba boşluk eksi iki yanda montaj boşluğu), leaf kanat eni (kasanın içi), leaves (kanatlar), clear net geçiş (kapı açıkken açık kanadın yüzünden karşı kasaya; açıda kanadın izdüşümü ve kalınlığı düşülür, kalınlık verilmezse 0.04 m), clearFirst (iki kanatlıda yalnız birinci kanat açıkken), sizeBy ve nominal (tipin ölçüsünün adladığı genişlik). types: projenin tipleri, bütün çizimde izleyen, eski sürümde kalan ve istisnalı açıklık sayılarıyla."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::OpeningsQuery>(),
            output: schema::<crate::OpeningsQueried>(),
            plan: Some(schema::<crate::OpeningsQueried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Zemin kattaki K01 kapıları".into(),
                input: json!({"type": "K01", "level": "zemin"}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_OPENINGS_DISTANCES.into(),
            version: crate::CAD_OPENINGS_DISTANCES_VERSION,
            title: "Açıklığın mesafeleri".into(),
            summary: "Kalıcı kimlikleri verilen kapı, pencere ve boşlukların geçici ölçülerini okur (docs/adr/0701): duvar kenarının seçilen yüzünde (face: left, varsayılan, ya da right; yolun yönüne göre) pervazlardan önceki (start) ve sonraki (end) en yakın referansa net mesafe. Referans yüzün köşesi ya da ucu (end), aynı kenardaki başka açıklığın pervazı (opening) ya da yüzü kesen başka duvarın yüzü (wall); from pervazın, to referansın yüzdeki noktası, metre. Yay kenardaki açıklık reddedilir (invalid_opening). Belgeye yazmaz; mesafeyi değiştirmek için cad.openings.edit'in distance işlemi."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::OpeningsDistancesInput>(),
            output: schema::<crate::OpeningsDistances>(),
            plan: Some(schema::<crate::OpeningsDistances>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kapının iç yüzdeki mesafeleri".into(),
                input: json!({ "uids": ["0192f5a1-6000-7000-8000-000000000203"], "face": "left" }),
                output: Some(json!({
                    "openings": [{
                        "uid": "0192f5a1-6000-7000-8000-000000000203", "edge": 0, "face": "left",
                        "start": { "distance": 1.425, "from": { "x": 1.55, "y": 0.125 }, "to": { "x": 0.125, "y": 0.125 },
                                   "reference": { "kind": "end" } },
                        "end": { "distance": 1.5, "from": { "x": 2.45, "y": 0.125 }, "to": { "x": 3.95, "y": 0.125 },
                                 "reference": { "kind": "wall", "uid": "0192f5a1-6000-7000-8000-000000000202" } }
                    }],
                    "revision": "12"
                })),
            }],
        },
        CommandDescriptor {
            id: crate::CAD_ARCH_AUDIT.into(),
            version: crate::CAD_ARCH_AUDIT_VERSION,
            title: "Mimari denetim".into(),
            summary: "Çizimin mimari sorunlarını tek listede verir (docs/adr/0703); çizimi değiştirmez. Gruplar (checks; yok: hepsi): walls (birleşmeyen duvarlar wall_join_*, öbür duvara maxGap kala biten uç arch_wall_end_gap, birbirine yakın ama buluşmayan uçlar arch_wall_ends_apart; her kat ayrı), openings (arch_opening_broken, tag_missing_type), spaces (arch_space_broken; alanı minArea'dan küçük arch_space_small; ortalama eni 2·alan/çevre minWidth'ten küçük arch_space_narrow), numbers (tag_duplicate_number), tags (tag_target_gone), dimensions (arch_dimension_set_broken, arch_dimension_broken), grids (docs/adr/0725 §6: aynı katta aynı etiketli ikinci ve sonraki akslar arch_grid_label_duplicate; düzeltmesi üst üste binen ve kolon bağlı olmayan aksı siler cad.entities.delete, öbürünü sıradaki boş etikete adlandırır cad.arch.edit), columns (arch_column_grid_gone), viewmarks (arch_viewmark_target_gone), levels (arch_level_missing: duvar, mahal, kolon, merdiven, rampa ya da mahal ayırıcı katsız ya da projede olmayan bir katta; düzeltmesi cad.levels.assign ile level girdisinin katı, yok: zemin kat — kotu 0 olan, yoksa ilk kat; projede olmayan level level_not_found), accessibility (docs/adr/0723 §4: yalnız profile verilince; erisilebilirlik-rampa/1 — TS 9111 özeti — ile rampanın eğimi kot farkının bandını aşarsa ramp_slope_over_profile, 10 m'den uzun eğimli kolu %6'dan dikse ramp_long_run_slope, sahanlığı 1.50 m'den kısaysa ramp_landing_short, dönen rampa 1.50 m'den darsa ramp_turn_narrow; son ikisinin düzeltmesi cad.arch.edit; katalogda olmayan profile invalid_arch_audit). docs/adr/0732: walls'ta minLength'ten (0.05 m) kısa açık duvar arch_wall_short (açıklığı yoksa düzeltmesi siler), aynı kattaki iki düz duvar boyları boyunca üst üste arch_wall_overlap (biri öbürünün aynısıysa sonrakini siler), aynı kattaki iki mahal ayırıcı aynı doğruda üst üste arch_separator_overlap; tags'te yazısı boş etiket tag_empty; stairs'te profil olmadan da merdivenin kendi uyarıları stair_short, stair_long, stair_rise_level (düzeltmesi heightFromLevel); ramps grubu ramp_no_run, ramp_slope_mismatch (düzeltmesi eğimi hesaplanana eşitler); casework grubu arch_casework_broken (düzeltmesi diziyi bırakır cad.casework.edit release). Eşikler metre ve m²: maxGap 0.10, minArea 0.10, minWidth 0.15, minLength 0.05. Her bulgu kalıcı anahtarı (key: kod:kimlikler, uç boşluğunda :start/:end, ikinci benzer #2), iletisi, onarımı (repair), nesneleri (uids), yeri (at) ve açık bir düzeltmesi varsa fix'iyle gelir: label ve step (cad.changeset.apply'ın adımı: command, input); step olduğu gibi çalıştırılır, tek geri alma adımıdır. within: levels (kat kimlikleri, null katsız) ve uids (nesneler) bulguyu nesnelerinden biri o katta ve o nesnelerdense tutar (level_not_found, entity_not_found); kapsam dışı sayısı outside. Projenin istisnaları (cad.arch.exceptions): özeti tutan bulgu exception (exceptions: hide varsayılan saklar ve excepted'te sayar, show listeler, only yalnız istisnalıları listeler), bulgusu değişen istisna lapsed ve exception_lapsed uyarısı, bulgusu artık olmayanların anahtarları stale. Bulgu yokluğu ruhsat ya da standart uygunluğu değildir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ArchAudit>(),
            output: schema::<crate::ArchAudited>(),
            plan: Some(schema::<crate::ArchAudited>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Duvarlar: öbür duvara 4 cm kala biten uç".into(),
                input: json!({ "checks": ["walls"] }),
                output: Some(json!({
                    "findings": [{
                        "key": "arch_wall_end_gap:0192f5a1-7030-7000-8000-000000000003,0192f5a1-7030-7000-8000-000000000002:end",
                        "check": "walls", "code": "arch_wall_end_gap",
                        "message": "Duvarın bitiş ucuyla öbür duvar arasında 0.040 m boşluk var; birleşmiyor.",
                        "repair": "Ucu öbür duvara uzatın; mahal ancak duvarlar birleşince kapanır.",
                        "uids": ["0192f5a1-7030-7000-8000-000000000003", "0192f5a1-7030-7000-8000-000000000002"],
                        "at": { "x": 5.0, "y": 4.14 },
                        "fix": {
                            "label": "Bitiş ucunu öbür duvara uzat",
                            "step": {
                                "command": "cad.walls.edit",
                                "input": {
                                    "uids": ["0192f5a1-7030-7000-8000-000000000003"],
                                    "operation": { "kind": "extend", "to": "0192f5a1-7030-7000-8000-000000000002", "end": "end" }
                                }
                            }
                        }
                    }],
                    "outside": 0,
                    "excepted": 0,
                    "revision": "7"
                })),
            }],
        },
        // İstisna (docs/adr/0732 §3), held together by fixtures/commands/v1/cad.arch.exceptions.json.
        CommandDescriptor {
            id: crate::CAD_ARCH_EXCEPTIONS.into(),
            version: crate::CAD_ARCH_EXCEPTIONS_VERSION,
            title: "Denetim istisnası".into(),
            summary: "Mimari denetimin bulgularını bilerek bırakılmış sayar ya da işareti kaldırır (docs/adr/0732 §3); tek geri alma adımı. operation: mark (İstisna yap: keys cad.arch.audit'in verdiği anahtarlar, note isteğe bağlı not; istisna bulgunun o anki kodu, nesneleri, iletisi ve yerinin özetine bağlanır, bulgu değişince düşer), unmark (İstisnayı kaldır: keys istisnaların anahtarları), prune (İstisnaları temizle: bulgusu artık olmayanlar). audit bulguları bulan denetimin girdisidir (yok: bütün gruplar, varsayılan eşikler, profilsiz); within ve exceptions okunmaz. İstisnalar projenin ayarıdır (arch.auditExceptions, KCAD 130). Retler: invalid_input (anahtar yok ya da yineleniyor, not 1000 karakterden uzun), denetimin retleri audit. yoluyla, finding_not_found (keys[i]: bulgu ya da istisna yok). Zaten öyle olanlar değişmez, değişen yoksa adım yazılmaz. Çıktı değişen anahtarlar (changed), istisna sayısı ve revizyon. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ArchExceptions>(),
            output: schema::<crate::ArchExceptionsEdited>(),
            plan: Some(schema::<crate::ArchExceptionsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bilerek kısa bırakılan duvar parçası istisna".into(),
                    input: json!({
                        "operation": "mark",
                        "keys": ["arch_wall_short:0192f5a1-7030-7000-8000-000000000009"],
                        "note": "Pencere ayağı; bilerek kısa."
                    }),
                    output: Some(json!({
                        "changed": ["arch_wall_short:0192f5a1-7030-7000-8000-000000000009"],
                        "exceptions": 1,
                        "revision": "8"
                    })),
                },
                CommandExample {
                    title: "Bulgusu kalmayan istisnaları temizle".into(),
                    input: json!({ "operation": "prune" }),
                    output: Some(json!({ "changed": [], "exceptions": 1, "revision": "8" })),
                },
            ],
        },
        // Bina programı (docs/adr/0773), held together by fixtures/commands/v1/cad.program.edit.json.
        CommandDescriptor {
            id: crate::CAD_PROGRAM_EDIT.into(),
            version: crate::CAD_PROGRAM_EDIT_VERSION,
            title: "Bina programı".into(),
            summary: "Bina programını ve bölümlerine verilen mahalleri yazar (docs/adr/0773); tek geri alma adımı “Bina programı”, hepsi ya da hiçbiri. program (verilmezse olduğu gibi): buildings (bina ve kanatları: kimlik, ad, levels: kapladıkları katların kimlikleri, kanadınki binasınınkilerden; docs/adr/0784), units (bağımsız bölüm, ortak alan ya da dış alan: kimlik, numara, ad, kind unit/common/outdoor, bina, kanat, recordedArea kayıtlı alan m²: ölçümle hiç yazılmaz, shareWeight: ağırlığa göre paylaştırmada bölümün pay ağırlığı, ondalık metin), shareRule (ortak alanların bağımsız bölümlere paylaştırılması: equal eşit, net net alana göre, weight ağırlığa göre; kural kentos.share/1), briefs (kullanıma göre tasarım hedefi: areaMin, areaMax m², minWidth net en, minPassage kapılar arası net geçiş, adjacent komşu kullanımlar, reach kapı ya da açık geçişle ulaşılacak kullanımlar, outdoor balkon gibi dış mahal; yasal sınır değildir), clearances (blok adına donatının önündeki kullanım alanı: width, depth m), version (hedeflerin adı ve sürümü, bulgularda yazılır). Kat ile bölüm arasında zorunlu bire bir bağ yoktur: bölümün katları mahallerinden okunur (dubleks iki kat). assign: uids mahalleri unit bölümüne verir (null bölümden çıkarır). Kaldırılan bölümde hâlâ mahal kalırsa unit_in_use (uids); kaldırılan bina ya da kanat hâlâ bölümlerde yazılıysa building_in_use (bölümlerin numaraları iletide). Retler: nothing_to_set, building_in_use, invalid_program (programın yolu), level_not_found (binanın ya da kanadın katı projede yok), no_entities, invalid_uid, entity_not_found, not_a_space, unit_not_found, unit_in_use, layer_locked. Çıktı yazılan program, bölümü değişen mahaller (assigned) ve revizyon. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ProgramEdit>(),
            output: schema::<crate::ProgramEdited>(),
            plan: Some(schema::<crate::ProgramEdited>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İki daire, ortak koridor ve yatak odasının hedefi".into(),
                input: json!({
                    "program": {
                        "units": [
                            { "id": "d1", "number": "1", "name": "Daire 1" },
                            { "id": "d2", "number": "2", "name": "Daire 2" },
                            { "id": "ortak", "number": "O", "name": "Ortak alan", "kind": "common" }
                        ],
                        "briefs": [
                            { "usage": "Yatak odası", "areaMin": 12.0, "areaMax": 16.0, "minWidth": 2.8, "reach": ["Koridor"] },
                            { "usage": "Koridor", "minPassage": 1.2 }
                        ],
                        "version": "Ofis konut hedefleri 2026-1"
                    },
                    "assign": [{ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e61"], "unit": "d1" }]
                }),
                output: None,
            }],
        },
        // Programı denetle (docs/adr/0773), held together by fixtures/commands/v1/cad.program.check.json.
        CommandDescriptor {
            id: crate::CAD_PROGRAM_CHECK.into(),
            version: crate::CAD_PROGRAM_CHECK_VERSION,
            title: "Bina programını denetle".into(),
            summary: "Bina programını çizimle karşılaştırır (salt okunur, docs/adr/0773). units: bölümlerin toplamları, her mahal bir kez: rooms (odalar), outdoor (hedefi outdoor olan balkon, teras), voids (şaft), methods (alanın alındığı yöntemler: net iç yüzler, duvar ekseni, brüt dış yüzler; birden çok ise karışık), levels (mahallerinin katları), recordedArea ve recordedDifference (kayıtlı alan yalnız okunur). totals: units, unitOutdoor, common, outdoor, voids, unassigned, all (boşluklar dışı hepsi; ortak alan bölüme girmez). spaces: mahal başına area (kendi yöntemiyle), width (içindeki en büyük dairenin çapı, net en), doors (kapılar ve başka mahale açık geçişler: ayırıcı), passage ve passageAt (hedefi minPassage olan mahalde kapılar arası en geniş geçiş ve en dar yeri), targetMin, targetMax, targetDifference (aralığın altı eksi, üstü artı, içi 0), passageDifference, adjacent ve reach (istenen kullanımlar ve onları karşılayan mahaller). findings: Mimari denetim'in program grubu: program_unit_missing (Düzelt: bölümden çıkar), program_area_below, program_area_above, program_width_narrow, program_passage_narrow, program_adjacent_missing, program_reach_missing, program_no_door, program_fixture_clearance; ileti ölçülen değeri, farkı ve kuralın sürümünü (kentos.program/1 · hedeflerin sürümü) yazar; hedefler yasal sınır değildir. findings false ise yalnız sayılar (geçiş taraması yapılmaz). units ve levels süzer. shares (programın shareRule'u varsa, docs/adr/0784): kentos.share/1 kuralı: her ortak alan kendi havuzudur, kapsamı kanadı, yoksa binası, yoksa bütün program; odaları (şaft ve dış mahal dışı) 0,01 m² adıma, tam ikili değeri yarıda çifte yuvarlanır; kapsamdaki bağımsız bölümlere ağırlıklarıyla (eşit 1, net alan adımda, pay ağırlığı tam ondalık) T·w/W tam payı, tabanı ve kalan adımlar en büyük artıklara (eşitlikte programdaki sıra); paylar havuzun alanına tam eşittir. pools: havuzun alanı, bölüm payları (weight, share, exact: m² kesri), unshared (neden); net, shared, unshared, gross toplamları ondalık metin. Bölüm satırının share'i: net, share, gross (süzgeç ne olursa olsun bütün mahalleriyle). Paylaştırılamayan havuz share_unshared uyarısı. program_level_outside: mahal bölümünün binasının ya da kanadının katlarında değil. Uzun iş olarak çalışabilir: ilerlemesini söyler, durdurulunca cancelled. Retler: unit_not_found, level_not_found."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProgramCheck>(),
            output: schema::<crate::ProgramChecked>(),
            plan: Some(schema::<crate::ProgramChecked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bütün program, bulgularıyla".into(),
                input: json!({}),
                output: None,
            }],
        },
        // Kat ver (docs/adr/0717), held together by fixtures/commands/v1/cad.levels.assign.json.
        CommandDescriptor {
            id: crate::CAD_LEVELS_ASSIGN.into(),
            version: crate::CAD_LEVELS_ASSIGN_VERSION,
            title: "Kat ver".into(),
            summary: "Mimari nesnelere projenin bir katını verir (docs/adr/0717); yerleri değişmez, tek geri alma adımı “Kat ver”. uids verilirse onlar (duvar, mahal, aks, kolon, merdiven, rampa, mahal ayırıcı; kapı, pencere, etiket, kaplama ve çizginin kendi katı yok: not_levelled), verilmezse katsız ya da projede olmayan bir kattaki bütün duvar, mahal, kolon, merdiven, rampa ve mahal ayırıcılar (Denetim'in arch_level_missing bulguları; akslar katsız kalabilir). Zaten o kattaki nesne değişmez; kilitli katmandakiler atlanır ve sayılır (layer_locked uyarısı), hepsi kilitliyse ret. Hiçbiri değişmeyecekse nothing_to_assign. Mahalın sınırı aynı adımda yeni katının duvarları arasında yeniden bulunur; kapanmayan ya da duvarları katından ayrılan mahal son sınırıyla kalır (arch_space_broken uyarısı). Plan kat verilecekleri (uid, id, kind, from) çizimin sırasıyla verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LevelsAssign>(),
            output: schema::<crate::LevelsAssigned>(),
            plan: Some(schema::<crate::LevelsAssigned>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Katsız duvar ve mahal zemin kata".into(),
                    input: json!({ "level": "zemin" }),
                    output: Some(json!({
                        "level": "zemin",
                        "assigned": [
                            { "uid": "0192f5a1-7040-7000-8000-000000000001", "id": 1, "kind": "wall" },
                            { "uid": "0192f5a1-7040-7000-8000-000000000002", "id": 2, "kind": "space" }
                        ],
                        "revision": "8"
                    })),
                },
                CommandExample {
                    title: "Seçili kolonu 1. kata taşı".into(),
                    input: json!({ "level": "kat-1", "uids": ["0192f5a1-7040-7000-8000-000000000003"] }),
                    output: Some(json!({
                        "level": "kat-1",
                        "assigned": [
                            { "uid": "0192f5a1-7040-7000-8000-000000000003", "id": 3, "kind": "column", "from": "zemin" }
                        ],
                        "revision": "9"
                    })),
                },
            ],
        },
        // Kuzey oku, Revizyon işaretle and Malzeme lejandı (docs/adr/0721), held together by
        // fixtures/commands/v1/cad.north.create.json, cad.revisions.mark.json and cad.legends.materials.json.
        CommandDescriptor {
            id: crate::CAD_NORTH_CREATE.into(),
            version: crate::CAD_NORTH_CREATE_VERSION,
            title: "Kuzey oku".into(),
            summary: "Kuzey okunu yerleştirir (docs/adr/0721 §3); tek geri alma adımı “Kuzey oku”. Çizimin kendi “Kuzey oku” bloğu varsa o, yoksa KentOS blok kitaplığınınki aynı adımda eklenip yerleştirilir (added). at okun dairesinin merkezidir; size kâğıttaki eni (mm, yok: 15; 0'dan büyük, en çok 200), çizim ölçeğiyle büyür: ölçek = size/1000 · ölçek paydası / bloğun doğu-batı eni (yazıları sayılmaz). north okun gösterdiği kuzeydir: plan (yok: çizimin yukarısı, projenin plan kuzeyi), true (bu noktadan geçen meridyen: projenin koordinat sisteminin meridyen yakınsaması, convergence; yerel projede no_true_north) ya da angle (planın kuzeyinden saat yönünün tersine radyan). Dönüş (rotation) bu açıdır. Ret sırası: not_finite (at, açı, boy), invalid_scale, invalid_revision, revision_conflict, no_true_north, katman (layer_not_found, not_a_layer, layer_locked); gizli katmana layer_hidden uyarısı. Plan yerleştirmeyi (id 0), eklenecek bloğu, dönüşü ve yakınsamayı verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::NorthCreate>(),
            output: schema::<crate::NorthCreated>(),
            plan: Some(schema::<crate::NorthPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Plan kuzeyine, 15 mm".into(),
                    input: json!({ "at": { "x": 12.0, "y": 8.0 }, "layerId": "isaret" }),
                    output: Some(json!({
                        "uid": "0192f5a1-7210-7000-8000-000000000001", "id": 1,
                        "block": "0192f5b0-0000-7000-8000-00000000000e", "added": true, "rotation": 0.0, "revision": "2"
                    })),
                },
                CommandExample {
                    title: "Coğrafi kuzeye, 20 mm".into(),
                    input: json!({ "at": { "x": 487000.0, "y": 4420000.0 }, "layerId": "isaret", "north": { "kind": "true" }, "size": 20.0 }),
                    output: Some(json!({
                        "uid": "0192f5a1-7210-7000-8000-000000000002", "id": 2,
                        "block": "0192f5b0-0000-7000-8000-00000000000e", "rotation": -0.0017027, "convergence": -0.0017027,
                        "revision": "3"
                    })),
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_REVISIONS_MARK.into(),
            version: crate::CAD_REVISIONS_MARK_VERSION,
            title: "Revizyon işaretle".into(),
            summary: "Revizyon bulutunu ve üçgenini birlikte yazar (docs/adr/0721 §4); tek geri alma adımı “Revizyon”. Bulut cad.revcloud.create'in çizdiğidir (frame: dikdörtgen, çokgen ya da nesne; arc yay boyu), özniteliği Revizyon numaradır (date, description verilirse Tarih, Açıklama). Üçgen KentOS blok kitaplığının “Revizyon” bloğudur (çizimin kendisininki varsa o; yoksa aynı adımda eklenir, added), REV özniteliği numara; size kâğıttaki eni (mm, yok: 6; en çok 100), çizim ölçeğiyle büyür. at üçgenin merkezidir; yoksa bulutun sağ üst köşesinin dışında: köşelerin en doğusu ve en kuzeyi artı yarım yay ve yarım üçgen. Revizyonları “Revizyon” bloğunun REV'e göre Blok çizelgesi listeler. Ret sırası: invalid_mark (numara boş, kenarı boşluklu, denetim karakterli ya da 16 karakterden uzun; tarih 32, açıklama 240 karakterden uzun), not_finite (at, boy), invalid_scale, sonra cad.revcloud.create'inkiler (çerçeve, yay, beklenen sürüm, katman). Plan bulutu ve üçgeni (id 0) ve eklenecek bloğu verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::RevisionsMark>(),
            output: schema::<crate::RevisionMarked>(),
            plan: Some(schema::<crate::RevisionPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Mutfak değişikliği: B revizyonu".into(),
                input: json!({
                    "layerId": "revizyon", "frame": { "kind": "rectangle", "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 4.0, "y": 3.0 } },
                    "arc": 0.5, "number": "B", "date": "27.10.2026", "description": "Mutfak tezgâhı uzatıldı"
                }),
                output: Some(json!({
                    "cloud": "0192f5a1-7210-7000-8000-000000000003", "cloudId": 3,
                    "tag": "0192f5a1-7210-7000-8000-000000000004", "tagId": 4,
                    "block": "0192f5b0-0000-7000-8000-000000000026", "added": true, "revision": "4"
                })),
            }],
        },
        CommandDescriptor {
            id: crate::CAD_LEGENDS_MATERIALS.into(),
            version: crate::CAD_LEGENDS_MATERIALS_VERSION,
            title: "Malzeme lejandı".into(),
            summary: "Duvarların malzemelerinin lejandını yazar (docs/adr/0721 §5); tek geri alma adımı “Malzeme lejandı”. Her satır bir duvar bileşenidir (duvarlar çizimin sırasıyla, bileşenleri sol yüzden): adı malzemesi (yoksa bileşenin adı), örneği duvarın taradığı gibi taranmış kutu; adı ya da taraması öncekilerden farklı olan yeni satırdır, walls onu taşıyan duvar sayısı. Lejant “Malzeme lejandı” bloğudur: çizimde varsa yerinde yeniden yapılır (remade, bütün yerleştirmeleri izler), at verilirse bir yerleştirmesi de konur (at sol üst köşe). height yazının kâğıttaki yüksekliğidir (mm, yok: 2,5; en çok 50), H = height · ölçek paydası / 1000: başlık sol üstte, i. satırın kutusu 6H eninde 3H yüksekliğinde, üstü 2H + 4H·i aşağıda, adı 7H sağda kutunun ortası hizasında. Ret sırası: not_finite (at, yükseklik), invalid_height, invalid_revision, revision_conflict, no_materials (bileşenli duvar yok), no_legend (at yok, lejant da yok), at ile katman (layer_not_found, not_a_layer, layer_locked); gizli katmana layer_hidden uyarısı. Plan satırları, bloğu ve yeni yerleştirmeyi verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LegendsMaterials>(),
            output: schema::<crate::LegendMade>(),
            plan: Some(schema::<crate::LegendPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Lejant paftanın köşesine".into(),
                    input: json!({ "at": { "x": 30.0, "y": 20.0 }, "layerId": "lejant" }),
                    output: Some(json!({
                        "materials": [
                            { "name": "Sıva", "walls": 4 },
                            { "name": "Tuğla", "pattern": "ANSI31", "walls": 4 }
                        ],
                        "block": "0192f5a1-7210-7000-8000-000000000010", "uid": "0192f5a1-7210-7000-8000-000000000011",
                        "id": 11, "revision": "12"
                    })),
                },
                CommandExample {
                    title: "Duvarlar değişti: lejant yerinde yeniden".into(),
                    input: json!({ "layerId": "lejant" }),
                    output: Some(json!({
                        "materials": [
                            { "name": "Sıva", "walls": 5 },
                            { "name": "Tuğla", "pattern": "ANSI31", "walls": 4 },
                            { "name": "Gazbeton", "pattern": "AR-CONC", "walls": 1 }
                        ],
                        "block": "0192f5a1-7210-7000-8000-000000000010", "remade": true, "revision": "14"
                    })),
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_CASEWORK_RUN.into(),
            version: crate::CAD_CASEWORK_RUN_VERSION,
            title: "Mutfak dizisi".into(),
            summary: "Düz, L ya da U mutfak dizisi yazar (docs/adr/0706, 0727); tek geri alma adımı “Mutfak dizisi”. from, through (en çok iki dik köşe) ve to dolapların sırtının dayandığı yoldur (duvarın yüzü), side yolun hangi yanında durdukları (left, right; yok: left); köşeler dolapların yanına döner. \
                      Köşe kör (corner blind, öntanımlı: sonraki kol köşeden tezgâh derinliği + 5 cm sonra başlar, köşe dolgusuyla) ya da açıktır (open: iki kol da köşeden tezgâh derinliği kadar uzakta durur). İlk kol köşesine dayanır, dolgusu başında; öbür kollar baştan dizilir, dolguları sonunda (casework_filler). Kör köşeye bir dolap derinliği kadar giren modül kördür (modüllerde blind: önünü sonraki kolun dolapları kapatır); orada evye, ocak ya da bulaşık makinesi uyarılır (casework_blind_corner). \
                      Alt dolaplar module eninde (kitaplığın 0.30–1.20 m, 0.15 adımlı enlerinden; yok: 0.60) dizilir; sink, cooktop ve dishwasher 1'den başlayan modül sırasıdır: evye 0.90 m modül, bulaşık makinesi ve fırın 0.60 m; oven true ocağın modülüne fırın koyar; fridge (start, end) yolun başına ya da sonuna 0.60 m buzdolabı yeri ayırır. perModule bir modülün kendi enini ve kapısının açıldığı yanı (left, right) verir; 0.60'tan geniş dolap çift kapılıdır. \
                      Tezgâh (alan) bütün kolları depth derinliğinde (0.30–1.20 m; yok: 0.60) tek parça örter, evye ve ocak için kesimleriyle; upper true üst dolapları ekler. Dolaplar Modül, Tür ve Açılma öznitelikleriyle yazılır. Çizimde adı olan blok olduğu gibi kullanılır, yoksa KentOS kitaplığından aynı adımda getirilir (blocks); dolabın eni Genişlik parametresidir. \
                      Kolun ucu bir duvarın düz yüzündeyse dizi o yüze bağlanır ve duvarı izler; önüne gelen kapı ve alçak pencere uyarılır (casework_clash). Dizi kaydıyla tutulur: nesneleri cad.casework.edit ile düzenlenir. \
                      Plan yazacaklarını, modülleri, kolları ve dolguyu verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::CaseworkRun>(),
            output: schema::<crate::CaseworkBuilt>(),
            plan: Some(schema::<crate::CaseworkPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "3,20 m duvar boyunca: evye ikinci, ocak dördüncü modülde".into(),
                    input: json!({
                        "layerId": "mutfak",
                        "from": { "x": 0.0, "y": 0.0 },
                        "to": { "x": 3.2, "y": 0.0 },
                        "sink": 2,
                        "cooktop": 4
                    }),
                    output: Some(json!({
                        "run": "0192f5a1-7270-7000-8000-000000000001",
                        "uids": [
                            "0192f5a1-7040-7000-8000-000000000001", "0192f5a1-7040-7000-8000-000000000002",
                            "0192f5a1-7040-7000-8000-000000000003", "0192f5a1-7040-7000-8000-000000000004",
                            "0192f5a1-7040-7000-8000-000000000005", "0192f5a1-7040-7000-8000-000000000006",
                            "0192f5a1-7040-7000-8000-000000000007", "0192f5a1-7040-7000-8000-000000000008"
                        ],
                        "ids": [2, 3, 4, 5, 6, 7, 8, 9],
                        "blocks": ["Alt dolap", "Evye", "Ocak"],
                        "modules": [
                            { "number": 1, "kind": "base", "start": 0.0, "width": 0.6, "hand": "left" },
                            { "number": 2, "kind": "sink", "start": 0.6, "width": 0.9, "hand": "double" },
                            { "number": 3, "kind": "base", "start": 1.5, "width": 0.6, "hand": "left" },
                            { "number": 4, "kind": "cooktop", "start": 2.1, "width": 0.6, "hand": "left" }
                        ],
                        "legs": [{ "length": 3.2, "start": 0.0, "end": 3.2, "filler": 0.5 }],
                        "filler": 0.5,
                        "revision": "3"
                    })),
                },
                CommandExample {
                    title: "Odanın köşesinde L dizi: kuzey duvarı boyunca köşeye, sonra batı duvarı boyunca".into(),
                    input: json!({
                        "layerId": "mutfak",
                        "from": { "x": 2.4, "y": 3.0 },
                        "through": [{ "x": 0.0, "y": 3.0 }],
                        "to": { "x": 0.0, "y": 0.6 },
                        "sink": 2,
                        "cooktop": 5,
                        "dishwasher": 3
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_CASEWORK_EDIT.into(),
            version: crate::CAD_CASEWORK_EDIT_VERSION,
            title: "Mutfak dizisini düzenle".into(),
            summary: "Mutfak dizisini yeniden dizer ya da bırakır (docs/adr/0727 §6). run dizinin kimliği ya da bir nesnesinin kalıcı kimliğidir. \
                      edit kind module: bir modülün kendi eni (width), kapısının açıldığı yan (hand) ve ne taşıdığı (holds: base, sink, cooktop, dishwasher); width ve hand verilmezse modülün kendi değerleri kalkar. kind recipe: dizinin bütün tarifi (cad.casework.run'ın girdisi). Dizi yeniden dizilir: nesneler dizideki yerleriyle (modül, evye, ocak, dolgu, köşe, üst dolap, tezgâh) yerinde güncellenir, artık olmayanlar silinir, yenileri eklenir; tek geri alma adımı “Mutfak dizisini düzenle”. \
                      kind release: dizinin kaydı kalkar, nesneleri olduğu gibi kalır (“Mutfak dizisini bırak”). \
                      Modül düzenlemesinin retleri edit, tarifinkiler edit.recipe yolundadır. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::CaseworkEdit>(),
            output: schema::<crate::CaseworkBuilt>(),
            plan: Some(schema::<crate::CaseworkPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Birinci modül 75 cm olur, sağdan açılır".into(),
                    input: json!({
                        "run": "0192f5a1-7270-7000-8000-000000000001",
                        "edit": { "kind": "module", "module": 1, "width": 0.75, "hand": "right" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Üçüncü modül bulaşık makinesi olur".into(),
                    input: json!({
                        "run": "0192f5a1-7040-7000-8000-000000000003",
                        "edit": { "kind": "module", "module": 3, "holds": "dishwasher" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Diziyi bırak: nesneler kalır".into(),
                    input: json!({
                        "run": "0192f5a1-7270-7000-8000-000000000001",
                        "edit": { "kind": "release" }
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_CASEWORK_QUERY.into(),
            version: crate::CAD_CASEWORK_QUERY_VERSION,
            title: "Mutfak dizilerini sorgula".into(),
            summary: "Çizimin mutfak dizilerini okur (docs/adr/0727 §6); hiçbir şey yazmaz. run verilirse yalnız o dizi (dizinin kimliği ya da bir nesnesinin kalıcı kimliği). \
                      Her dizi için tarifi, kolları (boy, dolapların başı ve sonu, dolgu, bağlı duvar), modülleri (sıra, tür, baş, en, kol, kapının açılması), buzdolabının yeri, toplam dolgu, nesneleri dizideki yerleriyle, çizimde artık olmayan nesneleri (missing) ve dizinin neden koptuğu (broken: wallGone duvarı silindi, notBuilt yeni yolunda dizilemedi)."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::CaseworkQuery>(),
            output: schema::<crate::CaseworkRuns>(),
            plan: Some(schema::<crate::CaseworkRuns>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimin bütün dizileri".into(),
                input: json!({}),
                output: Some(json!({ "runs": [], "revision": "3" })),
            }],
        },
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
                      Açıklıkta vertex part/ring 0, index 0..3 başlangıç-sol/sağ ve bitiş-sol/sağ jamb köşeleridir; start/end jamb ortaları, center bunların ortasıdır. Kopuk açıklığın önbellekteki köşeleri ölçülmez (no_such_place). Eğri duvardaki doğrusal ölçü kiriş mesafesidir; nominal açıklık eni değildir. \
                      Türler: aligned (Hizalı), linear (Doğrusal; angle 0 ΔY, 90 ΔX, yoksa at'ye göre), ordinate (Koordinat; çizgisi at'de biter), azimuth (Semt), slope (Eğim; za, zb ya da yerin köşesinin kotu), angle (iki düz kenar), angleVertex (köşe ve iki kol noktası), angleArc (yayın kendi açısı), radius, diameter, arcLength (between ile yayın iki nokta arası parçası), jogged (Kırıklı yarıçap: shownCentre ve on). \
                      at ölçü çizgisinin, yayının ya da çizgisinin geçtiği yerdir; offset verilirse onun yerine yazılan uzaklıktır (yarıçap, çap ve koordinat almaz: invalid_offset). dimensionStyle projenin ölçü stilinin adıdır (yoksa Standart); height verilmezse stilin yüksekliği, Standart'ta projenin Ölçü yüksekliği. \
                      Yer olarak verilen noktalar bağlanır: nesnesi değişince ölçü izler (ilişkili ölçü); kenarlardan açı kenarlarını, yarıçap, çap, kırıklı yarıçap, yay uzunluğu ve yaydan açı eğrisini izler (refs.rule). continue (Zincir ölçü) ve baseline (Baz ölçü) base ölçüden to noktasına sürer: ilk noktası temelinkini izler, stili ve görünüşü temelindir, at okunmaz. Ölçülendirme aracı ölçüyü bu komutla yazar: araç, Python ve ajan aynı girdiye aynı ölçüyü alır. \
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
        // Kesen çizgiyle iç ölçü zinciri (docs/adr/0711), held together by fixtures/commands/v1/cad.dimensions.interior.json.
        CommandDescriptor {
            id: crate::CAD_DIMENSIONS_INTERIOR.into(),
            version: crate::CAD_DIMENSIONS_INTERIOR_VERSION,
            title: "İç ölçü zinciri".into(),
            summary: "from'dan to'ya kesen çizginin kestiği duvar yüzlerini ve kolonları sırayla ölçen doğrusal ölçü zincirini layerId'ye tek adımda yazar, adı “İç ölçü zinciri” (docs/adr/0711; Archicad'in iç ölçülendirmesi). \
                      Duvarlarda faces outer (varsayılan) iki yüzü, skins bileşenler arasındaki her çizgiyi ölçer; kolonda columns ends (varsayılan) dikdörtgenin kesilen kenarlarını, centers merkezini, none hiçbirini (yuvarlak kolon her zaman merkezinden). Yalnız düz yüzler ölçülür: kesilen yay yüz atlanır, arch_curved_face_skipped uyarısıyla sayılır. \
                      Başka bir duvarın ya da kolonun gövdesinin içinde kalan kesişim (köşe ve T birleşimi, duvara gömülü kolon) görünmez yüzdür, ölçülmez. Gizli ya da dondurulmuş katmandaki duvar ve kolon kesilmez; level verilirse yalnız o mimari katınkiler (yoksa level_not_found). \
                      Noktalar kesen çizgi boyunca sıralanır, ardışık her çift bir ölçüdür (açısı kesen çizginin); en az iki nokta gerekir (no_dimension). Zincirin çizgisi at'den kesen çizgiye paralel geçer (yoksa kesen çizginin üstünde); total toplam ölçüyü bir sıra ötede (kâğıtta 8 mm) at'nin yanına ekler. \
                      Her ölçünün iki ucu kestiği yüze bağlıdır, kuralı cross (kesen çizgi): duvar kaydırılınca değer değişir, duvar uzayıp kısalınca ölçü çizgisi yerinde kalır; yüzün doğrusu kesen çizgiye paralel olursa nokta kopar. dimensionStyle, height, mask ve görünüş cad.dimensions.create'inkidir. Çıktı ve plan cad.entities.create'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DimensionsInterior>(),
            output: schema::<crate::EntitiesCreated>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Üç odayı soldan sağa kesen iç ölçü zinciri, kesen çizginin üstünde".into(),
                    input: json!({
                        "from": { "x": -1.0, "y": 2.0 },
                        "to": { "x": 13.0, "y": 2.0 },
                        "layerId": "olcu"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bütün katmanlar, kolon merkezleri ve toplam; zincir 1 m yukarıda".into(),
                    input: json!({
                        "from": { "x": -1.0, "y": 2.0 },
                        "to": { "x": 13.0, "y": 2.0 },
                        "at": { "x": 0.0, "y": 3.0 },
                        "faces": "skins",
                        "columns": "centers",
                        "total": true,
                        "layerId": "olcu"
                    }),
                    output: None,
                },
            ],
        },
        // A dimension's spacing, breaks, moved value, written value and options (docs/adr/0321 §4), held together by
        // fixtures/commands/v1/cad.dimensions.edit.json; the desktop's alone while the web is paused.
        CommandDescriptor {
            id: crate::CAD_DIMENSIONS_EDIT.into(),
            version: crate::CAD_DIMENSIONS_EDIT_VERSION,
            title: "Ölçüyü düzenle".into(),
            summary: "Ölçülerde tek bir düzenleme yapar; tek geri alma adımı düzenlemenin adıyla. \
                      space (Ölçü aralığı, DIMSPACE): base ölçüye paralel hizalı ve doğrusal ölçülerin çizgileri temelinkinden spacing kadar aralıkla dizilir (her yanda uzaklık sırasıyla; yoksa temel değer yüksekliğinin üç katı, 0 temelin çizgisine). \
                      break (Ölçüyü kır, DIMBREAK): hizalı ve doğrusal ölçülerin ölçü ve uzatma çizgileri objects'in (yoksa görünen, ölçü ve yazı olmayan bütün nesnelerin) kestiği yerlerden boşlukla kırılır; kesen yoksa no_crossing uyarısı. unbreak kırmaları kaldırır. \
                      moveText değeri kendi doğrultusunda (along) ve ona dik (across) kaydırır; 0 ve 0 yerine döndürür. text değerin yerine yazılanı koyar (<> ölçülen değerdir); null ölçülen değere döndürür. options sığdırma, yuvarlama, sıfırlar, ayak-inç biçimi, alternatif birim, tolerans ve kırma genişliğini yazar; null kaldırır. \
                      Değişmeyen ölçü sayılmaz; hiçbiri değişmezse hiçbir şey yazılmaz. Kilitli katmandaki ölçü reddedilir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DimensionsEdit>(),
            output: schema::<crate::DimensionsEdited>(),
            plan: Some(schema::<crate::DimensionsEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Üç ölçüyü temel ölçüden 2,5 m aralıkla diz".into(),
                    input: json!({
                        "edit": {
                            "kind": "space",
                            "base": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001",
                            "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003"],
                            "spacing": 2.5
                        }
                    }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003"], "revision": "12" })),
                },
                CommandExample {
                    title: "Ölçünün değerine simetrik tolerans ve mm alternatifi".into(),
                    input: json!({
                        "edit": {
                            "kind": "options",
                            "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"],
                            "options": { "tolerance": { "kind": "symmetric", "upper": 0.01 }, "alt": { "unit": "mm", "decimals": 0 } }
                        }
                    }),
                    output: None,
                },
            ],
        },
        // What dimensions measure and show (docs/adr/0321 §3), held together by
        // fixtures/commands/v1/cad.dimensions.query.json; the desktop's alone while the web is paused.
        CommandDescriptor {
            id: crate::CAD_DIMENSIONS_QUERY.into(),
            version: crate::CAD_DIMENSIONS_QUERY_VERSION,
            title: "Ölçüleri oku".into(),
            summary: "Ölçülerin (uids, yoksa çizimin bütün ölçüleri) ne ölçtüğünü ve ne gösterdiğini okur; hiçbir şey yazmaz. \
                      Her ölçü için: kind, geometrinin ölçtüğü değer (measured, unit: length metre, angle radyan, percent yüzde, coordinate metre), gösterilen yazı (shown: seçenekleriyle; elle yazılan değerde <> ölçülendir), \
                      overridden (elle yazılan değer <> içermiyor: gösterilen ölçülen değil) ve bond (free, partial, bound, broken: noktaları neyi izliyor). \
                      Çizimin boyutlarını okuyan program measured'ı okur: elle yazılan değer geometrinin ölçüsü değildir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::DimensionsQuery>(),
            output: schema::<crate::DimensionsRead>(),
            plan: Some(schema::<crate::DimensionsRead>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimin bütün ölçüleri".into(),
                input: json!({}),
                output: Some(json!({ "dimensions": [{
                    "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "kind": "aligned", "measured": 4.5, "unit": "length",
                    "shown": "4.500", "overridden": false, "bond": "bound"
                }], "revision": "12" })),
            }],
        },
        // Yazı tipleri (docs/adr/0324 §5), held together by fixtures/commands/v1/cad.fonts.report.json.
        CommandDescriptor {
            id: crate::CAD_FONTS_REPORT.into(),
            version: crate::CAD_FONTS_REPORT_VERSION,
            title: "Yazı tiplerini raporla".into(),
            summary: "Çizimin yazı stillerinin adlandırdığı yazı tiplerini ve KentOS'un hangi yazı tipiyle çizdiğini söyler; hiçbir şey yazmaz. \
                      KentOS yalnız kendi yedi yazı tipiyle çizer, ölçer ve PDF'e alt küme olarak gömer: Barlow, Arimo, Overpass, Quicksand, Architects Daughter, Courier Prime, IBM Plex Mono. \
                      Her stil için: adı, adlandırdığı dosya (DXF'in STYLE'ı), çizildiği yazı tipi ve nasıl (own: kendi yazı tipi; equivalent: ölçüleri aynı ikizi, Arial, Helvetica ve Liberation Sans için Arimo, Courier için Courier Prime; \
                      shx: AutoCAD'in SHX çizgi yazı tipi, okunmaz; missing: KentOS'ta yok), stili kullanan yazı sayısı. shx ve missing stilin kendi yazı tipiyle çizilir ve her biri font_substituted uyarısıdır: değişim sessiz değildir. \
                      names verilirse her ad aynı kuralla çözülür (face yoksa projenin yazı tipi). faces çizimin yazılarının, ölçülerinin ve kılavuzlarının çizildiği yazı tipleridir: paftanın PDF'i bunların alt kümelerini gömer."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::FontsReport>(),
            output: schema::<crate::FontsReported>(),
            plan: Some(schema::<crate::FontsReported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir SHX yazı tipinin nasıl çizileceği".into(),
                input: json!({ "names": ["romans.shx", "arial.ttf"] }),
                output: Some(json!({
                    "project": "barlow", "styles": [],
                    "names": [{ "name": "romans.shx", "how": "shx" }, { "name": "arial.ttf", "face": "arimo", "how": "equivalent" }],
                    "faces": [], "revision": "3"
                })),
            }],
        },
        // Taramanın sınırı, adaları ve ilişkisi (docs/adr/0329 §4), held together by fixtures/commands/v1/cad.hatches.edit.json.
        CommandDescriptor {
            id: crate::CAD_HATCHES_EDIT.into(),
            version: crate::CAD_HATCHES_EDIT_VERSION,
            title: "Taramaları düzenle".into(),
            summary: "Taramalarda tek bir düzenleme yapar; tek geri alma adımı düzenlemenin adıyla. \
                      recreate (Sınırı yeniden oluştur) taramanın sınırını layerId'ye (yoksa taramanın katmanı) nesne olarak yazar: form area tek alan (delikleri taramanınkiler; yaylar kesin, elips ve eğri kenarları 0,1 mm'lik kirişlerle, curves_approximated uyarısıyla), \
                      form polyline her halkayı ayrı nesne (doğru ve yaylardan kapalı çoklu çizgi, tam elips elips, tam kapalı eğri NURBS); associative (varsayılan açık) tarama sonra bu nesneleri izler. \
                      reselect (Sınırı yeniden seç) bölgeyi cad.entities.hatch'in point bölgesiyle yeniden bulur (desen, katman ve görünüş kalır). \
                      addIslands (Ada ekle) kapalı nesneleri ada yapar: ilişkili tarama onları adaları arasına alır ve nesnelerinden yeniden kesilir, ilişkisiz taramanın bölgesinden alanları çıkarılır. \
                      removeIslands (Ada çıkar) ilişkili taramanın adalarını nesneleriyle (uids), ilişkisiz taramanınkileri deliklerinin sırasıyla (holes, 0 ilki) kaldırır. \
                      untie (İlişkiyi kopar) taramalar bir şeyi izlemez. \
                      pattern (Deseni değiştir) taramaların desenini cad.entities.hatch'in pattern'iyle değiştirir (bölge, bağ ve görünüş kalır; origin köşesi her taramanın kutusunun, across ile hepsininkinin; origin verilmezse her biri kendininkini korur; annotative verilmezse ölçekleri kalır). \
                      align (Hizala) taramaların çizgilerini reference'ınkileri sürdürecek biçimde başlangıcını, aynı desendeyseler açısını ve ölçeğini de alır. Kilitli katmandaki tarama reddedilir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::HatchesEdit>(),
            output: schema::<crate::HatchesEdited>(),
            plan: Some(schema::<crate::HatchesPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Taramanın sınırını alan olarak yeniden oluştur ve taramayı ona bağla".into(),
                    input: json!({ "edit": { "kind": "recreate", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "form": "area" } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"],
                                         "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002"], "revision": "12" })),
                },
                CommandExample {
                    title: "Kolonu taramaya ada olarak ekle".into(),
                    input: json!({ "edit": { "kind": "addIslands", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001",
                                             "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003"] } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"], "revision": "13" })),
                },
            ],
        },
        // Çok kollu kılavuz (docs/adr/0326 §5), held together by fixtures/commands/v1/cad.leaders.edit.json.
        CommandDescriptor {
            id: crate::CAD_LEADERS_EDIT.into(),
            version: crate::CAD_LEADERS_EDIT_VERSION,
            title: "Kılavuzları düzenle".into(),
            summary: "Kılavuzlarda tek bir düzenleme yapar; tek geri alma adımı düzenlemenin adıyla. Kılavuzun ilk kolu pts'idir (ucu ilk köşe, ortak kolun başı son köşe); \
                      ek kolları arms (her biri ucundan kırıklarından geçerek son köşeye), blok içeriği blocks (notun yerinde, ortak kolun ucunda sırayla duran yerleştirmeler; notsuz), \
                      uçların bağı tip (ilişkili ölçünün başvurusu gibi: nesne değişince uç yerin gittiği yere gider, silinince kopar). \
                      addArm ucu pts[0] olan kol ekler (tip verilirse uç yerin noktasına); removeArm arm'ı (0 ilk kol) kaldırır, ilk kol kalkınca ilk ek kol ilk kol olur, tek kol kalkmaz; \
                      align (MLEADERALIGN) ilkini yerinde bırakır, diğerlerinin son köşelerini ilkinin son köşesinden angle doğrultusundaki doğruya izdüşümüyle, spacing verilirse önde ve arkada sırayla eşit aralıkla indirir; \
                      collect (MLEADERCOLLECT) blok içerikli kılavuzları ilkinde toplar (diğerlerinin kolları ek kol, blokları ilkinin ardına; diğerleri silinir, blokları kalır); \
                      content notu yazar ya da kaldırır; blocks içerik bloklarını yazar (not kalkar); look ok başını (filled ya da LeaderArrow adı), ok boyunu, yüksekliği, zemini; \
                      style bir kılavuz stilini izletir (değerleri yazılır) ya da bırakır; bind arm'ın ucunu bir yere bağlar ya da serbest bırakır. \
                      Kilitli katmandaki kılavuz reddedilir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LeadersEdit>(),
            output: schema::<crate::LeadersEdited>(),
            plan: Some(schema::<crate::LeadersPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kılavuza ikinci boruyu gösteren bir kol ekle".into(),
                    input: json!({ "edit": { "kind": "addArm", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001",
                                             "pts": [{ "x": 487012.5, "y": 4420003.0 }] } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"], "revision": "12" })),
                },
                CommandExample {
                    title: "Üç notu yatay bir hizaya, 3 m aralıkla diz".into(),
                    input: json!({ "edit": { "kind": "align",
                                             "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003"],
                                             "angle": 0, "spacing": 3 } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003"], "revision": "13" })),
                },
            ],
        },
        // Tablonun hücreleri, formülleri, alanları ve dosyası (docs/adr/0327), held together by
        // fixtures/commands/v1/cad.tables.edit.json.
        CommandDescriptor {
            id: crate::CAD_TABLES_EDIT.into(),
            version: crate::CAD_TABLES_EDIT_VERSION,
            title: "Tabloları düzenle".into(),
            summary: "Tablolarda tek bir işlem yapar; tek geri alma adımı. edit (Tabloyu düzenle) edits'i sırayla uygular: setCell hücrenin yazısı                       (= ile başlayan yazı formüldür, hücrenin yazısı sonucu: A1 başvuruları, A1:C3 aralıkları, + - * / ^, TOPLA/SUM, ORTALAMA/AVERAGE, MİN/MIN, MAK/MAX, SAY/COUNT,                       YUVARLA/ROUND, MUTLAK/ABS; argümanlar , ya da ; ile; hatalar #AD? #BAŞV! #DEĞER! #SAYI/0! #SAYI! #DÖNGÜ!), insertRows, deleteRows, insertColumns,                       deleteColumns (birleşik alanlar, biçimler, formüllerin başvuruları ve alanlar kayar; silinen hücreye başvuru #BAŞV!), merge, unmerge,                       style (aralığa hiza, kalın, yazı rengi, dolgu, formül ve alan değerinin basamağı; sonraki öncekinin üstüne), clearStyle,                       field (hücreyi bir kaynağa bağlar: nesnenin alanı, uzunluğu …, projenin değişkeni ya da blockCount adlı bloğun yerleştirme sayısı; değer her adımda yazılır),                       unfield, rowHeight, columnWidth, fit (sütunları yazılarına sığdırır). refresh (Tabloyu güncelle) çizelgeyi nesnelerinden, bağlı tabloyu dosyasının yolundan                       (göreliyse base'den) yeniden yazar: aynı baytlar (SHA-256 özeti) yazılmaz, yolunda olmayan dosya source_missing; elle değiştirilen hücre, dosyadaki                       karşılığı değişmedikçe korunur (overrides keep) ya da dosyanınki yazılır (replace); blok çizelgesi çizimin her adımında kendiliğinden yazılır.                       link (Dosyaya bağla) tabloyu dosyanın sayfasından yazar ve ona bağlar; unlink kaynağı bırakır. Kilitli katmandaki tablo reddedilir.                       expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::TablesEdit>(),
            output: schema::<crate::TablesEdited>(),
            plan: Some(schema::<crate::TablesPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Son satıra alanların toplamını yaz ve kalın yap".into(),
                    input: json!({ "operation": { "kind": "edit", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "edits": [
                        { "kind": "setCell", "row": 4, "col": 2, "words": "=TOPLA(C2:C4)" },
                        { "kind": "style", "row": 4, "col": 0, "rows": 1, "cols": 3, "bold": true, "fill": "#eef2f7" }
                    ] } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"], "revision": "12" })),
                },
                CommandExample {
                    title: "Bağlı tabloları dosyalarından güncelle".into(),
                    input: json!({ "operation": { "kind": "refresh", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"], "base": "/home/proje" } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"],
                                         "report": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "status": "updated", "kept": 1, "replaced": 0 }],
                                         "revision": "13" })),
                },
            ],
        },
        // Alanlar (docs/adr/0325 §1–§3), held together by fixtures/commands/v1/cad.fields.edit.json.
        CommandDescriptor {
            id: crate::CAD_FIELDS_EDIT.into(),
            version: crate::CAD_FIELDS_EDIT_VERSION,
            title: "Yazının alanlarını düzenle".into(),
            summary: "Yazının alanlarında tek bir düzenleme yapar; tek geri alma adımı. Alan, yazının harflerinden bir aralığın bir kaynağa bağlanmasıdır; \
                      harfleri kaynağın son yazılan değeridir ve kaynak değişince çizim aynı adımda yeniden yazar (sonraki biçim dilimleri ve alanlar kayar). \
                      Kaynak: object (uid'li nesnenin area alanı, perimeter çevresi, length uzunluğu, layer katmanının adı, label etiketi, attribute name adlı özniteliği, \
                      text yazının harfleri, x ve y yeri) ya da project (name proje adı, scale çizim ölçeği 1:500, unit çizim birimi, crs koordinat sistemi, variable name adlı değişkeni). \
                      format: decimals (yoksa projenin basamağı), unit (mm, cm, m, km uzunlukta; m2, donum, ha alanda; yoksa çizim birimi ve projenin alan birimi), comma ondalık virgül, prefix, suffix. \
                      insert uid'li yazının start–end harflerinin yerine alan koyar (eşitse o harfe ekler), değeri hemen yazılır; remove index'inci alanı kaldırır (harfleri yazı olarak kalır); \
                      format biçimini yazar (null projeninki); update uids'in (boşsa bütün alanlı yazıların) alanlarını yeniden yazar. \
                      Değeri okunamayan alanın harfleri #### olur, nedeni broken'da (removed nesne silindi, noValue değeri yok, notNamed öznitelik ya da değişken yok, cycle yazılar birbirini okuyor) ve field_broken uyarısıdır. \
                      Kilitli katmandaki yazı reddedilir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::FieldsEdit>(),
            output: schema::<crate::FieldsEdited>(),
            plan: Some(schema::<crate::TextsPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Mahal etiketine odanın alanını iki basamakla yaz".into(),
                input: json!({
                    "edit": {
                        "kind": "insert", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "start": 6, "end": 6,
                        "source": { "kind": "object", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "value": "area" },
                        "format": { "decimals": 2, "suffix": " m²" }
                    }
                }),
                output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"], "revision": "12" })),
            }],
        },
        // Yazının komutları (docs/adr/0325 §3), held together by fixtures/commands/v1/cad.texts.edit.json.
        CommandDescriptor {
            id: crate::CAD_TEXTS_EDIT.into(),
            version: crate::CAD_TEXTS_EDIT_VERSION,
            title: "Yazıları düzenle".into(),
            summary: "Yazılarda tek bir düzenleme yapar; tek geri alma adımı düzenlemenin adıyla. \
                      replace (Bul ve değiştir): uids'in (boşsa görünen bütün yazıların) her find'ı with olur; matchCase büyük-küçük harfi ayırır (Türkçe İ ve ı), wholeWord yalnız bütün sözcük; \
                      biçim dilimleri ve alanlar kayar, değiştirilen harflere değen alan kalkar; boş kalacak yazı (text_left_blank) ve kilitli katmandaki yazı (layer_locked) uyarıyla atlanır. \
                      number (Numarala, TCOUNT): uids sırayla start, start+step … sayılarını alır, en az digits basamakla (önüne sıfır), prefix ve suffix'le; place replace yazının yerine, prefix başına, suffix sonuna. \
                      combine (Yazıları birleştir, TXT2MTXT): yazılar en yukarıdan aşağıya (eşitse soldan) satır satır tek yazı olur; en yukarıdakinin yerinde (üstünden), yüksekliği ve görünüşüyle, öbürleri silinir. \
                      split (Satırlara ayır): yazının her paragrafı (satır sonları arası) durduğu yerde, biçimleri ve alanlarıyla ayrı yazı olur; yazı silinir. Tek paragraflı, sütunlu ya da eğri boyunca yazı not_splittable. \
                      fit (Yazıyı eğriye oturt): tek satırlık, nesneye bağlı olmayan yazılar curve'ün üstüne, kutularının ortasının düştüğü yere; \
                      turn (Doğrultuya döndür): düz yazılar edge'in at'a en yakın kenarının doğrultusuna, okunur; straighten (Düzleştir): eğri boyunca yazılar ilk harflerinde düz; \
                      almadıkları sayılır (skipped), hiçbirini almazsa text_not_taken. \
                      combine, split, number, fit, turn ve straighten'de kilitli katmandaki yazı reddedilir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::TextsEdit>(),
            output: schema::<crate::TextsEdited>(),
            plan: Some(schema::<crate::TextsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bütün yazılarda “Kat” sözcüğünü “Blok” yap".into(),
                    input: json!({ "edit": { "kind": "replace", "find": "Kat", "with": "Blok", "wholeWord": true } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"], "replaced": 1, "revision": "12" })),
                },
                CommandExample {
                    title: "Üç kapı etiketini K-01'den numarala".into(),
                    input: json!({ "edit": {
                        "kind": "number",
                        "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003"],
                        "start": 1, "step": 1, "digits": 2, "prefix": "K-"
                    } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3003"], "revision": "13" })),
                },
            ],
        },
        // Yazı denetimi (docs/adr/0325 §3), held together by fixtures/commands/v1/cad.texts.check.json.
        CommandDescriptor {
            id: crate::CAD_TEXTS_CHECK.into(),
            version: crate::CAD_TEXTS_CHECK_VERSION,
            title: "Yazıları denetle".into(),
            summary: "Yazıların biçim sorunlarını bulur; hiçbir şey yazmaz. doubleSpace: satırın içinde art arda iki ya da daha çok boşluk; \
                      repeatedWord: art arda iki kez yazılmış sözcük (“ve ve”, büyük-küçük harf Türkçe kuralıyla bir; sayılar değil); \
                      unbalanced: kapanmayan ya da açılmayan parantez ((), [], {}), açılmayan “ ” ve tek kalan \"; brokenField: değeri okunamayan alan. \
                      Sözlükle yazım denetimi yoktur. uids boşsa görünen bütün yazılar okunur; bulgular çizimin sırasıyla, her yazınınki yerine göre."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::TextsCheck>(),
            output: schema::<crate::TextsChecked>(),
            plan: Some(schema::<crate::TextsChecked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bütün yazıları denetle".into(),
                input: json!({}),
                output: Some(json!({ "findings": [{
                    "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001", "kind": "repeatedWord", "start": 6, "end": 8,
                    "words": "“ve” art arda iki kez yazılmış."
                }], "texts": 4, "revision": "12" })),
            }],
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
                      Sonraki seçimin durduğu parçayı hiçbir sınır kesmiyorsa parça bütünüyle kalkar: uçları önceki kesimlerdir. \
                      AutoCAD'in TRIM seçenekleri (ADR 0294): fence (Çit) açık bir yoldur, geçtiği her görünen nesne geçtiği yerden seçilir; crossing (Kesişen) bir penceredir, ona değen her görünen nesnenin sınırlar arasındaki pencereye değen her parçası kalkar. Çit ve pencerenin kilitli katmandaki nesneleri sayılarak atlanır (layer_locked uyarısı). \
                      edgeMode extend (AutoCAD'in EDGEMODE 1'i) sınırları uzantılarıyla da sayar: çizgi bütün doğrusuyla, yay çemberiyle, açık çoklu çizginin ilk ve son kenarı uçlarının ötesine. eraseUncut (hızlı kip) hiçbir sınırın kesmediği nesneyi seçildiği yerden bütünüyle siler. \
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
                CommandExample {
                    title: "Kesişen penceresiyle, sınırların uzantılarıyla; kesilmeyen nesne silinir (hızlı kip)".into(),
                    input: json!({ "crossing": { "min": { "x": 500004.0, "y": 4399999.0 }, "max": { "x": 500006.0, "y": 4400001.0 } }, "edgeMode": "extend", "eraseUncut": true }),
                    output: None,
                },
                CommandExample {
                    title: "Çitin geçtiği her nesne geçtiği yerden".into(),
                    input: json!({ "fence": [{ "x": 500003.0, "y": 4399995.0 }, { "x": 500003.0, "y": 4400005.0 }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_EXTEND.into(),
            version: crate::CAD_ENTITIES_EXTEND_VERSION,
            title: "Nesneleri uzat".into(),
            summary: "Kalıcı kimlikleriyle verilen açık çizgileri, çoklu çizgileri ve yayları tıklanan yere yakın uçlarından ilk sınıra kadar uzatır; tek geri alma adımı “Uzat”. \
                      Seçimler, sınırlar, fence (Çit), crossing (Kesişen: penceredeki her uç uzar) ve edgeMode cad.entities.trim'deki gibidir (ADR 0294); eraseUncut yok sayılır. Aynı nesnenin iki seçimi iki ucunu uzatabilir, bir uç bir kez uzar. Uç hiçbir sınıra ulaşmıyorsa seçim extend_failed uyarısıyla atlanır; hiçbir uç uzamazsa hiçbir şey yazılmaz. \
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
                CommandExample {
                    title: "Sınırın doğrusuna kadar uzat (Uzantılar)".into(),
                    input: json!({ "picks": [{ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "at": { "x": 500009.0, "y": 4400000.0 } }], "boundaries": { "kind": "objects", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"] }, "edgeMode": "extend" }),
                    output: None,
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
            summary: "Pencerenin (window) içindeki köşeleri dx, dy kadar taşır, öbürlerini bırakır; tek geri alma adımı “Esnet”. Yayın köşeleri uçlarıdır: \
                      yalnız ortası pencerede olan yay değişmez, bir ucu taşınan yay kiriş yüksekliğini korur; çoklu çizginin yay kenarı bükümünü korur. \
                      Bağlı olduğu nesneyi izleyen nesne kendisi esnemez, nesnesiyle izler: ilişkili tarama, bağlı yazı ve ölçünün bağlı noktası; \
                      izleyenler planın ve sonucun dependents'ındadır. \
                      uids verilirse yalnız onlar (seçim); penceresinde köşesi olmayan outside_window, bağlı olduğu nesneyi izleyen follows_object uyarısıyla atlanır. Verilmezse pencereye değen görünen nesneler. Kilitli katmandakiler locked_skipped uyarısıyla atlanır; hiçbiri kalmazsa nothing_to_stretch. \
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
                      Çizgi, çoklu çizgi, alan, yay, daire, elips, NURBS eğrisi, yardımcı çizgi ve ışın kırılır; öbürleri not_breakable. Kilitli katmandaki nesne kırılmaz. \
                      Nesnenin yerini ilk parça alır, öbür parçalar ondan yeni nesnelerdir. Her parça nesnenin taşıdıklarını alır (ADR 0296): katman, renk, kalınlık, çizgi tipi, saydamlık, sembol, öznitelikler, etiket ve ekler; kot, ölçü (M) ve genişlikler köşeleriyle gider. İlişkili ölçünün nesnedeki noktası üstünde durduğu parçayı izler, silinen kısımdaysa kopar (vertex_gone). Parçaları paylaşılan geometri çekirdeği bulur; bulamadığında break_failed çekirdeğin sözüyle döner. \
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
                      AutoCAD'in JOIN'i gibi (ADR 0296): aynı doğrudaki çizgiler tek çizgi, aynı çemberin yayları tek yay, kapanırsa daire olur; öbür zincirler çoklu çizgi. Birleşen nesne ilk nesnenin taşıdıklarını korur; öbürlerinin kot, ölçü ve genişlikleri köşeleriyle gelir, öznitelik ve etiketleri gelmez: değeri ayrışan anahtar data_conflict uyarısıyla söylenir. Hiçbir şeyle birleşmeyen nesne not_joined uyarısıyla nedeni söylenerek kalır. İlişkili ölçünün birleşen nesnedeki noktası birleşen nesneyi izler. \
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
            id: crate::CAD_ENTITIES_REVERSE.into(),
            version: crate::CAD_ENTITIES_REVERSE_VERSION,
            title: "Yönü çevir".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri aynı biçimde, öbür yöne çevirir (AutoCAD'in REVERSE'ü); tek geri alma adımı “Yönü çevir”. \
                      Çizginin uçları yer değiştirir; çoklu çizgi, kapalı çoklu çizgi, alan (delikleriyle), eğri ve NURBS eğrisi geriye doğru gider, yayları öbür yöne döner. Her nesne kimliğini ve taşıdıklarını korur; kot, ölçü (M) ve genişlikler köşeleriyle gider, yani onlar da tersine döner; ilişkili ölçünün noktası nesnedeki yerinde kalır (ADR 0296). \
                      Yay, daire ve elips hep saat yönünün tersine gider: onlar ve yönsüz türler not_reversible, kilitli katmandakiler layer_locked uyarısıyla atlanır; hiçbiri çevrilemezse ret. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesReverse>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir rotanın yönünü çevir; ölçüleri (M) de tersine döner".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"] }),
                output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "created": [], "removed": [], "revision": "64" })),
            }],
        },
        // Uzat-kısalt by its meaning (docs/adr/0298), held together by fixtures/commands/v1/cad.entities.lengthen.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_LENGTHEN.into(),
            version: crate::CAD_ENTITIES_LENGTHEN_VERSION,
            title: "Uzat-kısalt".into(),
            summary: "Kalıcı kimliğiyle verilen nesnenin uzunluğunu bir ucunda (end: start ya da end) değiştirir (AutoCAD'in LENGTHEN'i); tek geri alma adımı “Uzat-kısalt”. \
                      change: delta (length kadar uzar, eksiyle kısalır; yayda angle radyan kadar açısı değişir), percent (uzunluğun yüzdesi), total (yeni toplam length ya da yayın açısı angle), dynamic (uç to noktasına nesnenin kendi yolu boyunca en yakın yere gider). \
                      Çizgi, yay, elips yayı ve tek parçalı açık çoklu çizgi uzar ya da kısalır; kapalı çoklu çizgi, alan, daire, bütün elips ve eğri not_lengthenable. Uzatma uç kenarı kendi doğrusunda, çemberinde ya da elipsinde sürdürür: kenarlar genişliklerini korur, taşınan ucun kotu ve ölçüsü (M) uç kenar boyunca uzunlukla sürer. Kısaltma yolu keser: kalan köşeler kendilerininkini, yeni uç kesilen kenarınkini uzunlukla alır. \
                      Yeni uzunluk sıfırdan büyük değilse invalid_length, yay ya da yaylı kenar kendini kapatırsa arc_closes, açı yay dışında angle_needs_arc. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesLengthen>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir duvarın ucunu 1,5 m uzat".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "end": "end", "change": { "kind": "delta", "length": 1.5 } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "created": [], "removed": [], "revision": "65" })),
                },
                CommandExample {
                    title: "Yayın açısını 120°'ye getir".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "end": "end", "change": { "kind": "total", "angle": 2.0943951023931953 } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_DIVIDE.into(),
            version: crate::CAD_ENTITIES_DIVIDE_VERSION,
            title: "Nesneyi böl".into(),
            summary: "Kalıcı kimliğiyle verilen nesnenin üstüne noktalar koyar: division eşit parçalar (parts, 2–10 000) ya da aralık (step, metre). Noktalar nesnenin başından, fromEnd ile sonundan ölçülür; layerId'ye cad.entities.create'in kurallarıyla yazılır. Tek geri alma adımı “Böl”. \
                      Çizgi, çoklu çizgi, alan, yay, daire, elips ve eğri bölünür; öbürleri not_divisible. Aralık nesneden uzunsa no_points, 10 000'den çok nokta too_many_points. Noktaların yerini paylaşılan geometri çekirdeği bulur (elipsinkiler eğrinin üstünde, gerçek yay uzunluğuyla). \
                      block verilirse noktalar yerine bloğun adıyla yerleştirmeleri konur (align varsayılan açık: nesnenin oradaki doğrultusunda, rotation radyan eklenir; scale); bilinmeyen ad unknown_block. \
                      start aralıkla bölmede ilk noktanın ölçülen uçtan uzaklığıdır (0: uçta; parça sayısıyla start_needs_step); ends açık nesnenin kendi başını, sonunu ya da ikisini de işaretler (orada nokta yoksa). \
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
                CommandExample {
                    title: "Yol boyunca her 30 metrede, 5 metreden başlayarak, uçlara da yola hizalı direk bloğu".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "division": { "kind": "step", "length": 30.0 }, "start": 5.0, "ends": "both", "block": { "name": "Direk" }, "layerId": "aydinlatma" }),
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
                      region point: at'yi (ve alsoAt'nin her noktasını; iki noktanın bulduğu aynı bölge bir kez) çevreleyen bölge. way object (varsayılan, kapalı nesne): noktayı çevreleyen en küçük kapalı nesne, içindeki ya da kenarını aşan küçük kapalı nesneler adadır; associative (varsayılan açık) taramayı nesnelerine bağlar. way lines (çizgiler): sınırların (boundaries, boundaryLayer) çizgilerinin kapattığı yüz; associative açıksa tarama yüzü kapatan çizgileri izler; gap verilirse o kadar metreye kadar boşluklar sarkan uçtan en yakın çizgiye köprüyle kapanır, gaps_closed uyarısı kapanan boşlukları ve alanı söyler. islandStyle (verilirse islands'ın yerine): outer adaları boş bırakır, normal adaların içindeki kapalı nesneleri yeniden tarar (sırayla, ayrı taramalar; çizgiler yolunda outer gibi, islands_not_nested uyarısıyla), ignore adaları da tarar. texts açıksa bölgedeki yazılar ve bloklar kutularıyla boş kalır. \
                      region objects (Çoklu tara): her kapalı nesne, çok parçalı alanın her parçası kendi içinden taranır; kapalı olmayan not_closed uyarısıyla atlanır. \
                      pattern: name (Dolu, Çizgili, Çapraz, ANSI31, ISO02W100 …, degradeler; projenin kendi desenleri önce, cad.hatchpatterns.edit), scale (basis paper, varsayılan: kâğıtta, çizim ölçeğiyle çarpılır; basis world: desenin birimi çizimde), angle (derece), secondColor ve inverted (degrade), \
                      origin (desenin başlangıcı: world dünya başlangıcı, point bir nokta, corner her bölgenin kutusunun köşesi ya da ortası, across ile bütün bölgelerin kutusunun; yalnız çizgili ve desenli), annotative (ölçekli: çizim ölçeğinde, paftanın her ölçeğinde kâğıtta aynı büyüklükte; basis world ile ya da dolu ve degradede not_annotative). \
                      transparency (0–0,9) taramaların kendi saydamlığı; drawOrder katmanın içindeki sıra: none (yeni nesne gibi), back, front, behindBoundary ve aboveBoundary (sınırının kapalı nesnesi ve adalarının hemen altı ya da üstü; sınır başka katmandaysa sıra yazılmaz, order_other_layer); sınıra gelen sıra order_limit. \
                      Bilinmeyen ad unknown_pattern; desen bölge için çok sıksa ya da bütün bölgeler birlikte 200 000'den çok çizgi verirse pattern_too_dense; bölge yoksa not_enclosed. İş olarak çalışırken noktalar ve bölgeler arasında durdurulabilir (cancelled). \
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
        // Dosyadan blok ekle and Başka çizimden yeniden tanımla (docs/adr/0316 §3), held together by
        // fixtures/commands/v1/file.import.block.json.
        CommandDescriptor {
            id: crate::FILE_IMPORT_BLOCK.into(),
            version: crate::FILE_IMPORT_BLOCK_VERSION,
            title: "Dosyadan blok al".into(),
            summary: "path'teki .kcad çizimini (v2 ya da eski v1 JSON), DXF ya da DWG dosyasını bloklar olarak alır; tek geri alma adımı, adı “Dosyadan blok: ad.kcad”. \
                      mode drawing (varsayılan): çizimin nesneleri (resim ve tablo hariç) tek blok olur, adı name ya da dosyanın adı (çizimde varsa “(2)”), taban noktası nesnelerin sol alt köşesi; iç bloklar ve nesnelerin katmanları yollarıyla gelir. \
                      mode blocks: dosyanın names'teki (yoksa bütün) blokları, yerleştirdikleriyle gelir; çizimde aynı adlı blok varsa korunur (skipped), replace ile dosyanınkiyle yeniden tanımlanır ve bütün yerleştirmeleri yenisini gösterir. \
                      DXF ve DWG'nin sayıları dosyanın bildirdiği birimde ya da sourceUnit'te (mm, cm, m, in, ft) okunur; birim bildirmeyen dosya sourceUnit'siz reddedilir (source_unit_unknown), blok kimsenin seçmediği bir boyda gelmez. KentOS çizimi birimini taşır, sourceUnit almaz. \
                      category, alınan bloklardan kategorisi olmayanların kartına yazılır (kitaplığın klasörleri, “/” ile). Cevabın sizes'ı eklenen ve yeniden tanımlanan blokların en (x) ve boyunu (y) metre olarak verir; unit dosyanın birim kararıdır. \
                      plan okur, hiçbir şey yazmaz. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. Bloğu dosyaya yazmak file.export.block'tur."
                .into(),
            aliases: vec!["insertfile".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportBlock>(),
            output: schema::<crate::BlockFileImported>(),
            plan: Some(schema::<crate::BlockFileImported>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir çizimi tek blok olarak al".into(),
                    input: json!({ "path": "/tmp/kapi-90.kcad", "name": "Kapı 90" }),
                    output: None,
                },
                CommandExample {
                    title: "Ofis kitaplığından blokları yeniden tanımla".into(),
                    input: json!({ "path": "/tmp/ofis-bloklari.kcad", "mode": "blocks", "names": ["Kapı 90", "Pencere"], "replace": true }),
                    output: None,
                },
                CommandExample {
                    title: "Birim bildirmeyen DWG kitaplığından yatağı milimetre olarak al".into(),
                    input: json!({ "path": "/tmp/mobilya/yatak-odasi.dwg", "mode": "blocks", "names": ["Yatak 90"], "sourceUnit": "mm", "category": "Kitaplık/Mobilya/yatak-odasi" }),
                    output: None,
                },
            ],
        },
        // Bloğu başkasıyla değiştir (docs/adr/0317 §3), held together by fixtures/commands/v1/cad.blocks.replace.json.
        CommandDescriptor {
            id: crate::CAD_BLOCKS_REPLACE.into(),
            version: crate::CAD_BLOCKS_REPLACE_VERSION,
            title: "Bloğu başkasıyla değiştir".into(),
            summary: "from bloğunun çizimdeki yerleştirmelerini (ya da uids ile verilenleri) to bloğuna geçirir; tek geri alma adımı. \
                      Her yerleştirme yerini, ölçeğini, dönüşünü, katmanını ve görünüşünü korur; to'nun sabit olmayan özniteliklerinin değerleri ve taşınabilir özniteliklerinin yerleri kalır, öbürleri düşer ve söylenir (dropped, places); to'nun alanları doldurulur. \
                      purge ile from artık hiçbir yerde yerleşmiyorsa silinir. Kilitli katmandakiler atlanır ve söylenir. \
                      AutoCAD'in BLOCKREPLACE'i. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["blockreplace".into(), "blokdegistir".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::BlocksReplace>(),
            output: schema::<crate::BlocksReplaced>(),
            plan: Some(schema::<crate::BlocksReplaced>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Eski kapı bloğunu yenisiyle değiştir, eskisini temizle".into(),
                input: json!({
                    "from": "01925f3e-7c1c-7b22-8d33-4c5e6f708192",
                    "to": "01925f3e-7c1c-7b22-8d33-4c5e6f708193",
                    "purge": true
                }),
                output: None,
            }],
        },
        // Blok çizelgesi (docs/adr/0317 §3), held together by fixtures/commands/v1/cad.blocks.query.json.
        CommandDescriptor {
            id: crate::CAD_BLOCKS_QUERY.into(),
            version: crate::CAD_BLOCKS_QUERY_VERSION,
            title: "Blokları say, bul ve çizelgele".into(),
            summary: "Çizimin blok yerleştirmelerini sayar (report count: çizimde, katmana göre ve başka blokların içinden), özniteliklerine göre bulur (instances: kalıcı kimlikler) ya da çizelgeler (schedule: Blok, Katman, X, Y, Dönüş ve özniteliklerin değerleri). \
                      blocks ve layers kapsamı daraltır; where koşulları (eq, ne, contains, empty, notEmpty, lt, gt) öznitelik değerine bakar: sabit öznitelikte tanımın değeri, öbürlerinde yerleştirmenin, yoksa tanımın varsayılanı; metin karşılaştırmaları Türkçe büyük küçük harfe bakmaz, lt ve gt sayı karşılaştırır. \
                      columns çizelgenin öznitelik sütunlarıdır (verilmezse blokların bütün etiketleri). Hiçbir şey yazılmaz. AutoCAD'in BCOUNT'u ve öznitelik çıkarması."
                .into(),
            aliases: vec!["bcount".into(), "bloksay".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::BlocksQuery>(),
            output: schema::<crate::BlocksQueried>(),
            plan: Some(schema::<crate::BlocksQueried>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kapıların sayısı".into(),
                    input: json!({ "report": "count", "blocks": ["01925f3e-7c1c-7b22-8d33-4c5e6f708192"] }),
                    output: None,
                },
                CommandExample {
                    title: "NO'su K- ile başlayan kapılar".into(),
                    input: json!({ "report": "instances", "where": [{ "tag": "NO", "op": "contains", "value": "K-" }] }),
                    output: None,
                },
            ],
        },
        // An insert's drawn objects (docs/adr/0319 §4), held together by fixtures/commands/v1/cad.blocks.proxy.json.
        CommandDescriptor {
            id: crate::CAD_BLOCKS_PROXY.into(),
            version: crate::CAD_BLOCKS_PROXY_VERSION,
            title: "Bloğun vekil görünüşü".into(),
            summary: "Yerleştirmelerin çizdiği nesneleri düz nesneler olarak verir: dinamik bir bloğun yerleştirmesinde tanımın nesneleri parametrelerin değerleriyle (eylemler sırayla, dizinin kopyaları yerel kimlikleriyle sonda, görünürlük durumunun gizledikleri yok), düz bir bloğunkinde tanımın kendisi. \
                      Nesneler tanımın koordinatlarındadır; iç içe yerleştirme yerleştirme kalır. default yerleştirmenin tanımı olduğu gibi çizip çizmediğini, values bütün değerleri söyler. \
                      Dinamik bloğu bilmeyen okuyucu (DXF) ve Sabit bloğa çevir bunu alır. Hiçbir şey yazılmaz."
                .into(),
            aliases: vec!["blokvekil".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::BlocksProxy>(),
            output: schema::<crate::BlocksProxied>(),
            plan: Some(schema::<crate::BlocksProxied>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Genişletilmiş kapının çizdiği nesneler".into(),
                input: json!({ "uids": ["01925f3e-7c1c-7b22-8d33-4c5e6f708193"] }),
                output: None,
            }],
        },
        // A dynamic block's inserts made plain (docs/adr/0319 §4), held together by
        // fixtures/commands/v1/cad.blocks.freeze.json.
        CommandDescriptor {
            id: crate::CAD_BLOCKS_FREEZE.into(),
            version: crate::CAD_BLOCKS_FREEZE_VERSION,
            title: "Sabit bloğa çevir".into(),
            summary: "Dinamik bir bloğun yerleştirmelerini çizdikleriyle aynı düz bloklara geçirir; tek geri alma adımı, adı Sabit bloğa çevir. \
                      Aynı blok ve aynı değerler tek yeni blok olur: nesneleri cad.blocks.proxy'ninkiler, taban noktası, birimi, öznitelik tanımları, açıklaması ve kartı kaynağınki, dinamik parçası yok. \
                      Yeni bloğun adı name (sonrakilere 2, 3 …) ya da kaynağın adı ve (sabit), alınmış adlar atlanarak. \
                      Yerleştirme yerini, ölçeğini, dönüşünü, katmanını, görünüşünü ve özniteliklerinin değerlerini korur, parametreleri kalkar. Kilitli katmandaki yerleştirme atlanır. \
                      Bağımsız geometriye çevirmek için Patlat (cad.entities.explode) aynı görünüşü nesnelere açar."
                .into(),
            aliases: vec!["bloksabitle".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::BlocksFreeze>(),
            output: schema::<crate::BlocksFrozen>(),
            plan: Some(schema::<crate::BlocksFrozen>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Geniş kapıyı sabit bloğa çevir".into(),
                input: json!({ "uids": ["01925f3e-7c1c-7b22-8d33-4c5e6f708193"], "name": "Kapı 100" }),
                output: None,
            }],
        },
        // A dynamic block's inserts' parameters (docs/adr/0318 §3), held together by
        // fixtures/commands/v1/cad.blocks.parameters.json.
        CommandDescriptor {
            id: crate::CAD_BLOCKS_PARAMETERS.into(),
            version: crate::CAD_BLOCKS_PARAMETERS_VERSION,
            title: "Blok parametreleri".into(),
            summary: "Dinamik bir bloğun yerleştirmelerinin parametrelerini okur (get), değiştirir (set) ya da varsayılanlarına döndürür (reset); yazma tek geri alma adımı, adı Blok parametreleri. \
                      values parametrelerin adlarıyla değerleridir: doğrusal ve açı bir sayı (uzunluk, radyan), kutupsal ve nokta bir {x, y}, çevir true ya da false, görünürlük durumun adı, tablo satırın adı (satırın değerlerini yazar). \
                      Her değer parametrenin kuralına uyar (en küçük, en büyük, adım, liste); ifadeyle hesaplanan parametre verilmez, öbürleri değişince yeniden hesaplanır. \
                      Yerleştirme yalnız varsayılandan farklı değerleri tutar; cevap her yerleştirmenin bütün değerlerini verir. Kilitli katmandaki yerleştirme atlanır. AutoCAD'in dinamik blok özellikleri."
                .into(),
            aliases: vec!["bparameter".into(), "blokparametre".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::BlocksParameters>(),
            output: schema::<crate::BlocksParametersOut>(),
            plan: Some(schema::<crate::BlocksParametersOut>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kapıyı genişlet ve çevir".into(),
                    input: json!({ "operation": "set", "uids": ["01925f3e-7c1c-7b22-8d33-4c5e6f708193"], "values": { "Genişlik": 1.0, "Yön": true } }),
                    output: None,
                },
                CommandExample {
                    title: "Parametreleri oku".into(),
                    input: json!({ "operation": "get", "uids": ["01925f3e-7c1c-7b22-8d33-4c5e6f708193"] }),
                    output: None,
                },
            ],
        },
        // Blok ekle (docs/adr/0315 §4), held together by fixtures/commands/v1/cad.blocks.insert.json.
        CommandDescriptor {
            id: crate::CAD_BLOCKS_INSERT.into(),
            version: crate::CAD_BLOCKS_INSERT_VERSION,
            title: "Blok ekle".into(),
            summary: "Bir bloğu kimliğiyle ya da adıyla at noktasına, layerId katmanına yerleştirir; tek geri alma adımı, adı Ekle. \
                      scale x ekseni boyunca, yScale y ekseni boyunca çarpandır (yScale verilmezse scale; eşit değilse daire elipse döner), rotation radyan saat yönünün tersine, mirror bloğun x ekseninde aynalar. \
                      Birimsiz blok çizimin birimini alır (1 birimi milimetre çizimde 1 mm); birimi olan blok gerçek boyutuyla yerleşir, birimi çizimin biriminden başkaysa söylenir (insert_units_differ). \
                      attributes öznitelik değerlerini etiketleriyle verir; verilmeyen öznitelik varsayılanını gösterir, blokta olmayan etiket reddedilir. \
                      Blok ekle aracı bu komutla yazar. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
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
            input: schema::<crate::BlocksInsert>(),
            output: schema::<crate::BlockInserted>(),
            plan: Some(schema::<crate::BlocksInsertPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kapı bloğunu adıyla, çeyrek dönüşle yerleştir".into(),
                    input: json!({
                        "name": "Kapı 90",
                        "at": { "x": 12.5, "y": 4.0 },
                        "rotation": std::f64::consts::FRAC_PI_2,
                        "layerId": "kapilar"
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1d-7c33-8e44-5d6f708192a3",
                        "id": 41,
                        "block": "01925f3e-7c1c-7b22-8d33-4c5e6f708192",
                        "unitFactor": 1.0,
                        "revision": "42"
                    })),
                },
                CommandExample {
                    title: "Pencereyi 1,5 geniş, 1 yüksek yerleştir: eşit olmayan ölçek".into(),
                    input: json!({
                        "block": "01925f3e-7c1c-7b22-8d33-4c5e6f708192",
                        "at": { "x": 3.0, "y": 0.0 },
                        "scale": 1.5,
                        "yScale": 1.0,
                        "layerId": "pencereler",
                        "attributes": { "NO": "P-3" }
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
                      Ağ, katmanlardan her analizde kurulan grafın tanımıdır: kenar katmanları (çizgi, çoklu çizgi ve yaylar; isteğe bağlı süzgeç ifadesi), düğüm katmanları (bağlantı, kaynak ya da vana; süzgeç ve kapalı ifadesi), bağlanma (ends: uçlarda, bir uç başka kenara değiyorsa orada; vertices: bütün köşelerde de) ve tolerans (m), yön (both, digitized ya da field: alanın ileri, geri ve kapalı değerleri), uzunluktan başka maliyetler (speed: hız alanından dakika, varsayılan hızla; field: kenarın bütününün maliyeti) ve kapalı kenarların ifadesi; ayrıca turns (dönüşler), levels (seviyeler), terminals (cihazların terminalleri: kenarların başlangıç ve bitiş terminal alanları, cihaz katmanı, terminalleri ve aralarındaki yollar) ve subnetworks (alt ağlar: ad alanı, yazılacak öznitelik, reach). \
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
        // Alt ağlar (docs/adr/0557 §2), held together by fixtures/commands/v1/cad.subnetworks.{find,update}.json.
        // The web is paused (docs/adr/0228): the desktop's host alone runs them for now.
        CommandDescriptor {
            id: crate::CAD_SUBNETWORKS_FIND.into(),
            version: crate::CAD_SUBNETWORKS_FIND_VERSION,
            title: "Alt ağları bul".into(),
            summary: "Bir şebekenin (tanımında subnetworks olan ağın) alt ağlarını bulur; hiçbir şeyi değiştirmez, plan aynı cevabı verir. \
                      Ağın Kaynak düğüm katmanları alt ağ denetleyicileridir; adı ağın name alanından, yoksa etiketinden, yoksa Ad özniteliğinden, yoksa Kaynak <no>'dan okunur (Türkçe büyük küçük harf ayrımsız aynı ad tek alt ağdır). \
                      Alt ağ, denetleyicinin ağın kurallarıyla (yön: reach downstream ise akış aşağı, connected ise her yön; kapalı kenar ve düğümler; cihazların terminal yolları) ulaştığı nesnelerdir; başka bir denetleyicide ve kapalı düğümde durur (o düğüm ulaşılmış sayılır, geçilmez). \
                      Cevap: alt ağ başına adı, denetleyicileri, kenar ve düğüm nesnesi sayıları, uzunluğu, (members ile) nesnelerinin kalıcı kimlikleri ve ağın Denetle bulgularından geçtiyse sayısı (unverified: güvenilir sayılmaz); hiçbir denetleyicinin ulaşmadığı nesneler (orphans); farklı adlı iki denetleyicinin ulaştığı nesneler (shared); ağın subnetworks.attribute özniteliğinin söylediğiyle farkı: farklı, eksik ya da artakalan değerli nesneler (stale; hepsi boşsa güncel). \
                      Alt ağlar çizimden her seferinde hesaplanır; saklanan bir kopya yoktur, özniteliği Alt ağları güncelle yazar."
                .into(),
            aliases: vec!["altagbul".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SubnetworksFind>(),
            output: schema::<crate::SubnetworksFound>(),
            plan: Some(schema::<crate::SubnetworksFound>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Elektrik şebekesinin alt ağları, nesnelerinin kimlikleriyle".into(),
                input: json!({ "network": "ag-1", "members": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_SUBNETWORKS_UPDATE.into(),
            version: crate::CAD_SUBNETWORKS_UPDATE_VERSION,
            title: "Alt ağları güncelle".into(),
            summary: "Ağın alt ağlarının (Alt ağları bul'un bulduğu) adlarını ağın subnetworks.attribute özniteliğine, ağın her nesnesine yazar: iki alt ağda olan nesnede adlar “; ” ile birleşir, hiçbirinde olmayan nesnenin değeri alınır. \
                      Değeri zaten doğru olan nesneye dokunulmaz. Kilitli katmandakiler, alanı olan ve bu alanı taşımayan katmandakiler ve alanın kuralına uymayan değerler yazılmaz, sayılır ve uyarıyla söylenir (subnetwork_layer_skipped, values_skipped). \
                      Tek geri alma adımıdır (“Alt ağları güncelle”). Plan yazmadan aynısını söyler. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["altaguncelle".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SubnetworksUpdate>(),
            output: schema::<crate::SubnetworksUpdated>(),
            plan: Some(schema::<crate::SubnetworksUpdated>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Alt ağ adlarını özniteliğe yaz".into(),
                input: json!({ "network": "ag-1" }),
                output: None,
            }],
        },
        // Olayları rotalarına bağla (docs/adr/0569), held together by fixtures/commands/v1/cad.linearref.define.json.
        CommandDescriptor {
            id: crate::CAD_LINEARREF_DEFINE.into(),
            version: crate::CAD_LINEARREF_DEFINE_VERSION,
            title: "Olay bağı".into(),
            summary: "Projenin olay bağlarına bir bağ yazar (set: aynı kimlikli bağın yerini alır, yoksa sona eklenir) ya da birini siler (remove). \
                      Bağ, bir olay katmanının (noktalar ya da çizgiler ve çoklu çizgiler) rotalarına bağlanışıdır: rota katmanları (1–16) ve rotaların kimlik alanı, olayların rota alanı, nokta olayı için ölçü alanı (kind point) ya da çizgi olayı için başlangıç ve bitiş ölçüsü alanları (kind line), isteğe bağlı öteleme alanı ve yönü (side: right ya da left), tolerans (m, varsayılan 0,001). \
                      Bağ olayları rotalarında güncel tutar: rota katmanlarının ya da olay katmanının bir nesnesine dokunan her yazma (rotanın geometrisi, M'si ya da kimliği; olayın rota ya da ölçü öznitelikleri; olayın kendisi), aynı adımda, olay nesnelerini Olayları rotada göster'in kuralıyla rotanın verdiği yere koyar (Olay kaymasını denetle'nin Yerine taşı'sı gibi): nokta noktaya, çizgi rotanın parçasına. Geri alma yazmayı ve olayların yerlerini birlikte geri alır. \
                      Bir yazma bu yüzden hiçbir zaman reddedilmez: rotası olmayan, ölçüsü sayı olmayan ya da rotanın dışında kalan olay, kilitli olay katmanı ve kırıklı yola taşınamayan çizgi olayı olduğu yerde kalır ve yazmanın cevabına events_dirty uyarısı girer. \
                      Bağ proje ayarıdır: yazmak çizimi kirli yapar, geri alma adımı değildir ve olayları kendiliğinden taşımaz; bağdan sonraki yazmalar taşır. Çizimde olmayan katmanı adlandıran bağ yazılır, unknown_layer uyarısıyla. \
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
            input: schema::<crate::LinearRefDefine>(),
            output: schema::<crate::LinearRefDefined>(),
            plan: Some(schema::<crate::LinearRefDefined>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Kaza noktaları D-100 rotasında kilometreleriyle kalsın".into(),
                    input: json!({
                        "operation": "set",
                        "link": {
                            "id": "kazalar",
                            "name": "Kazalar",
                            "events": "kaza",
                            "kind": "point",
                            "routes": ["yol"],
                            "routeField": "Yol",
                            "eventRoute": "Yol",
                            "measure": "Km"
                        }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Olay bağını sil".into(),
                    input: json!({ "operation": "remove", "id": "kazalar" }),
                    output: None,
                },
            ],
        },
        // Topolojiyi denetle (docs/adr/0589), held together by fixtures/commands/v1/cad.topology.check.json.
        CommandDescriptor {
            id: crate::CAD_TOPOLOGY_CHECK.into(),
            version: crate::CAD_TOPOLOGY_CHECK_VERSION,
            title: "Topolojiyi denetle".into(),
            summary: "Projenin topoloji kurallarını (rules: bazılarının kimlikleri; verilmezse hepsi) çizimde çalıştırır; çizimi değiştirmez. \
                      Bağlı katmanın alan sınırlı parçasında (ADR 0510) alanın çevresindeki bant kaynağından okunur (dosyanın katmanın son eşleştiği sürümünden, veritabanının bugünkü hâlinden) ve komşu sayılır: parçanın nesnesiyle dışarıdaki satır arasındaki bulgu bulunur, dışarıdakilerin kendi aralarındaki bulgu parçanın değildir, verilmez. \
                      Komşular tam okunamazsa (dosya yok ya da başka sürümde, veritabanına ulaşılamıyor, parçanın nesnesi bandın dışına uzanıyor) complete false olur, incomplete nedenini söyler ve topology_incomplete uyarısı verilir: o katman temiz sayılmaz. \
                      Her bulgu kalıcı anahtarıyla (key), kuralıyla, sorunuyla, nesnelerinin kalıcı kimlikleriyle (dışarıdakiler outside'ta), yeri, ölçüsü, düzeltmeleri ve istisna durumuyla gelir; ilk limit tanesi (1–100 000, varsayılan 1 000) listelenir, sayılar hepsinindir. \
                      Geometri özetiyle yapılmış istisna, nesnelerin geometrisi değişince düşer: bulgu açık ve lapsed olur (exception_lapsed uyarısı). \
                      stamp denetimin gördüğü durumdur: çizimin revizyonu, kuralların, toleransın ve istisnaların özeti, bağlı katmanların kaynak sürümü ve okunan bandın özeti. \
                      preview bir bulgunun bir düzeltmesini yazmadan önizler: değişecek nesneler alanları ya da uzunluklarıyla önce ve sonra, silinecekler, kaynağına yazılacak bağlı katmanlar (yazmak cad.entities.edit'in topologyFix'idir). \
                      Retler: invalid_limit, topology_rule_not_found, finding_not_found, fix_unavailable."
                .into(),
            aliases: vec!["topolojidenetle".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::TopologyCheck>(),
            output: schema::<crate::TopologyChecked>(),
            plan: Some(schema::<crate::TopologyChecked>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Projenin bütün topoloji kurallarını denetle".into(),
                    input: json!({}),
                    output: None,
                },
                CommandExample {
                    title: "Bir kuralı denetle, ilk 50 bulgu".into(),
                    input: json!({ "rules": ["kural-1"], "limit": 50 }),
                    output: None,
                },
                CommandExample {
                    title: "Bir bulgunun düzeltmesini önizle".into(),
                    input: json!({ "preview": { "key": "3f9a0c12d4e5b678", "fix": "subtractFirst" } }),
                    output: None,
                },
            ],
        },
        // Topoloji istisnaları (docs/adr/0589 §3), held together by fixtures/commands/v1/cad.topology.exceptions.json.
        CommandDescriptor {
            id: crate::CAD_TOPOLOGY_EXCEPTIONS.into(),
            version: crate::CAD_TOPOLOGY_EXCEPTIONS_VERSION,
            title: "Topoloji istisnaları".into(),
            summary: "Bulguları istisna yapar (mark) ya da istisnalarını kaldırır (unmark); bulgular cad.topology.check'in verdiği kalıcı anahtarlarla (keys) ve çizimin bugünkü hâliyle anılır. \
                      İstisna bulgunun kuralını, nesnelerini, yerini ve nesnelerin geometrisinin özetini tutar: geometri değişince istisna düşer, bulgu yeniden açılır; düşmüş istisnanın bulgusunu mark bugünkü geometriyle yeniden istisna yapar, unmark kaldırır. \
                      Zaten istisna olan (mark) ya da istisnası olmayan (unmark) bulgu sayılmaz. \
                      İstisnalar proje ayarıdır: yazmak çizimi kirli yapar, geri alma adımı değildir. \
                      Retler: invalid_input (anahtar yok ya da iki kez), finding_not_found; expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::TopologyExceptions>(),
            output: schema::<crate::TopologyExceptionsEdited>(),
            plan: Some(schema::<crate::TopologyExceptionsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki bulguyu istisna yap".into(),
                    input: json!({ "operation": "mark", "keys": ["3f9a0c12d4e5b678", "0a1b2c3d4e5f6071"] }),
                    output: None,
                },
                CommandExample {
                    title: "Bir bulgunun istisnasını kaldır".into(),
                    input: json!({ "operation": "unmark", "keys": ["3f9a0c12d4e5b678"] }),
                    output: None,
                },
            ],
        },
        // Adres verisini denetle (docs/adr/0579), held together by fixtures/commands/v1/cad.addresses.check.json.
        CommandDescriptor {
            id: crate::CAD_ADDRESSES_CHECK.into(),
            version: crate::CAD_ADDRESSES_CHECK_VERSION,
            title: "Adres verisini denetle".into(),
            summary: "Projenin adres verisini (locator: kimliği; verilmezse projenin ilki) denetler; çizimi değiştirmez. \
                      Dizinin tuttuklarını sayar (kapılar, yollar, yol çizgileri, mahalleler, bölgeler), dışarıda kalanları söyler (nokta olmayan kapı, yol adı olmayan kapı, kapı numarası okunmayan kapı, çizgi olmayan yol, adsız çizgi ve bölge) \
                      ve üç listeyi verir: aynı yolda aynı kapı numarası (repeats), birden çok kapıdaki adres kodu (codes) ve kapıları yolun adını birden çok biçimde yazan yollar (spellings: her yazılış ve onu yazan kapı sayısı, en çok yazılan önce; adresin normalleştirilmesinin bulgusu). \
                      Mahalle ve ilçe adlarının yazılışları da (neighbourhoodSpellings: mahallenin ve ilçenin anahtarıyla; districtSpellings: ilçenin anahtarıyla; within mahallenin ilçesi); cad.addresses.normalize onları tek yazılışa getirir. \
                      Yollar adın Türkçe katlanmış biçimi, mahalle ve ilçeyle gruplanır (“Atatürk Cad.” ile “ATATÜRK CADDESİ” bir yoldur). \
                      Her liste önce sayısını, sonra ilk kapılarına göre ilk limit tanesini (1–100, varsayılan 100) nesnelerin kalıcı kimlikleriyle verir. \
                      Retler: invalid_limit, no_address_data, unknown_locator."
                .into(),
            aliases: vec!["adresdenetle".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::AddressesCheck>(),
            output: schema::<crate::AddressesChecked>(),
            plan: Some(schema::<crate::AddressesChecked>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Projenin adres verisini denetle".into(),
                    input: json!({}),
                    output: None,
                },
                CommandExample {
                    title: "Numarataj'ı denetle, her listeden ilk 20".into(),
                    input: json!({ "locator": "adres-1", "limit": 20 }),
                    output: None,
                },
            ],
        },
        // Adres kaydıyla karşılaştır (docs/adr/0736), held together by fixtures/commands/v1/cad.addresses.compare.json.
        CommandDescriptor {
            id: crate::CAD_ADDRESSES_COMPARE.into(),
            version: crate::CAD_ADDRESSES_COMPARE_VERSION,
            title: "Adres kaydıyla karşılaştır".into(),
            summary: "Projenin adres verisinin (locator; verilmezse ilki) kapılarını bir adres kaydıyla (kurumun ya da Ulusal Adres Veri Tabanı'nın adres kodu tablosu, UAVT) karşılaştırır; çizimi değiştirmez. \
                      Kayıt bir tablo dosyası (register.path: CSV ya da metin, XLSX, ODS; register.sheet çalışma kitabının sayfası) ya da çizimin bir katmanıdır (register.layer: nesnelerin öznitelikleri sütunlar, noktası yeri); ilk satır sütunları adlandırır. \
                      register.code adres kodunun sütunudur; street, door, neighbourhood, district, x ve y verildikleri yerde karşılaştırılır (mahalle ve ilçe yalnız kapılarda o alan varsa; x ve y projenin koordinatlarında). \
                      Bulgular: kodu kayıtta olmayan kapı (unregisteredDoors); kodu bir satırın ama adresi başka yazılan kapı (differences: yol, kapı no, mahalle, ilçe; adresin anahtarlarıyla karşılaştırılır, “Atatürk Blv.” ile “ATATÜRK BULVARI” aynıdır, yolun türü ikisi de söylüyorsa); satırın yerinden tolerance metreden (varsayılan 10) uzak kapı (distances); \
                      kodu hiçbir kapıda olmayan ve yolunda o numarada kapı bulunmayan kayıt adresleri (undrawnAddresses: bir binanın birden çok bağımsız bölümü tek adres, kodlarıyla); kayıtta birden çok satırdaki kod (registerRepeats: kapı ilk satırla karşılaştırılır). \
                      Her liste önce sayısını, sonra ilk limit tanesini (1–10 000, varsayılan 100) kapıların kalıcı kimlikleriyle, kayıt satırlarını sıra numaralarıyla (katmanda nesnelerin kimlikleriyle de) verir. \
                      Retler: invalid_limit, invalid_tolerance, register_missing, invalid_input, no_address_data, unknown_locator, no_address_codes, invalid_file_path, file_not_found, register_unreadable, layer_not_found, register_column_missing. Uyarı: register_cut (250 000 satırdan çok)."
                .into(),
            aliases: vec!["adreskaydi".into(), "uavtkarsilastir".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::AddressesCompare>(),
            output: schema::<crate::AddressesCompared>(),
            plan: Some(schema::<crate::AddressesCompared>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Belediyenin UAVT tablosuyla karşılaştır".into(),
                    input: json!({ "register": { "path": "/kayit/uavt.csv", "code": "ADRES_NO", "street": "YOL", "door": "DIS_KAPI", "neighbourhood": "MAHALLE", "x": "X", "y": "Y" } }),
                    output: None,
                },
                CommandExample {
                    title: "Çizimdeki kayıt katmanıyla, 25 m toleransla".into(),
                    input: json!({ "register": { "layer": "kayit", "code": "UAVT", "street": "YOL_ADI", "door": "KAPI_NO" }, "tolerance": 25.0 }),
                    output: None,
                },
            ],
        },
        // Adres yazılışlarını düzelt (docs/adr/0647 §2), held together by fixtures/commands/v1/cad.addresses.normalize.json.
        CommandDescriptor {
            id: crate::CAD_ADDRESSES_NORMALIZE.into(),
            version: crate::CAD_ADDRESSES_NORMALIZE_VERSION,
            title: "Adres yazılışlarını düzelt".into(),
            summary: "Projenin adres verisinin (locator; verilmezse ilki) kapılarının yol (what: street), mahalle (neighbourhood) ya da ilçe (district) adını her grupta tek yazılışa getirir; tek geri alma adımıdır (“Adres yazılışlarını düzelt”). \
                      Gruplar cad.addresses.check'inkilerdir: yolun kapıları; mahallenin ve ilçenin anahtarıyla gruplanan kapılar; ilçenin anahtarıyla gruplanan kapılar; yalnız adı birden çok biçimde yazılanlar. \
                      Her grup, prefer'deki yazılışlardan yazdığı ilkini, yoksa en çok yazılanını (eşitlikte önce yazılan) alır; kırpılmış adı başka olan her kapının kapı katmanındaki alanına o yazılır, alanın kuralından geçerek. \
                      Cevap grupları (to, from, doors), yazılan kapı sayısını ve kalıcı kimliklerini verir; yazılacak kapı yoksa adım açılmaz. \
                      Retler: invalid_revision, revision_conflict, no_address_data, unknown_locator, invalid_input (kapılarda o alan yok, boş tercih), layer_locked, alanın kuralına uymayan yazılış (invalid_attribute)."
                .into(),
            aliases: vec!["adresduzelt".into(), "yazilisduzelt".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::AddressesNormalize>(),
            output: schema::<crate::AddressesNormalized>(),
            plan: Some(schema::<crate::AddressesNormalized>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yolların adlarını en çok yazılana getir".into(),
                    input: json!({}),
                    output: None,
                },
                CommandExample {
                    title: "Mahalle adlarında “Kızılay Mahallesi” yazılışını tercih et".into(),
                    input: json!({ "what": "neighbourhood", "prefer": ["Kızılay Mahallesi"] }),
                    output: None,
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
        // Sistem ata (docs/adr/0583 §2), held together by fixtures/commands/v1/cad.layers.crs.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_CRS.into(),
            version: crate::CAD_LAYERS_CRS_VERSION,
            title: "Sistem ata".into(),
            summary: "Katmanın kaynak sistemi kaydını yazar ya da siler (srid verilmezse ya da null ise): nesnelerinin hangi koordinat sisteminden geldiği; adı, WKT2'si, EPSG eksen sırası, birimi ve kullanım alanı kayıttan. \
                      Nesneler her zaman projenin sistemindedir; atama hiçbir koordinatı değiştirmez (yanlış sistemde alınmış veriyi çevirmek için cad.layers.reproject). \
                      Katman ağacının değişikliğidir: tek geri alma adımı “Sistem ata”; kayıt aynıysa hiçbir şey yazılmaz. Kilitli katmanda da çalışır, veri değişmez. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["sistemata".into(), "assigncrs".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersCrs>(),
            output: schema::<crate::LayersCrsSet>(),
            plan: Some(schema::<crate::LayersCrsSet>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Katmanın verisi WGS 84'ten geldi".into(),
                    input: json!({ "layer": "gps", "srid": 4326 }),
                    output: None,
                },
                CommandExample {
                    title: "Kaydı sil".into(),
                    input: json!({ "layer": "gps" }),
                    output: None,
                },
            ],
        },
        // Yeniden izdüşür (docs/adr/0583 §2), held together by fixtures/commands/v1/cad.layers.reproject.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_REPROJECT.into(),
            version: crate::CAD_LAYERS_REPROJECT_VERSION,
            title: "Yeniden izdüşür".into(),
            summary: "Katmanın nesnelerini verilen sistemden (from; verilmezse katmanın kaynak sistemi) projenin sistemine çevirir: projenin sisteminde sanılarak alınmış veriyi düzeltir. \
                      Kotlar ve ölçüler kalır; yay ve eğriler dönüşümün kuralıyla çevrilir. Çevrilemeyen nesne varsa hiçbir şey yazılmaz (crs_unreached, uids). \
                      Katmanın kaynak sistemi from olur. Tek geri alma adımı “Yeniden izdüşür”. Kilitli katman reddedilir; projenin sistemi yoksa ya da from projeninkiyse crs_mismatch. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["izdusur".into(), "reproject".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersReproject>(),
            output: schema::<crate::LayersReprojected>(),
            plan: Some(schema::<crate::LayersReprojected>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "ED50 / TM36'da sanılan katmanı projenin sistemine çevir".into(),
                input: json!({ "layer": "eski-pafta", "from": 2322 }),
                output: None,
            }],
        },
        // Koşullu biçim (docs/adr/0413), held together by fixtures/commands/v1/cad.layers.table.json.
        // The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_LAYERS_TABLE.into(),
            version: crate::CAD_LAYERS_TABLE_VERSION,
            title: "Koşullu biçim".into(),
            summary: "Öznitelik tablosunun katmanın satırlarını ve hücrelerini koşullarla nasıl göstereceğini yazar (QGIS'in koşullu biçimi). Her kural: koşul (condition; katmanın alanları adlarıyla, ifade dili), uygulandığı yer (field: bir alanın hücresi; verilmezse bütün satır), zemin rengi (background), yazı rengi (color, #RRGGBB) ve kalınlık (bold). \
                      Bir hücrenin her biri için tutan ilk kural verir; alanın kuralları satırınkilerden önce gelir. En çok 50 kural. Kuralların listesi verilen olur; boş liste kaldırır. \
                      Katman ağacının değişikliğidir: tek geri alma adımı “Koşullu biçim”; aynıysa hiçbir şey yazılmaz. \
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
            input: schema::<crate::LayersTable>(),
            output: schema::<crate::LayersComposed>(),
            plan: Some(schema::<crate::LayersComposePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Hasarlı binalar kırmızı, kat sayısı yüksekse kalın".into(),
                input: json!({ "layer": "bina", "styles": [
                    { "condition": "Durum = 'kotu'", "background": "#FFE4E1", "color": "#B91C1C" },
                    { "field": "Kat", "condition": "Kat > 5", "bold": true }
                ] }),
                output: None,
            }],
        },
        // Kayıt kuralları (docs/adr/0438 §1), held together by fixtures/commands/v1/cad.layers.rules.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_RULES.into(),
            version: crate::CAD_LAYERS_RULES_VERSION,
            title: "Kayıt kuralları".into(),
            summary: "Katmanın kayıt kurallarını yazar: veri kümesinin kendi kısıtları (id, name, rule: kaydın alanlarını adlarıyla, $alan'ı ve @ değerlerini okuyan ifade, \
                      message, isteğe bağlı warning ve fields: formun kuralı gösterdiği alanların anahtarları). Kural, kaynağı ne olursa olsun (form, tablo, komut, \
                      içe aktarma, Python, ajan) katmanın her yazmasında sorulur: okuduğu alanları değiştiren yazma kuralı bozarsa reddedilir (rule_broken), uyarı kuralı \
                      söylenir (rule_unmet); boş cevap (okunan alan boş) kuralı bozmaz. Form kuralı kendi alanında gösterir. Boş liste kaldırır; tek geri alma adımı “Kayıt kuralları”."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersRules>(),
            output: schema::<crate::LayersComposed>(),
            plan: Some(schema::<crate::LayersComposePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bitiş tarihi başlangıçtan önce olamaz".into(),
                input: json!({ "layer": "ruhsat", "rules": [{ "id": "tarih-sirasi", "name": "Tarih sırası",
                    "rule": "Bitiş >= Başlangıç", "message": "Bitiş tarihi başlangıçtan önce olamaz.", "fields": ["Bitiş"] }] }),
                output: None,
            }],
        },
        // Dizinler (docs/adr/0758), held together by fixtures/commands/v1/cad.layers.indexes.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_INDEXES.into(),
            version: crate::CAD_LAYERS_INDEXES_VERSION,
            title: "Dizinler".into(),
            summary: "Katmanın dizinlerini yazar (ArcGIS'in öznitelik dizini, veritabanının CREATE INDEX'i): her dizin (name) 1–32 alanı (fields, \
                      anahtarlarıyla, sırayla) tutar; tekil olanı (unique) katmanın anahtarıdır: alanlarında başka bir kaydın değerlerini veren yazma, \
                      kaynağı ne olursa olsun (form, tablo, komut, içe aktarma, Python, ajan) reddedilir (not_unique). Boş değer değer sayılmaz; \
                      nullsEqual boşları aynı sayar (PostgreSQL'in NULLS NOT DISTINCT'i). Kayıtların zaten bozduğu yeni tekil dizin reddedilir \
                      (index_not_unique, kayıtların kimlikleriyle). GeoPackage'a yazılan katmanın tablosu dizinlerini alır, okunan tablonun dizinleri \
                      katmana gelir. Boş liste kaldırır; tek geri alma adımı “Dizinler”."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersIndexes>(),
            output: schema::<crate::LayersComposed>(),
            plan: Some(schema::<crate::LayersComposePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ada ve parsel numarası birlikte tekil, mahalle aranır".into(),
                input: json!({ "layer": "parsel", "indexes": [
                    { "name": "Ada ve parsel", "fields": ["Ada", "Parsel"], "unique": true },
                    { "name": "Mahalle", "fields": ["Mahalle"] }
                ] }),
                output: None,
            }],
        },
        // Değer bileşimleri (docs/adr/0423), held together by fixtures/commands/v1/cad.layers.contingents.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_CONTINGENTS.into(),
            version: crate::CAD_LAYERS_CONTINGENTS_VERSION,
            title: "Değer bileşimleri".into(),
            summary: "Katmanın değer gruplarını yazar (ArcGIS'in contingent values'u): her grup (name) 2–8 değer listeli alanı (fields, anahtarlarıyla) \
                      ve birlikte geçerli bileşimlerini (rows: cells, alan başına kind code ve code, kind any ya da kind empty; isteğe bağlı subtype) tutar. \
                      Grubun alanlarını değiştiren yazma, bileşimlerden birine uymuyorsa kısıtlayıcı grupta (restrictive) reddedilir (invalid_combination), \
                      öbüründe uyarılır (combination_unmet); formun ve tablonun seçenekleri öbür değerlere göre daralır. Boş liste kaldırır; tek geri alma adımı “Değer bileşimleri”."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersContingents>(),
            output: schema::<crate::LayersComposed>(),
            plan: Some(schema::<crate::LayersComposePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Konut her malzemeyle, sanayi yalnız betonarme ya da çelikle".into(),
                input: json!({ "layer": "bina", "groups": [{ "name": "Yapı", "fields": ["Tür", "Malzeme"], "restrictive": true, "rows": [
                    { "cells": [{ "kind": "code", "code": "konut" }, { "kind": "any" }] },
                    { "cells": [{ "kind": "code", "code": "sanayi" }, { "kind": "code", "code": "beton" }] },
                    { "cells": [{ "kind": "code", "code": "sanayi" }, { "kind": "code", "code": "celik" }] }
                ] }] }),
                output: None,
            }],
        },
        // Alt türler (docs/adr/0422), held together by fixtures/commands/v1/cad.layers.subtypes.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_SUBTYPES.into(),
            version: crate::CAD_LAYERS_SUBTYPES_VERSION,
            title: "Alt türler".into(),
            summary: "Katmanın alt türlerini yazar (ArcGIS'in subtype'ı): değer listeli tek değerli bir alan (field, alanın anahtarı) kayıtları türlerine ayırır; \
                      her alt tür (types: code, listenin bir değeri; label) öbür alanları o türün kayıtları için daraltır: sunulan değerler (values), yeni kaydın \
                      varsayılanı (default), zorunluluk (required). Alt türün sunmadığı değer kayda yeniden yazılamaz; tür değişince eski değerler kalır ve \
                      subtype_misfit uyarısıyla söylenir. subtypes verilmezse kaldırılır. Tek geri alma adımı “Alt türler”; aynıysa hiçbir şey yazılmaz."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersSubtypes>(),
            output: schema::<crate::LayersComposed>(),
            plan: Some(schema::<crate::LayersComposePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Sanayi binası yalnız betonarme ya da çelik olur".into(),
                input: json!({ "layer": "bina", "subtypes": { "field": "Tür", "types": [
                    { "code": "konut" },
                    { "code": "sanayi", "fields": [
                        { "field": "Malzeme", "values": ["beton", "celik"], "default": "celik", "required": true }
                    ] }
                ] } }),
                output: None,
            }],
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
        // Etiket denetimi (docs/adr/0533 §6), held together by fixtures/commands/v1/cad.labels.report.json.
        CommandDescriptor {
            id: crate::CAD_LABELS_REPORT.into(),
            version: crate::CAD_LABELS_REPORT_VERSION,
            title: "Etiketleri denetle".into(),
            summary: "Bir pencerenin etiketlerini bir ölçekte etiket motoruyla yerleştirir ve sayar; çizimi değiştirmez. Ölçek pxPerMetre (görünümün ölçeği: ekran pikseli bölü metre) ya da paperScale (1:N: bir pafta haritasının o ölçekte yazdığı gibi, kâğıdın CSS pikseliyle) biridir; pencere window (batı, güney, doğu, kuzey) ya da çizimin kapsamıdır; layers yalnız bu katmanların etiketlerini sayar (öbürleri yine de yer tutar ve engeldir). \
                      Cevap toplamlar ve katman katman sayılar: placed (yeri olan), unplaced (boş yer bulunamayan; çizilmez), hidden (elle gizlenen), pinned (yeri sabitlenen), forced (üst üste gelebilen sınıfın bindirdiği), collisions (yerleştirilenlerden kutusu başka bir yerleştirilenin kutusunu kesen; çizildiği gibi okunur, eğri yazı kutulanmaz: üst üste gelebilen sınıf yoksa 0), repeats (aynı katmanda ve sınıfta daha önce yerleşmiş bir etiketin metnini yineleyen: bir yolun adı boyunca yinelenir). \
                      Bir kaynağın parçasını tutan bağlı katmanın (cad.sources.edit extent) etiketi ve engeli yalnız o parçanın satırlarıdır: katmanın source ve part'ı söylenir, pencere parçanın ötesine uzanıyorsa partial true olur. \
                      Bir etiket motorunun sonucu görünüme göre değişir: bu komut tek bir pencereyi sayar; ardışık küçük kaydırmalarda etiketlerin yerinde kalması masaüstünün işidir (docs/adr/0533 §3), bu komutun değil."
                .into(),
            aliases: vec!["etiketdenetle".into(), "labelreport".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::LabelsReport>(),
            output: schema::<crate::LabelsReported>(),
            plan: Some(schema::<crate::LabelsReported>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Çizimin tamamını 1:1000 pafta ölçeğinde denetle".into(),
                    input: json!({ "paperScale": 1000 }),
                    output: None,
                },
                CommandExample {
                    title: "Parsel katmanını 2 piksel/metrede bir pencerede denetle".into(),
                    input: json!({ "pxPerMetre": 2.0, "window": [487000.0, 4420000.0, 487250.0, 4420180.0], "layers": ["parsel"] }),
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
                      merge kaynak katmanların (layers) nesnelerini hedef katmana (layer) taşır ve kaynakları siler, etkin katman kaynaksa hedef etkin olur; fields katmanın alanlarını yazar ve renames ile nesnelerinin anahtarlarını yeniden adlandırır, alanın adının geçtiği yerler de (cad.fields.usage'ın yerleri, görünüş dahil) aynı adımda yeni adı alır; \
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
                      active (layer: nesne alan, dondurulmamış bir katman; grup ve servis katmanı olmaz), isolate (layers: bu düğümler, üstlerindeki gruplar ve altlarındakiler görünür, gerisi gizlenir; gizlediklerini çizim oturum boyunca hatırlar), \
                      unisolate (isolate'in gizledikleri, hâlâ gizliyse yeniden görünür), showAll (hepsi görünür; dondurulmuş düğüm dondurulmuş kalır), plot (layer, value: yazdırılır mı), frozen (layer, value: dondurulmuş düğüm çizilmez, seçilmez, kenetlenmez, kapsama sayılmaz; etkin katman dondurulamaz) ve thawAll (hepsi çözülür). \
                      Bunlar ağacın kendi değişiklikleridir: çizimi kirli yapar ve sürümü değiştirir ama geri alma adımı değildir; etkin katman dosyada saklanır ama düzenleme sayılmaz; yalnız isolate'in hatırladıkları oturumundur. Değişmeyen hiçbir şey yazmaz (changed false). \
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
                      katmanların stilleri tek geri alma adımında (“Katman durumu: <ad>”) yazılır, saklandıysa dondurma ve yazdırma görünürlük gibi; ağaçta olmayan düğümler missing'de sayılır. \
                      export durumları (states, verilmezse hepsi) düğümlerinin yollarıyla bir dosya olarak verir (file; kentos.layer-states sürüm 1), çizimi değiştirmez; import dosyanın (file) durumlarını ekler: düğüm kimliğiyle, yoksa yoluyla (adlar büyük küçük harf ve Türkçe harfler katlanarak) bulunur; \
                      projede adı olan durum atlanır ya da replace ile üzerine yazılır (kimliği kalır), yenisi ilk boş durum-n kimliğini alır. Durumlar proje ayarıdır: kaydetmek, güncellemek, adlandırmak, silmek ve içe aktarmak çizimi kirli yapar, geri alma adımı değildir. \
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
                    output: Some(json!({ "state": "durum-1", "changed": true, "visible": 0, "locked": 0, "styles": 0, "frozen": 0, "plot": 0, "missing": 0, "revision": "15" })),
                },
                CommandExample {
                    title: "Pafta görünümüne dön".into(),
                    input: json!({ "operation": "apply", "state": "durum-1" }),
                    output: None,
                },
                CommandExample {
                    title: "Ofisin durumlarını başka bir çizimden al; aynı adlıların üzerine yaz".into(),
                    input: json!({ "operation": "import", "replace": true,
                                   "file": { "format": "kentos.layer-states", "version": 1,
                                             "states": [{ "id": "durum-1", "name": "Pafta",
                                                          "nodes": [{ "node": "layer-4", "path": "Mimari / A-DUVAR", "visible": true, "frozen": false, "plot": true }] }] } }),
                    output: None,
                },
            ],
        },
        // Alanın kullanıldığı yerler (docs/adr/0414 §3), held together by fixtures/commands/v1/cad.fields.usage.json.
        CommandDescriptor {
            id: crate::CAD_FIELDS_USAGE.into(),
            version: crate::CAD_FIELDS_USAGE_VERSION,
            title: "Alanın kullanıldığı yerler".into(),
            summary: "Bir katman alanının adının geçtiği yerler: alanın katmanının formlarının taslağı ve yayımlanmış sürümleri (öğe, koşullar, hesap, varsayılan, seçenek süzgeci, denetimler, notun metni), \
                      başka formların bu katmandan listeleri ve ilişkili kayıtları, ilişkiler, katmanın koşullu biçimi, süzgeci, görünüşü (işleyicinin ifadeleri, kuralların süzgeçleri, katmanda tanımlı simgelerin veriye bağlı değerleri; kitaplığın simgeleri değil), etiketleri, zamanı, bilgi kartı ve eylemleri. \
                      Her yerin Türkçe adı, yolu, alanı nasıl andığı (anahtar, ad, ifade, şablon) ve yayımlanmış sürümde olup olmadığı. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::FieldsUsage>(),
            output: schema::<crate::FieldsUsed>(),
            plan: Some(schema::<crate::FieldsUsed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bina katmanının Kat alanı nerede kullanılıyor".into(),
                input: json!({ "layer": "bina", "field": "Kat" }),
                output: None,
            }],
        },
        // Katmanları bul (docs/adr/0311 §4), held together by fixtures/commands/v1/cad.layers.find.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_FIND.into(),
            version: crate::CAD_LAYERS_FIND_VERSION,
            title: "Katmanları bul".into(),
            summary: "Katmanları adlarının örüntüsüyle ve özellikleriyle bulur (AutoCAD'in katman süzgeçleri): name AutoCAD'in wcmatch örüntüsüdür (* herhangi bir dizi, ? bir karakter, # rakam, @ harf, . ikisi de değil, [..] ve [~..] kümeler, ` kaçış, virgülle seçenekler, başta ~ değili; harfler Türkçe katlanır); \
                      group yalnız o grubun altındakiler; color, lineType, lineWeight katmanın kendi stili; on görünür (gözü ve üstündeki grupların gözü açık), frozen ve locked kendisi ya da üstündeki bir grup, plot üstündekilerle birlikte yazdırılıyor, empty nesnesiz (servis katmanı boş sayılmaz). \
                      Yalnız katmanlar bulunur, ağacın sırasıyla, yolları, nesne sayıları ve özellikleriyle. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersFind>(),
            output: schema::<crate::LayersFound>(),
            plan: Some(schema::<crate::LayersFound>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Adı A- ile başlayan, nesnesiz katmanlar".into(),
                    input: json!({ "name": "A-*", "empty": true }),
                    output: None,
                },
                CommandExample {
                    title: "Dondurulmuş ya da yazdırılmayan duvar katmanları".into(),
                    input: json!({ "name": "*duvar*", "frozen": true }),
                    output: None,
                },
            ],
        },
        // Katmanları çevir (docs/adr/0311 §5), held together by fixtures/commands/v1/cad.layers.translate.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_TRANSLATE.into(),
            version: crate::CAD_LAYERS_TRANSLATE_VERSION,
            title: "Katmanları çevir".into(),
            summary: "Katmanları kurallarla bir çizim standardının katmanlarına birleştirir (AutoCAD'in LAYTRANS'ı): nesne tutan her katman adına uyan ilk kuralı alır (from bir ad örüntüsüdür; adı kuralın to'su olan katman standart katmanın kendisidir); \
                      kuralın standart katmanı to adlı ilk katmandır, yoksa ve kuralı bir katman alırsa parent grubunun sonuna (verilmezse en üste) kuralın stiliyle (verilmeyen alanlar ilk kaynağınki) eklenir; var olanı kuralın verilen stilini alır. \
                      Kaynakların nesneleri standart katmana taşınır, kaynaklar silinir; etkin katman kaynaksa standart katman etkin olur; hepsi tek geri alma adımı (“Katmanları çevir”). Kilitli bir kaynak ya da standart katman hiçbir şeyi yazdırmaz (layer_locked). \
                      plan önizlemedir: neyin nereye gideceği, nesne sayıları, eklenecek ve stili değişecek katmanlar. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["laytrans".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::LayersTranslate>(),
            output: schema::<crate::LayersTranslated>(),
            plan: Some(schema::<crate::LayersTranslated>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Eski duvar katmanlarını ofisin A-DUVAR'ına çevir".into(),
                    input: json!({ "rules": [{ "from": "duvar*,wall*", "to": "A-DUVAR", "color": "#E5484D", "lineWeight": 0.5 }] }),
                    output: Some(json!({ "changed": true, "translations": [{ "layer": "layer-3", "name": "duvar", "rule": 0, "to": "layer-9", "toName": "A-DUVAR", "objects": 42 }],
                                         "created": ["layer-9"], "restyled": [], "objects": 42, "revision": "16" })),
                },
            ],
        },
        // Çizgi tipleri (docs/adr/0313 §2), held together by fixtures/commands/v1/cad.linetypes.edit.json.
        CommandDescriptor {
            id: crate::CAD_LINE_TYPES_EDIT.into(),
            version: crate::CAD_LINE_TYPES_EDIT_VERSION,
            title: "Çizgi tipleri".into(),
            summary: "Projenin desenli çizgi tiplerini değiştirir (AutoCAD'in LIN dosyaları ve LTYPE'ı): load bir LIN dosyasının metnindeki (lin; source dosyanın adı, her tipin kaynağı) tipleri alır, names verilmişse yalnız onları; \
                      projede aynı adlı tip (harf büyüklüğüne bakılmaz) kalır ve skipped'da söylenir, replace ile değişir; dosyanın okunmayan satırları problems'ta satırlarıyla söylenir. add tek bir tipi (lineType: ad, öğeler, birim paper ya da drawing) ekler, replace ile aynı adlıyı değiştirir. \
                      remove adı verilen tipi siler; bir katman, katman durumu ya da nesne onu kullanıyorsa line_type_in_use. scale projenin çizgi tipi ölçeğini (LTSCALE, 0'dan büyük, en çok 1000; 1 ölçeksiz) verir. export tipleri (names verilmişse onları) LIN metni olarak lin'de verir, çizimi değiştirmez. \
                      Tablo proje ayarıdır: değiştirmek çizimi kirli yapar, geri alma adımı değildir. Katmana desen cad.layers.set'in stiliyle (linePattern), nesneye cad.entities.set'in linePattern'iyle verilir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["linetype".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::LineTypesEdit>(),
            output: schema::<crate::LineTypesEdited>(),
            plan: Some(schema::<crate::LineTypesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir LIN dosyasının tiplerini al".into(),
                    input: json!({ "operation": "load", "lin": "*GAZ,Gaz hattı ---- GAZ ----\nA,.5,-.2,[\"GAZ\",STANDARD,S=.1,X=-.1,Y=-.05],-.25\n", "source": "kentos.lin" }),
                    output: None,
                },
                CommandExample {
                    title: "Çizgi tipi ölçeğini 2 yap".into(),
                    input: json!({ "operation": "scale", "scale": 2.0 }),
                    output: None,
                },
            ],
        },
        // Tarama desenleri (docs/adr/0331 §1), held together by fixtures/commands/v1/cad.hatchpatterns.edit.json.
        CommandDescriptor {
            id: crate::CAD_HATCH_PATTERNS_EDIT.into(),
            version: crate::CAD_HATCH_PATTERNS_EDIT_VERSION,
            title: "Tarama desenleri".into(),
            summary: "Projenin kendi tarama desenlerini değiştirir (AutoCAD'in PAT dosyaları): load bir PAT dosyasının metnindeki (pat; source dosyanın adı, her desenin kaynağı; unit sayıların birimi: mm ya da inch kâğıtta, drawing çizimde, yoksa mm) desenleri alır, names verilmişse yalnız onları; \
                      projede aynı adlı desen (harf büyüklüğüne bakılmaz) kalır ve skipped'da söylenir, replace ile değişir; dosyanın okunmayan satırları problems'ta satırlarıyla söylenir ve o desen alınmaz. add tek bir deseni (pattern: ad, açıklama, birim, aileler: açı, başlangıç, öteleme, kesikler) ekler, replace ile aynı adlıyı değiştirir. \
                      remove adı verilen deseni siler; tarama kendi tanımını taşıdığı için hiçbir tarama değişmez. export desenleri (names verilmişse onları) PAT metni olarak pat'ta verir, çizimi değiştirmez. \
                      Projenin desenleri kitaplığınkilerden önce aranır: cad.entities.hatch'in pattern.name'i ve Tarama'nın Desen menüsü. Tablo proje ayarıdır: değiştirmek çizimi kirli yapar, geri alma adımı değildir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["hatchpattern".into(), "pat".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::HatchPatternsEdit>(),
            output: schema::<crate::HatchPatternsEdited>(),
            plan: Some(schema::<crate::HatchPatternsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir PAT dosyasının desenlerini al".into(),
                    input: json!({ "operation": "load", "pat": "*TUGLA, Tuğla\n0, 0,0, 0,6.35\n90, 0,0, 6.35,6.35, 6.35,-6.35\n", "source": "ofis.pat", "unit": "mm" }),
                    output: None,
                },
                CommandExample {
                    title: "Projenin desenlerini PAT olarak ver".into(),
                    input: json!({ "operation": "export" }),
                    output: None,
                },
            ],
        },
        // Projenin stil kitaplığı (docs/adr/0314 §3), held together by fixtures/commands/v1/cad.library.edit.json.
        CommandDescriptor {
            id: crate::CAD_LIBRARY_EDIT.into(),
            version: crate::CAD_LIBRARY_EDIT_VERSION,
            title: "Stil kitaplığı".into(),
            summary: "Projenin stil kitaplığını değiştirir (semboller, çizimler ve resimler, nesne şablonları; sistemin ve kullanıcının kitaplıkları ev sahibinindir): create bir öğe ekler (item: id, kind, name, path ve türünün alanları; .kstil dosyasının kurallarıyla, çizimler temizlenir), \
                      update id'nin alanlarını patch'le değiştirir (null kaldırır), duplicate kopyasını newId (yoksa yeni), name ve path ile ekler, rename adını, move kategorisini değiştirir, remove bir nesne, katman, şablon ya da sembol kullanmıyorsa siler (library_item_in_use). \
                      import bir .kstil dosyasının metnini (kstil) alır: kitaplıkta olan kimlik kalır (conflict: skip) ya da değişir (replace); export öğeleri (ids, yoksa hepsi) çizdikleriyle .kstil metni olarak kstil'de verir, çizimi değiştirmez. \
                      Kitaplık geri alma adımı değildir, çizimi kirli yapar. assign sembolü nesnelere (uids; cad.entities.set'in “Sembol ata” adımı) ya da bir katmana (layer; tek sembollü işleyicinin sembolün türündeki yuvası, cad.layers.renderer'ın “Katman stili” adımı) verir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["stylelibrary".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::LibraryEdit>(),
            output: schema::<crate::LibraryEdited>(),
            plan: Some(schema::<crate::LibraryEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Projenin bir sembolünü kopyala".into(),
                    input: json!({ "operation": "duplicate", "id": "p-agac", "newId": "p-agac-2", "name": "Ağaç (büyük)" }),
                    output: None,
                },
                CommandExample {
                    title: "Sembolü seçili nesnelere ver".into(),
                    input: json!({ "operation": "assign", "id": "p-agac", "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"] }),
                    output: None,
                },
            ],
        },
        // Stil değişikliğinin etkisi (docs/adr/0314 §4), held together by fixtures/commands/v1/cad.styles.impact.json.
        CommandDescriptor {
            id: crate::CAD_STYLES_IMPACT.into(),
            version: crate::CAD_STYLES_IMPACT_VERSION,
            title: "Stil değişikliğinin etkisi".into(),
            summary: "Bir katmanın görünüşü, bir yazı ya da ölçü stili ya da bir çizgi tipi değişince neye ulaşacağını söyler (target: layer, textStyle, dimensionStyle, lineType; id katmanın ya da stilin kimliği, çizgi tipinin adı): \
                      değişikliği alan nesneler (follow, ilk 1000'inin kimlikleri uids'te), kendi değerini koruyanlar özelliklerine göre (own: color, lineType, lineWeight, transparency; yazıda font, bold, italic, oblique, widthFactor, height; ölçüde görünüşün alanları), \
                      kilitli katmanda kalanlar (stil değişince olduğu gibi kalır; katmanın görünüşü onlara da yansır), deseni kullanan katmanlar, onu izleyen blok parçaları ve onları gösteren yerleştirmeler (iç içe olanlar da), \
                      kendi görünüşünü saklayan katman durumları, onu geçersiz kılan pafta haritaları ve dış başvurular (KentOS dış başvuru taşımaz: hep 0); notes Türkçe özettir. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["styleimpact".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::StylesImpact>(),
            output: schema::<crate::StyleImpact>(),
            plan: Some(schema::<crate::StyleImpact>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Duvar katmanının rengi değişirse neye ulaşır".into(),
                input: json!({ "target": "layer", "id": "duvar" }),
                output: None,
            }],
        },
        // Standardı denetle (docs/adr/0312 §2), held together by fixtures/commands/v1/cad.standards.check.json.
        CommandDescriptor {
            id: crate::CAD_STANDARDS_CHECK.into(),
            version: crate::CAD_STANDARDS_CHECK_VERSION,
            title: "Standardı denetle".into(),
            summary: "Çizimin bir çizim standardından (AutoCAD'in CAD Standards'ı) neyi bozduğunu söyler: standart adıyla (name: mimari-ofis gibi yerleşik bir standart) ya da kendisiyle (standard: katmanları adları, renkleri, çizgi tipleri, kalınlıkları, Yazdır'ları ve topladıkları ad örüntüleriyle; \
                      öbür katmanların ad örüntüleri, yazı tipleri, yazı ve ölçü stilleri, çizgi tipleri, nesnelerin katmana göreliği) verilir. Bulgular sırasıyla: standardın katmanları (eksik; rengi, çizgi tipi, kalınlığı, Yazdır'ı farklı), \
                      çizimin öbür katmanları (bir standart katmana ait ad, ad örüntülerine uymayan ad, izinsiz çizgi tipi), projenin yazı tipi, stillerin yazı tipleri, eksik ve standartta olmayan stiller, izinsiz kendi çizgi tipli ve kendi görünüşlü nesnelerin sayıları. \
                      Her bulgu türü, konusu, katmanı, beklenen ve bulunan değeri, Türkçe iletisi ve cad.standards.apply'ın düzeltip düzeltmediğiyle gelir. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["checkstandards".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::StandardsCheck>(),
            output: schema::<crate::StandardsReport>(),
            plan: Some(schema::<crate::StandardsReport>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimi mimari ofis standardıyla denetle".into(),
                input: json!({ "name": "mimari-ofis" }),
                output: None,
            }],
        },
        // Standardı uygula (docs/adr/0312 §3), held together by fixtures/commands/v1/cad.standards.apply.json.
        CommandDescriptor {
            id: crate::CAD_STANDARDS_APPLY.into(),
            version: crate::CAD_STANDARDS_APPLY_VERSION,
            title: "Standardı uygula".into(),
            summary: "Çizimi bir çizim standardına getirir; cad.standards.check'in düzeltilebilir bulgularının ayrı, planlı düzeltmesidir. Standart denetimdeki gibi verilir. \
                      rename (varsayılan açık): standart katmanlara ait eski katmanlar (standart katmanın from örüntüsü) Katmanları çevir'in yazmasıyla birleşir; layers: eksik standart katmanlar standardın sırasıyla parent grubunun sonuna (verilmezse en üste) eklenir; \
                      restyle: var olan standart katmanlar standardın rengini, çizgi tipini, kalınlığını ve Yazdır'ını alır (birleşmede açılan katman her zaman alır). Katmanlar tek geri alma adımıdır (“Standardı uygula”). \
                      styles: projenin yazı tipi, stillerin yazı tipleri ve eksik yazı ve ölçü stilleri proje ayarı olarak yazılır (geri alma adımı değil; yeni stiller UUIDv7 kimlik alır, cevap onları adlarıyla söyler). Nesnelerin kendi görünüşüne dokunulmaz. \
                      plan önizlemedir. Kilitli bir eski katman hiçbir şeyi yazdırmaz (layer_locked). expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["standards".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::StandardsApply>(),
            output: schema::<crate::StandardsApplied>(),
            plan: Some(schema::<crate::StandardsApplied>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Mimari ofis standardını uygula".into(),
                    input: json!({ "name": "mimari-ofis" }),
                    output: None,
                },
                CommandExample {
                    title: "Yalnız eksik katmanları ekle".into(),
                    input: json!({ "name": "mimari-ofis", "restyle": false, "rename": false, "styles": false }),
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
                      remove stili siler, nesneleri görünüşlerini korur ve bağlarını yitirir; export türün stillerini (styles verilmişse onları) kentos.annotation-styles dosyasının metni olarak library'de verir, yazmaz; \
                      import library'deki dosyanın stillerini adlarıyla alır: yeni ad eklenir, tablodaki ad olduğu gibi kalır (skipped) ya da replace ile yerinde güncellenir ve nesneleri izler. Adlar büyük küçük harf ayırmadan bir kez kullanılır, Standart başka stile verilmez. \
                      Tablo proje ayarıdır: değiştirmek çizimi kirli yapar, geri alma adımı değildir; izleyen nesneler tek geri alma adımında (“Yazı stili”, “Ölçü stili”) yazılır, kilitli katmandakiler değişmez ve locked'da sayılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Duvar stilinin (kind wall) materialId'li bileşeni projenin malzemesinin metnini, taramasını ve kalemini alır; projede olmayan malzemeye bağlı bileşen create, update ve import'ta material_not_found ile reddedilir (docs/adr/0791)."
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
                    title: "Yazı stillerini dosyaya ver".into(),
                    input: json!({ "kind": "text", "operation": "export" }),
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
            summary: "Projenin kendi ayarlarından verilenleri değiştirir, verilmeyenler kalır: name (çizimin adı), projectType (cad ya da gis; türü seçilmemiş projeye bir kez verilir, mevcut tür değişmez), srid (kayıttaki bir EPSG kodu ya da yerel proje için 0; atanır, koordinatlar dönüştürülmez; tanımlı sistemin yerini alır, sistem birimi metre yapar), \
                      secondSrid (kayıttaki, projeninkinden başka bir EPSG kodu ya da yok için 0), drawingUnit (yalnız sistemsiz yerel projede mm, cm ya da m), lengthDecimals ve areaDecimals (0–4), areaUnit (m2, donum, ha), angleUnit (grad, deg), \
                      plotScale (1 ile 1 000 000 arası tam sayı payda; eski genel yükseklikteki yazılar, kılavuzlar, ölçüler ve tablolar yenisini tek geri alma adımında, “Yazı yüksekliklerini uydur”, alır), drawingFont, heights (kotların ne olduğu, bütün olarak: ellipsoidal ya da orthometric, ortometrikte verticalSrid ve geoid {name, sha256}; {kind: unknown} kaldırır; atanır, kot değişmez) ve coordinateEpoch (1900–2100 ondalık yıl; 0 kaldırır; elipsoit yüksekliği ve epok sistem ister). \
                      arch bütün mimari tercihtir: katlar, ayrıntı/alan yöntemi/evre, isteğe bağlı spaceNames/spaceUsages listeleri ve kalıcı kat kimliğine bağlı spaceNumberPrefixes. Liste değişiminde mevcut kat ve tercihleri kopyalayın; arch iç içe yama değildir. Listeler öneridir; mevcut mahal adı/numarası/kullanımı değişmez, önek yalnız açık seçimle başlangıca alınır. \
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
        // The drawing's user coordinate system (docs/adr/0261), held together by
        // fixtures/commands/v1/cad.ucs.set.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_UCS_SET.into(),
            version: crate::CAD_UCS_SET_VERSION,
            title: "Kullanıcı koordinat sistemi".into(),
            summary: "Çizimin 2B kullanıcı koordinat sistemini (UCS) değiştirir: dünyada bir başlangıç ve x ekseninin doğudan saat yönünün tersine açısı (radyan, [0, 2π)). \
                      to.kind: world (dünya), frame (origin ve angle verildiği gibi), origin (başlangıç point'e, açı kalır), rotate (açı angle kadar döner), twoPoints (başlangıç origin, x ekseni xPoint'e), \
                      object (uid'li nesneye at'ta hizalı: çizgide yakın uç, çoklu çizgi, alan ve taramada yakın kenar, yayda ve dairede merkez, elipste büyük eksen, yazıda ve blokta yerleştirme ve dönüş, noktada açı kalır), \
                      named (adlı UCS'e geçer), save (geçerli UCS'i adla saklar, aynı adlıyı değiştirir), delete (adlı UCS'i siler). \
                      Nesneler değişmez; tel her zaman dünyadır: komutların noktaları ve ölçüler UCS'ten bağımsızdır, UCS yalnız arayüzün yazma ve gösterme eksenidir. \
                      Proje ayarıdır, geri alma adımı değildir; aynı UCS'e geçmek bir şey değiştirmez (changed false). \
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
            input: schema::<crate::UcsSet>(),
            output: schema::<crate::UcsChanged>(),
            plan: Some(schema::<crate::UcsChanged>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Eğik bir binanın duvarı boyunca: iki nokta".into(),
                    input: json!({ "to": { "kind": "twoPoints", "origin": { "x": 487000.0, "y": 4420000.0 }, "xPoint": { "x": 487010.0, "y": 4420010.0 } } }),
                    output: Some(json!({
                        "ucs": { "origin": { "x": 487000.0, "y": 4420000.0 }, "angle": std::f64::consts::FRAC_PI_4 },
                        "named": [],
                        "changed": true,
                        "revision": "12"
                    })),
                },
                CommandExample {
                    title: "Geçerli UCS'i “Bina” adıyla sakla".into(),
                    input: json!({ "to": { "kind": "save", "name": "Bina" } }),
                    output: None,
                },
                CommandExample {
                    title: "Dünyaya dön".into(),
                    input: json!({ "to": { "kind": "world" } }),
                    output: None,
                },
            ],
        },
        // The project's named views (docs/adr/0285), held together by
        // fixtures/commands/v1/cad.views.edit.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_VIEWS_EDIT.into(),
            version: crate::CAD_VIEWS_EDIT_VERSION,
            title: "Adlı görünümler".into(),
            summary: "Projenin adlı görünümlerini düzenler. Görünüm, kameranın baktığı yerdir: alanın ortasındaki dünya noktası (center), gösterilen dünya eni ve boyu (width, height; ekranın x ve y'si boyunca) ve dönüşü (twist: ekranın x ekseninin doğudan saat yönünün tersine açısı, radyan, [0, 2π)'ye getirilir; 0 kuzeyi yukarıda gösterir). \
                      edit.kind: save (görünümü adla saklar; aynı adlı varsa yerine geçer, view_replaced uyarısı), rename (adlı görünümün adını to yapar, listedeki yeri kalır; to başka bir görünümün adıysa view_exists), delete (adlı görünümü siler). \
                      Adlar baştaki ve sondaki boşluklar atılarak karşılaştırılır; 1–64 karakter, denetim karakteri yok; projede en çok 100 görünüm. \
                      Proje ayarıdır, geri alma adımı değildir; bir görünüme gitmek arayüzün kamerasıdır, komut değildir; nesneler değişmez. \
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
            input: schema::<crate::ViewsEdit>(),
            output: schema::<crate::ViewsEdited>(),
            plan: Some(schema::<crate::ViewsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parselin çevresini “Ada 112” adıyla sakla, 30° döndürülmüş".into(),
                    input: json!({ "edit": { "kind": "save", "name": "Ada 112", "center": { "x": 487020.0, "y": 4420015.0 }, "width": 60.0, "height": 40.0, "twist": std::f64::consts::FRAC_PI_6 } }),
                    output: Some(json!({
                        "views": [{ "name": "Ada 112", "center": { "x": 487020.0, "y": 4420015.0 }, "width": 60.0, "height": 40.0, "twist": std::f64::consts::FRAC_PI_6 }],
                        "view": { "name": "Ada 112", "center": { "x": 487020.0, "y": 4420015.0 }, "width": 60.0, "height": 40.0, "twist": std::f64::consts::FRAC_PI_6 },
                        "changed": true,
                        "revision": "7"
                    })),
                },
                CommandExample {
                    title: "Yeniden adlandır".into(),
                    input: json!({ "edit": { "kind": "rename", "name": "Ada 112", "to": "Ada 112 kuzey" } }),
                    output: None,
                },
                CommandExample {
                    title: "Sil".into(),
                    input: json!({ "edit": { "kind": "delete", "name": "Ada 112 kuzey" } }),
                    output: None,
                },
            ],
        },
        // The project's forms (docs/adr/0401), held together by fixtures/commands/v1/cad.forms.*.json.
        // The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_FORMS_EDIT.into(),
            version: crate::CAD_FORMS_EDIT_VERSION,
            title: "Formlar".into(),
            summary: "Projenin formlarını düzenler. Form bir katmanın veri giriş ekranıdır: sekmeler, gruplar ve katmanın alanlarına kararlı anahtarlarıyla (alanın id'si, yoksa adı) bağlı alan öğeleri; bir alanı gösteren (visibleWhen), zorunlu kılan (requiredWhen) ve kilitleyen (readOnlyWhen) kurallar ve kaydın geçmesi gereken denetimler (checks); kurallar İfadeyle seç'in dilindedir. \
                      edit.kind: create (katmanın formu; body verilmezse katmanın alanlarından, id verilmezse adından üretilir), update (taslağı değiştirir; body.revision taslağın şimdiki sürümü olmalı, değilse form_draft_changed; saklanan taslak bir sonraki sürümdür), rename, remove, publish (taslağı sıradaki yayın olarak dondurur; ön denetimde hata varsa form_preflight, senaryolarından biri tutmazsa form_tests_failed; önceki yayına uyup yenisine uymayan kayıtlar sayılır: breaking ve form_breaking uyarısı; en çok 50 yayın, fazlası en eskiyi düşürür: form_release_dropped), enforce (on: son yayının kuralları katmanın her öznitelik yazmasında uygulanır, cad.entities.set'te form_rules; yayın yoksa form_unpublished, yayından kaldırılmışsa form_withdrawn), withdraw (on: form yayından kaldırılır, kayıt onunla yazılmaz: formla yazan komutlar form_withdrawn; yayın yoksa form_unpublished, kuralları uygulanıyorsa form_enforced; off: yeniden yayına alır), comment (gözden geçirenin forma ya da taslağın bir öğesine yorumu; olmayan öğe form_item_unknown), resolve (yorumu çözer ya da yeniden açar; olmayan numara form_comment_unknown). \
                      Çıktının issues'u taslağın ön denetimidir (form_field_unknown, form_widget_unfit, form_rule_invalid, form_required_hidden …; uyarılar da). Proje ayarıdır, geri alma adımı değildir; nesneler değişmez. \
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
            input: schema::<crate::FormsEdit>(),
            output: schema::<crate::FormsEdited>(),
            plan: Some(schema::<crate::FormsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bina katmanının alanlarından “Bina denetimi” formunu oluştur".into(),
                    input: json!({ "edit": { "kind": "create", "name": "Bina denetimi", "layer": "bina" } }),
                    output: None,
                },
                CommandExample {
                    title: "Taslağı yayınla".into(),
                    input: json!({ "edit": { "kind": "publish", "id": "bina-denetimi", "note": "İlk yayın" } }),
                    output: None,
                },
                CommandExample {
                    title: "Son yayının kurallarını katmanda uygula".into(),
                    input: json!({ "edit": { "kind": "enforce", "id": "bina-denetimi", "on": true } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_FORMS_VALIDATE.into(),
            version: crate::CAD_FORMS_VALIDATE_VERSION,
            title: "Formu denetle".into(),
            summary: "Bir formun ve bir kaydın durumunu söyler, hiçbir şey yazmaz. Formun ön denetimi (olmayan alan, uymayan öğe, okunamayan kural, gizlenebilen zorunlu alan; uyarılar da); uid verilirse o nesnenin öğeleri (gösterilen, zorunlu, salt okunur) ve values (alan anahtarlarıyla) yazılmış gibi kaydın sorunları: zorunlu alan boş (form_required), değer alanın kuralına uymuyor (form_value_invalid), denetim tutmuyor (form_check_failed; uyarı denetimi form_check_warning), gizli ya da salt okunur alan değişti (form_hidden, form_read_only), süzgecin sunmadığı seçenek (form_choice_filtered), hesap alana uymuyor (form_calc_invalid), tekil alanın değeri başka nesnede (form_unique). Kayıt formun tamamladığı gibidir: varsayılanlar ve hesaplar yazılmış; öğelerde süzgeçli alanın sunduğu seçenekler (choices) ve notun yazılmış metni (text). \
                      draft: son yayın yerine taslak (tasarımcının önizlemesi); defaults: verilen değerlerden sonra formun varsayılanları kaydın boş alanlarına yazılır (yeni kayıt); moment ve user kuralların @tarih, @simdi ve @kullanici'sidir. Ön denetimde hata olan form reddedilmez, kaydı sorulmaz."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsValidate>(),
            output: schema::<crate::FormsValidated>(),
            plan: Some(schema::<crate::FormsValidated>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir binanın kaydını “ek inceleme gerekli” yazılmış gibi denetle".into(),
                input: json!({ "form": "bina-denetimi", "uid": "0192f5a1-25a0-7000-8000-000000000201", "values": { "Durum": "inceleme" } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_FORMS_SUBMIT.into(),
            version: crate::CAD_FORMS_SUBMIT_VERSION,
            title: "Formla kaydet".into(),
            summary: "Bir nesnenin kaydını formla yazar: values alan anahtarlarıyla (boş değer özniteliği kaldırır); formun kuralları ve alanların kuralları geçerse bütün kayıt, geçmezse hiçbir şey (form_invalid: ilk sorun, path onu veren değer). Gizli alanın değeri silinmez (formun clearHidden'ı açıksa silinir); gösterilmeyen, gizli, salt okunur ya da hesaplanan alana başka değer yazılmaz. \
                      Kayıt yazılmadan tamamlanır: defaults (yeni kayıt) verilen değerlerden sonra boş kalan alanlara formun değer ve ifade varsayılanlarını yazar; hesaplanan alanlar formun ifadeleriyle yazılır. moment (2026-10-16T14:30:00) ve user verilirse kuralların @tarih, @simdi ve @kullanici'si onlardır; verilmezse onları okuyan varsayılan ve hesap yapılmaz. Tekil alanın değeri katmanın başka nesnesinde olamaz. Uyarı denetimi bozulursa kayıt yine yazılır, uyarı form_check_warning'dir. \
                      Varsayılan olarak son yayın kullanılır; draft: taslak (yayınlanmamış form). Tek geri alma adımı: “Form: <ad>”. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      commit: kaydın ve alt kayıtlarının katmanları tek GeoPackage dosyasına bağlıysa (cad.sources.edit) aynı adımın ardından dosyaya tek SQLite işleminde yazılır (cad.sources.sync'in with'i, docs/adr/0532); dosyanın reddi gönderimin reddidir: plan söyler, ne çizimde ne dosyada bir şey yazılır. Cevabın committed'ı dosyaya yazılanı verir; bağlı olmayan katman yazılmaz."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsSubmit>(),
            output: schema::<crate::FormsSubmitted>(),
            plan: Some(schema::<crate::FormsSubmitted>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir binanın durumunu ve notunu formla yaz".into(),
                input: json!({ "form": "bina-denetimi", "uid": "0192f5a1-25a0-7000-8000-000000000201", "values": { "Durum": "inceleme", "Not": "Çatı arası ayrıca bakılmalı." } }),
                output: None,
            }],
        },
        // Every record of a form's layer by its rules (docs/adr/0406 §1), held together by
        // fixtures/commands/v1/cad.forms.audit.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_FORMS_AUDIT.into(),
            version: crate::CAD_FORMS_AUDIT_VERSION,
            title: "Kayıtları formla denetle".into(),
            summary: "Formun katmanındaki bütün kayıtları formun kurallarıyla denetler, hiçbir şey yazmaz: her kayıt hesaplarıyla tamamlanır ve formla yazılsa nasıl değerlendirilecekse öyle değerlendirilir (zorunlu alan, değerin kuralı, süzülen seçenek, başka katmandan liste, denetimler, tekil alan). \
                      checked bütün kayıtlar, failing hatası olanlar, warned yalnız uyarısı olanlar; records hatası olanlar (warnings ile uyarısı olanlar da), katmanın sırasıyla, en çok limit (varsayılan 500, en çok 10000), fazlası more. draft taslakla, moment ve user cad.forms.submit'teki gibi."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsAudit>(),
            output: schema::<crate::FormsAudited>(),
            plan: Some(schema::<crate::FormsAudited>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bina formuna uymayan kayıtları listele".into(),
                input: json!({ "form": "bina-denetimi", "warnings": true }),
                output: None,
            }],
        },
        // A form's calculations again over its layer (docs/adr/0421), held together by
        // fixtures/commands/v1/cad.forms.recompute.json.
        CommandDescriptor {
            id: crate::CAD_FORMS_RECOMPUTE.into(),
            version: crate::CAD_FORMS_RECOMPUTE_VERSION,
            title: "Hesapları yenile".into(),
            summary: "Formun katmanındaki her kaydı formun bugünkü kurallarıyla tamamlar: hesaplar yeniden yapılır, form gizlenenleri siliyorsa \
                      gizlenen alanlar boşalır; varsayılanlar dolmaz. Değişen kayıtlar tek geri alma adımında yazılır. Hesabı alanına sığmayan \
                      kayıt olduğu gibi kalır (failed, form_calc_failed). draft taslakla, yoksa son yayınla."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsRecompute>(),
            output: schema::<crate::FormsRecomputed>(),
            plan: Some(schema::<crate::FormsRecomputed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bina formunun hesaplarını bütün kayıtlarda yenile".into(),
                input: json!({ "form": "bina-denetimi" }),
                output: None,
            }],
        },
        // The project's value lists (docs/adr/0430), held together by fixtures/commands/v1/cad.domains.edit.json.
        CommandDescriptor {
            id: crate::CAD_DOMAINS_EDIT.into(),
            version: crate::CAD_DOMAINS_EDIT_VERSION,
            title: "Değer listeleri".into(),
            summary: "Projenin değer listelerini düzenler (ArcGIS'in domain'i): bir kod listesi (values) ya da aralık (min, max) projede bir kez tutulur,                       birçok katmanın alanı ona bağlanır; bağlı alanın değerleri listenin kopyasıdır ve liste değişince aynı adımda yeniden yazılır.                       create yeni liste (domainKind: uyduğu alan türü), fromField bir alanın kendi listesinden liste yapıp alanı bağlar, update listeyi                       değiştirir (türü değişmez; kayıtların tuttuğu kod listeden çıkamaz: recode ile başka koda taşınır ya da kullanımdan kaldırılır),                       remove bağlı alanı olmayan listeyi siler (detach: alanlar kopyalarını tutar), link alanı bir listeye bağlar ya da bağı çözer.                       Tek geri alma adımı “Değer listeleri”."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DomainsEdit>(),
            output: schema::<crate::DomainsEdited>(),
            plan: Some(schema::<crate::DomainsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yapı malzemesi listesi".into(),
                    input: json!({ "edit": { "kind": "create", "name": "Yapı malzemesi", "domainKind": "text", "values": [
                        { "code": "ah", "label": "Ahşap" }, { "code": "bt", "label": "Betonarme" }, { "code": "ym", "label": "Yığma" }
                    ] } }),
                    output: None,
                },
                CommandExample {
                    title: "Binanın Malzeme alanını listeye bağla".into(),
                    input: json!({ "edit": { "kind": "link", "layer": "bina", "field": "Malzeme", "domain": "yapi-malzemesi" } }),
                    output: None,
                },
            ],
        },
        // The value lists as a package (docs/adr/0439 §7), held together by fixtures/commands/v1/cad.domains.export.json.
        CommandDescriptor {
            id: crate::CAD_DOMAINS_EXPORT.into(),
            version: crate::CAD_DOMAINS_EXPORT_VERSION,
            title: "Değer listelerini dışa aktar".into(),
            summary: "Projenin değer listelerini paket olarak verir (kentos.domains, .kliste), hiçbir şey yazmaz: verilen kimliklerin \
                      listeleri verildiği sırayla, kimlik verilmezse bütün listeler projenin sırasıyla. Kayıt, bağ ya da yetki taşımaz; \
                      cad.domains.edit'in import işlemi başka bir projeye alır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::DomainsExport>(),
            output: schema::<crate::DomainsPackage>(),
            plan: Some(schema::<crate::DomainsPackage>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Yapı malzemesi listesini paket olarak al".into(),
                input: json!({ "ids": ["yapi-malzemesi"] }),
                output: None,
            }],
        },
        // A layer's values in their fields' canonical texts (docs/adr/0439 §8), held together by
        // fixtures/commands/v1/cad.fields.migrate.json.
        CommandDescriptor {
            id: crate::CAD_FIELDS_MIGRATE.into(),
            version: crate::CAD_FIELDS_MIGRATE_VERSION,
            title: "Değerleri biçime çevir".into(),
            summary: "Katmanın alanları tiplenmeden önce yazılmış değerlerini alanlarının biçimine çevirir: 1,5 1.5, 17.10.2026 \
                      2026-10-17, evet true, listedeki değerin etiketi kodu olur. Kayıpsızdır: alana uymayan değer yazıldığı gibi kalır \
                      ve sayılır (values_unfit), boş değer boş, hiçbir alanın adlandırmadığı öznitelik olduğu gibi kalır. Alan verilmezse \
                      katmanın listesiz metin olmayan bütün alanları. Tek geri alma adımı “Değerleri biçime çevir”."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::FieldsMigrate>(),
            output: schema::<crate::FieldsMigrated>(),
            plan: Some(schema::<crate::FieldsMigrated>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsel katmanının değerlerini biçime çevir".into(),
                input: json!({ "layer": "parsel" }),
                output: None,
            }],
        },
        // What each attachment is (docs/adr/0442 §7), held together by fixtures/commands/v1/cad.attachments.inspect.json.
        CommandDescriptor {
            id: crate::CAD_ATTACHMENTS_INSPECT.into(),
            version: crate::CAD_ATTACHMENTS_INSPECT_VERSION,
            title: "Ekleri incele".into(),
            summary: "Nesnelerin eklerini baytlarından okur, hiçbir şey yazmaz: her ekin ortam türü (PNG, JPEG, GIF, TIFF, PDF ve ZIP baytlarından, \
                      ötekiler adının uzantısından), boyu ve SHA-256'sı; fotoğrafın EXIF'inin söylediği çekim anı ve WGS 84 konumu. Gömülü fotoğraf \
                      projenin kitaplığından, bağlı dosya diskten (göreli yol folder'dan), adres yalnız adıyla (indirilmez); bulunamayan missing. \
                      En çok 1 000 nesne; 256 MB'tan büyük dosyanın yalnız boyu."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::AttachmentsInspect>(),
            output: schema::<crate::AttachmentsInspected>(),
            plan: Some(schema::<crate::AttachmentsInspected>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir parselin eklerini incele".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "folder": "/home/kullanici/proje" }),
                output: None,
            }],
        },
        // The data schema a page at a time (docs/adr/0443 §1), held together by fixtures/commands/v1/cad.schema.describe.json.
        CommandDescriptor {
            id: crate::CAD_SCHEMA_DESCRIBE.into(),
            version: crate::CAD_SCHEMA_DESCRIBE_VERSION,
            title: "Şemayı keşfet".into(),
            summary: "Projenin veri şemasını sayfa sayfa listeler, hiçbir şey yazmaz: katmanlar (alanlarıyla), formlar, değer listeleri ve ilişkiler, \
                      bu sırayla; her biri kısa özetiyle (kimlik, ad, katman, sayı, son yayın), bütün tanımı yalnız expand ile. kinds türleri, layer bir \
                      katmanın sahip olduklarını (kendisi, formları, alanlarının bağladığı listeler, taraf olduğu ilişkiler), query kimliğinde ya da \
                      adında geçen sözcükleri (Türkçe katlanarak) süzer. Sayfa en çok 100 (varsayılan 20); next bir sonraki sayfanın imleci, total \
                      bütün bulunanlar. Bir ajanın bağlamına bütün şemayı değil, gerekeni koyması için."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SchemaDescribe>(),
            output: schema::<crate::SchemaDescribed>(),
            plan: Some(schema::<crate::SchemaDescribed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bina katmanının formlarını bul".into(),
                input: json!({ "kinds": ["forms"], "layer": "bina", "limit": 10 }),
                output: None,
            }],
        },
        // A form package out and in (docs/adr/0435), held together by fixtures/commands/v1/cad.forms.export.json and
        // cad.forms.import.json.
        CommandDescriptor {
            id: crate::CAD_FORMS_EXPORT.into(),
            version: crate::CAD_FORMS_EXPORT_VERSION,
            title: "Formu dışa aktar".into(),
            summary: "Formu paket olarak verir, hiçbir şey yazmaz: son yayının (draft ile taslağın) gövdesi, formun bağladığı ya da \
                      kurallarında okuduğu alanlar katmandaki tanımlarıyla ve bu alanların bağlı olduğu değer listeleri. Kayıt, bağlantı \
                      ya da yetki taşımaz; cad.forms.import başka bir katmana ya da projeye alır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsExport>(),
            output: schema::<crate::FormPackage>(),
            plan: Some(schema::<crate::FormPackage>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bina denetimi formunu paket olarak al".into(),
                input: json!({ "form": "bina-denetimi" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_FORMS_IMPORT.into(),
            version: crate::CAD_FORMS_IMPORT_VERSION,
            title: "Formu içe aktar".into(),
            summary: "Bir form paketini katmanın yeni formu (taslağı) yapar: paketin alanları katmanın alanlarına bağlanır (map, yoksa \
                      aynı anahtar, yoksa aynı ad; tür aynı olmalı), formun kuralları ve iş akışı yeni adlarla yazılır; katmanda olmayan \
                      alanlar addFields ile değer listeleriyle eklenir, yoksa reddedilir. Alanlar ve listeler tek geri alma adımı “Formu \
                      içe aktar”; form projenin ayarıdır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsImport>(),
            output: schema::<crate::FormsImported>(),
            plan: Some(schema::<crate::FormsImported>()),
            capabilities: None,
            examples: vec![],
        },
        // A record's workflow step (docs/adr/0428), held together by fixtures/commands/v1/cad.forms.transition.json.
        CommandDescriptor {
            id: crate::CAD_FORMS_TRANSITION.into(),
            version: crate::CAD_FORMS_TRANSITION_VERSION,
            title: "İş akışı adımı".into(),
            summary: "Kaydı formun iş akışında bir adım ilerletir: adım kaydın durumundan atılabilmeli; verilen değerler formun kurallarıyla yazılır \
                      (kilitli durumdaki kayıtta yalnız adımın gerekçesi), durum alanı adımın durumunu alır, kayıt yeniden tamamlanır; adım geçerli kayıt \
                      istiyorsa formun hatası olan kayıt, gerekçe istiyorsa boş gerekçe reddedilir. notes: alanların anahtarlarıyla inceleyenin notları, iş akışının not alanına yazılır \
                      (notsuz adım onu boşaltır; not alanı olmayan iş akışında ya da formda olmayan alanda form_invalid). Tek geri alma adımı “Form: <adım>”."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsTransition>(),
            output: schema::<crate::FormsTransitioned>(),
            plan: Some(schema::<crate::FormsTransitioned>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Denetimi tamamlanan binayı onaya gönder".into(),
                input: json!({ "form": "bina-denetimi", "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "step": "Gönder" }),
                output: None,
            }],
        },
        // A form's versions compared (docs/adr/0427), held together by fixtures/commands/v1/cad.forms.compare.json.
        CommandDescriptor {
            id: crate::CAD_FORMS_COMPARE.into(),
            version: crate::CAD_FORMS_COMPARE_VERSION,
            title: "Sürümleri karşılaştır".into(),
            summary: "Formun iki sürümü arasındaki değişiklikleri verir, hiçbir şey yazmaz: from (yoksa son yayın) ile to (yoksa taslak) arasında \
                      her öğe eklendi, kaldırıldı, taşındı (başka sekme ya da gruba, ya da kardeşleri arasında) ya da değişti (değişen özellikleriyle); \
                      ardından formun Gizlenenleri sil'i ve senaryoları. Değişiklikler yeni sürümün sırasıyla, kaldırılanlar sonda."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsCompare>(),
            output: schema::<crate::FormsCompared>(),
            plan: Some(schema::<crate::FormsCompared>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Taslak son yayından ne kadar farklı".into(),
                input: json!({ "form": "bina-denetimi" }),
                output: None,
            }],
        },
        // The sources of a form's lists (docs/adr/0763), held together by fixtures/commands/v1/cad.forms.sources.json.
        CommandDescriptor {
            id: crate::CAD_FORMS_SOURCES.into(),
            version: crate::CAD_FORMS_SOURCES_VERSION,
            title: "Formun kaynakları".into(),
            summary: "Formun başka katmandan listelerinin ve ilişkili kayıtlarının katmanlarının kaynağının durumunu verir, hiçbir şey yazmaz: \
                      çizimin kendi katmanı local; GeoPackage'a bağlı katman dosyasıyla (yok ya da tablo yok missing, okunamıyor failed, okunduğu \
                      baytlar değil changed, yoksa ready); veritabanına bağlı katman en çok waitMs (yoksa 5 000 ms) bekleyen bir bağlantıyla \
                      (ulaşılamıyor offline, yanıt yok timeout, giriş ya da tabloyu okuma yetkisi yok denied, veritabanı ya da tablo yok missing). \
                      Her durumun ayrı kodu ve çıkış yolu: retry (Yeniden dene), credentials (Bağlantıyı düzenle), relink (Yeniden bağla), \
                      read (Kaynaktan oku). Listeler her durumda çizimdeki çalışma kopyasından sunulur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsSources>(),
            output: schema::<crate::FormsSourced>(),
            plan: Some(schema::<crate::FormsSourced>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bina formunun listelerinin kaynakları".into(),
                input: json!({ "form": "bina-denetimi" }),
                output: None,
            }],
        },
        // A form's scenarios (docs/adr/0420), held together by fixtures/commands/v1/cad.forms.test.json.
        CommandDescriptor {
            id: crate::CAD_FORMS_TEST.into(),
            version: crate::CAD_FORMS_TEST_VERSION,
            title: "Form senaryolarını çalıştır".into(),
            summary: "Formun senaryolarını yeniden çalıştırır, hiçbir şey yazmaz: her senaryonun değerleri boş bir kayda yazılır (new ile varsayılanlarıyla), \
                      formun yaptığı (kaydın değerleri, gösterilen, zorunlu ve salt okunur öğeler, sorunlar) senaryonun beklediğiyle karşılaştırılır. \
                      draft taslağın senaryolarını taslakla, yoksa son yayının senaryolarını o yayınla; names yalnız verilenleri sırasıyla. Senaryolar anı ve kişiyi okumaz."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::FormsTest>(),
            output: schema::<crate::FormsTested>(),
            plan: Some(schema::<crate::FormsTested>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bina formunun taslağındaki senaryoları çalıştır".into(),
                input: json!({ "form": "bina-denetimi", "draft": true }),
                output: None,
            }],
        },
        // Records without a geometry, a table's rows (docs/adr/0410 §3), held together by
        // fixtures/commands/v1/cad.records.create.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_RECORDS_CREATE.into(),
            version: crate::CAD_RECORDS_CREATE_VERSION,
            title: "Kayıt ekle".into(),
            summary: "Verilen katmana geometrisiz kayıtlar (tablo satırları) ekler; tek geri alma adımıdır. Kayıt çizilmez, seçilmez, kenetlenmez; öznitelikleri, etiketi ve ekleri vardır, Öznitelik tablosunda, formlarda, ilişkilerde ve sorgularda yer alır. \
                      Her satırın öznitelikleri katmanın alanlarıyla yazılır: değer tek biçimine çevrilir, kuralı bozan reddedilir, verilmeyen alana varsayılanı yazılır; katmana form uygulanmışsa form da aynı adımda tamamlar. \
                      En çok 10000 satır. Kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
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
            input: schema::<crate::RecordsCreate>(),
            output: schema::<crate::RecordsCreated>(),
            plan: Some(schema::<crate::RecordsPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Binanın iki maliki".into(),
                input: json!({
                    "layerId": "malik",
                    "rows": [
                        { "attrs": { "Ad": "Ayşe Demir", "Pay": "1/2" }, "label": "Ayşe Demir" },
                        { "attrs": { "Ad": "Mehmet Kaya", "Pay": "1/2" } }
                    ]
                }),
                output: None,
            }],
        },
        // Kaydı çoğalt (docs/adr/0415), held together by fixtures/commands/v1/cad.records.duplicate.json.
        CommandDescriptor {
            id: crate::CAD_RECORDS_DUPLICATE.into(),
            version: crate::CAD_RECORDS_DUPLICATE_VERSION,
            title: "Kaydı çoğalt".into(),
            summary: "Nesneleri yerlerinde, bütün özellikleri ve ekleriyle yeni kalıcı kimliklerle kopyalar; tek geri alma adımıdır. \
                      children açıkken (varsayılan) nesnenin bileşik (cascade) ilişkilerindeki alt kayıtlar da kopyalanır, alanları kopyanın kimliğini alır; alt kaydın kendi bileşik ilişkileri de izlenir, her nesne bir kez kopyalanır. \
                      Girdideki bir nesne başka bir girdinin alt kaydıysa yalnız onunla kopyalanır. Katmanın bir formunun tekil tuttuğu alan kopyada boş kalır (unique_cleared). \
                      Kilitli katmandaki nesne ve alt kayıt atlanır, uyarıda sayılır; hepsi kilitliyse reddedilir. En çok 1000 nesne. \
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
            input: schema::<crate::RecordsDuplicate>(),
            output: schema::<crate::RecordsDuplicated>(),
            plan: Some(schema::<crate::RecordsDuplicatePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir binayı katları ve odalarıyla çoğalt".into(),
                input: json!({ "uids": ["0192f5a1-25a0-7000-8000-000000000201"] }),
                output: None,
            }],
        },
        // The project's relations (docs/adr/0405 §3), held together by
        // fixtures/commands/v1/cad.relations.edit.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_RELATIONS_EDIT.into(),
            version: crate::CAD_RELATIONS_EDIT_VERSION,
            title: "İlişkiler".into(),
            summary: "Projenin ilişkilerini oluşturur (create), değiştirir (update) ve siler (remove). İlişki: alt katmanın field alanı üst katmanın nesnesinin kalıcı kimliğini tutar; onDelete üst nesne silinince alt kayıtlara ne olacağıdır: unlink (varsayılan, alan boşalır), cascade (alt kayıtlar da silinir), restrict (alt kaydı olan üst nesne silinmez), keep (bağ kalır). \
                      Proje ayarıdır, geri alma adımı değildir; silinen ilişkinin bağları kayıtlarda kalır. \
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
            input: schema::<crate::RelationsEdit>(),
            output: schema::<crate::RelationsEdited>(),
            plan: Some(schema::<crate::RelationsEdited>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Sayaçları binalarına bağla; bina silinince sayaçlar da silinsin".into(),
                input: json!({ "edit": { "kind": "create", "relation": { "id": "bina-sayac", "name": "Binanın sayaçları", "parent": "bina", "child": "sayac", "field": "bina", "onDelete": "cascade" } } }),
                output: None,
            }],
        },
        // The project's join views kept and read (docs/adr/0527), held together by
        // fixtures/commands/v1/cad.joins.edit.json and cad.joins.query.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_JOINS_EDIT.into(),
            version: crate::CAD_JOINS_EDIT_VERSION,
            title: "Birleşik görünümler".into(),
            summary: "Projenin birleşik görünümlerini oluşturur (create), bir ilişkiden kurar (fromRelation: üst katman $uid ile, alt katman ilişkinin alanıyla), değiştirir (update) ve siler (remove). Görünüm bir katmanın kayıtlarını key alanıyla, with katmanının kayıtlarını withKey alanıyla eşler; anahtar bir alanın anahtarı ya da nesnenin kalıcı kimliği için $uid'dir. keep: all (varsayılan, eşleşmeyenler de) ya da matched; fields birleşen katmanın gösterilecek alanları. \
                      Satırlar kaydedilmez, cad.joins.query her okuyuşta çizimden kurar. Proje ayarıdır, geri alma adımı değildir. \
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
            input: schema::<crate::JoinsEdit>(),
            output: schema::<crate::JoinsEdited>(),
            plan: Some(schema::<crate::JoinsEdited>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parselleri maliklerine parsel numarasıyla bağla".into(),
                input: json!({ "edit": { "kind": "create", "view": { "id": "parsel-malik", "name": "Parseller ve malikleri", "layer": "parsel", "key": "no", "with": "malik", "withKey": "parsel" } } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_JOINS_QUERY.into(),
            version: crate::CAD_JOINS_QUERY_VERSION,
            title: "Birleşik görünümü oku".into(),
            summary: "Birleşik görünümün satırlarını çizimin şimdiki hâlinden sayfa sayfa verir, hiçbir şey yazmaz: kayıtlı görünüm (view) ya da tanım (definition, önizleme). \
                      Ana katmanın kaydı birden çok kayıtla eşleşirse birden çok satırdır; satırın kimliği iki kaydın kalıcı kimliğidir (left/right, eşleşmeyende left/), her okuyuşta aynı. \
                      Sütunlar kaydın tarafını (left, right) ve alanını söyler; bir hücre cad.entities.set ile o kayda yazılır, yazılamayanın (anahtar, kilitli katman) nedeni sütundadır. \
                      report kayıtların nasıl eşleştiğini (eşleşen, eşleşmeyen, anahtarsız, kardinalite 1:1, 1:N, N:1, N:N), totals sayı sütunlarının toplamını (her kayıt bir kez) verir. \
                      Katmanı ya da anahtarı kalkan görünüm broken ile söylenir, satır verilmez. Sayfa en çok 1000 (varsayılan 100); next sonraki sayfanın imleci."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::JoinsQuery>(),
            output: schema::<crate::JoinsAnswer>(),
            plan: Some(schema::<crate::JoinsAnswer>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsellerin maliklerini oku".into(),
                input: json!({ "view": "parsel-malik", "limit": 50 }),
                output: None,
            }],
        },
        // A layer as a dataset (docs/adr/0633), held together by fixtures/commands/v1/cad.datasets.edit.json and
        // cad.datasets.describe.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_DATASETS_EDIT.into(),
            version: crate::CAD_DATASETS_EDIT_VERSION,
            title: "Veri kümesi bilgisi".into(),
            summary: "Bir katmanın veri kümesi bilgisini yazar: başlık, açıklama, sorumlu (ad, kurum, e-posta), lisans, kullanım kısıtı, anahtar sözcükler, güncelleme günü (YYYY-AA-GG), ölçek (paydası), çözünürlük ve konum doğruluğu (metre), tamlık ve üretim açıklaması. \
                      Her parça isteğe bağlıdır; dataset verilmezse bilgi kaldırılır. Kapsam, koordinat sistemi, nesne sayıları ve veri sözlüğü yazılmaz, cad.datasets.describe her okuyuşta çizimden hesaplar. \
                      Tek geri alma adımı “Veri kümesi bilgisi”; aynısı yazılmaz. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DatasetsEdit>(),
            output: schema::<crate::DatasetsEdited>(),
            plan: Some(schema::<crate::DatasetsEditPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsellerin sorumlusunu, lisansını ve ölçeğini yaz".into(),
                input: json!({ "layer": "parsel", "dataset": { "title": "Kadastro parselleri", "responsible": { "organisation": "Harita Müdürlüğü", "email": "harita@ornek.bel.tr" }, "license": "CC-BY-4.0", "updated": "2026-10-20", "scale": 1000 } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_DATASETS_DESCRIBE.into(),
            version: crate::CAD_DATASETS_DESCRIBE_VERSION,
            title: "Veri kümesini tanımla".into(),
            summary: "Katmanları veri kümesi olarak tanımlar, hiçbir şey yazmaz: veri kümesi bilgisi, nesne sayıları ve türleri, kapsam (projenin sisteminde ve WGS 84'te), koordinat sistemi, bağlı kaynağı, veri sözlüğü (alanlar, türleri, zorunluluk, birim, değer sayısı, dolu kayıtlar). \
                      Bir işlem aracının açtığı katmanın türetim kaydı (araç, değerler, girdiler ve içerik özetleri, motor, zaman) ve girdilerinin bugünkü durumu (current, changed, gone); stale: bir girdisi değişti ya da geldiği bir katman eskidi. \
                      upstream katmanın geldiği katmanlar, downstream ondan üretilenler (türetim kayıtları üzerinden). records: true her katmanı OGC API Records kaydı (GeoJSON) olarak da verir. layers verilmezse nesne tutan bütün katmanlar ağacın sırasıyla."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::DatasetsDescribe>(),
            output: schema::<crate::DatasetsDescribed>(),
            plan: Some(schema::<crate::DatasetsDescribed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Tampon katmanının neyden üretildiğini ve eskiyip eskimediğini sor".into(),
                input: json!({ "layers": ["tampon"] }),
                output: None,
            }],
        },
        // Many records through a form at once (docs/adr/0403 §5), held together by
        // fixtures/commands/v1/cad.forms.fill.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_FORMS_FILL.into(),
            version: crate::CAD_FORMS_FILL_VERSION,
            title: "Formla toplu yaz".into(),
            summary: "Aynı değerleri birden çok kayda formla yazar (formun toplu kipi): uids aynı katmandaki nesneler (yinelenen bir kez yazılır), values alan anahtarlarıyla. Her kayıt kendi değerleri ve şekliyle formun kurallarından ve hesaplarından geçer; biri geçmezse hiçbiri yazılmaz (form_invalid: ilk sorunlu kaydın ilk sorunu, path onu veren değer, uids o kayıt). Tekil alana birden çok kayıtta aynı değer yazılamaz. \
                      moment ve user cad.forms.submit'teki gibidir. Uyarı denetimi yazmayı durdurmaz (form_check_warning). Tek geri alma adımı: “Form: <ad> (n nesne)”; changed değişen kayıtlar. \
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
            input: schema::<crate::FormsFill>(),
            output: schema::<crate::FormsFilled>(),
            plan: Some(schema::<crate::FormsFilled>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İki binayı aynı denetim durumuyla işaretle".into(),
                input: json!({ "form": "bina-denetimi", "uids": ["0192f5a1-25a0-7000-8000-000000000201", "0192f5a1-25a0-7000-8000-000000000202"], "values": { "Durum": "iyi" } }),
                output: None,
            }],
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
        // A derived GIS dataset against the drawing it was made from (docs/adr/0530), fixtures/commands/v1/cad.gis.derivation.json.
        CommandDescriptor {
            id: crate::CAD_GIS_DERIVATION.into(),
            version: crate::CAD_GIS_DERIVATION_VERSION,
            title: "GIS türevini denetle".into(),
            summary: "path'teki, file.export.gis'in yazdığı GeoPackage'ın her satırını açık çizimde kaynağı olan nesneyle karşılaştırır; hiçbir şey yazmaz. \
                      Satırın kalıcı kimliği (kentos_uids, yoksa kentos_uid sütunu) çizimdeki nesneyi bulur. Nesnenin türev özeti (kentos_digest: bakışı dışında bütün alanlarının SHA-256'sı, blokta tanımın öznitelikleri de) satırınkinden başkaysa satır stale'dir: kaynak dışa aktarmadan sonra değişti, türev eski. \
                      Özet aynıysa satırın geometrisi ve öznitelik sütunları tablonun kaydındaki algoritma ve toleransla yeniden türetilen satırla karşılaştırılır (koordinatlar 1e-9 m içinde); farklıysa satır edited'dır: GIS'te değiştirilmiş, kaynak nesne değişmemiş; hangi alanların (geometry, öznitelik adları) farklı olduğu fields'tadır. \
                      Kimliği çizimde olmayan satır source_gone, tablonun katmanında olup satırı olmayan nesne not_exported'tur. Bir tablonun kaydındaki sapma kaydındaki toleransın üstündeyse over_tolerance sayılır. \
                      Cevap her tablonun sayılarını, toplamları, dosyanın kaydettiği kaynağı (proje, ad, sürüm) ve eşleşmeyen satırlar önce olmak üzere en çok limit satırı listeler. Türev, kaynağın yerine geçmez: stale ve edited satırlar çizimi değiştirmez ve GIS'teki değişiklik çizime geri yazılmaz."
                .into(),
            aliases: vec!["turevdenetle".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::GisDerivation>(),
            output: schema::<crate::GisDerived>(),
            plan: Some(schema::<crate::GisDerived>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Dışa aktarılan türev hâlâ güncel mi".into(),
                input: json!({ "path": "/veri/parsel-turev.gpkg" }),
                output: None,
            }],
        },
        // What a GIS changed in a derived dataset written back (docs/adr/0634 §1), fixtures/commands/v1/cad.gis.takeback.json.
        CommandDescriptor {
            id: crate::CAD_GIS_TAKEBACK.into(),
            version: crate::CAD_GIS_TAKEBACK_VERSION,
            title: "GIS'ten geri al".into(),
            summary: "file.export.gis'in yazdığı türevde GIS'te değiştirilmiş satırları kaynağı olan nesnelere yazar, tek geri alma adımında (“GIS'ten geri al”). \
                      Yalnız cad.gis.derivation'ın edited dediği satır alınır: kaynağı dışa aktarmadaki gibi (özeti satırınkiyle aynı), satır türevin verdiğinden farklı. Kaynağı da değişen satır row_stale ile reddedilir: iki yan da değişti, GIS'teki değişiklik çizimdekinin üstüne yazılmaz. \
                      take: attributes (varsayılan) farklı sütunları kaynağın özniteliklerine yazar (katmanın alanının kuralıyla; yazıda metin, yükseklik, dönüş; blok yerleştirmesinde ölçek ve dönüş), geometri olduğu gibi kalır (geometry_left). \
                      take: geometry sütunları ve geometriyi alır: düz geometrili kaynak GIS'tekini olduğu gibi alır; kaynağı bir eğri olan (daire, yay, bükümlü kenar, elips, eğri, NURBS) nesne kirişleri alır ve analitik tanımı bırakılır, bağı kopar (link_broken, satırın broken'ı eski tür). \
                      Ardından satırlar eski (stale) olur: türevi yeniden dışa aktarın."
                .into(),
            aliases: vec!["turevdengerial".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::GisTakeback>(),
            output: schema::<crate::GisTaken>(),
            plan: Some(schema::<crate::GisTaken>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "GIS'te düzeltilen parselin özniteliklerini çizime al".into(),
                input: json!({ "path": "/veri/parsel-turev.gpkg", "rows": [{ "table": "Parsel", "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001" }] }),
                output: None,
            }],
        },
        // A derived table made the GIS's own data (docs/adr/0634 §2), fixtures/commands/v1/cad.gis.detach.json.
        CommandDescriptor {
            id: crate::CAD_GIS_DETACH.into(),
            version: crate::CAD_GIS_DETACH_VERSION,
            title: "GIS türevinin bağını kopar".into(),
            summary: "file.export.gis'in yazdığı türevin tablolarını (tables boşsa bütün türev tablolarını) GIS'in kendi verisi yapar: her tablonun türetim kaydı, ne olduğunu, ne zaman ve neden koparıldığını söyleyen bir notla (https://kentos.app/derivation-detached/1) değişir ve satırları çizimin kalıcı kimliklerini bırakır, tek SQLite işleminde. \
                      Ardından tablo her tablo gibi bağlanır, okunur ve içe aktarılır; satırları yeni nesnelerdir, kaynakları sayılmaz; cad.gis.derivation onu artık saymaz. Çizim değişmez ve geri alma adımı değildir: kopan bağ geri gelmez, türev yeniden dışa aktarılır. \
                      Satırların sütunları (kentos_uid, kentos_digest de) olduğu gibi kalır. plan yazmadan söyler."
                .into(),
            aliases: vec!["turevbaginikopar".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::GisDetach>(),
            output: schema::<crate::GisDetached>(),
            plan: Some(schema::<crate::GisDetached>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsel tablosu artık GIS'in".into(),
                input: json!({ "path": "/veri/parsel-turev.gpkg", "tables": ["Parsel"], "reason": "Kadastro güncellemesi GIS'te sürecek" }),
                output: None,
            }],
        },
        // Bound layers (docs/adr/0503), held together by fixtures/commands/v1/cad.sources.*.json.
        CommandDescriptor {
            id: crate::CAD_SOURCES_DESCRIBE.into(),
            version: crate::CAD_SOURCES_DESCRIBE_VERSION,
            title: "Kaynakları tanı".into(),
            summary: "path verilirse o GeoPackage'ın obje ve öznitelik tablolarını söyler: satır sayısı, geometri türü, bildirdiği EPSG, sütunlar (dosyadaki bildirimi ve alan türü) ve neler yapılabildiği (okuma, ekleme, güncelleme, silme, tek işlem, Z, M, eğri). \
                      path verilmezse projenin bağlı katmanlarını söyler: bağ, dosya yerinde mi (found), dosya katman onunla son eşleştiğinden beri değişti mi (changed), katman değişti mi (edited) ve katmanın tablodan farkı: tabloda olmayan (added), ikisinde olup farklı (modified) ve katmanda olmayan (removed) nesne sayıları. \
                      Nesne ile satır kalıcı kimlikle eşlenir, satır olarak karşılaştırılır (koordinatlar 0,001 mm içinde). Hiçbir şey yazmaz. \
                      path bir PostGIS adresiyse (postgresql://kullanıcı@sunucu:5432/veritabanı; parolasız: invalid_address) veritabanının tek tam sayı anahtarlı tablolarını şema.tablo adlarıyla söyler; ulaşılamayan veritabanı source_unreachable, bağlı katmanında found false."
                .into(),
            aliases: vec!["kaynaklar".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::SourcesDescribe>(),
            output: schema::<crate::SourcesDescribed>(),
            plan: Some(schema::<crate::SourcesDescribed>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir dosyanın tabloları".into(),
                    input: json!({ "path": "/veri/kadastro.gpkg" }),
                    output: None,
                },
                CommandExample {
                    title: "Projenin bağlı katmanları".into(),
                    input: json!({}),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SOURCES_EDIT.into(),
            version: crate::CAD_SOURCES_EDIT_VERSION,
            title: "Kaynağa bağla".into(),
            summary: "Bir katmanı bir GeoPackage tablosunun çalışma kopyası yapar ya da bağı değiştirir. link: layer verilmezse tablonun satırları yeni bir katmana alınır (file.import.gpkg gibi, tek geri alma adımı; boş tablo boş katman); layer verilirse o katman tablonun nesnelerini olduğu gibi taşımalıdır (source_differs). \
                      Başka sistemdeki tablo yalnız reproject ile bağlanır (yoksa crs_mismatch): katmanın nesneleri projenin sisteminde, tablonunkiler kendi sisteminde kalır; eşitleme her seferinde çevirir, yazma tablonun sistemine geri çevirerek yazar, yaylı nesneler kesin çevrilemediği için bağlanmaz ve tabloda dokunulmadan kalır. Bağ dosyanın SHA-256'sını ve katmanın özetini tutar; projenin ayarıdır (.kcad şema 74), geri alma adımı değildir. \
                      extent [minX, minY, maxX, maxY] (projenin sisteminde) yeni katmana bağlamada yalnız kutusu o alana değen satırları alır: büyük tablonun bir parçasının çalışma kopyası; eşitleme alanın dışındaki satırları silinmiş saymaz, alanın dışına taşınan nesne kendi satırına yazılır; ters alan, var olan katman, geometrisiz tablo ve veritabanı invalid_extent. \
                      unlink bağı çözer, nesneler kalır. relink taşınan dosyanın yeni yerini verir; dosya aynı (aynı SHA-256) olmalıdır (source_changed). Bir tablo ve bir katman tek bağa (source_bound), en çok 1000 bağ. \
                      path bir PostGIS adresi de olabilir (postgresql://kullanıcı@sunucu:5432/veritabanı, table şema.tablo): parola adrese yazılmaz (invalid_address), PGPASSWORD ya da ~/.pgpass'tan okunur; tablo tek tam sayı birincil anahtarlı olmalıdır; ulaşılamayan veritabanı source_unreachable."
                .into(),
            aliases: vec!["kaynakbagla".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::SourcesEdit>(),
            output: schema::<crate::SourcesEdited>(),
            plan: Some(schema::<crate::SourcesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parsel tablosunu yeni katman olarak bağla".into(),
                    input: json!({ "edit": { "kind": "link", "path": "/veri/kadastro.gpkg", "table": "parsel" } }),
                    output: None,
                },
                CommandExample {
                    title: "Taşınan dosyanın yeni yeri".into(),
                    input: json!({ "edit": { "kind": "relink", "id": "kaynak-1", "path": "/arsiv/kadastro.gpkg" } }),
                    output: None,
                },
            ],
        },
        // Toplu yazma of the attribute table (docs/adr/0507), held together by fixtures/commands/v1/cad.fields.write.json.
        CommandDescriptor {
            id: crate::CAD_FIELDS_WRITE.into(),
            version: crate::CAD_FIELDS_WRITE_VERSION,
            title: "Alanlara toplu yaz".into(),
            summary: "Bir katmanın nesnelerine öznitelik değerlerini tek geri alma adımında yazar. calculate (Alan hesapla): expression'ın (İfadeyle seç'in dili; geometri değerleri projenin çizim biriminde, $sıra nesnelerin sırası) her nesnedeki değeri field'a; uids boşsa katmanın bütün nesneleri; boş sonuçta değer kalır (empty: keep) ya da kalkar (clear). values (Yapıştır): her satırın değerleri kendi nesnesine, null değeri kaldırır. \
                      Her değer cad.entities.set'in kurallarıyla denetlenir: alanın türü ve kuralları (kanonik metne çevrilir), zorunlu alan, alt türler, değer bileşimleri, kayıt kuralları, tekil alanlar (yazılanlar birlikte). Alanı olan katmanda alanların adlarıyla yazılır (field_not_found). \
                      policy all (varsayılan): uymayan bir değer bütün yazmayı reddeder (invalid_values); policy valid: uyanlar yazılır, öbürleri values_skipped ile söylenir. Bir değeri gösteren etiket yeni değeri gösterir. \
                      plan yazmadan değişecekleri (nesne, alan, eski ve yeni değer; ilk 200) ve sorunları (nesne, alan, değer, kod, ileti) sayılarıyla verir; policy all'da da reddetmez, sorunları listeler. Adım “Alan hesapla: alan” ya da “Yapıştır”."
                .into(),
            aliases: vec!["alanhesapla".into(), "toplu-yaz".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::FieldsWrite>(),
            output: schema::<crate::FieldsWritten>(),
            plan: Some(schema::<crate::FieldsWritten>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parsellerin alanını iki ondalıkla yaz".into(),
                    input: json!({ "layer": "parsel", "write": { "kind": "calculate", "field": "Alan", "expression": "metin($alan, 2)" } }),
                    output: None,
                },
                CommandExample {
                    title: "Elektronik tablodan yapıştır".into(),
                    input: json!({ "layer": "parsel", "write": { "kind": "values", "rows": [
                        { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2401", "values": { "Ada": "101", "Parsel": "5" } }
                    ] }, "policy": "valid" }),
                    output: None,
                },
            ],
        },
        // İşlemler's tools as commands (docs/adr/0508).
        CommandDescriptor {
            id: crate::CAD_PROCESSING_TOOLS.into(),
            version: crate::CAD_PROCESSING_TOOLS_VERSION,
            title: "İşlem araçları".into(),
            summary: "İşlemler'in araçlarını parametreleriyle listeler: kimlik, ad, kategori yolu, açıklama, takma adlar; her parametrenin adı, etiketi, türü (features, number, text, boolean, choice, layer, point, expression, field, file, saveFile, network, addressSource, rasterValues, rasterPairs, fieldMap, layers), isteğe bağlılığı, bu çizimdeki varsayılanı ve seçimin değerleri. \
                      query adı, açıklaması, anahtar sözcükleri ya da kategorisi bütün sözcükleri tutanları (Türkçe harfler katlanır), kimliği ya da takma adı tam yazılanı bulur; category (kimliği) o kategorinin ve alt kategorilerinin araçlarını. Bilinmeyen kategori invalid_input. models: true modelleri de listeler (kimliği model: ile başlar, parametreleri girdileridir). Çizimi değiştirmez."
                .into(),
            aliases: vec!["islemler".into(), "araclar".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProcessingTools>(),
            output: schema::<crate::ProcessingToolsListed>(),
            plan: Some(schema::<crate::ProcessingToolsListed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Geometri araçları".into(),
                input: json!({ "category": "geometry" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_PROCESSING_RUN.into(),
            version: crate::CAD_PROCESSING_RUN_VERSION,
            title: "İşlem aracını çalıştır".into(),
            summary: "İşlemler'in bir aracını açık değerlerle çalıştırır; masaüstünün İşlemler penceresinin çalıştırıcısı, tek geri alma adımı (adı aracın adı). Verilmeyen parametre bu çizimdeki varsayılanını alır. \
                      Nesne girdisi { scope: all } (görünen katmanlar), { scope: layer, layerId }, { scope: uids, uids }, { scope: selection } (girdinin selection'ının adlandırdığı nesneler) ya da { scope: visible } (view kutusundakiler), isteğe bağlı kinds ile; nesne girdisinin varsayılanı seçimdir. Dosya parametresi { path } (tablo dosyası pencerenin okuduğu gibi okunur) ya da { name, rows }. Ekranın seçimi ve görünümü okunmaz: yalnız girdinin selection'ı ve view'u. \
                      Retler sırayla: tools_unavailable, unknown_tool, invalid_uid, entity_not_found, invalid_parameters (aracın olmayan parametresi, kuralına uymayan değer, okunmayan dosya, view'suz görünen alan; parametresiyle), tool_failed (araç çalıştı ve durdu: kilitli katman, girdi yok, aracın kendi reddi; çizim değişmez). \
                      Cevap aracın özeti, eklediği ve değiştirdiği nesneler (kalıcı kimlikleriyle), seçtikleri, açtığı katmanlar, çıktıları (nesne çıktıları kalıcı kimliklerle, sayılar, metinler, tablolar), notları ve adımın adı. plan aracı çizimin bir kopyasında çalıştırır: aynı algoritmayla ne yazılacağını söyler, yeni nesnelerin kimlikleri kopyanındır. Raster araçları başsız ortamda rasterin bağlı ya da gömülü GeoTIFF ve PNG dosyasını okur, sonucu COG olarak yazar; adresteki raster, JPEG, NetCDF ve nokta bulutu dosyaları yalnız masaüstünde, başsız ortamda tool_failed. \
                      sink { path, overwrite } sonucu çizim yerine yeni bir GeoPackage'a yazar: araç kopyada çalışır, yaptığı nesnelerin katmanları birer tablo (adı katmanın adı, alanları sütun, kimlikler kentos_uids'te), çizim değişmez; dosya geçici adla yazılıp tek adımla yerine konur, var olan dosya yalnız overwrite ile değişir; retler invalid_file_path, file_exists, file_unwritable, nothing_to_export, GeoPackage'ın tutmadığı nesneler export_losses; cevabın sink'i yolu, boyu, SHA-256'sı ve tabloları, inputs'u okunan katmanlar, içeriklerinin özeti ve bağlı katmanların kaynak sürümleri; her tablo soyunu GeoPackage'ın metadata uzantısında taşır (https://kentos.app/lineage/1: araç, istek, girdiler ve özetleri, motor, zaman). \
                      sink.mode: new (varsayılan: yeni dosya), addTables (var olan GeoPackage'a tablo ekler; aynı adlı tablo table_exists ile reddedilir, dosyanın başka tablosuna dokunulmaz) ya da replaceTables (aynı adlı nesne ya da öznitelik tablosunu yenisiyle değiştirir; başka tabloya dokunulmaz); mode new dışında dosya var olmalı (file_not_found), GeoPackage olarak okunmalı (import_unreadable), çizimin koordinat sistemini taşımalı (crs_mismatch), overwrite verilmez (invalid_input). Tablolar tek SQLite işleminde, beş bin satırlık parçalarla yazılır; hata ya da durdurma (iş olarak çalışırken) dosyayı olduğu gibi bırakır. sink.tables aracın tablo çıktılarını (özet, rapor) da birer öznitelik tablosu olarak yazar: adı çıktının etiketi, sütunları başlıkları (metin; hücreler aracın gösterdiği yazı), sinkReport'ta output adıyla görünür; yalnız tablo üreten araç yalnız onu yazar. Cevabın sinkReport'u kipi, her tablonun satırlarını, geometri türünü, Z ve M'ini, sütunlarının öznitelikleriyle eşlemesini, dosyanın dokunulmayan tablolarını ve parça sayısını söyler; satırların kalıcı kimlikleri kentos_uids tablosundadır. \
                      tool “model:” ve modelin kimliği de olabilir (model:builtin.parcelSheet; cad.processing.tools'un models'i listeler): modelin girdileri parametreleridir, adımları sırayla çalışır, hepsi tek geri alma adımıdır (adı modelin adı); bir adım çalışmazsa önceki adımların sonuçları geri alınır, hiçbir şey yazılmaz. \
                      Cevabın record'u koşunun kaydıdır (kentos.run-record 1): araç ya da model ve sürümü, aldığı değerler (varsayılanlar doldurulmuş, nesneler kalıcı kimliklerle: aynen verilince aynı işi yeniden çalıştırır), girdi katmanları ve içerik özetleri, çizimin sürümü, projenin koordinat sistemi, varsa tohum, motor ve ev sahibi, başlangıç zamanı ve süresi, her adımın girdi ve çıktı kayıt sayısı ve süresi, kesin toplam. Kayıt proje dosyasına yazılmaz (yazılsa kendisi bir düzenleme olurdu, çizimin sürümünü oynatırdı); requestId verilen çağrının cevabı, kaydıyla, proje dosyasının istek defterinde kalır (256 istek, her cevap 64 KB'a kadar) ve sink'e yazılan her tablo kaydı soyunun run alanında taşır. İş olarak çalışırken durdurma aracın kendi soru noktalarında da duyulur (iptal: çizim ve dosya olduğu gibi); aracın kendisi sormuyorsa araç bitince duyulur."
                .into(),
            aliases: vec!["islem".into(), "process".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProcessingRun>(),
            output: schema::<crate::ProcessingRan>(),
            plan: Some(schema::<crate::ProcessingRan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Seçili parsellerin köşelerini numarala".into(),
                    input: json!({ "tool": "points.numberVertices", "selection": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2401"] }),
                    output: None,
                },
                CommandExample {
                    title: "Parsel ve imar kesişimi dosyaya".into(),
                    input: json!({ "tool": "geometry.intersection", "parameters": { "input": { "scope": "layer", "layerId": "parsel" }, "overlay": { "scope": "layer", "layerId": "imar" } }, "sink": { "path": "/is/kesisim.gpkg" } }),
                    output: None,
                },
                CommandExample {
                    title: "Bir katmanın nesnelerine tampon".into(),
                    input: json!({ "tool": "geometry.buffer", "parameters": { "input": { "scope": "layer", "layerId": "parsel" }, "distance": 5 } }),
                    output: None,
                },
            ],
        },
        // Toplu çalıştırma (docs/adr/0529).
        CommandDescriptor {
            id: crate::CAD_PROCESSING_BATCH.into(),
            version: crate::CAD_PROCESSING_BATCH_VERSION,
            title: "İşlem aracını toplu çalıştır".into(),
            summary: "İşlemler'in bir aracını ya da modelini (tool, model: kimliğiyle) bir parametre tablosunun satırlarıyla sırayla çalıştırır: her satır cad.processing.run'ın aynı aracı, aynı çalıştırıcısı, aynı retleriyle koşar; parameters bütün satırların ortak değerleridir, rows[i].parameters satırın kendi değerleri onların üstüne gelir (örnek: aynı araç birkaç girdi katmanına ya da birkaç tampon uzaklığına); selection ve view her satıra aynen verilir. \
                      Satırlar sırayla çalışır, her biri kendinden öncekilerin yazdığını görür. Bütün toplu iş tek geri alma adımıdır (adı “Toplu: <araç>”) ve varsayılan onError: stop ile ya hep ya hiç yazılır: bir satır çalışıp dururlarsa ya da durdurulursa (iş olarak çalışırken: satırlar arasında ve çalışan aracın içinde) önceki satırların sonuçları geri alınır, hiçbir şey yazılmaz. onError: continue ile çalışırken duran satır tek başına geri alınır (modelin önceki adımlarını da), öteki satırlar yazılır, yine tek adımda; duran satırlar cevabın failed listesinde (index, label, code, message) ve kaydın failed'ında, batch_rows_failed uyarısıyla söylenir; bütün satırlar durursa hiçbir şey yazılmaz, adım yoktur. Durdurma (iptal) hiçbir kipte satır hatası değildir: bütün toplu iş geri alınır. En çok 500 satır. \
                      Retler sırayla: tools_unavailable, empty_batch, too_many_rows, sonra her satır için cad.processing.run'ın retleri (unknown_tool, invalid_uid, entity_not_found, invalid_parameters), yolu rows[i]. ile başlar; çalışırken batch_row_failed (onError: stop: satır çalıştı ve durdu ya da değerleri önceki satırların yazdıklarına artık uymuyor; ileti satırı söyler); çalıştırmadan önceki retler onError: continue ile de ret kalır. \
                      Cevap her satırın cad.processing.run cevabı (kendi kaydıyla), toplu işin özeti ve kaydı (kind batch, adımları satırlar) ve adımın adıdır. plan aynısını çizimin okuma kopyasında çalıştırır, çizim değişmez; sink bu komutta yoktur."
                .into(),
            aliases: vec!["topluislem".into(), "batch".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ProcessingBatch>(),
            output: schema::<crate::ProcessingBatched>(),
            plan: Some(schema::<crate::ProcessingBatched>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Aynı aracı üç tampon uzaklığıyla çalıştır".into(),
                    input: json!({ "tool": "geometry.buffer", "parameters": { "input": { "scope": "layer", "layerId": "parsel" } }, "rows": [
                        { "label": "5 m", "parameters": { "distance": 5 } },
                        { "label": "10 m", "parameters": { "distance": 10 } },
                        { "label": "20 m", "parameters": { "distance": 20 } }
                    ] }),
                    output: None,
                },
                CommandExample {
                    title: "Parsel ölçü yazıları modelini iki ön ekle çalıştır".into(),
                    input: json!({ "tool": "model:builtin.parcelSheet", "selection": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2401"], "rows": [
                        { "parameters": { "prefix": "A" } },
                        { "parameters": { "prefix": "B" } }
                    ] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SOURCES_SYNC.into(),
            version: crate::CAD_SOURCES_SYNC_VERSION,
            title: "Kaynakla eşitle".into(),
            summary: "Bağlı katmanı tablosuyla eşitler. write: katmanın değişikliklerini tabloya yazar, tek SQLite işleminde: yeni nesneler kalıcı kimlikleriyle eklenir (kentos_uids), değişenler fid'leriyle güncellenir, silinenler silinir, katmanın tabloda olmayan alanları sütun olarak eklenir; mekânsal dizin kendi tetikleyicileriyle izler, tanımadığı tablolara dokunulmaz. \
                      Dosya katman onunla son eşleştiğinden beri değiştiyse yazılmaz (source_changed, conflict); geometrisi tablonun sütununa uymayan nesne geometry_mismatch (Z ve M de: Z'yi yasaklayan sütuna kotlu, isteyene kotsuz, hiçbirine kısmen kotlu nesne yazılmaz; M için de aynı, docs/adr/0584 §2); tablonun tutmadığı nesneler (yazı, ölçü) ve alanı olmayan öznitelikler çizimde kalır (export_losses). Yazma geri alma adımı değildir: dosya yazılır. \
                      read: tablonun satırlarını katmana alır, tek geri alma adımı (“Kaynaktan oku: tablo”): değişen nesnelerin görünüşü kalır, yeni satırlar eklenir, tabloda olmayanlar silinir. Katman son eşleşmeden beri değiştiyse discard olmadan okunmaz (pending_changes, conflict). \
                      PostGIS tablosu da aynı kurallarla, tek veritabanı işleminde yazılır: tablo yazarlara kilitlenir, sürümü yeniden denetlenir, kalıcı kimlikler şemanın kentos_uids tablosunda; ulaşılamayan veritabanı source_unreachable. \
                      with (yalnız write) başka bağlı katmanları aynı işlemde yazar: üst kayıt ile alt kayıtları ya hep birlikte ya hiç (docs/adr/0532): her katman önce denetlenir, biri reddedilirse (source_readonly, geometry_mismatch, source_changed, relation_dangling) hiçbiri yazılmaz; katmanlar tek GeoPackage dosyasında ya da tek PostGIS veritabanında olmalıdır (docs/adr/0584 §1: veritabanında tablolar adlarının sırasıyla kilitlenir, her birinin satırları tabanına karşı denetlenir; iki kaynağa birden tek işlem olmaz, transaction_split); cevabın layers'ı her katmanın payı, relations'ı eşli ilişkilerin eşleme satırlarıdır. \
                      cad.sources.relate ile eşlenen çoktan çoğa ilişki katmanlarıyla birlikte yazılır: eşleme tablosunun satırları alt katmanın çok değerli alanını izler, alan metin sütunu olarak yazılmaz; eşlenmemiş çoktan çoğa başvuru “; ” ile birleşmiş metin olarak kalır ve relation_unmapped ile söylenir, dosyanın tanınmayan temsilli ilişkisi metne çevrilmez. read eşli ilişkinin alanını eşleme satırlarından doldurur. \
                      plan dosyayı okur, hiçbir şey yazmaz ve sayıları söyler."
                .into(),
            aliases: vec!["kaynakesitle".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::SourcesSync>(),
            output: schema::<crate::SourcesSynced>(),
            plan: Some(schema::<crate::SourcesSynced>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parsel katmanının değişikliklerini dosyaya yaz".into(),
                    input: json!({ "layer": "import-parsel", "direction": "write" }),
                    output: None,
                },
                CommandExample {
                    title: "Başkasının değiştirdiği tabloyu yeniden oku".into(),
                    input: json!({ "layer": "import-parsel", "direction": "read" }),
                    output: None,
                },
                CommandExample {
                    title: "Parseli ve maliklerini aynı işlemde yaz".into(),
                    input: json!({ "layer": "parsel", "direction": "write", "with": ["malik"] }),
                    output: None,
                },
            ],
        },
        // Both sides of a bound layer changed (docs/adr/0538), held together by
        // fixtures/commands/v1/cad.sources.reconcile.json.
        CommandDescriptor {
            id: crate::CAD_SOURCES_RECONCILE.into(),
            version: crate::CAD_SOURCES_RECONCILE_VERSION,
            title: "Kaynakla uzlaştır".into(),
            summary: "Hem katmanı hem tablosu son eşleşmeden beri değişen bağlı katmanı uzlaştırır. Bağın tabanı (son eşleşmede her nesnenin katmandaki ve tablodaki özeti, docs/adr/0538) ile karşılaştırılır: yalnız katmanın değiştirdiği nesne tabloya yazılır, yalnız tablonun değiştirdiği (QGIS'te, başka bir KentOS'ta) katmana alınır, ikisinin aynı biçimde değiştirdiği uyuşur. \
                      İkisinin de değiştirdiği nesne tabana karşı alan alan birleştirilir (docs/adr/0553): yalnız bir yanın değiştirdiği öznitelik ve geometri o yanınkini alır, iki yan birden farklı değiştirdiği öznitelik ya da geometri çatışmadır (iki yanın değerleriyle); nesne, tabanın alan kaydı yoksa bütün olarak çatışır (field_base_missing). Birinin silip öbürünün değiştirdiği nesne de çatışmadır. \
                      Birlikte değişen nesneler bir gruptur ve birlikte, tek yanla çözülür: ortak köşesini iki yanın da taşıdığı komşular ve bir yanın sildiği üst kayıtla öbür yanın eklediği ya da değiştirdiği alt kayıt (ilişki). Son yazan kazanmaz: take (nesne ya da grup için) ve rest (kalanlar) ile bir yan seçilir; bir grubun nesneleri farklı yanlara seçilemez (group_split). \
                      with ile aynı GeoPackage dosyasının başka bağlı katmanları aynı çağrıda ve tek SQLite işleminde uzlaştırılır (başka dosyadaki katman transaction_split). \
                      plan dosyayı okur, hiçbir şey yazmaz ve hepsini söyler. Yürütme bütün çatışmalar çözülmeden yapılmaz (unresolved_conflicts, conflict); çözülünce tablonun yanı katmana alınır, katmanın yanı tabloya yazılır: tek geri alma adımı (“Uzlaştır: tablo”) ve tek işlem, dosyanın yazılamaması ikisini de geri alır. \
                      Tabanı olmayan bağ (bu sürümden önce bağlanan, MAX_BASE_ROWS'tan çok nesneli katman) uzlaştırılamaz (no_source_base): önce yazın ya da okuyun. Alan sınırlı çalışma kopyasında yalnız alandaki satırlar karşılaştırılır."
                .into(),
            aliases: vec!["kaynakuzlastir".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::SourcesReconcile>(),
            output: schema::<crate::SourcesReconciled>(),
            plan: Some(schema::<crate::SourcesReconciled>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parsel katmanıyla tablonun farkını gör".into(),
                    input: json!({ "layer": "import-parsel" }),
                    output: None,
                },
                CommandExample {
                    title: "Bir çatışmada tablonun hâlini, kalanlarda katmanınkini al".into(),
                    input: json!({ "layer": "import-parsel", "take": [{ "object": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2401", "side": "table" }], "rest": "layer" }),
                    output: None,
                },
                CommandExample {
                    title: "Parsel ve bina katmanlarını tek çağrıda, tek işlemde uzlaştır".into(),
                    input: json!({ "layer": "import-parsel", "with": ["import-bina"], "rest": "table" }),
                    output: None,
                },
            ],
        },
        // A many-to-many reference in a bound GeoPackage (docs/adr/0532), held together by
        // fixtures/commands/v1/cad.sources.relate.json.
        CommandDescriptor {
            id: crate::CAD_SOURCES_RELATE.into(),
            version: crate::CAD_SOURCES_RELATE_VERSION,
            title: "İlişkiyi dosyaya eşle".into(),
            summary: "Projenin bir ilişkisinin (cad.relations.edit) çok değerli alanı (alt kayıt birden çok üstün kimliğini “; ” ile tutar, docs/adr/0409) bağlı GeoPackage'da nasıl tutuluyor: iki katman da aynı dosyanın tablolarına bağlıysa metin sütunu (text), KentOS'un ilişki temsili (mapped), dosyanın başka bir temsilli ilişkisi (foreign) ya da profile uymayan ilişkisi (broken); bağlı olmayan (unbound), ayrı kaynaklardaki (split) ve tek kimlikli (single) ilişki. \
                      check hiçbir şey yazmaz, durumu ve nedenini söyler. map GeoPackage ilişki profilinde (OGC 18-000, gpkgext_relations) “kentos_references” ilişkisini ve eşleme tablosunu (base_id: alt kaydın fid'i, related_id: üstün fid'i) tek SQLite işleminde yazar; satırlar katmanların alandaki başvurularından kurulur. Eşlenen alan sütun olarak yazılmaz ve okunmaz, eşleme tablosu tek temsildir; cad.sources.sync eşleme satırlarını katmanlarla birlikte yazar ve okur. Dosyanın başka bir temsilli ilişkisi ya da eşleme tablosu olmayan bir tablo üzerine yazılmaz (relation_unsupported), noktalı virgüllü metne de çevrilmez. \
                      unmap ilişkiyi ve eşleme tablosunu dosyadan kaldırır; başvurular katmanda kalır ve sonraki yazmada metin sütunu olur (relation_unmapped uyarısıyla). Dosya katmanlar onunla son eşleştiğinden beri değiştiyse yazılmaz (source_changed, conflict). Bağ ve dosya revizyonları güncellenir; geri alma adımı değildir (dosya yazılır). plan yazmadan söyler."
                .into(),
            aliases: vec!["iliskiesle".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::SourcesRelate>(),
            output: schema::<crate::SourcesRelated>(),
            plan: Some(schema::<crate::SourcesRelated>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parselin maliklerinin dosyada nasıl tutulduğunu söyle".into(),
                    input: json!({ "relation": "parsel-malik", "action": "check" }),
                    output: None,
                },
                CommandExample {
                    title: "Parsel-malik ilişkisini GeoPackage ilişki profiline eşle".into(),
                    input: json!({ "relation": "parsel-malik", "action": "map" }),
                    output: None,
                },
            ],
        },
        // A layer's records, or a relation's, a page at a time (docs/adr/0532), held together by
        // fixtures/commands/v1/cad.records.page.json.
        CommandDescriptor {
            id: crate::CAD_RECORDS_PAGE.into(),
            version: crate::CAD_RECORDS_PAGE_VERSION,
            title: "Kayıtları sayfa sayfa oku".into(),
            summary: "Bir katmanın kayıtlarını (layer) ya da bir ilişkinin bir kayda bağladığı kayıtları (relation ve of; side children: of üsttür, alt katmanın alanı onu tutan kayıtlar; parents: of alttır, üst katmanın alanın tuttuğu kayıtları alanın sırasıyla) sayfa sayfa okur: kalıcı kimlik sırasıyla (cursor önceki sayfanın next'i), sayfada 1–500 kayıt (varsayılan 50). \
                      total sayfalardan bağımsız eşleşen kayıtların sayısıdır ve katman süzgeçlerinden (cad.layers.filter) sonra sayılır: süzgecin dışarıda bıraktığı nesne ne kayıt ne toplam olarak görünür. query kaydın öznitelik değerlerinde bütün sözcükleri arar (büyük küçük harf Türkçe kuralıyla denk sayılır: I ı, İ i; harfler sadeleştirilmez, “ayse” Ayşe'yi bulmaz); fields yalnız adı verilen öznitelikleri döndürür. \
                      Katman bir kaynağa bağlıysa source kaynağın sürümünü söyler: id, tablo, revision (katman kaynakla son eşleştiğinde dosyanın SHA-256'sı) ve edited (katmanda kaynağın bilmediği değişiklik var: sayfa çalışma kopyasının, kaynağın değil). expectedRevision ve expectedSource ilk sayfanın söylediklerini geri verir: çizim ya da kaynak değiştiyse sonraki sayfa okunmaz (revision_conflict, source_changed; conflict), böylece sayfalar aynı verinin sayfalarıdır. Çizimi değiştirmez."
                .into(),
            aliases: vec!["kayitlar".into(), "kayitsayfa".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::RecordsPage>(),
            output: schema::<crate::RecordsPaged>(),
            plan: Some(schema::<crate::RecordsPaged>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Maliklerin ilk sayfası".into(),
                    input: json!({ "layer": "malik", "limit": 50 }),
                    output: None,
                },
                CommandExample {
                    title: "Bir parselin malikleri".into(),
                    input: json!({ "relation": "parsel-malik", "of": "0192f5a1-25a0-7000-8000-000000000301", "side": "parents" }),
                    output: None,
                },
            ],
        },
        // A question asked of a data set where it is (docs/adr/0581), held together by
        // fixtures/commands/v1/cad.sources.query.json.
        CommandDescriptor {
            id: crate::CAD_SOURCES_QUERY.into(),
            version: crate::CAD_SOURCES_QUERY_VERSION,
            title: "Kaynağı sorgula".into(),
            summary: "Bir veri kümesini bulunduğu yerde sorgular: path ve table ile bir GeoPackage dosyasının ya da bir PostGIS adresinin (postgresql://…, parolasız) tablosunu, layer ile çizimin bir katmanını (scope loaded: çizimin nesneleri, bağlı katmanda çalışma kopyası; scope source: bağlı katmanın kaynağının bütün tablosu). \
                      bbox (projenin sisteminde; kaynağın sistemi başkaysa ona çevrilir) kutusu değen kayıtları, near (at, tolerance) yere tolerans içindeki kayıtları (tanımlama), filter İfadeyle seç'in dilinde koşulu tutan kayıtları verir. fields döndürülecek alanları, geometry kaydın geometrisini projenin sisteminde ISO WKT olarak seçer. \
                      Kayıtlar anahtar sırasıyla sayfa sayfadır (limit 0–10 000, varsayılan 100; cursor önceki sayfanın next'i); anahtar sırası ekleme ve silmede kararlıdır. count sorunun tuttuğu bütün kayıtları sayar; statistics alanların istatistiğini (kentos.statistics/1: okunan, atlanan, toplam, ortalama, en az, en çok, standart sapma; kesin ondalık metinler) sayfadan değil bütün kayıtlardan verir. \
                      Kutu, sütunlar ve süzgecin kaynağın denetleyebildiği kısmı kaynağa gider: koşul ifadenin tuttuğu her kaydı tutar, kaynaktan gelen her kayıt ifadeyle burada yeniden sınanır; sonuç kaynağın kurallarından bağımsızdır. plan ne yapıldığını söyler: kutu (index, geometry, local), okunan sütunlar, kaynağa giden koşul değerleriyle ve tam olup olmadığı, burada yapılanlar, sıra. \
                      Kayıtların kalıcı kimliği (uid) aynı kaynağı açan her pencerede ve bağlı katmanın çalışma kopyasında aynıdır. Parçası bağlı katmanın çalışma kopyası tam veri kümesi değildir: partial ve partial_scope uyarısı. Çizimi değiştirmez."
                .into(),
            aliases: vec!["kaynaksorgu".into(), "sourcequery".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::SourcesQuery>(),
            output: schema::<crate::SourcesQueried>(),
            plan: Some(schema::<crate::SourcesQueried>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Büyük parsellerin ilk sayfası, dosyadan".into(),
                    input: json!({ "path": "/veri/kadastro.gpkg", "table": "parsel", "filter": "alan > 1000",
                                   "fields": ["ada", "parsel_no", "alan"], "limit": 50 }),
                    output: None,
                },
                CommandExample {
                    title: "Bağlı katmanın kaynağındaki bütün parsellerin alan istatistiği".into(),
                    input: json!({ "layer": "parsel", "scope": "source", "limit": 0, "count": true, "statistics": ["alan"] }),
                    output: None,
                },
            ],
        },
        // A PDF page into the drawing (docs/adr/0352), held together by fixtures/commands/v1/file.import.pdf.json.
        CommandDescriptor {
            id: crate::FILE_IMPORT_PDF.into(),
            version: crate::FILE_IMPORT_PDF_VERSION,
            title: "PDF içe aktar".into(),
            summary: "path'teki PDF'in bir sayfasının (page, varsayılan 1) çizgilerini, dolgularını ve yazılarını açık çizimin kendi nesneleri olarak alır: çizgi çizgi ya da çoklu çizgi (renk, kâğıttaki kalınlık, kesik), dolgu alan (beyaz dolgu kâğıt sayılır, alınmaz), yazı yazı (yer, yükseklik, dönüş, renk). strokes, fills, texts hangilerinin alınacağını seçer. \
                      Kâğıt ölçüsü scale ile çizime büyür (1:scale; kâğıdın bir metresi çizimin scale metresi), sayfanın sol alt köşesi at'e gelir. region (sayfanın sol alt köşesinden milimetre) yalnız bütün noktaları içinde kalan parçaları ve başı içinde kalan yazıları alır. \
                      Eğriler curves polyline ile tolerance (kâğıtta mm, varsayılan 0,01) içinde çoklu çizgi, nurbs ile kesin üçüncü dereceden NURBS eğrisi olur (aradaki düz kenarlar çizgi). Sayfanın isteğe bağlı içerik grupları katmandır (hiçbirinde olmayan “PDF”); skipLayers onları dışarıda bırakır, layers var olan katmana yönlendirir, yoksa aynı adlı katmana, o da yoksa dosyanın adındaki gruba yeni katman. \
                      PDF yazdırılanı taşır, çizimi değil: ölçü, kılavuz ve tablo çizgileri ve yazılarıyla gelir; resimler, gölgelemeler ve okunamayan harfler alınmaz (import_losses). Bütün alınanlar tek geri alma adımıdır (“PDF: ad.pdf”); plan dosyayı okur, kutuyu, katmanları ve kayıpları verir, hiçbir şey yazmaz."
                .into(),
            aliases: vec!["pdfin".into(), "pdfimport".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportPdf>(),
            output: schema::<crate::PdfImported>(),
            plan: Some(schema::<crate::PdfImported>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Mimarın 1:50 basılmış planı".into(),
                    input: json!({ "path": "/proje/kat-plani.pdf", "scale": 50, "curves": "nurbs" }),
                    output: None,
                },
                CommandExample {
                    title: "İkinci sayfanın yalnız sol yarısı, yazılar olmadan".into(),
                    input: json!({ "path": "/proje/paftalar.pdf", "page": 2, "texts": false,
                                   "region": { "a": { "x": 0, "y": 0 }, "b": { "x": 210, "y": 297 } } }),
                    output: None,
                },
            ],
        },
        // A GeoPackage into the drawing (docs/adr/0502), held together by fixtures/commands/v1/file.import.gpkg.json.
        CommandDescriptor {
            id: crate::FILE_IMPORT_GPKG.into(),
            version: crate::FILE_IMPORT_GPKG_VERSION,
            title: "GeoPackage içe aktar".into(),
            summary: "path'teki GeoPackage'ın (.gpkg) obje ve öznitelik tablolarını açık çizime alır: her tablo çizimde aynı adlı katmana, yoksa tablonun sütunlarını alan olarak taşıyan yeni bir katmana gider; yeni katmanlar dosyanın adındaki gruba girer. tables yalnız adı verilen tabloları alır, layers bir tabloyu var olan bir katmana yönlendirir. \
                      Nokta ve çok nokta nokta nesnesi, çizgi ve eğri (dairesel yay, birleşik eğri) çoklu çizgi (yaylar bulge olarak kesin), alan ve eğri alan delikleriyle alan, çok parçalılar parçalarıyla gelir; Z köşelerin kotu, M çoklu çizginin ölçüsüdür. Geometrisi olmayan ya da boş satır geometrisiz kayıttır. \
                      NULL değer özniteliğin yokluğu, boş metin boş metindir; 2⁵³'ten büyük tam sayılar büyük tam sayı (bigint), dilimli ya da kesirli tarih ve saat zaman damgası (instant), BLOB ikili (binary) alanına kayıpsız gelir; KentOS'un yazdığı dosyanın belgesi UUID ve JSON sütunlarını da söyler (docs/adr/0629). Her satırın kalıcı kimliği dosyanın kentos_uids'i ya da tablo ve fid'den türetilmiş kimliktir: aynı dosya ikinci kez alınmaz (entity_exists). \
                      Dosyanın koordinat sistemi projeninkinden başkaysa reproject ile projenin sistemine çevrilir (projenin datum seçimleriyle), yoksa crs_mismatch; sistem bildirmeyen dosya olduğu gibi alınır (crs_undeclared). Telde geometri metredir. Bütün alınanlar tek geri alma adımıdır (“GeoPackage: ad.gpkg”); plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir (eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır, yinelenen anahtar durdurur ya da reddedilir), rejects reddedilen satırları nedenleriyle CSV'ye yazar (docs/adr/0628)."
                .into(),
            aliases: vec!["gpkgin".into(), "geopackage".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportGpkg>(),
            output: schema::<crate::GpkgImported>(),
            plan: Some(schema::<crate::GpkgImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Belediyenin parsel ve bina GeoPackage'ı".into(),
                input: json!({ "path": "/veri/kadastro.gpkg", "tables": ["parsel", "bina"] }),
                output: None,
            }],
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
                      Birim (unit): fileUnit verilmişse değerler o birimdedir; verilmemişse koordinat sistemli projede de dosyanın $INSUNITS birimi korunur. scaleReference ile ham bir uzunluk gerçek metre uzunluğuna kalibre edilebilir; fileUnit ile birlikte verilemez. \
                      Birim bildirmeyen ya da tanınmayan kod bildiren dosyanın sayıları ölçeklenmez: projenin biriminde (sistemli projede metre) sayılır ve unit_assumed söyler; birimi başka bloklar block_units_differ ile söylenir. \
                      Telde geometri metredir. Bütün alınanlar tek geri alma adımıdır (“DXF: ad.dxf”); plan dosyayı okur, hiçbir şey yazmaz ve aynısını söyler. Çıktıdaki sha256 okunan dosyanındır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. Çoklu çizginin genişlik/yay/kot kaybı varsayılan olarak polyline_geometry_loss ile reddedilir; yalnız allowPolylineLoss=true kayıp raporuyla yaklaşık dönüşümü kabul eder."
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
        CommandDescriptor {
            id: crate::FILE_IMPORT_DWG.into(),
            version: crate::FILE_IMPORT_DWG_VERSION,
            title: "DWG içe aktar".into(),
            summary: "path'teki DWG dosyasını açık çizime alır, masaüstünün DWG içe aktar penceresi gibi, sürümü sabit OpenCADCodec (MPL-2.0) ile (ADR 0586), ağ veya uzak dönüştürücü olmadan: her kaynak katmanın nesneleri çizimde aynı adlı katmana (büyük küçük harf ve Türkçe işaretler bir), yoksa dosyanın görünüşüyle yeni bir katmana gider; \
                      yeni katmanlar dosyanın adındaki gruba girer (group ile başka bir grup, boş metinle ağacın üstü). layers bir kaynak katmanı var olan bir katmana yönlendirir, skipLayers kaynak katmanları dışarıda bırakır. \
                      Bloklar tanım olarak (çizimde aynı adlı blok varsa yeni adla, renamed söyler), explodeBlocks ile patlatılarak gelir; dosyanın yazı ve ölçü stilleri projeye eklenir. \
                      Birim (unit): fileUnit verilmişse değerler o birimdedir; verilmemişse koordinat sistemli projede de dosyanın $INSUNITS birimi korunur. scaleReference ile ham bir uzunluk gerçek metre uzunluğuna kalibre edilebilir; fileUnit ile birlikte verilemez. \
                      Birim bildirmeyen ya da tanınmayan kod bildiren dosyanın sayıları ölçeklenmez: projenin biriminde (sistemli projede metre) sayılır ve unit_assumed söyler; birimi başka bloklar block_units_differ ile söylenir. \
                      Kaynak DWG sürümü ve bellek içi DXF aktarma biçimi report.source içinde ayrı söylenir. R14–2018 biçim imzaları desteklenir; korunmayan içerik report.skipped/notes içindedir. Telde geometri metredir. Bütün alınanlar tek geri alma adımıdır (“DWG: ad.dwg”); plan dosyayı okur, hiçbir şey yazmaz ve aynısını söyler. Çıktıdaki sha256 okunan dosyanındır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. Çoklu çizginin genişlik/yay/kot kaybı varsayılan olarak polyline_geometry_loss ile reddedilir; yalnız allowPolylineLoss=true kayıp raporuyla yaklaşık dönüşümü kabul eder."
                .into(),
            aliases: vec!["dwgin".into()],
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
                    input: json!({ "path": "/plan/kat-plani.dwg" }),
                    output: Some(json!({
                        "file": "kat-plani.dwg", "bytes": 48213, "sha256": "9f2c4c0e1b7a3d5e8f6a2b4c6d8e0f1a3b5c7d9e1f2a4b6c8d0e2f4a6b8c0d2e",
                        "unit": { "declared": 4, "used": 4, "name": "milimetre", "source": "declared", "factor": 0.001, "assumed": false },
                        "closedPolylines": "line",
                        "objects": 312, "targets": { "DUVAR": "import-duvar", "0": "cizim" },
                        "layers": [{ "id": "import-duvar", "name": "DUVAR" }], "group": "import-kat-plani-dxf",
                        "blocks": 2, "textStyles": 1, "dimensionStyles": 0, "report": {}, "revision": "12"
                    })),
                },
                CommandExample {
                    title: "Birim bildirmeyen dosya, santimetre olduğu bilinerek, ağacın üstüne".into(),
                    input: json!({ "path": "/plan/eski.dwg", "fileUnit": "cm", "group": "", "skipLayers": ["DEFPOINTS"] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SHEETS_CREATE.into(), version: 1,
            title: "Şablondan pafta oluştur".into(), summary: "Sistem şablonundan (source.type=system, id) ya da tam özel şablondan (custom, template) yeni pafta oluşturur. Sistem şablonları ve tam kitap cad.sheets.set(details=true) ile okunur. options: sayfa, ad, metre cinsinden center, ölçek paydası scale ve değişken values cevapları. Yeni id paftanın kimliğidir; öğe ve ana sayfa kimlikleri bu id’den türetilir. Resimler otomatik gömülür; index varsa paftayı o sıraya koyar. Tasarımcıyla aynı oluşturma işlemleri; hata halinde hiçbir şey yazılmaz. Model geometri değişmez. Bağımsız çağrı kâğıt geçmişinde, açık betik grubunda modelle birlikte tek undo/redo adımıdır; inverse ve inversePictures geri alır. expectedRevision korunur.".into(),
            aliases: vec![], effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop], headless: true,
            requires: vec![CommandRequirement::Document], permissions: vec![],
            undo: CommandUndo::None, cost: CommandCost::Instant,
            input: schema::<crate::SheetsCreate>(), output: schema::<crate::SheetsEdited>(),
            plan: Some(schema::<crate::SheetsEdited>()), capabilities: None,
            examples: vec![CommandExample {title: "Şablondan pafta oluştur".into(), input: json!({"id":"kat-plani","source":{"type":"system","id":"sys:cad-mimari"},"options":{"name":"A-101 Zemin kat","scale":50,"center":{"x":2.5,"y":0.0}}}), output: None}],
        },
        CommandDescriptor {
            id: crate::CAD_SHEETS_FIT.into(), version: 1,
            title: "Görünüm penceresini nesnelere sığdır".into(), summary: "Sabit haritayı item kimliğiyle nesneye sığdırır. uids verilirse kalıcı kimlikli model nesneleri, bounds verilirse metre cinsinden [minX,minY,maxX,maxY], ikisi de yoksa bütün model kullanılır. uids ve bounds birlikte verilmez; boş seçim reddedilir. Döndürme korunur, iç boşluk ve yazı bandından kalan alan kullanılır; marginPct her yanda kutu boyunun yüzdesidir (varsayılan %5). standardScales=false en küçük sığan tam paydayı, true standart paydayı seçer. Tek nokta mevcut ölçeği korur. Kaynak nesneler ve modelin 1:1 boyu değişmez; yalnız görünüm merkez ve ölçeği değişir. Kilitli öğe ve atlas haritası reddedilir. Pafta tasarımcısı ile aynı çekirdek; inverse geri alır, açık betik grubunda modelle tek adımdır.".into(),
            aliases: vec![], effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop], headless: true,
            requires: vec![CommandRequirement::Document], permissions: vec![],
            undo: CommandUndo::None, cost: CommandCost::Instant,
            input: schema::<crate::SheetsFit>(), output: schema::<crate::SheetsEdited>(),
            plan: Some(schema::<crate::SheetsEdited>()), capabilities: None,
            examples: vec![CommandExample {title: "Görünüm penceresini nesnelere sığdır".into(), input: json!({"item":"kat-plani-item-0","bounds":[0.0,0.0,5.0,3.0],"marginPct":5}), output: None}],
        },
        // Kat planı paftaları (docs/adr/0718), held together by fixtures/commands/v1/cad.sheets.levels.json.
        CommandDescriptor {
            id: crate::CAD_SHEETS_LEVELS.into(),
            version: crate::CAD_SHEETS_LEVELS_VERSION,
            title: "Kat planı paftaları".into(),
            summary: "Projenin her katı için bir pafta yapar (docs/adr/0718): aynı şablondan (source; yok: sys:cad-mimari), haritası o katı çizer (docs/adr/0716; below: alt kat soluk), bütün haritalar aynı zemini gösterir: seçilen katların nesnelerinin kutusu (yoksa bütün çizim), options.scale ölçeğinde ya da verilmezse sığdığı standart ölçekte; planlar kâğıtta üst üste oturur. levels: katların kimlikleri (yok: projenin bütün katları). Paftalar first'ten (yok: A-101) son rakamları artarak numaralanır, kullanılan numaralar atlanır; adı “A-101 Zemin kat planı”, çizim bilgisi numara, başlık ve Mimari › Kat planları. Bir haritası zaten gösterdiği kat atlanır (Revit'in Do not duplicate existing views'i); again true yeniden yapar. Plan yapılacak paftaları ve haritalarını verir; hepsi tek değişiklik, inverse geri alır. Projede olmayan kat level_not_found, options.name invalid_options, katların hepsinin paftası varsa levels_have_sheets, haritasız şablon not_fixed_map, rakamla bitmeyen first bad_drawing_info."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SheetsLevels>(),
            output: schema::<crate::SheetsEdited>(),
            plan: Some(schema::<crate::SheetsEdited>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Her kat için A1 mimari pafta, 1:100".into(),
                input: json!({ "options": { "scale": 100 } }),
                output: None,
            }],
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
                      Tek başına çağrı kâğıdın geçmişindedir; açık Python/belge grubu ve changeset kitabı ve resimleri modelle tek undo/redo adımına alır. Sonuçtaki inverse işlemleri geri alır. pictures, addAssets bilgisindeki SHA-256 ve byte boyuyla eşleşen base64 resim baytlarıdır; applyTemplate içindeki resim baytları otomatik gömülür. inversePictures gelirse ters işlemlerle birlikte pictures olarak ve replacePictures=true ile verilir. Sonuç kitabın paftalarını boyları ve haritalarının ölçekleriyle sayar. \
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
    // Recipe metadata and its source/member objects travel in the same transaction (ADR0695).
    let mut live_changes = commands[0].clone();
    live_changes.version = crate::PROJECT_CHANGES_DIMENSION_SETS_VERSION;
    live_changes.title = "Proje değişiklikleri ve ilişkili ölçüler".into();
    live_changes.summary.push_str(" v2 dimensionSets tarifini, kaynak ve üye nesneleriyle aynı transaction içinde kabul eder; boş liste tarifleri kaldırır. v1 dimensionSets alanını reddeder. Sunucu desteği /v1/health projectChangesVersions ile öğrenilir.");
    commands.push(live_changes);
    commands.push(CommandDescriptor {
        id: crate::CAD_DIMENSION_SETS_CREATE.into(), version: crate::CAD_DIMENSION_SETS_VERSION,
        title: "Duvar ölçüleri oluştur".into(), summary: "Duvarın bir düz kenarının gerçek sol/sağ yüzünü ve açıklık uçlarını veya merkezlerini kalıcı bir tariften ölçer. Birden çok kenarlı (kapalı ya da çok köşeli) duvarda edge ölçülecek düz kenarı verir (köşe edge → edge+1); around ise her düz kenara birer dizi koyar, kapalı duvarda dış yüzden, hepsi tek adımda (çıktının sets'i; docs/adr/0700). Sıralar openings/overall; gapMm kağıt mm, geometri metre; belge stili kullanılır. Plan kimlik ayırmaz. Kaynak değişince üyelik güncellenir; köşe eklenip silinince kenar uçlarıyla izlenir; aynı kaynak çifti kimliğini ve elle yerleştirilen yazıyı korur. Yay kenar ve pahlanmış yüz işlemden önce reddedilir.".into(), aliases: vec![],
        effect: CommandEffect::Document, hosts: vec![CommandHost::Desktop], headless: true,
        requires: vec![CommandRequirement::Document], permissions: vec![],
        undo: CommandUndo::Step, cost: CommandCost::Instant,
        input: schema::<crate::DimensionSetsCreateInput>(), output: schema::<crate::DimensionSetsChanged>(),
        plan: Some(schema::<crate::DimensionSetsPlan>()), capabilities: None,
        examples: vec![CommandExample { title: "Duvar ölçüleri oluştur".into(), input: json!({"layerId":"olcu","wall":"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002","at":{"x":5.0,"y":-1.0},"face":"left","rows":["openings","overall"],"openings":"ends","gapMm":7.5}), output: None }],
    });
    commands.push(CommandDescriptor {
        id: crate::CAD_DIMENSION_SETS_EDIT.into(), version: crate::CAD_DIMENSION_SETS_VERSION,
        title: "Duvar ölçülerini düzenle".into(), summary: "Tarifi, grup veya sıra yerini değiştirir. move at metre, rowOffset metre ve seçilen gerçek yüz bazlıdır. set yüz/sıra/açıklık politikası/kağıt aralığı/belge stili/katman değiştirir. restoreHidden bastırılan parçaları, resetText yalnız yazı yerini, resetRows sıra yerlerini geri getirir. detach tarifi kaldırıp ilişkili ölçüleri bırakır; delete tarif ve üyeleri tek adımda siler.".into(), aliases: vec![],
        effect: CommandEffect::Document, hosts: vec![CommandHost::Desktop], headless: true,
        requires: vec![CommandRequirement::Document], permissions: vec![],
        undo: CommandUndo::Step, cost: CommandCost::Instant,
        input: schema::<crate::DimensionSetsEditInput>(), output: schema::<crate::DimensionSetsChanged>(),
        plan: Some(schema::<crate::DimensionSetsPlan>()), capabilities: None,
        examples: vec![CommandExample { title: "Duvar ölçülerini düzenle".into(), input: json!({"id":"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3009","operation":{"kind":"set","gapMm":10.0}}), output: None }],
    });
    commands.push(CommandDescriptor {
        id: crate::CAD_DIMENSION_SETS_QUERY.into(), version: crate::CAD_DIMENSION_SETS_VERSION,
        title: "Duvar ölçü gruplarını sorgula".into(), summary: "Kalıcı tarif, kaynak, üyelik, bastırılan parçalar, elle taşınan sıra yerleri ve kopuk durumunu okur. ids veya wall ile süzülür; boş girdi bütün grupları verir. Belgeye ve geçmişe yazmaz.".into(), aliases: vec![],
        effect: CommandEffect::Query, hosts: vec![CommandHost::Desktop], headless: true,
        requires: vec![CommandRequirement::Document], permissions: vec![],
        undo: CommandUndo::None, cost: CommandCost::Instant,
        input: schema::<crate::DimensionSetsQueryInput>(), output: schema::<crate::DimensionSetsQueried>(),
        plan: Some(schema::<crate::DimensionSetsQueried>()), capabilities: None,
        examples: vec![CommandExample { title: "Duvar ölçü gruplarını sorgula".into(), input: json!({}), output: None }],
    });
    commands.extend(protocol_commands());
    for d in &mut commands {
        if d.capabilities.is_none() {
            d.capabilities = crate::capabilities::capabilities_of(&d.id);
        }
    }
    CommandCatalog {
        format: CATALOG_FORMAT.into(),
        version: CATALOG_VERSION,
        defs: Default::default(),
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
        effects: crate::change_effects::CHANGE_EFFECTS
            .iter()
            .map(crate::change_effects::CatalogEffect::from)
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
                      Bağlı yazılar, taramalar ve ölçüler son adımdan sonra bir kez izler, çizimin bütünlüğü o zaman denetlenir. Nesneler, katmanlar, bloklar ve pafta kitabı label adlı tek geri alma adımıdır; ayarlar ve stil kitaplığı ayrı politikalarını korur, özet hangisinin değiştiğini söyler. \
                      İsteğe bağlı modelScope={sheet,item}, paftaya ait sabit ve yerleştirilmiş model görünümünü açıkça doğrular; model koordinatları yine metredir. Bu kapsamda pafta, görünüm, proje ayarı ve history komutları reddedilir. İç içe küme ve dosyaya yazan komut girmez. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
            id: crate::CAD_DONUT_CREATE.into(), version: crate::CAD_DONUT_CREATE_VERSION,
            title: "Halka oluştur".into(),
            summary: "Merkez (center), iç ve dış çap (inner, outer; metre) ile verilen katmana tek dolu halka taraması ekler. İç çap sıfırsa disk olur. Dış çap iç çaptan büyük olmalı. Sınırlar iki yarım daire yayıyla kesin olarak saklanır; Python, MCP ve Halka aracı aynı hesabı kullanır. Tek geri alma adımı.".into(),
            aliases: vec!["halka".into(), "donut".into()], effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop], headless: true, requires: vec![CommandRequirement::Document],
            permissions: vec![], undo: CommandUndo::Step, cost: CommandCost::Instant,
            input: schema::<crate::DonutCreate>(), output: schema::<crate::EntitiesCreated>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()), capabilities: None,
            examples: vec![CommandExample {title: "İç çapı 1 m, dış çapı 2 m halka".into(),
                input: json!({"layerId":"0", "center":{"x":0.0,"y":0.0}, "inner":1.0,"outer":2.0}), output:None}],
        },
        CommandDescriptor {
            id: crate::CAD_RECTANGLE_CREATE.into(),
            version: crate::CAD_RECTANGLE_CREATE_VERSION,
            title: "Dikdörtgen oluştur".into(),
            summary: "Verilen katmana bir dikdörtgen alan ekler; köşeleri komut hesaplar. Çerçeve iki köşe (corners), köşe ile boy ve en (size), köşe ile alan ve bir kenar (area) ya da kenar ve en (edge); rotation kenarların doğrultusu (radyan), corners keskin, yuvarlak (fillet, yarıçap) ya da pah (chamfer, uzaklık). \
                      Halka paylaşılan çekirdekle hesaplanır ve cad.polygon.create gibi yazılır: tek geri alma adımı, kilitli katmana yazılmaz, gizli katmana uyarıyla. \
                      Dikdörtgen ve Döndürülmüş dikdörtgen araçları, Python ve ajan aynı girdiye aynı dikdörtgeni alır."
                .into(),
            aliases: vec!["dikdortgen".into(), "rectang".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::RectangleCreate>(),
            output: schema::<crate::PolygonCreated>(),
            plan: Some(schema::<crate::PolygonPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "20 × 12.5 m'lik, köşeleri 1 m yuvarlak bir oda".into(),
                input: json!({ "layerId": "yapi",
                               "frame": { "kind": "size", "corner": { "x": 423500.0, "y": 4512300.0 },
                                          "length": 20.0, "width": 12.5, "toward": { "x": 423510.0, "y": 4512305.0 } },
                               "corners": { "kind": "fillet", "radius": 1.0 } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_POLYGON_REGULAR.into(),
            version: crate::CAD_POLYGON_REGULAR_VERSION,
            title: "Düzgün çokgen oluştur".into(),
            summary: "Verilen katmana kenar sayısıyla bir düzgün çokgen alan ekler; köşeleri komut hesaplar. Çerçeve merkez ve ilk köşe (inscribed), merkez ve ilk kenarın ortası (circumscribed) ya da bir kenar (edge, çokgen solunda). \
                      Halka paylaşılan çekirdekle hesaplanır ve cad.polygon.create gibi yazılır; Düzgün çokgen aracı, Python ve ajan aynı girdiye aynı çokgeni alır."
                .into(),
            aliases: vec!["cokgen".into(), "polygon".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::RegularPolygonCreate>(),
            output: schema::<crate::PolygonCreated>(),
            plan: Some(schema::<crate::PolygonPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Merkezi ve ilk köşesiyle altıgen".into(),
                input: json!({ "layerId": "yapi", "sides": 6,
                               "frame": { "kind": "inscribed", "center": { "x": 423500.0, "y": 4512300.0 },
                                          "corner": { "x": 423505.0, "y": 4512300.0 } } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_SECTOR_CREATE.into(),
            version: crate::CAD_SECTOR_CREATE_VERSION,
            title: "Daire dilimi oluştur".into(),
            summary: "Verilen katmana kapalı bir daire dilimi alan ekler: merkez (c), yarıçap (r) ve yayın doğudan saat yönünün \
                      tersine a0'dan a1'e süpürüldüğü açılar (radyan). Halkayı komut hesaplar: merkez, yayın başı ve sonu, ortadaki \
                      kenar kesin yay; alan cad.polygon.create gibi yazılır. Süpürmesi sıfır ya da tam tur olan dilim degenerate_shape, \
                      sıfırdan büyük olmayan yarıçap invalid_distance ile reddedilir. Daire dilimi aracı, Python ve ajan aynı girdiye \
                      aynı dilimi alır; tek geri alma adımı."
                .into(),
            aliases: vec!["dilim".into(), "sector".into(), "pie".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SectorCreate>(),
            output: schema::<crate::PolygonCreated>(),
            plan: Some(schema::<crate::PolygonPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Yarıçapı 10 m, doğudan kuzeye çeyrek dilim".into(),
                input: json!({ "layerId": "yapi", "c": { "x": 423500.0, "y": 4512300.0 }, "r": 10.0,
                               "a0": 0.0, "a1": std::f64::consts::FRAC_PI_2 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_REVCLOUD_CREATE.into(),
            version: crate::CAD_REVCLOUD_CREATE_VERSION,
            title: "Revizyon bulutu oluştur".into(),
            summary: "Verilen katmana kapalı bir revizyon bulutu alan ekler. Çerçeve doğu ve kuzey boyunca bir dikdörtgenin iki \
                      köşesi (rectangle), bir çokgenin köşeleri (polygon) ya da kapalı bir nesnenin kalıcı kimliğidir (object: alan, \
                      kapalı çoklu çizgi, daire, kapalı eğri, tam elips; nesne kalır). Halkanın her kenarı yaklaşık arc uzunluğunda \
                      eşit kirişlere bölünür, her kiriş dışa doğru 0,5 bükümlü bir yaydır; alan cad.polygon.create gibi yazılır. \
                      10 000'den çok yay too_many_points ile reddedilir. Revizyon bulutu aracı yay boyunu kâğıt milimetresinden \
                      çizimin ölçeğiyle çevirip aynı girdiyi verir; Python ve ajan aynı bulutu alır; tek geri alma adımı."
                .into(),
            aliases: vec!["bulut".into(), "revcloud".into(), "revizyon".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::RevcloudCreate>(),
            output: schema::<crate::PolygonCreated>(),
            plan: Some(schema::<crate::PolygonPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "20 × 10 m'lik dikdörtgenin çevresinde 2 m'lik yaylar".into(),
                input: json!({ "layerId": "yapi", "arc": 2.0,
                               "frame": { "kind": "rectangle", "a": { "x": 423500.0, "y": 4512300.0 },
                                          "b": { "x": 423520.0, "y": 4512310.0 } } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_ADJOIN.into(),
            version: crate::CAD_ENTITIES_ADJOIN_VERSION,
            title: "Bitişik alan".into(),
            summary: "Yalnız yeni sınır verilir: komşu bir alanın içinden ya da sınırından başka birine giden açık bir yol (pts, \
                      kenar başına bulges). Yolun neighbours katmanlarının (yoksa layerId'nin) görünen alanlarıyla kapattığı bölge \
                      yeni alandır; komşuların sınırı onun sınırı olur, parçaları ve delikleri olduğu gibi tek nesne olarak \
                      cad.polygon.create gibi yazılır, rules ile Çakışma ve Topoloji. within komşuları bir kutuya değenlerle \
                      sınırlar (aracın görünümü). Kapanmayan yol no_region ile reddedilir. Bitişik alan aracı, Python ve ajan \
                      aynı girdiye aynı alanı alır; tek geri alma adımı “Bitişik alan”."
                .into(),
            aliases: vec!["bitişik".into(), "bitisikalan".into(), "adjoin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesAdjoin>(),
            output: schema::<crate::PolygonCreated>(),
            plan: Some(schema::<crate::PolygonPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İki parselin arasını kapatan yol".into(),
                input: json!({ "layerId": "parsel",
                               "pts": [ { "x": 423510.0, "y": 4512305.0 }, { "x": 423510.0, "y": 4512330.0 },
                                        { "x": 423530.0, "y": 4512330.0 }, { "x": 423530.0, "y": 4512305.0 } ] }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_PARCELS_CREATE.into(),
            version: crate::CAD_PARCELS_CREATE_VERSION,
            title: "Parsel oluştur".into(),
            summary: "Etkin katman ne olursa olsun parsel katmanına (parsel) kapalı bir alan ekler ve numarasını komut verir: number \
                      verilirse o (başka parselde de varsa parcel_number_taken uyarısıyla), verilmezse parsellerin Parsel değerlerinin \
                      başındaki en büyük tam sayının bir fazlası. Öznitelikleri Ada, Parsel, Mahalle, Nitelik “Arsa”, Tapu alanı (m²), \
                      Pafta; attrs onların üstüne, etiketi numara; tapu alanı boş kalır (tapunundur). Çizimde parsel katmanı yoksa \
                      yeni projedeki gibi aynı adımda açılır. Alan cad.polygon.create gibi, rules ile Çakışma ve Topoloji. Parsel \
                      oluştur aracı, Python ve ajan aynı girdiye aynı parseli alır; tek geri alma adımı."
                .into(),
            aliases: vec!["parsel".into(), "parcel".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ParcelsCreate>(),
            output: schema::<crate::ParcelCreated>(),
            plan: Some(schema::<crate::ParcelPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Sıradaki numarayla 20 × 15 m'lik parsel".into(),
                input: json!({ "pts": [ { "x": 423500.0, "y": 4512300.0 }, { "x": 423520.0, "y": 4512300.0 },
                                        { "x": 423520.0, "y": 4512315.0 }, { "x": 423500.0, "y": 4512315.0 } ],
                               "attrs": { "Ada": "101" } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_SHAPES_EDIT.into(),
            version: crate::CAD_SHAPES_EDIT_VERSION,
            title: "Biçim işlemleri".into(),
            summary: "Nesnelerin biçimini kalıcı kimlikleriyle değiştirir; geometriyi komut ortak çekirdekle hesaplar: Biçim \
                      değiştir (reshape: alanı ya da çizgiyi çizilen hatla keser ya da büyütür; çizgi çoklu çizgi olur), \
                      Sürdür (continue: çizgiyi ya da çoklu çizgiyi ucundan ya da başından noktalarla ve bükümlerle uzatır, \
                      köşeler kotlarını korur), Paralel kaydır (edgeShift: at'a en yakın düz kenarı distance kadar ya da \
                      area m²'lik alana ulaşana dek kaydırır; alanda dışarı, çoklu çizgide yürüme yönünün sağı artı), \
                      Sadeleştir (simplify: toleranstaki köşeleri atar), Kot ver (elevation: Sabit, Artır, Sıfırla) ve \
                      Okunur yap (readable: baş aşağı okunan yazıları yarım tur döndürür) ve Parçala (split: çizgi, açık \
                      çoklu çizgi, yay ve daireyi kesişimlerinden, eşit parçalara ya da uzunluktan ayrı nesnelere böler; \
                      ilk parça nesnenin yerini ve kimliğini korur, yeniler created'da). Cevap ölçüleri (önce ve sonra \
                      alan ya da uzunluk, eklenen ya da atılan köşe, kaydırma, parça) verir. Tek geri alma adımı, aracın \
                      adıyla."
                .into(),
            aliases: vec!["bicim-islemleri".into(), "shapes".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ShapesEdit>(),
            output: schema::<crate::ShapesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parselin kenarını 1250 m²'ye ulaşana dek kaydır".into(),
                    input: json!({ "work": { "kind": "edgeShift", "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001",
                        "at": { "x": 423540.0, "y": 4512315.0 }, "area": 1250.0 } }),
                    output: None,
                },
                CommandExample {
                    title: "Seçilen nesnelerin köşelerine 125,40 m kot ver".into(),
                    input: json!({ "work": { "kind": "elevation", "ids": [ "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001" ],
                        "elevation": { "mode": "set", "z": 125.4 } } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SURVEY_POINTS.into(),
            version: crate::CAD_SURVEY_POINTS_VERSION,
            title: "Ölçme hesabı".into(),
            summary: "Ölçme pencerelerinin noktalarını hesaplayıp verilen katmana adlı noktalar olarak yazar; hesabı komut \
                      ortak çekirdekle yapar: Poligon hesabı (traverse: bağlı, kapalı ya da açık; kırılma açıları ve kenar \
                      uzunlukları, kapanma hataları dağıtılır), Kutupsal alım (polar: bakılan noktaya yöneltilmiş okumalar, \
                      başucu açısıyla kot), Önden kestirme (forward) ve Geriden kestirme (resection). Bilinen noktalar \
                      adıyla (named) ya da koordinatıyla (at); açılar unit'te, yoksa projenin açı biriminde. Noktalar Ad, \
                      Tür ve kotuyla yazılır. Tek geri alma adımı, pencerenin adıyla."
                .into(),
            aliases: vec!["olcme-hesabi".into(), "survey".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SurveyPoints>(),
            output: schema::<crate::SurveyPointsWritten>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "A ve B'den önden kestirme".into(),
                    input: json!({ "layerId": "nokta", "work": { "kind": "forward",
                        "a": { "kind": "named", "name": "A" }, "b": { "kind": "named", "name": "B" },
                        "alpha": 48.25, "beta": 51.75, "name": "P1" } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SURVEY_ADJUST.into(),
            version: crate::CAD_SURVEY_ADJUST_VERSION,
            title: "Ağ dengelemesi".into(),
            summary: "Yatay ağı (horizontal: bilinen noktalar, doğrultu ve kenar ölçüleri) ya da kot ağını (level: bilinen \
                      kotlar, kot farkları; geometrik ya da trigonometrik) ortak çekirdekle, projenin önsel doğrulukları \
                      ve ızgarasıyla dengeler ve yeni noktaları yazar; bilinenler yazılmaz. Yalnız adı verilen bilinen \
                      nokta ya da kot çizimin aynı adlı noktasınındır. Yatay ağ çizimin aynı adlı noktalarını, \
                      üzerlerindeki çizgi, çoklu çizgi ve alan köşeleriyle dengelenmiş yerlerine taşır, olmayanları \
                      layerId'ye Ağ noktası olarak ekler; kot ağı aynı adlı noktalara kotlarını, köşelerine ve Z (m)'lerine \
                      de yazar, çizimde olmayanları söyler. Çıktı yeni noktaların yerleri ya da kotları ve standart \
                      sapmaları, serbestlik, m0 ve model testi. Tek geri alma adımı, pencerenin adıyla."
                .into(),
            aliases: vec!["ag-dengelemesi".into(), "adjust".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::SurveyAdjust>(),
            output: schema::<crate::SurveyAdjusted>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "K1 ve K2'den kenarlarla bir nokta".into(),
                    input: json!({ "layerId": "nokta", "work": { "kind": "horizontal",
                        "known": [{ "name": "K1" }, { "name": "K2" }],
                        "rows": [{ "station": "K1", "target": "Y1", "distance": 351.2847 },
                                 { "station": "K2", "target": "Y1", "distance": 492.0591 },
                                 { "station": "K1", "target": "K2", "distance": 400.4979 }] } }),
                    output: None,
                },
                CommandExample {
                    title: "İki reperden nivelman".into(),
                    input: json!({ "work": { "kind": "level", "known": [{ "name": "R1", "h": 102.4567 }, { "name": "R2" }],
                        "rows": [{ "from": "R1", "to": "N1", "dh": 2.54398, "length": 820.0 },
                                 { "from": "N1", "to": "R2", "dh": -6.87731, "length": 1550.0 }] } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_IMAGES_CLIP.into(),
            version: crate::CAD_IMAGES_CLIP_VERSION,
            title: "Resmi kırp".into(),
            summary: "Resmin gösterilen kısmını sınırlar; kırpmayı komut ortak çekirdekle hesaplar: çizimde verilen sınır \
                      (rectangle: a ve b köşeleriyle eksenlere paralel dikdörtgen; polygon: en az üç köşe) resmin kendi \
                      kesirlerine çevrilip resme kesilir; remove kırpmayı kaldırır. Sınır resimle alan paylaşmıyorsa \
                      clip_outside. Resmin zaten gösterdiği kırpma hiçbir şey yazmaz. Tek geri alma adımı, Resmi kırp."
                .into(),
            aliases: vec!["resmi-kirp".into(), "imageclip".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ImagesClip>(),
            output: schema::<crate::ImagesClipped>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Resmin sol alt çeyreği".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001",
                        "clip": { "kind": "rectangle", "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 2.0, "y": 1.5 } } }),
                    output: None,
                },
                CommandExample {
                    title: "Kırpmayı kaldır".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "clip": { "kind": "remove" } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_TIME_VERSION.into(),
            version: crate::CAD_TIME_VERSION_VERSION,
            title: "Yeni sürüm oluştur".into(),
            summary: "Aralıklı zamansal katmanlardaki nesneleri bir tarihte sona erdirir; version (varsayılan) yanlarına \
                      geometrisi, rengi, kalınlığı, öznitelikleri, etiketi ve sembolüyle, at tarihinde başlayıp nesnenin \
                      bittiği yerde biten yeni sürümlerini de yazar, end yalnız sona erdirir. Her nesnenin katmanı \
                      başlangıç ve bitiş alanlı (not_ranged), kilitsiz (layer_locked), tarihleri okunur (time_unreadable), \
                      yeni sürümde kopyalanabilir (not_versionable: kayıt ve mimari nesneler) ve zamanı tarihi içermeli \
                      (not_within: başlangıcı önce, bitişi sonra); tipli alan tarihi almalı (invalid_attribute). Tarih \
                      alanı tarih türündeki katmanda gün yazılır. Tek geri alma adımı, aracın adıyla (Yeni sürüm oluştur \
                      ya da Sona erdir)."
                .into(),
            aliases: vec!["yeni-surum".into(), "sona-erdir".into(), "timeversion".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::TimeVersion>(),
            output: schema::<crate::TimeVersioned>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parselin 2024'ten sonraki sürümü".into(),
                    input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "at": "2024-03-05" }),
                    output: None,
                },
                CommandExample {
                    title: "Binayı sona erdir".into(),
                    input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "at": "05.03.2024",
                        "kind": "end" }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ROADS_EDIT.into(),
            version: crate::CAD_ROADS_EDIT_VERSION,
            title: "Plan yolu çizimi".into(),
            summary: "Plan yollarını çizer ve düzenler; geometriyi komut ortak çekirdekle hesaplar: Plan yolu (plan: \
                      eksenin iki yanında width genişliğinde yol alanı Tür ve Genişlik öznitelikleriyle; Yol'da kerb \
                      kaldırımlarıyla taşıt yolu, median refüjü yarım daire uçlu; axisLine ekseni de yazar; layerId'ye), \
                      Kavşak temizle (junctions: ids'in alanları Refüj ve Yol ekseni dışında türlerine göre birleşir, iç \
                      köşeleri taşıt yolunda kerbRadius, öbürlerinde blockRadius ile yuvarlanır) ve Refüj kapat (median: \
                      first ve second çizgileri refüj alanı olur, uçları yarım daire ya da round false ile düz; first alan \
                      olur, second silinir). Tek geri alma adımı, aracın adıyla."
                .into(),
            aliases: vec!["plan-yolu".into(), "roads".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::RoadsEdit>(),
            output: schema::<crate::RoadsEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "15 m'lik, 3 m kaldırımlı yol".into(),
                    input: json!({ "layerId": "yol", "work": { "kind": "plan", "road": "road",
                        "axis": [ { "x": 423500.0, "y": 4512300.0 }, { "x": 423560.0, "y": 4512310.0 } ],
                        "width": 15.0, "kerb": 3.0 } }),
                    output: None,
                },
                CommandExample {
                    title: "İki yolun kavşağı, ada köşeleri 6 m".into(),
                    input: json!({ "work": { "kind": "junctions",
                        "ids": [ "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002" ],
                        "blockRadius": 6.0, "kerbRadius": 3.0 } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_OBJECTS_DERIVE.into(),
            version: crate::CAD_OBJECTS_DERIVE_VERSION,
            title: "Nesneden türet".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerden yeni nesneler türetip verilen katmana ekler; hesabı komut \
                      ortak çekirdekle yapar: Köşelere nokta (vertexPoints: çizgi, çoklu çizgi ve alanların köşelerine \
                      adlı, kodlu, kotlu nokta; paylaşılan ve noktası olan köşe atlanır), Orta hat (centerline: iki \
                      kenarın ekseni, kenar bir nesne ya da uç uca birleşen zincir; eşleşen kenarlarda kesin, öbürlerinde \
                      step aralıkla) ve Toplu alan (polygonize: çizgilerin kapattığı her bölge bir alan, içindeki tek \
                      etiket attribute özniteliği). Cevap yeni nesneleri, atlananları ve sıradaki adı verir. Tek geri \
                      alma adımı, aracın adıyla."
                .into(),
            aliases: vec!["turet".into(), "derive".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ObjectsDerive>(),
            output: schema::<crate::ObjectsDerived>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parselin köşelerine N1'den başlayan noktalar".into(),
                    input: json!({ "layerId": "nokta", "work": { "kind": "vertexPoints",
                        "ids": [ "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001" ], "name": "N1", "code": "KS" } }),
                    output: None,
                },
                CommandExample {
                    title: "İki yol kenarının ekseni, 1 m adımla".into(),
                    input: json!({ "layerId": "eksen", "work": { "kind": "centerline",
                        "first": [ "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001" ],
                        "second": [ "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002" ], "step": 1.0 } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ANNOTATIONS_WRITE.into(),
            version: crate::CAD_ANNOTATIONS_WRITE_VERSION,
            title: "Açıklama yaz".into(),
            summary: "Koordinat ve km yazılarını verilen katmana ekler; yerleşimi komut ortak çekirdekle yapar: Koordinat \
                      yaz ve Köşelere koordinat yaz (coordinates: at yerlerine ya da ids nesnelerinin köşelerine; şablon \
                      {Y}, {X}, {Z}, {ad}, basamak, kol, yön; yerin kotu ve adı oradaki görünen nesnelerden) ve Km yaz \
                      (stations: güzergâhın aralıkla km'si, başlangıç, ters, ölçü, uçlar; yazı, işaret, enkesit, nokta) ve \
                      Eğri boyunca yazı (along: yazı eğrinin tıklanan yere en yakın yolunda, harflerinin kapladığı \
                      parçada, hizanın payıyla, okunur yönde), Hızlı ölçü (dimensions: ids'in düz kenarlarına hizalı, \
                      yaylarına yay uzunluğu ölçüsü, at'ın uzaklığında ya da distance'ta; style, mask) ve Etiketleri \
                      yazıya çevir (labels: ids'in ya da görünen \
                      bütün nesnelerin etiketleri etiket motorunun 1:scale'de yerleştirdiği yerde yazı, çağrı çizgileri \
                      çizgi; every, mask, linked). Yazının kâğıt yüksekliği mm, çizimde ölçekle çarpılır. Çizimde \
                      olmayan standart katman (yazi …) aynı adımda açılır. Tek geri alma adımı, aracın adıyla."
                .into(),
            aliases: vec!["aciklama-yaz".into(), "annotate".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::AnnotationsWrite>(),
            output: schema::<crate::AnnotationsWritten>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir noktaya kollu koordinat yazısı".into(),
                    input: json!({ "layerId": "yazi", "height": 2.5, "work": { "kind": "coordinates",
                        "at": [ { "x": 423500.0, "y": 4512300.0 } ], "leader": true, "lean": "northEast" } }),
                    output: None,
                },
                CommandExample {
                    title: "Yol ekseninde 20 m'de bir km".into(),
                    input: json!({ "layerId": "km", "work": { "kind": "stations",
                        "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "interval": 20.0, "ends": true,
                        "text": "left", "tick": true } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_AREAS_EDIT.into(),
            version: crate::CAD_AREAS_EDIT_VERSION,
            title: "Alan işlemleri".into(),
            summary: "Alan işlemlerini kalıcı kimlikleriyle verilen nesnelerde yapar; geometriyi komut ortak çekirdeğin kesin \
                      alan cebiriyle hesaplar (yaylar yay kalır, köşeler koordinatlarını korur): Alan birleştir (union; \
                      single ile değmeyenler tek çok parçalı alan), Alan kesiştir (intersect; erase ile kaynaklar silinir), \
                      Alan çıkar (subtract: from'dan cut çıkar; içteki alan delik bırakır), Alan böl (split: line noktaları \
                      ya da cutter nesnesiyle; ilk parça alanın kendisi), Alana çevir (toArea: kapalı nesneler yerinde, \
                      çizgilerin kapattığı bölgeler yeni alan), Çizgiye çevir (toPolyline), Parçaları birleştir ve \
                      Parçalara ayır (partsJoin, partsSplit: alan, çizgi ya da nokta), Delik ekle (holeAdd: halka), \
                      Deliği sil ve Deliği doldur (holeRemove, holeFill: delikteki nokta). Cevap yazılanları, sonucun \
                      nesnelerini (result) ve ölçülerini (alan m², parça sayısı) verir. Tek geri alma adımı, aracın adıyla."
                .into(),
            aliases: vec!["alan-islemleri".into(), "areas".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::AreasEdit>(),
            output: schema::<crate::AreasEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki parseli birleştir (tevhit)".into(),
                    input: json!({ "work": { "kind": "union", "ids": [
                        "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002" ] } }),
                    output: None,
                },
                CommandExample {
                    title: "Parseli iki noktadan geçen çizgiyle böl (ifraz)".into(),
                    input: json!({ "work": { "kind": "split", "ids": [ "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001" ],
                        "line": [ { "x": 423510.0, "y": 4512290.0 }, { "x": 423510.0, "y": 4512340.0 } ] } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_CONSTRUCTIONS_CREATE.into(),
            version: crate::CAD_CONSTRUCTIONS_CREATE_VERSION,
            title: "Yapım yardımcıları".into(),
            summary: "Yapım araçlarının nesnelerini verilen katmana ekler; geometriyi komut hesaplar: Paralel çizgi (eksenin iki \
                      yanı, isteğe bağlı eksen ya da koridor alanı), Dik in (noktalardan çizgiye ya da daireye dikler), Dik çık \
                      (dik ayak ve dik boyla), İki daireye teğet (tıklamalara en yakın ortak teğet), Dördüncü köşe (nokta ya da \
                      paralelkenar alanı), Menzil halkaları (aralık, sayı, ışın), Ara nokta (eşit parçalar, uzaklıklar ya da \
                      oranlar), Kesişim noktası (iki uzaklık, iki doğrultu ya da iki doğru), yardımcı çizginin Açıortay'ı ve \
                      Paralel'i. Uzaklıklar çizim biriminde, doğrultular kuzeyden saat yönünde radyan. Teğeti ya da kesişimi \
                      olmayan yapım no_intersection, çakışan ya da aynı doğrudaki noktalar degenerate_shape ile reddedilir. \
                      Araç, Python ve ajan aynı girdiye aynı nesneleri alır; tek geri alma adımı, aracın adıyla."
                .into(),
            aliases: vec!["yapim".into(), "construction".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ConstructionsCreate>(),
            output: schema::<crate::EntitiesCreated>(),
            plan: Some(schema::<crate::EntitiesCreatePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Eksenin iki yanında 3 m'lik paralel çizgiler".into(),
                    input: json!({ "layerId": "yol", "construction": { "kind": "parallel",
                        "axis": [ { "x": 423500.0, "y": 4512300.0 }, { "x": 423560.0, "y": 4512330.0 } ],
                        "left": 3.0, "right": 3.0 } }),
                    output: None,
                },
                CommandExample {
                    title: "İki noktadan uzaklıklarla kesişim noktası".into(),
                    input: json!({ "layerId": "nokta", "construction": { "kind": "intersection", "meeting": {
                        "method": "distances", "a": { "x": 423500.0, "y": 4512300.0 }, "da": 25.0,
                        "b": { "x": 423530.0, "y": 4512300.0 }, "db": 20.0 } } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_VERTEX.into(),
            version: crate::CAD_ENTITIES_VERTEX_VERSION,
            title: "Köşe ve kenar".into(),
            summary: "Bir yolun köşesini ya da kenarını düzenler: Köşeyi sil (köşe yeri), Ortasına köşe ekle, Düz kenar yap ve Yaya dönüştür (kenarın ortasının yeri); nesne kalıcı kimliğiyle, yer cad.snap.find ya da cad.entities.pick'in verdiği gibi. \
                      Yay kenara eklenen köşe yayı çemberinde ikiye böler; Yaya dönüştür kenarı kirişin dörtte biri sehimle halkanın dışına ya da açık yolun sağına büker, through ile verilen noktadan, radius ile verilen yarıçapla (artı: kenarın sağına) geçen yay yapar, yay kenarı da yeniden biçimler; Kenar genişliği (width) çoklu çizginin kenarına başlangıç ve bitiş genişliği verir; çizgiye köşe eklenince çoklu çizgi olur. Alanın delikleri ve öbür parçaları kalır; topology ile ortak köşe ve kenarları olan komşular aynı adımda izler. Tek geri alma adımı."
                .into(),
            aliases: vec!["kose".into(), "vertex".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesVertex>(),
            output: schema::<crate::EntitiesVertexed>(),
            plan: Some(schema::<crate::EntitiesVertexPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir alanın üçüncü köşesini sil".into(),
                input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "action": "remove",
                               "place": { "kind": "vertex", "part": 0, "ring": 0, "index": 2 } }),
                output: None,
            }, CommandExample {
                title: "Çoklu çizginin ikinci kenarını 25 m yarıçaplı yaya dönüştür".into(),
                input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "action": "arc", "radius": 25.0,
                               "place": { "kind": "mid", "part": 0, "ring": 0, "index": 1 } }),
                output: None,
            }, CommandExample {
                title: "Çoklu çizginin ilk kenarını 0,5 m'den 0'a incelt".into(),
                input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "action": "width", "widths": [0.5, 0.0],
                               "place": { "kind": "mid", "part": 0, "ring": 0, "index": 0 } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_PICK.into(),
            version: crate::CAD_ENTITIES_PICK_VERSION,
            title: "Tıklanan nesneler".into(),
            summary: "Bir tıklamanın seçebileceği nesneleri en belirliden verir (Sıradakini seç'in adayları: noktalar ve kenarlar uzaklıklarıyla, sonra içinde kalınan alanlar boylarıyla); her birinin kalıcı kimliği ve tıklamanın düştüğü alt nesne: toleranstaki en yakın noktası (uç, köşe, nokta, ekleme noktası), yoksa kenarının ya da eğrisinin en yakın noktası (along); blokta parçanın yolu. \
                      Cevabın uid, place ve path'i cad.dimensions.create'in place noktasına ve öbür komutlara olduğu gibi verilir. Yalnız gösterilen nesneler; kinds seçim süzgecidir. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["tikla".into(), "pick".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesPick>(),
            output: schema::<crate::EntitiesPicked>(),
            plan: Some(schema::<crate::EntitiesPicked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir köşenin yanına, yarım metrelik toleransla".into(),
                input: json!({ "at": { "x": 487010.1, "y": 4420009.9 }, "tolerance": 0.5 }),
                output: None,
            }],
        },
        // Points between coordinate systems with heights and the operation's record (docs/adr/0519), held together by
        // fixtures/commands/v1/cad.crs.transform.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_CRS_TRANSFORM.into(),
            version: crate::CAD_CRS_TRANSFORM_VERSION,
            title: "Koordinat dönüştür".into(),
            summary: "Noktaları from tarafının sisteminden to tarafınınkine taşır; taraf srid (kayıttaki EPSG kodu) verilmezse projenin sistemidir ve projenin yükseklik tanımıyla (heights) ve epokuyla gelir. Noktalar tarafın kendi biriminde: ızgarada metre doğu ve kuzey, coğrafide derece boylam ve enlem; z kot.  \
                      Kayıt (operation): işlemin kimliği (datum adımlarının EPSG kodları ya da projenin seçiminin adı), adımları, aynı işi yapan PROJ pipeline'ı, motor, ızgaralar (dosya adı ve SHA-256), kullanım alanı, belirtilen doğruluk, kotların nasıl gittiği, epoklar.  \
                      Kotlar: elipsoit yüksekliği datum kaymasıyla birlikte (yer merkezli koordinatlarda) taşınır; aynı düşey sistemdeki ortometrik kot kalır, aynı datumun başka biriminde çevrilir (MSL m ↔ ft). Ortometrik kot, yanın geoit modeliyle (heights.geoid, SHA-256'sıyla; bu cihazın ızgara kitaplığında GTX ya da GeoTIFF) elipsoit yüksekliğine ve geri çevrilir (H = h − N, PROJ'un vgridshift'i gibi çift doğrusal); geoit modeli söylenmemişse geoid_required, iki düşey datumdan birinin geoidi yoksa vertical_unsupported, ızgara bu cihazda yoksa geoid_missing.  \
                      Epoklar: iki ayrı epok velocity (hız ızgarası: doğu, kuzey, yukarı mm/yıl, GeoTIFF; SHA-256'sıyla) verilince noktalar kaynağın datumunda epok farkı kadar yer merkezli koordinatlarda taşınır (PROJ'un deformation'ı gibi); verilmezse epoch_unsupported, ızgara bu cihazda yoksa velocity_missing. KentOS yaklaşık yola (PROJ'un ballpark'ı gibi) geçmez.  \
                      Bir yanın yüksekliği söylenmiş öbürününki söylenmemişse kotlar varsayımla çevrilmez (heights_unknown); ikisi de söylenmemişse kotlar değişmeden kalır (heights_unchanged).  \
                      Rapor (report) sayıları ayrı tutar: kaynağın belirsizliği (uncertainty, verilen), dönüşümün doğruluğu, ikisinin karelerinin toplamının kökü (tahmin), topoloji toleransı ve gösterim basamakları. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["koordinat-donustur".into(), "cs2cs".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::CrsTransform>(),
            output: schema::<crate::CrsTransformed>(),
            plan: Some(schema::<crate::CrsTransformed>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "ED50 / TM30'dan TUREF / TM30'a, elipsoit yüksekliğiyle".into(),
                    input: json!({ "points": [{ "x": 412345.0, "y": 4512345.0, "z": 812.4 }], "from": { "srid": 2320, "heights": { "kind": "ellipsoidal" } }, "to": { "srid": 5254, "heights": { "kind": "ellipsoidal" } } }),
                    output: None,
                },
                CommandExample {
                    title: "Elipsoit yüksekliğinden geoit modeliyle ortometrik kota".into(),
                    input: json!({ "points": [{ "x": 487200.0, "y": 4420600.0, "z": 1036.12 }], "from": { "heights": { "kind": "ellipsoidal" } }, "to": { "heights": { "kind": "orthometric", "verticalSrid": 5775, "geoid": { "name": "TG-20", "sha256": "0000000000000000000000000000000000000000000000000000000000000000" } } } }),
                    output: None,
                },
                CommandExample {
                    title: "Antalya kotu EGM2008'e: reddedilir".into(),
                    input: json!({ "points": [{ "x": 412345.0, "y": 4512345.0, "z": 812.4 }], "from": { "srid": 5254, "heights": { "kind": "orthometric", "verticalSrid": 5775 } }, "to": { "srid": 5254, "heights": { "kind": "orthometric", "verticalSrid": 3855 } } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_RELATE.into(),
            version: crate::CAD_ENTITIES_RELATE_VERSION,
            title: "Standart ilişkiler".into(),
            summary: "a'nın her nesnesinin b'nin her nesnesiyle DE-9IM matrisini standart profille verir: kayan noktaya kadar kesin, Konuma göre seç'in 1 mm'lik toleransı yok (OGC SFA, PostGIS ST_Relate gibi). \
                      predicate (intersects, disjoint, contains, within, covers, coveredBy, touches, crosses, overlaps, equals) ya da pattern (dokuz harf: T, F, *, 0, 1, 2) verilirse yalnız tutan çiftler; ikisi de yoksa bütün çiftler matrisleriyle. \
                      Sınırdaki nokta covers'ta tutar, contains'te tutmaz. Nesne alanları ve delikleriyle (yaylar kesin), açık yollarıyla ya da noktalarıyla okunur; Z okunmaz; yardımcı çizgi ve ışın okunmaz (unread). Kimlikler verildiği için gizli ve kilitli nesneler de okunur. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["iliski".into(), "relate".into(), "de9im".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesRelate>(),
            output: schema::<crate::EntitiesRelated>(),
            plan: Some(schema::<crate::EntitiesRelated>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parselin sınırındaki nokta: kapsar ama içermez".into(),
                input: json!({ "a": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"], "b": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002"], "predicate": "covers" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_SELECT.into(),
            version: crate::CAD_ENTITIES_SELECT_VERSION,
            title: "Nesne seç".into(),
            summary: "Bir bölgenin, listenin ya da benzerliğin nesnelerini kalıcı kimlikleriyle verir: pencere, kesişen kutu, çit, çokgen (içindekiler, kesişenler, dışındakiler), daire, noktayı içeren alanlar (küçükten büyüğe), bir tıklamanın adayları, hepsi, verilenler (önceki seçim), ters çevir ve benzerleri (tür, katman, renk, sembol). \
                      Seçim süzgecinin türleri ve katman listesiyle süzülür, dışarıda kalanlar sayılır. Yalnız gösterilen nesneler sayılır (gizli katman, katman süzgeci ve zaman penceresi dışarıda); kilitli katmanınkiler seçilir. Araçlar aynı sorguyla seçer. modelScope verilirse pafta viewport katmanları, açıklama temsili ve kırpması uygulanır; uid modelin özgün kimliğidir. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["sec".into(), "select".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesSelect>(),
            output: schema::<crate::EntitiesSelected>(),
            plan: Some(schema::<crate::EntitiesSelected>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir pencerenin içindeki alanlar".into(),
                    input: json!({ "region": { "kind": "window", "a": { "x": 487000.0, "y": 4420000.0 },
                                               "b": { "x": 487050.0, "y": 4420030.0 } },
                                   "kinds": ["polygon"] }),
                    output: None,
                },
                CommandExample {
                    title: "Bir nesnenin katmanındaki ve türündeki bütün nesneler".into(),
                    input: json!({ "region": { "kind": "similar", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"],
                                               "criteria": { "kind": true, "layer": true, "color": false,
                                                             "symbol": false } } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SNAP_FIND.into(),
            version: crate::CAD_SNAP_FIND_VERSION,
            title: "Kenet bul".into(),
            summary: "Bir tıklamanın kenetleneceği noktayı verir: türü, noktası, nesnenin kalıcı kimliği, nesnedeki yeri (uç, köşe, kenar ortası, merkez, düğüm, kenar üstü) ve blok içindeyse parçanın yolu; all ile bütün adaylar en iyiden (Tab'ın sırası). \
                      Cevabın uid, place ve path'i cad.dimensions.create'in place noktasına olduğu gibi verilir: ölçü o yere bağlanır. Araçlar aynı kenet kuralıyla alır; gizli katmanlar kenetlenmez. modelScope ile viewport katman/temsil/kırpması aday sırasından önce uygulanır; kaynak uid değişmez. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["kenetbul".into(), "snap".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SnapFind>(),
            output: schema::<crate::SnapFound>(),
            plan: Some(schema::<crate::SnapFound>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir köşenin yanında, yarım metrelik açıklıkla".into(),
                input: json!({ "at": { "x": 487012.2, "y": 4420010.1 }, "aperture": 0.5 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_POINTS_CONSTRUCT.into(),
            version: crate::CAD_POINTS_CONSTRUCT_VERSION,
            title: "Nokta yapımı".into(),
            summary: "Çizim yardımlarının verdiği noktaları neyle yapıldıklarıyla hesaplar: iki nokta ortası (mid), bir ucun uzantısında uzaklık (extension), bir noktadan nesnenin düz kenarına paralel uzaklık (parallel), birinin X'i ve öbürünün Y'si (track, UCS'te), kutupsal (polar, açı UCS'in x ekseninden saat yönünün tersine radyan) ve hat üzerinde uzaklık (along). \
                      Noktalar koordinat ya da nesnenin yeri (uç, köşe, kenar ortası, merkez, düğüm) olarak verilir; araçlar aynı kuralla alır. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["noktayap".into(), "nokta-yapimi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::PointsConstruct>(),
            output: schema::<crate::PointsConstructed>(),
            plan: Some(schema::<crate::PointsConstructed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İki noktanın ortası ve bir doğrunun ucundan 5 m".into(),
                input: json!({ "constructions": [
                    { "kind": "mid", "a": { "kind": "at", "at": { "x": 0.0, "y": 0.0 } }, "b": { "kind": "at", "at": { "x": 10.0, "y": 4.0 } } },
                    { "kind": "extension", "end": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "place": { "kind": "end" } }, "distance": 5.0 }
                ] }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_CURVES_CONSTRUCT.into(),
            version: crate::CAD_CURVES_CONSTRUCT_VERSION,
            title: "Eğri yapımı".into(),
            summary: "Daire, Yay ve Elips araçlarının yöntemleriyle daireyi, yayı ve elipsi neyle yapıldıklarıyla hesaplar: iki ya da üç noktadan daire, iki nesneye teğet ve yarıçaplı, üç nesneye teğet daire; üç noktadan, başlangıç–merkez–bitiş, –açı, –kiriş, başlangıç–bitiş–açı, –yön, –yarıçap yay ve bir nesnenin ucundan teğet Devam; eksen uçlarından ya da merkezden elips ve eliptik yay (diğer yarı eksen ya da döndürme). \
                      Açılar doğudan saat yönünün tersine radyan. Cevap cad.circle.create, cad.arc.create ve cad.entities.create'in elipsinin aldığıdır; araçlar aynı hesabı kullanır. Çizimi değiştirmez; plan aynı cevabı verir."
                .into(),
            aliases: vec!["egriyap".into(), "egri-yapimi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::CurvesConstruct>(),
            output: schema::<crate::CurvesConstructed>(),
            plan: Some(schema::<crate::CurvesConstructed>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Üç noktadan yay ve bir çizginin ucundan teğet devam".into(),
                    input: json!({ "constructions": [
                        { "kind": "arcThreePoints", "start": { "kind": "at", "at": { "x": 10.0, "y": 0.0 } }, "mid": { "kind": "at", "at": { "x": 0.0, "y": 10.0 } }, "end": { "kind": "at", "at": { "x": -10.0, "y": 0.0 } } },
                        { "kind": "arcContinue", "from": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "place": { "kind": "end" } }, "end": { "kind": "at", "at": { "x": 20.0, "y": 10.0 } } }
                    ] }),
                    output: None,
                },
                CommandExample {
                    title: "İki nesneye teğet, 5 m yarıçaplı daire".into(),
                    input: json!({ "constructions": [
                        { "kind": "circleTangentTangentRadius", "first": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "near": { "x": 0.0, "y": 3.0 } }, "second": { "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60", "near": { "x": 3.0, "y": 0.0 } }, "r": 5.0 }
                    ] }),
                    output: None,
                },
                CommandExample {
                    title: "Eksen uçlarından eliptik yay".into(),
                    input: json!({ "constructions": [
                        { "kind": "ellipseAxis", "a": { "kind": "at", "at": { "x": 0.0, "y": 0.0 } }, "b": { "kind": "at", "at": { "x": 20.0, "y": 0.0 } }, "other": 4.0, "startAngle": 0.0, "endAngle": std::f64::consts::FRAC_PI_2 }
                    ] }),
                    output: None,
                },
            ],
        },
        // The project with the files it uses, in one folder (docs/adr/0506).
        CommandDescriptor {
            id: crate::FILE_PACKAGE.into(),
            version: crate::FILE_PACKAGE_VERSION,
            title: "Paketle".into(),
            summary: "Projeyi kullandığı dosyalarla path klasörüne toplar, başka bilgisayarda açılsın diye: klasörde proje dosyası (name, yoksa çizimin adı) ve veri/ altında bağlı katmanların GeoPackage'larının, bağlı resim ve rasterlerin (dünya dosyalarıyla), nokta bulutu dosyalarının, bağlı tabloların ve eklerin kopyaları; her dosya bir kez, aynı adlılar numaralanarak. \
                      Projedeki bağlar kopyalara yazılır: bağlı dosyalar klasöre göre göreli (veri/…), bağlı katmanın kaynağı kopyası (göreli yolu veri/…, klasör nereye giderse açılışta bulunur). Göreli bağlar from klasöründen okunur. Projenin kendi veri profilleri ve eşlemeleri de (from klasöründeki profiller/<ad>.profil.json ve <ad>.eslem.json) paketteki profiller/ altına bayt bayt kopyalanır, proje dosyasının yanında durduğundan komutlar onları orada bulur; okunamayan dosya kopyalanmaz ve package_left söyler (docs/adr/0558). \
                      Adres ve yerinde olmayan dosya kopyalanmaz; bağları olduğu gibi kalır, package_left her birini söyler. Parola ve belirteç projede olmadığından pakete girmez. \
                      Bağlı kaynakların yolu seçilir (docs/adr/0596): geopackages (varsayılan copy) ve databases (varsayılan keep) her biri copy, keep ya da embed. copy kaynağı pakete kopyalar: GeoPackage bayt bayt, veritabanı tablosu bağın okuduğu satırlarla (alanla bağlıda yalnız alanınkiler) projenin sisteminde, kalıcı kimlikleriyle bir GeoPackage olarak (veri/<şema>-<tablo>.gpkg; tablo katmanın son eşleşmesinden beri değiştiyse source_changed, ulaşılamazsa source_unreachable); bağ kopyayı gösterir, uzlaştırma tabanı düşer. keep bağı olduğu gibi bırakır (package_left: kept ya da database). embed paketin projesinde bağı kaldırır, katman nesneleriyle projenin kendi katmanı olur (cevabın embedded'ı). \
                      leaveOut pakete girmeyecekleri söyler: bir bağ (kaynak-N; katmanın nesneleri paketten çıkar), bir nesnenin kalıcı kimliği (dosyaları kopyalanmaz) ya da xref:<ad> (dış başvurunun kaynağı kopyalanmaz); package_left excluded der, var olmayanı gösteren öğe invalid_input. validate veritabanını okumaz. \
                      Klasör bütün yapılır ya da hiç: geçici adla yazılır, her kopya geri okunup SHA-256'sı karşılaştırılır, proje dosyası doğrulanır, sonra tek adımla yerine konur; path var olan ve boş olmayan bir klasörse reddedilir (file_exists). plan yazmadan yazılacak olanı söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["paketle".into(), "package".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::PackageProject>(),
            output: schema::<crate::ProjectPackaged>(),
            plan: Some(schema::<crate::ProjectPackaged>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Teslim klasörü".into(),
                input: json!({ "path": "/teslim/kadastro-paket", "name": "kadastro", "from": "/is/kadastro" }),
                output: None,
            }, CommandExample {
                title: "Veritabanı tabloları kopyalanır, bir katman dışarıda kalır".into(),
                input: json!({ "path": "/teslim/saha-paket", "from": "/is/kadastro", "databases": "copy", "leaveOut": ["kaynak-3"] }),
                output: None,
            }],
        },
        // The drawing's layers as a GeoPackage (docs/adr/0502).
        CommandDescriptor {
            id: crate::FILE_EXPORT_GPKG.into(),
            version: crate::FILE_EXPORT_GPKG_VERSION,
            title: "GeoPackage dışa aktar".into(),
            summary: "Katmanları (layers boşsa nesnesi olan bütün katmanları; grup katmanlarını verir) path'e yeni bir GeoPackage olarak yazar: her katman bir tablo, her nesne bir satır; katmanın alanları türleriyle, öbür öznitelikler metin sütunu olarak; nesnenin kalıcı kimliği kentos_uids uzantısında (geri alınınca aynı kimlikle gelir). \
                      Nokta ve çok nokta, çizgi ve çoklu çizgi (yaylar dairesel eğri olarak kesin), alan delikleri ve parçalarıyla, daire eğri alan, yay dairesel eğri olarak yazılır; Z ve M taşınır; elips, eğri ve NURBS 0,1 mm'lik kirişlerle yazılır, geometrisiz kayıt geometrisiz satırdır. \
                      Yazı, ölçü, blok, tarama, tablo, resim ve rasterin GeoPackage'da geometrisi yoktur: yazılmaz, export_losses türlerine göre sayar. Tabloların sistemi projeninkidir (EPSG kodu ve WKT'si; yerel proje tanımsız kartezyen). \
                      Dosya bütün yazılır ya da hiç: geçici adla yazılır, geri okunur, tek adımla yerine konur; var olan dosyanın üzerine yalnız overwrite ile yazılır. plan dosyayı yazmadan yazılacak olanı söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["gpkg".into(), "gpkgout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportGpkg>(),
            output: schema::<crate::GpkgExported>(),
            plan: Some(schema::<crate::GpkgExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsel ve yol katmanları".into(),
                input: json!({ "path": "/veri/teslim.gpkg", "layers": ["parsel", "yol"], "overwrite": true }),
                output: None,
            }],
        },
        // The drawing's layers as Shapefiles (docs/adr/0514).
        CommandDescriptor {
            id: crate::FILE_EXPORT_SHP.into(),
            version: crate::FILE_EXPORT_SHP_VERSION,
            title: "Shapefile dışa aktar".into(),
            summary: "Katmanları (layers boşsa nesnesi olan bütün katmanları; grup katmanlarını verir) ESRI Shapefile olarak yazar: path .zip ise hepsi tek arşivde, .shp ise dışa aktarmanın yaptığı tek Shapefile o adla, başka bir yol ise yeni ya da boş bir klasöre katmanların adlarıyla. \
                      Bir Shapefile tek geometri türü taşır: noktası, çizgisi ve alanı olan katman _nokta, _cizgi, _alan ekli ayrı Shapefile'lar olur; geometrisiz kayıt boş şekilli satırdır (yalnız kayıtlı katman yalnız .dbf). Alan halkaları Shapefile'ın yönüyle, delikleri ve parçalarıyla; Z ve çizginin M'si taşınır, kotsuz köşe 0 kotlu yazılır. \
                      Daire, yay, elips ve yaylı yollar eğriden en çok 0,1 mm sapan kirişlerle yazılır ve sayılır; blok açılır; yazı, ölçü, tablo, resim, raster ve nokta bulutu yazılmaz, türlerine göre sayılır. \
                      .dbf'te sütun adları en çok 10 bayttır ve büyük küçük harf ayırmaz: kısaltılan ya da numaralanan adlar söylenir. Alan türleri dBASE'inkine çevrilir (metin C en çok 254 bayt, tam sayı N en çok 18 hane, ondalık N en çok 15 anlamlı basamak, tarih D, evet/hayır L); sığmayan ya da kuralına uymayan alan metin sütunu olur ve söylenir; boş metin ile değerin yokluğu ayrılamaz. Kod sayfası .cpg'de (UTF-8 ya da Windows-1254); .prj projenin sistemini WKT olarak adlandırır. \
                      Bütün yazılır ya da hiç; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["shp".into(), "shpout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportShp>(),
            output: schema::<crate::ShpExported>(),
            plan: Some(schema::<crate::ShpExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsel katmanı tek arşivde, Windows-1254".into(),
                input: json!({ "path": "/veri/parsel.zip", "layers": ["parsel"], "encoding": "windows-1254", "overwrite": true }),
                output: None,
            }],
        },
        // A CSV table of objects, out and in (docs/adr/0522).
        CommandDescriptor {
            id: crate::FILE_EXPORT_CSV.into(),
            version: crate::FILE_EXPORT_CSV_VERSION,
            title: "CSV dışa aktar".into(),
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) nesnelerini tek bir CSV'ye yazar: UTF-8, CRLF satırlar, RFC 4180 tırnaklama; virgül, noktalı virgül ya da sekme. \
                      geometry wkt (varsayılan) geometriyi ilk sütunda ISO WKT olarak yazar (yaylar CIRCULARSTRING ve COMPOUNDCURVE, kesin; Z ve M; elips, eğri ve NURBS kirişlerle, söylenir); xy noktaları X (doğu), Y (kuzey) ve kotu olanda Z sütunlarıyla yazar, nokta olmayanları sayar; none yalnız öznitelikleri. \
                      Birden çok katmanda katman sütunu satırın katmanını adlandırır. Alanlar katmanların alanlarından (sırasıyla), alan olmayan öznitelikler adlarıyla metin sütunu; yanına GDAL'ın .csvt'si sütunların türlerini (WKT, CoordX, CoordY, String, Integer64, Real, Date, DateTime, Time, Integer(Boolean)), .prj projenin sistemini (Shapefile'ınki gibi ESRI WKT) yazar. \
                      Yazı, ölçü, tablo, resim, raster ve nokta bulutu yazılmaz, türlerine göre sayılır (export_losses). Bütün yazılır ya da hiç; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["csvout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportCsv>(),
            output: schema::<crate::CsvExported>(),
            plan: Some(schema::<crate::CsvExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parseller WKT'li CSV olarak".into(),
                input: json!({ "path": "/veri/parsel.csv", "layers": ["parsel"], "geometry": "wkt", "overwrite": true }),
                output: None,
            }],
        },
        // FlatGeobuf, out and in (docs/adr/0544).
        CommandDescriptor {
            id: crate::FILE_EXPORT_FGB.into(),
            version: crate::FILE_EXPORT_FGB_VERSION,
            title: "FlatGeobuf dışa aktar".into(),
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) nesnelerini tek bir FlatGeobuf dosyasına yazar (v3; GDAL, QGIS ve biçimin kendi kitaplıkları okur). \
                      Geometri biçimin türleriyle: nokta, çok nokta, çizgi, alan ve delikleri, çok parçalı; yaylar CircularString, CompoundCurve, CurvePolygon, MultiCurve ve MultiSurface olarak kesin; Z ve M köşelerde; elips, eğri ve NURBS kirişlerle (söylenir). \
                      Katmanın alanları sütun olur (metin String, tam sayı Long, ondalık Double, evet/hayır Bool, tarih ve tarih-saat DateTime); alan olmayan öznitelikler adlarıyla metin sütunu; birden çok katmanda katman sütunu nesnenin katmanını adlandırır. Projenin sistemi EPSG koduyla yazılır. \
                      Nesneler kutularının Hilbert sırasıyla, önlerinde paketli R-ağacı dizini (16'lık düğümler) yazılır; konumsuz kayıt varsa dizin yazılmaz (GDAL'ın kuralı). Yazı, ölçü, tablo, resim, raster ve nokta bulutu yazılmaz, türlerine göre sayılır (export_losses). Bütün yazılır ya da hiç; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["fgbout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportFgb>(),
            output: schema::<crate::FgbExported>(),
            plan: Some(schema::<crate::FgbExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parseller tek FlatGeobuf'ta".into(),
                input: json!({ "path": "/veri/parsel.fgb", "layers": ["parsel"], "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_FGB.into(),
            version: crate::FILE_IMPORT_FGB_VERSION,
            title: "FlatGeobuf içe aktar".into(),
            summary: "path'teki FlatGeobuf'un nesnelerini açık çizime alır, tek geri alma adımı (“FlatGeobuf: ad.fgb”): her nesne çizimin nesnesi ya da geometrisiz kayıt, özellikleri öznitelikleri. \
                      Bütün geometri türleri (yaylı olanlar kesin; çokyüzlü yüzey, TIN ve üçgen okunmaz, söylenir), Z ve M; sütunlar türleriyle alan olur (bütün değerleri gün olan tarih-saat sütunu tarih). Sistem dosyanın EPSG kodundan, yoksa srid'den; projeninkinden başkaysa reproject ile çevrilir, yoksa crs_mismatch. \
                      Nesneler layer'a, yoksa dosyanın katman adındaki katmana, yoksa sütunları alan olarak taşıyan yeni katmana gider (GeoPackage içe aktarmanın kuralı); kalıcı kimlik dosyanın adı ve nesnenin sırasından. Dizin okunmaz, atlanır; bozuk dosya (sınırların, uzunlukların ya da başvuruların dışı) reddedilir. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir (eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır, yinelenen anahtar durdurur ya da reddedilir), rejects reddedilen satırları nedenleriyle CSV'ye yazar (docs/adr/0628)."
                .into(),
            aliases: vec!["fgbin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportFgb>(),
            output: schema::<crate::GpkgImported>(),
            plan: Some(schema::<crate::GpkgImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "GDAL'ın yazdığı parseller".into(),
                input: json!({ "path": "/veri/parsel.fgb" }),
                output: None,
            }],
        },
        // GeoParquet, out and in (docs/adr/0628).
        CommandDescriptor {
            id: crate::FILE_EXPORT_PARQUET.into(),
            version: crate::FILE_EXPORT_PARQUET_VERSION,
            title: "GeoParquet dışa aktar".into(),
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) nesnelerini tek bir GeoParquet 1.1 dosyasına yazar (KentOS'un kendi Parquet yazıcısıyla; GDAL, geopandas, DuckDB ve QGIS okur). \
                      geometry sütunu ISO WKB: yaylar 0,1 mm'lik kirişlerle, elips, eğri ve NURBS kirişlerle (söylenir), Z köşelerde, M yazılmaz (söylenir); dış halkalar saat yönü tersine, delikler saat yönünde (orientation: counterclockwise). \
                      Katmanın alanları türleriyle sütun olur (metin STRING, tam sayı INT64, ondalık DOUBLE, evet/hayır BOOLEAN, tarih DATE, tarih-saat TIMESTAMP(MICROS), saat TIME(MICROS)); alan olmayan öznitelikler metin sütunu; kentos_uid nesnelerin kalıcı kimliği; birden çok katmanda katman sütunu. \
                      covering açıkken (varsayılan) bbox yapısı (xmin, ymin, xmax, ymax) satır kutularını taşır. geo anahtarı geometry_types, bbox ve projenin sisteminin PROJJSON'unu (kimliğiyle) yazar; yerel projede crs null. Sayfalar Snappy ile (compression: none sıkıştırmasız), 65 536 satırlık satır grupları, sütun istatistikleri. \
                      Yazı, ölçü, tablo, resim, raster ve nokta bulutu yazılmaz, türlerine göre sayılır (export_losses). Bütün yazılır ya da hiç; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["parquetout".into(), "geoparquetout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportParquet>(),
            output: schema::<crate::ParquetExported>(),
            plan: Some(schema::<crate::ParquetExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parseller tek GeoParquet'te".into(),
                input: json!({ "path": "/veri/parsel.parquet", "layers": ["parsel"], "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_PARQUET.into(),
            version: crate::FILE_IMPORT_PARQUET_VERSION,
            title: "GeoParquet içe aktar".into(),
            summary: "path'teki GeoParquet'in (ya da düz Parquet'in) satırlarını açık çizime alır, tek geri alma adımı (“GeoParquet: ad.parquet”): her satır çizimin nesnesi ya da geometrisiz kayıt, sütunları öznitelikleri. \
                      KentOS'un kendi Parquet okuyucusu: veri sayfası v1 ve v2, sözlük, PLAIN, RLE, DELTA_BINARY_PACKED, DELTA_LENGTH_BYTE_ARRAY, DELTA_BYTE_ARRAY, BYTE_STREAM_SPLIT; sıkıştırmasız, Snappy, GZIP, ZSTD, LZ4_RAW ve LZ4 (Brotli ve LZO reddedilir). \
                      geo anahtarının birincil sütunu WKB (Z kot, M ölçü) ya da GeoArrow (point … multipolygon) olarak, anahtar yoksa Parquet'in GEOMETRY türü okunur; başka geometri sütunu WKT metni olur (söylenir), covering kutu sütunu alınmaz. Sistem sütunun PROJJSON'unun kimliğinden (crs yoksa OGC:CRS84, null ise srid'den); projeninkinden başkaysa reproject ile çevrilir, yoksa crs_mismatch. \
                      Sütunlar türleriyle alan olur: tam sayılar (2⁵³'ü aşan metin), ondalık sayılar ve yarım duyarlılık, evet/hayır, metin, UUID, tarih, saat, zaman damgası ve INT96 (tarih-saat; saat dilimi ve saniyenin kesri söylenir), ondalık türü (15 basamağa kadar sayı, üstü kesin metin), tek düzeyli liste JSON metni; ikili, yapı, eşleme ve iç içe liste alınmaz (söylenir). \
                      Satırlar layer'a, yoksa dosyanın adındaki katmana, yoksa sütunları alan olarak taşıyan yeni katmana gider; kalıcı kimlik kentos_uid sütunundan, yoksa dosyanın adı ve satırın sırasından. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir; rejects reddedilen satırları CSV'ye yazar. plan dosyayı okur, hiçbir şey yazmaz."
                .into(),
            aliases: vec!["parquetin".into(), "geoparquetin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportParquet>(),
            output: schema::<crate::GpkgImported>(),
            plan: Some(schema::<crate::GpkgImported>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "geopandas'ın yazdığı parseller".into(),
                    input: json!({ "path": "/veri/parsel.parquet" }),
                    output: None,
                },
                CommandExample {
                    title: "Dönem verisini parsel numarasıyla birleştir, reddedilenleri yaz".into(),
                    input: json!({ "path": "/veri/parsel-2026.parquet", "merge": { "key": "parsel_no", "duplicates": "first" }, "rejects": "/veri/reddedilenler.csv" }),
                    output: None,
                },
            ],
        },
        // GML 3.2 and XYZ, out and in (docs/adr/0545).
        CommandDescriptor {
            id: crate::FILE_EXPORT_GML.into(),
            version: crate::FILE_EXPORT_GML_VERSION,
            title: "GML dışa aktar".into(),
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) nesnelerini tek bir GML 3.2 dosyasına yazar (OGC basit nesneler; GDAL, QGIS ve ArcGIS okur). \
                      Her katman bir nesne türüdür (adı katmanın adından, XML adına çevrilir); katmanın alanları özellik olur, boş değer xsi:nil, geometri geometryProperty'de. \
                      Geometri: Point, LineString, Polygon (exterior ve interior), MultiPoint, MultiCurve, MultiSurface, MultiGeometry; yaylar ve bileşik eğriler Curve'ün ArcString kesimi, Ring ve CompositeCurve olarak kesin; Z posList'in srsDimension'ıyla (Z'si olmayan köşe 0 yazılır, söylenir); M GML'de yok. Elips, eğri ve NURBS kirişlerle (söylenir). \
                      Projenin sistemi urn:ogc:def:crs:EPSG::kod olarak srsName'e yazılır ve koordinatlar sistemin EPSG eksen sırasında gider (TUREF TM dilimleri ve coğrafi sistemler kuzey ya da enlem önce). Dosyanın yanına alanların türlerini veren <ad>.xsd uygulama şeması yazılır (schema false ise yazılmaz). \
                      Yazı, ölçü, tablo, resim, raster ve nokta bulutu yazılmaz, türlerine göre sayılır (export_losses). Bütün yazılır ya da hiç (önce şema, sonra GML); plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["gmlout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportGml>(),
            output: schema::<crate::GmlExported>(),
            plan: Some(schema::<crate::GmlExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parseller GML 3.2 olarak".into(),
                input: json!({ "path": "/veri/parsel.gml", "layers": ["parsel"], "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_GML.into(),
            version: crate::FILE_IMPORT_GML_VERSION,
            title: "GML içe aktar".into(),
            summary: "path'teki GML'in nesnelerini açık çizime alır, tek geri alma adımı (“GML: ad.gml”): her nesne çizimin nesnesi ya da geometrisiz kayıt, özellikleri öznitelikleri. \
                      GML 2, 3.1 ve 3.2 okunur (featureMember, featureMembers ve member; Point, LineString, LinearRing, Polygon, MultiPoint, MultiCurve, MultiLineString, MultiSurface, MultiPolygon, MultiGeometry, Curve'ün LineStringSegment, ArcString ve Arc kesimleri, CompositeCurve, Ring, Surface; pos, posList ve coordinates). Yaylar kesin; Z posList'in srsDimension'ından ya da pos'un sayısından. \
                      Sistem geometrinin srsName'inden (urn ve http adları EPSG'nin eksen sırasıyla, EPSG:kod doğu önce), yoksa koleksiyonun boundedBy'ından, yoksa srid'den; projeninkinden başkaysa reproject ile çevrilir, yoksa crs_mismatch. \
                      Her nesne türü (tables, boşsa hepsi) bir tablodur: layers'ın adladığı katmana, yoksa türün adındaki katmana, yoksa özellikleri alan olan yeni katmana gider (GeoPackage içe aktarmanın kuralı). Özelliklerin türleri dosyanın yanındaki .xsd'den, yoksa .gfs'den, yoksa değerlerden; xsi:nil boş değer, boş öğe boş metin. Kalıcı kimlik dosyanın adı, nesne türü ve sırasından. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir (eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır, yinelenen anahtar durdurur ya da reddedilir), rejects reddedilen satırları nedenleriyle CSV'ye yazar (docs/adr/0628)."
                .into(),
            aliases: vec!["gmlin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportGml>(),
            output: schema::<crate::GpkgImported>(),
            plan: Some(schema::<crate::GpkgImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "GDAL'ın yazdığı parseller".into(),
                input: json!({ "path": "/veri/parsel.gml" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_SHP.into(),
            version: crate::FILE_IMPORT_SHP_VERSION,
            title: "Shapefile içe aktar".into(),
            summary: "path'teki Shapefile'ı (.shp; yanındaki .shx, .dbf, .prj ve .cpg adından, büyük küçük harfe bakmadan bulunur, eksik olan söylenir) ya da .zip arşivini (her .shp bir katman; entry biri seçer) açık çizime alır, tek geri alma adımı (“Shapefile: ad.shp”). \
                      Nokta, çok nokta, çizgi, çok parçalı çizgi, alan (delikleriyle, çok parçalı) ve Z ile M ölçüleri okunur; DBF alanları türleriyle katmanın alanları ve nesnelerin öznitelikleri olur (kod sayfası .cpg'den ya da DBF'in dil sürücüsünden). \
                      Sistem önce srid'den, yoksa dosyanın bildirdiğinden (.prj dosyası), hiçbiri yoksa projeninkinden alınır (crs_undeclared); okunamayan ya da KentOS'ta tanımlı olmayan bildirim crs_unknown verir. \
                      Projeninkinden başka bir sistemdeki dosya yalnız reproject ile projenin sistemine çevrilir (köşeler sistemden sisteme, kotlar olduğu gibi), yoksa crs_mismatch; cevabın moved'ı çevirmenin yolunu ve doğruluğunu söyler (datum işlemleri, ±metre, resmî mi), çevrilemeyen nesneler alınmaz ve raporda yazılır. Yerel projede (sistemsiz) koordinatlar olduğu gibi alınır. \
                      Her kaynak katman layers'ın adladığı katmana, yoksa layer'a, yoksa çizimdeki aynı adlı katmana, yoksa dosyanın alanlarını taşıyan yeni katmana (group'ta) gider; kilitli ya da grup olan hedef reddedilir. Alınamayanlar (okunamayan nesneler, tanınmayan türler, kısaltılan değerler) uyarıda ve cevabın raporunda söylenir. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla birleştirir (bu dosyaların satırlarının kalıcı kimliği yok; eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır), rejects reddedilen satırları nedenleriyle CSV'ye yazar (docs/adr/0628)."
                .into(),
            aliases: vec!["shpin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportShp>(),
            output: schema::<crate::GisImported>(),
            plan: Some(schema::<crate::GisImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "TM dilimindeki parseller, var olan katmana".into(),
                input: json!({ "path": "/veri/parseller.shp", "layer": "parsel" }),
                output: None,
            }, CommandExample {
                title: "WGS 84'teki arşiv projenin sistemine çevrilerek".into(),
                input: json!({ "path": "/veri/portal.zip", "entry": "binalar", "reproject": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_GEOJSON.into(),
            version: crate::FILE_IMPORT_GEOJSON_VERSION,
            title: "GeoJSON içe aktar".into(),
            summary: "path'teki GeoJSON dosyasını (FeatureCollection, Feature ya da çıplak geometri; RFC 7946 ve 2008'in crs üyesi) açık çizime alır, tek geri alma adımı (“GeoJSON: ad.geojson”). \
                      Point, MultiPoint, LineString, MultiLineString, Polygon (delikleriyle), MultiPolygon ve GeometryCollection okunur, üçüncü sayı kottur; KentOS'un yazdığı kentos üyesi katmanı ve etiketi geri verir, ötekiler dosyanın name'indeki ya da dosya adındaki katmana gider; özellikler öznitelik olur, alanların türleri değerlerinden çıkar. \
                      Sistem önce srid'den, yoksa dosyanın bildirdiğinden (crs üyesi, yoksa RFC 7946'nın WGS 84 boylam, enlemi), hiçbiri yoksa projeninkinden alınır (crs_undeclared); okunamayan ya da KentOS'ta tanımlı olmayan bildirim crs_unknown verir. \
                      Projeninkinden başka bir sistemdeki dosya yalnız reproject ile projenin sistemine çevrilir (köşeler sistemden sisteme, kotlar olduğu gibi), yoksa crs_mismatch; cevabın moved'ı çevirmenin yolunu ve doğruluğunu söyler (datum işlemleri, ±metre, resmî mi), çevrilemeyen nesneler alınmaz ve raporda yazılır. Yerel projede (sistemsiz) koordinatlar olduğu gibi alınır. \
                      Her kaynak katman layers'ın adladığı katmana, yoksa layer'a, yoksa çizimdeki aynı adlı katmana, yoksa dosyanın alanlarını taşıyan yeni katmana (group'ta) gider; kilitli ya da grup olan hedef reddedilir. Alınamayanlar (okunamayan nesneler, tanınmayan türler, kısaltılan değerler) uyarıda ve cevabın raporunda söylenir. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla birleştirir (bu dosyaların satırlarının kalıcı kimliği yok; eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır), rejects reddedilen satırları nedenleriyle CSV'ye yazar (docs/adr/0628)."
                .into(),
            aliases: vec!["geojsonin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportGeojson>(),
            output: schema::<crate::GisImported>(),
            plan: Some(schema::<crate::GisImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "WGS 84'teki noktalar projenin sistemine çevrilerek".into(),
                input: json!({ "path": "/veri/noktalar.geojson", "reproject": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_KML.into(),
            version: crate::FILE_IMPORT_KML_VERSION,
            title: "KML/KMZ içe aktar".into(),
            summary: "path'teki KML dosyasını ya da KMZ arşivini (ana .kml'i) açık çizime alır, tek geri alma adımı (“KML: ad.kml”). \
                      Klasörler katman, yer imleri nesne olur (Point, LineString, LinearRing, Polygon delikleriyle, MultiGeometry, gx:Track); biçemin rengi ve kalınlığı, adı etiket, açıklaması, zamanı ve verileri öznitelik olur; resim kaplamaları, modeller ve bağlantılar alınmaz, söylenir. \
                      Sistem önce srid'den, yoksa dosyanın bildirdiğinden (KML 2.2'nin WGS 84 boylam, enlemi), hiçbiri yoksa projeninkinden alınır (crs_undeclared); okunamayan ya da KentOS'ta tanımlı olmayan bildirim crs_unknown verir. \
                      Projeninkinden başka bir sistemdeki dosya yalnız reproject ile projenin sistemine çevrilir (köşeler sistemden sisteme, kotlar olduğu gibi), yoksa crs_mismatch; cevabın moved'ı çevirmenin yolunu ve doğruluğunu söyler (datum işlemleri, ±metre, resmî mi), çevrilemeyen nesneler alınmaz ve raporda yazılır. Yerel projede (sistemsiz) koordinatlar olduğu gibi alınır. \
                      Her kaynak katman layers'ın adladığı katmana, yoksa layer'a, yoksa çizimdeki aynı adlı katmana, yoksa dosyanın alanlarını taşıyan yeni katmana (group'ta) gider; kilitli ya da grup olan hedef reddedilir. Alınamayanlar (okunamayan nesneler, tanınmayan türler, kısaltılan değerler) uyarıda ve cevabın raporunda söylenir. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla birleştirir (bu dosyaların satırlarının kalıcı kimliği yok; eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır), rejects reddedilen satırları nedenleriyle CSV'ye yazar (docs/adr/0628)."
                .into(),
            aliases: vec!["kmlin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportKml>(),
            output: schema::<crate::GisImported>(),
            plan: Some(schema::<crate::GisImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Google Earth'ün yer imleri projenin sistemine çevrilerek".into(),
                input: json!({ "path": "/veri/yerler.kmz", "reproject": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_EXPORT_XYZ.into(),
            version: crate::FILE_EXPORT_XYZ_VERSION,
            title: "XYZ dışa aktar".into(),
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) konumlarını X Y Z satırları olarak metin dosyasına yazar (UTF-8, LF; X doğu, Y kuzey, projenin sisteminde; GDAL'ın XYZ sürücüsü ve nokta bulutu araçları okur). \
                      vertices points (varsayılan): nokta nesnelerinin ve çok noktanın parçalarının konumu, öbür nesneler yazılmaz ve türlerine göre sayılır; all: her nesnenin her köşesi (çizgi, alanın dış ve delik halkaları; kapalı halkanın tekrarlanan son köşesi bir kez; yayın uç noktaları, orta noktası değil). \
                      Ayırıcı boşluk, virgül, noktalı virgül ya da sekme; sayılar double'a geri okunan en kısa metin, decimals verilirse o kadar basamak (0 – 15); header ilk satıra sütun adlarını yazar. Kotu olmayan konum withoutZ'ye göre 0 kotla yazılır (zero; çizimdeki konum kotsuz kalır, söylenir) ya da yazılmaz (skip). \
                      Dosya koordinat sistemi taşımaz; projenin EPSG kodu uyarıda ve cevapta söylenir. Bütün yazılır ya da hiç; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["xyzout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportXyz>(),
            output: schema::<crate::XyzExported>(),
            plan: Some(schema::<crate::XyzExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ölçü noktaları, üç basamakla".into(),
                input: json!({ "path": "/veri/noktalar.xyz", "layers": ["nokta"], "decimals": 3, "header": true, "overwrite": true }),
                output: None,
            }],
        },
        // A table's cells as a workbook (docs/adr/0733 §7).
        CommandDescriptor {
            id: crate::FILE_EXPORT_TABLE.into(),
            version: crate::FILE_EXPORT_TABLE_VERSION,
            title: "Tabloyu dosyaya yaz".into(),
            summary: "Bir tablonun (uid: mimari çizelge, blok çizelgesi, dosyadan ya da elle yazılmış tablo) hücrelerini yolun uzantısına göre CSV, XLSX ya da ODS olarak yazar; ilk satır sütunların adları. \
                      Çizelge belgenin şimdi yazdığı gibidir: groups (varsayılan true) grup başlıklarını, ara toplamları ve genel toplamı da yazar, false yalnız veri satırlarını. \
                      numbers shown (varsayılan) sayıları çizelgede görüldüğü gibi projenin basamağıyla, exact hesaplanan sayıları çizimden yeniden okuyup 15 anlamlı basamağa yuvarlayarak yazar; nesnelere yazılan sütunlar (numara, ad, tür, kaplama, öznitelik) iki kipte de yazıldığı gibidir, yuvarlanmaz. \
                      Cevabın notes'u kipi ve yazıcının dediklerini (15 basamağı aşan değer metin yazıldı gibi) söyler. Bütün yazılır ya da hiç; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["tabloyaz".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportTable>(),
            output: schema::<crate::TableExported>(),
            plan: Some(schema::<crate::TableExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kapı çizelgesini Excel'e, ara toplamlarıyla".into(),
                input: json!({ "path": "/veri/kapilar.xlsx", "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e70", "overwrite": true }),
                output: None,
            }],
        },
        // The atlas of a sheet as a PDF with a page for each object (docs/adr/0534).
        CommandDescriptor {
            id: crate::FILE_EXPORT_ATLAS.into(),
            version: crate::FILE_EXPORT_ATLAS_VERSION,
            title: "Atlası PDF yap".into(),
            summary: "Atlası olan bir paftayı (sheet), kapsam katmanının her nesnesi için bir sayfayla tek PDF'e yazar. Sayfaları paftanın atlas kuralları seçer: süzgeç, sıralama, sayfa adı, ölçek ilkesi. \
                      Çıktı sabitlenmiş bir durumdan yapılır: çizimin parçalarının damgaları, her kullanılan katmanın özeti ve bağlı katmanın kaynak dosyasının SHA-256'sı (pin). plan'ın verdiği pin'i execute'a vermek, plan'dan sonra çizim ya da kaynak değiştiyse pin_mismatch (çatışma) ile reddedilir; sayfalar yapılırken bağlı bir dosya değişirse hiçbir şey yazılmaz (source_changed) ve çıktının yenilenmesi istenir. Çıktının Keywords'ünde kentos.atlas/1, pin'in özeti, tema, zaman ve sistem yazılıdır. \
                      Bağlar açıktır: theme (projenin bir katman durumu, atlas haritaları onunla çizilir), time (zamanlı katmanların gösterildiği an: kapsam nesneleri ve harita onunla kesilir), crs (yalnız projenin sistemi; başka sistem crs_unsupported). \
                      Sayfaların taşıdığı öznitelikler izinlidir: fields (yoksa kapsam katmanının tanımlı alanları; alanı olmayan katmanda hepsi); bir formda redact işaretli alan hiçbir zaman taşınmaz. Katmanın süzgecinden ya da zamandan geçmeyen nesne ne sayfa olur ne haritada, tabloda, lejantta görünür. GeoPDF'te (TM sistem) her harita çerçevesi coğrafi görüntü penceresi taşır. \
                      plan dosyayı yazmaz, sayfaları ve pin'i söyler; çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["atlaspdf".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportAtlas>(),
            output: schema::<crate::AtlasExported>(),
            plan: Some(schema::<crate::AtlasExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsel atlası, bugünün durumundan".into(),
                input: json!({ "path": "/veri/parsel-atlasi.pdf", "sheet": "atlas-paftasi", "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_CSV.into(),
            version: crate::FILE_IMPORT_CSV_VERSION,
            title: "CSV içe aktar".into(),
            summary: "path'teki CSV'nin satırlarını açık çizime alır, tek geri alma adımı (“CSV: ad.csv”): her satır bir nesne ya da geometrisiz kayıt, öbür sütunları öznitelikleri. \
                      Geometri (geometry): auto WKT adlı sütunu (WKT, geom, geometry, the_geom, wkt_geom), yoksa X ve Y çiftini (x/y, lon/lat, longitude/latitude, easting/northing, doğu/kuzey; Z ya da kot kotu) bulur; wkt ve xy sütunları adıyla verir; none her satırı kayıt yapar. \
                      Ayırıcı ve kodlama dosyadan bulunur. Sütunların türleri yanındaki .csvt'den (GDAL'ın), yoksa değerlerden: hepsi tam sayıysa tam sayı, sayıysa ondalık, tarihse tarih (başı sıfırlı kodlar metin kalır). Sistem .prj'den (ESRI ya da EPSG yetkisi), yoksa srid'den; projeninkinden başkaysa reproject ile çevrilir, yoksa crs_mismatch. \
                      Satırlar layer'a, yoksa dosyanın adındaki katmana, yoksa sütunları alan olarak taşıyan yeni katmana gider (GeoPackage içe aktarmanın kuralı); kalıcı kimlik dosyanın adı ve satırından. Okunamayan geometrili satır alınmaz, satır numarasıyla söylenir (import_losses). plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir (eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır, yinelenen anahtar durdurur ya da reddedilir), rejects reddedilen satırları nedenleriyle CSV'ye yazar (docs/adr/0628)."
                .into(),
            aliases: vec!["csvin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportCsv>(),
            output: schema::<crate::GpkgImported>(),
            plan: Some(schema::<crate::GpkgImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "QGIS'in yazdığı WKT'li CSV".into(),
                input: json!({ "path": "/veri/parsel.csv", "geometry": { "kind": "auto" } }),
                output: None,
            }],
        },
        // The drawing as a traceable GIS dataset (docs/adr/0530).
        CommandDescriptor {
            id: crate::FILE_EXPORT_GIS.into(),
            version: crate::FILE_EXPORT_GIS_VERSION,
            title: "GIS türevi dışa aktar".into(),
            summary: "Katmanları (layers boşsa nesnesi olan bütün katmanları) GeoPackage'a türev olarak yazar: çizim kaynak kalır, dosyadaki geometri onun bir türevidir. \
                      Her katman bir özellik tablosudur; her satır kaynak nesnenin kalıcı kimliğini (kentos_uid sütunu ve kentos_uids uzantısı) ve türünü (kentos_kind), türev sapmasını metrede (kentos_deviation) ve kaynağın özetini (kentos_digest) taşır, katmanın alanları ve öteki öznitelikler sütun olur; Z ve M köşelerde taşınır. \
                      Yaylar, daireler, çoklu çizgilerin yaylı kenarları ve elipsler, eğriler, NURBS düz kirişlerdir (GeoPackage'ın düz geometri türleri; eğri uzantısı yok): kirişler kaynağın eğrisinden en çok tolerance (metre, varsayılan 0,0001) uzakta kalır. Yay, daire ve yaylı kenarlarda algoritma kentos.chords/1'dir (eşit açılı kirişler, uçlar yayın üzerinde, sapma 2·r·sin²(φ/4) ile kesin); elips ve eğride sapma algoritmanın tuttuğu sınırdır, NURBS'ta örnek noktalarında denenen toleranstır. \
                      Her tablonun türetim kaydı (https://kentos.app/derivation/1: algoritma, tolerans, kaynak proje, ad ve sürüm, en büyük sapma, kaynak türlerine göre sapmanın nasıl bilindiği, zaman) GeoPackage'ın metadata uzantısındadır. \
                      Yazı ve bloklar tolerance'ın yanında annotations'a bağlıdır: table (varsayılan) onları katman tablolarına katmaz, ayrı tablolara yazar (her blok tanımının yerleştirmeleri blok adıyla nokta olarak, bloğun öznitelikleri sütun; yazılar kentos_annotations'ta nokta olarak, sözleri sütunda), skip onları sayar. Ölçü, tarama, kılavuz, tablo, resim, raster, nokta bulutu, yardımcı çizgi ve mimari nesneler yazılmaz; türlerine göre nedeniyle left'te ve export_losses uyarısındadır. \
                      Bütün yazılır ya da hiç; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir. Türevi çizime bağlamak ya da geri yazmak olmaz (derived_dataset); gerçeği cad.gis.derivation sınar."
                .into(),
            aliases: vec!["gisturev".into(), "gisexport".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportGis>(),
            output: schema::<crate::GisExported>(),
            plan: Some(schema::<crate::GisExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parseller 1 mm toleransla".into(),
                input: json!({ "path": "/veri/parsel-turev.gpkg", "layers": ["parsel"], "tolerance": 0.001, "overwrite": true }),
                output: None,
            }],
        },
        // A layer's look as OGC SLD 1.1 / SE 1.1, out and in (docs/adr/0516).
        CommandDescriptor {
            id: crate::FILE_EXPORT_SLD.into(),
            version: crate::FILE_EXPORT_SLD_VERSION,
            title: "Görünüşü SLD olarak dışa aktar".into(),
            summary: "Katmanın görünüşünü (işleyicisini, yoksa katmanın renk, çizgi tipi, kalınlık, dolgu ve nokta simgesini) OGC SLD 1.1 ve SE 1.1 olarak path'e (.sld) yazar; QGIS 4.2, GeoServer ve MapServer okur. \
                      Tek simge tek kural, kategorili görünüşün her sınıfı alanın değerine eşit süzgeçli kural, aralıklı görünüşün sınıfları alt ≤ değer < üst (sonuncusu ≤ üst), kurallı görünüş süzgeçleriyle ve ölçek aralıklarıyla (iç içe kurallar üstlerinin süzgeciyle birleşir, Diğerleri ElseFilter). \
                      Süzgeç alanın sayı ya da metinle karşılaştırması, arasında, boş, gibi, ve, veya ve değil ile söylenebilen ifadedir; başkası yazılmaz ve söylenir. Simgeler: basit dolgu, basit çizgi (renk, saydamlık, kalınlık, kesikler, uç ve köşe), şekil işaretleri (daire, kare, üçgen, yıldız, artı, çarpı); kâğıt mm SE'nin 0,28 mm'lik pikseliyle, metre SE'nin metre birimiyle. \
                      Tarama, degrade, desen ve resim dolgusu, işaretli çizgi, SVG ve yazı işareti, veriye göre değer, efektler ve tematik işleyiciler yazılmaz, export_losses ile söylenir. Simgeleyiciler katmanın nesnelerinin geometrisi için yazılır. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["sldout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportSld>(),
            output: schema::<crate::SldExported>(),
            plan: Some(schema::<crate::SldExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsel katmanının görünüşü".into(),
                input: json!({ "path": "/veri/parsel.sld", "layer": "parsel", "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_SLD.into(),
            version: crate::FILE_IMPORT_SLD_VERSION,
            title: "SLD'den görünüş al".into(),
            summary: "path'teki (ya da text'teki) SLD'nin görünüşünü katmanın işleyicisi yapar, tek geri alma adımı (Katman stili): bir alanın değerlerine eşit kurallar kategorili görünüş, bir alanın aralıkları aralıklı görünüş, süzgeçsiz tek kural tek simge, ötekiler KentOS'un ifadesine çevrilen süzgeçleriyle kurallı görünüş olur; ölçek aralıkları ve ElseFilter korunur. \
                      SE'nin dolgusu, çizgisi (kesikler, uç, köşe, kaydırma), iyi bilinen işaretleri, saydamlıkları ve birimleri (piksel 0,28 mm kâğıt, metre) alınır; dış resim, işaretli çizgi ve desenli dolgu, etiket ve raster simgeleyicisi, çevrilemeyen süzgeç alınmaz, import_losses ile söylenir. \
                      QGIS'in, GeoServer'ın ve KentOS'un yazdığı SLD 1.0 ve 1.1 okunur; birden çok görünüşten ilki alınır. Katman aynı görünüşteyse adım yazılmaz (changed false). plan dosyayı okur, hiçbir şey yazmaz."
                .into(),
            aliases: vec!["sldin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportSld>(),
            output: schema::<crate::SldImported>(),
            plan: Some(schema::<crate::SldImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "QGIS'in yazdığı parsel görünüşü".into(),
                input: json!({ "layer": "parsel", "path": "/veri/parsel.sld" }),
                output: None,
            }],
        },
        // A layer's look as a QGIS layer style file, out and in (docs/adr/0555).
        CommandDescriptor {
            id: crate::FILE_EXPORT_QML.into(),
            version: crate::FILE_EXPORT_QML_VERSION,
            title: "Görünüşü QML olarak dışa aktar".into(),
            summary: "Katmanın görünüşünü (işleyicisini, yoksa katmanın renk, çizgi tipi, kalınlık, dolgu ve nokta simgesini) QGIS'in stil dosyası olarak path'e (.qml) yazar; QGIS 4.2 Stil › Stil yükle ile açar. \
                      Tek simge, kategorili (Diğerleri QGIS'in NULL sınıfı), aralıklı ve kurallı görünüş (iç içe kurallar, ölçek aralıkları, ELSE, kapalı kural) QGIS'in aynı türüyle; kuralların süzgeci QGIS'in ifadesiyle (\"Ad\" = 'x'): alanın sayı ya da metinle karşılaştırması, arasında, boş, gibi, ve, veya ve değil. \
                      Simgeler: basit dolgu, basit çizgi (renk, kalınlık, kesikler, uç, köşe, halkalar), şekil işaretleri (on beş şekil; döndürme QGIS'in saat yönüyle), merkez işareti; birimler mm, piksel ve haritada metre olarak kalır, simgenin saydamlığı ve kapalı katman korunur. \
                      QML tek geometri türü içindir: alanlar, sonra çizgiler, sonra noktalar; öbürleri, tarama, degrade, desen ve resim dolgusu, işaretli çizgi, SVG ve yazı işareti, veriye göre değer ve efektler yazılmaz, export_losses ile söylenir. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["qmlout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportQml>(),
            output: schema::<crate::QmlExported>(),
            plan: Some(schema::<crate::QmlExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsel katmanının görünüşü".into(),
                input: json!({ "path": "/veri/parsel.qml", "layer": "parsel", "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_QML.into(),
            version: crate::FILE_IMPORT_QML_VERSION,
            title: "QML'den görünüş al".into(),
            summary: "path'teki (ya da text'teki) QGIS stil dosyasının görünüşünü katmanın işleyicisi yapar, tek geri alma adımı (Katman stili): QGIS'in tek simge, kategorili, aralıklı ve kurallı görünüşü KentOS'un aynı türü olur; birden çok değerli sınıf her değer için bir sınıf, NULL sınıfı Diğerleri, kuralların süzgeci KentOS'un ifadesine çevrilir (IN de). \
                      Basit dolgu (kenarı ayrı çizgi), basit çizgi (Qt'nin kesik desenleri ya da özel kesikler), basit işaret ve merkez dolgusu, simgenin saydamlığı, kapalı katman, mm, nokta, inç, piksel ve harita birimi alınır. \
                      Başka simge katmanları, fırça desenleri, veriye bağlı değerler, etiketler, katmanın saydamlığı ve çevrilemeyen süzgeç alınmaz, import_losses ile söylenir; aralıkların sınır kuralı farkı da söylenir. QGIS 3 ve 4'ün yazdığı QML okunur. Katman aynı görünüşteyse adım yazılmaz (changed false). plan dosyayı okur, hiçbir şey yazmaz."
                .into(),
            aliases: vec!["qmlin".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportQml>(),
            output: schema::<crate::QmlImported>(),
            plan: Some(schema::<crate::QmlImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "QGIS'in yazdığı parsel görünüşü".into(),
                input: json!({ "layer": "parsel", "path": "/veri/parsel.qml" }),
                output: None,
            }],
        },
        // The layers' looks as a MapLibre GL style, out and in (docs/adr/0560).
        CommandDescriptor {
            id: crate::FILE_EXPORT_MAPLIBRE.into(),
            version: crate::FILE_EXPORT_MAPLIBRE_VERSION,
            title: "Görünüşü MapLibre stili olarak dışa aktar".into(),
            summary: "Bir katmanın (layer) ya da görünen bütün katmanların görünüşünü MapLibre GL stili (sürüm 8) olarak path'e (.json) yazar; MapLibre GL JS, MapLibre Native, Maputnik ve QGIS'in vektör karo stili açar. \
                      Her katmanın nesneleri sourceUrl verilmezse WGS 84 GeoJSON olarak stilin yanına yazılır (tek katmanda <ad>.geojson, birden çokta <ad>.<katman kimliği>.geojson) ve stilin kaynağı o dosyadır; sourceUrl verilirse kaynak o adrestir (vektör karo ya da GeoJSON adresi), katman adı source-layer olur. \
                      Katmanlar alan, çizgi ve daire (circle) katmanlarına yazılır: tek simge sabit değerlerle, kategorili görünüş `match` ile, aralıklı görünüş `step` ile (sınıflar aynı yapıdaysa her simge katmanı için tek stil katmanı, değilse sınıf başına katman ve süzgeç), kurallar MapLibre'nin ifade süzgeciyle (iç içe kurallar üst kuralın süzgeciyle, ELSE öncekilerin değili, ölçek aralıkları yakınlaştırma düzeyi olarak); etiketler yazı (symbol) katmanı olarak, metin şablonu `concat` ile. \
                      Birimler: mm 96 dpi'de CSS pikseline çevrilir (1 mm = 96/25,4 px), piksel kalır, metre ekvatordaki ölçüyle yakınlaştırmayla büyüyen piksel olur; simgenin ve katmanın saydamlığı katılır, katman görünürlüğü yalnız görünen katmanları seçer. \
                      Daire dışındaki işaret şekilleri, tarama, degrade, desen ve resim dolgusu, işaretli çizgi, SVG ve yazı işareti, veriye göre değer, efektler ve etiketin çağrı çizgisi, zemini, gölgesi gibi ayarları MapLibre'de yoktur; export_losses ile söylenir. Çizimi değiştirmez ve geri alma adımı değildir."
                .into(),
            aliases: vec!["maplibreout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportMaplibre>(),
            output: schema::<crate::MaplibreExported>(),
            plan: Some(schema::<crate::MaplibreExported>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Parsel katmanı, GeoJSON'u yanında".into(),
                    input: json!({ "path": "/veri/parsel.json", "layer": "parsel", "overwrite": true }),
                    output: None,
                },
                CommandExample {
                    title: "Görünen katmanlar, kendi vektör karolarına".into(),
                    input: json!({ "path": "/veri/harita.json", "sourceUrl": "https://karo.example.org/{z}/{x}/{y}.pbf" }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::FILE_IMPORT_MAPLIBRE.into(),
            version: crate::FILE_IMPORT_MAPLIBRE_VERSION,
            title: "MapLibre stilinden görünüş al".into(),
            summary: "path'teki (ya da text'teki) MapLibre GL stilinin (sürüm 8) fill, line ve circle katmanlarının görünüşünü katmanların işleyicisi yapar, hepsi tek geri alma adımı (Katman stili). Stil katmanı KentOS katmanınındır: metadata kentos:layer katmanın kimliğiyse (KentOS'un yazdığı stil), yoksa source-layer ya da id katmanın kimliği ya da adıysa, from verilmişse source-layer ya da id from ise. layer verilirse o katmanın, verilmezse stilin katmanı olduğu her katmanın. \
                      Bir alanın `match`i kategorili, `step`i aralıklı, süzgeçli katmanlar kurallı işleyici, hiçbiri yoksa tek simge olur; süzgeç (ifade ve eski biçim: ==, !=, <, in, has, all, any, !, match) KentOS'un ifadesine çevrilir; geometri türü süzgeci hangi simgenin olduğunu söyler; pikseller piksel, yakınlaştırmayla büyüyen metre genişlikleri metre olur; yakınlaştırma düzeyleri ölçek aralığı olur. \
                      Etiket (symbol) katmanları, başka katman türleri, yakınlaştırmaya göre eğriler, case ve başka ifadeler, çevrilemeyen süzgeç, desen ve kaydırma gibi özellikler alınmaz, import_losses ile söylenir. Katman aynı görünüşteyse adım yazılmaz (changed false). plan dosyayı okur, hiçbir şey yazmaz."
                .into(),
            aliases: vec!["maplibrein".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ImportMaplibre>(),
            output: schema::<crate::MaplibreImported>(),
            plan: Some(schema::<crate::MaplibreImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "KentOS'un yazdığı stil, bütün katmanlar".into(),
                input: json!({ "path": "/veri/harita.json" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_EXPORT_DXF.into(),
            version: crate::FILE_EXPORT_DXF_VERSION,
            title: "DXF dışa aktar".into(),
            summary: "Çizimi AutoCAD 2007 DXF dosyası olarak yazar: bütün nesneler ya da layers'ın katmanlarındakiler, katmanlarıyla; DXF'in tutamadıkları KentOS verisiyle. \
                      Dosya yarım kalmaz: geçici adla yazılır, diske indirilir, geri okunup denetlenir, sonra tek adımda yerine konur; eski dosya ancak yenisi tamsa gider. overwrite verilmezse var olan dosya reddedilir (file_exists). \
                      plan dosyayı yazmadan yazılacak olanı söyler (nesneler, katmanlar, baytlar, yazıcının raporu). Çizimi değiştirmez ve geri alma adımı değildir: çizimin geri alması dosyayı geri almaz. Çoklu çizginin genişlik/yay/kot kaybı varsayılan olarak polyline_geometry_loss ile reddedilir; yalnız allowPolylineLoss=true kayıp raporuyla yaklaşık dönüşümü kabul eder."
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
            id: crate::FILE_EXPORT_DWG.into(),
            version: crate::FILE_EXPORT_DWG_VERSION,
            title: "DWG dışa aktar".into(),
            summary: "Çizimi seçilen binary DWG biçiminde yazar (version: AC1014, AC1015, AC1018, AC1021, AC1024, AC1027 veya varsayılan AC1032). AutoCAD ürün yılı binary biçim yılı değildir. Sürümü sabit OpenCADCodec ile bellekte dönüştürür; dosya dışarı gönderilmez: bütün nesneler, uids ile seçilenler ya da layers'ın katmanlarındakiler, katmanlarıyla; DWG'in tutamadıkları KentOS verisiyle. \
                      Dosya yarım kalmaz: geçici adla yazılır, diske indirilir, geri okunup denetlenir, sonra tek adımda yerine konur; eski dosya ancak yenisi tamsa gider. overwrite verilmezse var olan dosya reddedilir (file_exists). \
                      plan dosyayı yazmadan yazılacak olanı söyler (nesneler, katmanlar, baytlar, yazıcının raporu). Çizimi değiştirmez ve geri alma adımı değildir: çizimin geri alması dosyayı geri almaz. Çoklu çizginin genişlik/yay/kot kaybı varsayılan olarak polyline_geometry_loss ile reddedilir; yalnız allowPolylineLoss=true kayıp raporuyla yaklaşık dönüşümü kabul eder."
                .into(),
            aliases: vec!["dwg".into(), "dwgout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportDwg>(),
            output: schema::<crate::DwgExported>(),
            plan: Some(schema::<crate::DwgExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bütün çizim".into(),
                input: json!({ "path": "/tmp/kat-plani.dwg", "overwrite": true, "version": "AC1032" }),
                output: None,
            }],
        },
        // WBLOCK (docs/adr/0316 §3).
        CommandDescriptor {
            id: crate::FILE_EXPORT_BLOCK.into(),
            version: crate::FILE_EXPORT_BLOCK_VERSION,
            title: "Bloğu dosyaya yaz".into(),
            summary: "Bir bloğu (block) ya da kalıcı kimlikleriyle verilen nesneleri (uids) yeni bir .kcad çizimi olarak yazar (AutoCAD'in WBLOCK'u): kullandıkları katmanlar, iç içe bloklar, stiller, kitaplık öğeleri ve proje ayarlarıyla. \
                      base çizimin başlangıcına (0, 0) gelir; blokta verilmezse bloğun taban noktasıdır, nesnelerde verilmezse nesneler yerinde kalır. Bloğun kendi katmanındaki nesneler 0 katmanına yazılır. \
                      Dosya yarım kalmaz: geçici adla yazılır, geri okunup denetlenir, tek adımda yerine konur; overwrite verilmezse var olan dosya reddedilir (file_exists). \
                      plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir. Dosyadan blok almak file.import.block'tur."
                .into(),
            aliases: vec!["wblock".into(), "blokyaz".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportBlock>(),
            output: schema::<crate::BlockFileExported>(),
            plan: Some(schema::<crate::BlockFileExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kapı bloğu kendi dosyasına".into(),
                input: json!({ "path": "/tmp/kapi-90.kcad", "block": "01925f3e-7c1c-7b22-8d33-4c5e6f708192" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_EXPORT_PDF.into(),
            version: crate::FILE_EXPORT_PDF_VERSION,
            title: "PDF olarak yazdır".into(),
            summary: "Projenin pafta kitabını PDF olarak yazar: sheets'teki paftalar (yoksa hepsi, kitabın sırasıyla), her biri bir sayfa; projenin sistemi transverse Mercator ise GeoPDF. \
                      Vektör haritaların yazıları aranabilir kalır; zengin stiller seçili dpi ile resme çevrilir, notes nedeni ve çözünürlük sınırını söyler. Pafta çekirdeğinin bulguları findings içindedir. Dosya DXF'teki gibi yarım kalmadan yazılır; overwrite verilmezse var olan dosya reddedilir. \
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
        CommandDescriptor {
            id:crate::FILE_PREFLIGHT_SHEETS.into(),version:crate::FILE_PREFLIGHT_SHEETS_VERSION,
            title:"Baskı öncesi tam denetim".into(),summary:"Dosya yazmadan pafta yerleşimini, fontları, kaynakları, açıklama ilişkilerini, görünüm kilidini ve nihai fiziksel resim çözünürlüğünü denetler. Yerel kaynaklar gerçek PDF hattında okunur; desteklenmeyen görünür içerik açık hata olarak bildirilir. Plan/execute aynı salt okuma raporudur; iptal edilebilir.".into(),aliases:vec!["PAFTADENETLE".into()],effect:CommandEffect::Query,hosts:vec![CommandHost::Desktop],headless:true,requires:vec![CommandRequirement::Document],permissions:vec![],undo:CommandUndo::None,cost:CommandCost::Job,input:schema::<crate::PreflightSheets>(),output:schema::<crate::SheetsPreflighted>(),plan:Some(schema::<crate::SheetsPreflighted>()),capabilities:None,examples:vec![CommandExample{title:"Kitabın tamamını denetle".into(),input:json!({}),output:None}],
        },
        CommandDescriptor {
            id:crate::FILE_PUBLISH_SHEETS.into(),version:crate::FILE_PUBLISH_SHEETS_VERSION,
            title:"Paftaları toplu yayınla".into(),
            summary:"Seçilen paftaları açık sırasıyla tek veya ayrı PDF olarak yeni/boş bir teslim klasörüne yayınlar. Ayrı dosyalarda {index}, {sheet}, {id} ad şablonu; çakışan adlara sabit sayısal ek. Her pafta kendi dpi ayarını korur; açık dpi bütün yayın için geçerlidir. plan dosya yazmadan çıktı adlarını, kaynak pinini ve pafta bazlı kaynak/ön denetim raporunu verir; hatalı set yayınlanmaz. Bütün klasör tek seferde görünür olur; iptalde kısmi teslim bırakılmaz. İş ilerlemesi sayfa sınırlarında güncellenir. yayin.json çıktı özetlerini ve kaynak sürümünü taşır; çizimin geri alması teslim dosyalarını kaldırmaz.".into(),
            aliases:vec!["PAFTAYAYINLA".into()],effect:CommandEffect::Query,
            hosts:vec![CommandHost::Desktop],headless:true,requires:vec![CommandRequirement::Document],permissions:vec![],undo:CommandUndo::None,cost:CommandCost::Job,
            input:schema::<crate::PublishSheets>(),output:schema::<crate::SheetsPublished>(),plan:Some(schema::<crate::SheetsPublished>()),capabilities:None,
            examples:vec![CommandExample{title:"Paftaların tamamı, ayrı PDF dosyaları".into(),input:json!({"path":"/tmp/pafta-teslimi","mode":"separate","naming":"{index}-{sheet}"}),output:None}],
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
                    crate::CAD_ARRAYS_EDIT => {
                        serde_json::from_value::<crate::ArraysEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_CONSTRAINTS_EDIT => {
                        serde_json::from_value::<crate::ConstraintsEdit>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_CONSTRAINTS_QUERY => {
                        serde_json::from_value::<crate::ConstraintsQuery>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_PARAMETERS_EDIT => {
                        serde_json::from_value::<crate::ParametersEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_EXPLODE => {
                        serde_json::from_value::<crate::EntitiesExplode>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_CURVES_EDIT => {
                        serde_json::from_value::<crate::CurvesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_MATCH => {
                        serde_json::from_value::<crate::EntitiesMatch>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_CLEANUP => {
                        serde_json::from_value::<crate::EntitiesCleanup>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_DRAWING_AUDIT => {
                        serde_json::from_value::<crate::DrawingAudit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DRAWING_COMPARE => {
                        serde_json::from_value::<crate::DrawingCompare>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_COPY => {
                        serde_json::from_value::<crate::EntitiesCopy>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_PASTE => {
                        serde_json::from_value::<crate::EntitiesPaste>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_NURBS_CREATE => {
                        serde_json::from_value::<crate::NurbsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_WALLS_CREATE => {
                        serde_json::from_value::<crate::WallsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_WALLS_RECTANGLE => {
                        serde_json::from_value::<crate::WallsRectangle>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_WALLS_FROM_LINES => {
                        serde_json::from_value::<crate::WallsFromLines>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_WALLS_EDIT => {
                        serde_json::from_value::<crate::WallsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_WALLS_JOINS => {
                        serde_json::from_value::<crate::WallsJoins>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_WALLS_DISTANCES => {
                        serde_json::from_value::<crate::WallsDistancesInput>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_COLUMNS_DISTANCES => {
                        serde_json::from_value::<crate::ColumnsDistancesInput>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DIMENSIONS_INTERIOR => {
                        serde_json::from_value::<crate::DimensionsInterior>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_OPENINGS_CREATE => {
                        serde_json::from_value::<crate::OpeningsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SPACES_CREATE => {
                        serde_json::from_value::<crate::SpacesCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SLABS_CREATE => {
                        serde_json::from_value::<crate::SlabsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SLABS_EDIT => {
                        serde_json::from_value::<crate::SlabsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SPACES_DISCOVER => {
                        serde_json::from_value::<crate::SpacesDiscover>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SPACES_NUMBER => {
                        serde_json::from_value::<crate::SpacesNumber>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SPACES_SEPARATORS => {
                        serde_json::from_value::<crate::SpacesSeparators>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_GRIDS_CREATE => {
                        serde_json::from_value::<crate::GridsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_COLUMNS_CREATE => {
                        serde_json::from_value::<crate::ColumnsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STAIRS_CREATE => {
                        serde_json::from_value::<crate::StairsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_RAMPS_CREATE => {
                        serde_json::from_value::<crate::RampsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ARCH_EDIT => {
                        serde_json::from_value::<crate::ArchEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TAGS_CREATE => {
                        serde_json::from_value::<crate::TagsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TAGS_CHECK => {
                        serde_json::from_value::<crate::TagsCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TAGS_EDIT => {
                        serde_json::from_value::<crate::TagsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_VIEWMARKS_CREATE => {
                        serde_json::from_value::<crate::ViewMarksCreate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_VIEWMARKS_EDIT => {
                        serde_json::from_value::<crate::ViewMarksEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_VIEWMARKS_REFERENCES => {
                        serde_json::from_value::<crate::ViewMarksReferences>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SPACES_FINISH => {
                        serde_json::from_value::<crate::SpacesFinish>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TEMPLATES_APPLY => {
                        serde_json::from_value::<crate::TemplatesApply>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROFILES_APPLY => {
                        serde_json::from_value::<crate::ProfilesApply>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROFILES_CHECK => {
                        serde_json::from_value::<crate::ProfilesCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROFILES_IMPORT => {
                        serde_json::from_value::<crate::ProfilesImport>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROFILES_LIST => {
                        serde_json::from_value::<crate::ProfilesList>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROFILES_MIGRATE => {
                        serde_json::from_value::<crate::ProfilesMigrate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_PROFILES_REMOVE => {
                        serde_json::from_value::<crate::ProfilesRemove>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SHEETS_SET => {
                        serde_json::from_value::<crate::SheetsSet>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_NAVIGATOR_READ => {
                        serde_json::from_value::<crate::NavigatorRead>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_XREFS_EDIT => {
                        serde_json::from_value::<crate::XrefsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_XREFS_LIST => {
                        serde_json::from_value::<crate::XrefsList>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_IMAGES_EDIT => {
                        serde_json::from_value::<crate::ImagesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_RASTERS_GEOREF => {
                        serde_json::from_value::<crate::RastersGeoref>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_RASTERS_MOSAIC => {
                        serde_json::from_value::<crate::RastersMosaic>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_RASTERS_REPROJECT => {
                        serde_json::from_value::<crate::RastersReproject>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_RASTERS_GPKGREAD => {
                        serde_json::from_value::<crate::RastersGpkgRead>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_RASTERS_GPKGWRITE => {
                        serde_json::from_value::<crate::RastersGpkgWrite>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_INSERTS_CLIP => {
                        serde_json::from_value::<crate::InsertsClip>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ASSETS_CHECK => {
                        serde_json::from_value::<crate::AssetsCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STAC_SEARCH => {
                        serde_json::from_value::<crate::StacSearch>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STAC_PREPARE => {
                        serde_json::from_value::<crate::StacPrepare>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TIME_QUERY => {
                        serde_json::from_value::<crate::TimeQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SENSORS_INGEST => {
                        serde_json::from_value::<crate::SensorsIngest>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SENSORS_LATEST => {
                        serde_json::from_value::<crate::SensorsLatest>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_POINTCLOUDS_DESCRIBE => {
                        serde_json::from_value::<crate::PointCloudsDescribe>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_POINTCLOUDS_QUERY => {
                        serde_json::from_value::<crate::PointCloudsQuery>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_POINTCLOUDS_REGION => {
                        serde_json::from_value::<crate::PointCloudsRegion>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_POINTCLOUDS_PROFILE => {
                        serde_json::from_value::<crate::PointCloudsProfile>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_MESHES_DESCRIBE => {
                        serde_json::from_value::<crate::MeshesDescribe>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_MESHES_QUERY => {
                        serde_json::from_value::<crate::MeshesQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_MESHES_REGION => {
                        serde_json::from_value::<crate::MeshesRegion>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_MESHES_PROFILE => {
                        serde_json::from_value::<crate::MeshesProfile>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SCHEDULES_CREATE => {
                        serde_json::from_value::<crate::SchedulesCreate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SCHEDULES_EDIT => {
                        serde_json::from_value::<crate::SchedulesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SPACES_EDIT => {
                        serde_json::from_value::<crate::SpacesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SPACES_QUANTITIES => {
                        serde_json::from_value::<crate::SpacesQuantities>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_OPENINGS_EDIT => {
                        serde_json::from_value::<crate::OpeningsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_OPENINGS_DISTANCES => {
                        serde_json::from_value::<crate::OpeningsDistancesInput>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_OPENING_TYPES_EDIT => {
                        serde_json::from_value::<crate::OpeningTypesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_MODULES_EDIT => {
                        serde_json::from_value::<crate::ModulesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_MATERIALS_EDIT => {
                        serde_json::from_value::<crate::MaterialsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_MODULES_QUERY => {
                        serde_json::from_value::<crate::ModulesQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_OPTIONS_EDIT => {
                        serde_json::from_value::<crate::OptionsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_OPTIONS_QUERY => {
                        serde_json::from_value::<crate::OptionsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ELEVATIONS_EDIT => {
                        serde_json::from_value::<crate::ElevationsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ELEVATIONS_QUERY => {
                        serde_json::from_value::<crate::ElevationsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_OPENINGS_QUERY => {
                        serde_json::from_value::<crate::OpeningsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ARCH_AUDIT => {
                        serde_json::from_value::<crate::ArchAudit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ARCH_EXCEPTIONS => {
                        serde_json::from_value::<crate::ArchExceptions>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROGRAM_EDIT => {
                        serde_json::from_value::<crate::ProgramEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROGRAM_CHECK => {
                        serde_json::from_value::<crate::ProgramCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LEVELS_ASSIGN => {
                        serde_json::from_value::<crate::LevelsAssign>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_NORTH_CREATE => {
                        serde_json::from_value::<crate::NorthCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_REVISIONS_MARK => {
                        serde_json::from_value::<crate::RevisionsMark>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LEGENDS_MATERIALS => {
                        serde_json::from_value::<crate::LegendsMaterials>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_CASEWORK_RUN => {
                        serde_json::from_value::<crate::CaseworkRun>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_CASEWORK_EDIT => {
                        serde_json::from_value::<crate::CaseworkEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_CASEWORK_QUERY => {
                        serde_json::from_value::<crate::CaseworkQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_ORDER => {
                        serde_json::from_value::<crate::EntitiesOrder>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ANNOTATIONS_SCALES => {
                        serde_json::from_value::<crate::AnnotationsScales>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ENTITIES_OFFSET => {
                        serde_json::from_value::<crate::EntitiesOffset>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DIMENSIONS_REASSOCIATE => {
                        serde_json::from_value::<crate::DimensionsReassociate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_DIMENSION_SETS_CREATE => {
                        serde_json::from_value::<crate::DimensionSetsCreateInput>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_DIMENSION_SETS_EDIT => {
                        serde_json::from_value::<crate::DimensionSetsEditInput>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_DIMENSION_SETS_QUERY => {
                        serde_json::from_value::<crate::DimensionSetsQueryInput>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_DIMENSIONS_CREATE => {
                        serde_json::from_value::<crate::DimensionsCreate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_DIMENSIONS_EDIT => {
                        serde_json::from_value::<crate::DimensionsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DIMENSIONS_QUERY => {
                        serde_json::from_value::<crate::DimensionsQuery>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_FIELDS_EDIT => {
                        serde_json::from_value::<crate::FieldsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TABLES_EDIT => {
                        serde_json::from_value::<crate::TablesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LEADERS_EDIT => {
                        serde_json::from_value::<crate::LeadersEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_HATCHES_EDIT => {
                        serde_json::from_value::<crate::HatchesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TEXTS_EDIT => {
                        serde_json::from_value::<crate::TextsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TEXTS_CHECK => {
                        serde_json::from_value::<crate::TextsCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FONTS_REPORT => {
                        serde_json::from_value::<crate::FontsReport>(e.input.clone()).map(|_| ())
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
                    crate::CAD_ENTITIES_REVERSE => {
                        serde_json::from_value::<crate::EntitiesReverse>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ENTITIES_LENGTHEN => {
                        serde_json::from_value::<crate::EntitiesLengthen>(e.input.clone())
                            .map(|_| ())
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
                    crate::CAD_BLOCKS_INSERT => {
                        serde_json::from_value::<crate::BlocksInsert>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_BLOCKS_REPLACE => {
                        serde_json::from_value::<crate::BlocksReplace>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_BLOCKS_QUERY => {
                        serde_json::from_value::<crate::BlocksQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_BLOCKS_PARAMETERS => {
                        serde_json::from_value::<crate::BlocksParameters>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_BLOCKS_PROXY => {
                        serde_json::from_value::<crate::BlocksProxy>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_BLOCKS_FREEZE => {
                        serde_json::from_value::<crate::BlocksFreeze>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_BLOCK => {
                        serde_json::from_value::<crate::ImportBlock>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_BLOCK => {
                        serde_json::from_value::<crate::ExportBlock>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_SERVICE => {
                        serde_json::from_value::<crate::LayersService>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_NETWORK_DEFINE => {
                        serde_json::from_value::<crate::NetworkDefine>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LINEARREF_DEFINE => {
                        serde_json::from_value::<crate::LinearRefDefine>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SUBNETWORKS_FIND => {
                        serde_json::from_value::<crate::SubnetworksFind>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SUBNETWORKS_UPDATE => {
                        serde_json::from_value::<crate::SubnetworksUpdate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ADDRESSES_DEFINE => {
                        serde_json::from_value::<crate::AddressesDefine>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ADDRESSES_CHECK => {
                        serde_json::from_value::<crate::AddressesCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ADDRESSES_COMPARE => {
                        serde_json::from_value::<crate::AddressesCompare>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_ADDRESSES_NORMALIZE => {
                        serde_json::from_value::<crate::AddressesNormalize>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_TOPOLOGY_CHECK => {
                        serde_json::from_value::<crate::TopologyCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TOPOLOGY_EXCEPTIONS => {
                        serde_json::from_value::<crate::TopologyExceptions>(e.input.clone())
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
                    crate::CAD_LAYERS_CRS => {
                        serde_json::from_value::<crate::LayersCrs>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_REPROJECT => {
                        serde_json::from_value::<crate::LayersReproject>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_LAYERS_TABLE => {
                        serde_json::from_value::<crate::LayersTable>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_SUBTYPES => {
                        serde_json::from_value::<crate::LayersSubtypes>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_CONTINGENTS => {
                        serde_json::from_value::<crate::LayersContingents>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_LAYERS_RULES => {
                        serde_json::from_value::<crate::LayersRules>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_INDEXES => {
                        serde_json::from_value::<crate::LayersIndexes>(e.input.clone()).map(|_| ())
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
                    crate::CAD_LABELS_REPORT => {
                        serde_json::from_value::<crate::LabelsReport>(e.input.clone()).map(|_| ())
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
                    crate::CAD_LAYERS_FIND => {
                        serde_json::from_value::<crate::LayersFind>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FIELDS_USAGE => {
                        serde_json::from_value::<crate::FieldsUsage>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LAYERS_TRANSLATE => {
                        serde_json::from_value::<crate::LayersTranslate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_STANDARDS_CHECK => {
                        serde_json::from_value::<crate::StandardsCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_LINE_TYPES_EDIT => {
                        serde_json::from_value::<crate::LineTypesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_HATCH_PATTERNS_EDIT => {
                        serde_json::from_value::<crate::HatchPatternsEdit>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_LIBRARY_EDIT => {
                        serde_json::from_value::<crate::LibraryEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STYLES_IMPACT => {
                        serde_json::from_value::<crate::StylesImpact>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STANDARDS_APPLY => {
                        serde_json::from_value::<crate::StandardsApply>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STYLES_EDIT => {
                        serde_json::from_value::<crate::StylesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SETTINGS_SET => {
                        serde_json::from_value::<crate::SettingsSet>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_UCS_SET => {
                        serde_json::from_value::<crate::UcsSet>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_VIEWS_EDIT => {
                        serde_json::from_value::<crate::ViewsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_EDIT => {
                        serde_json::from_value::<crate::FormsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_VALIDATE => {
                        serde_json::from_value::<crate::FormsValidate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_SUBMIT => {
                        serde_json::from_value::<crate::FormsSubmit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_FILL => {
                        serde_json::from_value::<crate::FormsFill>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_JOINS_EDIT => {
                        serde_json::from_value::<crate::JoinsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_JOINS_QUERY => {
                        serde_json::from_value::<crate::JoinsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DATASETS_EDIT => {
                        serde_json::from_value::<crate::DatasetsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DATASETS_DESCRIBE => {
                        serde_json::from_value::<crate::DatasetsDescribe>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_RELATIONS_EDIT => {
                        serde_json::from_value::<crate::RelationsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_AUDIT => {
                        serde_json::from_value::<crate::FormsAudit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_TEST => {
                        serde_json::from_value::<crate::FormsTest>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DOMAINS_EDIT => {
                        serde_json::from_value::<crate::DomainsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DOMAINS_EXPORT => {
                        serde_json::from_value::<crate::DomainsExport>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FIELDS_MIGRATE => {
                        serde_json::from_value::<crate::FieldsMigrate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ATTACHMENTS_INSPECT => {
                        serde_json::from_value::<crate::AttachmentsInspect>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SCHEMA_DESCRIBE => {
                        serde_json::from_value::<crate::SchemaDescribe>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_EXPORT => {
                        serde_json::from_value::<crate::FormsExport>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_IMPORT => {
                        serde_json::from_value::<crate::FormsImport>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_TRANSITION => {
                        serde_json::from_value::<crate::FormsTransition>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_FORMS_COMPARE => {
                        serde_json::from_value::<crate::FormsCompare>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_SOURCES => {
                        serde_json::from_value::<crate::FormsSources>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FORMS_RECOMPUTE => {
                        serde_json::from_value::<crate::FormsRecompute>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_RECORDS_CREATE => {
                        serde_json::from_value::<crate::RecordsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_RECORDS_DUPLICATE => {
                        serde_json::from_value::<crate::RecordsDuplicate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SHEETS_LEVELS => {
                        serde_json::from_value::<crate::SheetsLevels>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SHEETS_CREATE => {
                        serde_json::from_value::<crate::SheetsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SHEETS_FIT => {
                        serde_json::from_value::<crate::SheetsFit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SHEETS_EDIT => {
                        serde_json::from_value::<crate::SheetsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_UNITS_CHANGE => {
                        serde_json::from_value::<crate::UnitsChange>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_DXF | crate::FILE_IMPORT_DWG => {
                        serde_json::from_value::<crate::ImportDxf>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_PDF => {
                        serde_json::from_value::<crate::ImportPdf>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_GPKG => {
                        serde_json::from_value::<crate::ImportGpkg>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_GIS_DERIVATION => {
                        serde_json::from_value::<crate::GisDerivation>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_GIS_DETACH => {
                        serde_json::from_value::<crate::GisDetach>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_GIS_TAKEBACK => {
                        serde_json::from_value::<crate::GisTakeback>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_GIS => {
                        serde_json::from_value::<crate::ExportGis>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SOURCES_DESCRIBE => {
                        serde_json::from_value::<crate::SourcesDescribe>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SOURCES_EDIT => {
                        serde_json::from_value::<crate::SourcesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SOURCES_SYNC => {
                        serde_json::from_value::<crate::SourcesSync>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SOURCES_RELATE => {
                        serde_json::from_value::<crate::SourcesRelate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SOURCES_RECONCILE => {
                        serde_json::from_value::<crate::SourcesReconcile>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_RECORDS_PAGE => {
                        serde_json::from_value::<crate::RecordsPage>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SOURCES_QUERY => {
                        serde_json::from_value::<crate::SourcesQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_FIELDS_WRITE => {
                        serde_json::from_value::<crate::FieldsWrite>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROCESSING_TOOLS => {
                        serde_json::from_value::<crate::ProcessingTools>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_PROCESSING_RUN => {
                        serde_json::from_value::<crate::ProcessingRun>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROCESSING_BATCH => {
                        serde_json::from_value::<crate::ProcessingBatch>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_CHANGESET_APPLY => {
                        serde_json::from_value::<crate::ChangesetApply>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_DXF => {
                        serde_json::from_value::<crate::ExportDxf>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_DWG => {
                        serde_json::from_value::<crate::ExportDwg>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_GPKG => {
                        serde_json::from_value::<crate::ExportGpkg>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_SHP => {
                        serde_json::from_value::<crate::ExportShp>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_CSV => {
                        serde_json::from_value::<crate::ExportCsv>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_ATLAS => {
                        serde_json::from_value::<crate::ExportAtlas>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_CSV => {
                        serde_json::from_value::<crate::ImportCsv>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_FGB => {
                        serde_json::from_value::<crate::ExportFgb>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_FGB => {
                        serde_json::from_value::<crate::ImportFgb>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_PARQUET => {
                        serde_json::from_value::<crate::ExportParquet>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_PARQUET => {
                        serde_json::from_value::<crate::ImportParquet>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_SHP => {
                        serde_json::from_value::<crate::ImportShp>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_GEOJSON => {
                        serde_json::from_value::<crate::ImportGeojson>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_KML => {
                        serde_json::from_value::<crate::ImportKml>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_GML => {
                        serde_json::from_value::<crate::ExportGml>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_GML => {
                        serde_json::from_value::<crate::ImportGml>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_XYZ => {
                        serde_json::from_value::<crate::ExportXyz>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_TABLE => {
                        serde_json::from_value::<crate::ExportTable>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SCHEDULES_QUERY => {
                        serde_json::from_value::<crate::SchedulesQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_SLD => {
                        serde_json::from_value::<crate::ExportSld>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_SLD => {
                        serde_json::from_value::<crate::ImportSld>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_QML => {
                        serde_json::from_value::<crate::ExportQml>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_QML => {
                        serde_json::from_value::<crate::ImportQml>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_MAPLIBRE => {
                        serde_json::from_value::<crate::ExportMaplibre>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_MAPLIBRE => {
                        serde_json::from_value::<crate::ImportMaplibre>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_PACKAGE => {
                        serde_json::from_value::<crate::PackageProject>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_PDF => {
                        serde_json::from_value::<crate::ExportPdf>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_PREFLIGHT_SHEETS => {
                        serde_json::from_value::<crate::PreflightSheets>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::FILE_PUBLISH_SHEETS => {
                        serde_json::from_value::<crate::PublishSheets>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SNAP_FIND => {
                        serde_json::from_value::<crate::SnapFind>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DONUT_CREATE => {
                        serde_json::from_value::<crate::DonutCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_RECTANGLE_CREATE => {
                        serde_json::from_value::<crate::RectangleCreate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_POLYGON_REGULAR => {
                        serde_json::from_value::<crate::RegularPolygonCreate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SECTOR_CREATE => {
                        serde_json::from_value::<crate::SectorCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SHAPES_EDIT => {
                        serde_json::from_value::<crate::ShapesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_OBJECTS_DERIVE => {
                        serde_json::from_value::<crate::ObjectsDerive>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ROADS_EDIT => {
                        serde_json::from_value::<crate::RoadsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_IMAGES_CLIP => {
                        serde_json::from_value::<crate::ImagesClip>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_TIME_VERSION => {
                        serde_json::from_value::<crate::TimeVersion>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SURVEY_POINTS => {
                        serde_json::from_value::<crate::SurveyPoints>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_SURVEY_ADJUST => {
                        serde_json::from_value::<crate::SurveyAdjust>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ANNOTATIONS_WRITE => {
                        serde_json::from_value::<crate::AnnotationsWrite>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_AREAS_EDIT => {
                        serde_json::from_value::<crate::AreasEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_CONSTRUCTIONS_CREATE => {
                        serde_json::from_value::<crate::ConstructionsCreate>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_PARCELS_CREATE => {
                        serde_json::from_value::<crate::ParcelsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_ADJOIN => {
                        serde_json::from_value::<crate::EntitiesAdjoin>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_REVCLOUD_CREATE => {
                        serde_json::from_value::<crate::RevcloudCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_VERTEX => {
                        serde_json::from_value::<crate::EntitiesVertex>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_PICK => {
                        serde_json::from_value::<crate::EntitiesPick>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_CRS_TRANSFORM => {
                        serde_json::from_value::<crate::CrsTransform>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_RELATE => {
                        serde_json::from_value::<crate::EntitiesRelate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ENTITIES_SELECT => {
                        serde_json::from_value::<crate::EntitiesSelect>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_POINTS_CONSTRUCT => {
                        serde_json::from_value::<crate::PointsConstruct>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_CURVES_CONSTRUCT => {
                        serde_json::from_value::<crate::CurvesConstruct>(e.input.clone())
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
                        crate::CAD_ENTITIES_EDIT => {
                            serde_json::from_value::<crate::EntitiesEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_ARRAY => {
                            serde_json::from_value::<crate::EntitiesArrayed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_NURBS_CREATE => {
                            serde_json::from_value::<crate::NurbsCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_ORDER => {
                            serde_json::from_value::<crate::EntitiesOrdered>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FIELDS_EDIT => {
                            serde_json::from_value::<crate::FieldsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LEADERS_EDIT => {
                            serde_json::from_value::<crate::LeadersEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_HATCHES_EDIT => {
                            serde_json::from_value::<crate::HatchesEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_TABLES_EDIT => {
                            serde_json::from_value::<crate::TablesEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_TEXTS_EDIT => {
                            serde_json::from_value::<crate::TextsEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_TEXTS_CHECK => {
                            serde_json::from_value::<crate::TextsChecked>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FONTS_REPORT => {
                            serde_json::from_value::<crate::FontsReported>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ANNOTATIONS_SCALES => {
                            serde_json::from_value::<crate::AnnotationsScaled>(output.clone())
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
                        crate::CAD_DIMENSIONS_EDIT => {
                            serde_json::from_value::<crate::DimensionsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_DIMENSIONS_QUERY => {
                            serde_json::from_value::<crate::DimensionsRead>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_CONSTRAINTS_EDIT => {
                            serde_json::from_value::<crate::ConstraintsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_CONSTRAINTS_QUERY => {
                            serde_json::from_value::<crate::ConstraintsListed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_PARAMETERS_EDIT => {
                            serde_json::from_value::<crate::ParametersEdited>(output.clone())
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
                        crate::CAD_ENTITIES_BREAK
                        | crate::CAD_ARRAYS_EDIT
                        | crate::CAD_ENTITIES_EXPLODE
                        | crate::CAD_CURVES_EDIT
                        | crate::CAD_ENTITIES_MATCH
                        | crate::CAD_ENTITIES_JOIN
                        | crate::CAD_ENTITIES_REVERSE
                        | crate::CAD_ENTITIES_LENGTHEN => {
                            serde_json::from_value::<crate::EntitiesEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_CLEANUP => {
                            serde_json::from_value::<crate::EntitiesCleaned>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_DRAWING_AUDIT => {
                            serde_json::from_value::<crate::DrawingAudited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_DRAWING_COMPARE => {
                            serde_json::from_value::<crate::DrawingCompared>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ENTITIES_COPY => {
                            serde_json::from_value::<crate::Clip>(output.clone()).map(|_| ())
                        }
                        crate::CAD_ENTITIES_PASTE => {
                            serde_json::from_value::<crate::EntitiesPasted>(output.clone())
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
                        crate::CAD_LINEARREF_DEFINE => {
                            serde_json::from_value::<crate::LinearRefDefined>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ADDRESSES_DEFINE => {
                            serde_json::from_value::<crate::AddressesDefined>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ADDRESSES_CHECK => {
                            serde_json::from_value::<crate::AddressesChecked>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_TOPOLOGY_CHECK => {
                            serde_json::from_value::<crate::TopologyChecked>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_TOPOLOGY_EXCEPTIONS => serde_json::from_value::<
                            crate::TopologyExceptionsEdited,
                        >(output.clone())
                        .map(|_| ()),
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
                        crate::CAD_LAYERS_CRS => {
                            serde_json::from_value::<crate::LayersCrsSet>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_REPROJECT => {
                            serde_json::from_value::<crate::LayersReprojected>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_COMPOSE => {
                            serde_json::from_value::<crate::LayersComposed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_TABLE => {
                            serde_json::from_value::<crate::LayersComposed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_SUBTYPES => {
                            serde_json::from_value::<crate::LayersComposed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LAYERS_CONTINGENTS => {
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
                        crate::CAD_LAYERS_FIND => {
                            serde_json::from_value::<crate::LayersFound>(output.clone()).map(|_| ())
                        }
                        crate::CAD_FIELDS_USAGE => {
                            serde_json::from_value::<crate::FieldsUsed>(output.clone()).map(|_| ())
                        }
                        crate::CAD_LAYERS_TRANSLATE => {
                            serde_json::from_value::<crate::LayersTranslated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_STANDARDS_CHECK => {
                            serde_json::from_value::<crate::StandardsReport>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LINE_TYPES_EDIT => {
                            serde_json::from_value::<crate::LineTypesEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_HATCH_PATTERNS_EDIT => {
                            serde_json::from_value::<crate::HatchPatternsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_LIBRARY_EDIT => {
                            serde_json::from_value::<crate::LibraryEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_STYLES_IMPACT => {
                            serde_json::from_value::<crate::StyleImpact>(output.clone()).map(|_| ())
                        }
                        crate::CAD_BLOCKS_INSERT => {
                            serde_json::from_value::<crate::BlockInserted>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_BLOCKS_REPLACE => {
                            serde_json::from_value::<crate::BlocksReplaced>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_BLOCKS_QUERY => {
                            serde_json::from_value::<crate::BlocksQueried>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_BLOCKS_PARAMETERS => {
                            serde_json::from_value::<crate::BlocksParametersOut>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_BLOCKS_PROXY => {
                            serde_json::from_value::<crate::BlocksProxied>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_BLOCKS_FREEZE => {
                            serde_json::from_value::<crate::BlocksFrozen>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_STANDARDS_APPLY => {
                            serde_json::from_value::<crate::StandardsApplied>(output.clone())
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
                        crate::CAD_UCS_SET => {
                            serde_json::from_value::<crate::UcsChanged>(output.clone()).map(|_| ())
                        }
                        crate::CAD_VIEWS_EDIT => {
                            serde_json::from_value::<crate::ViewsEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_FORMS_EDIT => {
                            serde_json::from_value::<crate::FormsEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_FORMS_VALIDATE => {
                            serde_json::from_value::<crate::FormsValidated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_SUBMIT => {
                            serde_json::from_value::<crate::FormsSubmitted>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_FILL => {
                            serde_json::from_value::<crate::FormsFilled>(output.clone()).map(|_| ())
                        }
                        crate::CAD_JOINS_EDIT => {
                            serde_json::from_value::<crate::JoinsEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_JOINS_QUERY => {
                            serde_json::from_value::<crate::JoinsAnswer>(output.clone()).map(|_| ())
                        }
                        crate::CAD_DATASETS_EDIT => {
                            serde_json::from_value::<crate::DatasetsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_DATASETS_DESCRIBE => {
                            serde_json::from_value::<crate::DatasetsDescribed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_RELATIONS_EDIT => {
                            serde_json::from_value::<crate::RelationsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_AUDIT => {
                            serde_json::from_value::<crate::FormsAudited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_TEST => {
                            serde_json::from_value::<crate::FormsTested>(output.clone()).map(|_| ())
                        }
                        crate::CAD_DOMAINS_EDIT => {
                            serde_json::from_value::<crate::DomainsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_DOMAINS_EXPORT => {
                            serde_json::from_value::<crate::DomainsPackage>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FIELDS_MIGRATE => {
                            serde_json::from_value::<crate::FieldsMigrated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_ATTACHMENTS_INSPECT => {
                            serde_json::from_value::<crate::AttachmentsInspected>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_SCHEMA_DESCRIBE => {
                            serde_json::from_value::<crate::SchemaDescribed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_EXPORT => {
                            serde_json::from_value::<crate::FormPackage>(output.clone()).map(|_| ())
                        }
                        crate::CAD_FORMS_IMPORT => {
                            serde_json::from_value::<crate::FormsImported>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_TRANSITION => {
                            serde_json::from_value::<crate::FormsTransitioned>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_COMPARE => {
                            serde_json::from_value::<crate::FormsCompared>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_SOURCES => {
                            serde_json::from_value::<crate::FormsSourced>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_FORMS_RECOMPUTE => {
                            serde_json::from_value::<crate::FormsRecomputed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_RECORDS_CREATE => {
                            serde_json::from_value::<crate::RecordsCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_RECORDS_DUPLICATE => {
                            serde_json::from_value::<crate::RecordsDuplicated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_SHEETS_EDIT
                        | crate::CAD_SHEETS_CREATE
                        | crate::CAD_SHEETS_FIT
                        | crate::CAD_SHEETS_LEVELS => {
                            serde_json::from_value::<crate::SheetsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_UNITS_CHANGE => {
                            serde_json::from_value::<crate::UnitsChanged>(output.clone())
                                .map(|_| ())
                        }
                        crate::FILE_IMPORT_DXF | crate::FILE_IMPORT_DWG => {
                            serde_json::from_value::<crate::DxfImported>(output.clone()).map(|_| ())
                        }
                        crate::CAD_WALLS_CREATE | crate::CAD_WALLS_RECTANGLE => {
                            serde_json::from_value::<crate::WallsCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_WALLS_FROM_LINES => {
                            serde_json::from_value::<crate::WallsFromLinesDone>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_WALLS_EDIT => {
                            serde_json::from_value::<crate::WallsEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_WALLS_JOINS => {
                            serde_json::from_value::<crate::WallsJoined>(output.clone()).map(|_| ())
                        }
                        crate::CAD_WALLS_DISTANCES => {
                            serde_json::from_value::<crate::WallsDistances>(output.clone()).map(|_| ())
                        }
                        crate::CAD_COLUMNS_DISTANCES => {
                            serde_json::from_value::<crate::ColumnsDistances>(output.clone()).map(|_| ())
                        }
                        crate::CAD_DIMENSIONS_INTERIOR => {
                            serde_json::from_value::<crate::EntitiesCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_OPENINGS_CREATE => {
                            serde_json::from_value::<crate::OpeningsCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_SPACES_CREATE | crate::CAD_SPACES_DISCOVER => {
                            serde_json::from_value::<crate::SpacesCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_SLABS_CREATE => {
                            serde_json::from_value::<crate::SlabsCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_SPACES_SEPARATORS => {
                            serde_json::from_value::<crate::SpaceSeparatorsChanged>(output.clone()).map(|_| ())
                        }
                        crate::CAD_GRIDS_CREATE => {
                            serde_json::from_value::<crate::GridsCreated>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_OPENINGS_EDIT => {
                            serde_json::from_value::<crate::OpeningsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_OPENINGS_DISTANCES => {
                            serde_json::from_value::<crate::OpeningsDistances>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_OPENING_TYPES_EDIT => {
                            serde_json::from_value::<crate::OpeningTypesEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_MODULES_EDIT => {
                            serde_json::from_value::<crate::ModulesEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_MODULES_QUERY => {
                            serde_json::from_value::<crate::ModulesQueried>(output.clone()).map(|_| ())
                        }
                        crate::CAD_OPTIONS_EDIT => {
                            serde_json::from_value::<crate::OptionsEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_OPTIONS_QUERY => {
                            serde_json::from_value::<crate::OptionsQueried>(output.clone()).map(|_| ())
                        }
                        crate::CAD_ELEVATIONS_EDIT => {
                            serde_json::from_value::<crate::ElevationsEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_ELEVATIONS_QUERY => {
                            serde_json::from_value::<crate::ElevationsQueried>(output.clone()).map(|_| ())
                        }
                        crate::CAD_OPENINGS_QUERY => {
                            serde_json::from_value::<crate::OpeningsQueried>(output.clone()).map(|_| ())
                        }
                        crate::CAD_ARCH_AUDIT => {
                            serde_json::from_value::<crate::ArchAudited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_ARCH_EXCEPTIONS => {
                            serde_json::from_value::<crate::ArchExceptionsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_PROGRAM_EDIT => {
                            serde_json::from_value::<crate::ProgramEdited>(output.clone()).map(|_| ())
                        }
                        crate::CAD_PROGRAM_CHECK => {
                            serde_json::from_value::<crate::ProgramChecked>(output.clone()).map(|_| ())
                        }
                        crate::CAD_LEVELS_ASSIGN => {
                            serde_json::from_value::<crate::LevelsAssigned>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_NORTH_CREATE => {
                            serde_json::from_value::<crate::NorthCreated>(output.clone()).map(|_| ())
                        }
                        crate::CAD_REVISIONS_MARK => {
                            serde_json::from_value::<crate::RevisionMarked>(output.clone()).map(|_| ())
                        }
                        crate::CAD_LEGENDS_MATERIALS => {
                            serde_json::from_value::<crate::LegendMade>(output.clone()).map(|_| ())
                        }
                        crate::CAD_CASEWORK_RUN => {
                            serde_json::from_value::<crate::CaseworkBuilt>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_CASEWORK_QUERY => {
                            serde_json::from_value::<crate::CaseworkRuns>(output.clone())
                                .map(|_| ())
                        }
                        other => panic!("{other}: add its output type to this test"),
                    };
                    parsed.unwrap_or_else(|err| panic!("{}: {}: output: {err}", d.id, e.title));
                }
            }
        }
    }

    /// The file's form (docs/adr/0731): the schemas' definitions once, in the catalog's `$defs`; each schema read
    /// back whole is the one the catalog builds, so nothing is lost on the way.
    #[test]
    fn the_shared_catalog_gives_back_every_schema() {
        let c = catalog();
        let shared = c.shared().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(shared.version, CATALOG_SHARED_VERSION);
        assert!(!shared.defs.is_empty());
        for (whole, part) in c.commands.iter().zip(&shared.commands) {
            for (a, b) in [
                (Some(&whole.input), Some(&part.input)),
                (Some(&whole.output), Some(&part.output)),
                (whole.plan.as_ref(), part.plan.as_ref()),
            ] {
                let (Some(a), Some(b)) = (a, b) else {
                    assert_eq!(a.is_some(), b.is_some(), "{}", whole.id);
                    continue;
                };
                assert!(b.get("$defs").is_none(), "{}: a schema keeps its $defs", whole.id);
                assert_eq!(
                    &shared.standalone(b).unwrap_or_else(|e| panic!("{}: {e}", whole.id)),
                    a,
                    "{}: the schema read back from the shared catalog",
                    whole.id
                );
            }
        }
        assert_eq!(shared.standalone_all().unwrap_or_else(|e| panic!("{e}")), c);
        // Every definition is some schema's: the file carries no orphan.
        let mut used = std::collections::BTreeSet::new();
        for d in &c.commands {
            for s in [Some(&d.input), Some(&d.output), d.plan.as_ref()].into_iter().flatten() {
                for name in s.get("$defs").and_then(serde_json::Value::as_object).into_iter().flatten() {
                    used.insert(name.0.clone());
                }
            }
        }
        assert_eq!(used, shared.defs.keys().cloned().collect());
    }

    #[test]
    fn generated_catalog_file_is_current() {
        let shared = catalog().shared().unwrap_or_else(|e| panic!("{e}"));
        let text = format!(
            "{}\n",
            serde_json::to_string_pretty(&shared).expect("the catalog serializes")
        );
        let path = catalog_file();
        if std::env::var("KENTOS_WRITE_CATALOG").as_deref() == Ok("1") {
            std::fs::write(&path, &text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            return;
        }
        let on_disk = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            on_disk == text,
            "{} is out of date: run `KENTOS_WRITE_CATALOG=1 cargo test -p kentos-commands catalog`, read the diff, commit it",
            path.display()
        );
    }
}
