//! The catalog's commands of mapping, cadastre and survey (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
        CommandDescriptor {
            id: crate::CAD_ENTITIES_GRADE.into(),
            version: crate::CAD_ENTITIES_GRADE_VERSION,
            title: "Eğimle kot ver".into(),
            summary: "Bir çizginin, çoklu çizginin ya da alanın dış halkasının from'dan to'ya köşelerine sabit eğimle kot verir: başlangıç kotu start (yoksa köşenin kendi kotu), sonra ya bitiş kotu end (eğim ondan çıkar) ya eğim grade (düşey bölü yatay: 0,02 %2'dir; bitiş ondan çıkar); aradaki her köşe başlangıçtan yol boyunca yatay uzaklığıyla (yaylı kenar yayı boyunca). Aralık dışındaki köşeler kotlarını korur. Yüksek/alçak nokta: grade yerine ahead (başlangıçtan aralığın içine eğim) ve back (bitişten başlangıca doğru eğim), artı yukarı; bitiş kotu end ya da bitiş köşesinin kendi kotu. İki eğimin kesiştiği yere köşe eklenir (yaylı kenarda iki yarı yayın çemberinde kalır; çizgi Köşe ekle'deki gibi çoklu çizgi olur), orada bir köşe varsa ona kot verilir; ona kadar köşeler ileri, sonrakiler geri eğimle. Cevap: aralığın yatay uzunluğu, başlangıç ve bitiş kotu, eğim (yüksek/alçak noktada ileri eğim, back geri eğim), köşelerin yeni kotları (eklenen dahil) ve point: noktanın köşe sırası, eklenip eklenmediği, başlangıçtan uzaklığı, yeri, kotu ve türü (high, low, break). Tek geri alma adımı “Eğimle kot ver”. \
                      Retler: invalid_revision, revision_conflict, invalid_uid, entity_not_found, not_a_path, wrong_place, invalid_input, elevation_missing, no_intersection (eğimler paralel ya da aralığın içinde kesişmiyor), layer_locked."
                .into(),
            aliases: vec!["egimlekot".into(), "setgrade".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesGrade>(),
            output: schema::<crate::EntitiesGraded>(),
            plan: Some(schema::<crate::EntitiesGraded>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İlk köşeden dördüncüye %2 eğimle".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "from": 0, "to": 3, "start": 850.0, "grade": 0.02 }),
                    output: None,
                },
                CommandExample {
                    title: "Yüksek nokta: başlangıçtan %3, bitişten %2 yukarı".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "from": 0, "to": 3, "ahead": 0.03, "back": 0.02 }),
                    output: None,
                },
            ],
        },
        // Referanstan kot ver (plan/map-cad.md MAP-06, docs/adr/3780).
        CommandDescriptor {
            id: crate::CAD_ENTITIES_RAISE.into(),
            version: crate::CAD_ENTITIES_RAISE_VERSION,
            title: "Referanstan kot ver".into(),
            summary: "Bir çizgiyi, çoklu çizgiyi ya da alanı bir referans noktasına göre bütünüyle yükseltir ya da alçaltır: at köşesi (0'dan; çizginin uçları, çoklu çizginin ilk parçası, alanın dış halkası) referansın kotu artı eğim × referanstan yatay uzaklığı (grade, düşey bölü yatay, referanstan köşeye doğru artı yukarı: 0,02 %2'dir) ya da artı kot farkı (difference, m) olur; nesnenin öbür bütün kotları (alanın delikleri, her parçası) aynı farkla kayar, kendi eğimleri değişmez. Kotsuz köşeler kotsuz kalır (Kot ver'in Artır'ı gibi). Referans x, y, z ile verilir (bir noktanın, bir köşenin ya da yazılan). Cevap: yatay uzaklık, köşenin önceki ve yeni kotu, fark (by), referanstan köşeye eğim (köşe referansın üstündeyse yok), kayan kot ve kotsuz köşe sayısı. Tek geri alma adımı “Referanstan kot ver”. \
                      Retler: invalid_revision, revision_conflict, invalid_uid, entity_not_found, not_a_path, wrong_place, invalid_input (grade ile difference birlikte ya da hiçbiri; sonlu olmayan sonuç), elevation_missing (köşenin kotu yok), layer_locked."
                .into(),
            aliases: vec!["referanskot".into(), "raiselowerbyref".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesRaise>(),
            output: schema::<crate::EntitiesRaised>(),
            plan: Some(schema::<crate::EntitiesRaised>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Üçüncü köşe eşikten %2 aşağıda".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "at": 2, "reference": { "x": 500120.0, "y": 4420050.0, "z": 851.25 }, "grade": -0.02 }),
                    output: None,
                },
                CommandExample {
                    title: "İlk köşe referanstan 15 cm yukarıda".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "at": 0, "reference": { "x": 500120.0, "y": 4420050.0, "z": 851.25 }, "difference": 0.15 }),
                    output: None,
                },
            ],
        },
        // Yüzeyden kot ver (plan/map-cad.md MAP-06, docs/adr/4180).
        CommandDescriptor {
            id: crate::CAD_ENTITIES_DRAPE.into(),
            version: crate::CAD_ENTITIES_DRAPE_VERSION,
            title: "Yüzeyden kot ver".into(),
            summary: "Noktaların, çizgilerin, çoklu çizgilerin ve alanların her köşesine (her parça ve delik) projenin arazi yüzeyinin altındaki kotunu verir; yüzeyin dışındaki köşe kendi kotunu korur. surface Harita ya da Yol projesinin yüzeyinin kimliği ya da adıdır. breaks ile düz bir kenarın yüzeyin üçgenlerinin kenarlarını kestiği her yere kotlu ara köşe eklenir (Insert Intermediate Grade Break Points gibi); yay bölünmez, çizgi iki ucunu tutar, ölçülü (M) ya da genişlikli çoklu çizgi bölünmez (unsplit). Kotlar yüzeye bağlı kalmaz. Cevap: her nesnenin kot verilen, yüzey dışında kalan ve eklenen köşe sayısı ve toplamlar. Tek geri alma adımı “Yüzeyden kot ver”. \
                      offset köşelere yüzeyin kotu artı bu farkı verir (artı üstte, eksi altta); range (Partial) tek nesnenin from ile to arasındaki köşelerine (çizginin uçları, çoklu çizginin ya da alanın dış halkası, 0'dan) kot verir, ara kırık noktası yalnız aradaki kenarlara girer. \
                      Retler: invalid_revision, revision_conflict, not_a_road_project, unknown_terrain, invalid_input (nesne yok, 10 000'den çok, biri iki kez, kot farkı sonlu değil, aralık birden çok nesneye), invalid_uid, entity_not_found, not_a_path (noktaya aralık da), wrong_place (aralık köşelerin dışında ya da başı sonundan önce değil), terrain_unbuildable, off_surface (yüzeyde köşe yok), layer_locked."
                .into(),
            aliases: vec!["yuzeydenkot".into(), "drape".into(), "elevationsfromsurface".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesDrape>(),
            output: schema::<crate::EntitiesDraped>(),
            plan: Some(schema::<crate::EntitiesDraped>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bordür çizgisi zemin yüzeyinden, ara kırık noktalarıyla".into(),
                    input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11"], "surface": "Zemin", "breaks": true }),
                    output: None,
                },
                CommandExample {
                    title: "Bordür üstü zeminin 15 cm üstünde, yalnız 2.–5. köşeler".into(),
                    input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11"], "surface": "Zemin", "offset": 0.15, "range": { "from": 1, "to": 4 } }),
                    output: None,
                },
            ],
        },
        // Tarama uyumu denetimi (plan/map-cad.md MAP-13, docs/adr/1500).
        CommandDescriptor {
            id: crate::CAD_POINTCLOUDS_OVERLAP.into(),
            version: crate::CAD_POINTCLOUDS_OVERLAP_VERSION,
            title: "Tarama uyumu denetimi".into(),
            summary: "Aynı yerin iki taramasını (şerit, oturum; uids: iki nokta bulutu) örtüştükleri yerde tam çözünürlükte karşılaştırır: hücre ızgarasında (cell, ilk değer 2 m) her bulutun noktalarına düzlem uydurulur (eğim fark sayılmaz), iki düzlemin hücre merkezindeki farkı (ikinci − birinci); en az least (ilk değer 4) noktalı hücrelerde RMSDz, ortalama (sistematik kayma), standart sapma, en büyük fark ve en kötü 20 hücre; classes ile sınıf süzgeci. \
                      control (nokta katmanı, ad alanı field) verilirse her bulutun kontrol noktası çevresindeki (radius) düzleminin noktaya farkı ve KOH_Z'si. Sınırlar kural paketindendir (rules; ilk değer KentOS örneği: şeritler arası 8 cm RMSDz, USGS QL2'den, Türk mevzuatı değil; KOH_Z BÖHHBÜY m. 91(2)). Kontrolsüz bulut ölçüye kesin hazır sayılmaz. Salt okunur; hiçbir bulut taşınmaz. \
                      Retler: invalid_uid, entity_not_found, not_a_pointcloud, invalid_input, layer_not_found, too_many_cloud_points."
                .into(),
            aliases: vec!["taramauyumu".into(), "seritfarki".into(), "rmsdz".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::PointCloudsOverlap>(),
            output: schema::<crate::PointCloudsOverlapped>(),
            plan: Some(schema::<crate::PointCloudsOverlapped>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İki şerit, 2 m hücre, GNSS kontrolüyle".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12"], "control": "gnss" }),
                output: None,
            }],
        },
        // Tarama ağı dengelemesi (plan/map-cad.md MAP-13, docs/adr/4720).
        CommandDescriptor {
            id: crate::CAD_POINTCLOUDS_NETWORK.into(),
            version: crate::CAD_POINTCLOUDS_NETWORK_VERSION,
            title: "Tarama ağı dengelemesi".into(),
            summary: "Üç ve daha çok taramayı (nokta bulutu; uids) birlikte kaydeder, bir GNSS ağının bazlarını dengelemesi gibi: her bağ (iki tarama) cad.pointclouds.register'ın iki taramayı kaydettiği gibi kaydedilir (birincisi sabit, ikincisi hareketli; spacing, reach, iterations, tolerance, neighbours, robust, reject her bağda aynı), sonucu ikincinin birinciye göre düzeltmesinin ölçüsüdür ve kaydın son normal matrisiyle ağırlıklandırılır. Ölçü ve ağırlık ağın merkezine (taramaların kutularının ortalarının ortalaması) taşınır. Sabit olmayan taramaların düzeltmeleri (ω, φ, κ, öteleme) en küçük karelerle birlikte dengelenir (birinci derece: küçük açılar; Cholesky). \
                      links verilmezse taramaların sırasıyla plan kutuları örtüşen her çift bir bağdır. Cevap: her taramanın düzeltmesi ve Dönüşüm uygula'ya verilecek matrisi (p ↦ R(p − merkez) + merkez + öteleme, R = Rz(κ)·Ry(φ)·Rx(ω)); her bağın eşleşmesi, örtüşmesi, kayıt sonrası RMS'i, gücü, ölçüsü, dengeleme artığı ve artığın eşleşmelerin normalleri boyunca kayması (√(rᵀ·N·r / eşleşme)); her döngünün (sabit taramalar tek düğüm sayılır) kapanmaması (dönme ve öteleme uzunluğu); ağın RMS kayması. rounds (1–10) dış döngüdür: ikinci turdan başlayarak her bağ taramaların bir önceki turdaki yerinden yeniden kaydedilir, bir taramanın kutusunun köşesi tolerance kadar kaymayınca durulur; cevabın rounds listesi her turun en büyük kaymasıdır, matrisler turların toplamı, bağlar ve döngüler son turunkidir. Salt okunur: taramalar yerinde durur. \
                      Retler: invalid_uid, entity_not_found, not_a_pointcloud (uids[i]); invalid_input (üçten az tarama, aynı tarama iki kez; sabit tarama yok, hepsi sabit, sabit tarama ağda değil; bağ iki kez, bir taramayı kendine bağlıyor, taraması ağda değil; bir tarama hiçbir sabit taramaya bağlanmıyor; bağın kaydında altıdan az eşleşme; ağın bir taramanın konumunu belirlememesi, düz zeminde yatay öteleme gibi; tur sayısı 1–10 dışında); kaydın kendi seçeneklerinin retleri; file_not_found, import_unreadable."
                .into(),
            aliases: vec!["taramaagi".into(), "agdengelemesi".into(), "coklutarama".into(), "bundleregistration".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::PointCloudsNetwork>(),
            output: schema::<crate::PointCloudsNetworked>(),
            plan: Some(schema::<crate::PointCloudsNetworked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Üç tarama, birincisi sabit, bağlar örtüşmeden".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13"], "fixed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11"] }),
                output: None,
            }],
        },
        // Bulutu konumla (plan/map-cad.md MAP-13, docs/adr/5100).
        CommandDescriptor {
            id: crate::CAD_POINTCLOUDS_PLACE.into(),
            version: crate::CAD_POINTCLOUDS_PLACE_VERSION,
            title: "Bulutu konumla".into(),
            summary: "Nokta bulutunun dosyalarına, dosyaları yeniden yazmadan çizimde bir konum verir (taramanın konumu gibi): katı bir dönüşüm (dönme ve öteleme), dosyaların bugünkü konumundan sonra uygulanır; Tarama kaydı'nın ya da Tarama ağı'nın matrisi taramayı çizimde gösterildiği yerden taşır. Bulutun noktaları bundan sonra çizimde, sorguda ve ölçüde bu konumdadır; dosyaya yalnız Dönüşüm uygula yazar. \
                      Dönüşüm: matrix (satır satır 16 sayı), axisAngle (eksen, açı derece, sağ el kuralı), euler (yaw Z, pitch dönmüş Y, roll iki kez dönmüş X çevresinde, bu sırayla, derece, her biri sağ el kuralıyla; X doğu, Y kuzey, Z yukarı; R = Rz(yaw)·Ry(pitch)·Rx(roll)); center verilmezse bulutun çizimdeki kutusunun ortası, translation verilmezse öteleme yok; remove konumu kaldırır. Ölçek, kayma ya da yansıma içeren matris reddedilir (uzunlukları değiştirir: Dönüşüm uygula ile dosyaya yazılır). \
                      Cevap: alınan matris, her dosyanın sonraki konumu, bulutun çizimdeki kutusu, kutunun ortasının kayması ve dönme açısı. Konumu olmayan bulutta remove hiçbir şey yazmaz. Tek geri alma adımı, Bulutu konumla. \
                      Retler: invalid_uid, entity_not_found, not_a_pointcloud, layer_locked (uid); not_finite, invalid_input (place); invalid_revision, revision_conflict."
                .into(),
            aliases: vec!["bulutukonumla".into(), "taramakonumu".into(), "pose".into(), "scanpose".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::PointCloudsPlace>(),
            output: schema::<crate::PointCloudsPlaced>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Taramayı Euler açılarıyla döndür ve ötele".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12",
                        "place": { "kind": "euler", "yaw": 0.5, "pitch": 0.0, "roll": 0.0, "translation": [0.2, -0.1, 0.05] } }),
                    output: None,
                },
                CommandExample {
                    title: "Konumu kaldır".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12", "place": { "kind": "remove" } }),
                    output: None,
                },
            ],
        },
        // Tarama kaydı denetimi (plan/map-cad.md MAP-13, docs/adr/1800).
        CommandDescriptor {
            id: crate::CAD_POINTCLOUDS_REGISTER.into(),
            version: crate::CAD_POINTCLOUDS_REGISTER_VERSION,
            title: "Tarama kaydı denetimi".into(),
            summary: "İkinci nokta bulutunu (hareketli) birinciye (sabit) örtüştükleri yerde nokta-düzlem ICP ile oturtur ve kaydı denetler, salt okunur: iki bulut tam çözünürlükte okunup voksel başına bir noktaya (noktalarının ortalaması, spacing, ilk değer 0,5 m) indirilir; sabit bulutun normali en yakın komşularından (neighbours, ilk değer 8), her hareketli nokta erişim içindeki (reach, ilk değer 1 m) en yakın sabit noktayla eşlenir; adım Low'un (2004) doğrusal en küçük kareleriyle, adım toleransın (ilk değer 0,1 mm) altına inince ya da yineleme sınırında (ilk değer 30) durur. \
                      Cevap: dönüşüm tarifi (merkez, dönüklük ω φ κ, öteleme ve projenin koordinatlarında 4×4 matris), önce ve sonra nokta-düzlem RMS'i, eşleşen pay (örtüşme), geometrinin altı parametreyi tutma gücü ve yatay öteleme gücü (düz zeminde yatay bilinmez), kapsam hücrelerinin güvenilir alanı (hücre RMS'i paketin şeritler arası örnek sınırıyla, lidar-uyum), en kötü hücreler ve kontrol noktalarında sabit bulutun ve hareketli bulutun önce/sonra KOH_Z'si (BÖHHBÜY m. 91(2) sınırıyla). Bulut taşınmaz; kontrolsüz kayıt ölçüye kesin hazır sayılmaz. \
                      robust: her adımda eşleşmeler ağırlıklanır; huber (k, ilk değer 1,345; ölçek 1,4826 × ortanca |artık|) ya da trimmed (ratio: en küçük artıklı eşleşmelerin tutulan payı, kullanıcının; ilk değeri yok); cevap son adımın ölçeğini ve ağırlığı birden küçük eşleşmeleri verir. \
                      reject (PCL'in eşleşme ayıklayıcıları): edges, sabit noktası bulutunun kenarında olan eşleşmeyi (erişim içindeki komşularının teğet düzlemdeki yönleri 90°'den büyük boşluk bırakıyor: tarama sınırı, gölge) her adımın dışında bırakır; angle, sabit normali ile hareketli noktanın kendi normali bu kadar dereceden çok ayrılan (ya da hareketli normali olmayan) eşleşmeyi bırakır: bir taramanın görüp öbürünün görmediği duvar, araç yanı; cevap son adımda bırakılanları sayar. \
                      Retler: invalid_uid, entity_not_found, not_a_pointcloud, invalid_input (reject boş, reject.angle 0–90 dışı), layer_not_found, too_many_cloud_points."
                .into(),
            aliases: vec!["taramakaydi".into(), "icp".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::PointCloudsRegister>(),
            output: schema::<crate::PointCloudsRegistered>(),
            plan: Some(schema::<crate::PointCloudsRegistered>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İki şerit, 0,5 m voksel, GNSS kontrolüyle".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12"], "control": "gnss" }),
                output: None,
            }],
        },
        // Ortofoto doğruluk denetimi (plan/map-cad.md MAP-12, docs/adr/1480).
        CommandDescriptor {
            id: crate::CAD_ORTHOPHOTO_CHECK.into(),
            version: crate::CAD_ORTHOPHOTO_CHECK_VERSION,
            title: "Ortofoto doğruluk denetimi".into(),
            summary: "Bir ortofotoyu (raster) arazide ölçülmüş noktalarla (known katmanı, GNSS) ve aynı noktaların görüntüde okunan yerleriyle (observed katmanı) adlarına göre (field, ilk değer Ad) denetler: her noktanın hatası (görüntü − arazi) doğu, kuzey ve iki yanda kot varsa yükseklikte; bağımsız denetim noktalarının KOH_X,Y'si (√(Σ(dx²+dy²)/2n)), radyal RMSE, ortalama (yönlü) hata, en büyük hata ve KOH_Z'si. Görüntünün oturtulduğu kontrol noktaları (georef) uyum noktasıdır, doğruluğa sayılmaz. \
                      Sınırlar kural paketindendir (rules; ilk değer BÖHHBÜY 2018 m. 91(2): KOH_X,Y (cm) ≤ −1,665·10⁻⁶·S² + 0,01745·S − 1,166, KOH_Z ≤ 1,33·KOH_X,Y; m. 54 yer örnekleme aralığı). Yer örnekleme aralığı pikselin alanının karekökü; KOH_X,Y yarım pikselden küçükse belowPixel (görüntüyü okuma inceliğinin altında: sonuç iyimser olabilir). Salt okunur: görüntünün koordinatları hiç değişmez. \
                      Retler: invalid_uid, entity_not_found, not_a_raster, crs_mismatch (raster projenin sisteminde değil), layer_not_found, invalid_input."
                .into(),
            aliases: vec!["ortofotodogruluk".into(), "koh".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::OrthophotoCheck>(),
            output: schema::<crate::OrthophotoChecked>(),
            plan: Some(schema::<crate::OrthophotoChecked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ortofotoyu GNSS denetim noktalarıyla, 1/1000".into(),
                input: json!({ "raster": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "known": "gnss", "observed": "ortofoto-okuma", "scale": 1000 }),
                output: None,
            }],
        },
        // İki ortofotonun karşılaştırılması (plan/map-cad.md MAP-12, docs/adr/1780).
        CommandDescriptor {
            id: crate::CAD_ORTHOPHOTO_COMPARE.into(),
            version: crate::CAD_ORTHOPHOTO_COMPARE_VERSION,
            title: "Ortofoto karşılaştırması".into(),
            summary: "İki ortofotoyu (raster; first eskisi ya da mozaikte komşusuna göre bakılan, second öbürü) aynı noktaların iki görüntüde okunan yerleriyle karşılaştırır: noktalar adlarına göre (field, ilk değer Ad) firstPoints ve secondPoints katmanlarından eşlenir; her noktanın kayması ikinci − birinci (doğu, kuzey, uzunluk), kaymaların KOH_X,Y'si, radyal RMSE'si, ortalaması (görüntüler arasındaki sistematik fark) ve en büyüğü. \
                      Nokta başına sınır kural paketinin mosaic katsayısı × iki görüntüden iri YÖA'dır (ilk değer BÖHHBÜY 2018 m. 67(6): mozaikteki ortofotolar arasındaki kayıklık en çok ±2·YÖA; iki farklı pikselde iri olanı almak KentOS'un okumasıdır); sınırı aşan nokta işaretlenir, passed hepsinin içinde olup olmadığıdır. Arazide ölçülmüş noktalar (known) verilirse her görüntü ayrıca cad.orthophoto.check ile kendi doğruluğuyla denetlenir (firstCheck, secondCheck). İki zamanın farkı kendiliğinden deformasyon değildir. Salt okunur. \
                      İstenirse (seam) kayma görüntülerin kendisinden ölçülür: örtüşmede step metrede bir yer, birinci görüntünün window × window piksellik parçası ikincide search piksel içinde sıfır ortalamalı normalize çapraz korelasyonla aranır, tepe parabolle piksel altına inceltilir; yer durumu ok, low (correlation'dan düşük), edge (tepe arama sınırında), flat (dokusuz); ok yerlerin kayma istatistiği ve sınır. Görüntülerin pikselleri aynı boyda ve dönüksüz olmalı ya da seam.resample ile ikinci görüntü birincinin ızgarasına kübik evrişimle örneklenir (birinci kuzeye bakmalı; cevapta resampled); dosya göreliyse seam.folder. seam.polygons: iki görüntünün mozaik elemanları (TUCBS Ortogörüntü; görüntünün mozaikte kullanılan bölümü: alan ya da düz kenarlı kapalı çoklu çizgi); yerler ortak sınırları boyunca (birincinin halkalarının ikincininkilere 1 mm içinde oturan parçaları, birincinin sırasıyla), her parçanın başından adım aralıklı; cevapta seam.shared (parçalar, uzunluk, yerler, dışarıdakiler), her yerin part'ı; line ile birlikte verilmez. \
                      Retler: invalid_uid, entity_not_found, not_a_raster, crs_mismatch (raster projenin sisteminde değil), layer_not_found, invalid_input (aynı raster iki kez, ölçek 0, kural paketi kimliksiz, kaynaksız ya da mosaic'siz; seam'in step, window, search, correlation'ı; resample'sız farklı pikseller, yeniden örneklemede dönük birinci, örtüşme yok; line ile polygons birlikte, aynı çokgen iki kez, çizimde olmayan, alan olmayan ya da yaylı mozaik elemanı, ortak sınır yok, ortak sınır örtüşmede değil), file_not_found (görüntünün dosyası yok)."
                .into(),
            aliases: vec!["ortofotokarsilastir".into(), "mozaikkayiklik".into(), "dikishatti".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::OrthophotoCompare>(),
            output: schema::<crate::OrthophotoCompared>(),
            plan: Some(schema::<crate::OrthophotoCompared>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "2024 ve 2026 ortofotoları, GNSS denetim noktalarıyla".into(),
                input: json!({ "first": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "second": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12", "firstPoints": "okuma-2024", "secondPoints": "okuma-2026", "known": "gnss" }),
                output: None,
            }],
        },
        // Birleşme çizgisi ağı (plan/map-cad.md MAP-12, docs/adr/5260).
        CommandDescriptor {
            id: crate::CAD_ORTHOPHOTO_NETWORK.into(),
            version: crate::CAD_ORTHOPHOTO_NETWORK_VERSION,
            title: "Birleşme çizgisi ağı".into(),
            summary: "İki ya da daha çok ortofotonun (images, 2–50; projenin sistemindeki rasterler) mozaik elemanlarını kurar: ayak izlerinin (footprints, aynı sırayla; alan ya da kapalı çoklu çizgi, düz kenarlı, dışbükey; verilmezse rasterlerin dört köşesi) birleşiminin her yeri, onu kaplayan görüntülerden ayak izinin merkezi (alan ağırlık merkezi) en yakın olana gider; eşitse önce verilene. Her görüntünün elemanı ayak izinden başkalarının daha yakın kısımları çıkarılarak bulunur; birleşme çizgileri elemanların ortak kenarlarıdır. \
                      Cevapta her elemanın parçaları (dış halka saat yönünün tersine en güneydeki köşesinden; delikler), alanı, ayak izinin merkezi ve yazılınca kimliği; üç ya da daha çok elemanın buluştuğu düğümler (junctions: yeri ve görüntüleri; üçlü düğüm); kaplanan alan; notlar. Elemanlar alan olarak layer katmanına (verilmezse mozaik-elemanlari, yoksa aynı adımda açılır), öznitelikleri Görüntü ve Yöntem; tek geri alma adımı “Mozaik elemanları ağı”. \
                      Retler: invalid_uid, entity_not_found, not_a_raster, crs_mismatch (images[k]); invalid_input (ikiden az ya da 50'den çok görüntü, bir görüntü iki kez, ayak izleri her görüntüye bir değil, dışbükey olmayan ya da alan kaplamayan ayak izi, her yeri başka görüntüye daha yakın olan görüntü); layer_not_found, layer_locked; invalid_revision, revision_conflict."
                .into(),
            aliases: vec!["birlesmecizgisiagi".into(), "mozaikagi".into(), "seamnetwork".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::OrthophotoNetwork>(),
            output: schema::<crate::OrthophotoNetworkBuilt>(),
            plan: Some(schema::<crate::OrthophotoNetworkBuilt>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Üç ortofotonun mozaik elemanları".into(),
                input: json!({ "images": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13"] }),
                output: None,
            }],
        },
        // Ayak izi (plan/map-cad.md MAP-12, docs/adr/5260).
        CommandDescriptor {
            id: crate::CAD_ORTHOPHOTO_FOOTPRINT.into(),
            version: crate::CAD_ORTHOPHOTO_FOOTPRINT_VERSION,
            title: "Ayak izi".into(),
            summary: "Bir rasterin ayak izini piksellerinden çıkarır: NoData olmayan piksellerin (bantlarından biri NoData değilse geçerli; NoData yoksa her piksel) piksel kenarlarından dış sınırı, 4 komşulukla (köşeden değen pikseller ayrı parça), parçaları ve delikleriyle; hull ile bunun yerine geçerli piksellerin dışbükey zarfı (birleşme çizgisi ve ağ bunu ayak izi olarak alır). \
                      Cevapta parçalar (dış halka saat yönünün tersine en güneydeki köşesinden; delikler), alan, okunan ve geçerli piksel sayısı, NoData değeri, katman ve yazılınca kimliği. Alan layer katmanına (verilmezse ayak-izleri, yoksa aynı adımda açılır), öznitelikleri Görüntü ve Yöntem; tek geri alma adımı “Ayak izi”. Göreli yollu raster dosyası için folder çizimin klasörüdür. \
                      Retler: invalid_uid, entity_not_found, not_a_raster, crs_mismatch (raster); file_not_found, invalid_file_path, import_unreadable (görüntünün dosyası); invalid_input (25 milyondan çok piksel, her piksel NoData); layer_not_found, layer_locked; invalid_revision, revision_conflict."
                .into(),
            aliases: vec!["ayakizi".into(), "footprint".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::OrthophotoFootprint>(),
            output: schema::<crate::OrthophotoFootprintBuilt>(),
            plan: Some(schema::<crate::OrthophotoFootprintBuilt>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ortofotonun dışbükey ayak izi".into(),
                input: json!({ "raster": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "hull": true }),
                output: None,
            }],
        },
        // Mozaik birleşme çizgisi (plan/map-cad.md MAP-12, docs/adr/4700).
        CommandDescriptor {
            id: crate::CAD_ORTHOPHOTO_SEAMLINES.into(),
            version: crate::CAD_ORTHOPHOTO_SEAMLINES_VERSION,
            title: "Mozaik birleşme çizgisi".into(),
            summary: "İki ortofotonun (raster; first, second) mozaikteki birleşme çizgisini ve mozaik elemanlarını kurar (TUCBS Ortogörüntü: mozaik elemanı görüntünün mozaikte kullanılan bölümüdür, elemanlar örtüşmez). Ayak izleri rasterlerin sınırı ya da verilen alanlardır (footprints: alan ya da düz kenarlı kapalı çoklu çizgi, dışbükey). \
                      method voronoi, eşit uzaklık (ilk değer; Pan ve ark. 2009, örtüşmeli alan Voronoi diyagramı): örtüşmenin bir noktası kendi bölgesi (ayak izinin öbürününkinin dışında kalan bölümü) daha yakın olan görüntüye düşer; birleşme çizgisi iki bölgeden eşit uzak noktalardır: en yakın öğeleri iki kenar (ya da iki köşe, ya da kenarın doğrusundaki köşe) olan parçası düz (açıortay) ve yalnız dönüm noktaları yazılır; köşe ile kenar arasındaki eğri parçasının noktaları kirişe dik doğrularda step metrede bir (ilk değer 1). Uçlar iki sınırın kesiştiği yerler ya da ortak sınırın iki bölgeden eşit uzak noktasıdır (ends: crossing, shared). \
                      method radiometry, radyometrik fark (Chon, Kim, Lin 2010): ızgara birinci görüntünün cell × cell pikselidir (ilk değer 1); pikselin farkı ortak bantların mutlak farklarının en büyüğü (ikinci, merkezleri çakışmıyorsa Keys kübik evrişimiyle birincinin piksel merkezinde okunur); hücrenin farkı piksellerininkilerin en büyüğü. Hücre bütün pikselleri örtüşmede ve okunur, merkezi eşit uzaklık çizgisine corridor (arama bandı) metreden yakın (verilirse) ve hiçbir pikseli avoid katmanının alanlarında (geçilmeyecek alanlar: bina, köprü; USGS) değilse aranır. Eşit uzaklık çizgisinin uçlarına en yakın aranan hücreler arasında önce en büyük farkı en küçük yol, sonra onu aşmayan hücrelerde en az maliyetli yol (hareketin boyu × (1 + iki hücrenin farklarının ortalaması); 8 komşu); çizgi başlangıç, yolun dönen hücrelerinin merkezleri ve bitiştir. difference: en büyük fark, yolun ve aranan hücrelerin ortalaması, hücre sayıları, hücrenin yerdeki boyu. \
                      Mozaik elemanlarının halkaları: birincininki kendi sınırında bitişten başlangıca, sonra birleşme çizgisi; ikincininki başlangıçtan bitişe, sonra çizginin tersi (saat yönünün tersine); ikisi de basit olmalı. Plan hiçbir şey yazmaz; execute iki alanı (öznitelikleri Görüntü: rasterin kimliği, Yöntem) layer katmanına tek geri alma adımında (“Mozaik elemanları”) yazar; layer yoksa mozaik-elemanlari (“Mozaik elemanları”) aynı adımda açılır (opened). Sonuç Ortofoto karşılaştırması'nın seam.polygons'ına doğrudan verilir. BÖHHBÜY birleşme çizgisinin yerine kural koymaz; m. 67(6) ortak alanda radyometrik düzeltme ve ±2·YÖA kayıklık ister. \
                      Retler: invalid_uid, entity_not_found, not_a_raster, crs_mismatch (first, second); invalid_input (aynı raster iki kez; step, cell (1..64), corridor; voronoi ile cell, corridor ya da avoid; ayak izi iki kez, çizimde yok, alan ya da kapalı çoklu çizgi değil, yaylı, delikli ya da parçalı, dışbükey değil; örtüşme yok, biri öbürünün içinde, sınır ikiden çok el değiştiriyor; radyometrik farkta aranacak hücre yok, çok piksel, iki uç arasında yol yok; kendini kesen mozaik elemanı); layer_not_found (layer, avoid), layer_locked; invalid_revision, revision_conflict; invalid_file_path, file_not_found, import_unreadable (görüntünün dosyası)."
                .into(),
            aliases: vec!["birlesmecizgisi".into(), "mozaikelemani".into(), "seamline".into(), "buildseamlines".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::OrthophotoSeamlines>(),
            output: schema::<crate::OrthophotoSeamlinesBuilt>(),
            plan: Some(schema::<crate::OrthophotoSeamlinesBuilt>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki ortofotonun sınırlarından eşit uzaklık (Voronoi) birleşme çizgisi".into(),
                    input: json!({ "first": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "second": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12" }),
                    output: None,
                },
                CommandExample {
                    title: "Radyometrik fark, binalardan geçmeden, eşit uzaklık çizgisinden en çok 5 m".into(),
                    input: json!({ "first": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "second": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12", "method": "radiometry", "corridor": 5.0, "avoid": "bina" }),
                    output: None,
                },
            ],
        },
        // Düzenleme dağıtım taslağı (plan/map-cad.md MAP-11, docs/adr/1460).
        CommandDescriptor {
            id: crate::CAD_READJUSTMENT_DISTRIBUTE.into(),
            version: crate::CAD_READJUSTMENT_DISTRIBUTE_VERSION,
            title: "Düzenleme dağıtım taslağı".into(),
            summary: "Arazi düzenlemesinin ilk sayılarını tekrarlanabilir bir taslak olarak hesaplar, karar olarak değil: her parselin tapu alanı (ondalık yazı, yazıldığı gibi), kural paketinin kesinti oranıyla kesinti, net alan ve her hak sahibinin kesin payından (1/3, 1/3 kalır) hakkı. \
                      Kural paketi (rules) kurumunu, kaynak belgesini, yürürlüğünü, sürümünü, onayını, kesinti oranını (deduction, [0, 1)) ve sayı politikasını (scale ≤ 6, rounding: half_even…, remainder: reject | largest_remainder) taşır; hiçbir oran kodda sabit değildir. Kesintinin kesin toplamı bir kez yuvarlanır ve parsellere alanlarıyla dağıtılır, net alan hak sahiplerine paylarıyla; her artığın kime gittiği söylenir. Toplamlar kesin denetlenir (conserved). Sayılar ondalık yazıdır, float değil. Sonuç her zaman taslaktır (draft); resmî karar verilmez. \
                      Değer esaslı karşılaştırma (rules.basis: value; ilk değer area, yönetmeliğin alan esaslı dağıtımı): dağıtım kütlesi (tapu toplamı − kesinti) parsellere düzenlemeden önceki değerleri oranında dağıtılır, parselin kesintisi alanı − payıdır (alan kazanırsa eksi); her değer kullanıcınındır (entries[i].value, parselin her kaydında aynı) ve kaynağıyla (rules.valueSource); KentOS değer üretmez. Cevapta basis, valueSource, parsellerin value ve valueShare'i (kesir), kütlenin artığı (massRemainder, massRemainderTo); her zaman value_basis_not_a_rule uyarısı: yönetmelikte kural değildir. \
                      Retler: invalid_input (değer esasında: bilinmeyen esas, kaynak yok, değeri olmayan, eksi ya da parselde farklı değer, değerler toplamı sıfır; alan esasında verilen değer), area_mismatch, duplicate_holder, shares_not_whole, needs_rule."
                .into(),
            aliases: vec!["duzenlemedagitim".into(), "18madde".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ReadjustmentDistribute>(),
            output: schema::<crate::ReadjustmentDistributed>(),
            plan: Some(schema::<crate::ReadjustmentDistributed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Sentetik iki parsel, %40 kesinti, iki basamak".into(),
                input: json!({ "rules": { "id": "ornek-paket", "release": 1, "jurisdiction": "Örnek kurum", "source": "Örnek belge", "effective": "2026-01-01", "approved": false, "deduction": "0.40", "scale": 2, "rounding": "half_even", "remainder": "largest_remainder" }, "entries": [ { "parcel": "101/1", "holder": "H1", "area": "1000", "share": "1/3" }, { "parcel": "101/1", "holder": "H2", "area": "1000", "share": "2/3" } ] }),
                output: None,
            }],
        },
        // Düzenleme kapsamı (plan/map-cad.md MAP-11, docs/adr/2060).
        CommandDescriptor {
            id: crate::CAD_READJUSTMENT_SCOPE.into(),
            version: crate::CAD_READJUSTMENT_SCOPE_VERSION,
            title: "Düzenleme kapsamı".into(),
            summary: "Bir arazi düzenlemesinin sınırına (boundary: kapalı bir alan) giren parselleri ayırma çapının sayılarıyla ve dağıtıma esas alanla verir (Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13, 31): her parselin tescilli yüzölçümü (field, ilk değer Tapu alanı (m²); yazıldığı gibi), koordinatlardan hesaplanan yüzölçümü, farkı ve yanılma sınırı (cad.parcels.check'in 2022 Yönetmeliği m. 8(2) çizelgesi), sınırın içinde ve dışında kalan kısımlar (kesin; yaylar yay kalır), dışarıdaki parçalar (her biri ayrı parsel numarası alır). \
                      Esas alan: tamamı giren ve yanılma sınırı içindeki parselde tescilli yüzölçümü (31/1); kısmen girende giren kısmın hesaplanan yüzölçümü, scale ve rounding ile bir kez yuvarlanmış (31/2), hata girmeyen kısımda kalır (outsideRecorded = tescilli − esas). Esas yok: yanılma sınırı dışında (needsCorrection: düzeltilmiş yüzölçümü kadastronun işidir), kayıt yok ya da okunmuyor, tescilli politikanın basamağından ince (finerThanPolicy), girmeyen kısım hatayı taşıyamıyor (outsideTooSmall), toleranstan (tolerance, ilk değer 0,01 m, ortalama genişlik 2A/P) dar parça var (sliver: sınır parselin kenarının yakınından geçiyor). \
                      Toplamlar: sınırın alanı, parsellerin içerideki kısımları, hiçbir parselin kaplamadığı alan, esasların kesin toplamı, sayılar. uids yoksa parsel katmanının (parsel) sınırın içinde kısmı olan alanları. Salt okunur; hesaplanan alan tescilli alanın üstüne yazılmaz, resmî karar verilmez. \
                      Retler: invalid_input (scale > 6, rounding, tolerance), invalid_uid, entity_not_found, not_an_area (boundary), not_a_parcel (uids[i])."
                .into(),
            aliases: vec!["duzenlemekapsami".into(), "ayirmacapi".into(), "duzenlemesiniri".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ReadjustmentScope>(),
            output: schema::<crate::ReadjustmentScoped>(),
            plan: Some(schema::<crate::ReadjustmentScoped>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Düzenleme sınırına giren parseller, iki basamak, yarımda çifte".into(),
                input: json!({ "boundary": "0192f800-2060-7000-8000-000000000001", "scale": 2, "rounding": "half_even" }),
                output: None,
            }],
        },
        // Ayırma çapı paftaları (plan/map-cad.md MAP-11, docs/adr/2680, 3080).
        CommandDescriptor {
            id: crate::CAD_READJUSTMENT_SHEETS.into(),
            version: crate::CAD_READJUSTMENT_SHEETS_VERSION,
            title: "Ayırma çapı paftaları".into(),
            summary: "Düzenleme sınırına kısmen giren her parsel için bir ayırma çapı paftası yapar (Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13: parsel başına bir çap, dışarıda kalan bütün parçalar aynı çapta). Sayılar ve parçalar cad.readjustment.scope'un (scope: onun girdisi): tescilli ve hesaplanan yüzölçümü, fark, yanılma sınırı, giren ve girmeyen kısım; parçaların sınırları paftanın numaralı noktalarıyla (1-2-3-4-1). \
                      Noktalar çizime yazılır: her parselinki “Ayırma çapı noktaları” grubunda parselin adını taşıyan kendi katmanında, numaralarıyla etiketli; paftanın haritası parsele standart ölçekte sığar (%5 kenar payı; options.scale verilirse o ölçekte ortalanır) ve öbür parsellerin nokta katmanlarını dondurur, koordinat listesi parselin katmanını okur, Yüzölçümleri ve Kısımlar tabloları politikanın basamağı ve yuvarlamasıyla yazılır. \
                      Şablon yoksa KentOS'un ayırma çapı (sys:ayirma-capi); kendi şablonunda harita kimlikli sabit görünüm penceresi gerekir, koordinatlar, yuzolcumleri ve kisimlar varsa doldurulur. Pafta adı “Ayırma çapı ‹Ada/Parsel›”; il, ilce, mahalle, ada, parsel soruları parselin özniteliklerinden. numbers: dışarıda kalan parçaların tescil edileceği parsel numaraları (m. 13/3), parselin kimliğiyle parçaların sırasında; Kısımlar'da adın yanında yazılır (“Girmeyen 1 · parsel 101/24”), KentOS numara vermez; numaralı her dış parça parselin katmanına da yazılır: parça alan olarak (Parsel, Kısım, ParselNo öznitelikleriyle) ve içinde numarasıyla etiketli bir nokta (parçanın köşelerinin orta yüksekliğindeki yatay çizginin parçanın içindeki en geniş aralığının ortası); cevapta labels. numberHeight (kâğıt mm, kullanıcının) verilince numara nokta yerine parçanın içine sığan bir yazıdır: boyu numberHeight × ölçek / 1000, sığana dek onda bir adımlarla küçülür; önce yatay, sonra parçanın en uzun düz kenarı boyunca denenir; labels'ta height, rotation, share ve fits. numberLeader (numberHeight ile): tam boyda sığmayan numara küçülmez, parçanın dışına kılavuzla yazılır; yerler en yakından denenir (çizgi parçanın sınırından 1, 2, 3 yazı boyu öte; doğu, batı, kuzey, güney, sonra ara yönler) ve kılavuzun parçanın dışındaki çizgisi, inişi ve notu paftanın haritasının gösterdiği hiçbir şeye (gösterilen katmanlardaki çizgiler ve alanların sınırları, yazılar, kılavuzlar, noktalar; paftanın öbür numaraları) değmeyen ilk yer seçilir; labels'ta leader (uç) ve clear. Hiçbir yer temiz değilse en az şeye değen yer: clear false, clashes (nesnelerin kalıcı kimlikleri, numaraların parça adları) ve number_crowded uyarısı. Tek geri alma adımı. Taslaktır: çaplar parselasyon planıyla birlikte onaylanır (m. 13/2), KentOS onay vermez. \
                      Retler: kapsamın kendi retleri (scope.…), no_partial_parcels, no_identity, duplicate_parcel, unknown_template, invalid_options (ad, merkez ya da parselin özniteliğine cevap), invalid_input (numbers: kısmen girmeyen parsel, parça sayısını tutmayan, boş ya da aynı numara; numberHeight 0'dan büyük değil; numberHeight'sız numberLeader), not_fixed_map, separation_sheet_exists (pafta ya da nokta katmanı zaten var)."
                .into(),
            aliases: vec!["ayirmacapipaftalari".into(), "capinpaftalari".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ReadjustmentSheets>(),
            output: schema::<crate::ReadjustmentSheetsMade>(),
            plan: Some(schema::<crate::ReadjustmentSheetsMade>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kısmen giren parsellerin ayırma çapı paftaları, iki basamak, yarımda çifte".into(),
                input: json!({ "scope": { "boundary": "0192f800-2060-7000-8000-000000000001", "scale": 2, "rounding": "half_even" } }),
                output: None,
            }],
        },
        // Senaryo ölçüleri (plan/map-cad.md MAP-10, docs/adr/1440; “Alternatif etkisi” until 2026-11-07).
        CommandDescriptor {
            id: crate::CAD_SCENARIOS_IMPACT.into(),
            version: crate::CAD_SCENARIOS_IMPACT_VERSION,
            title: "Senaryo ölçüleri".into(),
            summary: "Seçilen senaryoyu yerine geçtiği mevcut durumla sayılarla, yan yana karşılaştırır: nesne sayısı, alanların toplamı (m², düzlemsel), çizgilerin uzunluğu (m), inşaat alanı (Σ alan × KAKS, m²), her birinin birimi ve yöntemi; kullanıma (field, ilk değer Kullanim) göre alanlar; anahtarıyla (key, ilk değer KararKimlik) eşlenen kararlardan değişenler (added, removed, geometry, decision, both) önce/sonra kullanım, alan ve inşaat alanıyla; veri eksikleri (KAKS'ı, kullanımı ya da anahtarı olmayan alanlar) ve varsayımlar. Tek puanla en iyi senaryo seçilmez. \
                      stamp mevcut durumun katmanlarının özetidir: önceki bir karşılaştırmanın damgası since ile verilince stale taban o zamandan beri değişti mi söyler (senaryo kendiliğinden yeni veriye uymuş sayılmaz). İstenince, değerleri kullanıcının iki ölçü daha (KentOS değer vermez): impervious (her kullanımın geçirimsizlik payı 0–1): geçirimsiz alan = Σ alan × pay, payı verilmeyen kullanım veri eksiği; access (distance m, from ve to kullanımları): ağırlık merkezi en yakın to alanına düz uzaklıkta (içindeyse 0, yaylar kesin) olan from alanlarının toplamı ve from alanlarına oranı (%), to alanı olmayan taraf veri eksiği; infrastructure (layer, distance m, from): ağırlık merkezi katmanın bir nesnesine o uzaklıkta olan alanların toplamı, near'da yakın ve uzak alanlar ve katmanın nesneleri, baseLinks ve alternativeLinks'te her ölçülen alan en yakın nesnesine bağlı: nesne, ağırlık merkezi, nesnenin en yakın yeri ve uzaklık (eşitlerde çizimde önce gelen); yakınlıktır, kapasite ya da bağlanma hakkı değildir; slope (surface, threshold %, from): ölçülen alanların projenin arazi yüzeyinin üçgenlerine düşen parçaları, her üçgenin eğimi düzleminden (normali, düşeyin yataya oranı, %), ortalama eğim (alan ağırlıklı, slopeMean), eşiği aşan dik alan (slopeSteep) ve payı, yüzeyin dışındaki alan (slopeOff, veri eksiği); slope.classes (artan % sınırları) ile sınıflara, slope.aspect ile bakıya göre alanlar; slope.map ile slopeMap: her ölçülen alanın üçgenlere düşen parçaları sınıfı, bakısı, eğimi, alanı ve halkasıyla (haritada boyamak için; sınıf ölçülerinin parçalarıdır). \
                      access.network (projenin Ağlar'daki ağı, kimlik ya da ad) ve access.connect (m, kullanıcının) ile erişim ağ boyunca: ağırlık merkezi ağın en yakın yerine bağlanır (connect'ten uzaksa ağa bağlanamaz, erişmez sayılır, veri eksiği), uzaklık = bağlanma + ağın tanımına göre en kısa yol + hedefin merkezine bağlanma; reach'te her alanın yolu (baseRoutes/alternativeRoutes: hedef, uzaklık, iki bağlanma, noktalar) ve ağa bağlanamayanlar; senaryonun yerine geçtiği ağ katmanı her tarafın kendisinindir. \
                      scenarioStamp senaryo katmanlarının özetidir; records cad.scenarios.record ile çizime yazılmış bu senaryonun tablolarıdır, her biri tabanın ya da senaryonun o zamandan beri değişip değişmediğiyle (eskiyen tablo kendiliğinden güncellenmez). Salt okunur. Retler: invalid_input (impervious.<kullanım> 0–1 dışında, access.distance 0'dan büyük değil, access.from/to boş, access.connect ağla verilmemiş ya da 0'dan büyük değil, slope.threshold 0'dan büyük değil, slope.classes), scenario_not_found, nothing_to_compare, not_a_road_project, unknown_terrain, terrain_unbuildable (slope), unknown_network, invalid_network, tools_unavailable (access.network)."
                .into(),
            aliases: vec!["senaryoolculeri".into(), "alternatifetkisi".into(), "senaryoetkisi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ScenariosImpact>(),
            output: schema::<crate::ScenariosImpacted>(),
            plan: Some(schema::<crate::ScenariosImpacted>()),
            capabilities: None,
            examples: vec![CommandExample { title: "Bir senaryoyu mevcut durumla".into(), input: json!({ "scenario": "alternatif-a" }), output: None }],
        },
        // Ölçüleri çizime yaz (plan/map-cad.md MAP-10, docs/adr/5060).
        CommandDescriptor {
            id: crate::CAD_SCENARIOS_RECORD.into(),
            version: crate::CAD_SCENARIOS_RECORD_VERSION,
            title: "Ölçüleri çizime yaz".into(),
            summary: "cad.scenarios.impact'in karşılaştırmasını (impact, onun girdisi) çizime tablo olarak yazar: başlık (ölçü ve senaryonun adı, birim, mevcut durum, senaryo, fark), her ölçü bir satır (alanlar projenin alan basamağıyla, uzunluklar uzunluk basamağıyla, paylar bir basamak, sayılar tam; fark işaretli), son satırda taban ve senaryo damgaları. Senaryonun kimliği ve iki damga tablonun öznitelikleridir (SenaryoKimlik, TabanDamgasi, SenaryoDamgasi): senaryo yeniden karşılaştırılınca records tablonun eskiyip eskimediğini söyler; tablo kendiliğinden güncellenmez. \
                      layerId karşılaştırılan katmanlardan biri olamaz (tablo ölçüleri değiştirir); at tablonun sol üst köşesi (yoksa karşılaştırılan nesnelerin sağında, tablo yüksekliğinin iki katı, üstleri hizasında). Tek geri alma adımı “Ölçüleri çizime yaz”. Retler: karşılaştırmanınkiler (yolları impact. altında), invalid_input (layerId: katman yok, grup ya da karşılaştırılan katman; at sonlu değil), invalid_revision, revision_conflict, layer_locked."
                .into(),
            aliases: vec!["olculeriyaz".into(), "senaryotablosu".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ScenariosRecord>(),
            output: schema::<crate::ScenariosRecorded>(),
            plan: Some(schema::<crate::ScenariosRecorded>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Senaryonun ölçüleri Senaryo ölçüleri katmanına".into(),
                input: json!({ "impact": { "scenario": "alternatif-a" }, "layerId": "senaryo-olculeri" }),
                output: None,
            }],
        },
        // İmar planı denetimi (plan/map-cad.md MAP-09, docs/adr/1420).
        CommandDescriptor {
            id: crate::CAD_ZONING_CHECK.into(),
            version: crate::CAD_ZONING_CHECK_VERSION,
            title: "İmar planı denetimi".into(),
            summary: "İmar planını sembolden değil KentOS'un imar-plani profilinin tipli nesnelerinden denetler: plan sınırı (kademe, üst plan), karar alanı (kullanım, TAKS, KAKS, kat adedi/Hmax, planda verilmeyen değerler, plan notları), plan notu (no, kapsam). \
                      Bulgular: missingLayer (profil katmanı yok), noLevel, upperMissing, levelOrder (üst plan üst kademede değil), outsideUpper (plan sınırı üst planının sınırı dışına taşıyor), noPlan, noUse, unknownUse, missingValue (verilmemiş ve Planda verilmeyen'de de olmayan değer; boş değer 0 sayılmaz), declaredValued, kaksReview (KAKS > TAKS × kat adedi, gözden geçir), overlap (iki karar aynı yerde: karar çelişkisi), uncovered (plan sınırında kararsız kısım), outside (karar plan dışına taşıyor), roadGap (Yollar katmanı varsa: yol aksının ucu ne başka bir yola ne plan sınırına bağlanıyor, kopuk yol), roadWidth (genişliği yazılmış yolun aksının iki yanında genişliğin yarısı kadar şeride yolun planının yol olmayan bir kararı giriyor; yolun planı en uzun kenarının ortasını içine alan en alt kademeli plan), roadParcel (aynı şeride parsel \
                      katmanındaki bir parsel giriyor, hangi planda olursa olsun), noteMissing, noteUnused. Alan ölçen bulgu area taşır; roadWidth ve roadParcel sayılan örtüşme parçalarını regions'ta taşır (her bölgenin dış halkası saat yönünün tersine, delikleri saat yönünde, en alttaki köşeden; bölgeler ilk köşelerine göre). tolerance (m, ilk değer 0,01) bundan dar parçayı saymaz. \
                      symbolMismatch: karar alanının sembolü kararının sembolü değil (projenin saklı seçimi, yoksa KentOS'un tablosu); draftStale: plan değişikliği taslağı (cad.zoning.draft) yazıldıktan sonra karar alanının değeri değişti, alan ya da plan çizimde yok. \
                      Salt okunur; Bakanlığın MPYY/PlanGML şemasına ya da mevzuata uygunluk iddiası değildir. Ret: invalid_input (tolerance)."
                .into(),
            aliases: vec!["imardenetim".into(), "plandenetim".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ZoningCheck>(),
            output: schema::<crate::ZoningChecked>(),
            plan: Some(schema::<crate::ZoningChecked>()),
            capabilities: None,
            examples: vec![CommandExample { title: "Çizimin imar planı".into(), input: json!({}), output: None }],
        },
        // Sembolü karardan ver (plan/map-cad.md MAP-09, docs/adr/4240).
        CommandDescriptor {
            id: crate::CAD_ZONING_SYMBOLIZE.into(),
            version: crate::CAD_ZONING_SYMBOLIZE_VERSION,
            title: "Sembolü karardan ver".into(),
            summary: "Karar alanlarına MPYY sembolünü kararlarından verir, tersini hiç yapmaz: sembol alanın kullanımından (Kullanim, imar-plani profilinin plan-kullanim listesi) ve planının kademesinden (plan sınırının Seviye'si) seçilir. KentOS'un tablosu (catalog/v1/profiles/imar-plani-semboller.json) kademenin MPYY alan sembollerinden adı kullanımın adıyla aynı yazılan tek sembolü verir; birebir eşi olmayan kullanım açık kalır (unmapped), sembolünü symbols ile kullanıcı seçer (kademenin alan sembollerinden; KentOS tahmin etmez). \
                      Yalnız nesnenin kitaplık sembolü yazılır; karar (öznitelikler) hiç değişmez, sonra değişen sembol kararı değiştirmez. uids yoksa profilin karar katmanının bütün alanları. Her alanın durumu: set, same, unmapped, noPlan, noLevel, noUse, unknownUse; önceki sembolü was. Tek geri alma adımı “Sembolü karardan ver”. \
                      Kullanıcının seçimleri projede saklanır: keep verilen seçimleri projenin saklı seçimleri yapar (aynı geri alma adımında; seçim verilmezse proje seçim saklamaz, eski saklılar okunmaz) ve kept onları verir; keep yoksa saklı seçimler verilenlerden sonra, tablodan önce okunur (from project). Kademenin sembollerinden olmayan saklı seçim kullanılmaz, uyarı kept_symbol_ignored. \
                      Retler: invalid_revision, revision_conflict, invalid_input (seçimin kademesi ya da kullanımı, aynı çift iki kez; aynı alan iki kez), unknown_plan_symbol, layer_not_found (karar katmanı yok), invalid_uid, entity_not_found, not_a_decision (uids[i]), layer_locked."
                .into(),
            aliases: vec!["sembolkarar".into(), "imarsembol".into(), "mpyysembol".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ZoningSymbolize>(),
            output: schema::<crate::ZoningSymbolized>(),
            plan: Some(schema::<crate::ZoningSymbolized>()),
            capabilities: None,
            examples: vec![
                CommandExample { title: "Bütün karar alanları KentOS'un tablosuyla".into(), input: json!({}), output: None },
                CommandExample {
                    title: "Konut alanının sembolü kullanıcıdan".into(),
                    input: json!({ "symbols": [{ "level": "uip", "use": "konut", "symbol": "mpyy.uip.konut.gelisme-konut-alani" }] }),
                    output: None,
                },
                CommandExample {
                    title: "Seçim projede saklanır".into(),
                    input: json!({ "symbols": [{ "level": "uip", "use": "konut", "symbol": "mpyy.uip.konut.gelisme-konut-alani" }], "keep": true }),
                    output: None,
                },
            ],
        },
        // Plan değişikliği taslağı (plan/map-cad.md MAP-09, docs/adr/5040).
        CommandDescriptor {
            id: crate::CAD_ZONING_DRAFT.into(),
            version: crate::CAD_ZONING_DRAFT_VERSION,
            title: "Plan değişikliği taslağı".into(),
            summary: "İmar planının değişikliği için projede taslak saklar (id; aynı kimlikteki taslağın yerine geçer) ya da siler (remove): plan (plan sınırının PlanKimlik'i), gerekçe (reason; Mekânsal Planlar Yapım Yönetmeliği m. 9(4) değişiklik gerekçesini açıklayan plan raporunu ister) ve değişiklikler (changes: planın karar alanı uid, alanı field: Kullanim, Nizam, TAKS, KAKS, KatAdedi, Hmax, OnCekme, YanCekme, ArkaCekme; yeni değeri value). Taslak çizimin sürümünü ve her değerin öncesini saklar; yürürlükteki karar alanları hiç değişmez. \
                      Cevap planın alanlarını kullanıma göre önce ve sonra verir (uses, m²) ve m. 26'nın sorduklarını söyler: infrastructureLess (sosyal ya da teknik altyapı kullanımının alanı küçülüyor, m. 26(3); eşdeğerlik değerlendirilmez), densityUp (KAKS ya da kat adedi artıyor, m. 26(5)(a)), heightUp (kat adedi ya da Hmax artıyor, m. 26(4), (5)(b)). Taslak onaylı değişiklik değildir: onay, plan işlem numarası ve askı KentOS dışındadır. Sonradan değişen karar alanını cad.zoning.check draftStale ile söyler. Tek geri alma adımı. \
                      Retler: invalid_revision, revision_conflict, invalid_input, unknown_draft (remove), layer_not_found, unknown_plan, invalid_uid, entity_not_found, not_a_decision (changes[i])."
                .into(),
            aliases: vec!["plandegisikligi".into(), "imartaslak".into(), "plantaslak".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ZoningDraft>(),
            output: schema::<crate::ZoningDrafted>(),
            plan: Some(schema::<crate::ZoningDrafted>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Park alanının ticarete dönmesi".into(),
                    input: json!({ "id": "T-1", "plan": "UİP-1", "reason": "Gerekçe: …", "changes": [{ "uid": "0192f5c0-5040-7000-8000-000000000005", "field": "Kullanim", "value": "ticaret" }] }),
                    output: None,
                },
                CommandExample { title: "Taslağı sil".into(), input: json!({ "id": "T-1", "remove": true }), output: None },
            ],
        },
        // Saha paketi (plan/map-cad.md MAP-19, docs/adr/1920).
        CommandDescriptor {
            id: crate::CAD_FIELD_PACKAGE.into(),
            version: crate::CAD_FIELD_PACKAGE_VERSION,
            title: "Saha paketi".into(),
            summary: "Tasarımdan sahaya gidecek paketi tek .zip olarak yazar (path; overwrite): saha-paketi.json (kentos.field-package v1: proje, stamp, çizimin sürümü, tasarım özeti, koordinat sistemi EPSG ve adıyla, epok, kot sistemi, verilen kabul sınırları, kalite notları, yüzey kimliği, adı ve kaynak özetiyle, dosyalar), noktalar.csv (Ad;Y;X;Z;Kimlik: Y doğu, X kuzey; sayılar yuvarlanmadan) ve cizgiler.csv (Kimlik;Sıra;Y;X;Bulge). \
                      Tasarım cad.asbuilt.check'in okuduğu gibidir (design: noktaların katmanı, adları field'da, yoksa Ad; lines: çizgilerin katmanı; en az biri; surface: yüzey) ve özeti onunkidir (designDigest): pakette ölçülen kayıt expectedDesign ile denetlenir, eski tasarımla ölçülen kayıt yeni tasarıma sessiz bağlanmaz. Kabul sınırları (horizontal, vertical, quality; m) kullanıcınındır; verilmeyen yazılmaz. Dosya ya bütün yazılır ya hiç; çizim değişmez. \
                      Retler: invalid_field_package, layer_not_found, not_a_road_project, unknown_terrain, terrain_unbuildable, invalid_file_path, file_exists, file_unwritable."
                .into(),
            aliases: vec!["sahapaketi".into(), "aplikasyonpaketi".into(), "fieldpackage".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::FieldPackage>(),
            output: schema::<crate::FieldPackaged>(),
            plan: Some(schema::<crate::FieldPackaged>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Tasarım noktaları ve bordür, 2 cm yatay, 3 cm düşey kabul".into(),
                input: json!({ "design": "tasarim", "lines": "bordur", "horizontal": 0.02, "vertical": 0.03, "path": "/tmp/saha.zip" }),
                output: None,
            }],
        },
        // Uygulama kontrolü (plan/map-cad.md MAP-19, docs/adr/1700).
        CommandDescriptor {
            id: crate::CAD_ASBUILT_CHECK.into(),
            version: crate::CAD_ASBUILT_CHECK_VERSION,
            title: "Uygulama kontrolü".into(),
            summary: "Sahada ölçülen noktaları (measured katmanının noktaları, adları field'da, yoksa Ad; kotları) tasarımla karşılaştırır (salt okunur): tasarım noktaları (design katmanı; önce adla, ad tutmazsa ve search verilmişse search yarıçapındaki tek tasarım noktasıyla öneri olarak, iki aday belirsiz), tasarım yüzeyi (surface; Harita/Yol; ölçülen − yüzey kotu) ve tasarım çizgileri (lines katmanı; en yakın çizgiye işaretli dik uzaklık, çizgi yönünün solu artı). \
                      Her nokta: eşlenen tasarım noktası ve nasıl (name, place), Δdoğu, Δkuzey, Δkot (ölçülen − tasarım), yatay uzaklık, yüzey farkı, çizgi uzaklığı, σ; durum within, outside, unmatched, ambiguous; işaretler proposed, outsideHorizontal, outsideVertical, outsideSurface, outsideLine, lowQuality (σ > quality; durumu değiştirmez), noHeight, offSurface. Ölçülmeyen tasarım noktaları missing; eşlenenlerin yatay ve düşey KOH'u ve en büyüğü. \
                      Toleranslar (horizontal, vertical), arama yarıçapı ve kalite sınırı kullanıcınındır; KentOS değer vermez, verilmeyen denetlenmez. Tasarım özeti (designDigest: noktalar ve çizgiler, SHA-256) cevaptadır; expectedDesign verilir ve tutmazsa designChanged: eski tasarımla ölçülen kayıt yeni tasarıma sessizce bağlanmaz. \
                      Saha paketiyle (package: cad.field.package'ın .zip'i, mutlak yol): paketin tasarım özeti beklenen özet olur, kabul sınırları girdi vermediyse uygulanır (kalite sınırı qualityField ile), sistemi, epoku ve kot sistemi projeninkiyle karşılaştırılır; cevapta package (yol, tarih, revizyon, özet, sameSystem, sameEpoch, sameHeights, sınırlar, taken). packageDesign ile kayıt paketin kendi tasarımıyla (noktalar.csv, cizgiler.csv; ekibin elindeki tasarım) denetlenir: dosyalar paketin özetiyle tutmalı, verilen design ve lines katmanları yalnız çizimdeki tasarımın paketten sonra değişip değişmediğini söyler; cevapta package.design. \
                      İzleme işinin dönem dizisinde (epochs) istenirse: trend.confidence ile her noktanın günlü dönemlerinden (en az 3) en küçük kareler doğrusu (tracks[].fit: doğu, kuzey, kot hızı m/yıl, σ'ları s/√Sxx, yatay hız; moving: |v| > t(n − 2; (1 + güven)/2)·σ olan bileşenler) ve thresholds (warning, alarm: yatay ve düşey hareket m, yatay ve düşey hız m/yıl; uyarı alarmdan küçük) ile her noktanın state'i (normal, warning, alarm) ve aştığı sınırlar (exceeded); hız doğrununki, doğru yoksa ortalama hız. Eşikler ve güven kullanıcınındır; izleme için yönetmelik değeri yoktur. \
                      Retler: invalid_asbuilt_check, invalid_file_path (paket yolu), file_not_found (paket yok), layer_not_found, not_a_road_project, unknown_terrain, terrain_unbuildable."
                .into(),
            aliases: vec!["uygulamakontrolu".into(), "asbuilt".into(), "aplikasyonkontrolu".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::AsbuiltCheck>(),
            output: schema::<crate::AsbuiltChecked>(),
            plan: Some(schema::<crate::AsbuiltChecked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ölçülen noktalar tasarım noktalarına ve tasarım yüzeyine karşı".into(),
                input: json!({ "measured": "olcum", "design": "tasarim", "surface": "Tasarım" }),
                output: None,
            }],
        },
        // İzleme incelemesi (plan/map-cad.md MAP-19, docs/adr/5220).
        CommandDescriptor {
            id: crate::CAD_ASBUILT_REVIEW.into(),
            version: crate::CAD_ASBUILT_REVIEW_VERSION,
            title: "İzleme incelemesi".into(),
            summary: "Bir izleme işinin denetimini (check: cad.asbuilt.check'in kind: monitoring girdisi) inceleyenin beyanıyla projede bir adla saklar: iş (ölçülen ve önceki dönemin katmanı, dönem dizisi ve günleri, bu dönemin günü, ad özniteliği, kontrol noktaları, sınırlar, doğrunun güveni), eşikler ve kaynakları (thresholdsSource: kim, hangi belge ya da kararla, ne zaman; eşik verildiyse gerekir), her noktanın son yatay ve düşey hareketi, yatay hızı (doğrununki, yoksa ortalama), durumu ve aştığı sınırlar; işin okuduğu noktaların özeti (digest: SHA-256). \
                      İnceleyen (reviewer): rolü (responsible: sorumlu mühendis, control: kontrol mühendisi, staff: kontrolü yapan teknik personel, approver: onaylayan), unvanı, adı, günü (YYYY-AA-GG), ibaresi (checked: “Kontrol edilmiştir”, seen: “Görülmüştür”), imza türü (wet: ıslak, electronic: güvenli elektronik imza) ve notu. KentOS inceleyenin yetkisini ya da imzasını doğrulamaz; beyanı yazar. \
                      Aynı ad yenisiyle değişir (replaced); remove adı verilen incelemeyi kaldırır. Cevap bütün incelemeleri listeler: inceleyen, gün, ibare, alarm ve uyarıdaki nokta sayısı ve current (işin okuduğu noktalar incelenenlerle aynı mı; değiştiyse inceleme eski sonuca aittir). Tek geri alma adımı “İzleme incelemesi”. \
                      Retler: invalid_revision, revision_conflict, invalid_input (ad; check ya da reviewer eksik; remove ile birlikte check, reviewer ya da kaynak; izleme işi olmayan check; eşiksiz kaynak ya da kaynaksız eşik; inceleyenin unvanı, adı, günü ya da notu), unknown_monitoring_review (kaldırılacak inceleme yok) ve check'in retleri (invalid_asbuilt_check, layer_not_found)."
                .into(),
            aliases: vec!["izlemeincelemesi".into(), "kontroledilmistir".into(), "monitoringreview".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::AsbuiltReview>(),
            output: schema::<crate::AsbuiltReviewed>(),
            plan: Some(schema::<crate::AsbuiltReviewed>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dört dönemli izlemeyi eşikleri ve kaynağıyla incele".into(),
                    input: json!({
                        "name": "Ekim 2026 incelemesi",
                        "check": { "measured": "Dönem 4", "design": "Dönem 3", "kind": "monitoring",
                                   "epochs": [{ "layer": "Dönem 1", "date": "2026-01-15" }], "date": "2026-10-15",
                                   "thresholds": { "warning": { "horizontal": 0.01 }, "alarm": { "horizontal": 0.02 } } },
                        "thresholdsSource": "İdarenin izleme şartnamesi, 2026",
                        "reviewer": { "role": "control", "title": "Harita Mühendisi", "name": "A. Yılmaz", "date": "2026-10-20",
                                      "phrase": "checked", "signature": "wet" }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "İncelemeyi kaldır".into(),
                    input: json!({ "name": "Ekim 2026 incelemesi", "remove": true }),
                    output: None,
                },
            ],
        },
        // Boru hattı denetimi (plan/map-cad.md MAP-16, docs/adr/1640).
        CommandDescriptor {
            id: crate::CAD_PIPES_CHECK.into(),
            version: crate::CAD_PIPES_CHECK_VERSION,
            title: "Boru hattı denetimi".into(),
            summary: "Projenin bir altyapı ağının borularını cazibeli hat olarak denetler (salt okunur): akar kotları (iki öznitelik ya da köşe kotları), akış yönünde eğim (ağın yön alanı), iç çap (öznitelik ve birimi; isteğe bağlı çap kataloğu), tepe üstü örtü (Harita/Yol yüzeyinden her üçgen kenarı kesişiminde, ya da bacaların kapak kotu özniteliğinden uçlarda). \
                      Bulgular: detached (en uzun parçaya bağlı değil), noStructure (bacasız uç), longSpan (iki baca arası kural paketinin en çok aralığından uzun), noInvert, invertUnit (kot cm/mm yazılmış olabilir), invertIsDepth (kot yerine derinlik), reverseSlope, flat, lowSlope, exceptionalSlope (eğim istisnai en az ile en az arasında), steepSlope, outletAbove (çıkış en alçak girişten yüksek), noDiameter, diameterUnit, notInCatalog, noRule (çap kural paketinin hiçbir sınıfında yok), noWall (et kalınlığı da dış çap da yok), wallMismatch (dış çap iç çaptan küçük ya da iç çap + 2 × et değil), noGround, offSurface, aboveGround (tepe zeminin üstünde), shallow, deep, curved (yaylı boruda örtü yalnız köşelerde). \
                      İstenirse (clearance) başka bir altyapı ağının borularına açıklık: planda kesiştikleri yerde düşey açıklık (üstteki borunun akarı eksi alttakinin tepesi; crossings), kesişmeden geçtiklerinde en az yatay açıklık (eksenler arası en kısa uzaklık eksi iki yarıçap; near); bulgular clash (çakışma), closeCrossing (düşey açıklık en azdan küçük), wrongSide (üstte olması istenen boru altta), closeAlongside (yatay açıklık en azdan küçük). Çaplar iç çaptır; diameter.wall et kalınlığının özniteliğini verirse (çapın biriminde) örtü borunun dış tepesinden (akar + çap + et), açıklıklar dıştan dışa ölçülür, et kalınlığı olmayan boru noWall ile söylenir ve iç tepeden ölçülür; verilmezse et bilinmez. diameter.outer dış çapın özniteliğini verirse et dış çap eksi iç çapın yarısıdır; ikisi birlikte sorulursa uyuşmaları (dış = iç + 2 × et, dış çapın milyarda biri kadar) denetlenir, uyuşmayan borunun eti hiçbirinden alınmaz (wallMismatch). \
                      Sınırlar (minCover, maxCover, minSlope, maxSlope, clearance.vertical, clearance.horizontal) kullanıcınındır; program kendiliğinden sınır koymaz, verilmeyen sınır denetlenmez. rules istenirse yazılmayan sınırlar bir kural paketinin sisteminden alınır: KentOS'unki (catalog/v1/rules/kanal-csb-2017.json) Atıksu Toplama ve Uzaklaştırma Sistemleri Hakkında Yönetmelik EK-1'i (RG 06.01.2017/29940; atiksu: çap sınıfına göre 1:A eğimler, en az örtü 2,70 m, baca aralığı) ve Yağmursuyu Toplama, Depolama ve Deşarj Sistemleri Hakkında Yönetmelik EK-1 Çizelge 1.4'üdür (RG 23.06.2017/30105; yagmursuyu: her çapın en az, istisnai en az ve en çok eğimi, en az örtü 1,2 m); rules.package başka bir paket verir. Her borunun sınıfı cevapta (class). Yönetmeliğin dili “olabilir”, “alınabilir”dir: bulgu uygunluk kararı değildir. Birim hatası önerilir, düzeltilmez. \
                      Retler: invalid_pipe_check, unknown_network, not_a_pipe_network, not_a_road_project, unknown_terrain, terrain_unbuildable, invalid_network, tools_unavailable."
                .into(),
            aliases: vec!["borudenetimi".into(), "kanaldenetimi".into(), "boruortusu".into(), "tersegim".into(), "boruacikligi".into(), "altyapicakismasi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::PipesCheck>(),
            output: schema::<crate::PipesChecked>(),
            plan: Some(schema::<crate::PipesChecked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Atıksu ağı, akar kotları özniteliklerde, örtü Zemin yüzeyinden".into(),
                input: json!({
                    "network": "atiksu",
                    "inverts": {"from": "fields", "start": "Akar baş", "end": "Akar son"},
                    "diameter": {"field": "Çap", "unit": "mm"},
                    "ground": {"from": "surface", "surface": "Zemin"}
                }),
                output: None,
            }, CommandExample {
                title: "Atıksu ağı, et dış çaptan, sınırlar atıksu yönetmeliğinin EK-1'inden".into(),
                input: json!({
                    "network": "atiksu",
                    "inverts": {"from": "fields", "start": "Akar baş", "end": "Akar son"},
                    "diameter": {"field": "Çap", "unit": "mm", "outer": "Dış çap"},
                    "ground": {"from": "surface", "surface": "Zemin"},
                    "rules": {"system": "atiksu"}
                }),
                output: None,
            }],
        },
        // Hat profili (plan/map-cad.md MAP-16, docs/adr/1860).
        CommandDescriptor {
            id: crate::CAD_PIPES_PROFILE.into(),
            version: crate::CAD_PIPES_PROFILE_VERSION,
            title: "Hat profili".into(),
            summary: "Bir altyapı ağında cazibeli hattın boykesitini (kanal boykesiti) bir katmana (layerId) çizer. Hat başlangıç borusundan (start, uid) akış yönünde (ağın yön alanı) ilerler; tek borunun çıktığı her düğümden geçer, istenen bitiş borusunda (end), çıkışta (outlet) ya da çatalda (split) biter. \
                      Değerler cad.pipes.check'inkidir: akar kotları (iki öznitelik ya da köşe kotları), çap (öznitelik ve birimi), zemin (yüzeyin her üçgen kenarı kesişimi ya da bacaların kapak kotu); her borunun akar ve tepe kotları, eğimi, çapı, en az örtüsü; her düğümde baca (Ad), zemin, giren ve çıkan akar kotu, derinlik. Çapta et (diameter.wall) ya da dış çap (diameter.outer) sorulursa eti bilinen her borunun eti cevapta (wall), örtüsü dış tepeden, eti 0'dan büyükse dış duvarları (akarın et kadar altı ve tepenin et kadar üstü) ince kırmızı çizgilerle çizilir; et ile dış çapı uyuşmayan borunun dış duvarı çizilmez. \
                      Çizim road.profiles.draw'unki gibidir: at çerçevenin sol altı (hattın başı kıyas kotunda), bir metre mesafe bir birim, bir metre kot exaggeration birim (yoksa 10), yazılar height kâğıt mm (yoksa 2,5), kıyas kotu en alçak kotun bir metre altı; kot ızgarası, zemin, akar ve tepe çizgileri (kırmızı), bacalar; çerçevenin altında bantlar: Baca, Mesafe, Zemin kotu, Akar giriş, Akar çıkış, Derinlik, Eğim ‰, Çap mm, Boy. Tek geri alma adımı “Hat profili”. Harita ve Yol projesinde çizim ağıyla tutulur (zemin yüzeyi ve kesişen ağ da kaynağıdır): ağın bir nesnesi ya da tanımı değişince Proje ağacında güncel değil olur, road.project.rebuild ya da kendiliğinden yeniden üretim aynı girdiyle yeniden çizer. \
                      Retler: invalid_pipe_check, unknown_network, not_a_pipe_network, not_a_road_project, unknown_terrain, terrain_unbuildable, invalid_network, tools_unavailable, invalid_pipe_run, layer_not_found, not_a_layer, layer_locked, invalid_revision, revision_conflict."
                .into(),
            aliases: vec!["hatprofili".into(), "kanalboykesiti".into(), "boruboykesiti".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::PipesProfile>(),
            output: schema::<crate::PipesProfiled>(),
            plan: Some(schema::<crate::PipesProfiled>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Atıksu hattının boykesiti, zemin Zemin yüzeyinden".into(),
                input: json!({
                    "network": "atiksu",
                    "start": "0192f640-1640-7000-8000-0000000000e5",
                    "inverts": {"from": "fields", "start": "Akar baş", "end": "Akar son"},
                    "diameter": {"field": "Çap", "unit": "mm"},
                    "ground": {"from": "surface", "surface": "Zemin"},
                    "layerId": "profil",
                    "at": [487000.0, 4419700.0]
                }),
                output: None,
            }],
        },
        // Parsel alan denetimi (plan/map-cad.md MAP-08, docs/adr/1400).
        CommandDescriptor {
            id: crate::CAD_PARCELS_CHECK.into(),
            version: crate::CAD_PARCELS_CHECK_VERSION,
            title: "Parsel alan denetimi".into(),
            summary: "Her parselin koordinatlardan hesaplanan alanını (yay ve delikler dahil) kayıtlı alanıyla (Tapu alanı (m²), yazıldığı gibi; düz ondalık okunur, öbür yazı invalidRecord) karşılaştırır: fark (hesap − kayıt) ve yanılma sınırı, Tapu Planlarında Yanılma Sınırının Belirlenmesi Hakkında Yönetmelik (RG 27.09.2022/31966) Madde 8(2)'nin arşiv onaylı geometri tablosundan (artan oranlı dilimler; küsurat alt dilimden). Durum: within, outside, noRecord, invalidRecord. \
                      Kimlik İl/İlçe/Mahalle/Ada/Parsel birlikte: yalnız parsel numarası değil; iki parselin aynı kimliği duplicates, Ada ya da Parsel'i olmayan noIdentity. uids yoksa parsel katmanının (parsel) bütün alanları. Salt okunur: hesap alanı kayıtlı alanın üstüne yazılmaz; sonuç resmî uygunluk iddiası değildir. \
                      Retler: invalid_uid, entity_not_found, not_a_parcel (uids[i])."
                .into(),
            aliases: vec!["parselalan".into(), "yanilmasiniri".into(), "tecviz".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ParcelsCheck>(),
            output: schema::<crate::ParcelsChecked>(),
            plan: Some(schema::<crate::ParcelsChecked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Parsel katmanının bütün parselleri".into(),
                input: json!({}),
                output: None,
            }],
        },
        // Sınır grafiği (plan/map-cad.md MAP-08, docs/adr/2000).
        CommandDescriptor {
            id: crate::CAD_PARCELS_BOUNDARIES.into(),
            version: crate::CAD_PARCELS_BOUNDARIES_VERSION,
            title: "Sınır grafiği".into(),
            summary: "Parsellerin sınırlarını bir grafik olarak okur; ortak sınır iki parselin tesadüfen çakışan koordinatları değil, iki parselin bir sınırıdır. Köşeler (K1…: bir halkanın köşesinin olduğu yer, koordinatlar saklandığı gibi aynı; doğu sonra kuzey sırasıyla), sınırlar (S1…: iki köşe arasındaki kenar, aynı kabarıklıkla bir kez; sol ve sağındaki parsel, kenarın kendi halkasının yönünden; boy), parseller (kimlik İl/İlçe/Mahalle/Ada/Parsel, geometrik alan, sınırları halka sırasıyla, komşuları) ve sorunlar: missingVertex (bir köşe başka bir sınırın içinde 1 µm'de, o sınırın orada köşesi yok: sınır yalnız kısmen ortak; düz sınırda dikmesinin ayağı uçların arasında, yaylı sınırda yayın çemberinde ve kirişin yay yanında), splitCorner (1 µm'den yakın iki ayrı köşe: tesadüfen çakışan koordinat), overlap (üç ya da daha çok parselin, ya da bir yanda iki parselin kenarı), arcChord (aynı iki köşe arasında yay ile kiriş ya da iki farklı yay), approxArc (yaylı sınırı komşu kirişlerle çizmiş: yayın bir köşesinden öbürüne, köşeleri yayın üstünde (1 µm'de ya da arcTolerance metrede) duran sınır zinciri; yay ile zincir arasındaki şeridin alanı ve en geniş yeri; zincirin köşeleri ayrıca missingVertex sayılmaz). \
                      Kimlikler geometriden türetilir (aynı geometri aynı kimlikleri verir), kalıcı kayıt değildir. uids yoksa parsel katmanının (parsel) bütün alanları. Salt okunur; geometrik alan kayıtlı alanın üstüne yazılmaz. \
                      Retler: invalid_uid, entity_not_found, not_a_parcel (uids[i]), invalid_input (arcTolerance eksi ya da sonlu değil)."
                .into(),
            aliases: vec!["sinirgrafigi".into(), "parselsinir".into(), "ortaksinir".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ParcelsBoundaries>(),
            output: schema::<crate::ParcelsBounded>(),
            plan: Some(schema::<crate::ParcelsBounded>()),
            capabilities: None,
            examples: vec![
                CommandExample { title: "Parsel katmanının sınır grafiği".into(), input: json!({}), output: None },
                CommandExample {
                    title: "Santimetreye yuvarlanmış komşunun kirişlerle çizdiği yaylar da".into(),
                    input: json!({ "arcTolerance": 0.01 }),
                    output: None,
                },
            ],
        },
        // Parsel kaydı (plan/map-cad.md MAP-08, docs/adr/4220).
        CommandDescriptor {
            id: crate::CAD_PARCELS_RECORD.into(),
            version: crate::CAD_PARCELS_RECORD_VERSION,
            title: "Parsel kaydı".into(),
            summary: "Çizimin parsel kayıtlarına bir belge yazar: belgenin adı (çizimde, taslaklarla birlikte tek), türü, tarihi (yazıldığı gibi) ve notu. Belge alanlardan parsel oluşturur (create: kimlik İl/İlçe/Mahalle/Ada/Parsel alanın özniteliklerinden, belgedeki alan statedArea ya da alanın Tapu alanı (m²)'si, düz ondalık olarak yazıldığı gibi; halkalar sınır zinciri olarak) ve önceki parselleri kapatır (close: numaraları; kapanan parsel tarihçe olarak kalır). \
                      Halka kenarının iki köşesi (saklandığı gibi) ve yayı güncel bir sınırınkiyle aynıysa (1e-9) o sınırdır: iki parsel onu numarasıyla paylaşır; hiçbir güncel parselin kullanmadığı sınırı belge kapatır. Belgenin yeni köşeleri güncel bir komşunun düz ya da yaylı sınırının üstüne (1 µm) düşüp parçalar belgenin sınırlarıysa sınır bölünür, komşu parçaları alır. \
                      Belge kenarların ölçülerini yazar (create[i].lengths: halka, kenar, metre olarak düz ondalık; yoksa alanın Kenar ölçüleri (m) özniteliği, dış halkanın kenarları sırayla, ; ile ayrılmış): ölçü belgenindir, sınırın boyu değişince değişmez. draft ile belge taslak olarak yazılır: güncel hiçbir şey değişmez, cevap onayın bugün yazacağıdır; taslağı cad.parcels.approve yazar. \
                      Alanlara dokunulmaz: kayıt çizimden ayrıdır; sonra kenar taşınırsa geometrik alan değişir, belgedeki alan ve kayıttaki şekil değişmez. Tek geri alma adımı “Parsel kaydı”. \
                      Retler: invalid_revision, revision_conflict, invalid_input (belgenin adı, türü, tarihi ya da notu; oluşturacak ya da kapatacak parsel yok; aynı numara ya da alan iki kez; 120 karakteri aşan kimlik; aynı yerde iki köşe; düz ondalık olmayan alan; olmayan ya da iki kez verilen kenar, sıfırdan büyük düz ondalık olmayan kenar ölçüsü; kayıtların sınırları), document_name_taken, unknown_parcel_record, parcel_closed (close[i]), invalid_uid, entity_not_found, not_a_parcel, no_parcel_identity, parcel_recorded (alan kapatılmayan güncel bir parseli çiziyor), parcel_identity_taken (create[i])."
                .into(),
            aliases: vec!["parselkaydi".into(), "parselbelgesi".into(), "parcelrecord".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ParcelsRecord>(),
            output: schema::<crate::ParcelsRecorded>(),
            plan: Some(schema::<crate::ParcelsRecorded>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İfraz: 101/7 kapanır, iki yeni parsel oluşur".into(),
                    input: json!({
                        "document": { "name": "2026/145 ifraz", "kind": "Değişiklik", "date": "2026-03-14" },
                        "create": [
                            { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "statedArea": "412.35" },
                            { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12", "statedArea": "311.17" }
                        ],
                        "close": [1]
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Taslak: kenar ölçüleriyle, onaylanınca yazılır".into(),
                    input: json!({
                        "document": { "name": "2026/146 ifraz", "kind": "Değişiklik" },
                        "create": [{
                            "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11",
                            "lengths": [{ "edge": 0, "length": "20,00" }, { "edge": 1, "length": "10,00" }]
                        }],
                        "close": [1],
                        "draft": true
                    }),
                    output: None,
                },
            ],
        },
        // Belgenin onayı (plan/map-cad.md MAP-08, docs/adr/5020).
        CommandDescriptor {
            id: crate::CAD_PARCELS_APPROVE.into(),
            version: crate::CAD_PARCELS_APPROVE_VERSION,
            title: "Belgenin onayı".into(),
            summary: "Parsel kayıtlarındaki bir belgenin dışarıda verilen onayını yazar: onay sayısı (boş olmaz), tarihi ve onaylayan, yazıldığı gibi. KentOS onay vermez; onayı kaydeder. \
                      draft: belge taslağı taslaktaki gibi (alanlarının taslaktaki şekli, kimliği, belgedeki alanı ve kenar ölçüleri, kapatacağı parseller) sıradaki numarayla belge olarak yazılır ve taslak kalkar; cevap cad.parcels.record'unkini taşır. document: kayıtlı bir belgenin onayı yazılır, belge değişmez. Onay bir kez yazılır. Tek geri alma adımı “Belge onayı”. \
                      Retler: invalid_revision, revision_conflict, invalid_input (draft ile document'tan biri verilmeli; onay sayısı boş ya da 64 karakteri aşıyor; tarih ya da onaylayan çok uzun), unknown_parcel_draft, unknown_parcel_document, document_approved; taslak bugün yazılamıyorsa cad.parcels.record'un retleri (parcel_closed: kapatacağı parsel o arada kapanmış; parcel_recorded; parcel_identity_taken; invalid_input) draft yolunda."
                .into(),
            aliases: vec!["belgeonayi".into(), "taslagionayla".into(), "parcelapprove".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ParcelsApprove>(),
            output: schema::<crate::ParcelsApproved>(),
            plan: Some(schema::<crate::ParcelsApproved>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Taslağı onay sayısıyla yaz".into(),
                    input: json!({ "draft": 1, "approval": { "reference": "2026/145-K", "date": "2026-03-20", "by": "Kadastro Müdürlüğü" } }),
                    output: None,
                },
                CommandExample {
                    title: "Kayıtlı belgenin onayını yaz".into(),
                    input: json!({ "document": 2, "approval": { "reference": "2026/77" } }),
                    output: None,
                },
            ],
        },
        // Taslağı geri çek (plan/map-cad.md MAP-08, docs/adr/5020).
        CommandDescriptor {
            id: crate::CAD_PARCELS_WITHDRAW.into(),
            version: crate::CAD_PARCELS_WITHDRAW_VERSION,
            title: "Taslağı geri çek".into(),
            summary: "Parsel kayıtlarındaki bir belge taslağını kaldırır; güncel parseller, sınırlar ve belgeler değişmez, taslağın adı yeniden kullanılabilir. Tek geri alma adımı “Taslağı geri çek”. Retler: invalid_revision, revision_conflict, unknown_parcel_draft."
                .into(),
            aliases: vec!["taslagigericek".into(), "parcelwithdraw".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ParcelsWithdraw>(),
            output: schema::<crate::ParcelsWithdrawn>(),
            plan: Some(schema::<crate::ParcelsWithdrawn>()),
            capabilities: None,
            examples: vec![CommandExample { title: "1. taslağı geri çek".into(), input: json!({ "draft": 1 }), output: None }],
        },
        // Parsel kayıtları (plan/map-cad.md MAP-08, docs/adr/4220).
        CommandDescriptor {
            id: crate::CAD_PARCELS_RECORDS.into(),
            version: crate::CAD_PARCELS_RECORDS_VERSION,
            title: "Parsel kayıtları".into(),
            summary: "Çizimin parsel kayıtlarını çizimle birlikte okur: belgeler (onayı yazıldığı gibi, oluşturdukları ve kapattıkları parseller), taslaklar (oluşturacakları kimlikler, kapatacakları, kenar ölçüsü sayısı), parseller (kimlik, belgeleri, belgedeki alan yazıldığı gibi, kayıttaki halkaların alanı, onu çizen alan ve onun bugünkü geometrik alanı; durum current: kayıttaki gibi çizili, changed: başka türlü çizili, missing: alanı yok, closed; sınırları halka sırasıyla; paylaşılan sınırdan komşuları; belgesinin kapattıkları before, onu kapatan belgenin oluşturdukları after) ve sınırlar (köşeleri, yayı, boyu, belgeleri, kullanan parseller, kayıttaki yerinde hâlâ çizili mi; belgelerin yazdığı ölçüler, kayıttaki boyla farkı ve cephe denetiminin sınırı d = 0,05 + 0,001·S m, S belgedeki ölçü, BÖHHBÜY m. 48(1); overLimit sınırı aşan güncel sınır sayısı). \
                      history ile kapanan parseller ve sınırlar da. Salt okunur; geometrik alan belgedeki ya da kayıttaki alanın üstüne yazılmaz. Ret yok: kaydı olmayan çizim boş listeler verir."
                .into(),
            aliases: vec!["parselkayitlari".into(), "parselgecmisi".into(), "parcelrecords".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ParcelsRecords>(),
            output: schema::<crate::ParcelsRecordsListed>(),
            plan: Some(schema::<crate::ParcelsRecordsListed>()),
            capabilities: None,
            examples: vec![
                CommandExample { title: "Güncel parseller ve sınırları".into(), input: json!({}), output: None },
                CommandExample { title: "Kapanan parseller de".into(), input: json!({ "history": true }), output: None },
            ],
        },
        // Topolojik temizlik, taşınma raporuyla (plan/map-cad.md MAP-07, docs/adr/1262).
        CommandDescriptor {
            id: crate::CAD_TOPOLOGY_CLEAN.into(),
            version: crate::CAD_TOPOLOGY_CLEAN_VERSION,
            title: "Topolojik temizlik (taşınma raporu)".into(),
            summary: "Çizgi ağını ve alan sınırlarını verilen toleransla düzeltir (topolojik temizliğin kuralları: Uçlar, Köşeler, Uzat, Buda; Topolojik temizlik aracıyla aynı çekirdek) ve büyük toplu temizlikte yazmadan önce taşınmayı raporlar: her taşınma (nesne, tür, nereden, nereye, uzaklık), değişen her nesnenin yerinde kalan ve taşınan kaynak köşeleri, en uzak ve toplam taşınması, alanı ya da uzunluğu önce ve sonra, alanların toplamındaki değişim. \
                      Köşe hiçbir zaman ortalamaya ya da yuvarlanmış koordinata değil var olan bir köşeye gider. Kalite profili varsa kabul eşiği ve onu aşan taşınma sayısı (beyond) söylenir; plan yine raporlar, yazma reddeder. \
                      uids yoksa görünen, kilitsiz katmandaki bütün çizgi, çoklu çizgi, yay ve alan düzeltilir; verilenlerden düzeltilemeyenler skipped sayılır. Tek geri alma adımı “Topolojik temizlik”. \
                      Retler: invalid_input (tolerance), invalid_revision, revision_conflict, invalid_uid, entity_not_found (uids[i]), clean_beyond_acceptance."
                .into(),
            aliases: vec!["topolojiktemizlik".into(), "tasinmaraporu".into(), "mapclean".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::TopologyClean>(),
            output: schema::<crate::TopologyCleaned>(),
            plan: Some(schema::<crate::TopologyCleaned>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bütün çizimi 2 cm toleransla, köşeler de".into(),
                input: json!({ "tolerance": 0.02, "vertices": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_SURVEY_BASELINES.into(),
            version: crate::CAD_SURVEY_BASELINES_VERSION,
            title: "GNSS baz ağı dengelemesi".into(),
            summary: "GNSS baz vektörlerinden kurulan ağı en küçük karelerle dengeler; hiçbir şey yazmaz (önce analiz). İstasyonlar ECEF X, Y, Z'de (m); baz ΔX, ΔY, ΔZ ve tam 3×3 kovaryansıyla (xx, xy, xz, yy, yz, zz, m²); scale kovaryansların çarpanı (varsayılan 1; işleyicinin kovaryansı çoğu kez iyimserdir). Bilinen istasyon fixed ile sabit, sigma ile ağırlıklıdır; ikisi de yoksa datum: minimal koordinatı olan ilk istasyonu tutar ve söyler (held). Her bazın kovaryansı L·Lᵀ ayrıştırılıp üç denklemi L⁻¹ ile korelasyonsuz yapılır; m0, χ² testi (%95), artık sayıları ve Baarda testi (α₀ = 0,001, w > 3,29) bu bileşenler üzerindendir, bazın sonucu bileşenlerinin en büyük test değeri. Cevap: istasyonların X, Y, Z'si ve standart sapmaları (m0 ile ölçeklenmiş) ile yerel doğu, kuzey, yukarı sapmaları (GRS80), bazların artık vektörü (hesaplanan − gözlenen), n, u, f, vᵀPv, m0, χ², geçti mi, en kötü baz. Ağ metinle de verilir: K;ad;X;Y;Z[;sX;sY;sZ], N;ad[;X;Y;Z], B;başlangıç;bitiş;dX;dY;dZ;xx;xy;xz;yy;yz;zz. Normal denklem seyrek çözülür (istasyonlar ters Cuthill–McKee sırasında, Cholesky zarfında, kovaryansın yalnız okunan girdileri); zarfı 30 milyon girdiyi aşan ağ reddedilir. robust {method: huber|danish, c} dayanıklı kestirimdir (yinelemeli yeniden ağırlıklandırma): her adımda bazın standart artığı bileşenlerinin |v|/σ'sının en büyüğü (σ verilen kovaryanstan, önsel σ0 = 1), |w| > c ise ağırlık çarpanı Huber c/|w|, Danimarka exp(−((|w| − c)/c)²); c verilmezse Huber 1,5, Danimarka 2,0; hiçbir çarpan 10⁻⁹'dan çok değişmeyince (en çok 100 adım) durur. Baz silinmez: cevapta her bazın factor'ı ve robust (adım, oturdu mu, ağırlığı düşen baz sayısı); istatistikler yeniden ağırlıklı modelindir. \
                      Retler: invalid_input, no_baselines, duplicate_station, unknown_station, invalid_covariance, datum_missing, network_too_large, network_disconnected."
                .into(),
            aliases: vec!["gnssag".into(), "bazdengele".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::SurveyBaselines>(),
            output: schema::<crate::SurveyBaselinesAdjusted>(),
            plan: Some(schema::<crate::SurveyBaselinesAdjusted>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir sabit istasyon, iki yeni istasyon, üç baz (metinle)".into(),
                input: json!({ "text": "K;A;4208000;2334000;4171000\nN;B\nN;C\nB;A;B;100.012;-50.004;20.001;1e-6;0;0;1e-6;0;1e-6\nB;B;C;-30.003;80.002;-10.000;1e-6;0;0;1e-6;0;1e-6\nB;A;C;70.006;29.999;10.004;1e-6;0;0;1e-6;0;1e-6" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::MAP_FEATURES_CODE.into(),
            version: crate::MAP_FEATURES_CODE_VERSION,
            title: "Kodla çiz".into(),
            summary: "Sahada kodla alınmış noktaları veri profilinin sınıflarına göre işler. Noktanın kodu field alanındadır (yoksa Kod): ilk sözcük büyük harfli kod ve ardından dizinin numarası (BRD1), sonraki sözcükler denetim kodu: B yeni dizi başlatır, K diziyi kapatır, S bitirir. Kod profilin bir sınıfının codes'unda aranır. Nokta sınıfının noktaları sınıfın katmanına taşınır; çizgi sınıfının dizileri çoklu çizgi (K ile kapalı), alan sınıfının K ile kapatılmış dizileri alan olarak sınıfın katmanına çizilir, noktalar yerinde kalır; bütün noktaların kotu varsa köşeler kotlu. uids verilirse o noktalar o sırayla, verilmezse görünen ve kilitsiz katmanlardaki kodlu noktalar çizimin sırasıyla. name KentOS'un profili (tucbs-temel, harita-varliklari) ya da drawing'in yanındaki profil. Tek geri alma adımı “Kodla çiz”. Uyarılar: unknown_code (profilde olmayan kod, noktaları kalır), control_unknown, string_too_short, area_not_closed. \
                      Retler: invalid_revision, revision_conflict, unknown_profile, invalid_profile, invalid_input, invalid_uid, entity_not_found, not_a_point, no_coded_points, class_layer_missing (önce profili uygulayın), layer_locked."
                .into(),
            aliases: vec!["kodlaciz".into(), "fieldtofinish".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::FeaturesCode>(),
            output: schema::<crate::FeaturesCoded>(),
            plan: Some(schema::<crate::FeaturesCoded>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Harita varlıkları profiliyle bütün kodlu noktalar".into(),
                input: json!({ "name": "harita-varliklari" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_CRS_FACTORS.into(),
            version: crate::CAD_CRS_FACTORS_VERSION,
            title: "Ölçek çarpanları".into(),
            summary: "Projenin haritasında verilen noktalarda (doğu, kuzey; m) ölçülen bir uzunluğun zemin, elipsoit ve harita arasında geçtiği çarpanları okur; hiçbir şey yazmaz: projeksiyonun nokta ölçeği (scale: haritadaki uzunluk = elipsoitteki × scale), yükseklik çarpanı R/(R + h) doğu yönünde (heightFactor; R doğu yönündeki Euler yarıçapı, yani birinci düşey eğrilik yarıçapı N) ve ikisinin çarpımı birleşik çarpan (combined: zemindeki uzunluk × combined = haritadaki uzunluk). height verilmezse projenin ortalama elipsoit yüksekliği (Proje ayarları › Ölçme); ikisi de yoksa yalnız scale. Cevap sistemin adını, kullanılan yüksekliği ve ölçme pencerelerinin uzunlukları projeksiyona indirip indirmediğini (reducesToGrid) de verir. Çizim ve kamera ölçeği bu çarpanları değiştirmez. Sistemin ulaşmadığı nokta çarpansız kalır, uyarı point_beyond_system. \
                      Her noktanın kendi yüksekliği: heights noktaların elipsoit yükseklikleri; elevations çizimin kotları, projenin kotlarıyla elipsoit yüksekliğine çevrilir (elipsoit yüksekliğiyse olduğu gibi, ortometrikse projenin geoit modelinin noktadaki N'i eklenerek; cevapta height ve undulation). lines noktalar arasındaki çizgileri sıralarıyla [a, b] verir: her çizginin ölçeği (uçlarının ve ortasının ölçeğiyle Simpson), yükseklik çarpanı (jeodeziğin ortasında doğrultusundaki Euler yarıçapı, uçlarının yüksekliklerinin ortalamasında) ve birleşik çarpanı; Kutupsal alım ölçülen uzunluğu bu çarpanlarla indirir. Geoit ızgarasının dışındaki noktanın kotu çevrilmez, uyarı points_outside. \
                      Retler: invalid_input (nokta yok, 10 000'den çok, sonlu olmayan koordinat ya da yükseklik, yükseklik −500…9000 m dışında; height, heights ve elevations'tan birden çoğu; nokta başına bir değer değil; çizginin ucu nokta değil, iki ucu aynı yerde, 10 000'den çok çizgi; kotlar metre değil), no_system (proje yerel koordinatlarda), no_point_scale (coğrafi sistem, Pseudo-Mercator ya da afinle bağlı yerel sistem), heights_unknown (elevations: projenin kotlarının ne olduğu söylenmemiş), geoid_required (ortometrik kot, geoit modeli yok), geoid_missing (geoit modelinin ızgarası bu cihazda yok)."
                .into(),
            aliases: vec!["olcekcarpani".into(), "scalefactor".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::CrsFactors>(),
            output: schema::<crate::CrsFactorsRead>(),
            plan: Some(schema::<crate::CrsFactorsRead>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "TUREF / TM30'da iki nokta, 850 m yükseklikle".into(),
                input: json!({ "points": [[550000.0, 4540000.0], [500000.0, 4540000.0]], "height": 850.0 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::MAP_WORK_QUERY.into(),
            version: crate::MAP_WORK_QUERY_VERSION,
            title: "Harita işini sorgula".into(),
            summary: "Harita projesinin iş yapısını bugün okur; hiçbir şey yazmaz: öğeler (arazi ölçümü, altlık, çalışma verisi, tasarım, denetim, teslim) projedeki sırasıyla, her birinin türü, adı, durumu (draft, review, approved: ofisin kaydı, kurum onayı değil), katmanları, katmanlarındaki nesne sayısı, dosyası (yolu, kabul edilen SHA-256 revizyonu, found, current, changed, missing) ve durumu (current, missing, changed, incomplete, stale) nedenleriyle; türetme kayıtları (kaynak öğe, üretim işlemi, kaynağın o zamanki revizyonu, nesne sayısı) ve durumları (current, stale: kaynağı değişti, missing: nesnesi silindi). Bir nesne kendi katmanının ya da üstündeki en yakın grubun öğesinindir (katman profili). uid verilince o nesnenin kaynağı: source (arazi ölçümü ya da altlık katmanında), derived (türetme kaydı, kaynağı, revizyonu, işlemi, güncelliği) ya da free (kullanıcının çizimi). deep her dosyayı okuyup SHA-256'sını karşılaştırır; içeriği değişen dosyanın türevleri güncel değildir. drawing (açık projenin dosyası, mutlak yol) verilince yolunda olmayan dosya proje klasöründen göreli yoluyla, sonra projenin arama yollarında aranır; dosyanın via'sı (path, relative, searchPath, none), at'i (bulunduğu yer) ve relative'i (kayıtlı göreli yol). \
                      Onaydan sonra değişen öğenin changes'i: eklenen, silinen, değişen nesneler; onay parçaları tuttuysa her değişen nesnenin değişen parçaları; değerleri de tuttuysa değişen özniteliklerin adları, onaylanan ve şimdiki değerleri, geometrisi değişen nesnenin onaylanan hâli (objects[].approved) ve ne kadar kaydığı (objects[].shift: aynı türde ve aynı köşe sayısındaysa köşe köşe en büyük kayma, köşe numarası ve KOH; her durumda iki şeklin Hausdorff uzaklığı ve yeri), silinen nesnelerin onaylanan hâli (removedObjects). \
                      Profili olan öğenin profile'ı: profil bulundu mu, başlığı, sürümü, öğenin sınıfları (katmanı, nesne ve bulgu sayısı), bulguları (cad.profiles.check'inkiler) ve sınıfı olmayan, nesnesi olan serbest katmanları. \
                      limit: kayma sınırı; basis own (metres), detail (BÖHHBÜY m. 46(1), ±7 cm), drawing (m. 78(1)(h), ±0,2 mm × scale) ya da method (Tapu Planlarında Yanılma Sınırı Yönetmeliği m. 9(1) tablosu: method ve scale; tabloda olmayan ölçek ve boş hücre reddedilir). Değerler kural dosyasındadır (konum-dogrulugu-2022). Her kaymanın beyond'u (köşeleri eşleniyorsa en büyük köşe kayması, eşlenmiyorsa Hausdorff sınırdan büyük mü) ve over'ı (sınırı aşan köşe sayısı), changes.beyond (sınırı aşan nesne sayısı), ağacın limit'i (değeri ve kaynağı); sınır içinde olmak teknik hata olmadığı yargısı değildir. \
                      suggest: profili olmayan öğeye türüne uyan profiller (suggested: adı, başlığı, sınıfları; sınıfın work alanı öğe türlerini söyler): KentOS'unkiler, çizimin içindekiler ve proje klasörününkiler (kurumun profili). \
                      Retler: not_a_map_project, invalid_file_path, invalid_input (limit), invalid_uid, entity_not_found."
                .into(),
            aliases: vec!["haritaisi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::MapWorkQuery>(),
            output: schema::<crate::MapWorkTree>(),
            plan: Some(schema::<crate::MapWorkTree>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dosyaları okuyarak, bir noktanın kaynağıyla".into(),
                    input: json!({ "deep": true, "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11" }),
                    output: None,
                },
                CommandExample {
                    title: "Kaymalar sayısal üretimin 1/1000 nokta konum doğruluğuyla; önerilen profillerle".into(),
                    input: json!({ "limit": { "basis": "method", "method": "sayisal", "scale": 1000 }, "suggest": true }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::MAP_WORK_EDIT.into(),
            version: crate::MAP_WORK_EDIT_VERSION,
            title: "Harita işini düzenle".into(),
            summary: "Harita projesinin iş yapısını değiştirir; her işlem tek geri alma adımı, ya hep ya hiç. add: yeni öğe is-N (türü, adı, katmanları: her katman tek öğenin; file mutlak yol ise SHA-256'sı revizyonudur). set: adı, durumu (draft, review, approved) ya da katmanları; katmanlarında kaynağı değişmiş türetilmiş nesne olan öğe onaylanmaz (approval_stale); onay ofisin teslim revizyonu kaydıdır, kurum onayı değildir ve hiçbir denetim onu kendiliğinden koymaz. remove: öğe ve ondan türetme kayıtları gider, nesneler kalır. relink: kaynağın dosyası artık file; öğenin yolu ve katmanlarında eski dosyayı gösteren raster ve resimlerin yolu aynı adımda değişir; içerik farklıysa yeni revizyon yazılır, türevleri güncel değildir (uyarı source_content_changed). refresh: dosyanın bugünkü içeriği revizyon olur. derive: nesneler dosyası olan bir öğeden operation ile, onun bugünkü revizyonundan türetildi diye kaydedilir; bir nesne tek kayıtta olur. forget: türetme kaydı gider. release: verilen nesneler kendi türetme kayıtlarından çıkar (bağımsız çizim olur), nesnesi kalmayan kayıt gider (freed, released). relink, kaynağı eski dosya olan dış başvuruları da aynı adımda yeni dosyaya geçirir, son yükledikleri içerik yeni dosyanınkiyse (links); başka içerik yükleyen dış başvuruya ve eski dosyadaki bağlı katmana dokunmaz (bağ projenin ayarıdır, geri alınmaz), uyarı link_not_followed. profile: öğenin tipli katman profili (name: veri profilinin adı, classes: sınıfları; yoksa hepsi); eksik sınıf katmanları profilin yerine eklenir, katmanların eksik alanları eklenir, sınıfların katmanları öğeye girer, öbür katmanlar serbest kalır; name verilmezse profil kalkar. Profili olan öğe tipli katmanlarında bulgu varken onaylanmaz (approval_profile). drawing: açık projenin dosyası, profil klasörü için. \
                      Retler: invalid_revision, revision_conflict, not_a_map_project, unknown_work_item, unknown_derivation, invalid_work_text, layer_not_found, layer_in_work_item, nothing_to_set, invalid_file_path, file_not_found, work_item_no_file, no_entities, invalid_uid, entity_not_found, already_derived, not_derived, approval_stale, unknown_profile, invalid_profile, unknown_profile_class, approval_profile, layer_refused, layer_locked, work_full."
                .into(),
            aliases: vec!["haritaisiduzenle".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::MapWorkEdit>(),
            output: schema::<crate::MapWorkEdited>(),
            plan: Some(schema::<crate::MapWorkEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "GNSS arazi ölçümü, dosyasıyla".into(),
                    input: json!({ "operation": { "op": "add", "kind": "campaign", "name": "GNSS 2026-10", "layers": ["gnss"], "file": "/veri/gnss-2026-10.nmea" } }),
                    output: None,
                },
                CommandExample {
                    title: "Taşınan ortofotoyu yeniden bağla".into(),
                    input: json!({ "operation": { "op": "relink", "item": "is-2", "file": "/arsiv/ortofoto.tif" } }),
                    output: None,
                },
            ],
        },
        // Onaylı hâline getir (plan/map-cad.md MAP-01, docs/adr/4080).
        CommandDescriptor {
            id: crate::MAP_WORK_RESTORE.into(),
            version: crate::MAP_WORK_RESTORE_VERSION,
            title: "Onaylı hâline getir".into(),
            summary: "Onaylı bir harita işi öğesinin (item) nesnelerini onayın tuttuğu değerlerle (şema 213) geri getirir; tek geri alma adımı “Onaylı hâline getir”, hepsi ya da hiçbiri. Onaydan sonra silinen nesne aynı kalıcı kimlikle yeniden oluşturulur (restored); değişen ya da öğenin katmanlarından çıkan nesne onaylanan değerine döner (reverted). uids verilirse yalnız o onaylı nesneler (her biri silinmiş ya da değişmiş olmalı); verilmezse hepsi. Onaydan sonra eklenen nesneler yerinde kalır (added sayısı); asApproved öğenin içeriğinin onaylanan içerikle aynı olup olmadığını söyler. Geri gelen nesnenin katmanı çizimde, katman, kilitsiz ve türüne uygun olmalı; değişen nesnenin şimdiki katmanı kilitli olmamalı. \
                      Uyarılar: layer_hidden. Retler: invalid_revision, revision_conflict, not_a_map_project, unknown_work_item, work_not_approved, no_approved_values (dosyalı öğe, değerleri tutmayan onay), invalid_uid, not_approved_object, nothing_to_restore, layer_not_found, not_a_layer, layer_locked, layer_geometry_mismatch."
                .into(),
            aliases: vec!["onaylihalinegetir".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::MapWorkRestore>(),
            output: schema::<crate::MapWorkRestored>(),
            plan: Some(schema::<crate::MapWorkRestored>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Tasarım yüzeyinin silinen ve değişen nesneleri".into(),
                input: json!({ "item": "is-4" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::MAP_SURVEY_QUERY.into(),
            version: crate::MAP_SURVEY_QUERY_VERSION,
            title: "GNSS arşivini sorgula".into(),
            summary: "Harita projesinin GNSS ölçüm arşivini bugün okur; hiçbir şey yazmaz: arazi ölçümleri (epok kararı, hız ızgarası, bağlı harita işi öğesi ve o öğenin hâlâ olup olmadığı), oturumlar (işaret, zamanlar, zaman sistemi, alıcı, anten ve düşey ARP yüksekliği, dosyası found/missing ya da deep ile current/changed, ham ölçüm sayısı ve oynatma özeti: fix sayıları, RTK sabitten düşüş, zaman atlaması, olağan adım), işaretler (oturumları, çözümleri: son ölçülen, kabul edilen, dengelemede, dışlandı ve nedeni, işaret yüksekliği = h + uygulanmış anten − oturumun düşey anteni), kabul edilen noktalar (current, stale: oturumun anteni değişti, epok kararı değişti, çözüm dışlandı; missing: nesnesi yok; sonuç ve eski sonuç sayısı) ve dayananlar. HDOP birimsizdir, metre değildir; RTK sabit doğruluk garantisi değildir. uid verilince o nesnenin ölçüsü. places ile her bilinen noktanın projedeki \
                      kendi yeri (epoku önce kendi hızıyla ya da hız ızgarasıyla, sonra koordinat zinciriyle; işaretinin kabul noktasından \
                      farkı ve yatay uzaklığı) ya da neden yeri olmadığı; tolerance ile kullanıcının yatay ve düşey sınırını \
                      aşan yer beyond; girdide sınır yoksa projede saklı sınır (tolerance, kaynağıyla) kullanılır. Ağaç baz oturumlarını (baselineSessions) ve bazın oturumunu da verir; oturumdaki dışlanmamış bazın independent'i oturumun n − 1 bağımsız bazından mı (kısadan uzuna yayılan ağaç) yoksa bağımlı mı olduğunu söyler. Retler: invalid_input (sınır places'siz, boş ya da 0'dan büyük değil), not_a_map_project, invalid_uid, \
                      entity_not_found."
                .into(),
            aliases: vec!["gnssarsivi".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::MapSurveyQuery>(),
            output: schema::<crate::MapSurveyTree>(),
            plan: Some(schema::<crate::MapSurveyTree>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Dosyaları okuyarak, kabul edilen bir noktanın ölçüsüyle".into(),
                input: json!({ "deep": true, "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::MAP_SURVEY_EDIT.into(),
            version: crate::MAP_SURVEY_EDIT_VERSION,
            title: "GNSS arşivini düzenle".into(),
            summary: "Harita projesinin GNSS ölçüm arşivini değiştirir; her işlem tek geri alma adımı, ya hep ya hiç. campaign / setCampaign: arazi ölçümü kp-N (adı, harita işinin arazi ölçümü öğesi is-N, operatör, epok kararı, hız ızgarası). session: oturum os-N; file RINEX gözlem dosyasıysa başlığından işaret, alıcı, anten (DELTA H ARP'ye düşey), zamanlar ve zaman sistemi; NMEA ya da GPX ise ham ölçümler ve ilk/son zaman (UTC); verilen alan dosyanınkini yener (uyarı rinex_differs), okunmayan satırlar uyarıdır (gnss_lines_skipped). setSession: işaret, zamanlar, alıcı, anten (arp ya da slant: √(S²−R²)−offset), operatör; anten değişince o anteni kullanan kabul noktası güncel değildir. solve: alıcı çözümü cz-N, from..to aralığında ve fixes içindeki ham ölçümlerin soldan sağa ortalaması; çerçeve (srid, kayıttaki coğrafi sistem) ve uygulanmış anten yüksekliği kullanıcının söylediğidir. solution: işlenmiş baz/PPK, PPP ya da elle çözüm (σ, tam kovaryans KK KD KY DD DY YY pozitif tanımlı, düzeltme kaynağı ve yaşı, rapor dosyası, yeniden işleme tarifi). exclude (nedenle, kayıt silinmez), include, adjust (dengelemeye girer). accept: işaretin noktası; single, mean ya da weighted (1/σ²); işaret yüksekliği h + uygulanmış − düşey anten; projeye koordinat zinciriyle (cad.crs.transform: çözümün çerçevesi ve epoku → projenin sistemi, arazi ölçümünün epok kararı, hız ızgarası) taşınır; nokta nesnesi ve kn-N aynı adımda. regenerate: bugünkü anten ve epok kararıyla (istenirse başka çözüm ya da yöntemle) yeniden; eski sonuç geçmişte kalır, nesne yeni yerine gider, revizyon artar, dayananlar güncel değildir. use: nesneler kabul noktalarının bugünkü revizyonlarından yapıldı diye kaydedilir. remove: kayıt çıkar; kabul noktasının ya da dayananın dayandığı kayıt çıkmaz, ağ dengelemesindeki baz ve bilinen nokta da; baz oturumu bo-N çıkar (bazları kalır), dengelemedeki oturum ve bir oturumdaki baz çıkmaz. baselines: bir dosyanın baz vektörleri bz-N (GVX 1.0: işaretten işarete ECEF ΔX ΔY ΔZ, SDX/SDY/SDZ ve PXY/PXZ/PYZ'den kovaryans, GPS zamanı, çözüm türü, yazılım; oturumların çapraz korelasyonları baz oturumu bo-N olarak saklanır (bazları, satır satır dokuzar korelasyonlu blokları; blokları eksik oturum saklanır, uyarı gvx_session_incomplete; iki vektör anmayan oturum alınmaz, uyarı gvx_correlations_ignored); ya da B; satırlı baz metni, K/N satırları alınmaz, uyarı baseline_text_stations). baseline: elle tek baz. known: bilinen nokta bn-N (işaret, enlem, boylam, elipsoit yüksekliği, coğrafi çerçeve, epok; σ kuzey/doğu/yukarı verilirse ağırlıklı, yoksa sabit; kaynak); bir işaretin dışlanmamış tek bilinen noktası olur. exclude/include/adjust bz- ve bn- kimliklerini de alır. network: map.survey.network'ün ağı; serbest ya da dayalı dengelemede uyuşumsuz ölçü varsa network_blunder, model testi geçmezse network_test_failed ile reddedilir; yoksa da-N dengeleme kaydı (bazlar, bilinen noktalar, tutulan, çarpan, sınıf, m0 ve serbestlikler, çerçeve, epok) ve bilinen nokta olmayan her işaretin noktası (yeni nesnesi layer'a) ya da yeniden üretimi (eski sonuç geçmişte, nesne yeni yerine), method network, tek adım; sınıf tutmazsa uyarı network_rules_failed; ellipses verilirse (layer, scale 1–100 000) her noktanın hata elipsi aynı adımda büyütülerek çizilir, büyük ekseni semtinden meridyen yakınsaması çıkarılmış yönde, kb-N dayanan kaydıyla; heights ile kot çubuğu, labels ile yarı eksen etiketi: etiket önce doğuda, oraya elips, çubuk ya da önceki etiket düşerse sağ üst, sol üst, sağ alt, sol alt, batı, üst, alt sırasıyla ilk boş yerde; hiçbiri boş değilse doğuda kalır (labelsMoved, labelsCrowded). Bazı dışlanan ya da bilinen noktası dışlanan dengelemenin noktası güncel değildir. network'ün correlations'ı ve velocities'i map.survey.network'ünkiler gibidir: dengeleme kaydı kullandığı baz oturumlarını (sessions), noktanın sonucu hızını (velocity: doğu, kuzey, yukarı m/yıl, ızgaranın adı ve SHA-256'sı) tutar. tolerance: bilinen noktaların kabul farkı sınırı (horizontal, vertical metre; source: yönetmelik maddesi ya da idarenin şartnamesi, zorunlu) projeye yazılır, map.survey.query'nin places'i girdide sınır yoksa onunla değerlendirir; üçü de verilmezse saklı sınır kalkar. independent: bir baz oturumunun (session verilmezse bütün oturumların) bağımsız bazları dengelemeye girer, bağımlıları girmez (n alıcılı oturumda n − 1 bağımsız baz: dışlanmamış bazlar kısadan uzuna, işaretleri henüz bağlanmamış ikisini bağlayan bağımsızdır; dışlananlar dışarıda kalır); tek geri alma adımı. \
                      Retler: invalid_revision, revision_conflict, not_a_map_project, unknown_survey_record, unknown_work_item, invalid_survey_text, invalid_survey_value, invalid_file_path, file_not_found, unreadable_gnss_file, nothing_to_set, survey_in_use, unknown_frame, no_observations, no_ellipsoidal_height, solution_excluded, mark_mismatch, method_count, sigma_missing, solutions_differ, antenna_unknown, antenna_eccentric, already_accepted, layer_not_found, layer_locked, no_entities, invalid_uid, entity_not_found, survey_full, ağın retleri (no_baselines, no_velocity_grid, datum_missing, known_differ, unknown_rule_class, duplicate_station, invalid_covariance, session_blocks_missing, session_covariance_invalid, network_too_large, network_disconnected, network_blunder, network_test_failed) ve koordinat zincirinin kendi retleri (epoch_unsupported, velocity_missing, geoid_missing, no_system)."
                .into(),
            aliases: vec!["gnssarsividuzenle".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::MapSurveyEdit>(),
            output: schema::<crate::MapSurveyEdited>(),
            plan: Some(schema::<crate::MapSurveyEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "RINEX dosyasından oturum".into(),
                    input: json!({ "operation": { "op": "session", "campaign": "kp-1", "file": "/veri/N1_3070.26o" } }),
                    output: None,
                },
                CommandExample {
                    title: "İki günün çözümünün ortalamasını kabul et".into(),
                    input: json!({ "operation": { "op": "accept", "mark": "N1", "solutions": ["cz-1", "cz-2"], "method": "mean", "layer": "gnss" } }),
                    output: None,
                },
                CommandExample {
                    title: "Kötü çözümü silmeden dışla".into(),
                    input: json!({ "operation": { "op": "exclude", "solution": "cz-3", "reason": "Ağaç altında, float'a düştü" } }),
                    output: None,
                },
                CommandExample {
                    title: "GVX dosyasının bazlarını arşive al".into(),
                    input: json!({ "operation": { "op": "baselines", "campaign": "kp-1", "file": "/veri/ag-2026-11.gvx" } }),
                    output: None,
                },
                CommandExample {
                    title: "Ağı BÖHHBÜY C2 sınıfıyla dengeleyip kabul et".into(),
                    input: json!({ "operation": { "op": "network", "class": "C2", "layer": "gnss" } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::MAP_FEATURES_RECONCILE.into(),
            version: crate::MAP_FEATURES_RECONCILE_VERSION,
            title: "Detay eşleştirme".into(),
            summary: "Birkaç kaynaktan (nokta bulutu ve ortofoto çıkarımı, saha ölçümü) gelen harita varlığı adaylarını (detayları) bir veri profilinin sınıflarıyla eşleştirir. Aday, bir kaynağın katmanında sınıf alanı (field, yoksa Kod) profilin bir sınıfını kimliğiyle ya da koduyla adlandıran nokta ya da açık çoklu çizgidir; confidence sayısal güven alanıdır (büyük iyidir). radius (m) içindeki adaylar bir gruptur (zincirle): noktada düzlem uzaklığı, çoklu çizgide iki yönlü ayrık Hausdorff uzaklığının büyüğü. Grubun kaynak temsilcisi güveni en yüksek adayıdır (güvensiz en alt; eşitlikte ilki), kaynağın öteki adayları duplicate. Grup existing (sınıfının kabul edilmiş varlığı yarıçap içinde: yeni varlık yazılmaz), conflict (sınıflar farklı), agreed (iki ya da daha çok kaynak bir sınıfta) ya da single (tek kaynak). Execute agreed grupları ve kabul kararı verilmiş tekleri, zorunlu alanları tamamsa, sınıfın katmanına yazar: nokta temsilcilerin ortalamasında, çizgi güveni en yüksek temsilcinin geometrisiyle; alanlar temsilcilerden güven sırasıyla, kararların alanları üstte; Kaynaklar, Adaylar ve Yayılım öznitelikleri. Adaylar Eşleştirme özniteliğiyle (varlığın kimliği ya da Reddedildi: neden) işaretlenir ve bir daha okunmaz (eski çizimlerin Uzlaştırma özniteliği de okunur): insan kararları sonraki çıkarımla ezilmez. decisions: class (sınıf düzeltme), accept, reject (nedenle), fields (eksik alanlar). \
                      Tutan her karar nesneden ayrı, çözülmüş bir inceleme notudur (cad.review.notes.list okur): yazarı reviewer, zamanı reviewedAt, adaya ve varlığa damgalarıyla bağlı, \
                      decision'ı verilen karar; notes numaralarını söyler. Tek geri alma adımı; plan yazmadan söyler. \
                      Retler: invalid_revision, revision_conflict, unknown_profile, invalid_profile, invalid_input, layer_not_found, invalid_uid, entity_not_found, class_layer_missing, layer_locked, too_many_review_notes."
                .into(),
            aliases: vec!["eslestir".into(), "detayeslestir".into(), "uzlastir".into(), "adayuzlastir".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::FeaturesReconcile>(),
            output: schema::<crate::FeaturesReconciled>(),
            plan: Some(schema::<crate::FeaturesReconciled>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "LiDAR ve ortofoto adayları, 0,5 m".into(),
                input: json!({ "name": "harita-varliklari", "sources": [{ "layer": "aday-lidar", "name": "LiDAR" }, { "layer": "aday-orto", "name": "Ortofoto" }], "confidence": "Güven", "radius": 0.5 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::MAP_SURVEY_NETWORK.into(),
            version: crate::MAP_SURVEY_NETWORK_VERSION,
            title: "GNSS ağını dengele".into(),
            summary: "Harita projesinin GNSS arşivindeki bazlardan (dengelemeye giren, dışlanmamış; campaign verilirse yalnız o arazi ölçümünün) ve uçlarındaki bilinen noktalardan (dışlanmamış; aynı coğrafi çerçeve ve epokta, ölçme epokunda) kurulan ağı iki kez dengeler; hiçbir şey yazmaz. Serbest: tek bilinen nokta (hold, verilmezse ilki) sabit; uyuşumsuz bazlar ve ağın kendi duyarlığı (BÖHHBÜY Madde 15(1)(b)). Dayalı: bilinen noktalar sabit, σ'sı verilmişse kuzey/doğu/yukarı σ'sıyla ağırlıklı (ECEF'e döndürülmüş tam kovaryans); işaretlerin koordinatları (15(1)(ç)). Her aşamada m0, serbestlik, vᵀPv, χ² (%95) ve model testi, uyuşumsuz ölçü sayısı ve en kötüsü; her baz için uzunluk, işlenmiş σX σY σZ (varyans çarpanıyla), serbest dengelemede dengelenmiş vektörün σ'sı, iki aşamada başlangıcının doğu/kuzey/yukarısında artık, w ve durum. Bilinen nokta olmayan işaretler koordinat zinciriyle projeye taşınır, kabul edilen noktasından farkı (dx, dy, dz) verilir. class (KentOS'un paketinde BÖHHBÜY 2018 C1, C2, C3; rules ile başka paket) baz σ'sını a + b·ppm·uzunluk sınırıyla (C1 serbest dengelemenin, C2/C3 işlenmiş σ), noktanın σ kuzey/doğu/yukarısını sınırlarla ve testleri sınıfın güveniyle (C1: 1−α = 0,95, w > 1,96; başkasında Baarda α₀ = 0,001, w > 3,29) denetler. acceptable ve why: kabul edilip edilemeyeceği. Kurum onayı değildir. \
                      correlations: session ile her baz oturumunun (bo-N) dengelemeye giren iki ya da daha çok bazı tam kovaryansıyla (çapraz bloklar ρ·σ·σ) birlikte girer, bazlar korelasyonlu ölçü gibi bileşen bileşen sınanır (w = |Pv|ᵢ/√(PQvvP)ᵢᵢ); none ya da verilmezse her baz kendi 3×3'üyle; cevabın correlations'ı hangisinin kullanıldığını, giren ve kullanılmayan oturumları söyler (uyarı session_correlations_unused). velocities ile bilinen nokta olmayan her işaretin hızı (doğu, kuzey, yukarı m/yıl) arazi ölçümünün hız ızgarasından, koordinat zinciriyle bir yıllık taşınmadan okunur; velocityGrid ızgaranın adı ve SHA-256'sı; ızgarasız ret no_velocity_grid, ızgaranın dışındaki işaret why'da. \
                      Retler: not_a_map_project, unknown_survey_record, invalid_survey_value, unknown_rule_class, no_baselines, no_velocity_grid, solutions_differ, datum_missing, known_differ, unknown_frame, duplicate_station, invalid_covariance, session_blocks_missing, session_covariance_invalid, network_too_large, network_disconnected."
                .into(),
            aliases: vec!["gnssagi".into(), "arsivagdengele".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::MapSurveyNetwork>(),
            output: schema::<crate::MapSurveyNetworked>(),
            plan: Some(schema::<crate::MapSurveyNetworked>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "BÖHHBÜY C2 sınıfıyla, varyans çarpanı 2".into(),
                input: json!({ "class": "C2", "scale": 2.0 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_EXPORT_LANDXML.into(),
            version: crate::FILE_EXPORT_LANDXML_VERSION,
            title: "LandXML dışa aktar".into(),
            summary: "Yol projesinin arazi yüzeylerini (TIN: noktalar ve üçgenler), akslarını (doğru, daire eğrisi ve klotoid; km denklemleri) ve akslarının boykesitlerini (PVI'lar; simetrik ve asimetrik parabol, daire eğrisi) LandXML 1.2 dosyasına yazar; yol projelendirme yazılımları okur. Yer kuzey doğu sırasıyla, uzunluk metre, doğrultu ve açı ondalık derece (doğrultu kuzeyden saat yönünde); elemanların ve PVI'ların km'leri iç km'dir (başlangıç km'si + aks boyunca uzaklık). surfaces ve alignments id ya da adla (yoksa hepsi, boş liste hiçbiri); withoutProfiles boykesitleri yazmaz; stamp dosyanın tarihi ve saati (yoksa şimdi, UTC). Aksın proje hızı ve kurplarının kendi hızları Roadways/Roadway/Speeds/DesignSpeed olarak km aralıklarıyla yazılır (kurbun hızı TS–ST, aksınki kalanı; km/sa, velocityUnit kilometersPerHour). Paralel aks LandXML'de olmadığından yazılmaz (export_losses). Bütün yazılır ya da hiç: yanına yazılır, okuyucuyla geri okunur, sonra yerine konur; plan yazmadan söyler. Çizimi değiştirmez ve geri alma adımı değildir. \
                      Dosya projenin sistemlerini CoordinateSystem'de söyler (EPSG adı ve kodu, düşey sistemin adı ve datumu). machineControl: true makine kontrolü içindir: projenin kotlarının düşey sistemi söylenmiş olmalı (field_heights_unknown), her yüzeyin buluttan alınan zemini o sistemde gösterilmiş olmalı (unproven_heights). Retler: not_a_road_project, invalid_file_path, file_exists, unknown_terrain, unknown_alignment, invalid_landxml, field_heights_unknown, unproven_heights."
                .into(),
            aliases: vec!["landxmlver".into(), "landxmlout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportLandxml>(),
            output: schema::<crate::LandxmlExported>(),
            plan: Some(schema::<crate::LandxmlExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ana aks boykesitleriyle ve zemin yüzeyi".into(),
                input: json!({ "path": "/veri/yol.xml", "surfaces": ["Mevcut zemin"], "alignments": ["Ana aks"], "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_EXPORT_GLTF.into(),
            version: crate::FILE_EXPORT_GLTF_VERSION,
            title: "glTF dışa aktar".into(),
            summary: "Yol projesinin arazi yüzeylerini ve koridorlarının üst yüzeylerini 3B sahne olarak glTF 2.0 ikili (GLB) dosyasına yazar: her biri bir ağ (üçgenler, normalsiz: okuyucu düz normal kurar), malzemesi türüne göre. Konumlar float32, sahnenin kökenine göre; köken (sınırların ortası, tam metreye) kök düğümün translation'ında tam (f64) yazılır, böylece milyonlarca metrelik koordinat milimetresini korur; error her köşenin float32 konumu ile kaynağı arasındaki en büyük eksen farkıdır. glTF'in ekseni +Y yukarı: (doğu, yukarı, −kuzey); metre. surfaces ve corridors id ya da adla (yoksa hepsi, boş liste hiçbiri); adla seçilen kurulamazsa ret, seçilmeyen kurulamazsa export_losses. Bütün yazılır ya da hiç; çizimi değiştirmez. \
                      Retler: not_a_road_project, invalid_file_path, file_exists, unknown_terrain, unknown_corridor, invalid_gltf."
                .into(),
            aliases: vec!["gltfver".into(), "glbout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportGltf>(),
            output: schema::<crate::GltfExported>(),
            plan: Some(schema::<crate::GltfExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Zemin ve ana koridor".into(),
                input: json!({ "path": "/veri/yol.glb", "surfaces": ["Mevcut zemin"], "corridors": ["Ana koridor"], "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::FILE_EXPORT_IFC.into(),
            version: crate::FILE_EXPORT_IFC_VERSION,
            title: "IFC dışa aktar".into(),
            summary: "Yol projesinin akslarını ve boykesitlerini IFC 4.3 (IFC4X3_ADD2) dosyasına yazar: iş mantığı (IfcAlignmentHorizontal ve IfcAlignmentVertical, IfcAlignmentSegment'leri ve tasarım değerleri: başlangıç noktası ve doğrultusu, yarıçaplar (saat yönü tersine artı, 0 sonsuz), boy; düşeyde başlangıç uzaklığı, yatay boy, kot, eğimler, yarıçap) ve geometri (IfcCompositeCurve ve IfcGradientCurve: IfcCurveSegment'ler IfcLine, IfcCircle, IfcClothoid, IfcPolynomialCurve üstünde; IfcOpenShell'in eşlemesi); her düzen sıfır boylu parçayla biter. İlk boykesit aksın, öbürleri alt aksların düşeyidir. Başlangıç km'si ve km denklemleri Pset_Stationing'li IfcReferent'tir; projenin sistemi IfcMapConversion ile IfcProjectedCRS'e. Uzunluk metre, açı radyan. alignments id ya da adla (yoksa hepsi); withoutProfiles boykesitleri yazmaz; stamp tarih ve saat. Paralel aks yazılmaz (export_losses). Bütün yazılır ya da hiç; çizimi değiştirmez. \
                      Retler: not_a_road_project, invalid_file_path, file_exists, unknown_alignment, invalid_ifc."
                .into(),
            aliases: vec!["ifcver".into(), "ifcout".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportIfc>(),
            output: schema::<crate::IfcExported>(),
            plan: Some(schema::<crate::IfcExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ana aks boykesitleriyle".into(),
                input: json!({ "path": "/veri/yol.ifc", "alignments": ["Ana aks"], "overwrite": true }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_ARCH_IFC.into(),
            version: crate::CAD_ARCH_IFC_VERSION,
            title: "Binayı IFC'ye aktar".into(),
            summary: "Çizimin binasını IFC 4.3 (IFC4X3_ADD2) dosyasına yazar (Dijital Proje Yönetmeliği EK-5 §5.4): proje, saha, bina ve her kat bir IfcBuildingStorey; duvarlar, döşemeler, kolonlar, kirişler (lento), çatılar, korkuluklar ve merdivenler kesit modelinin gövdeleriyle (duvar katman katman, kolonun yüzünde ve kirişin altında biter); her açıklık duvarını oyan bir IfcOpeningElement ve onu dolduran IfcDoor ya da IfcWindow (etiketi çizimdeki numarası); mahaller IfcSpace. Adlar EK-5 §5.5'in Disiplin-KategoriKodu-Açıklama kalıbında (MM-DVR-Dış_duvar_25, MM-KAP-K01); EK-6'dan çizimin bildikleri: IsExternal, LoadBearing, malzeme, mahal kaplamaları, net alanlar, emsal ve TAKS. GlobalId nesnenin KentOS kimliğidir. Uzunluk metre. stamp tarih ve saat. Gövdesi olmayan nesne yazılmaz ve söylenir (export_losses). Bütün yazılır ya da hiç; çizimi değiştirmez. \
                      Retler: invalid_file_path, file_exists, invalid_ifc."
                .into(),
            aliases: vec!["ifcbina".into(), "ifcbuilding".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::ExportIfcBuilding>(),
            output: schema::<crate::IfcBuildingExported>(),
            plan: Some(schema::<crate::IfcBuildingExported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Binanın modeli".into(),
                input: json!({ "path": "/veri/bina.ifc", "overwrite": true }),
                output: None,
            }],
        },
    ]
}
