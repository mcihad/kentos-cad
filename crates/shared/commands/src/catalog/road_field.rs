//! The catalog's commands of road stakeout and sheets (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
        CommandDescriptor {
            id: crate::ROAD_ALIGNMENTS_IMPORT.into(),
            version: crate::ROAD_ALIGNMENTS_IMPORT_VERSION,
            title: "LandXML'den aks al".into(),
            summary: "LandXML dosyasının akslarını KentOS aksı olarak alır: PI'lar elemanlardan bulunur (her aliymanlar arası kurp grubu bir PI: giriş ve çıkış teğetlerinin buluştuğu yer, daire eğrisinin yarıçapı, klotoidlerin boyu; açıyla buluşan iki aliyman kurpsuz PI), PI'lardan kurulan aks dosyanın elemanlarıyla uç uca karşılaştırılır (en çok 1 mm; cevabın deviation'ı). Aksı adıyla anan Roadway'in DesignSpeed'leri (Imperial'da mil/saat, metersPerSecond 3,6 ile km/sa'e çevrilir) en uzun aralığınki aksın proje hızı, dairesel kısmının ortası başka hızlı aralığa düşen kurbun kendi hızı olur; 0'dan büyük, en çok 200 km/sa olmayan landxml_left ile alınmaz. Km denklemleri ve withoutProfiles verilmezse boykesitleri (PVI'lar ve parabol, asimetrik parabol, daire eğrisi; km'ler iç km'den aksın boyuna çevrilir) de gelir. alignments dosyanın akslarını adla seçer (yoksa hepsi). Aralarında aliyman olmayan bileşik ya da ters kurp, ove klotoid, klotoid olmayan geçiş eğrisi, düzensiz çizgi ve zincir alınmaz, landxml_left ile söylenir. Tek geri alma adımı “LandXML aks al”. \
                      Retler: not_a_road_project, file_not_found, import_unreadable, landxml_no_alignment, invalid_alignment, invalid_revision, revision_conflict."
                .into(),
            aliases: vec!["landxmlaksal".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::AlignmentsImport>(),
            output: schema::<crate::AlignmentsImported>(),
            plan: Some(schema::<crate::AlignmentsImported>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Dosyanın ana aksı boykesitleriyle".into(),
                input: json!({ "path": "/veri/yol.xml", "alignments": ["Ana aks"] }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::ROAD_STAKEOUT_QUERY.into(),
            version: crate::ROAD_STAKEOUT_QUERY_VERSION,
            title: "Aplikasyon cetveli".into(),
            summary: "Koridorun kodlu kesit noktalarını km'leriyle verir; hiçbir şey yazmaz. Cevap neyden hesaplandığını sources'ta söyler: koridorun, aksının, boykesitinin, bölgelerinin tip kesitlerinin ve dever çizelgesinin bugünkü sürümleri, arazinin bugünkü kaynak özeti (digest) ve son kabul edildiği özet (built; ikisi farklıysa çizim kabulden sonra değişmiştir) ve koridorun kübaj hesapları (sürüm, onaylı mı); masaüstü cetvelin yanına bunları ve yazıldığı zamanı Kaynaklar sayfasına yazar. Her nokta: km ve km yazısı, bölge, kod (axis, left.c1 … left.cN, ditchToe, ditchBottom, hinge, daylight), yan (left, axis, right), eksenden öteleme (sağa artı, plan boyu), x, y, z. from ve to ile km aralığı (yoksa bölgelerin uçları); step ile her step metrede (step'in katları ve aralığın uçları, her bölgede), yoksa koridorun kendi km'leri; codes ile yalnız o kodlar (tam kod ya da her iki yan için kodun sözcüğü). distances: grid (öntanımlı) ya da ground: öteleme ayrıca zemin boyu olarak (eksenden noktaya çizginin ölçeği ve projenin ortalama elipsoit yüksekliğinin çarpanına bölünür); method bunu söyler. \
                      Uyarılar: koridorun kendi uyarıları (corridor_stale, corridor_unsolved …), stakeout_unsolved. Retler: not_a_road_project, unknown_corridor, invalid_stakeout, stakeout_no_ground."
                .into(),
            aliases: vec!["aplikasyoncetveli".into(), "yolaplikasyon".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::StakeoutQuery>(),
            output: schema::<crate::StakeoutQueried>(),
            plan: Some(schema::<crate::StakeoutQueried>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ana koridorun her 20 metresi, şev ve hendek noktaları zemin boyuyla".into(),
                input: json!({ "corridor": "Ana koridor", "step": 20.0, "codes": ["axis", "hinge", "daylight"], "distances": "ground" }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::ROAD_STAKEOUT_SEND.into(),
            version: crate::ROAD_STAKEOUT_SEND_VERSION,
            title: "Aplikasyonu alete gönder".into(),
            summary: "Aplikasyon cetvelinin noktalarını (road.stakeout.query'nin corridor, from, to, step ve codes'u; yerler plan koordinatı) alete dosya olarak yazar. format: landxml (LandXML 1.2 string yol: koridorun aksı, yalnız kendi boykesiti ve her kodun noktaları km sırasıyla PlanFeature olarak; bir km'de kodu olmayan dize orada kesilir, tek noktalı dize yazılmaz; saha yazılımları okur), gsi16, gsi8, gts7, jobxml, nikon, csv (alet yazıcıları). Noktalar first'ten (yoksa 1) sırayla numaralanır, kodu taşınamayan noktanın kodu düşer (stakeout_code_dropped), noktası taşınamayan nokta yazılmaz (left, export_losses). job JobXML'in iş adı ve Nikon RAW'ın notu (yoksa koridorun adı); stamp tarih ve saat. Bütün yazılır ya da hiç; çizimi değiştirmez. \
                      Sahaya giden kotlar projenin söylediği düşey sisteme bağlıdır: söylenmemişse ret (field_heights_unknown, BÖHHBÜY: TUDKA99'a dayalı Helmert ortometrik yükseklik); LandXML dosyası sistemleri CoordinateSystem'de söyler. Retler: not_a_road_project, unknown_corridor, invalid_stakeout, invalid_file_path, file_exists, invalid_stakeout_send, field_heights_unknown."
                .into(),
            aliases: vec!["aplikasyongonder".into(), "aletegonder".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::StakeoutSend>(),
            output: schema::<crate::StakeoutSent>(),
            plan: Some(schema::<crate::StakeoutSent>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Ana koridor string yol olarak".into(),
                    input: json!({ "corridor": "Ana koridor", "step": 20.0, "format": "landxml", "path": "/veri/ana.xml" }),
                    output: None,
                },
                CommandExample {
                    title: "Şev noktaları Leica GSI-16 olarak".into(),
                    input: json!({ "corridor": "Ana koridor", "step": 20.0, "codes": ["daylight"], "format": "gsi16", "path": "/veri/sev.gsi" }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::ROAD_SHEETS_PLAN.into(),
            version: crate::ROAD_SHEETS_PLAN_VERSION,
            title: "Yol paftaları".into(),
            summary: "Aksı plan–boykesit paftalarına dizer: bir paftanın aralığı şablonun plan haritasının genişliğinin scale ölçeğindeki zemin boyudur, her plan haritası aralığın kirişinin ortasında, aks soldan sağa akacak biçimde döndürülür; overlap kadar bindirilir. profile verilince (aynı aksta) her paftanın km'lerinin boykesit görünümü (road.profiles.draw'ın from ve to'suyla) layerId katmanına çizilir, ilki at'ta, her biri bir öncekinin bantlarının altına; şablonun ikinci haritası onu aynı ölçekte gösterir, bir km planda ve boykesitte üst üste düşer. bands: kgm görünümleri KGM'nin 2019 bantlarıyla çizer, superelevation onların Dever bandının çizelgesidir; KGM bantları kâğıtta 2,5 mm yazıyla 107,5 mm tutar, A3 şablonun boykesit haritasına sığmaz (road_sheet_outside). Şablon yoksa sys:yol-plan-boykesit, bands: kgm ile KGM'nin A1 paftası sys:yol-plan-boykesit-kgm (plan 791 × 230 mm, boykesit 791 × 318 mm, sağ üstte KGM klişesi); paftalar first'ten (yoksa Y-101) numaralanır, Yol › aks altında. Pafta takımı aksın ve boykesitin revizyonlarıyla yol ayarlarında saklanır. Boykesit görünümleri, takım ve paftalar tek geri alma adımı “Yol paftaları” (paftanın kendi düzenlemeleri geri alma adımı değildir). \
                      Uyarılar: road_sheet_outside, layer_hidden, profile_stale. Retler: invalid_revision, revision_conflict, not_a_road_project, unknown_alignment, unknown_road_profile, invalid_road_sheets, katmanın retleri, unknown_template, invalid_options, not_fixed_map, bad_drawing_info."
                .into(),
            aliases: vec!["yolpaftalari".into(), "planboykesit".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::RoadSheetsPlan>(),
            output: schema::<crate::RoadSheetsLaid>(),
            plan: Some(schema::<crate::RoadSheetsLaid>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Ana aks 1:1000, boykesitiyle".into(),
                input: json!({ "alignment": "Ana aks", "profile": "Proje kotu", "layerId": "0", "at": [0.0, -500.0], "scale": 1000 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::ROAD_SHEETS_SECTIONS.into(),
            version: crate::ROAD_SHEETS_SECTIONS_VERSION,
            title: "Enkesit paftaları".into(),
            summary: "Kesit takımının enkesit görünümlerini paftalara dizer: bir takımın görünümleri aynı boydadır (road.sections.draw: şerit genişliği × en yüksek kot aralığı), her pafta şablonun haritasının scale ölçeğindeki zemin boyuna sığan kadar sütun ve satır alır; görünümler layerId katmanına grup grup çizilir, ilk grubun sol üstü at'ta, her biri bir öncekinin bir harita yüksekliği ve aralık altında; her paftanın haritası kendi grubunu gösterir. stations (yoksa takımın hepsi), exaggeration, height road.sections.draw'ınki. Şablon yoksa sys:cad-teknik; paftalar first'ten (yoksa E-101) numaralanır. Takım kesit takımının revizyonuyla yol ayarlarında saklanır; görünümler, takım ve paftalar tek geri alma adımı “Enkesit paftaları”. \
                      Retler: invalid_revision, revision_conflict, not_a_road_project, unknown_section_set, invalid_label, invalid_places, no_terrain, katmanın retleri, invalid_road_sheets, unknown_template, invalid_options, not_fixed_map, bad_drawing_info."
                .into(),
            aliases: vec!["enkesitpaftalari".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::RoadSheetsSections>(),
            output: schema::<crate::RoadSheetsLaid>(),
            plan: Some(schema::<crate::RoadSheetsLaid>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Enkesitler 1:200".into(),
                input: json!({ "set": "Enkesitler", "layerId": "0", "at": [0.0, -1000.0], "scale": 200 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::ROAD_ALIGNMENTS_FIT.into(),
            version: crate::ROAD_ALIGNMENTS_FIT_VERSION,
            title: "Çizgiden aks".into(),
            summary: "Çizimin bir çizgisinden ya da açık çoklu çizgisinden aks kurar (DWG ya da DGN'den gelen yol ekseni); çizgi silinmez. method elements: düz parçalar aliyman, yay parçaları (büküm) daire kurbu; yaysız buluşan iki aliymana radius verilirse o yarıçapta kurp. method bestFit (Create Best Fit Alignment): köşe, komşularıyla geçen çember maxRadius'tan küçükse kurpta, değilse aliymanda; her aliyman koşusundan en küçük karelerle doğru, doğruların kesişimleri PI, kurbun yarıçapı iki doğruya teğet yayın köşelere en yakın olanı; klotoid yok. reverse çizginin sonundan başlatır; name (yoksa ‹n›. aks), start başlangıç km'si. Cevap aksın PI'larını, aliyman ve kurp sayısını, boyunu ve çizginin köşelerinin (ve yayların ortasının) aksa en büyük uzaklığını yeriyle verir; tolerance aşılırsa alignment_fit_deviation. Tek geri alma adımı “Çizgiden aks”. \
                      Retler: invalid_revision, revision_conflict, not_a_road_project, entity_not_found, invalid_alignment_fit."
                .into(),
            aliases: vec!["cizgidenaks".into(), "aksuydur".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::AlignmentsFit>(),
            output: schema::<crate::AlignmentFitted>(),
            plan: Some(schema::<crate::AlignmentFitted>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "DWG'den gelen eksen, en iyi uyumla".into(),
                input: json!({ "uid": "0191f0aa-0000-7000-8000-000000000001", "method": "bestFit", "maxRadius": 1000.0, "tolerance": 0.05 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::ROAD_PROFILES_FIT.into(),
            version: crate::ROAD_PROFILES_FIT_VERSION,
            title: "Çizimden boykesit".into(),
            summary: "Bir boykesit görünümünde çizilmiş çizgiden ya da açık çoklu çizgiden boykesit kurar (DWG ya da DGN'den gelen proje kotu); çizgi silinmez. Görünüm road.profiles.draw'ınki gibidir: at noktası station'da (aksın başından uzaklık, yoksa 0) ve datum kotundadır, bir metre km bir çizim birimi, bir metre kot exaggeration birim (yoksa 1). Düz parçalar eğim, iki eğimin köşesi PVI (length verilirse o boyda simetrik parabol), yay parçası iki ucu arasında düşey kurp: PVI uçlardaki teğetlerin kesişimi, PVI ortadaysa simetrik, değilse asimetrik parabol; eğimin değişmediği köşe PVI değildir; art arda iki yay ret. Sağdan sola çizilmiş çizgi soldan sağa okunur. name (yoksa ‹n›. boykesit), surface siyah kotun yüzeyi. Cevap boykesitin PVI'larını, eğim ve kurp sayısını, km aralığını ve çizginin köşelerinin (ve yayların ortasının) boykesite en büyük düşey uzaklığını km ve kotuyla verir; tolerance aşılırsa profile_fit_deviation. Tek geri alma adımı “Çizimden boykesit”. \
                      Retler: invalid_revision, revision_conflict, not_a_road_project, unknown_alignment, unknown_terrain, entity_not_found, invalid_profile_fit."
                .into(),
            aliases: vec!["cizimdenboykesit".into(), "boykesituydur".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ProfilesFit>(),
            output: schema::<crate::ProfileFitted>(),
            plan: Some(schema::<crate::ProfileFitted>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "1:10 abartmalı görünümde çizilmiş proje kotu".into(),
                input: json!({ "uid": "0191f0aa-0000-7000-8000-000000000001", "alignment": "Ana aks", "at": [0.0, -500.0], "datum": 90.0, "exaggeration": 10.0, "length": 80.0 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::ROAD_ALIGNMENTS_QUERY.into(),
            version: crate::ROAD_ALIGNMENTS_QUERY_VERSION,
            title: "Aksı sorgula".into(),
            summary: "Yol projesinin akslarını bugün kuruldukları gibi verir; hiçbir şey yazmaz. alignment verilmezse her aksın raporu; verilince yalnız onunki ve soruları: stations aksın kendi km'lerindeki (başından yatay uzunluk) yerler; kms km yazısındaki yerler (bir km birden çok yere düşerse region verilmezse station_ambiguous); every her every metrelik yuvarlak km'nin yeri (planın km işaretleri); points her noktanın km ve ötelemesi (sağa artı): en yakın ayak, bütün ayaklar uzaklık sırasıyla, eşit uzaklıkta küçük km önce; ayak yoksa beyond: start ya da end. \
                      Her yer: km, km yazısı (2+345.678), bölge, x, y, doğrultu (rad, doğudan saat yönü tersine), eğrilik (1/m, sola artı) ve eleman. Paralel aksın yerleri kaynağının km'lerindedir. \
                      Retler: not_a_road_project, unknown_alignment, invalid_places, station_outside, station_ambiguous ve aks artık kurulamıyorsa geometrinin retleri."
                .into(),
            aliases: vec!["kmsor".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::AlignmentsQuery>(),
            output: schema::<crate::AlignmentsQueried>(),
            plan: Some(schema::<crate::AlignmentsQueried>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir noktanın km ve ötelemesi".into(),
                    input: json!({ "alignment": "Ana aks", "points": [[1520.0, 2030.0]] }),
                    output: None,
                },
                CommandExample {
                    title: "Her 100 m'nin km işareti".into(),
                    input: json!({ "alignment": "Ana aks", "every": 100.0 }),
                    output: None,
                },
            ],
        },
    ]
}
