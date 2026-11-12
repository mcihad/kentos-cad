//! The catalog's commands of GIS data, forms, sources and sheets (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
        // Topolojiyi denetle (docs/adr/0589), held together by fixtures/commands/v1/cad.topology.check.json.
        CommandDescriptor {
            id: crate::CAD_TOPOLOGY_CHECK.into(),
            version: crate::CAD_TOPOLOGY_CHECK_VERSION,
            title: "Topolojiyi denetle".into(),
            summary: "Projenin topoloji kurallarını (rules: bazılarının kimlikleri; verilmezse hepsi) çizimde çalıştırır; çizimi değiştirmez. \
                      Bağlı katmanın alan sınırlı parçasında alanın çevresindeki bant kaynağından okunur (dosyanın katmanın son eşleştiği sürümünden, veritabanının bugünkü hâlinden) ve komşu sayılır: parçanın nesnesiyle dışarıdaki satır arasındaki bulgu bulunur, dışarıdakilerin kendi aralarındaki bulgu parçanın değildir, verilmez. \
                      Komşular tam okunamazsa (dosya yok ya da başka sürümde, veritabanına ulaşılamıyor, parçanın nesnesi bandın dışına uzanıyor) complete false olur, incomplete nedenini söyler ve topology_incomplete uyarısı verilir: o katman temiz sayılmaz. \
                      Her bulgu kalıcı anahtarıyla (key), kuralıyla, sorunuyla, nesnelerinin kalıcı kimlikleriyle (dışarıdakiler outside'ta), yeri, ölçüsü, düzeltmeleri ve istisna durumuyla gelir; ilk limit tanesi (1–100 000, varsayılan 1 000) listelenir, sayılar hepsinindir. \
                      Geometri özetiyle yapılmış istisna, nesnelerin geometrisi değişince düşer: bulgu açık ve lapsed olur (exception_lapsed uyarısı); kural sürümüyle yapılmış işaret kural değişince de düşer. \
                      Her bulgunun state'i yaşam döngüsündeki yeridir: open (değerlendirilmedi), error (hata diye onaylandı, düzeltilecek), exception (bilerek bırakıldı), lapsed (işaretlendikten sonra geometrisi ya da kuralı değişti, yeniden değerlendirilmeli); reason işaretin gerekçesidir, errors onaylanmış hata sayısı. \
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
            summary: "Bulguları istisna yapar (mark), hata diye onaylar (error) ya da işaretlerini kaldırır (unmark); bulgular cad.topology.check'in verdiği kalıcı anahtarlarla (keys) ve çizimin bugünkü hâliyle anılır. \
                      İşaret gerekçeyi (reason; projenin kalite profili varsa zorunlu, yoksa reason_required), kuralın sürümünü ve istisnada kapsamı (scope: finding yerindeki bulgu, objects aynı nesneler arasındaki kuralın her bulgusu) taşır; kural değişince de düşer. \
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
                CommandExample {
                    title: "Bir bulguyu gerekçesiyle hata diye onayla".into(),
                    input: json!({ "operation": "error", "keys": ["3f9a0c12d4e5b678"], "reason": "Sayısallaştırma hatası; ölçüyle düzeltilecek" }),
                    output: None,
                },
            ],
        },
        // Topoloji onarım tarifi (docs/adr/1260 §4), held together by fixtures/commands/v1/cad.topology.repair.json.
        CommandDescriptor {
            id: crate::CAD_TOPOLOGY_REPAIR.into(),
            version: crate::CAD_TOPOLOGY_REPAIR_VERSION,
            title: "Topolojiyi onar (tarif)".into(),
            summary: "Bir onarım tarifini uygular: adımlar (steps, 1–1000) sırayla, her biri bir bulgunun kalıcı anahtarı (key) ve uygulanacak düzeltmesi (fix: subtractFirst, removeVertex …; bulgunun sunduklarından). \
                      Her adımdan önce kurallar önceki adımların bıraktığı çizimde yeniden çalışır (GRASS v.clean'in her araçtan önce topolojiyi kurması gibi); anahtar o çizimdeki bulgunun anahtarıdır. Kör “hepsini onar” yoktur: her adım bulgusunu anar. \
                      Cevap uygulanan tarifi (adım başına kural, sorun, nesneler), değişen her nesneyi (alanı ya da uzunluğu önce ve sonra, silindiyse removed, kaynak köşelerinden yerinde kalan ve taşınan sayısı, en uzak ve toplam taşınma uzaklığı, onu değiştiren adımlar), bütün taşınan köşe sayısını ve uzaklıklarını (kalite profili varsa depolama hassasiyetinin altında taşınanlar belowPrecision; yine taşınmış sayılır, yuvarlanmaz), alanların toplamındaki değişimi ve önce/sonra açık bulgu sayısını verir. Plan aynı cevabı verir, yazmaz. \
                      Hepsi ya da hiçbiri, tek geri alma adımı (Topoloji düzelt). Retler: invalid_input, invalid_revision, revision_conflict, finding_not_found (steps[i].key), repair_refused_exception (istisna olan bulgu), fix_unavailable (steps[i].fix), layer_locked (steps[i]: kilitli katmandaki nesneyi değiştiren adım), repair_beyond_acceptance (bir kaynak köşeyi kalite profilinin kabul eşiğinden uzağa taşıyan adım). İptal edilen iş hiçbir şey yazmaz."
                .into(),
            aliases: vec!["topolojionar".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::TopologyRepair>(),
            output: schema::<crate::TopologyRepaired>(),
            plan: Some(schema::<crate::TopologyRepaired>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki bulguyu sırayla onar".into(),
                    input: json!({ "steps": [
                        { "key": "3f9a0c12d4e5b678", "fix": "subtractFirst" },
                        { "key": "0a1b2c3d4e5f6071", "fix": "removeVertex" }
                    ] }),
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
        // Katmanın koordinat sistemini tanımla (docs/adr/0583 §2), held together by fixtures/commands/v1/cad.layers.crs.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_CRS.into(),
            version: crate::CAD_LAYERS_CRS_VERSION,
            title: "Katmanın koordinat sistemini tanımla".into(),
            summary: "Katmanın kaynak sistemi kaydını yazar ya da siler (srid verilmezse ya da null ise): nesnelerinin hangi koordinat sisteminden geldiği; adı, WKT2'si, EPSG eksen sırası, birimi ve kullanım alanı kayıttan. \
                      Nesneler her zaman projenin sistemindedir; atama hiçbir koordinatı değiştirmez (yanlış sistemde alınmış veriyi çevirmek için cad.layers.reproject). \
                      Katman ağacının değişikliğidir: tek geri alma adımı “Katmanın koordinat sistemini tanımla”; kayıt aynıysa hiçbir şey yazılmaz. Kilitli katmanda da çalışır, veri değişmez. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["tanimla".into(), "sistemata".into(), "assigncrs".into()],
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
        // Katmanın koordinatlarını dönüştür (docs/adr/0583 §2), held together by fixtures/commands/v1/cad.layers.reproject.json.
        CommandDescriptor {
            id: crate::CAD_LAYERS_REPROJECT.into(),
            version: crate::CAD_LAYERS_REPROJECT_VERSION,
            title: "Katmanın koordinatlarını dönüştür".into(),
            summary: "Katmanın nesnelerini verilen sistemden (from; verilmezse katmanın kaynak sistemi) projenin sistemine çevirir: projenin sisteminde sanılarak alınmış veriyi düzeltir. \
                      Kotlar ve ölçüler kalır; yalnız noktalar ve düz kenarlı çizgiler çevrilir. Çevrilemeyen nesne (sistemin ya da ızgaranın dışında; yay, daire, yazı, blok gibi türden) varsa hiçbir şey yazılmaz: crs_unreached her nedeni sayısıyla söyler, uids nesneleri verir. \
                      Katmanın kaynak sistemi from olur. Tek geri alma adımı “Katmanın koordinatlarını dönüştür”. \
                      Cevap önizlemedir, plan da aynısını verir: her nesnenin ilk noktasının şimdiki ve yeni yeri, kayması, noktalarının en küçük ve en büyük kayması; noktaları EPSG kullanım alanlarının dışına düşen nesnenin notları (outside_source_area, outside_operation_area, outside_project_area; outside_area uyarısı); bütün noktaların ortalama, en küçük ve en büyük kayması; dönüşüm zincirinin kararı (status, reasons; sınırlıysa chain_limited uyarısı). En çok 1000 nesne listelenir, notlular önce. \
                      Kilitli katman reddedilir; projenin sistemi yoksa ya da from projeninkiyse crs_mismatch. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec!["katmandonustur".into(), "izdusur".into(), "reproject".into()],
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
            summary: "Öznitelik tablosunun katmanın satırlarını ve hücrelerini koşullarla nasıl göstereceğini yazar. Her kural: koşul (condition; katmanın alanları adlarıyla, ifade dili), uygulandığı yer (field: bir alanın hücresi; verilmezse bütün satır), zemin rengi (background), yazı rengi (color, #RRGGBB) ve kalınlık (bold). \
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
            summary: "Katmanın dizinlerini yazar: her dizin (name) 1–32 alanı (fields, \
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
            summary: "Katmanın değer gruplarını yazar: her grup (name) 2–8 değer listeli alanı (fields, anahtarlarıyla) \
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
            summary: "Katmanın alt türlerini yazar: değer listeli tek değerli bir alan (field, alanın anahtarı) kayıtları türlerine ayırır; \
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
                      Bir etiket motorunun sonucu görünüme göre değişir: bu komut tek bir pencereyi sayar; ardışık küçük kaydırmalarda etiketlerin yerinde kalması masaüstünün işidir, bu komutun değil."
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
            summary: "Katmanları adlarının örüntüsüyle ve özellikleriyle bulur: name joker örüntüsüdür (* herhangi bir dizi, ? bir karakter, # rakam, @ harf, . ikisi de değil, [..] ve [~..] kümeler, ` kaçış, virgülle seçenekler, başta ~ değili; harfler Türkçe katlanır); \
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
            summary: "Katmanları kurallarla bir çizim standardının katmanlarına birleştirir: nesne tutan her katman adına uyan ilk kuralı alır (from bir ad örüntüsüdür; adı kuralın to'su olan katman standart katmanın kendisidir); \
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
            summary: "Projenin desenli çizgi tiplerini değiştirir: load bir LIN dosyasının metnindeki (lin; source dosyanın adı, her tipin kaynağı) tipleri alır, names verilmişse yalnız onları; \
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
            summary: "Projenin kendi tarama desenlerini değiştirir: load bir PAT dosyasının metnindeki (pat; source dosyanın adı, her desenin kaynağı; unit sayıların birimi: mm ya da inch kâğıtta, drawing çizimde, yoksa mm) desenleri alır, names verilmişse yalnız onları; \
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
        CommandDescriptor {
            id: crate::CAD_STANDARDS_RESOLVE.into(),
            version: crate::CAD_STANDARDS_RESOLVE_VERSION,
            title: "Proje standartlarını birleştir".into(),
            summary: "Projeye bağlı standartları değiştirmeden birleşik bir standart hazırlar. Genel kurallar ilk kaynaktan, katman ve stil tanımları ile sürüm 2 çizgi tipi, tarama ve yayımlanmış blok referanslarının her adı ilk bulunduğu kaynaktan alınır. Tanımların kaynağı, kaynak sırası ve belge revizyonu sonuçla döner. Bu sorgu çizime hiçbir şey uygulamaz; sonucu cad.standards.check veya cad.standards.apply ile ayrı kullanın. Geçersiz veya eksik standart reddedilir.".into(),
            aliases: vec![], effect: CommandEffect::Query, hosts: vec![CommandHost::Desktop], headless: true,
            requires: vec![CommandRequirement::Document], permissions: vec![], undo: CommandUndo::None,
            cost: CommandCost::Instant, input: schema::<crate::StandardsResolve>(),
            output: schema::<crate::StandardsResolved>(), plan: Some(schema::<crate::StandardsResolved>()),
            capabilities: None, examples: vec![CommandExample { title: "Bağlı standartları birleştir".into(), input: json!({}), output: None }],
        },
        // Ofis araç paletleri (docs/adr/1014), held together by fixtures/commands/v1/cad.standards.palettes.json.
        CommandDescriptor {
            id: crate::CAD_STANDARDS_PALETTES.into(),
            version: crate::CAD_STANDARDS_PALETTES_VERSION,
            title: "Ofis araç paletleri".into(),
            summary: "Projeye bağlı standartların yayımladığı araç paletlerini okur (sürüm 2 paketin resources.toolPalettes'i). Palet çizime kurulmaz, paketinden okunur; aynı palet kimliği bir kez, ilk standardınkidir. \
                      Her araç bir katalog komutu, sürümü ve tam girdisidir: cevap aracın kişisel favori anahtarını (standart:<paket>:<palet>:<araç>), tarifin özetini (komut, sürüm ve girdinin kanonik JSON'unun SHA-256'sı; özet değişince favori başka araca kaymaz) ve komutun bu çizimde girdiyi kabul edip etmediğini (ready; komutun kendi validate'i, çizimin kopyasında, hiçbir şey yazmadan) verir; etmiyorsa nedenini komutun koduyla ve sözüyle (problem). \
                      Bağlı standart yoksa paletler boştur. Ret: invalid_standard."
                .into(),
            aliases: vec!["paletler".into(), "toolpalettes".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::StandardsPalettes>(),
            output: schema::<crate::StandardsPalettesRead>(),
            plan: Some(schema::<crate::StandardsPalettesRead>()),
            capabilities: None,
            examples: vec![CommandExample { title: "Ofisin araç paletleri".into(), input: json!({}), output: None }],
        },
        // Standart kaynaklarını kur (docs/adr/1002), held together by fixtures/commands/v1/cad.standards.install.json.
        CommandDescriptor {
            id: crate::CAD_STANDARDS_INSTALL.into(),
            version: crate::CAD_STANDARDS_INSTALL_VERSION,
            title: "Standart kaynaklarını kur".into(),
            summary: "Projeye bağlı ofis standartlarının adlandırılmış kaynaklarını (çizgi tipi, tarama deseni, yazı, ölçü ve kılavuz stili; cad.standards.resolve'un ilk kaynak önceliğiyle) çizime kurar; tek geri alma adımı, adı “Standart kaynaklarını kur”. \
                      Her kaynak çizimdeki aynı adlıyla karşılaştırılır: missing (yok), same (görünüşü aynı), differs (farklı; farklı alanlar fields'ta). Varsayılan: eksik olan eklenir, farklı olan çizimdeki olarak kalır; aynı adlı farklı kaynak seçim olmadan asla ezilmez. \
                      choices: kind, name ve action — add (eksikse), replace (farklıysa yerinde; çizimdeki ad ve yerel kimlik korunur, onu kullananlar izler), keepLocal, renameIncoming (farklıysa standarttakini newName ile ekler). Hiçbir kaynak silinmez. Plan aynı cevabı çizimi değiştirmeden verir. \
                      libraries: projenin ya da ofisin .kblok blok kitaplıklarının mutlak yolları; verilen sırayla KentOS'un kitaplıklarından önce aranır, çizime yazılmaz. \
                      Retler: invalid_file_path, file_not_found, block_library_unreadable (kitaplık yolu), no_standard, invalid_standard, standard_resource_not_found, invalid_install_choice, duplicate_name, invalid_revision, revision_conflict, block_refused."
                .into(),
            aliases: vec!["standartkur".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::StandardsInstall>(),
            output: schema::<crate::StandardsInstalled>(),
            plan: Some(schema::<crate::StandardsInstalled>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Eksikleri ekle, farklı GAZ çizgi tipini yeni adla al".into(),
                input: json!({ "choices": [{ "kind": "lineType", "name": "GAZ", "action": "renameIncoming", "newName": "GAZ-OFIS" }] }),
                output: None,
            }],
        },
        // Standart bulgusunu gerekçeyle kabul et (docs/adr/1129), held together by
        // fixtures/commands/v1/cad.standards.exceptions.json.
        CommandDescriptor {
            id: crate::CAD_STANDARDS_EXCEPTIONS.into(),
            version: crate::CAD_STANDARDS_EXCEPTIONS_VERSION,
            title: "Standart bulgusunu kabul et".into(),
            summary: "cad.standards.check'in bulgularını gerekçesiyle kabul eder (operation accept) ya da kabulü geri alır (withdraw). Bulgu bugünkü denetimdeki türü, konusu ve katmanıyla anılır; standart denetimdeki gibi verilir (name ya da standard, resources) ve origin bağlı paketin kimliği ve revizyonudur (yoksa standardın adı). \
                      Kabul standardın kaynağını, revizyonunu ve bulgunun o anki beklenen ve bulunan değerini saklar; revizyon ya da çizimdeki değer değişince denetim bulguyu lapsed (yeniden değerlendirilecek) verir, kabul edilenleri accepted olarak listede tutar ve ayrı sayar. \
                      Gerekçe zorunludur (en çok 500 karakter); by ve at (kim, ne zaman) verilirse kaydedilir. Kabuller proje ayarıdır (standardExceptions): çizimi kirli yapar, geri alma adımı değildir. Eski projeyi açmak standardı uygulamaz. \
                      Retler: invalid_input, reason_required, standard_not_found, no_standard, invalid_standard, invalid_revision, revision_conflict, finding_not_found."
                .into(),
            aliases: vec!["standartistisna".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::StandardsExceptions>(),
            output: schema::<crate::StandardsExceptionsEdited>(),
            plan: Some(schema::<crate::StandardsExceptionsEdited>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "A-DUVAR'ın rengini müşteri isteğiyle kabul et".into(),
                input: json!({ "operation": "accept", "name": "mimari-ofis",
                    "findings": [{ "kind": "layerColor", "subject": "A-DUVAR", "layer": "duvar" }],
                    "reason": "Müşteri duvarların kırmızı basılmasını istedi" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_STANDARDS_CONFLICTS.into(),
            version: crate::CAD_STANDARDS_CONFLICTS_VERSION,
            title: "Bağlı standartlardaki farklı tanımları göster".into(),
            summary: "Projeye bağlı standartlarda genel kuralları ve aynı adlı katman, yazı ve ölçü stili ile sürüm 2 çizgi tipi, tarama ve yayımlanmış blok referanslarını ilk tanımla karşılaştırır. Farklı alanları ve iki kaynağın kimliğini gösterir. Standartları birleştirmez ve çizimi değiştirmez. Okunamayan bağlı belge varsa raporu reddeder.".into(),
            aliases: vec![], effect: CommandEffect::Query, hosts: vec![CommandHost::Desktop], headless: true,
            requires: vec![CommandRequirement::Document], permissions: vec![], undo: CommandUndo::None,
            cost: CommandCost::Instant, input: schema::<crate::StandardsConflicts>(),
            output: schema::<crate::StandardsConflictsReport>(), plan: Some(schema::<crate::StandardsConflictsReport>()),
            capabilities: None, examples: vec![CommandExample { title: "Bağlı standartları karşılaştır".into(), input: json!({}), output: None }],
        },
        // Standardı denetle (docs/adr/0312 §2), held together by fixtures/commands/v1/cad.standards.check.json.
        CommandDescriptor {
            id: crate::CAD_STANDARDS_CHECK.into(),
            version: crate::CAD_STANDARDS_CHECK_VERSION,
            title: "Standardı denetle".into(),
            summary: "Çizimin bir çizim standardından neyi bozduğunu söyler: standart adıyla (name: mimari-ofis gibi yerleşik bir standart) ya da kendisiyle (standard: katmanları adları, renkleri, çizgi tipleri, kalınlıkları, Yazdır'ları ve topladıkları ad örüntüleriyle; \
                      öbür katmanların ad örüntüleri, yazı tipleri, yazı ve ölçü stilleri, çizgi tipleri, nesnelerin katmana göreliği) verilir. Bulgular sırasıyla: standardın katmanları (eksik; rengi, çizgi tipi, kalınlığı, Yazdır'ı farklı), \
                      çizimin öbür katmanları (bir standart katmana ait ad, ad örüntülerine uymayan ad, izinsiz çizgi tipi), projenin yazı tipi, stillerin yazı tipleri, eksik ve standartta olmayan stiller, izinsiz kendi çizgi tipli ve kendi görünüşlü nesnelerin sayıları. \
                      resources verilirse çizgi tipi ve tarama deseninin eksik/farklı tanımları ile yayımlanmış blok referansının eksik veya farklı kimlik/sürümü de raporlanır; bunlar otomatik düzeltilmez. Her bulgu türü, konusu, katmanı, beklenen ve bulunan değeri, Türkçe iletisi ve cad.standards.apply'ın düzeltip düzeltmediğiyle gelir. Çizimi değiştirmez; plan aynı cevabı verir."
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
                      Duvar stilinin (kind wall) materialId'li bileşeni projenin malzemesinin metnini, taramasını ve kalemini alır; projede olmayan malzemeye bağlı bileşen create, update ve import'ta material_not_found ile reddedilir. \
                      Tablo stili (kind table) başlık, sütun başı ve veri hücre stilleriyle tablosunu kendi değeri olmayan yerde çizdirir: update ve rename tablo yazmaz; remove stilin tablolarının bağını tek adımda (“Tablo stili”) kaldırır. \
                      Yazı stili yeniden adlandırılınca tablo stillerinin hücre stilleri onu yeni adıyla anar."
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
                    title: "CETVEL tablo stili: veri hücreleri sağa, 2,5 mm, sütun başı 0,5 mm kenarla".into(),
                    input: json!({
                        "kind": "table",
                        "operation": "create",
                        "table": { "id": "", "name": "CETVEL", "header": { "align": "center", "borderWeight": 0.5 },
                                   "data": { "align": "right", "height": 2.5, "horizontalMargin": 1.5 } }
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
                      arch bütün mimari tercihtir: katlar, ayrıntı, alan yöntemi, tadilat görünümü, planın kesme yüksekliği (cutHeight, katın döşemesinden metre, 0,3–3; yoksa 1,10), çizgi kalınlıkları (lineGroups: byScale TS 88-23 çizgi grubu ölçeğe göre, own ya da yoksa her nesne kendi kaleminde), isteğe bağlı spaceNames/spaceUsages listeleri ve kalıcı kat kimliğine bağlı spaceNumberPrefixes. Liste değişiminde mevcut kat ve tercihleri kopyalayın; arch iç içe yama değildir. Listeler öneridir; mevcut mahal adı/numarası/kullanımı değişmez, önek yalnız açık seçimle başlangıca alınır. \
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
                      commit: kaydın ve alt kayıtlarının katmanları tek GeoPackage dosyasına bağlıysa (cad.sources.edit) aynı adımın ardından dosyaya tek SQLite işleminde yazılır (cad.sources.sync'in with'i); dosyanın reddi gönderimin reddidir: plan söyler, ne çizimde ne dosyada bir şey yazılır. Cevabın committed'ı dosyaya yazılanı verir; bağlı olmayan katman yazılmaz."
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
            summary: "Projenin değer listelerini düzenler: bir kod listesi (values) ya da aralık (min, max) projede bir kez tutulur,                       birçok katmanın alanı ona bağlanır; bağlı alanın değerleri listenin kopyasıdır ve liste değişince aynı adımda yeniden yazılır.                       create yeni liste (domainKind: uyduğu alan türü), fromField bir alanın kendi listesinden liste yapıp alanı bağlar, update listeyi                       değiştirir (türü değişmez; kayıtların tuttuğu kod listeden çıkamaz: recode ile başka koda taşınır ya da kullanımdan kaldırılır),                       remove bağlı alanı olmayan listeyi siler (detach: alanlar kopyalarını tutar), link alanı bir listeye bağlar ya da bağı çözer.                       Tek geri alma adımı “Değer listeleri”."
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
            summary: "Yerel projenin çizim birimini from'dan to'ya değiştirir (mm, cm, m, in, ft); from çizimin şimdiki birimi olmalıdır. geometry ne olacağını söyler: \
                      keep boyları korur, yalnız okuma ve yazma birimi değişir (4000 mm'lik duvar 4 m okunur, hiçbir nesne değişmez); \
                      scale sayıları korur, çizim iki birimin oranıyla 0,0'a göre ölçeklenir (4000 mm 4000 m okunur: ×1000; cm'den mm'ye ÷10). \
                      scale'de nesnelerin koordinatları, uzunlukları, yarıçapları, kotları, yazı, ölçü, kılavuz ve tablo yükseklikleri ve boyları, paragraf kutusu, eğri boyunca yazının eğrisi, tarama aralığı ve desen ölçeği, resim ve raster çerçeveleri, blok tanımları, ana görünüm ve geçerli/kayıtlı UCS başlangıçları ölçeklenir; \
                      açılar, oranlar, yerleştirmenin kendi ölçeği, kâğıt üstü boylar ve ölçüler (M) kalır. Nokta bulutlarının noktaları, pafta kitabı ve projenin toleransları değişmez; uyarı söyler. \
                      Telde geometri her durumda metredir. Birim ve ölçeklenen çizim tek geri alma adımıdır (“Birimi değiştir”). Koordinat sistemli projenin birimi sistemindir (unit_needs_local); scale'de kilitli katmanda nesne varsa hiçbir şey yazılmaz (layer_locked). \
                      Ölçeklenen UCS başlangıcı izin verilen alanın dışına çıkacaksa hiçbir şey yazılmaz (far_point). expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
                      Dosya katman onunla son eşleştiğinden beri değiştiyse yazılmaz (source_changed, conflict); geometrisi tablonun sütununa uymayan nesne geometry_mismatch (Z ve M de: Z'yi yasaklayan sütuna kotlu, isteyene kotsuz, hiçbirine kısmen kotlu nesne yazılmaz; M için de aynı); tablonun tutmadığı nesneler (yazı, ölçü) ve alanı olmayan öznitelikler çizimde kalır (export_losses). Yazma geri alma adımı değildir: dosya yazılır. \
                      read: tablonun satırlarını katmana alır, tek geri alma adımı (“Kaynaktan oku: tablo”): değişen nesnelerin görünüşü kalır, yeni satırlar eklenir, tabloda olmayanlar silinir. Katman son eşleşmeden beri değiştiyse discard olmadan okunmaz (pending_changes, conflict). \
                      PostGIS tablosu da aynı kurallarla, tek veritabanı işleminde yazılır: tablo yazarlara kilitlenir, sürümü yeniden denetlenir, kalıcı kimlikler şemanın kentos_uids tablosunda; ulaşılamayan veritabanı source_unreachable. \
                      with (yalnız write) başka bağlı katmanları aynı işlemde yazar: üst kayıt ile alt kayıtları ya hep birlikte ya hiç: her katman önce denetlenir, biri reddedilirse (source_readonly, geometry_mismatch, source_changed, relation_dangling) hiçbiri yazılmaz; katmanlar tek GeoPackage dosyasında ya da tek PostGIS veritabanında olmalıdır (veritabanında tablolar adlarının sırasıyla kilitlenir, her birinin satırları tabanına karşı denetlenir; iki kaynağa birden tek işlem olmaz, transaction_split); cevabın layers'ı her katmanın payı, relations'ı eşli ilişkilerin eşleme satırlarıdır. \
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
            summary: "Hem katmanı hem tablosu son eşleşmeden beri değişen bağlı katmanı uzlaştırır. Bağın tabanı (son eşleşmede her nesnenin katmandaki ve tablodaki özeti) ile karşılaştırılır: yalnız katmanın değiştirdiği nesne tabloya yazılır, yalnız tablonun değiştirdiği (başka bir yazılımda ya da KentOS'ta) katmana alınır, ikisinin aynı biçimde değiştirdiği uyuşur. \
                      İkisinin de değiştirdiği nesne tabana karşı alan alan birleştirilir: yalnız bir yanın değiştirdiği öznitelik ve geometri o yanınkini alır, iki yan birden farklı değiştirdiği öznitelik ya da geometri çatışmadır (iki yanın değerleriyle); nesne, tabanın alan kaydı yoksa bütün olarak çatışır (field_base_missing). Birinin silip öbürünün değiştirdiği nesne de çatışmadır. \
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
            summary: "Projenin bir ilişkisinin (cad.relations.edit) çok değerli alanı (alt kayıt birden çok üstün kimliğini “; ” ile tutar) bağlı GeoPackage'da nasıl tutuluyor: iki katman da aynı dosyanın tablolarına bağlıysa metin sütunu (text), KentOS'un ilişki temsili (mapped), dosyanın başka bir temsilli ilişkisi (foreign) ya da profile uymayan ilişkisi (broken); bağlı olmayan (unbound), ayrı kaynaklardaki (split) ve tek kimlikli (single) ilişki. \
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
                      NULL değer özniteliğin yokluğu, boş metin boş metindir; 2⁵³'ten büyük tam sayılar büyük tam sayı (bigint), dilimli ya da kesirli tarih ve saat zaman damgası (instant), BLOB ikili (binary) alanına kayıpsız gelir; KentOS'un yazdığı dosyanın belgesi UUID ve JSON sütunlarını da söyler. Her satırın kalıcı kimliği dosyanın kentos_uids'i ya da tablo ve fid'den türetilmiş kimliktir: aynı dosya ikinci kez alınmaz (entity_exists). \
                      Dosyanın koordinat sistemi projeninkinden başkaysa reproject ile projenin sistemine çevrilir (projenin datum seçimleriyle), yoksa crs_mismatch; sistem bildirmeyen dosya olduğu gibi alınır (crs_undeclared). Telde geometri metredir. Bütün alınanlar tek geri alma adımıdır (“GeoPackage: ad.gpkg”); plan dosyayı okur, hiçbir şey yazmaz. merge var olan katmanın nesneleriyle anahtar sütunuyla ya da kalıcı kimlikle birleştirir (eşleşen güncellenir, eşleşmeyen eklenir ya da atlanır, yinelenen anahtar durdurur ya da reddedilir), rejects reddedilen satırları nedenleriyle CSV'ye yazar."
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
            summary: "path'teki DWG dosyasını açık çizime alır, masaüstünün DWG içe aktar penceresi gibi, sürümü sabit OpenCADCodec (MPL-2.0) ile, ağ veya uzak dönüştürücü olmadan: her kaynak katmanın nesneleri çizimde aynı adlı katmana (büyük küçük harf ve Türkçe işaretler bir), yoksa dosyanın görünüşüyle yeni bir katmana gider; \
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
            summary: "Projenin her katı için bir pafta yapar: aynı şablondan (source; yok: sys:cad-mimari), haritası o katı çizer (below: alt kat soluk), bütün haritalar aynı zemini gösterir: seçilen katların nesnelerinin kutusu (yoksa bütün çizim), options.scale ölçeğinde ya da verilmezse sığdığı standart ölçekte; planlar kâğıtta üst üste oturur. levels: katların kimlikleri (yok: projenin bütün katları). Paftalar first'ten (yok: A-101) son rakamları artarak numaralanır, kullanılan numaralar atlanır; adı “A-101 Zemin kat planı”, çizim bilgisi numara, başlık ve Mimari › Kat planları. Bir haritası zaten gösterdiği kat atlanır; again true yeniden yapar. Plan yapılacak paftaları ve haritalarını verir; hepsi tek değişiklik, inverse geri alır. Projede olmayan kat level_not_found, options.name invalid_options, katların hepsinin paftası varsa levels_have_sheets, haritasız şablon not_fixed_map, rakamla bitmeyen first bad_drawing_info."
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
        // Detay paftaları (docs/adr/1090 §2), held together by fixtures/commands/v1/cad.sheets.details.json.
        CommandDescriptor {
            id: crate::CAD_SHEETS_DETAILS.into(),
            version: crate::CAD_SHEETS_DETAILS_VERSION,
            title: "Detay paftaları".into(),
            summary: "Aynı modelin genel 2B detaylarını kesin oranlarla paftalara yerleştirir; mimari kat verisi gerekmez. Her detay: name, bounds (model kapsamı, metre: en küçük doğu, kuzey, en büyük doğu, kuzey) ya da table {table, part} (bölünmüş model tablosunun parçası), ratio {paper, model} (1:5 = paper 1, model 5; 2:1 = paper 2, model 1), isteğe bağlı layers (katman durumu {type:theme,name}; başka kullanıcının katman aç/kapası baskıyı değiştirmez), layerOverrides, annotation {type:fixed,scale} ve locked (yok: kilitli; ekran yakınlaştırması baskı ölçeğini değiştirmez). Pencere boyu kapsam × paper/model, en yakın µm'ye yuvarlanır. Detaylar şablonun (source; yok: sys:cad-teknik) ilk sabit görünüm penceresinin alanına verilen sırayla, soldan sağa satır satır (aralık 10 mm, başlık bandı 6,5 mm, hücre en az 40 mm) yerleşir; paftaya sığmayan detay setin sonraki paftasına geçer. Şablonun penceresi detaylara yer açar; her detayın altında “<ad> — Ölçek 1/N” yazısı görünümün kendi ölçeğini okur. Paftalar first'ten (yok: D-501) numaralanır, başlık title (yok: Detaylar), alt küme Detaylar. Plan yapılacak paftaları ve her detayın kâğıt çerçevesini verir; hepsi tek değişiklik, inverse geri alır. Boş kapsam invalid_extent, geçersiz oran invalid_ratio, 1 mm'den küçük pencere detail_too_small, boş alana bile sığmayan detay detail_does_not_fit, bounds ile table birlikte ya da hiçbiri ambiguous_source, tablo olmayan nesne not_a_table, olmayan parça table_part_missing."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SheetsDetails>(),
            output: schema::<crate::SheetsDetailed>(),
            plan: Some(schema::<crate::SheetsDetailed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Aynı bağlantının 1:1, 1:5, 1:20 ve 1:100 detayları".into(),
                input: json!({ "details": [
                    { "name": "Bulon", "bounds": [0.0, 0.0, 0.1, 0.06], "ratio": { "paper": 1.0, "model": 1.0 } },
                    { "name": "Levha", "bounds": [0.0, 0.0, 0.5, 0.3], "ratio": { "paper": 1.0, "model": 5.0 } },
                    { "name": "Bağlantı", "bounds": [-0.5, -0.5, 1.5, 1.0], "ratio": { "paper": 1.0, "model": 20.0 } },
                    { "name": "Genel", "bounds": [-5.0, -3.0, 5.0, 3.0], "ratio": { "paper": 1.0, "model": 100.0 } }
                ] }),
                output: None,
            }],
        },
        // Plot style tables (.ctb, .stb) (docs/adr/1130), held together by fixtures/commands/v1/cad.plotstyles.import.json.
        CommandDescriptor {
            id: crate::CAD_PLOTSTYLES_IMPORT.into(),
            version: crate::CAD_PLOTSTYLES_IMPORT_VERSION,
            title: "Baskı stili tablosu içe aktar".into(),
            summary: "İkili baskı stili tablosunu (.ctb renge bağlı, ACI 1–255 için 255 kalem; .stb adlı stiller) pafta kitabına bir baskı profili olarak alır. Dosya yalnız okunur: 60 baytlık üst bilgi (PIAFILEVERSION_2.0,CTBVER1|STBVER1,compress, pmzlibcodec, üç küçük-sonlu u32) ve zlib ile sıkıştırılmış ad=değer metni. Her alan açıkça eşlenir: renk (-1 nesnenin; 0xC2/0xC3 türlü RGB #rrggbb), gri tonlama, tarama (yalnız titreşim açıkken), kalınlık (kalınlık tablosunun mm değeri µm'ye; 0 nesnenin), uç/birleşim (elmas yok), çizgi tipi (0 sürekli, 31 nesnenin; 1–30 eşlenmez). KentOS'un başka bastığı her şey losses'ta türü, ağırlığı (lost, approximate, note), stilleri ve dosyadaki değerleriyle söylenir; sessiz kırpma yoktur. Profil dosyanın biçimini, SHA-256'sını ve adını taşır. assignments kalemleri ya da stilleri model katmanlarına ({type:modelLayer,id}) veya kâğıt öğelerine açıkça verir (katmanın RGB'sinden kalem tahmin edilmez); sheets profili paftalara uygular, yayın onunla basar. Kitabın tek değişikliğidir; inverse geri alır. Plan yazmaz. Aynı ad ya da aynı dosya ikinci kez duplicate_plot_profile; .ctb/.stb olmayan dosya unsupported_plot_style_file; bozuk üst bilgi, metin ya da alan bad_plot_style_file."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::PlotStylesImport>(),
            output: schema::<crate::PlotStylesImported>(),
            plan: Some(schema::<crate::PlotStylesImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Siyah-beyaz CTB'yi al, 1. kalemi Duvar katmanına ver ve A-101 paftasına uygula".into(),
                input: json!({
                    "path": "siyah.ctb",
                    "assignments": [{ "target": { "type": "modelLayer", "id": "duvar" }, "key": { "type": "aci", "index": 1 } }],
                    "sheets": ["a-101"]
                }),
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
        },
    ]
}
