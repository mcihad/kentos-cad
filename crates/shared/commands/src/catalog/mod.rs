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
mod annotation;
#[cfg(feature = "schema")]
mod arch;
#[cfg(feature = "schema")]
mod arch_project;
#[cfg(feature = "schema")]
mod data;
#[cfg(feature = "schema")]
mod drawing;
#[cfg(feature = "schema")]
mod gis;
#[cfg(feature = "schema")]
mod map;
#[cfg(feature = "schema")]
mod project;
#[cfg(feature = "schema")]
mod road;
#[cfg(feature = "schema")]
mod road_field;

#[cfg(feature = "schema")]
fn schema<T: schemars::JsonSchema>() -> serde_json::Value {
    schemars::schema_for!(T).to_value()
}

/// The catalog, with schemas derived from the contract types.
#[cfg(feature = "schema")]
pub fn catalog() -> CommandCatalog {
    use serde_json::json;
    let mut commands = project::descriptors();
    commands.extend(drawing::descriptors());
    commands.extend(arch::descriptors());
    commands.extend(data::descriptors());
    commands.extend(arch_project::descriptors());
    commands.extend(annotation::descriptors());
    commands.extend(road::descriptors());
    commands.extend(map::descriptors());
    commands.extend(road_field::descriptors());
    commands.extend(gis::descriptors());
    // Recipe metadata and its source/member objects travel in the same transaction (ADR0695).
    let mut live_changes = commands[0].clone();
    live_changes.version = crate::PROJECT_CHANGES_DIMENSION_SETS_VERSION;
    live_changes.title = "Proje değişiklikleri ve ilişkili ölçüler".into();
    live_changes.summary.push_str(" v2 dimensionSets tarifini, kaynak ve üye nesneleriyle aynı transaction içinde kabul eder; boş liste tarifleri kaldırır. v1 dimensionSets alanını reddeder. Sunucu desteği /v1/health projectChangesVersions ile öğrenilir.");
    commands.push(live_changes);
    commands.push(CommandDescriptor {
        id: crate::CAD_DIMENSION_SETS_CREATE.into(), version: crate::CAD_DIMENSION_SETS_VERSION,
        title: "Duvar ölçüleri oluştur".into(), summary: "Duvarın bir düz kenarının gerçek sol/sağ yüzünü ve açıklık uçlarını veya merkezlerini kalıcı bir tariften ölçer. Birden çok kenarlı (kapalı ya da çok köşeli) duvarda edge ölçülecek düz kenarı verir (köşe edge → edge+1); around ise her düz kenara birer dizi koyar, kapalı duvarda dış yüzden, hepsi tek adımda (çıktının sets'i). Sıralar openings/overall; gapMm kağıt mm, geometri metre; belge stili kullanılır. Plan kimlik ayırmaz. Kaynak değişince üyelik güncellenir; köşe eklenip silinince kenar uçlarıyla izlenir; aynı kaynak çifti kimliğini ve elle yerleştirilen yazıyı korur. Yay kenar ve pahlanmış yüz işlemden önce reddedilir.".into(), aliases: vec![],
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
                      area m²'lik alana ulaşana dek kaydırır; alanda dışarı, çoklu çizgide yürüme yönünün sağı artı; topology ile \
                      ortak köşeleri ve kenarları paylaşan görünen nesneler aynı adımda birlikte kayar, komşu parsel \
                      edinilen alanı verir, neighbours her komşunun önceki ve sonraki boyunu söyler; kilitli komşu ayrılır \
                      ve neighbour_locked ile söylenir ya da stopOnLocked ile shared_boundary_locked reddi gelir; \
                      reference ile distance kenarın referans kenarın doğrusuna uzaklığıdır, kenar ve referans 0,001° \
                      içinde paralel olmalı; keep (Bu uzaklığı koru) uzaklığı aynı adımda kalıcı bağ olarak yazar, \
                      referans değişince kenar izler, bonds yeni bağların kimlikleri), \
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
                    title: "Parselin ön kenarını yolun kenarından 5 m'de tut (Bu uzaklığı koru)".into(),
                    input: json!({ "work": { "kind": "edgeShift", "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001",
                        "at": { "x": 423540.0, "y": 4512315.0 }, "distance": 5.0,
                        "reference": { "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002",
                            "at": { "x": 423540.0, "y": 4512308.0 }, "keep": true } } }),
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
                      uzunlukları, kapanma hataları dağıtılır; heights ile trigonometrik kotlar; heights.back ile karşılıklı \
                      başucu açıları: geri nişanlı kenarın kot farkı iki yönün ortalaması, k düşer, aynı iki uç arasındaki çiftin \
                      kırılma katsayısı refractions'ta), Kutupsal alım (polar: bakılan noktaya yöneltilmiş okumalar, \
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
                      sapmaları, serbestlik, m0 ve model testi. Tek geri alma adımı, pencerenin adıyla. \
                      Kot ağında leapfrog (atlamalı trigonometrik nivelman, yalnız method trigonometric): alet iki noktanın \
                      arasında; her kurulumun geri ve ileri nişanı (eğik uzunluk, başucu açısı, işaret yüksekliği); kot farkı \
                      ileri nişanın S·cos z + (1 − k)·D²/2R − t'sinden geri nişanınki (projenin k'sı, alet yüksekliği düşer), \
                      uzunluğu iki yatay uzunluğun toplamı; kot farklarının ardından dengelenir, çıktının leapfrog'u."
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
                      Yay kenara eklenen köşe yayı çemberinde ikiye böler; Yaya dönüştür kenarı kirişin dörtte biri sehimle halkanın dışına ya da açık yolun sağına büker, through ile verilen noktadan, radius ile verilen yarıçapla (artı: kenarın sağına) geçen yay yapar, yay kenarı da yeniden biçimler; Kenar genişliği (width) çoklu çizginin kenarına başlangıç ve bitiş genişliği verir; çizgiye köşe eklenince çoklu çizgi olur. Köşeyi sil iki kenarı düz bir kenar yapar; joinArcs (uzunluk toleransı) ile iki yanı aynı yöne dönen ve merkezleri ile yarıçapları tolerans içinde aynı olan iki yay tek yay olur (süpürmeleri toplanır), olmazsa kenar düz kalır ve arcs_not_joined uyarısı nedenini söyler; aynı yayları paylaşan komşu topology ile tek yayı alır. Alanın delikleri ve öbür parçaları kalır; topology ile ortak köşe ve kenarları olan komşular aynı adımda izler; topology.stopOnLocked ile kilitli komşu düzenlemeyi reddeder (shared_boundary_locked). Tek geri alma adımı."
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
                title: "İki yayın ortak köşesini sil, aynı dairedeyse (1 mm içinde) tek yay olsun".into(),
                input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002", "action": "remove", "joinArcs": 0.001,
                               "place": { "kind": "vertex", "part": 0, "ring": 0, "index": 1 } }),
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
                      Rapor (report) sayıları ayrı tutar: kaynağın belirsizliği (uncertainty, verilen), dönüşümün doğruluğu, ikisinin karelerinin toplamının kökü (tahmin), topoloji toleransı ve gösterim basamakları.  \
                      Zincir (chain): iki tarafın yatay sistemi (kod, aile, datum, birim, eksen sırası, datum topluluğunun doğruluğu), düşey sistemi, epoku ve epokun nereden geldiği; okunan ızgaraların türü, SHA-256'sı ve düğüm kutusu; her adımın doğruluk dayanağı; karar (complete ya da limited) ve nedenleri; motorun PROJ'a göre sayısal bütçesi. Sınırlı zincir hassas dönüşüm değildir (chain_limited). Nokta başına karar: her dönüştürülen nokta EPSG'nin kullanım alanlarına karşı okunur (kaynak sistemin ve datum dönüşümünün alanı noktanın kaynaktaki yerinde, hedef sistemin alanı vardığı yerde; kenara 10⁻⁹° yakın kenardadır; datum dönüşümünün alanı EPSG alanının çokgenidir, kaydın area.extent'i); dışında kalan flagged'de sırası ve notlarıyla (outside_source_area, outside_operation_area, outside_target_area) söylenir, yine dönüştürülür, outside_area uyarısı sayar. Projenin kayıtlı kodu KentOS'un çözdüğü sistemlerden değilse unsupported_srid. Çizimi değiştirmez; plan aynı cevabı verir."
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
            id: crate::CAD_ENTITIES_ISOLATE.into(),
            version: crate::CAD_ENTITIES_ISOLATE_VERSION,
            title: "Geçici nesne yalıtımı".into(),
            summary: "action.kind=isolate verilen nesneler dışındakileri yalnız bu oturumda gizler; action.kind=end bitirir, action.kind=query durumu okur. Kalıcı katman süzgeci, görünürlük, kilit ve geometri değişmez. Sonradan oluşturulan nesneler görünürdür. Yalıtım dosyaya, çıktıya ve geometri Undo geçmişine yazılmaz. Tekrar isolate mevcut geçici maskeyi değiştirir. Plan salt okunurdur; hidden/retained sayıları yalnız yalıtım maskesinindir, diğer görünürlük kuralları bağımsızdır.".into(),
            aliases: vec!["isolateobjects".into(), "unisolateobjects".into()],
            effect: CommandEffect::View,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesIsolate>(),
            output: schema::<crate::EntitiesIsolated>(),
            plan: Some(schema::<crate::EntitiesIsolated>()),
            capabilities: None,
            examples: vec![CommandExample { title: "Geçici yalıtımı bitir".into(), input: json!({"action":{"kind":"end"}}), output: None }],
        },
        CommandDescriptor {
            id: crate::CAD_ENTITIES_SELECT.into(),
            version: crate::CAD_ENTITIES_SELECT_VERSION,
            title: "Nesne seç".into(),
            summary: "Bir bölgenin, listenin ya da benzerliğin nesnelerini kalıcı kimlikleriyle verir: pencere, kesişen kutu, çit, çokgen (içindekiler, kesişenler, dışındakiler), daire, noktayı içeren alanlar (küçükten büyüğe), bir tıklamanın adayları, hepsi, verilenler (önceki seçim), ters çevir ve benzerleri (tür, katman, renk, sembol). \
                      Seçim süzgecinin türleri ve katman listesiyle süzülür, dışarıda kalanlar sayılır. Yalnız gösterilen nesneler sayılır (gizli katman, katman süzgeci ve zaman penceresi dışarıda); kilitli katmanınkiler seçilir. Araçlar aynı sorguyla seçer. modelScope verilirse pafta viewport katmanları, açıklama temsili ve kırpması uygulanır; uid modelin özgün kimliğidir. \
                      properties bölgesi source bölgesini query ile daraltır: textHeightMin/Max gerçek model metresi, assignedColor nesnenin kendi rengi (byLayer: kendi rengi yok), displayedColor katmandan çözülmüş CAD rengi (tema paletinden önce; tematik renderer bulunan çizimde reddedilir), expression katman süzgeci dili ve proje çizim birimindeki geometri okumalarıdır. Blokta hedef yerleştirmedir; tanım üyeleri değiştirilmez. İç içe properties reddedilir. Çizimi değiştirmez; plan aynı cevabı verir."
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
                      Projedeki bağlar kopyalara yazılır: bağlı dosyalar klasöre göre göreli (veri/…), bağlı katmanın kaynağı kopyası (göreli yolu veri/…, klasör nereye giderse açılışta bulunur). Göreli bağlar from klasöründen okunur. Projenin kendi veri profilleri ve eşlemeleri de (from klasöründeki profiller/<ad>.profil.json ve <ad>.eslem.json) paketteki profiller/ altına bayt bayt kopyalanır, proje dosyasının yanında durduğundan komutlar onları orada bulur; okunamayan dosya kopyalanmaz ve package_left söyler. \
                      Adres ve yerinde olmayan dosya kopyalanmaz; bağları olduğu gibi kalır, package_left her birini söyler. Parola ve belirteç projede olmadığından pakete girmez. \
                      Bağlı kaynakların yolu seçilir: geopackages (varsayılan copy) ve databases (varsayılan keep) her biri copy, keep ya da embed. copy kaynağı pakete kopyalar: GeoPackage bayt bayt, veritabanı tablosu bağın okuduğu satırlarla (alanla bağlıda yalnız alanınkiler) projenin sisteminde, kalıcı kimlikleriyle bir GeoPackage olarak (veri/<şema>-<tablo>.gpkg; tablo katmanın son eşleşmesinden beri değiştiyse source_changed, ulaşılamazsa source_unreachable); bağ kopyayı gösterir, uzlaştırma tabanı düşer. keep bağı olduğu gibi bırakır (package_left: kept ya da database). embed paketin projesinde bağı kaldırır, katman nesneleriyle projenin kendi katmanı olur (cevabın embedded'ı). \
                      leaveOut pakete girmeyecekleri söyler: bir bağ (kaynak-N; katmanın nesneleri paketten çıkar), bir nesnenin kalıcı kimliği (dosyaları kopyalanmaz) ya da xref:<ad> (dış başvurunun kaynağı kopyalanmaz); package_left excluded der, var olmayanı gösteren öğe invalid_input. validate veritabanını okumaz. \
                      Klasör bütün yapılır ya da hiç: geçici adla yazılır, her kopya geri okunup SHA-256'sı karşılaştırılır, proje dosyası doğrulanır, sonra tek adımla yerine konur; path var olan ve boş olmayan bir klasörse reddedilir (file_exists). Cevabın resources'ı çizimin ve dış başvurularının kullandığı her yazı stili yüzünü, SHX şekil dosyasını, çizgi tipini, tarama desenini ve baskı profilini nasıl taşıdığıyla söyler: builtIn (KentOS'la gelir), embedded (proje dosyasında), carried (yalnız pakete kopyalanan dış başvuru kaynağında; bu çizimde yok), substituted (yerine başka yüzle çizilir) ya da missing; kimin kullandığı (users) ve nesne sayısıyla; eksik ve yalnız kaynakta olanlar package_resources uyarısıdır, paket.json'a da yazılır. exchange istenince paketin nesneleri açık biçimlerde de devir/ altına yazılır (biçimler boşsa altısı): her dosya teslim edildiği hâliyle KentOS'un okuyucusuyla geri okunur, destek matrisi ve her biçimin denetimi cevabın ve paket.json'un exchange'ine, matrisin Türkçesi devir/devir-raporu.csv'ye yazılır; korunmayan özellikler export_losses uyarısıdır. Proje dosyası kaynaktır. plan yazmadan yazılacak olanı söyler. Çizimi değiştirmez ve geri alma adımı değildir."
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
                      Birden çok katmanda katman sütunu satırın katmanını adlandırır. Alanlar katmanların alanlarından (sırasıyla), alan olmayan öznitelikler adlarıyla metin sütunu; yanına .csvt dosyası sütunların türlerini (WKT, CoordX, CoordY, String, Integer64, Real, Date, DateTime, Time, Integer(Boolean)), .prj projenin sistemini (Shapefile'ınki gibi ESRI WKT) yazar. \
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
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) nesnelerini tek bir FlatGeobuf dosyasına yazar (v3; CBS yazılımları ve biçimin kendi kitaplıkları okur). \
                      Geometri biçimin türleriyle: nokta, çok nokta, çizgi, alan ve delikleri, çok parçalı; yaylar CircularString, CompoundCurve, CurvePolygon, MultiCurve ve MultiSurface olarak kesin; Z ve M köşelerde; elips, eğri ve NURBS kirişlerle (söylenir). \
                      Katmanın alanları sütun olur (metin String, tam sayı Long, ondalık Double, evet/hayır Bool, tarih ve tarih-saat DateTime); alan olmayan öznitelikler adlarıyla metin sütunu; birden çok katmanda katman sütunu nesnenin katmanını adlandırır. Projenin sistemi EPSG koduyla yazılır. \
                      Nesneler kutularının Hilbert sırasıyla, önlerinde paketli R-ağacı dizini (16'lık düğümler) yazılır; konumsuz kayıt varsa dizin yazılmaz (dizin her kaydın kutusunu ister). Yazı, ölçü, tablo, resim, raster ve nokta bulutu yazılmaz, türlerine göre sayılır (export_losses). Bütün yazılır ya da hiç; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir."
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
                      Nesneler layer'a, yoksa dosyanın katman adındaki katmana, yoksa sütunları alan olarak taşıyan yeni katmana gider (GeoPackage içe aktarmanın kuralı); kalıcı kimlik dosyanın adı ve nesnenin sırasından. Dizin okunmaz, atlanır; bozuk dosya (sınırların, uzunlukların ya da başvuruların dışı) reddedilir. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir (eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır, yinelenen anahtar durdurur ya da reddedilir), rejects reddedilen satırları nedenleriyle CSV'ye yazar."
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
                title: "Başka bir yazılımın yazdığı parseller".into(),
                input: json!({ "path": "/veri/parsel.fgb" }),
                output: None,
            }],
        },
        // GeoParquet, out and in (docs/adr/0628).
        CommandDescriptor {
            id: crate::FILE_EXPORT_PARQUET.into(),
            version: crate::FILE_EXPORT_PARQUET_VERSION,
            title: "GeoParquet dışa aktar".into(),
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) nesnelerini tek bir GeoParquet 1.1 dosyasına yazar (KentOS'un kendi Parquet yazıcısıyla; CBS ve veri çözümleme yazılımları okur). \
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
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) nesnelerini tek bir GML 3.2 dosyasına yazar (OGC basit nesneler; CBS yazılımları okur). \
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
                      Her nesne türü (tables, boşsa hepsi) bir tablodur: layers'ın adladığı katmana, yoksa türün adındaki katmana, yoksa özellikleri alan olan yeni katmana gider (GeoPackage içe aktarmanın kuralı). Özelliklerin türleri dosyanın yanındaki .xsd'den, yoksa .gfs'den, yoksa değerlerden; xsi:nil boş değer, boş öğe boş metin. Kalıcı kimlik dosyanın adı, nesne türü ve sırasından. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir (eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır, yinelenen anahtar durdurur ya da reddedilir), rejects reddedilen satırları nedenleriyle CSV'ye yazar."
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
                title: "Başka bir yazılımın yazdığı parseller".into(),
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
                      Her kaynak katman layers'ın adladığı katmana, yoksa layer'a, yoksa çizimdeki aynı adlı katmana, yoksa dosyanın alanlarını taşıyan yeni katmana (group'ta) gider; kilitli ya da grup olan hedef reddedilir. Alınamayanlar (okunamayan nesneler, tanınmayan türler, kısaltılan değerler) uyarıda ve cevabın raporunda söylenir. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla birleştirir (bu dosyaların satırlarının kalıcı kimliği yok; eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır), rejects reddedilen satırları nedenleriyle CSV'ye yazar."
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
                      Her kaynak katman layers'ın adladığı katmana, yoksa layer'a, yoksa çizimdeki aynı adlı katmana, yoksa dosyanın alanlarını taşıyan yeni katmana (group'ta) gider; kilitli ya da grup olan hedef reddedilir. Alınamayanlar (okunamayan nesneler, tanınmayan türler, kısaltılan değerler) uyarıda ve cevabın raporunda söylenir. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla birleştirir (bu dosyaların satırlarının kalıcı kimliği yok; eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır), rejects reddedilen satırları nedenleriyle CSV'ye yazar."
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
                      Her kaynak katman layers'ın adladığı katmana, yoksa layer'a, yoksa çizimdeki aynı adlı katmana, yoksa dosyanın alanlarını taşıyan yeni katmana (group'ta) gider; kilitli ya da grup olan hedef reddedilir. Alınamayanlar (okunamayan nesneler, tanınmayan türler, kısaltılan değerler) uyarıda ve cevabın raporunda söylenir. plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla birleştirir (bu dosyaların satırlarının kalıcı kimliği yok; eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır), rejects reddedilen satırları nedenleriyle CSV'ye yazar."
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
            summary: "Katmanların (layers boşsa nesnesi olan bütün katmanların) konumlarını X Y Z satırları olarak metin dosyasına yazar (UTF-8, LF; X doğu, Y kuzey, projenin sisteminde; CBS ve nokta bulutu yazılımları okur). \
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
                      Her sayfanın kaynak dökümü: sayfanın kendi nesnesi önce, sonra o sayfada çizilen haritaların (atlas haritası sayfanın yerinde, genel bakış onu izleyerek, duran haritalar kendi yerinde) zeminine değen ve haritanın bastığı nesneler, tabloların okuduğu nesneler, kalıcı kimlikleriyle çizim sırasında (pages[].sources); listelenen her nesnenin içerik özeti (objects, kentos.object-digest/1). \
                      Kaynak kaydı: execute PDF'in yanına <ad>.kaynaklar.json yazar (kentos.atlas-sources/1: PDF'in adı ve SHA-256'sı, pafta, gün, pin, sayfalar ve kaynakları, nesnelerin özetleri; record). previous önceki atlasın kaydının mutlak yoludur; verilince changes her sayfayı ona göre söyler: same, changed (listelediği nesnelerden özeti değişen changed, yeni listelenen added, artık listelenmeyen removed; projenin settings, layers, styles ya da sheets parçası değiştiyse parts ve her sayfa changed), new; nesnesi artık sayfa olmayan önceki sayfalar gone. Kayıt değilse ya da başka paftanınsa no_previous. \
                      Sayfanın görünüşü: sayfalar yapılınca her sayfanın look'u (SHA-256): paftanın görüntü listesi yayının kendi damgaları dışarıda (gün, kullanıcı, boş kaynak notuna yazılan çizim durumu), haritalarının yolları ve yazıları, tabloları, koordinat listeleri, lejantları, kalite notları ve her sayfada duran haritalar; kayıtta da. Önceki kayıt görünüşleri biliyorsa değişen her sayfanın changes[].look'u same (yalnız çizmediği değişti) ya da changed. \
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
                      Ayırıcı ve kodlama dosyadan bulunur. Sütunların türleri yanındaki .csvt'den, yoksa değerlerden: hepsi tam sayıysa tam sayı, sayıysa ondalık, tarihse tarih (başı sıfırlı kodlar metin kalır). Sistem .prj'den (ESRI ya da EPSG yetkisi), yoksa srid'den; projeninkinden başkaysa reproject ile çevrilir, yoksa crs_mismatch. \
                      Satırlar layer'a, yoksa dosyanın adındaki katmana, yoksa sütunları alan olarak taşıyan yeni katmana gider (GeoPackage içe aktarmanın kuralı); kalıcı kimlik dosyanın adı ve satırından. Okunamayan geometrili satır alınmaz, satır numarasıyla söylenir (import_losses). plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir (eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır, yinelenen anahtar durdurur ya da reddedilir), rejects reddedilen satırları nedenleriyle CSV'ye yazar."
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
                title: "Başka bir yazılımın yazdığı WKT'li CSV".into(),
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
            summary: "Katmanın görünüşünü (işleyicisini, yoksa katmanın renk, çizgi tipi, kalınlık, dolgu ve nokta simgesini) OGC SLD 1.1 ve SE 1.1 olarak path'e (.sld) yazar; CBS yazılımları ve harita sunucuları okur. \
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
                      CBS yazılımlarının, harita sunucularının ve KentOS'un yazdığı SLD 1.0 ve 1.1 okunur; birden çok görünüşten ilki alınır. Katman aynı görünüşteyse adım yazılmaz (changed false). plan dosyayı okur, hiçbir şey yazmaz."
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
                title: "Başka bir yazılımın yazdığı parsel görünüşü".into(),
                input: json!({ "layer": "parsel", "path": "/veri/parsel.sld" }),
                output: None,
            }],
        },
        // A layer's look as a QML layer style file, out and in (docs/adr/0555).
        CommandDescriptor {
            id: crate::FILE_EXPORT_QML.into(),
            version: crate::FILE_EXPORT_QML_VERSION,
            title: "Görünüşü QML olarak dışa aktar".into(),
            summary: "Katmanın görünüşünü (işleyicisini, yoksa katmanın renk, çizgi tipi, kalınlık, dolgu ve nokta simgesini) QML stil dosyası olarak path'e (.qml) yazar; QML okuyan CBS yazılımları açar. \
                      Tek simge, kategorili (Diğerleri: NULL sınıfı), aralıklı ve kurallı görünüş (iç içe kurallar, ölçek aralıkları, ELSE, kapalı kural) QML'in aynı türüyle; kuralların süzgeci QML'in ifadesiyle (\"Ad\" = 'x'): alanın sayı ya da metinle karşılaştırması, arasında, boş, gibi, ve, veya ve değil. \
                      Simgeler: basit dolgu, basit çizgi (renk, kalınlık, kesikler, uç, köşe, halkalar), şekil işaretleri (on beş şekil; döndürme saat yönüyle), merkez işareti; birimler mm, piksel ve haritada metre olarak kalır, simgenin saydamlığı ve kapalı katman korunur. \
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
            summary: "path'teki (ya da text'teki) QML stil dosyasının görünüşünü katmanın işleyicisi yapar, tek geri alma adımı (Katman stili): QML'in tek simge, kategorili, aralıklı ve kurallı görünüşü KentOS'un aynı türü olur; birden çok değerli sınıf her değer için bir sınıf, NULL sınıfı Diğerleri, kuralların süzgeci KentOS'un ifadesine çevrilir (IN de). \
                      Basit dolgu (kenarı ayrı çizgi), basit çizgi (Qt'nin kesik desenleri ya da özel kesikler), basit işaret ve merkez dolgusu, simgenin saydamlığı, kapalı katman, mm, nokta, inç, piksel ve harita birimi alınır. \
                      Başka simge katmanları, fırça desenleri, veriye bağlı değerler, etiketler, katmanın saydamlığı ve çevrilemeyen süzgeç alınmaz, import_losses ile söylenir; aralıkların sınır kuralı farkı da söylenir. QML'in 3.x ve 4.x sürümlerinin yazdığı dosyalar okunur. Katman aynı görünüşteyse adım yazılmaz (changed false). plan dosyayı okur, hiçbir şey yazmaz."
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
                title: "Başka bir yazılımın yazdığı parsel görünüşü".into(),
                input: json!({ "layer": "parsel", "path": "/veri/parsel.qml" }),
                output: None,
            }],
        },
        // The layers' looks as a MapLibre GL style, out and in (docs/adr/0560).
        CommandDescriptor {
            id: crate::FILE_EXPORT_MAPLIBRE.into(),
            version: crate::FILE_EXPORT_MAPLIBRE_VERSION,
            title: "Görünüşü MapLibre stili olarak dışa aktar".into(),
            summary: "Bir katmanın (layer) ya da görünen bütün katmanların görünüşünü MapLibre GL stili (sürüm 8) olarak path'e (.json) yazar; MapLibre okuyucuları ve CBS yazılımlarının vektör karo stili açar. \
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
            summary: "Çizimi DXF R2007 dosyası olarak yazar: bütün nesneler ya da layers'ın katmanlarındakiler, katmanlarıyla; DXF'in tutamadıkları KentOS verisiyle. \
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
            summary: "Çizimi seçilen binary DWG biçiminde yazar (version: AC1014, AC1015, AC1018, AC1021, AC1024, AC1027 veya varsayılan AC1032). Ürün yılı binary biçim yılı değildir. Sürümü sabit OpenCADCodec ile bellekte dönüştürür; dosya dışarı gönderilmez: bütün nesneler, uids ile seçilenler ya da layers'ın katmanlarındakiler, katmanlarıyla; DWG'in tutamadıkları KentOS verisiyle. \
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
        // Dışa aktarma kaybı denetimi (plan/map-cad.md MAP-17, docs/adr/1660).
        CommandDescriptor {
            id: crate::FILE_EXCHANGE_CHECK.into(),
            version: crate::FILE_EXCHANGE_CHECK_VERSION,
            title: "Dışa aktarma kaybı denetimi".into(),
            summary: "Çizimi (layers boşsa biçimin yazıcısının varsayılan katmanlarını) seçilen değişim biçimine (gpkg, parquet, fgb, gml, shp, dxf) KentOS'un yazıcısıyla geçici bir dosyaya yazar, KentOS'un aynı biçim okuyucusuyla çizimin boş bir kopyasına (ayarlar ve katmanlar; nesne, blok tanımı ve ilişki olmadan: geri okunanlar okuyucunundur) geri okur ve nesne nesne karşılaştırır; dosya silinir, açık çizim değişmez. \
                      Eşleme: kalıcı kimliği aynı olan, yoksa sınıfı (nokta, çizgi, alan) aynı ve kutu merkezi en yakın nesne. \
                      Sayısal (numeric): geri okunan her köşenin özgün geometriye uzaklığı (çizgi, yay, daire kesin; elips, eğri, NURBS 1024 parçalık çevreyle), köşe sayısı aynıysa kot ve ölçü farkı, uzunluk ve alan farkı; en büyükleri, en büyük kaymanın nesnesi ve toleransı (metre, varsayılan 0,001) aşanların sayısı. \
                      Anlamsal (losses): notWritten (türüne göre), kindChanged (circle → polygon gibi), chorded (eğri kirişe döndü), zLost, zAdded, mLost, attrLost, attrChanged, attrAdded (öznitelik adıyla), styleLost (color, lineWeight, symbol), identityLost (kalıcı kimlik korunmadı), extra (eşleşmeyen geri okunan), blockChanged (blok başka tanımla geri okundu: ad, taban, nesnelerin türü ve yeri, ekin ölçeği, dönüklüğü, aynası; blok adıyla), tableChanged (tablonun cells, sizes ya da merges'i değişti), relationLost (yazılan katmanlar arasındaki proje ilişkisi geri okunmadı ya da anahtarı ana nesneye ulaşmıyor; ilişkinin adıyla); her biri sayısı ve özgün nesnelerin kimlikleriyle (en çok 100). Yazıcının ve okuyucunun uyarıları writer ve reader'da. \
                      Sonuç KentOS'un yazıcısıyla okuyucusunun birlikte yaptığıdır; başka bir programın okuduğu kendisinindir. \
                      Retler: invalid_tolerance, layer_not_found (layers[i]), yazıcının kendi reddi (export.<alan>), exchange_unreadable, file_unwritable."
                .into(),
            aliases: vec!["aktarmakaybi".into(), "kayipraporu".into(), "roundtrip".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExchangeCheck>(),
            output: schema::<crate::ExchangeChecked>(),
            plan: Some(schema::<crate::ExchangeChecked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimin GeoPackage gidiş-dönüşü".into(),
                input: json!({ "format": "gpkg" }),
                output: None,
            }],
        },
        // Destek matrisi (plan/map-cad.md MAP-17, docs/adr/1880).
        CommandDescriptor {
            id: crate::FILE_EXCHANGE_MATRIX.into(),
            version: crate::FILE_EXCHANGE_MATRIX_VERSION,
            title: "Destek matrisi".into(),
            summary: "Çizimi istenen biçimlerin (formats; boşsa altısı: gpkg, parquet, fgb, gml, shp, dxf) her biriyle file.exchange.check'in gidiş-dönüşünden geçirir (layers ve tolerance onunki gibi) ve her biçimin neyi koruduğunu, yaklaşıklaştırdığını, dönüştürdüğünü, eklediğini ya da kaybettiğini satır satır verir: objects (yazılmayan nesneler, türleriyle), kinds (circle → polygon), curves (kirişe dönen eğriler), z (kaybolan, yoksa eklenen kot), m, attributes (kaybolan, yoksa değişen, yoksa eklenen; adlarıyla), style, identity (kalıcı kimlik), position (toleransı aşan köşeler; en büyük kayma), extra (eşleşmeyen geri okunan), blocks ve tables (yazılmayan ekler ya da tablolar kaybolur, başka türe dönen ya da değişen dönüşür), relations (geri okunmayan proje ilişkileri, adlarıyla). Hücre: format, state (kept, approximated, converted, added, lost), count, detail. Her biçimin denetim cevabı da checks'te. Açık çizim değişmez. \
                      Retler: file.exchange.check'inkiler (invalid_tolerance, layer_not_found, yazıcının kendi reddi, exchange_unreadable, file_unwritable), biçimin yolu formats[i] altında."
                .into(),
            aliases: vec!["destekmatrisi".into(), "bicimmatrisi".into(), "exchangematrix".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExchangeMatrix>(),
            output: schema::<crate::ExchangeMatrixed>(),
            plan: Some(schema::<crate::ExchangeMatrixed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimin altı biçimle destek matrisi".into(),
                input: json!({}),
                output: None,
            }],
        },
        // Kurtar (docs/adr/1013), held by fixtures/commands/v1/file.recover.json.
        CommandDescriptor {
            id: crate::FILE_RECOVER.into(),
            version: crate::FILE_RECOVER_VERSION,
            title: "Bozuk çizimi kurtar".into(),
            summary: "Normal okuyucunun açmadığı bir .kcad dosyasından (path) okunabilen her şeyi ayrı bir kopyaya yazar (target; verilmezse kaynağın yanında <ad>-kurtarilan.kcad). Kaynak yalnız okunur, üzerine hiçbir zaman yazılmaz; kopya var olan bir dosyanın üzerine yazılmaz (file_exists). \
                      Özeti tutmayan, kesik ya da sonunda fazla baytı olan dosya da okunur; yük kesildiği yere kadar okunur. Kurtarılan her parça normal okuyucunun bütün kurallarından geçer: okunmasını engelleyen çizim bölümü (document.<anahtar>) ve tek başına okunmayan nesne (entities[i], kalıcı kimliğiyle) çıkarılır, okuyucunun kendi koduyla ve sözüyle lost'ta listelenir. Kesik dosyada kaybolan zorunlu bölümler (ayarlar, etkin katman) boş çizimin değeriyle konur ve söylenir: koordinat sistemi bilinmez, yeniden atanmalıdır. \
                      Sağlam okunan dosya whole: true ile söylenir, kopya yazılmaz. plan yazmadan söyler. Açık çizimi değiştirmez, geri alma adımı değildir. \
                      Retler: invalid_file_path, file_not_found, file_exists, kcad_unrecoverable, file_unwritable."
                .into(),
            aliases: vec!["kurtar".into(), "recover".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::RecoverKcad>(),
            output: schema::<crate::KcadRecovered>(),
            plan: Some(schema::<crate::KcadRecovered>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bozuk çizimin kurtarılan kopyası".into(),
                input: json!({ "path": "/tmp/pafta.kcad" }),
                output: None,
            }],
        },
        // WBLOCK (docs/adr/0316 §3).
        CommandDescriptor {
            id: crate::FILE_EXPORT_BLOCK.into(),
            version: crate::FILE_EXPORT_BLOCK_VERSION,
            title: "Bloğu dosyaya yaz".into(),
            summary: "Bir bloğu (block) ya da kalıcı kimlikleriyle verilen nesneleri (uids) yeni bir .kcad çizimi olarak yazar: kullandıkları katmanlar, iç içe bloklar, stiller, kitaplık öğeleri ve proje ayarlarıyla. \
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
            summary:"Seçilen paftaları açık sırasıyla tek veya ayrı PDF olarak yeni/boş bir teslim klasörüne yayınlar. Ayrı dosyalarda {index}, {sheet}, {id} ad şablonu; çakışan adlara sabit sayısal ek. Her pafta kendi dpi ayarını korur; açık dpi bütün yayın için geçerlidir. plan dosya yazmadan çıktı adlarını, kaynak pinini ve pafta bazlı kaynak/ön denetim raporunu verir; hatalı set yayınlanmaz. Bütün klasör tek seferde görünür olur; iptalde kısmi teslim bırakılmaz. İş ilerlemesi sayfa sınırlarında güncellenir. yayin.json çıktı özetlerini, kaynak sürümünü ve her sayfanın kaynak nesnelerini içerik özetleriyle taşır; çizimin geri alması teslim dosyalarını kaldırmaz. previous ile önceki bir teslimin güncellemesi yeni klasöre yayınlanır: sayfası aynı çizen ve PDF'i teslim edilen baytları taşıyan pafta baytı baytına ve kendi tarihiyle kopyalanır (carried), öbürleri bugünün tarihiyle yeniden üretilir; önceki teslim olduğu gibi kalır. Değişip revizyonu ilerletilmeyen paftaya revision_unchanged uyarısı. Yalnız ayrı PDF'lerle, devam ettirilebilir yayın olmadan; geo, layers, dpi ya da bağlı resimler önceki teslimdekinden başkaysa hiçbir sayfa kopyalanmaz.".into(),
            aliases:vec!["PAFTAYAYINLA".into()],effect:CommandEffect::Query,
            hosts:vec![CommandHost::Desktop],headless:true,requires:vec![CommandRequirement::Document],permissions:vec![],undo:CommandUndo::None,cost:CommandCost::Job,
            input:schema::<crate::PublishSheets>(),output:schema::<crate::SheetsPublished>(),plan:Some(schema::<crate::SheetsPublished>()),capabilities:None,
            examples:vec![CommandExample{title:"Paftaların tamamı, ayrı PDF dosyaları".into(),input:json!({"path":"/tmp/pafta-teslimi","mode":"separate","naming":"{index}-{sheet}"}),output:None},CommandExample{title:"Önceki teslimin güncellemesi: yalnız değişen paftalar yeniden üretilir".into(),input:json!({"path":"/tmp/pafta-teslimi-2","mode":"separate","previous":"/tmp/pafta-teslimi"}),output:None}],
        },
        // Yayın durumu (docs/adr/1090 §6): a delivery against the drawing now, page by page.
        CommandDescriptor {
            id:crate::FILE_PUBLISH_STATUS.into(),version:crate::FILE_PUBLISH_STATUS_VERSION,
            title:"Yayın durumu".into(),
            summary:"Bir teslim klasörünün son yayınını çizimin şimdiki hâliyle sayfa sayfa karşılaştırır; dosya yazmaz. Her paftanın sayfası başlık bağlamı (kullanıcı, tarih) olmadan yeniden çizilir ve içerik özeti yayındakiyle karşılaştırılır: current (aynı), stale (değişti; yeniden yayınlayın), failed (şimdi üretilemiyor), fileChanged (teslim edilen PDF eksik ya da değişmiş), removed (kitapta yok), unpublished (yayında yok), unknown (özetsiz eski yayın). Çizimin şimdiki kaynak pini (current) ile yayının kaynağı (published) ayrı verilir; stale yeniden üretilecek paftaları yayın sırasıyla listeler ve file.publish.sheets'in sheets girdisine olduğu gibi verilir. Durdurulan ya da başarısız devam edilebilir bir yayın (resumable) için stopped: neden, bitmiş ve kalan sayfalar, bulgular ve resume ile devam edilip edilemeyeceği. Her sayfanın teslim tarihi (date) ve kaynaklarının teslimden beri değişimi (changes): içeriği değişen, sayfaya giren ve sayfadan çıkan nesneler kalıcı kimlikleriyle, paftanın kendi kaydının değişip değişmediği ve değişen proje parçaları (settings, layers, styles, blocks, book, resources).".into(),
            aliases:vec!["YAYINDURUMU".into()],effect:CommandEffect::Query,
            hosts:vec![CommandHost::Desktop],headless:true,requires:vec![CommandRequirement::Document],permissions:vec![],undo:CommandUndo::None,cost:CommandCost::Job,
            input:schema::<crate::PublishStatus>(),output:schema::<crate::PublishStatusReport>(),plan:Some(schema::<crate::PublishStatusReport>()),capabilities:None,
            examples:vec![CommandExample{title:"Teslim klasörünün güncelliği".into(),input:json!({"path":"/tmp/pafta-teslimi"}),output:None}],
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
                    crate::CAD_DRAWING_INSPECT => serde_json::from_value::<crate::DrawingInspect>(e.input.clone()).map(|_| ()),
                    crate::CAD_ARRAYS_FIND => {
                        serde_json::from_value::<crate::ArraysFind>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ARRAYS_ADOPT => {
                        serde_json::from_value::<crate::ArraysAdopt>(e.input.clone()).map(|_| ())
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
                    crate::CAD_ARCH_RECOGNIZE => {
                        serde_json::from_value::<crate::ArchRecognize>(e.input.clone()).map(|_| ())
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
                    crate::CAD_BEAMS_CREATE => {
                        serde_json::from_value::<crate::BeamsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_ROOFS_CREATE => {
                        serde_json::from_value::<crate::RoofsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_RAILINGS_CREATE => {
                        serde_json::from_value::<crate::RailingsCreate>(e.input.clone()).map(|_| ())
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
                    crate::CAD_PROFILES_COMPARE => {
                        serde_json::from_value::<crate::ProfilesCompare>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROFILES_CARRY => {
                        serde_json::from_value::<crate::ProfilesCarry>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROFILES_REMOVE => {
                        serde_json::from_value::<crate::ProfilesRemove>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PROFILES_NAMES => {
                        serde_json::from_value::<crate::ProfilesNames>(e.input.clone()).map(|_| ())
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
                    crate::CAD_EXTRACT_RUN => {
                        serde_json::from_value::<crate::ExtractRun>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_EXTRACT_TABLE => {
                        serde_json::from_value::<crate::ExtractTable>(e.input.clone()).map(|_| ())
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
                        serde_json::from_value::<crate::ElevationsQuery>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SECTIONS_EDIT => {
                        serde_json::from_value::<crate::BuildingSectionsEdit>(e.input.clone())
                            .map(|_| ())
                    }
                    crate::CAD_SECTIONS_QUERY => {
                        serde_json::from_value::<crate::BuildingSectionsQuery>(e.input.clone())
                            .map(|_| ())
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
                    crate::CAD_ENTITIES_FLATTEN => {
                        serde_json::from_value::<crate::EntitiesFlatten>(e.input.clone()).map(|_| ())
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
                    crate::CAD_BLOCKS_CONVERT => {
                        serde_json::from_value::<crate::BlocksConvert>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_BLOCKS_FIND => {
                        serde_json::from_value::<crate::BlocksFind>(e.input.clone()).map(|_| ())
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
                    crate::FILE_RECOVER => {
                        serde_json::from_value::<crate::RecoverKcad>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXCHANGE_CHECK => {
                        serde_json::from_value::<crate::ExchangeCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXCHANGE_MATRIX => {
                        serde_json::from_value::<crate::ExchangeMatrix>(e.input.clone()).map(|_| ())
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
                    crate::ROAD_TERRAINS_CREATE => {
                        serde_json::from_value::<crate::TerrainsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_TERRAINS_EDIT => {
                        serde_json::from_value::<crate::TerrainsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_TERRAINS_QUERY => {
                        serde_json::from_value::<crate::TerrainsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_TERRAINS_CONTOURS => {
                        serde_json::from_value::<crate::TerrainsContours>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_TERRAINS_SHEET_CONTOURS => {
                        serde_json::from_value::<crate::TerrainsSheetContours>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_TERRAINS_SECTIONCHECK => {
                        serde_json::from_value::<crate::TerrainsSectionCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_TERRAINS_ZEROLINE => {
                        serde_json::from_value::<crate::TerrainsZeroLine>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_TERRAINS_CUTFILL => {
                        serde_json::from_value::<crate::TerrainsCutFill>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_IMPORT_LANDXML => {
                        serde_json::from_value::<crate::ImportLandxml>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_ALIGNMENTS_CREATE => {
                        serde_json::from_value::<crate::AlignmentsCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_ALIGNMENTS_EDIT => {
                        serde_json::from_value::<crate::AlignmentsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_ALIGNMENTS_QUERY => {
                        serde_json::from_value::<crate::AlignmentsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_ALIGNMENTS_LABEL => {
                        serde_json::from_value::<crate::AlignmentsLabel>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_CREATE => {
                        serde_json::from_value::<crate::ProfilesCreate>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_EDIT => {
                        serde_json::from_value::<crate::ProfilesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_QUERY => {
                        serde_json::from_value::<crate::ProfilesQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_DRAW => {
                        serde_json::from_value::<crate::ProfilesDraw>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_SIGHT => {
                        serde_json::from_value::<crate::ProfilesSight>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_PROPOSE => {
                        serde_json::from_value::<crate::ProfilesPropose>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_ALIGNMENTS_CHECK => {
                        serde_json::from_value::<crate::AlignmentsCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_CHECK => {
                        serde_json::from_value::<crate::RoadProfilesCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_LENGTHS => {
                        serde_json::from_value::<crate::RoadProfilesLengths>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_TEMPLATES_EDIT => {
                        serde_json::from_value::<crate::TemplatesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SECTIONS_EDIT => {
                        serde_json::from_value::<crate::SectionsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SECTIONS_QUERY => {
                        serde_json::from_value::<crate::SectionsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SECTIONS_DRAW => {
                        serde_json::from_value::<crate::SectionsDraw>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SUPERELEVATIONS_EDIT => {
                        serde_json::from_value::<crate::SuperelevationsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SUPERELEVATIONS_QUERY => {
                        serde_json::from_value::<crate::SuperelevationsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SUPERELEVATIONS_CHECK => {
                        serde_json::from_value::<crate::SuperelevationsCheck>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SUPERELEVATIONS_WIDEN => {
                        serde_json::from_value::<crate::SuperelevationsWiden>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SUPERELEVATIONS_TRANSITIONS => {
                        serde_json::from_value::<crate::SuperelevationsTransitions>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CORRIDORS_EDIT => {
                        serde_json::from_value::<crate::CorridorsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CORRIDORS_QUERY => {
                        serde_json::from_value::<crate::CorridorsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CORRIDORS_DRAW => {
                        serde_json::from_value::<crate::CorridorsDraw>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CORRIDORS_WATER => {
                        serde_json::from_value::<crate::CorridorsWater>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CORRIDORS_DAYLIGHTS => {
                        serde_json::from_value::<crate::CorridorsDaylights>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CORRIDORS_PROTECTED => {
                        serde_json::from_value::<crate::CorridorsProtected>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CORRIDORS_PREFLIGHT => {
                        serde_json::from_value::<crate::CorridorsPreflight>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_EDIT => {
                        serde_json::from_value::<crate::VolumesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_QUERY => {
                        serde_json::from_value::<crate::VolumesQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_APPROVE => {
                        serde_json::from_value::<crate::VolumesApprove>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_UNCERTAINTY => {
                        serde_json::from_value::<crate::VolumesUncertainty>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_PROGRESS => {
                        serde_json::from_value::<crate::VolumesProgress>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_COMPARE => {
                        serde_json::from_value::<crate::VolumesCompare>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_LEDGER => {
                        serde_json::from_value::<crate::VolumesLedger>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_STATES => {
                        serde_json::from_value::<crate::VolumesStates>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_IMPACT => {
                        serde_json::from_value::<crate::ProfilesImpact>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_ALIGNMENTS_IMPACT => {
                        serde_json::from_value::<crate::AlignmentsImpact>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_VOLUMES_HISTORY => {
                        serde_json::from_value::<crate::VolumesHistory>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_HAULS_EDIT => serde_json::from_value::<crate::HaulsEdit>(e.input.clone()).map(|_| ()),
                    crate::ROAD_HAULS_QUERY => serde_json::from_value::<crate::HaulsQuery>(e.input.clone()).map(|_| ()),
                    crate::ROAD_HAULS_PLAN => serde_json::from_value::<crate::HaulsPlan>(e.input.clone()).map(|_| ()),
                    crate::ROAD_HAULS_APPROVE => serde_json::from_value::<crate::HaulsApprove>(e.input.clone()).map(|_| ()),
                    crate::ROAD_HAULS_NETWORK => serde_json::from_value::<crate::HaulsNetwork>(e.input.clone()).map(|_| ()),
                    crate::ROAD_HAULS_TRUCKS => serde_json::from_value::<crate::HaulsTrucks>(e.input.clone()).map(|_| ()),
                    crate::ROAD_GUARDRAILS_EDIT => {
                        serde_json::from_value::<crate::GuardrailsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_GUARDRAILS_QUERY => {
                        serde_json::from_value::<crate::GuardrailsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_GUARDRAILS_DRAW => {
                        serde_json::from_value::<crate::GuardrailsDraw>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CULVERTS_EDIT => {
                        serde_json::from_value::<crate::CulvertsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CULVERTS_QUERY => {
                        serde_json::from_value::<crate::CulvertsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CULVERTS_DRAW => {
                        serde_json::from_value::<crate::CulvertsDraw>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CULVERTS_HYDRAULICS => {
                        serde_json::from_value::<crate::CulvertsHydraulics>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_CULVERTS_CLOSURE => {
                        serde_json::from_value::<crate::CulvertsClosure>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PLATFORMS_EDIT => serde_json::from_value::<crate::PlatformsEdit>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_FROM_CLOUD => serde_json::from_value::<crate::TerrainsFromCloud>(e.input.clone()).map(|_| ()),
                    crate::ROAD_DRAINAGE_QUERY => serde_json::from_value::<crate::DrainageQuery>(e.input.clone()).map(|_| ()),
                    crate::ROAD_DRAINAGE_DRAW => serde_json::from_value::<crate::DrainageDraw>(e.input.clone()).map(|_| ()),
                    crate::ROAD_SWEPT_QUERY => serde_json::from_value::<crate::SweptQuery>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_ANALYSIS => serde_json::from_value::<crate::TerrainsAnalysis>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_COMPARE => serde_json::from_value::<crate::TerrainsCompare>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_QUALITY => serde_json::from_value::<crate::TerrainsQuality>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_REPAIR => serde_json::from_value::<crate::TerrainsRepair>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_COVERAGE => serde_json::from_value::<crate::TerrainsCoverage>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW => serde_json::from_value::<crate::TerrainsCoverageDraw>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_DENSITY => serde_json::from_value::<crate::TerrainsDensity>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_STOCKPILE => serde_json::from_value::<crate::TerrainsStockpile>(e.input.clone()).map(|_| ()),
                    crate::ROAD_TERRAINS_SPARSE_DRAW => serde_json::from_value::<crate::TerrainsSparseDraw>(e.input.clone()).map(|_| ()),
                    crate::ROAD_SWEPT_DRAW => serde_json::from_value::<crate::SweptDraw>(e.input.clone()).map(|_| ()),
                    crate::ROAD_PLATFORMS_QUERY => serde_json::from_value::<crate::PlatformsQuery>(e.input.clone()).map(|_| ()),
                    crate::ROAD_PLATFORMS_DRAW => serde_json::from_value::<crate::PlatformsDraw>(e.input.clone()).map(|_| ()),
                    crate::ROAD_WALLS_EDIT => {
                        serde_json::from_value::<crate::RetainingWallsEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_WALLS_QUERY => {
                        serde_json::from_value::<crate::RetainingWallsQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_WALLS_DRAW => {
                        serde_json::from_value::<crate::RetainingWallsDraw>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PAY_EDIT => serde_json::from_value::<crate::PayEdit>(e.input.clone()).map(|_| ()),
                    crate::ROAD_PAY_QUERY => serde_json::from_value::<crate::PayQuery>(e.input.clone()).map(|_| ()),
                    crate::ROAD_ROUTES_FIND => serde_json::from_value::<crate::RoutesFind>(e.input.clone()).map(|_| ()),
                    crate::ROAD_ROUTES_DRAW => serde_json::from_value::<crate::RoutesDraw>(e.input.clone()).map(|_| ()),
                    crate::ROAD_JUNCTIONS_EDIT => serde_json::from_value::<crate::JunctionsEdit>(e.input.clone()).map(|_| ()),
                    crate::ROAD_JUNCTIONS_QUERY => serde_json::from_value::<crate::JunctionsQuery>(e.input.clone()).map(|_| ()),
                    crate::ROAD_JUNCTIONS_CLEARANCE => serde_json::from_value::<crate::JunctionsClearance>(e.input.clone()).map(|_| ()),
                    crate::ROAD_JUNCTIONS_TIE => serde_json::from_value::<crate::JunctionsTie>(e.input.clone()).map(|_| ()),
                    crate::ROAD_JUNCTIONS_OVERLAP => serde_json::from_value::<crate::JunctionsOverlap>(e.input.clone()).map(|_| ()),
                    crate::ROAD_PROJECT_QUERY => serde_json::from_value::<crate::RoadProjectQuery>(e.input.clone()).map(|_| ()),
                    crate::ROAD_PROJECT_GRAPH => serde_json::from_value::<crate::RoadProjectGraphQuery>(e.input.clone()).map(|_| ()),
                    crate::ROAD_PROJECT_REBUILD => serde_json::from_value::<crate::RoadProjectRebuild>(e.input.clone()).map(|_| ()),
                    crate::ROAD_PROJECT_PRODUCE => serde_json::from_value::<crate::RoadProjectProduce>(e.input.clone()).map(|_| ()),
                    crate::ROAD_PROJECT_EDIT => serde_json::from_value::<crate::RoadProjectEdit>(e.input.clone()).map(|_| ()),
                    crate::MAP_WORK_QUERY => serde_json::from_value::<crate::MapWorkQuery>(e.input.clone()).map(|_| ()),
                    crate::MAP_WORK_EDIT => serde_json::from_value::<crate::MapWorkEdit>(e.input.clone()).map(|_| ()),
                    crate::MAP_WORK_RESTORE => serde_json::from_value::<crate::MapWorkRestore>(e.input.clone()).map(|_| ()),
                    crate::CAD_CRS_FACTORS => serde_json::from_value::<crate::CrsFactors>(e.input.clone()).map(|_| ()),
                    crate::MAP_FEATURES_CODE => serde_json::from_value::<crate::FeaturesCode>(e.input.clone()).map(|_| ()),
                    crate::CAD_SURVEY_BASELINES => serde_json::from_value::<crate::SurveyBaselines>(e.input.clone()).map(|_| ()),
                    crate::CAD_ENTITIES_GRADE => serde_json::from_value::<crate::EntitiesGrade>(e.input.clone()).map(|_| ()),
                    crate::CAD_ENTITIES_RAISE => serde_json::from_value::<crate::EntitiesRaise>(e.input.clone()).map(|_| ()),
                    crate::CAD_ENTITIES_DRAPE => serde_json::from_value::<crate::EntitiesDrape>(e.input.clone()).map(|_| ()),
                    crate::CAD_TOPOLOGY_CLEAN => serde_json::from_value::<crate::TopologyClean>(e.input.clone()).map(|_| ()),
                    crate::CAD_PARCELS_CHECK => serde_json::from_value::<crate::ParcelsCheck>(e.input.clone()).map(|_| ()),
                    crate::CAD_PARCELS_BOUNDARIES => serde_json::from_value::<crate::ParcelsBoundaries>(e.input.clone()).map(|_| ()),
                    crate::CAD_PARCELS_RECORD => serde_json::from_value::<crate::ParcelsRecord>(e.input.clone()).map(|_| ()),
                    crate::CAD_PARCELS_RECORDS => serde_json::from_value::<crate::ParcelsRecords>(e.input.clone()).map(|_| ()),
                    crate::CAD_PARCELS_APPROVE => serde_json::from_value::<crate::ParcelsApprove>(e.input.clone()).map(|_| ()),
                    crate::CAD_ASBUILT_REVIEW => serde_json::from_value::<crate::AsbuiltReview>(e.input.clone()).map(|_| ()),
                    crate::CAD_PARCELS_WITHDRAW => serde_json::from_value::<crate::ParcelsWithdraw>(e.input.clone()).map(|_| ()),
                    crate::CAD_PIPES_CHECK => serde_json::from_value::<crate::PipesCheck>(e.input.clone()).map(|_| ()),
                    crate::CAD_PIPES_PROFILE => serde_json::from_value::<crate::PipesProfile>(e.input.clone()).map(|_| ()),
                    crate::CAD_ASBUILT_CHECK => serde_json::from_value::<crate::AsbuiltCheck>(e.input.clone()).map(|_| ()),
                    crate::CAD_FIELD_PACKAGE => serde_json::from_value::<crate::FieldPackage>(e.input.clone()).map(|_| ()),
                    crate::CAD_ZONING_CHECK => serde_json::from_value::<crate::ZoningCheck>(e.input.clone()).map(|_| ()),
                    crate::CAD_ZONING_SYMBOLIZE => serde_json::from_value::<crate::ZoningSymbolize>(e.input.clone()).map(|_| ()),
                    crate::CAD_ZONING_DRAFT => serde_json::from_value::<crate::ZoningDraft>(e.input.clone()).map(|_| ()),
                    crate::CAD_SCENARIOS_IMPACT => serde_json::from_value::<crate::ScenariosImpact>(e.input.clone()).map(|_| ()),
                    crate::CAD_SCENARIOS_RECORD => serde_json::from_value::<crate::ScenariosRecord>(e.input.clone()).map(|_| ()),
                    crate::CAD_READJUSTMENT_DISTRIBUTE => serde_json::from_value::<crate::ReadjustmentDistribute>(e.input.clone()).map(|_| ()),
                    crate::CAD_READJUSTMENT_SCOPE => serde_json::from_value::<crate::ReadjustmentScope>(e.input.clone()).map(|_| ()),
                    crate::CAD_READJUSTMENT_SHEETS => serde_json::from_value::<crate::ReadjustmentSheets>(e.input.clone()).map(|_| ()),
                    crate::CAD_ORTHOPHOTO_CHECK => serde_json::from_value::<crate::OrthophotoCheck>(e.input.clone()).map(|_| ()),
                    crate::CAD_POINTCLOUDS_REGISTER => serde_json::from_value::<crate::PointCloudsRegister>(e.input.clone()).map(|_| ()),
                    crate::CAD_POINTCLOUDS_NETWORK => serde_json::from_value::<crate::PointCloudsNetwork>(e.input.clone()).map(|_| ()),
                    crate::CAD_POINTCLOUDS_PLACE => serde_json::from_value::<crate::PointCloudsPlace>(e.input.clone()).map(|_| ()),
                    crate::CAD_ORTHOPHOTO_COMPARE => serde_json::from_value::<crate::OrthophotoCompare>(e.input.clone()).map(|_| ()),
                    crate::CAD_ORTHOPHOTO_SEAMLINES => serde_json::from_value::<crate::OrthophotoSeamlines>(e.input.clone()).map(|_| ()),
                    crate::CAD_ORTHOPHOTO_NETWORK => serde_json::from_value::<crate::OrthophotoNetwork>(e.input.clone()).map(|_| ()),
                    crate::CAD_ORTHOPHOTO_FOOTPRINT => serde_json::from_value::<crate::OrthophotoFootprint>(e.input.clone()).map(|_| ()),
                    crate::CAD_POINTCLOUDS_OVERLAP => serde_json::from_value::<crate::PointCloudsOverlap>(e.input.clone()).map(|_| ()),
                    crate::MAP_SURVEY_QUERY => serde_json::from_value::<crate::MapSurveyQuery>(e.input.clone()).map(|_| ()),
                    crate::MAP_SURVEY_NETWORK => serde_json::from_value::<crate::MapSurveyNetwork>(e.input.clone()).map(|_| ()),
                    crate::MAP_FEATURES_RECONCILE => serde_json::from_value::<crate::FeaturesReconcile>(e.input.clone()).map(|_| ()),
                    crate::MAP_SURVEY_EDIT => serde_json::from_value::<crate::MapSurveyEdit>(e.input.clone()).map(|_| ()),
                    crate::ROAD_JUNCTIONS_DRAW => serde_json::from_value::<crate::JunctionsDraw>(e.input.clone()).map(|_| ()),
                    crate::ROAD_STAKEOUT_QUERY => {
                        serde_json::from_value::<crate::StakeoutQuery>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_LANDXML => {
                        serde_json::from_value::<crate::ExportLandxml>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_IFC => serde_json::from_value::<crate::ExportIfc>(e.input.clone()).map(|_| ()),
                    crate::CAD_ARCH_IFC => {
                        serde_json::from_value::<crate::ExportIfcBuilding>(e.input.clone()).map(|_| ())
                    }
                    crate::FILE_EXPORT_GLTF => serde_json::from_value::<crate::ExportGltf>(e.input.clone()).map(|_| ()),
                    crate::ROAD_STAKEOUT_SEND => {
                        serde_json::from_value::<crate::StakeoutSend>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SHEETS_PLAN => {
                        serde_json::from_value::<crate::RoadSheetsPlan>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_SHEETS_SECTIONS => {
                        serde_json::from_value::<crate::RoadSheetsSections>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_ALIGNMENTS_FIT => {
                        serde_json::from_value::<crate::AlignmentsFit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_PROFILES_FIT => {
                        serde_json::from_value::<crate::ProfilesFit>(e.input.clone()).map(|_| ())
                    }
                    crate::ROAD_ALIGNMENTS_IMPORT => {
                        serde_json::from_value::<crate::AlignmentsImport>(e.input.clone()).map(|_| ())
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
                    crate::CAD_TOPOLOGY_REPAIR => {
                        serde_json::from_value::<crate::TopologyRepair>(e.input.clone()).map(|_| ())
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
                    crate::CAD_STANDARDS_PALETTES => {
                        serde_json::from_value::<crate::StandardsPalettes>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STANDARDS_RESOLVE => {
                        serde_json::from_value::<crate::StandardsResolve>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STANDARDS_CONFLICTS => {
                        serde_json::from_value::<crate::StandardsConflicts>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STANDARDS_INSTALL => {
                        serde_json::from_value::<crate::StandardsInstall>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_STANDARDS_EXCEPTIONS => {
                        serde_json::from_value::<crate::StandardsExceptions>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DRAWING_REPAIR => {
                        serde_json::from_value::<crate::DrawingRepair>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_DRAWING_TAKE => {
                        serde_json::from_value::<crate::DrawingTake>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_REVIEW_NOTES_EDIT => {
                        serde_json::from_value::<crate::ReviewNotesEdit>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_REVIEW_NOTES_LIST => {
                        serde_json::from_value::<crate::ReviewNotesList>(e.input.clone()).map(|_| ())
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
                    crate::CAD_SHEETS_DETAILS => {
                        serde_json::from_value::<crate::SheetsDetails>(e.input.clone()).map(|_| ())
                    }
                    crate::CAD_PLOTSTYLES_IMPORT => {
                        serde_json::from_value::<crate::PlotStylesImport>(e.input.clone()).map(|_| ())
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
                    crate::FILE_PUBLISH_STATUS => {
                        serde_json::from_value::<crate::PublishStatus>(e.input.clone()).map(|_| ())
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
                    crate::CAD_ENTITIES_ISOLATE => {
                        serde_json::from_value::<crate::EntitiesIsolate>(e.input.clone()).map(|_| ())
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
                        crate::ROAD_TERRAINS_CREATE | crate::ROAD_TERRAINS_EDIT => {
                            serde_json::from_value::<crate::TerrainsWritten>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_TERRAINS_QUERY => {
                            serde_json::from_value::<crate::TerrainsQueried>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_TERRAINS_CONTOURS => {
                            serde_json::from_value::<crate::TerrainsContoured>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_TERRAINS_SHEET_CONTOURS => {
                            serde_json::from_value::<crate::TerrainsSheetContoured>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_TERRAINS_SECTIONCHECK => {
                            serde_json::from_value::<crate::TerrainsSectionChecked>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_TERRAINS_ZEROLINE => {
                            serde_json::from_value::<crate::TerrainsZeroLined>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_TERRAINS_CUTFILL => {
                            serde_json::from_value::<crate::TerrainsCutFilled>(output.clone())
                                .map(|_| ())
                        }
                        crate::FILE_IMPORT_LANDXML => {
                            serde_json::from_value::<crate::LandxmlImported>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_ALIGNMENTS_CREATE | crate::ROAD_ALIGNMENTS_EDIT => {
                            serde_json::from_value::<crate::AlignmentsWritten>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_ALIGNMENTS_QUERY => {
                            serde_json::from_value::<crate::AlignmentsQueried>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_ALIGNMENTS_LABEL => {
                            serde_json::from_value::<crate::AlignmentsLabelled>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_PROFILES_CREATE | crate::ROAD_PROFILES_EDIT => {
                            serde_json::from_value::<crate::ProfilesWritten>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_PROFILES_QUERY => {
                            serde_json::from_value::<crate::ProfilesQueried>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_PROFILES_DRAW => {
                            serde_json::from_value::<crate::ProfilesDrawn>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_PROFILES_SIGHT => {
                            serde_json::from_value::<crate::ProfilesSightRead>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_PROFILES_PROPOSE => {
                            serde_json::from_value::<crate::ProfileProposed>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_ALIGNMENTS_CHECK => {
                            serde_json::from_value::<crate::AlignmentsChecked>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_PROFILES_CHECK => {
                            serde_json::from_value::<crate::RoadProfilesChecked>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_PROFILES_LENGTHS => {
                            serde_json::from_value::<crate::RoadProfileLengths>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_TEMPLATES_EDIT => {
                            serde_json::from_value::<crate::TemplatesWritten>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_SECTIONS_EDIT => {
                            serde_json::from_value::<crate::SectionsWritten>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_SECTIONS_QUERY => {
                            serde_json::from_value::<crate::SectionsQueried>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_SECTIONS_DRAW => {
                            serde_json::from_value::<crate::SectionsDrawn>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_SUPERELEVATIONS_EDIT => {
                            serde_json::from_value::<crate::SuperelevationsWritten>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_SUPERELEVATIONS_QUERY => {
                            serde_json::from_value::<crate::SuperelevationsQueried>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_SUPERELEVATIONS_CHECK => {
                            serde_json::from_value::<crate::SuperelevationsChecked>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_SUPERELEVATIONS_WIDEN => {
                            serde_json::from_value::<crate::SuperelevationsWidened>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_SUPERELEVATIONS_TRANSITIONS => {
                            serde_json::from_value::<crate::SuperelevationsTransitioned>(output.clone())
                                .map(|_| ())
                        }
                        crate::ROAD_CORRIDORS_EDIT => {
                            serde_json::from_value::<crate::CorridorsWritten>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CORRIDORS_QUERY => {
                            serde_json::from_value::<crate::CorridorsQueried>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CORRIDORS_DRAW => {
                            serde_json::from_value::<crate::CorridorsDrawn>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CORRIDORS_WATER => {
                            serde_json::from_value::<crate::CorridorsWatered>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CORRIDORS_DAYLIGHTS => {
                            serde_json::from_value::<crate::CorridorDaylights>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CORRIDORS_PROTECTED => {
                            serde_json::from_value::<crate::CorridorProtected>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CORRIDORS_PREFLIGHT => {
                            serde_json::from_value::<crate::CorridorPreflight>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_EDIT => {
                            serde_json::from_value::<crate::VolumesWritten>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_QUERY => {
                            serde_json::from_value::<crate::VolumesQueried>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_APPROVE => {
                            serde_json::from_value::<crate::VolumesWritten>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_UNCERTAINTY => {
                            serde_json::from_value::<crate::VolumeUncertaintyReport>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_PROGRESS => {
                            serde_json::from_value::<crate::VolumeProgressReport>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_COMPARE => {
                            serde_json::from_value::<crate::VolumesCompared>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_LEDGER => {
                            serde_json::from_value::<crate::VolumeLedgerRead>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_STATES => {
                            serde_json::from_value::<crate::VolumeStatesRead>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_PROFILES_IMPACT => {
                            serde_json::from_value::<crate::ProfileImpact>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_ALIGNMENTS_IMPACT => {
                            serde_json::from_value::<crate::AlignmentImpact>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_VOLUMES_HISTORY => {
                            serde_json::from_value::<crate::VolumeHistoryRead>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_HAULS_EDIT => {
                            serde_json::from_value::<crate::HaulsWritten>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_HAULS_QUERY => {
                            serde_json::from_value::<crate::HaulsQueried>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_HAULS_PLAN => {
                            serde_json::from_value::<crate::HaulPlanned>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_HAULS_APPROVE => {
                            serde_json::from_value::<crate::HaulsWritten>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_HAULS_NETWORK => {
                            serde_json::from_value::<crate::HaulNetworkRead>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_HAULS_TRUCKS => {
                            serde_json::from_value::<crate::HaulTrucked>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_GUARDRAILS_EDIT => {
                            serde_json::from_value::<crate::GuardrailsWritten>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_GUARDRAILS_QUERY => {
                            serde_json::from_value::<crate::GuardrailsQueried>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_GUARDRAILS_DRAW => {
                            serde_json::from_value::<crate::GuardrailsDrawn>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CULVERTS_EDIT => {
                            serde_json::from_value::<crate::CulvertsWritten>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CULVERTS_QUERY => {
                            serde_json::from_value::<crate::CulvertsQueried>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CULVERTS_DRAW => {
                            serde_json::from_value::<crate::CulvertsDrawn>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CULVERTS_HYDRAULICS => {
                            serde_json::from_value::<crate::CulvertHydraulicsRead>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_CULVERTS_CLOSURE => {
                            serde_json::from_value::<crate::CulvertClosureRead>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_PLATFORMS_EDIT => serde_json::from_value::<crate::PlatformsWritten>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_FROM_CLOUD => serde_json::from_value::<crate::TerrainsFromCloudWritten>(output.clone()).map(|_| ()),
                        crate::ROAD_DRAINAGE_QUERY => serde_json::from_value::<crate::DrainageQueried>(output.clone()).map(|_| ()),
                        crate::ROAD_DRAINAGE_DRAW => serde_json::from_value::<crate::DrainageDrawn>(output.clone()).map(|_| ()),
                        crate::ROAD_SWEPT_QUERY => serde_json::from_value::<crate::SweptQueried>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_ANALYSIS => serde_json::from_value::<crate::TerrainsAnalysed>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_COMPARE => serde_json::from_value::<crate::TerrainsCompared>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_QUALITY => serde_json::from_value::<crate::TerrainsQualityRead>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_REPAIR => serde_json::from_value::<crate::TerrainsRepaired>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_COVERAGE => serde_json::from_value::<crate::TerrainsCovered>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW => serde_json::from_value::<crate::TerrainsCoverageDrawn>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_DENSITY => serde_json::from_value::<crate::TerrainsDensityRead>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_STOCKPILE => serde_json::from_value::<crate::TerrainsStockpiled>(output.clone()).map(|_| ()),
                        crate::ROAD_TERRAINS_SPARSE_DRAW => serde_json::from_value::<crate::TerrainsCoverageDrawn>(output.clone()).map(|_| ()),
                        crate::ROAD_SWEPT_DRAW => serde_json::from_value::<crate::SweptDrawn>(output.clone()).map(|_| ()),
                        crate::ROAD_PLATFORMS_QUERY => serde_json::from_value::<crate::PlatformsQueried>(output.clone()).map(|_| ()),
                        crate::ROAD_PLATFORMS_DRAW => serde_json::from_value::<crate::PlatformsDrawn>(output.clone()).map(|_| ()),
                        crate::ROAD_WALLS_EDIT => {
                            serde_json::from_value::<crate::RetainingWallsWritten>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_WALLS_QUERY => {
                            serde_json::from_value::<crate::RetainingWallsQueried>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_WALLS_DRAW => {
                            serde_json::from_value::<crate::RetainingWallsDrawn>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_PAY_EDIT => serde_json::from_value::<crate::PayWritten>(output.clone()).map(|_| ()),
                        crate::ROAD_PAY_QUERY => serde_json::from_value::<crate::PayQueried>(output.clone()).map(|_| ()),
                        crate::ROAD_ROUTES_FIND => serde_json::from_value::<crate::RouteFound>(output.clone()).map(|_| ()),
                        crate::ROAD_ROUTES_DRAW => serde_json::from_value::<crate::RoutesDrawn>(output.clone()).map(|_| ()),
                        crate::ROAD_JUNCTIONS_EDIT => serde_json::from_value::<crate::JunctionsWritten>(output.clone()).map(|_| ()),
                        crate::ROAD_JUNCTIONS_QUERY => serde_json::from_value::<crate::JunctionsQueried>(output.clone()).map(|_| ()),
                        crate::ROAD_JUNCTIONS_CLEARANCE => serde_json::from_value::<crate::JunctionsCleared>(output.clone()).map(|_| ()),
                        crate::ROAD_JUNCTIONS_TIE => serde_json::from_value::<crate::JunctionsTied>(output.clone()).map(|_| ()),
                        crate::ROAD_JUNCTIONS_OVERLAP => serde_json::from_value::<crate::JunctionsOverlapped>(output.clone()).map(|_| ()),
                        crate::ROAD_PROJECT_QUERY => serde_json::from_value::<crate::RoadProjectTree>(output.clone()).map(|_| ()),
                        crate::ROAD_PROJECT_GRAPH => serde_json::from_value::<crate::RoadProjectGraph>(output.clone()).map(|_| ()),
                        crate::ROAD_PROJECT_REBUILD => serde_json::from_value::<crate::RoadProjectRebuilt>(output.clone()).map(|_| ()),
                        crate::ROAD_PROJECT_PRODUCE => serde_json::from_value::<crate::RoadProjectProduced>(output.clone()).map(|_| ()),
                        crate::ROAD_PROJECT_EDIT => serde_json::from_value::<crate::RoadProjectEdited>(output.clone()).map(|_| ()),
                        crate::MAP_WORK_QUERY => serde_json::from_value::<crate::MapWorkTree>(output.clone()).map(|_| ()),
                        crate::MAP_WORK_EDIT => serde_json::from_value::<crate::MapWorkEdited>(output.clone()).map(|_| ()),
                        crate::MAP_WORK_RESTORE => serde_json::from_value::<crate::MapWorkRestored>(output.clone()).map(|_| ()),
                        crate::CAD_CRS_FACTORS => serde_json::from_value::<crate::CrsFactorsRead>(output.clone()).map(|_| ()),
                        crate::MAP_FEATURES_CODE => serde_json::from_value::<crate::FeaturesCoded>(output.clone()).map(|_| ()),
                        crate::CAD_SURVEY_BASELINES => serde_json::from_value::<crate::SurveyBaselinesAdjusted>(output.clone()).map(|_| ()),
                        crate::CAD_ENTITIES_GRADE => serde_json::from_value::<crate::EntitiesGraded>(output.clone()).map(|_| ()),
                        crate::CAD_ENTITIES_RAISE => serde_json::from_value::<crate::EntitiesRaised>(output.clone()).map(|_| ()),
                        crate::CAD_ENTITIES_DRAPE => serde_json::from_value::<crate::EntitiesDraped>(output.clone()).map(|_| ()),
                        crate::CAD_TOPOLOGY_CLEAN => serde_json::from_value::<crate::TopologyCleaned>(output.clone()).map(|_| ()),
                        crate::CAD_PARCELS_CHECK => serde_json::from_value::<crate::ParcelsChecked>(output.clone()).map(|_| ()),
                        crate::CAD_PARCELS_BOUNDARIES => serde_json::from_value::<crate::ParcelsBounded>(output.clone()).map(|_| ()),
                        crate::CAD_PARCELS_RECORD => serde_json::from_value::<crate::ParcelsRecorded>(output.clone()).map(|_| ()),
                        crate::CAD_PARCELS_RECORDS => serde_json::from_value::<crate::ParcelsRecordsListed>(output.clone()).map(|_| ()),
                        crate::CAD_PARCELS_APPROVE => serde_json::from_value::<crate::ParcelsApproved>(output.clone()).map(|_| ()),
                        crate::CAD_ASBUILT_REVIEW => serde_json::from_value::<crate::AsbuiltReviewed>(output.clone()).map(|_| ()),
                        crate::CAD_PARCELS_WITHDRAW => serde_json::from_value::<crate::ParcelsWithdrawn>(output.clone()).map(|_| ()),
                        crate::CAD_PIPES_CHECK => serde_json::from_value::<crate::PipesChecked>(output.clone()).map(|_| ()),
                        crate::CAD_PIPES_PROFILE => serde_json::from_value::<crate::PipesProfiled>(output.clone()).map(|_| ()),
                        crate::CAD_ASBUILT_CHECK => serde_json::from_value::<crate::AsbuiltChecked>(output.clone()).map(|_| ()),
                        crate::CAD_FIELD_PACKAGE => serde_json::from_value::<crate::FieldPackaged>(output.clone()).map(|_| ()),
                        crate::CAD_ZONING_CHECK => serde_json::from_value::<crate::ZoningChecked>(output.clone()).map(|_| ()),
                        crate::CAD_ZONING_SYMBOLIZE => serde_json::from_value::<crate::ZoningSymbolized>(output.clone()).map(|_| ()),
                        crate::CAD_ZONING_DRAFT => serde_json::from_value::<crate::ZoningDrafted>(output.clone()).map(|_| ()),
                        crate::CAD_SCENARIOS_IMPACT => serde_json::from_value::<crate::ScenariosImpacted>(output.clone()).map(|_| ()),
                        crate::CAD_SCENARIOS_RECORD => serde_json::from_value::<crate::ScenariosRecorded>(output.clone()).map(|_| ()),
                        crate::CAD_READJUSTMENT_DISTRIBUTE => serde_json::from_value::<crate::ReadjustmentDistributed>(output.clone()).map(|_| ()),
                        crate::CAD_READJUSTMENT_SCOPE => serde_json::from_value::<crate::ReadjustmentScoped>(output.clone()).map(|_| ()),
                        crate::CAD_READJUSTMENT_SHEETS => serde_json::from_value::<crate::ReadjustmentSheetsMade>(output.clone()).map(|_| ()),
                        crate::CAD_ORTHOPHOTO_CHECK => serde_json::from_value::<crate::OrthophotoChecked>(output.clone()).map(|_| ()),
                        crate::CAD_POINTCLOUDS_REGISTER => serde_json::from_value::<crate::PointCloudsRegistered>(output.clone()).map(|_| ()),
                        crate::CAD_POINTCLOUDS_NETWORK => serde_json::from_value::<crate::PointCloudsNetworked>(output.clone()).map(|_| ()),
                        crate::CAD_POINTCLOUDS_PLACE => serde_json::from_value::<crate::PointCloudsPlaced>(output.clone()).map(|_| ()),
                        crate::CAD_ORTHOPHOTO_COMPARE => serde_json::from_value::<crate::OrthophotoCompared>(output.clone()).map(|_| ()),
                        crate::CAD_ORTHOPHOTO_SEAMLINES => serde_json::from_value::<crate::OrthophotoSeamlinesBuilt>(output.clone()).map(|_| ()),
                        crate::CAD_ORTHOPHOTO_NETWORK => serde_json::from_value::<crate::OrthophotoNetworkBuilt>(output.clone()).map(|_| ()),
                        crate::CAD_ORTHOPHOTO_FOOTPRINT => serde_json::from_value::<crate::OrthophotoFootprintBuilt>(output.clone()).map(|_| ()),
                        crate::CAD_POINTCLOUDS_OVERLAP => serde_json::from_value::<crate::PointCloudsOverlapped>(output.clone()).map(|_| ()),
                        crate::MAP_SURVEY_QUERY => serde_json::from_value::<crate::MapSurveyTree>(output.clone()).map(|_| ()),
                        crate::MAP_SURVEY_NETWORK => serde_json::from_value::<crate::MapSurveyNetworked>(output.clone()).map(|_| ()),
                        crate::MAP_FEATURES_RECONCILE => serde_json::from_value::<crate::FeaturesReconciled>(output.clone()).map(|_| ()),
                        crate::MAP_SURVEY_EDIT => serde_json::from_value::<crate::MapSurveyEdited>(output.clone()).map(|_| ()),
                        crate::ROAD_JUNCTIONS_DRAW => serde_json::from_value::<crate::JunctionsDrawn>(output.clone()).map(|_| ()),
                        crate::ROAD_STAKEOUT_QUERY => {
                            serde_json::from_value::<crate::StakeoutQueried>(output.clone()).map(|_| ())
                        }
                        crate::FILE_EXPORT_LANDXML => {
                            serde_json::from_value::<crate::LandxmlExported>(output.clone()).map(|_| ())
                        }
                        crate::FILE_EXPORT_IFC => {
                            serde_json::from_value::<crate::IfcExported>(output.clone()).map(|_| ())
                        }
                        crate::CAD_ARCH_IFC => {
                            serde_json::from_value::<crate::IfcBuildingExported>(output.clone()).map(|_| ())
                        }
                        crate::FILE_EXPORT_GLTF => {
                            serde_json::from_value::<crate::GltfExported>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_STAKEOUT_SEND => {
                            serde_json::from_value::<crate::StakeoutSent>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_SHEETS_PLAN | crate::ROAD_SHEETS_SECTIONS => {
                            serde_json::from_value::<crate::RoadSheetsLaid>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_ALIGNMENTS_FIT => {
                            serde_json::from_value::<crate::AlignmentFitted>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_PROFILES_FIT => {
                            serde_json::from_value::<crate::ProfileFitted>(output.clone()).map(|_| ())
                        }
                        crate::ROAD_ALIGNMENTS_IMPORT => {
                            serde_json::from_value::<crate::AlignmentsImported>(output.clone()).map(|_| ())
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
                        crate::CAD_TOPOLOGY_REPAIR => {
                            serde_json::from_value::<crate::TopologyRepaired>(output.clone())
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
                        crate::CAD_SHEETS_DETAILS => {
                            serde_json::from_value::<crate::SheetsDetailed>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_PLOTSTYLES_IMPORT => {
                            serde_json::from_value::<crate::PlotStylesImported>(output.clone())
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
                        crate::CAD_ARCH_RECOGNIZE => {
                            serde_json::from_value::<crate::ArchRecognized>(output.clone())
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
                            serde_json::from_value::<crate::ElevationsQueried>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_SECTIONS_EDIT => {
                            serde_json::from_value::<crate::BuildingSectionsEdited>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_SECTIONS_QUERY => {
                            serde_json::from_value::<crate::BuildingSectionsQueried>(output.clone())
                                .map(|_| ())
                        }
                        crate::CAD_OPENINGS_QUERY => {
                            serde_json::from_value::<crate::OpeningsQueried>(output.clone())
                                .map(|_| ())
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
