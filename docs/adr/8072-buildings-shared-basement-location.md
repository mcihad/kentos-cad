# ADR 8072: Çok bina, ortak bodrum, konum ve kot

- **Durum:** kısmen uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-11,
  [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-05; uygulama sırasında B11. Bu ADR maddenin bütün adımlarını tutar; koddaki
  "docs/adr/8072 §N" atıfları bölümleri gösterir: §1 binalar (kod, kullanım, ±0.00, ortak kat). Kararların numarası
  bölümler boyunca sürer. Ortak sınamalar ve sıradaki adımlar numarasız son bölümlerdedir.
- **Bağlam:** ADR [0773](0773-building-program-units-and-design-brief.md) (bina programı), [0784](0784-building-wing-window-and-common-area-shares.md) §1
  (binaların ve kanatların katları), [8032](8032-site-plan-parcel-setbacks-and-datum.md) (vaziyet planı, saha röperi),
  [8047](8047-ifc-building-export.md) (binanın IFC'si), [8069](8069-room-zone-far-model.md) (emsal bölgeleri).
- **Dayanak:** Dijital Proje Yönetmeliği (RG 5.8.2026/33331) EK-2 §2.3 ve Tablo 2.1 (bina kodu, dosya adı); EK-6 Tablo
  6.7 (`IfcBuilding`), 6.55 (`IfcSite`); EK-7'nin ortak öznitelikleri. Metinler `mevzuat/bim/ek-2-…`, `ek-6-1-…`,
  `ek-6-2-…`.
- **Web:** dokunulmadı (ADR 0228).

## §1. Binalar: kod, kullanım, ±0.00, ortak kat

**Araştırma.**

- **EK-2 §2.3:** "'Bina kodu' aynı vaziyet planı içerisinde birden fazla yapının (bina/blok) yer alması ve projelerin
  bina/bloklar için ayrı hazırlanması durumda kullanılan bina kodunu temsil eder. İki karakterli alfanümerik kısaltma ile
  ifade edilir. Projenin tek bir binadan oluştuğu durumda '00' kısaltması kullanılır." **Tablo 2.1:** dosya adı Proje
  Kodu (6) _ Bina Kodu (2) _ Disiplin (2) _ Öge (4) _ Dosya Türü (2) _ Dosya Tipi (3) _ Kat/Konum (3) _ Sıra No (2) _
  Revizyon No (3).
- **EK-6 Tablo 6.7 (`IfcBuilding`, zorunlu):** Uzun İsim, İsim; `Pset_BuildingCommon` BuildingID, IsPermanentID,
  ConstructionMethod, FireProtectionClass, OccupancyType ("Kullanım Türü"), GrossPlannedArea, NetPlannedArea,
  NumberOfStoreys; adres; `Qto_BuildingBaseQuantities` Height, EavesHeight, FootPrintArea. **Tablo 6.55 (`IfcSite`):**
  RefElevation "Deniz seviyesine göre referans (datum) kotudur"; FloorAreaRatio, SiteCoverageRatio, TotalArea.
- **IFC 4.3** (buildingSMART IFC4.3.x-development, `IfcSite.md`): "reference height of site is provided by
  IfcSite.RefElevation … the reference height of each building situated at the site is given against the same height
  datum … the elevations of each storey belonging to each building are given as local height relative to the reference
  height of the building"; `IfcBuildingStorey.Elevation` binanın 0,00'ına göredir (4.3'te `Pset_BuildingStoreyCommon`
  ElevationOfSSLRelative ve ElevationOfFFLRelative önerilir); `IfcBuilding.ElevationOfRefHeight` deniz seviyesine göre
  binanın 0,00'ıdır (4.3'te kullanımdan kalkıyor).
- **ADR 0784 §1:** programın binaları ve kanatları katlarını kaplar; bir kat başka binanın da olabilir (ortak bodrum),
  ama hangi binanın sayılacağı yazılmıyordu.
- **Üreticiler (karşılaştırma için):** Archicad bir projede tek `IfcBuilding` tutar; bir sahayı paylaşan ayrı
  projelerde aynı "Project ID" ve "Site ID", farklı "Building ID" verilmesini önerir (help.graphisoft.com, "IFC Data
  Types"). KentOS'ta binalar tek projede durur; dışa aktarma bina başına model yazacaktır (§2).

**Karar.**

1. **Binanın yeri programdadır** (`ProgramBuilding`): `code` (EK-2 §2.3 bina kodu: iki büyük harf ya da rakam, her
   binada ayrı; "00" yalnız tek binalı projede), `usage` (kullanım türü, EK-6'nın OccupancyType'ı), `zero` (±0.00'ı,
   projenin ±0.00'ına göre metre; en çok ±100 m). Binanın mutlak ±0.00'ı saha röperi (ADR 8032) artı bu farktır; tek
   binada fark verilmezse projeninkidir.
2. **Ortak kat** (`BuildingProgram::shared`, `SharedLevel { level, building }`): birden çok binanın kapladığı kat
   (ortak bodrum) seçilen binanın katı sayılır, alanı bir kez. Seçim kapsayan binalardan biri olmalı, her kat bir kez;
   tek binanın katı için seçim reddedilir. Seçilmeyen ortak kat kimsenin değildir (`unshared_levels`).
3. **Her nesnenin binası katından okunur** (`BuildingProgram::building_of_level`): katı kaplayan tek bina, birden
   çoksa ortak katın seçtiği; hiçbiri ya da seçilmemişse yok.
4. `.kcad` mimari verisi 9 (`ARCH_WITH_BUILDING_PLACES`); `cad.program.edit` yeni alanları programla birlikte yazar ve
   doğrular. Bina programı › Binalar: her binanın Kodu (büyük harfe çevrilir), Kullanım türü ve ±0.00'ı; "Ortak katlar"
   altında her ortak katın Sayıldığı bina; seçilmeyen için "Seçilmedi" notu.

**Sınama.** `program::tests::a_shared_basement_counts_in_the_building_chosen` (seçimle ve seçimsiz ortak kat,
kodun biçimi, tekrarı, "00", seçimin katı ve binası, ±0.00'ın sınırı); `.kcad`'ın
`building_places_round_trip_under_arch_data_9`'u; ortak durumlar (`cad.program.edit` 40: iki blok ve tek binanın 00'ı
kabul, sekiz ret); masaüstünde `two_blocks_get_codes_zeros_and_a_shared_level_through_the_window` (pencereden yazılır,
"00" reddedilir, hiçbir şey yazılmaz). Resim: `bina-ortak-kat` (`.run/shots/arac-bina-ortak-kat-*`).

## Ortak sınamalar

- `cargo test -p kentos-contracts --lib program`, `-p kentos-kcad --test all -- arch_elements`,
  `KENTOS_CASES_ONLY=cad.program … every_command_case`, `-p kentos-desktop --lib -- program_scenes`.

## Sıradaki adımlar

- **§2 Bina başına model:** `cad.arch.ifc`'nin `building`'i: o binanın katları (sayıldığı ortak kat dahil) ve nesneleri;
  proje ve saha bilgileri her dosyada aynı; dosya adında bina kodu (EK-2 Tablo 2.1); `IfcBuilding`'in adı, BuildingID'si,
  OccupancyType'ı, NumberOfStoreys'i; katların kotu binanın ±0.00'ına göre, saha RefElevation'ı röper.
- **§3 Konum (IFC-05):** yerel modelin harita dönüşümü kontrol noktalarından (dönüş ve ölçek); bağımsız okuyucuda üç
  kontrol noktası; kotu bilinmeyen ±0.00 sıfır sayılmaz, eksik olarak söylenir; yerleşim zincirinde döngü ve çift öteleme
  denetimi.
- **§4 Alanlar:** ortak bodrumun alanı emsalde ve metrajda bir kez; ruhsat takımında bina seçimi.
- **§5 Onaylı plan kararları:** CBS'den kaynağı, onay tarihi ve plan notuyla; çelişen iki karar gösterilir.
- Denetim bulguları: seçilmeyen ortak kat, birden çok binada kodsuz ya da ±0.00'ı verilmeyen bina.
