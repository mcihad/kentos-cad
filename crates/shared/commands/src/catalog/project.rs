//! The catalog's commands of the projects' and the cloud's (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use crate::{CommitResult, PROJECT_ACCESS_REVOKE, PROJECT_ACCESS_REVOKE_VERSION, PROJECT_CHANGES, PROJECT_CHANGES_VERSION, PROJECT_SHARE, PROJECT_SHARE_VERSION, ProjectAccessChange, ProjectAccessRevoke, ProjectChanges, ProjectPermission, ProjectShare};
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
        CommandDescriptor {
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
    ]
}
