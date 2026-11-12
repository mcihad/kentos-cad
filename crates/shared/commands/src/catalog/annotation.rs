//! The catalog's commands of dimensions, texts, tables, blocks, layers and networks (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
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
            summary: "from'dan to'ya kesen çizginin kestiği duvar yüzlerini ve kolonları sırayla ölçen doğrusal ölçü zincirini layerId'ye tek adımda yazar, adı “İç ölçü zinciri”. \
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
                      shx: SHX çizgi yazı tipi, okunmaz; missing: KentOS'ta yok), stili kullanan yazı sayısı. shx ve missing stilin kendi yazı tipiyle çizilir ve her biri font_substituted uyarısıdır: değişim sessiz değildir. \
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
                      height (Yazı yüksekliği): uids yazılarının gerçek model yüksekliğini height metre yapar; içerik, yer, hizalama ve alanlar korunur, stil yüksekliği override olarak işaretlenir. Kilitli hedefte tüm işlem reddedilir. \
                      number (Numarala, TCOUNT): uids sırayla start, start+step … sayılarını alır, en az digits basamakla (önüne sıfır), prefix ve suffix'le; place replace yazının yerine, prefix başına, suffix sonuna. \
                      combine (Yazıları birleştir, TXT2MTXT): yazılar en yukarıdan aşağıya (eşitse soldan) satır satır tek yazı olur; en yukarıdakinin yerinde (üstünden), yüksekliği ve görünüşüyle, öbürleri silinir. \
                      split (Satırlara ayır): yazının her paragrafı (satır sonları arası) durduğu yerde, biçimleri ve alanlarıyla ayrı yazı olur; yazı silinir. Tek paragraflı, sütunlu ya da eğri boyunca yazı not_splittable. \
                      fit (Yazıyı eğriye oturt): tek satırlık, nesneye bağlı olmayan yazılar curve'ün üstüne, kutularının ortasının düştüğü yere; \
                      turn (Doğrultuya döndür): düz yazılar edge'in at'a en yakın kenarının doğrultusuna, okunur; straighten (Düzleştir): eğri boyunca yazılar ilk harflerinde düz; \
                      almadıkları sayılır (skipped), hiçbirini almazsa text_not_taken. \
                      combine, split, number, fit, turn ve straighten'de kilitli katmandaki yazı reddedilir. Birleşik karakterin içindeki replace eşleşmesi invalid_input ile bütün işlemi reddeder; aksanıyla birlikte bütün karakter aranmalıdır. Unicode profilli sonuç ortak glyph planıyla yazmadan doğrulanır. Unicode içeren combine farklı yerleşim/font/paragraf biçimlerini sessizce dönüştürmez; önce biçimler açıkça eşitlenmelidir. Konum/boy düzenlemesinden önce eski veya doğrulanamayan field açıkça yenilenmelidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
                      Buda'nın seçenekleri: fence (Çit) açık bir yoldur, geçtiği her görünen nesne geçtiği yerden seçilir; crossing (Kesişen) bir penceredir, ona değen her görünen nesnenin sınırlar arasındaki pencereye değen her parçası kalkar. Çit ve pencerenin kilitli katmandaki nesneleri sayılarak atlanır (layer_locked uyarısı). \
                      edgeMode extend sınırları uzantılarıyla da sayar: çizgi bütün doğrusuyla, yay çemberiyle, açık çoklu çizginin ilk ve son kenarı uçlarının ötesine. eraseUncut (hızlı kip) hiçbir sınırın kesmediği nesneyi seçildiği yerden bütünüyle siler. \
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
                      Seçimler, sınırlar, fence (Çit), crossing (Kesişen: penceredeki her uç uzar) ve edgeMode cad.entities.trim'deki gibidir; eraseUncut yok sayılır. Aynı nesnenin iki seçimi iki ucunu uzatabilir, bir uç bir kez uzar. Uç hiçbir sınıra ulaşmıyorsa seçim extend_failed uyarısıyla atlanır; hiçbir uç uzamazsa hiçbir şey yazılmaz. \
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
                      corner köşeyi söyler: vertex (çoklu çizginin ya da alanın düz kenarlar arasındaki köşesi, kimliği ve köşe sırası), lines (iki düz kenar veya bağımsız çizgi/yay/daire çifti; tıklanan yer en yakın teğet çözümü seçer, daireler bütün kalır, yaylar mevcut açı aralığında kırpılır; elips/NURBS ve yaylı yol kenarları desteklenmez) ya da all (verilen çoklu çizgi ve alanların bütün köşeleri; sığmayan köşe atlanır ve sayılır). \
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
                      topology verilirse (Topolojik düzenleme) taşınan köşeleri paylaşan görünen komşular aynı adımda izler; points noktaları da köşe sayar. Kilitli ya da geçersiz kalacak komşu uyarıyla değişmez; topology.stopOnLocked ile kilitli komşu bütün düzenlemeyi reddeder, ortak sınır ayrılmaz (shared_boundary_locked). \
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
                      Nesnenin yerini ilk parça alır, öbür parçalar ondan yeni nesnelerdir. Her parça nesnenin taşıdıklarını alır: katman, renk, kalınlık, çizgi tipi, saydamlık, sembol, öznitelikler, etiket ve ekler; kot, ölçü (M) ve genişlikler köşeleriyle gider. İlişkili ölçünün nesnedeki noktası üstünde durduğu parçayı izler, silinen kısımdaysa kopar (vertex_gone). Parçaları paylaşılan geometri çekirdeği bulur; bulamadığında break_failed çekirdeğin sözüyle döner. \
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
                      Birleştirme kuralı: aynı doğrudaki çizgiler tek çizgi, aynı çemberin yayları tek yay, kapanırsa daire olur; öbür zincirler çoklu çizgi. Birleşen nesne ilk nesnenin taşıdıklarını korur; öbürlerinin kot, ölçü ve genişlikleri köşeleriyle gelir, öznitelik ve etiketleri gelmez: değeri ayrışan anahtar data_conflict uyarısıyla söylenir. Hiçbir şeyle birleşmeyen nesne not_joined uyarısıyla nedeni söylenerek kalır. İlişkili ölçünün birleşen nesnedeki noktası birleşen nesneyi izler. \
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
            summary: "Kalıcı kimlikleriyle verilen nesneleri aynı biçimde, öbür yöne çevirir; tek geri alma adımı “Yönü çevir”. \
                      Çizginin uçları yer değiştirir; çoklu çizgi, kapalı çoklu çizgi, alan (delikleriyle), eğri ve NURBS eğrisi geriye doğru gider, yayları öbür yöne döner. Her nesne kimliğini ve taşıdıklarını korur; kot, ölçü (M) ve genişlikler köşeleriyle gider, yani onlar da tersine döner; ilişkili ölçünün noktası nesnedeki yerinde kalır. \
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
            summary: "Kalıcı kimliğiyle verilen nesnenin uzunluğunu bir ucunda (end: start ya da end) değiştirir; tek geri alma adımı “Uzat-kısalt”. \
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
                      Nesnenin kendi çizgi tipi lineType, projenin çizgi tiplerinden biri adıyla linePattern'dır: tek başına verilen desenin en yakın yerleşik tipi nesnenin tipi olur; projede olmayan ad line_type_not_found (objects[i].linePattern). \
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
        // Read-only analytic candidate recognition (CAD-ADV-07; ADR 0866).
        CommandDescriptor {
            id: crate::CAD_BLOCKS_FIND.into(),
            version: crate::CAD_BLOCKS_FIND_VERSION,
            title: "Tekrarlanan bileşenleri bul".into(),
            summary: "Kaynak çizgi, çember, yay, 2B çoklu çizgi, düz tek satırlı yazı ve statik blok koleksiyonunun tam geometrik eşlerini bulur; hiçbir nesneyi, bağı veya seçimi değiştirmez. Kaynak en çok 128, arama kapsamı en çok 50 000 nesnedir; desteklenmeyen kaynak öğesi atlanmaz, bütün istek reddedilir. Döndürme varsayılan açık, aynalama ve ölçek eşleme kapalıdır. geometryOnly açıkça seçilmedikçe katman, görünüş ve nesne verisi de aynı olmalıdır. textVariants yalnız yazı değerlerini, attributeVariants yalnız blokta tanımlı etiketlerin değerlerini ayrı ve uyarılı seçeneklerle karşılaştırma dışında bırakır; biçim ve blok tanımı aynı kalır. İç bloklar 200000 açılım iş sınırıyla bütünüyle denetlenir; aday sınırları çizilen içeriği kapsar. Tolerans mutlak metre cinsindedir, varsayılan 0.000001 (0.001 mm), en çok 0.001. Her eşleşmenin members listesi kaynak sırasıyla kimlik eşlemesidir; bounds ve dönüşüm verilir. Kesişen adaylar overlaps ile, sonuç/iş sınırına ulaşan eksik arama truncated ile bildirilir. related, bulunanlara bağlı kök ölçü/tarama/tablo/yazı/etiket adetleridir; dönüşüm yapılmış veya bağlar taşınmış sayılmaz. uids yoksa belge, boşsa boş kapsam aranır. expectedRevision eskiyse yazısız conflict. Uzun arama drawing.job.start ile başlatılıp iptal edilebilir.".into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::BlocksFind>(),
            output: schema::<crate::BlocksFound>(),
            plan: Some(schema::<crate::BlocksFound>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Seçilen bir teknik sembolün döndürülmüş ve aynalanmış eşlerini bul".into(),
                input: json!({"source":["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],"mirrors":true}),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_BLOCKS_CONVERT.into(), version: crate::CAD_BLOCKS_CONVERT_VERSION,
            title: "Tekrarları bloğa dönüştür".into(),
            summary: "Seçilmiş, nesne paylaşmayan tekrarları bir blok tanımına dönüştürür. Geometri ve katman/görünüş yeniden doğrulanır; geometryOnly aramasının sonucu veri kaybına izin değildir. Her adayın ilk nesnesi UUID ve çizim yerini blok yerleştirmesi olarak korur; diğer kökler sıfır tabanlı blok parçasına açık mapping ile gider. Sıradan bağlı ölçülerin köşe/kenar/oran yerleri ters ve döngüsel yönleriyle taşınır; belirsiz eşleme reddedilir. Plan tanımı, yerleştirmeleri, eski/yeni bağları, sapmaları ve silinecek kökleri yazmadan gösterir. Kapsam tek, görünür, veri/şema içermeyen CAD katmanındaki sürekli çizgili analitik çizgi/çember/yay/2B çoklu çizgi, düz yazı ve statik iç bloklardır. textToAttributes açık seçilirse çizim sırasının sonundaki temsil edilebilir tek satırlı yazıların her değeri TEXT_n özniteliğinde saklanır ve gerçek yerleştirilmiş yazı doğrulanır; mapping.attribute etiketi gösterir, path boş olur ve bu bir SubRef yaprağı değildir. İç blokların aynı tanımlı öznitelik değerleri aynen korunur, değer çeşitleri bloklaştırmada birleştirilmez. İç ölçü yollarının önüne yeni parça yolu eklenir; öznitelik yazısına bağlı ölçü açık ret alır. Genel nesne verisi/sembol, özel çizgi fazı, interleaved çizim sırası, tablo ve taşınamayan kök ilişkisi yazmadan açıklanır. Her seçilmiş aday yeniden tam doğrulanır; istemcinin matrisine güvenilmez. En çok 1000 grup ve 50000 çizim nesnesi; arama ve bağ eşleme durdurulabilir, iş sınırı atomik ret verir. units=m tanımı metre modelini korur; proje gösterim birimi değişmez. Tek geri alma adımı. expectedRevision eskiyse hiçbir şey yazılmaz.".into(),
            aliases: vec![], effect: CommandEffect::Document, hosts:vec![CommandHost::Desktop], headless:true,
            requires:vec![CommandRequirement::Document],permissions:vec![],undo:CommandUndo::Step,cost:CommandCost::Interactive,
            input:schema::<crate::BlocksConvert>(),output:schema::<crate::BlocksConverted>(),plan:Some(schema::<crate::BlocksConvertPlan>()),capabilities:None,
            examples:vec![CommandExample{title:"İncelenen tek bir teknik sembol grubunu dönüştür".into(),input:json!({"name":"Teknik sembol","source":["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],"groups":[["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]]}),output:None}],
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
                      dynamic, parametre/eylem ve isteğe bağlı typeParameters/typeValues ortak tip kapsamını yazar; ortak değer değişimi bütün kök/iç kullanımlara uygulanır. Farklı örnek değerleri ortak tipe sessizce birleştirilmez; kilitli etkilenen kullanım ve uyumsuz girdi atomik reddedilir. Tip formülü yalnız ortak girdileri okur. Tanım değişince bütün yerleştirmeleri yenisini gösterir. Çizimde ya da başka bir tanımda yerleştirmesi olan tanım silinmez; tanım kendini içeremez, en çok 16 düzey iç içe olabilir. \
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
                      Tipten tipe geçiş hedefin ortak değerini benimser; tipten örneğe geçiş eski ortak literal girdiyi korur. Aynı ad ve türdeki uyumlu örnek parametreleri korunur, hedef formülleri yeniden hesaplanır; taşınamayan bir örnek değeri varsa invalid_parameter_value ile tüm işlem reddedilir, sessizce sıfırlanmaz. \
                      İsteğe bağlı library (kentos.blocks sürüm 2; ortak tip değerleri varsa 3) verilirse to kaynak kitaplığın kimliğidir: aynı aile/tipin daha yeni yayını ayrı bağımlılık grafiğiyle alınır; yalnız kapsam yerleştirmeleri geçirilir. Eski tanımlar korunur; yerel hedef değişikliği ve uyumsuz girdiler atomik reddedilir. Bu kipte purge verilmez; target ve added çözümlenen hedefi bildirir. \
                      purge ile from artık hiçbir yerde yerleşmiyorsa silinir. Kilitli katmandakiler atlanır ve söylenir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
                      columns çizelgenin öznitelik sütunlarıdır (verilmezse blokların bütün etiketleri). Hiçbir şey yazılmaz."
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
                      Ortak tip değerleri get ile okunur; set ve adla reset ortak adı veya ortak lookup satırını değiştiremez. Ortak kapsam/değer için cad.blocks.edit dynamic kullanılır. Tam reset yalnız örnek girdilerini sıfırlar. Yerleştirme yalnız varsayılandan farklı örnek değerlerini tutar; cevap her yerleştirmenin bütün etkin değerlerini verir. Kilitli katmandaki yerleştirme atlanır."
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
            summary: "Katman ağacına bir harita servisinden çizilen katman ekler (add: XYZ ve TMS, WMTS, WMS, OGC API Tiles, ArcGIS REST, Google'ın 2B karoları, vektör karolar), nesneleri bir servisten alınan katman ekler (addFeed: WFS, OGC API Features, ArcGIS REST katmanı, GeoJSON adresi; alanlarıyla), birini değiştirir (update: ad, servis ya da kaynak) ya da bir servis katmanını siler (remove); tek geri alma adımı, adı işlemin. \
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
    ]
}
