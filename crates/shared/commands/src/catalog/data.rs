//! The catalog's commands of views, references, rasters, sensors, point clouds and meshes (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
        CommandDescriptor {
            id: crate::CAD_TEMPLATES_APPLY.into(),
            version: crate::CAD_TEMPLATES_APPLY_VERSION,
            title: "Mimari şablonu".into(),
            summary: "Mimari çalışma paketinin eksik katman, yazı/ölçü stili, blok, duvar stili, kat ve paftalarını ekler. \
                      settingsMode=keep birim ve çizim ölçeğini korur (GUI varsayılanı); settingsMode=template gösterilen paket ayarını (m / 1:100, yeni Mimari projesiyle aynı; unit verilirse o gösterim birimi) uygular. Koordinat sistemli projede birim metre kalır; template kipi bu projede başka birimi unit_needs_local ile reddeder. Model koordinatlarını yeniden ölçeklemez. \
                      Açık kiplerde mevcut tanımlar, fontlar ve katman durumları korunur; planın review alanı önce/sonra birim-ölçek/mahal yöntemi ile kimlik/ad eşleşmesinde korunacak parçaları verir. Tüm paket ayarlar ve paftalar dahil tek undo adımıdır; hata kısmi yazmaz. keep ile unit birlikte template_settings_conflict olur. \
                      settingsMode alanı yoksa eski v1 davranışı sürer: boş çizime cm (unit yalnız mm/cm), nesneli çizimde birimi koruma uyarısı, her çizime 1:50 (paketin kendisi m / 1:100 olsa da); ayarlar ve pafta kitabı bu eski çağrıda undo dışında. Eski otomasyonu korumak içindir; yeni çağrılar açık kip kullanmalı. \
                      Paket: mimari-ofis standardı, KentOS blok kitaplığı ve duvar stilleri, iki kat, A-101/A-301 1:100 ve A-501 1:20 A3 paftalar. Çıktı eklenen adları sayar; plan yazmaz. expectedRevision değişmişse conflict olur."
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
                      Profil kurumun kendi adlarıyla da uygulanır: name yerleşik olmayan bir ad verilirse profil çizimin “profiller” klasöründeki dosyasından (drawing çizimin dosyası) okunur; mapping (ya da projenin eşlemesinin adı, mappingName) verilirse profil o eşlemeden okunur: kurumun katman yolları, alan adları ve kodlarıyla bulunur ve eklenir, hiçbir şey kendiliğinden yeniden adlandırılmaz (unknown_mapping, invalid_profile_mapping). \
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
                      Profil, Veri profilini uygula gibi, çizimin klasöründeki bir dosyadan (name, drawing) ve bir eşlemeden (mapping, mappingName) okunabilir: bulgular kurumun adlarıyla gelir, raporda mapping eşlemenin adını söyler."
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
                      Dosya biçimiyle tanınır: kentos.data-profile bir veri profili, kentos.profile-mapping bir eşlemedir; başka biçim unknown_profile_format, 1'den başka biçim sürümü unsupported_profile_version, okunmayan JSON ya da tutmayan profil invalid_profile, tutmayan eşleme invalid_profile_mapping ile reddedilir (eşleme profiliyle denetlenir: sınıflar, alanlar, kodlar profilde olmalı, kurumdaki adlar Türkçe katlanmış olarak çakışmamalı; names, yani ad biçimi lowerCamelCase ya da upperSnakeCase, eşlemenin yazmadığı alan adlarını yazar, onunla sınıfsız eşleme de geçerlidir, biçimin yazamadığı ad eşlemeyi geçersiz kılar). \
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
                      Katmanı, alanı ya da değer listesi çizimde olmayan adım atlanır ve söylenir (migration_step_skipped); zaten uygulanmış adım sessiz atlanır, kilitli katman atlanır (profile_layer_skipped). Hepsi tek geri alma adımıdır (“Profil göçü”, değer listeleri dahil); yalnız katman adları adımın dışındadır: katmanın adı çizimin bayrağıdır, hiçbir katman yeniden adlandırması geri alınmaz, geri alma alanları, değerleri ve listeleri geri verir. layer_refused'ta hiçbir şey yazılmaz. Göç yeni sürümün getirdiklerini eklemez: Veri profilini uygula ekler, Veri profilini denetle verinin hâlâ ayrıldığı yeri söyler; önce göç, sonra uygula. \
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
        // Katalog geçiş raporu (plan/map-cad.md MAP-04, docs/adr/4940).
        CommandDescriptor {
            id: crate::CAD_PROFILES_COMPARE.into(),
            version: crate::CAD_PROFILES_COMPARE_VERSION,
            title: "Katalog geçiş raporu".into(),
            summary: "Çizimin verisinin bir nesne kataloğundan (veri profili) ötekine geçişinin raporu; hiçbir şey yazmaz. Eski katalog from (yerleşik ya da projenin profilinin adı) ya da fromProfile (profilin kendisi), yeni katalog to ya da toProfile ile verilir. \
                      Her eski sınıf yeni katalogda önce kalıcı kimliğiyle (aynı id: same ya da katmanı, geometrisi, kodları ya da alanları değişmişse changed), yoksa saha kodlarıyla izlenir: kodlarının hepsi tek bir sınıfa gidiyorsa renamed, o sınıf başka bir eski sınıfın da kodlarını alıyor ya da bir eski sınıfın kimliğini taşıyorsa merged, kodları iki ve daha çok sınıfa gidiyorsa split, ne kimliği ne kodu varsa retired; hiçbir eski sınıfın gitmediği yeni sınıf added. Addan ya da konumdan tahmin yapılmaz. \
                      Her sınıf satırı eski sınıfın katmanındaki nesneleri sayar (kalıcı kimlikleri en çok 1000); retired ve split sınıfın, geometrisi değişen sınıfın nesneleri karar ister (decide). Tek sınıfa giden sınıfın alanları karşılaştırılır: removed (değeri olan nesneler taşınamaz), kindChanged ve listChanged (değeri olan nesneler yeniden okunmalı), nowRequired ve zorunlu added (boş olan nesneler). Değer listelerinin kodları: removed, retired (değeri o kod olan nesneler karar ister), relabelled, added. \
                      Detay eşleştirmenin çözülmüş inceleme notu olarak kayıtlı kararları (ret dışında) sınıflarıyla izlenir: kept, follows (sınıfı yeniden adlandırılmış ya da birleşmiş), fieldDropped (verdiği alan kalkıyor), classRetired, classSplit. decide karar isteyen satırları sayar. Retler: unknown_profile (from, to), invalid_profile (fromProfile, toProfile ya da projenin dosyası). Plan aynı cevabı verir."
                .into(),
            aliases: vec!["kataloggecisi".into(), "katalogkarsilastir".into(), "profilkarsilastir".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesCompare>(),
            output: schema::<crate::ProfilesCompared>(),
            plan: Some(schema::<crate::ProfilesCompared>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Harita varlıkları kataloğundan kurumun kataloğuna geçiş".into(),
                input: json!({ "from": "harita-varliklari", "to": "belediye-harita", "drawing": "/projeler/harita/alim.kcad" }),
                output: None,
            }],
        },
        // Kataloğa taşı (plan/map-cad.md MAP-04, docs/adr/5340).
        CommandDescriptor {
            id: crate::CAD_PROFILES_CARRY.into(),
            version: crate::CAD_PROFILES_CARRY_VERSION,
            title: "Kataloğa taşı".into(),
            summary: "Çizimin verisini bir nesne kataloğundan (veri profili) ötekine, Katalog geçiş raporunun (cad.profiles.compare) sınıf izlemesiyle taşır; tek geri alma adımı, ya hep ya hiç. Eski katalog from ya da fromProfile, yeni katalog to ya da toProfile ile verilir; drawing projenin klasörü içindir. \
                      Aynı kalan, değişen, yeniden adlandırılan ya da birleşen sınıfın nesneleri yeni sınıfın katmanına gider; bölünen sınıfın her nesnesi kendi saha kodunu (field alanının, yoksa Kod'un baştaki büyük harfleri) taşıyan yeni sınıfa gider; kalkan sınıfın ve kodu yeni sınıflarda olmayan nesneler yerinde kalır (uyarı catalog_objects_left). Nesne alan yeni sınıfın katmanı yoksa alanlarıyla açılır, varsa eksik alanları eklenir (cad.profiles.apply'ın yazıcısı). Değerler nesneyle taşınır; hiçbir değer silinmez ya da yeniden kodlanmaz. \
                      Çıktı her nesnesi olan eski sınıfın taşınan ve kalan nesne sayısı, açılan katmanlar ve alanlar. Retler: invalid_revision, revision_conflict, unknown_profile, invalid_profile, layer_locked (nesnenin bıraktığı ya da gittiği katman kilitli), layer_refused (yeni sınıfın yolu bir gruba ya da katmanın içine çıkıyor), nothing_to_set (taşınacak nesne yok). Uyarılar: catalog_objects_left, profile_field_differs, profile_domain_differs. Plan yazmadan aynı sayıları verir."
                .into(),
            aliases: vec!["katalogatasi".into(), "katalogtasima".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesCarry>(),
            output: schema::<crate::ProfilesCarried>(),
            plan: Some(schema::<crate::ProfilesCarried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Harita varlıklarından kurumun kataloğuna taşı".into(),
                input: json!({ "from": "harita-varliklari", "to": "belediye-harita", "drawing": "/projeler/harita/alim.kcad" }),
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
            id: crate::CAD_PROFILES_NAMES.into(),
            version: crate::CAD_PROFILES_NAMES_VERSION,
            title: "Kurumdaki alan adları".into(),
            summary: "Bir veri profilinin her alanının kurumdaki adını, adın nereden geldiğini ve sorununu söyler; hiçbir şey yazmaz. \
                      Ad önce eşlemenin kendi yazdığıdır (fields, source mapping), yoksa eşlemenin ad biçimidir (names, source form), yoksa profilin adıdır (source profile). \
                      Ad biçimi: case lowerCamelCase (ilk sözcük küçük, sonrakilerin ilk harfi büyük, boşluk yok; yalnız büyük harfli iki ve daha çok harfli sözcük kısaltmadır, ilk sözcük değilse olduğu gibi kalır) ya da upperSnakeCase (her sözcük büyük harfle, “_” ile); letters keep (Türkçe harfler korunur, i/İ ve ı/I Türkçe büyük küçük harfle) ya da ascii (ç ğ ı İ ö ş ü â î û → c g i I o s u a i u; başka harf yazılamaz). Sözcükler harf ve rakam dizileridir; küçük harften ya da rakamdan sonra gelen büyük harf, bir de ardından küçük harf gelen büyük harf yeni sözcük başlatır (TUCBSNo: TUCBS, No); rakam önceki sözcükte kalır. \
                      names verilirse eşlemenin kendi biçiminin yerine okunur (eşleme yoksa yalnız biçim). Satır sorunları: unfit (biçim adı yazamıyor: harf ya da rakam yok, ASCII karşılığı olmayan harf), clash (sınıfın iki alanı aynı ada düşüyor; ikisi de söylenir). holds, eşlemenin bu biçimle tuttuğunu söyler; tutmayan eşleme Veri profilini uygula ve denetle'de invalid_profile_mapping ile reddedilir. \
                      Profil adıyla (name) ya da kendisiyle (profile), eşleme kendisiyle (mapping) ya da projenin adıyla (mappingName, drawing ile) verilir. Ret: unknown_profile, invalid_profile, unknown_mapping, invalid_profile_mapping (eşlemenin biçimi; adlar satırlarda söylenir). Biçimler kurumun verisinin uygunluğunu kanıtlamaz: kurumun kod listesindeki ad eşlemede yazılır."
                .into(),
            aliases: vec!["kurumadlari".into(), "adbicimi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesNames>(),
            output: schema::<crate::ProfileNames>(),
            plan: Some(schema::<crate::ProfileNames>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Profilin alan adlarını BÜYÜK_HARF ve ASCII ile göster".into(),
                    input: json!({ "name": "tucbs-temel", "names": { "case": "upperSnakeCase", "letters": "ascii" } }),
                    output: None,
                },
                CommandExample {
                    title: "Projenin eşlemesinin kendi biçimiyle".into(),
                    input: json!({ "name": "tucbs-temel", "mappingName": "belediye-adlari", "drawing": "/projeler/belediye/imar.kcad" }),
                    output: None,
                },
            ],
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
                      uid verilince nesnenin nerede göründüğü: katı, onu çizen görünümler (alanının kutusu nesnenin kutusuyla kesişen, tarifi katını, tadilat durumunu ve türünü dışarıda bırakmayan), onu çizen haritalar (cad.sheets.set'in shows'u), onu listeleyen çizelgeler ve onu adlandıran etiketler. Kutu kesişimidir: fazladan listelenebilir, eksik listelenmez."
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
            summary: "Başka bir çizim dosyasını dış başvuru olarak bağlar ve yönetir; her işlem tek geri alma adımı. operation.kind: attach (path mutlak .kcad ya da .pdf yolu; page PDF'in sayfası, 1'den, yoksa 1; mode attach ya da overlay; name yoksa dosyanın adı; at, scale, rotation derece; layerId yerleştirmenin katmanı): kaynağın nesneleri (resim, raster ve tablo dışında) bir blok tanımı olur, katmanları “Dış başvuru: ad” grubunun altına gelir, bir yerleştirme konur; reload kaynağı yeniden okur (yerleştirmeler aynı, kaynağın nesnelerine bağlı ölçüler ve kılavuz uçları aynı nesneleri izler, silineninki kopar); unload içeriği boşaltır; repath yeni yoldaki dosyayı bağlar ve yükler; detach tanımı, getirdiklerini, bütün yerleştirmelerini ve katman grubunu siler; bind (mode bind: katmanlar grupta kalır; insert: aynı yoldaki katmanlarla birleşir) sıradan bloğa çevirir; paths projenin arama yollarını yazar (ayar; kaynak yolunda ve göreli yolunda yoksa bu klasörlerde aranır); underlay PDF altlığının görünüşünü yazar (fade %0–80 soldurma, monochrome tek renk, plot false yazdırılmaz); resetLayers bu çizimde dış başvurunun katmanlarına yapılan değişiklikleri (renk, çizgi tipi, kalınlık tek geri alma adımı; görünürlük, kilit, dondurma ve yazdırma katman bayrağı) kaynağınkine döndürür, katman durumları (paftaların kendi görünüşü) kalır. \
                      diff: true ile reload ve repath (planı önizlemedir) değişeni söyler: parçalar yerel kimlikleriyle same, changed, added, removed; getirdiği tanımlardan gelen ve giden; parçalara bağlı ölçü, kılavuz, tarama, yazı alanı ve tablo hücreleri followers'ta (follows, breaks, unties); kopan bağ ties_break uyarısıyla söylenir. Kaynağın kalıcı kimliğini tanımadığı nesne içeriğiyle bulunur; kaynakta silinen parçanın yeri ve yerel kimliği başka nesneye geçmez. \
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
                CommandExample {
                    title: "Yeniden yüklemenin farkı (plan ile önizleme)".into(),
                    input: json!({ "operation": { "kind": "reload", "name": "plan" }, "drawing": "/proje/ofis.kcad", "diff": true }),
                    output: None,
                },
                CommandExample {
                    title: "Taşınan kaynağı içerikle bulunan adaya bağla".into(),
                    input: json!({ "operation": { "kind": "repath", "name": "plan", "path": "/arsiv/plan-tasindi.kcad" }, "diff": true }),
                    output: None,
                },
                CommandExample {
                    title: "Katmanları kaynağa döndür".into(),
                    input: json!({ "operation": { "kind": "resetLayers", "name": "plan" } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_XREFS_LIST.into(),
            version: crate::CAD_XREFS_LIST_VERSION,
            title: "Dış başvurular".into(),
            summary: "Çizimin dış başvurularını blok sırasıyla listeler; çizimi değiştirmez. Her biri: adı, blok kimliği, türü, bulunduğu yol (yolunda yoksa drawing'in klasöründen göreli yolu), durumu (loaded, unloaded, missing: dosya yok, changed: dosyanın özeti yüklenenden başka, Yeniden yükle alır), onu getiren, yerleştirme ve nesne sayısı, katman grubu, nasıl bulunduğu (path, relative, searchPath, none) ve bulunamadıysa denenen yollar; affected: true her birinin yerleştirmelerini gösteren paftaları ve adlı görünümleri de verir. review: true kaynak ağını inceler: kullanılan revizyon (digest, değiştiyse dosyanınki current), ağdaki derinlik (depth), çizimin gösterdiği (content: source kaynağın kendisi, cached kaynak yok ya da başka revizyon, son yüklenen kopya; none boşaltılmış), getirdikleri (brings), katmanlarını adlandıran katman durumları (layerStates), pafta pencerelerinin katmanları için kendi ezmeleri (viewportOverrides: pafta, harita öğesinin kimliği ve adı, katmanın ağaçtaki yolu, dondurma, yazdırma, renk, çizgi tipi, desen, kalınlık µm; hiçbir dış başvuru işlemi, Katmanları kaynağa döndür de, onlara dokunmaz) ve bulunamayan ya da değişen için izinli yerlerdeki (kayıtlı yolun, göreli yolun ve çizimin klasörü, arama yolları; alt klasörlere inilmez, en çok 512 dosya, dosya başına 512 MB) adaylar: içerik özeti aynı olan (match content: taşınmış ya da yeniden adlandırılmış) önce, aynı adlı başka içerik (match name) sonra. Aday kendiliğinden bağlanmaz; Yolu değiştir (repath) ile bağlanır. Kayıtlı mutlak yolu olmayan kaynağın yedekleri (göreli yol, arama yolları, sonunda çizimin klasöründe dosyanın adı) yalnız son yüklenen baytlarsa kaynaktır."
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
                CommandExample {
                    title: "Kaynak ağının incelemesi ve taşınan kaynağın adayları".into(),
                    input: json!({ "drawing": "/proje/ofis.kcad", "affected": true, "review": true }),
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
            summary: "Birçok GeoTIFF dosyasını tek raster gibi gösterilen ve çözümlenen sanal mozaik yapar: path'e VRT dosyası yazılır (piksel kopyalanmaz; CBS yazılımları da açar). \
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
        // Rasteri projenin sistemine çevir (docs/adr/0644 §3), held together by fixtures/commands/v1/cad.rasters.reproject.json.
        CommandDescriptor {
            id: crate::CAD_RASTERS_REPROJECT.into(),
            version: crate::CAD_RASTERS_REPROJECT_VERSION,
            title: "Rasteri projenin sistemine çevir".into(),
            summary: "Başka koordinat sistemindeki bir raster dosyasını (file; GeoTIFF ya da dünya dosyalı PNG ve JPEG, mutlak ya da drawing'in klasöründen göreli) projenin sistemine çevirir: pikselleri projenin sisteminde kuzeyi yukarıda bir ızgaraya yeniden örneklenir, output'a GeoTIFF yazılır (yoksa dosyanın yanına <ad>-epsg<kod>.tif) ve raster o dosyaya bağlı olarak katmana eklenir (layer; yoksa dosyanın adıyla yeni katman en üstte), tek geri alma adımı “Rasteri projenin sistemine çevir”. \
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
                      Pencerenin sınırı GeoPackage karo okuyucularının kuralıyla pikselin tamına yuvarlanır (başlangıç (min − küme başlangıcı) / piksel'in yuvarlağı, boy (en büyük − en küçük) / piksel + 0,5'in tamı) ve ızgaradan taşan yer kesilir. Karolar PNG olmalı ve matrisin karo boyunda olmalı (8 bit gri, gri+alfa, RGB, RGBA ve paletli); JPEG, WebP ve 8 bitten fazla PNG okunmaz (tile_format_unsupported); raster en çok 25 milyon piksel olur (gpkg_tiles_too_large). \
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
            summary: "Blok ve dış başvuru yerleştirmelerini kırpar; tek geri alma adımı “Kırp”. operation.kind: set (boundary çizimde: rectangle a, b köşeleriyle ya da polygon points; inverted dışını gösterir; frame hide, show: yalnız ekranda çerçeve, plot: çerçeve içerikle çizilir ve yazdırılır), remove (sınır kalkar), frame (çerçevenin görünüşü), invert (içi ve dışı yer değiştirir). Sınır yerleştirmenin tanımında saklanır: yerleştirme taşınınca, döndürülünce, ölçeklenince onunla gider. Dolu alanlar alan olarak, çizgiler ve kapalı eğrilerin sınırları yol olarak kesilir; yazı, nokta ve tarama kutusunun ortasına göre ya bütün ya hiç. Kilitli katmandakiler atlanır ve sayılır. Retler: no_entities, invalid_uid, entity_not_found, not_an_insert, layer_locked, invalid_clipping (en az 3, en çok 10 000 köşe, alanı olan, kendini kesmeyen), not_clipped, invalid_revision, revision_conflict; uyarılar layer_locked, clip_empty. Plan yazmadan ne olacağını söyler."
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
                      columns sütunlardır (field: number, type, form, width, height, sill, head, wall, level, phase, count, area, perimeter, volume, name, usage, tag, place, length, thickness, attr; title başlık, attr öznitelik adı); verilmezse türün sütunları. filter kat, tadilat durumu, biçim (yalnız açıklıkta) ve katmanla süzer; sort sütunlara göre sıralar (yoksa çizimin sırası); group aynı satırları birleştirir (adet, alan, çevre, hacim ve uzunluk toplanır); total toplam satırı ekler. \
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
            id: crate::CAD_EXTRACT_RUN.into(),
            version: crate::CAD_EXTRACT_RUN_VERSION,
            title: "Veri çıkarımı".into(),
            summary: "Bir veri çıkarımı tarifini (recipe) ya da bir çıkarım tablosunun (table) tarifini çalıştırır; hiçbir şey yazmaz. Tarif açık çizimi (current, varsayılan evet) ve salt okunur başka çizimleri (drawings: .kcad ya da KentOS JSON yolları, base klasöründen) okur; türler (kinds: insert, line, polyline, polygon, circle, arc, ellipse, spline, hatch, dimension, text, point, leader; yoksa insert), blok adları ve katman adlarıyla süzer. \
                      Sayım kapsamı (scope) açıktır: nested iç içe blokların içine bakar (her iç yerleştirme ve parça kendi yoluyla sayılır), arrays items her dizi öğesini, whole diziyi bir Dizi olarak sayar; hidden görünürlük durumunun gizlediği parçaları ve gizli/dondurulmuş katmanlardakileri de sayar; xrefs dış referansın içine bakar, tarifin ayrıca okuduğu çizime giden referansa asla bakmaz (bir kez sayılır). Her oluşumun yolu (çizim#kimlik/yerel kimlik, dış referansta /x:kaynak kimliği) bir kez sayılır ve members'ta izlenir. \
                      Sütunlar (columns): drawing, kind, block, layer, variant, attribute (tag), count (adet), length (m/cm/mm/km), area (m2/donum/ha), measured (ölçünün gerçek uzunluğu), shown (ölçünün yazılan metni: sözcüktür, toplanmaz), overridden, computed (önceki bir miktar sütunu × noktalı ondalık katsayı, birimiyle), joined (join ile eşlenen dış tablonun sütunu). group yazı sütunları aynı satırları birleştirip miktarları toplar; sort sütunlara göre sıralar; total yuvarlanmamış değerlerden Toplam satırı yazar; hücreler en sonda yuvarlanır. \
                      join: CSV/XLSX/ODS dosyasının key başlıklı sütunu tarifin on sütunuyla eşlenir; eşlenmeyen anahtarlar notes.unmatched, kullanılmayanlar notes.unused. table verilince stale tablonun kaynaklarının değişip değişmediğini söyler. drawingPath açık çizimin kendi dosyasıdır: tarif onu okursa reddedilir. Okunan dosyalar yalnız okunur, özetleri read'de.".into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExtractRun>(),
            output: schema::<crate::ExtractRan>(),
            plan: Some(schema::<crate::ExtractRan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki çizimden iç içe bloklarla parça listesi, dış açıklama tablosuyla".into(),
                    input: json!({ "recipe": { "name": "Mobilya listesi", "drawings": ["kat2.kcad"], "kinds": ["insert"], "scope": { "nested": true, "xrefs": true }, "columns": [{ "label": "Blok", "value": { "of": "block" } }, { "label": "Açıklama", "value": { "of": "joined", "field": "Açıklama" } }, { "label": "Adet", "value": { "of": "count" } }], "group": true, "sort": [{ "column": 0 }], "total": true, "join": { "path": "aciklama.csv", "key": "Blok", "on": 0 } }, "base": "/proje" }),
                    output: None,
                },
                CommandExample {
                    title: "Duvar uzunluğu ve ölçülerin gerçek değeri".into(),
                    input: json!({ "recipe": { "name": "Metraj", "kinds": ["line", "dimension"], "columns": [{ "label": "Katman", "value": { "of": "layer" } }, { "label": "Uzunluk", "value": { "of": "length" }, "decimals": 2 }, { "label": "Gerçek ölçü", "value": { "of": "measured" } }, { "label": "Yazılan", "value": { "of": "shown" } }], "group": true, "total": true } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_EXTRACT_TABLE.into(),
            version: crate::CAD_EXTRACT_TABLE_VERSION,
            title: "Veri çıkarımı tablosu".into(),
            summary: "cad.extract.run'ın tarifini sol üst köşesi at olan bir tablo olarak yazar; tek geri alma adımı “Veri çıkarımı tablosu”. Tablo tarifini, okunan kaynakların özetlerini, her veri satırının anahtarını ve son çıkarılan hücreleri saklar. \
                      Tabloyu güncelle (cad.tables.edit refresh) tarifi yeniden çalıştırır: kaynağı değişmeyen elle düzeltilmiş hücre ve kullanıcının eklediği sütunlar, biçimler, formüller, alanlar ve satır yükseklikleri satır anahtarıyla taşınır; toplamın tüm veri satırlarını kapsayan formül aralığı büyür ya da küçülür. Kaynağı da değişen elle düzeltme, kullanıcı verisi olan ve kaynaktan kalkan satır ya da elle eklenen satır tabloyu olduğu gibi bırakır (refresh_kept); overrides replace kaynağı yazar ve sayar. \
                      Satır bulamayan tarif tablo yazmaz (extraction_empty). Katman verilmezse etkin katman; height yazı yüksekliği (m; verilmezse projenin tablo yüksekliği çizim ölçeğinde). Miktar sütunları sağa yaslanır.".into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExtractTable>(),
            output: schema::<crate::ExtractTabled>(),
            plan: Some(schema::<crate::ExtractTablePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Blok adetleri tablosu".into(),
                input: json!({ "recipe": { "name": "Bloklar", "columns": [{ "label": "Blok", "value": { "of": "block" } }, { "label": "Adet", "value": { "of": "count" } }], "group": true, "total": true }, "at": { "x": 20.0, "y": 0.0 } }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_SCHEDULES_EDIT.into(),
            version: crate::CAD_SCHEDULES_EDIT_VERSION,
            title: "Çizelgeyi düzenle".into(),
            summary: "Bir çizelgeyi değiştirir; tek geri alma adımı “Çizelgeyi düzenle”. operation: cell (row tablonun satırı, 0 başlıktır; column sütun; value hücrenin yazısı) satırın nesnelerinin kendi alanına yazılır: numara, tür, genişlik, denizlik, lento, ad, kullanım, etiket, kalınlık ve öznitelik düzenlenir, uzunluk projenin biriminde okunur, boş yazı isteğe bağlı alanı kaldırır; \
                      adet, alan, çevre, hacim, yükseklik, uzunluk, duvar, kat, tadilat durumu, konum ve biçim hesaplanır (cell_computed); birden çok nesneli gruplu satırın numarası yazılmaz (schedule_group_number). set sütunları, süzgeci (null kaldırır), sıralamayı, grubu ve toplamı değiştirir. \
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
    ]
}
