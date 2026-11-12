//! The catalog's commands of architecture's elements (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
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
                      arch duvarın katını (projenin katlarından), tadilat durumunu, yapım durumunu ve disiplinini taşır. \
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
        // Planı tanı (plan/architecture-advanced.md M48, docs/adr/8048), held together by
        // fixtures/commands/v1/cad.arch.recognize.json.
        CommandDescriptor {
            id: crate::CAD_ARCH_RECOGNIZE.into(),
            version: crate::CAD_ARCH_RECOGNIZE_VERSION,
            title: "Planı tanı".into(),
            summary: "Patlatılmış bir planı (ofisin DWG/DXF'i: duvar, kapı ve pencere düz çizgi ve yay olarak gelmiş) KentOS duvarlarına, kapılarına, pencerelerine ve boşluklarına çevirir; tek geri alma adımı “Planı tanı”. \
                      Her katmanın rolü roles ile verilir (wall, door, window, column, ignore); verilmeyen katmanın rolü adından çıkar (duvar, perde, bölme, wall → duvar; kapı, door → kapı; pencere, window, glaz → pencere; kolon, column → kolon; tarama, ölçü, yazı, aks, mobilya ve diğerleri yok sayılır); hiçbir katman duvar değilse adı geçmeyen her katman duvar sayılır. \
                      Bir doğru üzerindeki çizgiler (angle derece içinde, 5 mm içinde) parça parça ya da üst üste çizilmiş olsa da tek çizgidir. Paralel çizgiler en çok maxWidth (verilmezse 0,6 m), en az minWidth (verilmezse 0,05 m) aralıklı bir bant olur; 0,1 m'den dar aralıklar bileşen çizgisidir (yalıtım, sıva), duvarın çekirdeği tek geniş aralıktır. \
                      Bandın iki parçası arasındaki boşluk sırayla: bir yüz sürüyorsa birleşim; bir pervazın çevresinde boşluk kadar (iki kanatta yarısı kadar) yay kapı (menteşe tarafı ve açıldığı yan yaydan); boşluk boyunca pencere çizgileri pencere; kolon ya da kesen/saplanan başka duvar birleşim; içi boş ve maxOpening'den (verilmezse 3 m) dar boşluk Boşluk (0,3 m'den darı çizim hatası, duvar sürer); daha genişi duvarı bitirir. \
                      Duvar ucu başka duvarın eksenine yarı kalınlıkları (ya da durduğu kolon) kadar yakınsa ona uzar: T'de eksene, köşede ikisi de kesişime; birleşimleri duvarlar çözer. Duvar düz, ortası eksende, ölçülen kalınlıkta; arch verilirse (kat, tadilat durumu) her duvar onu alır. Kapı türü genişliğine göre Kapı 80 ya da 90, iki kanatlıysa İki kanatlı kapı 140; pencere Pencere 120; boşluk Boşluk 100; her biri okunduğu genişlikte. \
                      Duvarlar wallLayerId'ye, yoksa ilk çizgisinin katmanına; kapı ve pencereler openingLayerId'ye, yoksa rolündeki ilk katmana, boşluk duvarının katmanına yazılır. Kolon katmanının dört dik köşeli çevresi dikdörtgen, dairesi yuvarlak kolon olur (dolgulu; içindeki tarama onunla gider). Bloklar çizim değişmeden yerinde açılır: parça kendi katmanının, 0 katmanındaki parça yerleştirmenin rolüyle okunur. Bütünüyle okunan kaynaklar keepSources verilmezse silinir; okunmayanlar (yok sayılan katman, L kolon, açılamayan blok, duvara ait olmayan çizgi) çizimde kalır ve plan_unrecognized ile söylenir. Hiç duvar okunmazsa ret. Plan yazmadan duvarları, açıklıkları, silinecek ve kalacak kaynakları gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::ArchRecognize>(),
            output: schema::<crate::ArchRecognized>(),
            plan: Some(schema::<crate::ArchRecognizePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ofisin DXF'inden gelen zemin kat planı, mevcut duvarlar olarak".into(),
                input: json!({
                    "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"],
                    "roles": [{ "layerId": "import-duvar", "role": "wall" }, { "layerId": "import-aks", "role": "ignore" }],
                    "arch": { "level": "zemin", "phase": "existing" }
                }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_WALLS_EDIT.into(),
            version: crate::CAD_WALLS_EDIT_VERSION,
            title: "Duvarı düzenle".into(),
            summary: "Kalıcı kimlikleriyle verilen duvarların kendi alanlarını değiştirir; tek geri alma adımı “Duvarı düzenle”, hepsi ya da hiçbiri. \
                      operation.kind set: genişlik, hiza, öteleme, stil (kimliği ya da adı; null stilsiz tek gövde), uçlar, şematik dolgu, yükseklik, kat (projenin katlarından), tadilat durumu, yapım durumu ve disiplin; verilmeyenler kalır, null olanlar kalkar. \
                      Stilin bileşenleri duvara geçer; bileşenlerin sığmadığı genişlik ya da yayı yeni genişlikte çizilemeyen duvar yazılmaz. \
                      split (Böl): her duvar yolunun at noktasına en yakın yerinde ikiye; nokta duvarın iki yüzü arasında olmalı (point_off_wall), uçta olmaz (split_at_end); kapalı duvar orada açılır. join (Birleştir): uçları 1 mm içinde buluşan, alanları aynı iki ya da daha çok duvar tek duvar olur, ilk verilenin kimliğiyle (ters yöndeki gövdesi yerinde çevrilir; walls_not_touching, walls_differ); uçları buluşursa kapanır. \
                      extend (Uzat): her duvarın ucu (end, verilmezse to duvarına yakın olanı) son kenarının doğrultusunda to duvarının yoluna dek; trim (Buda): her duvar to duvarının yolunu kestiği yerde kısalır, keep noktasına en yakın parça (verilmezse uzun olanı) kalır (wall_misses, wall_closed). flip (Çevir): yol ters döner, gövde yolun öbür yanına aynalanır, uçların birleşimi yer değiştirir. \
                      replaceStyle (Stili değiştir): from stilindeki duvarlar (uids boşsa çizimin bütün duvarları) to stilini alır (style_not_used). joins (Birleşim): başlangıcın ve sonun birleşimi (auto, none, miter, butt, through) ve bileşen temizliği (cleanup); resetJoins ikisini de kurala döndürür. \
                      shift (Kaydır): tek duvarın düz kenarı edge (verilmezse 0) kendine dik by metre kayar (yolun soluna artı, sağına eksi); yanındaki köşeler bitişik kenarlar boyunca, yol ucu ona uç uca bağlı duvarın doğrultusunda (o duvarın ucu da aynı noktaya), saplandığı duvar boyunca ya da serbestse dik kayar; kenara T ile saplanan duvarların uçları kendi doğrultularında izler; açıklıklar yerinde kalır, izleyen duvarlar çıktının joined'ındadır. distance (Mesafe): side (left ya da right) yanındaki en yakın paralel duvara net mesafe distance olacak kadar Kaydır (cad.walls.distances aynı mesafeyi okur), o duvar sabit. Yay kenar, aynı doğrultudaki bitişik kenar ya da uç uca duvar, köşede birden çok duvar, kenarı yok eden ya da ters çeviren kaydırma invalid_wall; sıfır kaydırma nothing_to_set; izleyen duvar kilitli katmandaysa layer_locked. \
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
            summary: "Kalıcı kimlikleri verilen duvarların düz kenarı edge'de (verilmezse 0) iki yanındaki net mesafeleri okur: yolun soluna (left) ve sağına (right) kenarın yüzünden, aynı kattaki (duvarın kendi öbür kenarları da) ona paralel ve boyunca örtüşen en yakın düz duvar kenarının bakan yüzüne dik uzaklık; uid, edge ve face o duvarı ve yüzünü, from ve to örtüşmenin ortasında iki yüzdeki noktaları verir (metre). Paralel duvarı olmayan yan yoktur. Yay kenar ya da olmayan kenar invalid_wall. Belgeye yazmaz; kenarı kaydırmak için cad.walls.edit'in distance ya da shift işlemi."
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
            summary: "Kalıcı kimlikleri verilen kolonların dört yanındaki net mesafeleri okur: her yandan (side: 0 merkezin altı, 1 sağı, 2 üstü, 3 solu, kolonun dönüşüyle; yuvarlak kolonda çevreleyen karenin yanları) dışarıya, aynı kattaki ona paralel ve boyunca örtüşen en yakın yüze dik uzaklık: bir duvarın bakan yüzü (target wall, edge yolun kenarı, face yüzü) ya da başka bir kolonun yanı (target column, edge onun yanı). from ve to örtüşmenin ortasında iki yüzdeki noktalardır (metre); paralel yüzü olmayan yan yoktur. Akslara bağlı kolon tied ile söylenir: yeri aksların kesişimidir. Belgeye yazmaz; kolonu kaydırmak için cad.arch.edit'in distance işlemi."
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
            summary:"Sıfır kalınlıklı açık mahal sınırları. operation.create: layerId ve paths (pts metre, bulges kenar başına tan(açı/4), closed, arch açık kat ve tadilat durumu); 1–10000 yol, toplam en çok 100000 köşe, yol başına 2–10000 köşe. Duvar malzemesi/metrajı oluşturmaz; net/eksen/brüt ayırıcıyı ötelemez. operation.set uygun tek parçalı, düzlemsel, genişliksiz çoklu çizgilere açık spaceSeparator rolü ve tam arch bilgisi verir; clear yalnız rolü kaldırır, geometriyi ve görünüşü korur. Set/clear operation.uids kalıcı nesne kimlikleridir; düz çizgi dahil diğer türler reddedilir. Sıradan çizgi, katman adı, renk veya kesikli görünüş kendiliğinden sınır değildir. Kaynak değişiminde mahal/etiket/çizelge aynı undo adımında güncellenir; açık kaynak UUID listesi ayırıcıyı yalnız listede ise kullanır. Plan yazmaz, execute tek adım, expectedRevision eski önizlemenin yazılmasını engeller. KCAD 106 rolü korur; eski dosyalar değiştirilmez.".into(),
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
                      number, name, usage, arch (kat, tadilat durumu), height (yoksa katın), finishes (döşeme, duvar, tavan kaplaması) ve katmanın tipli alanlarıyla öznitelikler. Alan, çevre ve hacim sınırdan ölçülür; yöntemin bir mevzuata uyduğu söylenmez. \
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
                      operation: set (numara, ad, kullanım, yöntem, columns kolon tercihi, kat, tadilat durumu, yükseklik, kaplamalar; null alanı kaldırır; duvarlardan bulunan mahalin yöntemi değişince sınırı yeniden bulunur), unit bina programının bölümü, null bölümden çıkarır, programda olmayan unit_not_found), merge (mahaller birleşir: keep verilen mahalin, yoksa ilkinin kimliği sürer, numarası ve bölümü onun, number başka numara verir, null kaldırır; öbürleri silinir, numaraları, kaplamaları ve etiketleri onlarla gider, cevabın decisions'ı yazmadan önce söyler; bölümleri farklı mahaller space_units_merged uyarısı; birleşen sınır çizilmiş olur; değmeyen mahaller spaces_not_touching; keep birleşenlerden değilse invalid_input), split (line: iki ya da daha çok nokta; mahal çizgiyle bölünür, keep noktasını içeren parça, yoksa en büyük kimliğini korur; yeni parçalar büyüklük sırasıyla numbers numaralarını alır, verilmezse numarasız kalır (aynı numara iki kez yazılmaz), bölümü alır, finishes false değilse kaplamaları; decisions her parçanın kimliğini, numarasını, alanını, kaplamalarını ve içinde duran donatıları söyler; bölmeyen çizgi split_missed, parça sayısıyla uymayan numbers ya da hiçbir parçada olmayan keep invalid_input), rebound (sınır duvarlardan yeniden bulunur: seed verilirse ondan, yoksa kendi tohumundan, o da yoksa etiket noktasından; çizilmiş mahal duvarlardan bulunan olur). \
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
                      role (floor döşeme, balcony balkon, terrace teras), thickness (yoksa 15 cm), offset (üst yüzün katın kotuna göre farkı; ıslak hacimde eksi), arch (kat, tadilat durumu), material ve katmanın tipli alanlarıyla öznitelikler. \
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
                      operation: set (tür, kalınlık, kot farkı, kat, tadilat durumu, malzeme, kenar, metraj dışı; null alanı kaldırır, edge null kenarı kaldırır), offset (distance: bütün kenarlar ötelenir, artı dışa), subtract (parts ya da seed/boundary ile bulunan alan döşemeden çıkarılır: delik, girinti; döşemeye değmeyen ya da hepsini götüren alan slab_cut_missed), add (aynı alan döşemeye eklenir). \
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
                      Rampa (ramps): kat, genişlik, kot farkı, çizildiği eğim (kot farkının büyüklüğü ÷ eğimli uzunluk; eğimli kolu yoksa yok), eğimli uzunluk (orta çizgi eksi sahanlıklar), orta çizginin uzunluğu, sahanlık sayısı, plan alanı (dönüşlerde gönyeli şerit), başlangıç ve bitiş kotu, kata bağlıysa farkı (baseOffset), korkuluk çizgilerinin uzunluğu; toplamlarda rampArea ve rampRailingLength. \
                      Kiriş (beams): numara, tür (kiriş, lento, hatıl), kat, genişlik, yükseklik, eksenin boyu, taşıdığı döşemenin taşıyıcısı (slab) ve beton hacmi: kirişin döşemenin altında kalan kısmı (genişlik × (yükseklik − döşeme) × boy; döşemenin içindeki kısmı döşemenin hacmindedir), lento ve hatılın tamamı; toplamı beamVolume. \
                      levels, phases, usages ve layers verilen kat, tadilat durumu, kullanım ve katmanlarla süzer (usages verilince yalnız mahaller)."
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
                      radial: eğrisel aks ağı: center'dan radyal akslar, ilki angle doğrultusunda, sonrakiler turns'teki açılar kadar (radyan; hepsi aynı yöne, toplamı tam turdan az), center çevresinde yaylar, ilki radius'ta, sonrakiler rings kadar dışarıda; her aks extension (öntanımlı 1.5 m) kadar uzar: radyal ilk yaydan o kadar içeriden (merkezi geçmeden) son yaydan o kadar dışarıya, yay kendi boyunca dış radyallerden öteye (kalan turun dörtte birini geçmeden); etiketler radialStart (1) ve arcStart (A); columns kesişimlere yay yay akslara bağlı kolonlar, eni radyal aksı boyunca.                       ends (aksın, serinin, ağların): kesik aks, yalnız uçlarında kâğıtta bu kadar mm (0'dan büyük, en çok 1000).                       Aksın çizimi (kesikli çizgi, balonlar, etiket) alanlarından türer. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
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
            summary: "Kolonlar ekler; tek geri alma adımı “Kolon”. columns: her biri merkez (at) ya da iki aks (grids: kalıcı kimlikler; kolon kesişimlerinde durur, at verilirse ona en yakın kesişimde), biçim (shape: rect, round, lShape), en (width), boy (depth; yoksa en), L kolonun kolları (legs: en boyunca kolun boy yönündeki ve boy boyunca kolun en yönündeki kalınlığı; yalnız L'de, gerekli, o yanından ince), hiza (anchor: center, left, right, bottom, top, bottomLeft, bottomRight, topLeft, topRight; at ve aks kesişimi kutunun bu noktasıdır, §2), dönüş (rotation, radyan, hiza noktası çevresinde), tür (type), etiket (tag), dolu (fill). \
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
                      Basamak çizgileri başlangıçtan tread aralıkla, sahanlıkları atlayarak çizilir. Balansmanlı merdiven (winders): L ve U merdivende dönüşlerde sahanlık yerine dönüşün iki yanında winders basamak dengelenir; yürüme çizgisinde her basamak tread, iç kenarda köşeye doğru doğrusal daralır, basamak çizgisi iç kenardan yürüme çizgisinden geçip dış kenara uzanır; landing ile birlikte, düz ya da döner merdivende, sıfır, dönüşün iç kenar kaybından (genişlik × tan(dönüş/2)) kısa, kola sığmayan ya da iki dönüşte çakışan balansman invalid_stair ile reddedilir. Uymayanlar uyarıdır, ret değildir: yol rıhtlara yetmiyorsa stair_short, uzunsa stair_long; 2 × rıht + basamak 0,60–0,65 m dışındaysa stair_step_rule (yalnız bilgi); katın yüksekliği biliniyorsa rıht sayısı × rıht onu tutmuyorsa stair_rise_level. \
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
            summary: "Rampalar ekler; tek geri alma adımı “Rampa”. Her biri orta çizgi (pts: 2–4 nokta), genişlik, toplam yükseklik (rise), eğim (slope, yüzde; yoksa yükseklik ve eğimli uzunluktan), başlangıç kotu (base; yoksa 0) ya da katına göre farkı (baseOffset: başlangıç katın kotu artı fark, kat kotu değişince izler; katı ister, base ile birlikte verilmez), dönüşteki sahanlığın boyu (landing; yoksa genişlik), fit (son kol eğime göre biter), maxRun (en uzun eğimli kol: aşan kola düz köşe ve ara sahanlık), korkuluk (railings). \
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
            id: crate::CAD_BEAMS_CREATE.into(),
            version: crate::CAD_BEAMS_CREATE_VERSION,
            title: "Kiriş".into(),
            summary: "Kirişler ekler; tek geri alma adımı “Kiriş”. Her biri eksen (pts: en az 2, en çok 1000 nokta, düz parçalar), gövde genişliği (width, en çok 3 m), yükseklik (height: döşeme dahil toplam, TBDY 2018 7.4.1.1 b; en çok 5 m), hiza (justify: left eksen sol yüzü, gövde sağda; center varsayılan; right), kot farkı (offset: tepesinin asıldığı yerden farkı — katının üstündeki döşemenin taşıyıcı üstü, yoksa katın tepesi; en çok ±10 m), tür (role: yoksa kiriş; lintel lento, bondBeam hatıl), kattan yükseklik (baseOffset: alt yüzünün katın döşemesinden yüksekliği, lentonun açıklığın başında; kot farkıyla birlikte olmaz), numara (tag, K101 gibi), malzeme (material; yoksa Betonarme), kat ve tadilat durumu (arch).                       Planda kesit düzleminin üstünde kaldığı için kesikli ve ince çizilir (ISO 128-23 02.1); 1:100'de numarası, 1:50 ve ayrıntılı çizimde numarası ve ölçüsü (K101 25/50) yanına yazılır. Altındaki duvar kirişin altında biter; lento ve hatıl duvarın içindedir, duvar onların üstünden sürer. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::BeamsCreate>(),
            output: schema::<crate::BeamsCreated>(),
            plan: Some(schema::<crate::BeamsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Zemin katın tavanında 25/50 kiriş".into(),
                    input: json!({ "layerId": "kiris", "beams": [{ "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 5.0, "y": 0.0 }], "width": 0.25, "height": 0.5, "tag": "K101", "arch": { "level": "zemin" } }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ROOFS_CREATE.into(),
            version: crate::CAD_ROOFS_CREATE_VERSION,
            title: "Çatı".into(),
            summary: "Çatılar ekler; tek geri alma adımı “Çatı”. Her biri saçak çizgisi (pts: en az 3, en çok 1000 nokta, kapalı, iki yönde de; her köşesi dışa dönük — içe dönen köşeli saçak henüz çizilmez, invalid_roof), her kenarın eğimi (slopes: pts[i]'den sonrakine kenarın yüzde eğimi, sıfırdan büyük, en çok %500; null kalkan: hepsi eğimli kırma, karşılıklı ikisi beşik, biri tek eğimli, hiçbiri düz çatı), kot farkı (offset: saçağın katının tepesinden — üstündeki katın döşemesinden — farkı; en çok ±10 m), kalınlık (thickness: düşey ölçülen çatı katmanları; yoksa 0,25 m), saçak (overhang: saçağın duvarların dış yüzünden uzaklığı, 0–5 m; PAİY 42 1,00 m), kaplama (material; yoksa Kiremit), kat ve tadilat durumu (arch). Planda saçak ve kalkanlar nesnenin kalemiyle, mahya, kırma ve dereler ince, her yüzde aşağı bakan eğim oku ve yüzdesi (%33), mahyanın kotu (+7.49) çizilir (Mimarlar Odası Mimarlık Hizmetleri Şartnamesi 14.3.3 u, 14.5.3 w). Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::RoofsCreate>(),
            output: schema::<crate::RoofsCreated>(),
            plan: Some(schema::<crate::RoofsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "12 × 9 m binaya 50 cm saçaklı %33 kırma çatı".into(),
                    input: json!({ "layerId": "cati", "roofs": [{ "pts": [{ "x": -0.5, "y": -0.5 }, { "x": 12.5, "y": -0.5 }, { "x": 12.5, "y": 9.5 }, { "x": -0.5, "y": 9.5 }], "slopes": [33.0, 33.0, 33.0, 33.0], "overhang": 0.5, "arch": { "level": "k1" } }] }),
                    output: None,
                },
                CommandExample {
                    title: "Beşik çatı: uzun kenarlar %40, kısalar kalkan".into(),
                    input: json!({ "layerId": "cati", "roofs": [{ "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 12.0, "y": 0.0 }, { "x": 12.0, "y": 9.0 }, { "x": 0.0, "y": 9.0 }], "slopes": [40.0, null, 40.0, null] }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_RAILINGS_CREATE.into(),
            version: crate::CAD_RAILINGS_CREATE_VERSION,
            title: "Korkuluk".into(),
            summary: "Bağımsız korkuluklar ekler; tek geri alma adımı “Korkuluk”. Her biri küpeştesinin orta çizgisi (pts: açıkken en az 2, kapalıyken 3, en çok 1000 nokta; art arda aynı nokta yok; closed: son noktadan ilkine döner — galeri boşluğunun çevresi, ilk nokta sonda yinelenmez), yükseklik (height: döşemesinden küpeştenin üstüne; yoksa 1,10 m, PAİY 38/1 “döşeme kotundan itibaren en az 1.10 metre”; en çok 3 m), küpeşte eni (width: yoksa 0,05 m; en çok 0,5 m), dolgu (infill: balusters dikey çubuk — yoksa —, glass cam, solid dolu panel), çubuk aralığı (gap: çubuklar arasındaki net boşluk, yoksa 0,10 m, PAİY 38/4 “en fazla 0.10 metre çapında geçişe izin verilir”; en çok 1 m; yalnız dikey çubukta okunur), kot farkı (offset: döşemesinin katının döşemesinden farkı, yoksa 0; en çok ±10 m), kat ve tadilat durumu (arch). Planda küpeşte, eni kadar aralıklı iki ince çizgi (açık korkuluğun uçları kapalı); 1/200 sadeleştirilmiş görünüşte yalnız orta çizgi. Katman açıkça verilir; kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::RailingsCreate>(),
            output: schema::<crate::RailingsCreated>(),
            plan: Some(schema::<crate::RailingsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Sahanlığın boş kenarına 1,10 m dikey çubuklu korkuluk".into(),
                    input: json!({ "layerId": "korkuluk", "railings": [{ "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 3.0, "y": 0.0 }, { "x": 3.0, "y": 1.2 }], "arch": { "level": "k1" } }] }),
                    output: None,
                },
                CommandExample {
                    title: "Galeri boşluğunun çevresine cam korkuluk".into(),
                    input: json!({ "layerId": "korkuluk", "railings": [{ "pts": [{ "x": 5.0, "y": 2.0 }, { "x": 9.0, "y": 2.0 }, { "x": 9.0, "y": 5.0 }, { "x": 5.0, "y": 5.0 }], "closed": true, "infill": "glass", "arch": { "level": "k1" } }] }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_ARCH_EDIT.into(),
            version: crate::CAD_ARCH_EDIT_VERSION,
            title: "Mimari nesneyi düzenle".into(),
            summary: "Kalıcı kimlikleri verilen aks, kolon, kiriş, merdiven, rampa, çatı ve korkulukları değiştirir; tek geri alma adımı “Mimari nesneyi düzenle”, hepsi ya da hiçbiri. \
                      operation: set (türün alanları: aksın label, bubbles, bubbleSize, ends (null bütün aks); kolonun shape (L olmayan biçim kolları kaldırır), width, depth, legs, anchor (yeri kalır, gövdesi onun çevresinde kayar), rotation, type, tag, fill, material (null Betonarme); merdivenin width, risers, riserHeight, tread, landing, cut, winders (dönüşün iki yanında dengelenen basamak; sahanlığı kaldırır; null sahanlığa döner), numbering; rampanın width, rise, slope, base (kat bağını kaldırır), baseOffset (katına bağlar; null bağı kaldırır, kot yerinde kalır; base ile birlikte verilmez), landing; kirişin width, height, justify, offset, role (beam, lintel, bondBeam: kiriş, lento, hatıl), baseOffset (alt yüzünün katın döşemesinden yüksekliği; null döşemeden asılır; kot farkıyla birlikte olmaz), tag, material (hatalı kiriş invalid_beam); korkuluğun width (küpeşte eni), height, infill (balusters, glass, solid: dikey çubuk, cam, dolu), gap (çubuk aralığı; null PAİY 38/4'ün 0,10 m'si), offset (hatalı korkuluk invalid_railing); hepsinin level, phase; null alanı kaldırır; nesnenin türünde olmayan alan field_not_for_kind), renumber (akslar ya da kolonlar, tek tür, verildiği sırayla start'tan; aks etiketi değişince bağlı kolonların konum yazısı da), attach (kolonları iki aksın kesişimine bağlar, merkezlerine en yakın kesişime), detach (bağı koparır, kolon yerinde kalır), distance (tek kolon, yanı side — 0 merkezin altı, 1 sağı, 2 üstü, 3 solu, dönüşüyle — baktığı paralel yüzden distance metre olacak kadar o yana dik kayar, karşıdaki yüz sabit; cad.columns.distances aynı mesafeyi okur. Akslara bağlı kolon column_on_grids, kolon olmayan not_a_column, o yanda paralel yüz yoksa ya da eksi mesafe invalid_column, sıfır kaydırma nothing_to_set). \
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
                      Mahalin floorElevation değeri bitmiş döşemesinin kotudur (kendi döşeme kot farkı, yoksa altındaki döşemenin üstü, yoksa katın kotu); ceilingElevation tavanının kotudur (asma tavan, yazılan iç yükseklik ya da üst döşemenin sıvalı altı). Katın döşemeden döşemeye yüksekliği tavan sayılmaz; katı bilinmeyen mahalde ikisi, tavanı bilinmeyende ceilingElevation boş yazılır. Kot yazımı diğer uzunluklar gibi çizim birimi ve basamağındadır. netArea mahalın aynı kat/kapsam/kaynak ve kolon tercihiyle iç yüzler alanıdır; area seçilmiş yöntem alanı olarak kalır. Kopuk sınır veya elle çizilmiş brüt/eksen mahalında netArea boş kalır. \
                      shape çerçevedir (circle, hexagon, rectangle, rounded, none; verilmezse açıklıkta altıgen, mahalde dikdörtgen, ötekilerde daire), size yazının kâğıttaki yüksekliği (mm, verilmezse 2,5), leader çerçeveden nesnenin bağ noktasına çizgi. at verilmezse mahalde bağ noktası, ötekilerde onun 4 yazı yüksekliği üstü. \
                      all verilen türlerin etiketsiz ve görünen nesnelerine etiket koyar. all.refresh yok/null/false mevcut etiketleri korur. true yalnız layerId katmanındaki uygun mevcut etiketlere açıkça verilen all.content/shape/size alanlarını uygular; boş alan mevcut değeri korur. Kimlik, elle konum, kol ve diğer özellikler korunur; başka etiket katmanlarına dokunulmaz ve orada etiketi bulunan kaynak için ikinci etiket yaratılmaz. all.layers kaynak CAD katmanlarını, all.phases kaynak tadilat durumlarını, all.levels mimari kat kimliklerini süzer. levels yok/null tüm katlar, [null] yalnız atanmamış, dolu kimlik listesi verilen katlar, [] hiçbir kaynaktır; birleşim ve tekrar desteklenir. Dolu kimlik projede yoksa level_not_found/all.levels[i] tüm çağrıyı reddeder. Açıklık katını duvardan alır; host kayıpsa/duvar değilse açık kat ve atanmamış kapsamına girmez. Bloklar kat atanmamış sayılır. Bu süzgeç görünüş/pafta bağlamı değildir. Açık tags girdisindeki kaynaklar all tarafından yeniden işlenmez. Plan tags yeni, refreshed değişen mevcut etiketlerdir; çıktı uids/ids yalnız yeni etiketler, isteğe bağlı refreshed yenilenen UUID'lerdir. Değişmeyenler yazılmaz; yeni+yenilenen en çok10.000 ve birlikte tek undo'dur. \
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
    ]
}
