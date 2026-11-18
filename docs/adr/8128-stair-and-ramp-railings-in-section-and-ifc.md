# ADR 8128: Merdiven ve rampa korkuluğu kesitte, görünüşte, 3B'de ve IFC'de

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) B9 (BIM-09'un kalanı: "rampa ve
  merdiven korkuluğunun kesit gövdesi"), [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-03 (IfcRamp satırının kalanı
  "korkulukları").
- **Bağlam:** ADR [0705](0705-ramps-by-slope-and-railings.md) §2 (merdivenin ve rampanın korkuluk yanları; planda
  bandın kenarından 5 cm içeride çizgi), [8045](8045-balanced-stairs-and-special-elements.md) §3 (çizilen korkuluk:
  1,10 m, 5 cm küpeşte, 0,10 m aralıklı 2 cm çubuk; kesitte küpeşte katı, çubuklar çizgi), [8042](8042-section-derived-from-a-section-mark.md)
  (merdivenin kesit gövdesi), [8070](8070-missing-building-elements.md) §1 (rampanın gövdesi).
- **Dosya, komut, `.kcad`:** değişmedi. Merdivenin ve rampanın verisinde korkuluğun yalnız yanları var (sol, sağ, iki
  yan); yükseklik, çubuk aralığı ve küpeşte çizilen korkuluğun varsayılanlarıdır. IFC dosyasına `IfcRailing` parçaları
  ve korkuluk çubuklarının gövdeleri eklendi.

## §1. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| PAİY Md. 38/1 (ADR 8045'te alıntı) | Merdivenlerde ve rampalarda "döşeme kotundan itibaren en az 1.10 metre" korkuluk | Yükseklik 1,10 m (`RAILING_HEIGHT`) |
| PAİY Md. 38/4 | Boşluklarda en fazla 0,10 m çaplı geçiş | Çubuklar arası net 0,10 m (`RAILING_GAP`), çubuk 2 cm (ADR 8045) |
| International Building Code 1015.3 (New York City Building Code, 2025 California Building Code, WAC 51-50-1015 benimsemeleri) | Korkuluk yüksekliği düşey ölçülür: yürünen yüzeyden; merdivende basamak burunlarının ön kenarlarını birleştiren çizgiden; rampada korkuluğun yanındaki rampa yüzeyinden | PAİY'nin "döşeme kotu" merdiven kolunda basamak burunlarının çizgisi, rampada rampanın yüzeyi, sahanlıkta sahanlığın döşemesi okunur (K294) |
| buildingSMART IFC 4 ADD2 TC1 ve IFC 4.3, `IfcStair` / `IfcRamp`; IFC 4.3 Element Decomposition kullanım tablosu | Merdiven kolları ve sahanlıklarla `IfcRelAggregates` ile; "merdivene ait `IfcRailing` de bu birleşime katılabilir"; IFC 4.3 tablosunda `IfcStair` ve `IfcRamp` bütününün parçaları arasında `IfcRailing` | Merdivenin ve rampanın her yanının korkuluğu onun parçası olan bir `IfcRailing` (K296) |
| buildingSMART forumu, "IfcRailing proper ownership" | Korkuluğu belirli bir kola bağlamanın yolu tartışmalı | Korkuluk kola değil merdivenin ya da rampanın bütününe bağlanır |
| IFC 4.3 `IfcRailingTypeEnum` | BALUSTRADE: bir dizi çubuğun üstünde küpeşte | Çizilen dikey çubuklu korkuluk gibi BALUSTRADE |

## §2. Karar

- **K294 — Korkuluğun ölçüldüğü düzlem:** merdivenin ve rampanın her yanının korkuluk çizgisi planınkidir (bandın
  kenarından 5 cm içeride, dönüşlerde gönye: ADR 0705 §2). Çizgi, yürüme çizgisinin parçalarının üstünde parçalara
  bölünür; her parça altındaki yürüme düzlemini taşır:
  - Merdivende bir rıht aralığıyla ardışık basamakların üstünde basamak burunlarının çizgisi (her rıhtın üstü kendi
    istasyonunda, rıhtın kaldırdığı yükseklikte); bir kolun bütün burunları bir düzlemdir.
  - Sahanlıkta ve sahanlıktan önceki basamakta sahanlığın yüksekliğinde düz; ilk rıhttan önce döşeme, son rıhttan
    sonra son basamak.
  - Rampada kolunun yüzeyi, sahanlığında sahanlığın döşemesi.
  Bir kolun burun çizgisi sahanlığın kenarında sahanlığa ulaşır; sahanlıktan sonraki kolun çizgisi bir rıht yukarıdan
  başlar (orada korkuluk bir rıht basamaklanır). Balansmanlı merdivenin dönen basamaklarında ve eğri merdivende yan
  boyunca burunlar bir düzlem değildir: bu adımda korkulukları yoktur.
- **K295 — Kesitteki gövde:** her parçanın küpeştesi 5 cm genişliğinde ve 5 cm derinliğinde bir katıdır, üstü yürüme
  düzleminin 1,10 m üstünde (kolda ve rampada onunla eğik); çubukları çizgi: her yanın çizgisi boyunca 0,12 m'de bir
  (0,10 + 0,02), ilki yarım adım içeride, ayağı altındaki basamakta (merdiven), yüzeyde (rampa) ya da sahanlıkta,
  küpeştenin altına kadar. Çizilen dikey çubuklu korkuluğun katmanında (MM-KORK-CUBUK), ince ve taramasız. Korkuluk
  merdivenin ya da rampanın gövdesi değildir (kesit modelinde `Part::Rail`, yanıyla).
- **K296 — IFC:** her yanın korkuluğu bir `IfcRailing` BALUSTRADE; merdivenin ya da rampanın `IfcRelAggregates`'ında
  parçası, kendi katına ayrıca konmaz; GlobalId'si nesnenin kimliğinden ve yanından türer (kalıcı); 3B'de tıklanınca
  merdiven ya da rampa seçilir. Merdivenin ve rampanın gövdesi değişmez.
- **K297 — Çubukların gövdesi:** dikey çubuklu korkuluğun (çizilen korkuluğun da) IFC gövdesi küpeştesi ve çubuklarıdır:
  her çubuk 2 cm'lik kare bir çubuk, ayağından küpeştenin altına. Kesit onları çizgi olarak çizmeye devam eder (ADR
  8045: önündekini gizlemezler). Camlı korkuluğun camı ve dolu korkuluk bu adımda değişmedi.
- Çubuğun 2 cm'si `kentos_contracts::BALUSTER_WIDTH`'e taşındı; kesit modeli ve IFC aynı değeri okur.

## §3. Uygulama

- `crates/shared/geometry-core/src/geom/stair_rails.rs` (yeni): `Plane`, `Stretch`, `RailRun`, `rail_runs` (yanın çizgisi
  parçalara), `stair_rails` (düz merdivenin parçaları); `stair_body.rs`'te `winding_zones` ortak.
- `crates/native/application/src/arch/sections.rs`: `push_rails`; merdiven `stair_rails`'ı, rampa kollarını ve
  sahanlıklarını `Stretch` olarak verir; `Part::Rail(yan)`.
- `crates/shared/ifc/src/arch.rs`: `Part::Rail`, yan yan `IfcRailing` parçaları, `balusters`;
  `crates/native/application/src/arch/arch_ifc.rs` eşleme.
- `crates/shared/contracts/src/arch/railing.rs`: `BALUSTER_WIDTH`.

## §4. Sınamalar

- **Çekirdek** (`stair_rails::tests`, elle hesaplanmış): 18 rıhtlı düz kolun iki yanı tek parça, her burunda
  0,17·(i + 1); 40 çubuk, ayakları basamaklarında (0,17, 0,34, 2,89). U merdivende ortak sahanlık 2,24–4,64: ilk kolun
  çizgisi kenarda 1,53'e ulaşır, sahanlık düz 1,53, son kol 1,70'ten 3,06'ya; iç yan sahanlığı 5, 10, 5 cm'lik
  parçalarla geçer; çubuk sayıları 39 ve 76. Eğri merdivende ve korkuluksuz merdivende parça yok.
- **Kesit ve model** (`sections::a_ramp_s_railings_stand_over_its_surface_and_are_its_parts`): L rampada 8 küpeşte (4'ü
  sahanlıkta düz, 63/136 + 1,10), eğik olanların üstü yüzeyin 1,10 m üstünde; 72 + 95 çubuk yüzeyden 1,05 m; y = 2
  kesiti ikinci kolun iki küpeştesini 39/68 + 1,10'da keser; IFC'de rampanın üç gövdesi aynı, iki `IfcRailing`
  parçası (4 parça + çubukları). `a_stair_s_railings_follow_its_nosings_and_are_its_parts`: düz merdivende küpeşte
  her rıhtın üstünden 1,10 m (1,27 → 4,16), 40'ar çubuk, IFC'de iki parça. Çizilen korkuluk sınaması: dikey çubuklu
  korkuluğun gövdesi küpeşte ve 20 çubuk; camlı ve dolu bir gövde.
- **Değişmeyenler:** bayt bayt karşılaştırılan IFC fikstürleri (`ifc_types`, `element_types`), olgu başvuruları, BIM
  paketi; hiçbir fikstürde korkuluk yok, fark çıkmadı.
- **Resimler:** `.run/gui-evidence/korkuluk-8128/` (kesit binasının merdiveni iki yanlı korkuluklu ve doğusunda
  korkuluklu rampa: plan, A-A, B-B, kuzeydoğudan 3B, yalnız zemin kat 3B; açık ve koyu).

## §5. Kalanlar

- Balansmanlı merdivenin dönen basamaklarında ve eğri merdivende korkuluk (yan boyunca burunların eğrisi).
- Camlı korkuluğun camı IFC gövdesinde; dikme, tutamak (küpeştenin altında ayrı tutunma) ayrıntısı.
- Merdivenin ve rampanın korkuluğunun kendi yüksekliği ve aralığı (verisinde yalnız yanları var).
