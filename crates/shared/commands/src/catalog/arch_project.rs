//! The catalog's commands of an architectural project's types, options, levels and casework (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
        CommandDescriptor {
            id: crate::CAD_OPENINGS_CREATE.into(),
            version: crate::CAD_OPENINGS_CREATE_VERSION,
            title: "Açıklık ekle".into(),
            summary: "Duvarlara kapı, pencere ya da boşluk koyar; tek geri alma adımı, ilkinin biçimiyle “Kapı”, “Pencere” ya da “Boşluk”. \
                      Her açıklık duvarıyla (host: kalıcı kimliği) ve yeriyle verilir: at (duvarın taban çizgisi boyunca başından açıklığın ortasına, m), ya da point (duvarın yanında bir nokta: ortası taban çizgisinin ona en yakın yerinde), ya da point ile distance ve from (açıklığın yakın pervazı duvarın o ucundan, köşeden distance m; from start, end ya da near: noktaya yakın uç), ya da point ile middle (noktanın yanındaki kenarın ortası). \
                      type KentOS'un açıklık türüdür (kimliği ya da adı: Kapı 90, Kapı 80, İki kanatlı kapı 140, Sürme kapı 160, Pencere 120, Sabit pencere 100, Boşluk 100); verilmezse biçiminin ilki. form, width, leaves (1 ya da 2), operation (swing kanatlı, slide sürme, fixed sabit), hand (menteşe yolun başında ya da sonunda), side (kanadın açıldığı yan; verilmezse noktanın yanı, o da yoksa sol), swing (açılma açısı, derece) ve number türün değerlerinin üstüne yazılır. \
                      distanceReference uzaklığın yakın pervaza (jamb, varsayılan), merkeze (center) veya hand ile seçilen menteşeye (hinge) olduğunu söyler; distance gerekir. span iki pervaz noktasıdır: host yoluna izdüşümlerinin yay uzunluğu genişlik, yol üzerindeki ortası konum olur; at/distance/middle/width ile birlikte verilmez. point ayrıca açılma yanını seçebilir. \
                      Açıklık duvarın tek bir kenarında (düz ya da yay), uçlarının arasında olmalıdır (opening_off_wall) ve aynı duvardaki başka açıklığa binmez (openings_overlap). Pervazları duvardan çizilir, duvarın çizgileri onların arasında açılır; duvar taşınınca, uzayınca ya da kalınlaşınca açıklık onunla gider. \
                      phase açıklığın kendi tadilat durumudur (existing Mevcut, demolish İptal, new İlave, temporary Geçici; verilmezse duvarınki): Yeni durumda mevcut duvara açılan kapı new. \
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
                      operation: set (alanlar: genişlik, biçim, kanat, açılış, menteşe, yan, açı, açılım yayı, kasa, denizlik, lento, numara; null alanı kaldırır), move (by: duvar boyunca kaydırma, sona doğru artı; at: yeni konum), flip (what: side açılış yanı, hand menteşe, both ikisi), rehost (to: başka duvarın kimliği; point verilirse ortası oraya en yakın yerde, verilmezse pervazlarının ortasına en yakın yerde; aynalanmış gibi düşerse menteşe ve yan çevrilir), replaceType (to: KentOS'un türü; uids boşsa from türündeki bütün açıklıklar), renumber (prefix ve start: K1, K2 …; duvarların sırasıyla ve her duvar boyunca; uids boşsa form biçimindeki ya da bütün açıklıklar), copy (offsets: her uzaklıkta duvar boyunca bir kopya, numarasız), distance (side start/end, face left/right, distance metre: pervazın o yöndeki en yakın referansa — yüzün köşesi ya da ucu, aynı kenardaki başka açıklığın pervazı, yüzü kesen başka duvar — net mesafesi bu olur; referans sabit, açıklık kenarında kalır; mesafeyi önce cad.openings.distances okur), array (tek kaynak, korunur; layout.count yalnız yeni kopyaların adedi 1–10000; spacing: distance net pervaz arası veya centers merkez aralığı, direction start/end; fit: startMargin/endMargin net uç payları arasında eşit dağıtım, 1 adet ortalanır; uzunluklar metre, analitik duvar yolu boyunca; numbering isteğe bağlı prefix/start/step, yalnız yeni kopyalar, aynı biçimdeki mevcut açıklıkla yinelenen numara tüm işlemi reddeder). \
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
            summary: "Projenin kapı ve pencere tiplerini (ofisin K01'i) yönetir; tek geri alma adımı “Açıklık tipi”, tipi izleyen açıklıklar aynı adımda. operation: define (name, id isteğe bağlı, copyOf: projenin ya da KentOS'un tipi, form, change; sürüm 1), update (type: kimlik ya da ad, change; sürüm bir artar, tipi izleyen açıklıklar yeni değerleri alır), rename (type, name; kimlik değişmez), remove (type; izleyen açıklık varsa opening_type_in_use, detach: true açıklıkları değerleriyle tipten ayırır), import (library: kentos.opening-types 1 dosyasının içeriği, tiplerin kaynağı kitaplığın adı; onConflict: refuse, skip, replace — yalnız aynı kimlik ve kaynak, yeni sürüm —, rename), export (library adı, types; dosyanın içeriği library çıktısında, çizim değişmez). change: width (kaba boşluk) ya da nominal (by: rough kaba boşluk, unit kasa dış eni, leaf kanat eni; value: kaba boşluk bundan, montaj boşluğu ve kasadan hesaplanır), leaves, operation, swing, showSwing, frame, sill, head, depth, build (sizeBy, fit montaj boşluğu, leafThickness, split eşit olmayan kanatlarda birinci kanadın payı, frameDepth, frameOffset, threshold eşik, casing pervaz, sillBoard denizlik çıkıntısı, mullions, transoms, useZone kullanım alanı), unset (yapının alanlarını kaldırır). Tipi izleyen açıklığın istisnası (overrides) olan alan korunur (report.kept, opening_type_kept); kilitli katmandaki açıklık eski sürümde kalır (report.skipped, opening_type_skipped); yeni değerle duvarına sığmayan açıklık bütün değişikliği reddeder (opening_type_refused) ya da onRefused: skip ile eski sürümde kalır (report.refused). Plan yazmadan tabloyu ve her açıklığın sonucunu gösterir (etkilenenlerin önizlemesi)."
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
            summary: "Projenin yapı malzemelerini (kalıcı kimlik, ad, kategori, metrajda alan ya da hacim, kesitin taraması ve kalemi) yönetir ve duvar stili bileşenlerini, duvar bileşenlerini ve döşemeleri onlara bağlar; tek geri alma adımı “Malzeme”, bağlı stiller, duvarlar ve döşemeler aynı adımda. operation: define (name, id isteğe bağlı: adından; change: category structure/insulation/finish/membrane/other, quantity area/volume, hatch {pattern, scale, angle, color}, cut {color, lineWeight, lineType}), update (material: kimlik ya da ad, change; name yeniden adlandırır, kimlik değişmez; bağlı bileşenlerin material metni, taraması ve kalemi, döşemelerin material metni yeni değerleri alır), remove (material; bağlı bir şey varsa material_in_use ve bağlı duvar ve döşemelerin kimlikleri), assign (material: kimlik ya da ad, yoksa bağdan ayırır; targets: wallStyle {style, component}, wall {uid, component}, slab {uid}; bileşen sırası soldan 0'dan), adopt (bağsız her metin malzemeden bir proje malzemesi açar ve bağlar; aynı metnin taraması ya da kalemi farklı bileşenler bağlanmaz, görünüş değişmez: material_adopt_left). Kilitli katmandaki duvar ve döşeme eski değerleriyle kalır (report.locked, material_locked); bağlanacak nesne kilitliyse layer_locked. Plan yazmadan tabloyu ve etkilenenleri (report: styles, walls, slabs) gösterir. Malzeme lejandı ve Metraj bağlı bileşenleri malzemenin kimliğiyle gruplar. Ofis malzeme kitaplığı: import (library: kentos.materials 1 dosyası ya da name: KentOS kitaplığı kentos-temel; materials: kimlik ya da ad, yoksa hepsi; onConflict: refuse varsayılan material_exists, keep projeninkini korur material_kept, replace aynı kimlikliyi kitaplığınkiyle değiştirir ve bağlıları izler; başka kimlikli aynı ad keep dışında reddedilir; report.taken, replaced, kept), export (library: adı, materials: yoksa hepsi; çizim değişmez, cevabın library'si dosyanın içeriği). Bozuk ya da bilinmeyen kitaplık invalid_library."
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
            summary: "Bir banyoyu ya da odayı bir kez çizip bağlı örnekler olarak yeniden kullanır; her yazma tek geri alma adımı “Oda modülü”. operation: define (name, id isteğe bağlı, uids: bir ya da birkaç kattaki duvarlar (eğri olanlar da), açıklıklar, mahaller, mahal ayırıcılar, donatılar, kolonlar, merdivenler, rampalar, döşemeler, ölçüler, etiketler ve yazılar; akslara bağlı kolon module_kind; seçimdeki mutfak dizisi bütünüyle alınır ve modülün runs'ında tarif olarak durur, her örnek onu kendi duvarlarına yeniden kurar (aynada yanı ve tek kapakların menteşesi ters), dizinin bir kısmı ya da seçim dışındaki duvara dayanan dizi module_outside; base taban noktası; nesneler yerinde kalır ve ilk örnek olur; açıklığın duvarı, mahalin kaynakları, ölçünün noktaları ya da etiketin nesnesi seçimin dışındaysa module_outside), place (module, at, rotation radyan, mirror kendi y ekseninde aynalı, level hedef kat, phase tadilat durumu, numbering: prefix ve suffix mahal, açıklık numaralarına; örnek içi duvar, mahal kaynağı, ölçü ve etiket bağları örneğin kendi nesnelerine bağlanır), update (module, from: kaynağı veren örnek, instances: güncellenecekler, boş: hepsi; tanımın sürümü bir artar, bağlı örnekler yeni tanımı alır, yerel istisna ve açıklamaların görünüşteki yeri korunur, kilitli katmandaki örnek eski sürümde kalır: module_instance_skipped), except ve restore (uids: örneğin nesneleri; yerel istisna yap, kaynağa döndür), detach (module, instance: örnek modülden ayrılır, nesneler kalır), rename, remove (tanım silinir; örneklerin nesneleri son hâliyle kalır, bağları kopuk: sourceGone). addMembers (module, instance, uids: örnekte çizilen yeni nesneler tanıma katılır; öbür bağlı örneklere kendi matrisleriyle yerleşir), removeMembers (uids: örneğin nesneleri anahtarlarıyla tanımdan çıkar; kendisi sıradan nesne olarak kalır, öbür örneklerdeki karşılıkları silinir, istisna ya da kilitli olanlar serbest kalır: module_member_released; kalan bir üye ona bağlıysa module_member_needed), repeat (from: kaynak kat, modules: süzgeç, to: hedef katlar, her birinde numbering, unit ve örneğe özel satırlar instances: module, instance, numbering, unit; kattaki bağlı örnekler aynı yer, dönüş ve aynayla her hedef kata yerleşir; boş kat module_repeat_empty, zaten yerleşmiş module_repeat_taken; loose: kattaki modülsüz duvar, açıklık, mahal, donatı, ölçü ve etiketler de düz kopya olarak her hedefe gider, birbirlerine ve tekrarlanan örneklerin nesnelerine bağları kopyalara çevrilir, numaraları hedefin numbering'iyle (yoksa katın mahal öneki değişir); kilitli, başka türden, tekrarlanmayan örneğin ya da dışarıya bağlı olan kalır: report.left, module_repeat_left; cevabın copied'ı). Birden çok kattaki nesneler çok katlı modül olur (taban en alçak kat; duvar, mahal ve donatının storey'i taban katının kaç kat üstünde olduğu; katlar kotlarına göre sıralanır); place ve repeat üst katları hedefin üstündeki katlara koyar, yetmeyen kat module_storey_missing; katlı ile katsız nesne birlikte module_levels. place ve repeat'in unit'i bina programının bölümü: örneğin mahalleri o bölümde olur (daire modülü). Elle değişmiş (istisna yapılmamış) örnek nesnesi güncellemede kaynağın değeriyle yazılır ve söylenir: module_drift_overwritten. Plan yazmadan yapılacakları (created, changed, placed, report.updated, skipped, conflicts, changedKeys, addedKeys, removedKeys, released, deleted) gösterir. Tasarım alternatifi kapsam dışıdır."
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
            summary: "Planın bir parçası için sınırlı alternatifler (mutfak A/B) kurar ve yönetir; her yazma tek geri alma adımı “Tasarım seçeneği”. Çizim, paftalar, metraj ve çizelgeler ana modeli ve her kümenin birincil seçeneğini gösterir; görünüş tarifi (cad.views.edit recipe.options) başka bir seçeneği seçebilir. Aynı kümenin seçenekleri birbirini görmez: duvarları birleşmez, açıklıkları çakışmaz, mahal sınırını ana model ve kendi seçeneğinin duvarlarında bulur. operation: define (name, id ve option isteğe bağlı, uids: hiçbir seçenekte olmayan nesneler; ilk seçenek birincil), copy (set, option, name, primary: seçeneğin nesnelerinin bağımsız kopyası aynı yerde, aralarındaki bağlar kopyalara çevrilir), addMembers (set, option, uids: ana modelden seçeneğe), removeMembers (set, uids: seçenekten ana modele), primary (set, option), accept (set, option yoksa birincil: seçenek ana model olur, öbür seçeneklerin nesneleri silinir, küme kalkar), delete (set, option: birincil olmayan seçenek nesneleriyle silinir), rename (set, option isteğe bağlı, name). Ana modelin bir nesnesi seçeneğin nesnesine ya da bir seçeneğin nesnesi başka bir seçeneğinkine bağlıysa option_leaning. Otomatik birleştirme yoktur. Tadilat durumu seçenek kimliği değildir."
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
            summary: "Projenin tasarım seçeneği kümelerini okur; çizimi değiştirmez. set ile tek küme; her seçeneğin nesne sayısı, türlere göre sayıları, mahallerinin alanı (çizim biriminin karesi), duvarlarının boyu, silinmiş nesneleri. uid ile nesnenin kümesi ve seçeneği. compare (set, a, b, search, tolerance) iki seçeneği Veri karşılaştır'ın kuralıyla konumdan eşler: aynı sayısı, yalnız b'de (added), yalnız a'da (removed), değişen çiftler, alan ve duvar boyu farkı (b − a), sayısı farklı türler."
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
        // Kesit çıkar (docs/adr/8042).
        CommandDescriptor {
            id: crate::CAD_SECTIONS_EDIT.into(),
            version: crate::CAD_SECTIONS_EDIT_VERSION,
            title: "Kesit çıkar".into(),
            summary: "Bir kesit işaretinin kesitini modelin bildiklerinden 1/50 çizer: işaretin ilk iki noktasından geçen düşey düzlem, bakış işaretin oklarının yönünde. Kesilen (düzlemin iki yanında olan) duvarlar katmanlarıyla, döşemeler katmanlarıyla, kolonlar, kirişler, lentolar ve merdivenler malzeme malzeme tek kesit olarak kalın çizilir ve taranır; ötede görünenler ince çizilir (gizli çizgiler ayıklanır); korkuluklar ince ve taramasız (dikey çubuklu ve camlı korkulukta yalnız küpeşte katıdır, çubuklar ve camın kenarları arkasını gizlemeyen çizgilerdir; dolu korkuluk tam yüksekliğiyle); katların kotları (EK-4 Şekil 4.3), kat yükseklikleri, kesilen akslar, geçilen mahallerin numarası ve adı, ±0,00'da tesviye zemini, başlık (A-A KESİTİ) ve ölçek. Çizim işaretin bloğundadır (MM-XXXX-KESITA; nesneleri EK-4'ün malzeme katmanlarında, taramaları -T'de, görünenler derinliğe göre -1…-4) ve bir kez yerleştirilir. Modelin bilmediği çizilmez, leftOut'ta listelenir: çatı, tabii zemin, eğri duvar ve merdiven, rampa, lentosuz açıklık, plaksız merdiven, niş ve baca. operation: derive (mark: kesit işaretinin kimliği; at: çizginin sol ucunun ±0,00'ı nereye, yoksa çizimin 5 m sağı), refresh (mark: bloğu modelin şimdiki hâliyle yeniden yazar; her yerleşim güncellenir). Tek geri alma adımı “Kesit çıkar” ya da “Kesiti yeniden çiz”."
                .into(),
            aliases: vec!["KESITCIKAR".into(), "BIMSECTION".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::BuildingSectionsEdit>(),
            output: schema::<crate::BuildingSectionsEdited>(),
            plan: Some(schema::<crate::BuildingSectionsEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "A-A kesitini çizimin sağına çıkar".into(),
                    input: json!({"operation": {"kind": "derive", "mark": "0192f5a1-7880-7000-8000-000000000009"}}),
                    output: None,
                },
                CommandExample {
                    title: "Kapı değişti: kesiti yeniden çiz".into(),
                    input: json!({"operation": {"kind": "refresh", "mark": "0192f5a1-7880-7000-8000-000000000009"}}),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_SECTIONS_QUERY.into(),
            version: crate::CAD_SECTIONS_QUERY_VERSION,
            title: "Kesitler".into(),
            summary: "Çizimin kesit işaretlerini ve kesitlerinin durumunu okur: güncel (yeniden çizilse aynı), eski (model değişti), çıkarılmadı (işaretin kesiti yok), kaynağı yok (bloğun işareti silinmiş); her birinin adı (A-A kesiti), bloğu ve yerleşim sayısı. mark ile bir işaretin, uid ile bir yerleşimin kesiti.".into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::BuildingSectionsQuery>(),
            output: schema::<crate::BuildingSectionsQueried>(),
            plan: Some(schema::<crate::BuildingSectionsQueried>()),
            capabilities: None,
            examples: vec![],
        },
        // Türetilmiş çizim: oda iç görünüşü (docs/adr/0799), held together by fixtures/commands/v1/cad.elevations.edit.json
        // and cad.elevations.query.json (scripts/fixtures/room_elevation_cases.py).
        CommandDescriptor {
            id: crate::CAD_ELEVATIONS_EDIT.into(),
            version: crate::CAD_ELEVATIONS_EDIT_VERSION,
            title: "Oda iç görünüşü".into(),
            summary: "Bir mahalin iç görünüşlerini modelin bildiklerinden türetir: mahalin dış halkalarının her düz yanı (0'dan, parça parça numaralı) odanın içinden bakılarak çizilir; yüzü o yanda olan aynı kattaki duvarlar, yüksekliği (mahalinki, yoksa duvarların en alçağı: duvarın kendisi ya da katın), duvarlardaki açıklıklar denizlik ve lento kotlarıyla, açıklık numaraları, başlık ve iki ölçü. Nesneler İç görünüşler grubunda yeni bir katmanda, katmanın türetim kaydıyla (mahal, yanlar, başlangıç, her nesnenin rolü ve kimliği, çizilenin özeti); tek geri alma adımı “Oda iç görünüşü”. Modelin bilmediği çizilmez, leftOut'ta listelenir: yaylı yan (curvedSide), mahalin boşluğu (hole), odadaki kolon, merdiven, rampa ve blok (column, stair, ramp, fixture), baca (flue), kopuk açıklık (brokenOpening); üst kotu olmayan açıklığın pervazları kesikli (headUnknown). operation: derive (space, sides isteğe bağlı, at: başlangıç, yoksa mahalin kutusunun 3 m sağı; gap: yanların arası, yoksa 1 m; name: katmanın adı), refresh (layer: modelin şimdiki hâliyle yeniden çizer, nesneler rollerine göre aynı kimlikle kalır), release (layer: türetim kaydını kaldırır, çizim kalır). Kesit/görünüş işareti (cad.viewmarks.create) yalnız başvurudur; işaret koymak çizim üretmez."
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
            summary: "Çizimin türetilmiş oda iç görünüşlerini katman ağacının sırasıyla okur; çizimi değiştirmez. Her biri için katmanı, mahali ve adı, istenen yanlar, durum: current (yeniden çizilse aynı ve nesnelerinin hepsi yerinde), stale (model değişti ya da nesnelerinden bazıları silindi), gone (mahali yok); silinmiş nesneler (missing), model değişmemişken elle değiştirilmiş nesneler (edited) ve türetildiği çizim sürümü. layer ile tek katman, uid ile nesnenin üyesi olduğu iç görünüş."
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
            summary: "Projenin oda modüllerini örnekleriyle okur; çizimi değiştirmez. module ile tek modül, uid ile nesnenin hangi modülün hangi örneğinde ve hangi anahtarla olduğu. Her örnek: sürümü, stale (tanımdan eski), nesne sayısı, yerel istisnaları, missing (nesnesi silinmiş anahtarlar), drifted (elle değişmiş, istisna olmayan nesneler), broken (sourceGone), unit (bina programının bölümü), katı, aynalı olup olmadığı; çok katlı modülde storeys (kat sayısı) ve örneğin levels'ı (durduğu katlar, taban önce); uid ile drifted o nesnenin elle değişip değişmediği."
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
            summary: "Çizimin kapı, pencere ve öbür açıklıklarını tipleri ve genişlikleriyle okur; çizimi değiştirmez. Süzgeçler: uids (seçim kapsamı), type (projenin ya da KentOS'un tipi, kimlik ya da ad: Aynı tiptekileri seç), level (duvarın katı; null katsız), form. Her satır: uid, form, number, type ve typeName, typeVersion (izlediği tipin aldığı sürümü), stale (tipinden eski), overrides (örnek istisnaları), level, host ve sizes: rough kaba boşluk (açıklığın width'i), unit kasa dış eni (kaba boşluk eksi iki yanda montaj boşluğu), leaf kanat eni (kasanın içi), leaves (kanatlar), clear net geçiş (kapı açıkken açık kanadın yüzünden karşı kasaya; açıda kanadın izdüşümü ve kalınlığı düşülür, kalınlık verilmezse 0.04 m), clearFirst (iki kanatlıda yalnız birinci kanat açıkken), sizeBy ve nominal (tipin ölçüsünün adladığı genişlik). types: projenin tipleri, bütün çizimde izleyen, eski sürümde kalan ve istisnalı açıklık sayılarıyla."
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
            summary: "Kalıcı kimlikleri verilen kapı, pencere ve boşlukların geçici ölçülerini okur: duvar kenarının seçilen yüzünde (face: left, varsayılan, ya da right; yolun yönüne göre) pervazlardan önceki (start) ve sonraki (end) en yakın referansa net mesafe. Referans yüzün köşesi ya da ucu (end), aynı kenardaki başka açıklığın pervazı (opening) ya da yüzü kesen başka duvarın yüzü (wall); from pervazın, to referansın yüzdeki noktası, metre. Yay kenardaki açıklık reddedilir (invalid_opening). Belgeye yazmaz; mesafeyi değiştirmek için cad.openings.edit'in distance işlemi."
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
            summary: "Çizimin mimari sorunlarını tek listede verir; çizimi değiştirmez. Gruplar (checks; yok: hepsi): walls (birleşmeyen duvarlar wall_join_*, öbür duvara maxGap kala biten uç arch_wall_end_gap, birbirine yakın ama buluşmayan uçlar arch_wall_ends_apart; her kat ayrı), openings (arch_opening_broken, tag_missing_type), spaces (arch_space_broken; alanı minArea'dan küçük arch_space_small; ortalama eni 2·alan/çevre minWidth'ten küçük arch_space_narrow), numbers (tag_duplicate_number), tags (tag_target_gone), dimensions (arch_dimension_set_broken, arch_dimension_broken), grids (aynı katta aynı etiketli ikinci ve sonraki akslar arch_grid_label_duplicate; düzeltmesi üst üste binen ve kolon bağlı olmayan aksı siler cad.entities.delete, öbürünü sıradaki boş etikete adlandırır cad.arch.edit), columns (arch_column_grid_gone), viewmarks (arch_viewmark_target_gone), levels (arch_level_missing: duvar, mahal, kolon, kiriş, merdiven, rampa ya da mahal ayırıcı katsız ya da projede olmayan bir katta; düzeltmesi cad.levels.assign ile level girdisinin katı, yok: zemin kat — kotu 0 olan, yoksa ilk kat; projede olmayan level level_not_found), accessibility (yalnız profile verilince; erisilebilirlik-rampa/1 — TS 9111 özeti — ile rampanın eğimi kot farkının bandını aşarsa ramp_slope_over_profile, 10 m'den uzun eğimli kolu %6'dan dikse ramp_long_run_slope, sahanlığı 1.50 m'den kısaysa ramp_landing_short, dönen rampa 1.50 m'den darsa ramp_turn_narrow; son ikisinin düzeltmesi cad.arch.edit; katalogda olmayan profile invalid_arch_audit). walls'ta minLength'ten (0.05 m) kısa açık duvar arch_wall_short (açıklığı yoksa düzeltmesi siler), aynı kattaki iki düz duvar boyları boyunca üst üste arch_wall_overlap (biri öbürünün aynısıysa sonrakini siler), aynı kattaki iki mahal ayırıcı aynı doğruda üst üste arch_separator_overlap; tags'te yazısı boş etiket tag_empty; stairs'te profil olmadan da merdivenin kendi uyarıları stair_short, stair_long, stair_rise_level (düzeltmesi heightFromLevel); ramps grubu ramp_no_run, ramp_slope_mismatch (düzeltmesi eğimi hesaplanana eşitler); casework grubu arch_casework_broken (düzeltmesi diziyi bırakır cad.casework.edit release). Eşikler metre ve m²: maxGap 0.10, minArea 0.10, minWidth 0.15, minLength 0.05. Her bulgu kalıcı anahtarı (key: kod:kimlikler, uç boşluğunda :start/:end, ikinci benzer #2), iletisi, onarımı (repair), nesneleri (uids), yeri (at) ve açık bir düzeltmesi varsa fix'iyle gelir: label ve step (cad.changeset.apply'ın adımı: command, input); step olduğu gibi çalıştırılır, tek geri alma adımıdır. within: levels (kat kimlikleri, null katsız) ve uids (nesneler) bulguyu nesnelerinden biri o katta ve o nesnelerdense tutar (level_not_found, entity_not_found); kapsam dışı sayısı outside. Projenin istisnaları (cad.arch.exceptions): özeti tutan bulgu exception (exceptions: hide varsayılan saklar ve excepted'te sayar, show listeler, only yalnız istisnalıları listeler), bulgusu değişen istisna lapsed ve exception_lapsed uyarısı, bulgusu artık olmayanların anahtarları stale. regulation (Mevzuat): Planlı Alanlar İmar Yönetmeliği; konut piyesleri 29/1 (oturma odası 3,00 m/12 m², yatak odası 2,50/9, mutfak 1,50/3,30, banyo 1,50/3, tuvalet 1,00/1,20; dar kenar en büyük iç dairenin çapı, alan iç yüzler) paiy_room_area_below, paiy_room_width_below, bölümde eksik piyes paiy_room_missing (29/2: üç ve daha az odada banyo tuvaleti karşılar), hol ve koridor 1,20 m paiy_corridor_narrow (29/3), konutta kat yüksekliği 3,60 m (bodrum hariç üç kata kadar 4,00) paiy_storey_height_over (28/1-c), adı ya da kullanımı olan mahalin iç yüksekliği bitmiş döşemesinden tavanına (asma tavan, yazılan iç yükseklik, üst döşemenin sıvalı altı; yoksa kattan kata) iskân edilen mahalde 2,60 (28/4), banyo, tuvalet, lavabo, kiler, depo, merdiven altı ve geçitte 2,20 (28/5), garaj, otopark, kazan dairesi ve teknik mahalde 2,20 m (28/6) paiy_inside_height_below (balkon, teras, sığınak, şaft, çatı arası ve asma kattaki mahal okunmaz), kapı yüksekliği 2,10 m paiy_door_height_below (39/1-a), net temiz genişlik bina girişinde 1,50, bağımsız bölüm girişinde 1,00, öbür kapılarda 0,90 m paiy_door_width_below (39/1-b, c; 13.8.2025); bulgu rule alanında maddeyi ve Resmî Gazete'yi taşır. Niteliği daire, mesken ya da konut olmayan bölümün piyesleri okunmaz. Merdiven 31 — ortak merdiven kolu ve ara sahanlığı konut binasında 1,20, konut dışında 1,50, bağımsız bölüm içinde 1,00 m paiy_stair_width_narrow, paiy_stair_landing_short (merdivenin durduğu mahalin bölümünden; hiçbir mahalde değilse dış merdiven, kol ölçülmez), rıht asansörsüz binada 0,16, asansörlüde 0,18 m paiy_stair_riser_high, 2a+b 0,60–0,64, basamak 0,27, döner merdivende dar uçta 0,10 m paiy_stair_step; korkuluk ortak merdivende iki yanda (31/1-ç), öbür merdivende birden çok basamakta ve rampada en az bir (38/1) paiy_railing_missing; rampa eğimi kot farkına göre %8/7/6/5 paiy_ramp_steep (30/12; 2,75 m ve daha geniş rampa taşıt rampasıdır, okunmaz); zemin kattan yukarıda denizliği 0,90 m'den alçak pencere paiy_window_railing (38/1); Işıklık adlı boşluk 1–6 katta 1,50 m ve 4,50 m², 7 ve daha fazlada 2,00 m ve 9 m² paiy_lightwell_small (32/2), Hava bacası 0,60 × 0,60 m paiy_air_shaft_small (32/3); bodrumları (teknik hacimler dışında mahali olanlar) dahil 3 katta asansör yeri, 4 ve daha fazlada asansör paiy_elevator_missing (34/1, 1.7.2026; tek bağımsız bölümlü konut hariç), Asansör adlı mahalin net ölçüsü kabinden (1,20 m, 1,80 m²) küçük ya da kitaplığın Asansör bloğunun kabini bundan, kapısı 0,90 m'den küçük paiy_elevator_small (kuyu net ölçüsüyle, içindeki blok kabin ve kapısıyla ölçülür; bloğun düzeltmesi parametreleri, cad.blocks.parameters), asansör kapısının açıldığı sahanlık sürme kapıda 1,20, kanatlıda 1,50 m'den dar paiy_elevator_landing_narrow (34/2); duvarı olan çizimde Kesit çıkar'ın çıkardığı kesit ikiden azsa paiy_sections_missing, çıkarılmış kesitlerin hiçbirinin çizgisi ortak merdivenin kollarını ya da sahanlığını kesmiyorsa paiy_section_stair_missing (57/2-a-6); binanın dış çizgisinin (katıların birleşimi) kuzey okuna göre sekiz yönden en uzunun beşte birinden ve 2 m'den uzun her cephesine bakan, Görünüş çıkar'ın çıkardığı bir görünüş yoksa paiy_elevations_missing (57/2-a-6: yeteri sayıda görünüş; Mimarlar Odası: bütün görünüşler çizilir). Çatılar: mahya saçak seviyesinden (son kat tavan döşemesinin üstü, çatının kot farkı dahil) 5,00 m'yi aşarsa paiy_ridge_high (40/4; Sivas 40/5-g 4,00 m), binadan çizilen çatının saçağı 1,00 m'den genişse paiy_eaves_wide (42/1), hava bacası çatı örtüsünü 0,60 m'den çok aşarsa paiy_roof_vent_high (40/5; düzeltmesi 0,60 m), duman bacası mahyanın 0,80 m üstüne çıkmazsa bykhy_chimney_low (BYKHY 58/2; düzeltmesi 0,80 m); yerel yönetmelik Sivas ise eğim %40'ı aşarsa local_roof_slope_steep (40/5-g; düzeltmesi dik kenarları %40'a indirir), eğimli çatıda kar siperliği yoksa local_snow_guard_missing (42/5; düzeltmesi saçaktan 30 cm), çatı penceresi 1,20 × 1,50 m'den büyükse local_roof_window_large, bina cephe hattına 1 m'den ya da başka bir pencereye çatının üstünde 2 m'den yakınsa local_roof_window_close (40/5-e). Korkuluklar: döşemesinden 1,10 m'den alçak korkuluk paiy_railing_low (38/1; düzeltmesi 1,10 m), dikey çubuklarının arası 0,10 m'den geniş korkuluk paiy_railing_gap (38/4; düzeltmesi 0,10 m), döşeme boşluğunun (galeri boşluğu) duvarsız ve alt kattan gelen merdivenin çıkışı dışındaki kenarlarının ya da kenarı olmayan balkonun, terasın serbest kenarlarının 0,10 m ve daha uzunu katının korkuluklarınca tutulmuyorsa paiy_railing_missing (38/1; balkonda ve terasta düzeltmesi döşemeye korkuluk kenarı, cad.slabs.edit; boşlukta Korkuluk aracının Boşluktan'ı). Asansör: kitaplığın Asansör bloğu asansördür (34/1, rıht 0,18 m); ±0,00'dan sayılan 10 ve daha fazla katlı binada en az bir asansörün kabini 1,20 m dar kenarlı ve 2,52 m², kapısı 1,10 m değilse paiy_elevator_stretcher (34/5; düzeltmesi en büyük kabini sedye ölçüsüne getirir). Otopark yerleri: kitaplığın Otopark yeri blokları çizimin park yerleridir (Otopark hesabı onları sayar; yoksa mahal alanı ÷ 20 m²); kısa kenarı 2,40 m'den (engellide 3,50, kamyonette 3,00) ya da uzun kenarı 4,90 m'den (kamyonette 6,00) küçükse parking_space_small (5/1-h-8, 9; düzeltmesi bloğun En ve Boy'u), Otopark ya da Garaj adlı mahaldeki yer sayısı net alanı ÷ 20 m²'yi aşarsa parking_area_short (4/1-ç), umumi binada (konutsuz, büro ya da dükkân bölümlü) her 20 yerde 1 ve en az 1 engelli yeri yoksa parking_disabled_short (4/1-ı), zorunlu otopark 20 ve üstündeyse %5'i ve en az 1 şarjlı yer yoksa parking_charging_short (5/1-j; düzeltmesi bir binek yerini Şarjlı yapar). Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-9 model kalite kontrol formunun çizimin bozduğu soruları, cad.arch.ifc'nin yazdığı modelden: künyede EK-7'nin proje bilgileri eksikse digital_project_info_missing (Md. 13), proje kodu yoksa ya da 000000 ise digital_project_code_missing (Md. 6, EK-1), numarası ya da adı olmayan mahaller digital_space_unnamed (Md. 10), çizim yerel koordinatta, sistemi TUREF 3° dilimi (EPSG:5253–5259) değil ya da ±0.00'ın kotu yoksa digital_system_wrong (EK-7 Tablo 7.6–7.7), modelde mahal yoksa digital_no_spaces, IFC'ye girmeyen nesneler (rampa, gövdesiz nesne) digital_model_loss (Md. 9; nesneleri bulgunun); duvarsız çizimde hiçbiri. Projenin yerel imar yönetmeliği (settings.arch.localRules: sivas) varsa değerleri PAİY'nin üstüne okunur (PAİY 69/3): sıkı olan bağlar ve iletide öbürü de yazılır (Sivas ortak merdiven ve ara sahanlık 1,30 m, 3 katta asansör, konutta kat yüksekliği 3,60 m); ölçü yerel yönetmeliğe uyup PAİY'ye uymuyorsa regulation_conflict (Sivas hava bacası 0,30 m ve 0,36 m², tuvalet ve balkon kapısı 0,80 m; rule iki maddeyi taşır). Merdiven ve rampa bulgularının düzeltmesi cad.arch.edit (genişlik, sahanlık, iki yana korkuluk). Yangın (Binaların Yangından Korunması Hakkında Yönetmelik): kaçış yolu mahallerin net içinden, kapı ve boşluklardan, mahal ayırıcılardan yürünür; en uzak nokta duvardan 0,40 m içeride (32/6), köşeler de 0,40 m'den dönülür; kat çıkışı Merdiven adlı mahal, açık merdivenin ayağı (üst katta başı) ya da ±0,00 ve altında dışarı açılan kapı; bir bölümün yolu başka bölümün mahallerinden geçmez (33/6-b). Konut olmayan mahalde tek çıkışta bütün yol, iki çıkışta tek yönlü kısım ve bütün yol Ek-5/B'yi aşarsa bykhy_escape_distance_over (büro 15/45 m, yağmurlamada 30/75 …), koridorun çıkmaz kısmı 15 m (yağmurlamada 20, 4/i); konutta daire içi 20 m (yağmurlamada 30, 48/2) ve daire kapısından kat çıkışına Ek-5/B apartman (48/4) — bodrum dahil 4 kata kadar konutta ve tek evde aranmaz (48/1); çıkışa ulaşmayan mahal bykhy_escape_no_route; kişi yükü Ek-5/A (salon sıraları net, öbürleri eksen alanıyla) ile katın toplam çıkış eni kişi ÷ birim genişlik kişisi × 0,50 m ve 50/501/2001 kişide her kaçış yolu 1,00/1,50/2,00 m bykhy_exit_width_below (33/1; yüksek binada merdiven 1,20 m, 33/2), 50 kişiyi aşan mahalde 2, katta 500'ü aşanda 3, 1000'i aşanda 4 çıkış bykhy_exit_count_below (39/2), kaçış yolu olan koridor 1,10 m (yüksek binada 1,20) bykhy_corridor_narrow, konutta yapı yüksekliği 21,50/30,50/51,50 m'yi aşınca en az 2 kaçış merdiveni bykhy_stair_count_below (48/5). settings.arch.sprinklers yağmurlamalı sınırları okutur. Korunumluluk, yangın dayanımı ve güvenlik holü çizimden okunmaz; bulguda yazılır. Otopark ve Sığınak Yönetmelikleri — bölümlerin Ek-1 ihtiyacından (konutta BB brüt alanına göre 3/2/1 daireye 1, 180 m² üstü daireye 2; dükkân, mağaza, banka 50 m²'ye 1, bölüm başına en az 1; büro 50 m²'ye 1; toplam bir üste) Otopark ya da Garaj adlı mahallerin net alanı ÷ 20 m² (otopark yeri blokları çizildiyse onların sayısı) az ise parking_short; konutta oda sayısından (1 oda 2, 2 oda 3, 3 ve daha fazla 4 kişi), öbür bölümlerde brüt alan ÷ 20'den kişi başına 1 m² net, Sığınak adlı mahallerin alanı az ise shelter_short (7/a–c muafiyetlerinde aranmaz), 100 m²'yi aşan sığınakta 2 çıkış shelter_exit_count (8/c), iç yükseklik 2,40 m shelter_height_low (8/a). regulation, doors ve fixtures gibi yalnız adıyla istenince çalışır. Bulgu yokluğu ruhsat ya da standart uygunluğu değildir."
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
            summary: "Mimari denetimin bulgularını bilerek bırakılmış sayar ya da işareti kaldırır; tek geri alma adımı. operation: mark (İstisna yap: keys cad.arch.audit'in verdiği anahtarlar, note isteğe bağlı not; istisna bulgunun o anki kodu, nesneleri, iletisi ve yerinin özetine bağlanır, bulgu değişince düşer), unmark (İstisnayı kaldır: keys istisnaların anahtarları), prune (İstisnaları temizle: bulgusu artık olmayanlar). audit bulguları bulan denetimin girdisidir (yok: bütün gruplar, varsayılan eşikler, profilsiz); within ve exceptions okunmaz. İstisnalar projenin ayarıdır (arch.auditExceptions, KCAD 130). Retler: invalid_input (anahtar yok ya da yineleniyor, not 1000 karakterden uzun), denetimin retleri audit. yoluyla, finding_not_found (keys[i]: bulgu ya da istisna yok). Zaten öyle olanlar değişmez, değişen yoksa adım yazılmaz. Çıktı değişen anahtarlar (changed), istisna sayısı ve revizyon. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
            summary: "Bina programını ve bölümlerine verilen mahalleri yazar; tek geri alma adımı “Bina programı”, hepsi ya da hiçbiri. program (verilmezse olduğu gibi): buildings (bina ve kanatları: kimlik, ad, levels: kapladıkları katların kimlikleri, kanadınki binasınınkilerden), units (bağımsız bölüm, ortak alan ya da dış alan: kimlik, numara, ad, kind unit/common/outdoor, bina, kanat, recordedArea kayıtlı alan m²: ölçümle hiç yazılmaz, shareWeight: ağırlığa göre paylaştırmada bölümün pay ağırlığı, ondalık metin), shareRule (ortak alanların bağımsız bölümlere paylaştırılması: equal eşit, net net alana göre, weight ağırlığa göre; kural kentos.share/1), briefs (kullanıma göre tasarım hedefi: areaMin, areaMax m², minWidth net en, minPassage kapılar arası net geçiş, adjacent komşu kullanımlar, reach kapı ya da açık geçişle ulaşılacak kullanımlar, outdoor balkon gibi dış mahal; yasal sınır değildir), clearances (blok adına donatının önündeki kullanım alanı: width, depth m), version (hedeflerin adı ve sürümü, bulgularda yazılır). Kat ile bölüm arasında zorunlu bire bir bağ yoktur: bölümün katları mahallerinden okunur (dubleks iki kat). assign: uids mahalleri unit bölümüne verir (null bölümden çıkarır). Kaldırılan bölümde hâlâ mahal kalırsa unit_in_use (uids); kaldırılan bina ya da kanat hâlâ bölümlerde yazılıysa building_in_use (bölümlerin numaraları iletide). Retler: nothing_to_set, building_in_use, invalid_program (programın yolu), level_not_found (binanın ya da kanadın katı projede yok), no_entities, invalid_uid, entity_not_found, not_a_space, unit_not_found, unit_in_use, layer_locked. Çıktı yazılan program, bölümü değişen mahaller (assigned) ve revizyon. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
            summary: "Bina programını çizimle karşılaştırır (salt okunur). units: bölümlerin toplamları, her mahal bir kez: rooms (odalar), outdoor (hedefi outdoor olan balkon, teras), voids (şaft), methods (alanın alındığı yöntemler: net iç yüzler, duvar ekseni, brüt dış yüzler; birden çok ise karışık), levels (mahallerinin katları), recordedArea ve recordedDifference (kayıtlı alan yalnız okunur). totals: units, unitOutdoor, common, outdoor, voids, unassigned, all (boşluklar dışı hepsi; ortak alan bölüme girmez). spaces: mahal başına area (kendi yöntemiyle), width (içindeki en büyük dairenin çapı, net en), doors (kapılar ve başka mahale açık geçişler: ayırıcı), passage ve passageAt (hedefi minPassage olan mahalde kapılar arası en geniş geçiş ve en dar yeri), targetMin, targetMax, targetDifference (aralığın altı eksi, üstü artı, içi 0), passageDifference, adjacent ve reach (istenen kullanımlar ve onları karşılayan mahaller). findings: Mimari denetim'in program grubu: program_unit_missing (Düzelt: bölümden çıkar), program_area_below, program_area_above, program_width_narrow, program_passage_narrow, program_adjacent_missing, program_reach_missing, program_no_door, program_fixture_clearance; ileti ölçülen değeri, farkı ve kuralın sürümünü (kentos.program/1 · hedeflerin sürümü) yazar; hedefler yasal sınır değildir. findings false ise yalnız sayılar (geçiş taraması yapılmaz). units ve levels süzer. shares (programın shareRule'u varsa): kentos.share/1 kuralı: her ortak alan kendi havuzudur, kapsamı kanadı, yoksa binası, yoksa bütün program; odaları (şaft ve dış mahal dışı) 0,01 m² adıma, tam ikili değeri yarıda çifte yuvarlanır; kapsamdaki bağımsız bölümlere ağırlıklarıyla (eşit 1, net alan adımda, pay ağırlığı tam ondalık) T·w/W tam payı, tabanı ve kalan adımlar en büyük artıklara (eşitlikte programdaki sıra); paylar havuzun alanına tam eşittir. pools: havuzun alanı, bölüm payları (weight, share, exact: m² kesri), unshared (neden); net, shared, unshared, gross toplamları ondalık metin. Bölüm satırının share'i: net, share, gross (süzgeç ne olursa olsun bütün mahalleriyle). Paylaştırılamayan havuz share_unshared uyarısı. program_level_outside: mahal bölümünün binasının ya da kanadının katlarında değil. Uzun iş olarak çalışabilir: ilerlemesini söyler, durdurulunca cancelled. Retler: unit_not_found, level_not_found."
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
            summary: "Mimari nesnelere projenin bir katını verir; yerleri değişmez, tek geri alma adımı “Kat ver”. uids verilirse onlar (duvar, mahal, döşeme, aks, kolon, kiriş, merdiven, rampa, mahal ayırıcı, blok; kapı, pencere, etiket, kaplama ve çizginin kendi katı yok: not_levelled), verilmezse katsız ya da projede olmayan bir kattaki bütün duvar, mahal, döşeme, kolon, kiriş, merdiven, rampa ve mahal ayırıcılar (Denetim'in arch_level_missing bulguları; akslar ve bloklar katsız kalabilir). Zaten o kattaki nesne değişmez; kilitli katmandakiler atlanır ve sayılır (layer_locked uyarısı), hepsi kilitliyse ret. Hiçbiri değişmeyecekse nothing_to_assign. Mahalın sınırı aynı adımda yeni katının duvarları arasında yeniden bulunur; kapanmayan ya da duvarları katından ayrılan mahal son sınırıyla kalır (arch_space_broken uyarısı). Plan kat verilecekleri (uid, id, kind, from) çizimin sırasıyla verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
            summary: "Kuzey okunu yerleştirir; tek geri alma adımı “Kuzey oku”. Çizimin kendi “Kuzey oku” bloğu varsa o, yoksa KentOS blok kitaplığınınki aynı adımda eklenip yerleştirilir (added). at okun dairesinin merkezidir; size kâğıttaki eni (mm, yok: 15; 0'dan büyük, en çok 200), çizim ölçeğiyle büyür: ölçek = size/1000 · ölçek paydası / bloğun doğu-batı eni (yazıları sayılmaz). north okun gösterdiği kuzeydir: plan (yok: çizimin yukarısı, projenin plan kuzeyi), true (bu noktadan geçen meridyen: projenin koordinat sisteminin meridyen yakınsaması, convergence; yerel projede no_true_north) ya da angle (planın kuzeyinden saat yönünün tersine radyan). Dönüş (rotation) bu açıdır. Ret sırası: not_finite (at, açı, boy), invalid_scale, invalid_revision, revision_conflict, no_true_north, katman (layer_not_found, not_a_layer, layer_locked); gizli katmana layer_hidden uyarısı. Plan yerleştirmeyi (id 0), eklenecek bloğu, dönüşü ve yakınsamayı verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
            summary: "Revizyon bulutunu ve üçgenini birlikte yazar; tek geri alma adımı “Revizyon”. Bulut cad.revcloud.create'in çizdiğidir (frame: dikdörtgen, çokgen ya da nesne; arc yay boyu), özniteliği Revizyon numaradır (date, description verilirse Tarih, Açıklama). Üçgen KentOS blok kitaplığının “Revizyon” bloğudur (çizimin kendisininki varsa o; yoksa aynı adımda eklenir, added), REV özniteliği numara; size kâğıttaki eni (mm, yok: 6; en çok 100), çizim ölçeğiyle büyür. at üçgenin merkezidir; yoksa bulutun sağ üst köşesinin dışında: köşelerin en doğusu ve en kuzeyi artı yarım yay ve yarım üçgen. Revizyonları “Revizyon” bloğunun REV'e göre Blok çizelgesi listeler. Ret sırası: invalid_mark (numara boş, kenarı boşluklu, denetim karakterli ya da 16 karakterden uzun; tarih 32, açıklama 240 karakterden uzun), not_finite (at, boy), invalid_scale, sonra cad.revcloud.create'inkiler (çerçeve, yay, beklenen sürüm, katman). Plan bulutu ve üçgeni (id 0) ve eklenecek bloğu verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
            summary: "Duvarların malzemelerinin lejandını yazar; tek geri alma adımı “Malzeme lejandı”. Her satır bir duvar bileşenidir (duvarlar çizimin sırasıyla, bileşenleri sol yüzden): adı malzemesi (yoksa bileşenin adı), örneği duvarın taradığı gibi taranmış kutu; adı ya da taraması öncekilerden farklı olan yeni satırdır, walls onu taşıyan duvar sayısı. Lejant “Malzeme lejandı” bloğudur: çizimde varsa yerinde yeniden yapılır (remade, bütün yerleştirmeleri izler), at verilirse bir yerleştirmesi de konur (at sol üst köşe). height yazının kâğıttaki yüksekliğidir (mm, yok: 2,5; en çok 50), H = height · ölçek paydası / 1000: başlık sol üstte, i. satırın kutusu 6H eninde 3H yüksekliğinde, üstü 2H + 4H·i aşağıda, adı 7H sağda kutunun ortası hizasında. Ret sırası: not_finite (at, yükseklik), invalid_height, invalid_revision, revision_conflict, no_materials (bileşenli duvar yok), no_legend (at yok, lejant da yok), at ile katman (layer_not_found, not_a_layer, layer_locked); gizli katmana layer_hidden uyarısı. Plan satırları, bloğu ve yeni yerleştirmeyi verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
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
            summary: "Düz, L ya da U mutfak dizisi yazar; tek geri alma adımı “Mutfak dizisi”. from, through (en çok iki dik köşe) ve to dolapların sırtının dayandığı yoldur (duvarın yüzü), side yolun hangi yanında durdukları (left, right; yok: left); köşeler dolapların yanına döner. \
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
            summary: "Mutfak dizisini yeniden dizer ya da bırakır. run dizinin kimliği ya da bir nesnesinin kalıcı kimliğidir. \
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
            summary: "Çizimin mutfak dizilerini okur; hiçbir şey yazmaz. run verilirse yalnız o dizi (dizinin kimliği ya da bir nesnesinin kalıcı kimliği). \
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
    ]
}
