# ADR 8137: Duvar stilinin taşıyıcılığı

- **Durum:** uygulandı (18 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13 (Kalan: "duvar stilinin
  taşıyıcılığı"), sıra B15; ADR [8090](8090-declared-load-bearing-and-escape-roof.md) §6 ("stil işleminin 'alan yoksa
  sil' anlamını 'alan yoksa kalır'a çevirmek bir sözleşme kararıdır").
- **Bağlam:** ADR [8068](8068-fire-performance-material-declarations.md) (duvar stilinin yangın sınıfı, duvarlar
  taşır), [8090](8090-declared-load-bearing-and-escape-roof.md) (duvarın taşıyıcılık beyanı, Ek-3/B 2, IFC
  `Pset_WallCommon.LoadBearing`), [8105](8105-wall-style-types-and-layer-sets.md) (duvar stilleri `IfcWallType`),
  [8124](8124-b7-remaining-declarations-and-ifc-fields.md) (çizimin okuması ve beyan).
- **Dayanak:** BYKHY Ek-3/B 2; Dijital Proje Yönetmeliği EK-6 Tablo 6.133 (`Pset_WallCommon.LoadBearing`, isteğe bağlı).
- **Sözleşme:** `WallStyleDef.load_bearing` (`DeclaredBearing`); `cad.arch.performance` v1'in `wallStyle` işlemine
  `loadBearing` (K338'in uyumlu anlamıyla); `load_bearing_carried`. **`.kcad`:** belge şeması değişmedi; mimari verisi 26
  (`ARCH_WITH_STYLE_BEARING`, `settings.wallStyles[].loadBearing`). **Web:** TS türleri yeniden üretildi; Duvar
  stilleri'nin alanı TODOS.md §0.2'de.

## §1. Sorun

Taşıyıcılık yalnız duvarın kendi beyanıydı (8090, K116): aynı stilde onlarca taşıyıcı duvarı olan projede her duvar ayrı
beyan ister. Stilin yangın sınıfı gibi taşıyıcılığı da stilde söylenebilmeli. 8090 bunu `wallStyle` işleminin anlamı
yüzünden ertelemişti: o işlemde `fireRating` verilmezse stilin sınıfı silinir; taşıyıcılık eklenip yalnız o verilince
sınıf da silinirdi.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| IFC 4.3 `IfcWallType`, `IfcRelDefinesByType` ([buildingSMART](https://ifc43-docs.standards.buildingsmart.org/IFC/RELEASE/IFC4x3/HTML/lexical/IfcRelDefinesByType.htm)) | Tipin özellik kümeleri bütün örneklerine geçer; aynı adlı özellik hem tipte hem örnekte varsa örneğinki geçerlidir | Stilin rolü, kendi beyanı olmayan duvarın rolüdür; duvarın beyanı stilinkini geçer (K337) |
| EK-6 Tablo 6.133 | `Pset_WallCommon.LoadBearing` (Taşıyıcı), isteğe bağlı | Duvara yazılır (kendi ya da stilinden), sınıfı (`FireRating`) gibi; beyansız duvara yazılmaz (K340) |
| BYKHY Ek-3/B 2 | Yük taşıyıcı duvar R, Ek-3/C kadar | `dayanim` değişmedi: olgular duvarın taşıdığı rolü verir (K339) |
| KentOS'ta komut sürümü | Hiçbir komutun sürüm 2'si yok; sürüm atlatmanın altyapısı yok | Sürüm 1 korunur, anlam uyumlu genişler (K338) |

## §3. Karar

- **K337 — Stilin rolü:** `WallStyleDef.load_bearing` (Evet, Hayır, dayanağı); kendi beyanı olmayan duvar taşır.
  Duvarın kendi beyanı stilinkini geçer (IFC'nin tip ve örnek kuralı). Kolonun ve öbür elemanların rolü değişmedi.
- **K338 — `wallStyle` işlemi, uyumlu genişleme:** `loadBearing` verilmezse işlem eskisi gibidir: `fireRating` verilirse
  yazılır, yoksa ya da null ise stilin sınıfı kalkar. `loadBearing` verilirse verilmeyen `fireRating` kalır, null onu
  kaldırır; `loadBearing` null stilin rolünü kaldırır. Önce sınıf, sonra rolün dayanağı okunur
  (`invalid_declared`, `operation.loadBearing.basis`). Çıktı: verilen sınıfı ya da rolü taşıyan duvarlar (kendi
  değeri olmayanlar, kilitli katmandakiler de), çizimin sırasıyla. Eski çağrının anlamı değişmez; sürüm 1 kalır.
- **K339 — Taşınan rol:** `load_bearing_carried` (duvarın kendi beyanı, yoksa stilininki) olgulara
  (`elements[].loadBearing`), Ek-3'ün okumasına ve IFC'ye gider.
- **K340 — IFC:** stilin rolü kendi beyanı olmayan duvarın `Pset_WallCommon.LoadBearing`'i olarak yazılır, sınıfı
  gibi örnekte; `IfcWallType`'ın kendi kümeleri yok (8105), tipe yazılmaz.
- **K341 — Arayüz:** Duvar stilleri'nde "Taşıyıcı" (Evet, Hayır, Bilinmiyor; "Kendi rolü olmayan duvarlar taşır");
  aynı cevap belgeli dayanağı korur, başkası müellif beyanıdır. Öznitelikler'in Taşıyıcı satırı beyansız duvarda
  "Stilinden (Evet)" der, notu "Duvar stilinden: taşıyıcı."; kendi beyanı varsa "Beyan duvar stilinin rolünü geçer." ya
  da "Beyan edildi; duvar stili de öyle.".

## §4. Uygulama

- `crates/shared/contracts/src/arch/arch.rs` (`WallStyleDef.load_bearing`, `problem`, KentOS'un stilleri);
  `crates/shared/commands/src/arch/cad_arch_performance.rs` (`WallStyle`), `catalog/arch_project.rs`.
- `crates/native/application/src/arch/arch_performance.rs` (`load_bearing_carried`, `wallStyle` işlemi).
- `crates/shared/kcad/src/lib.rs`, `decode.rs` (mimari verisi 26).
- `crates/native/processing/src/audit_facts/building.rs` (olgular); `crates/shared/ifc/src/arch.rs` (IFC).
- `apps/desktop/src/arch/wall_styles.rs` (`Event::LoadBearing`, alan); `apps/desktop/src/cad/properties/rows/arch.rs`.

## §5. Sınamalar

- **Ortak durumlar** (`cad.arch.performance`, 125; 9'u yeni): stil taşıyıcı, sınıfı kalır, kendi rolü olmayan duvarları
  (kilitlisi de) taşır; taşıyıcı değil belgeyle, sınıfla birlikte; rolü kaldırılır, sınıf kalır; taşıyıcılıkla null
  sınıf kalkar; adıyla; kendi rolü olan duvar taşımaz; adsız belge; önce sınıf okunur; olmayan stil. Beklenenler
  `scripts/fixtures/arch_performance_cases.py`'nin ADR'den yazılmış kurallarından; eski `wallStyle` durumları değişmedi.
- **`.kcad`:** `a_wall_style_s_load_bearing_role_round_trips_under_arch_data_26`.
- **IFC** (`arch_ifc`): taşıyıcı stilin kendi beyanı olmayan duvarı `LoadBearing` true, Hayır beyanlı duvarı false.
- **Masaüstü** (`performance_tests`): kesit binasında Duvar stilleri'nde Taşıyıcı Evet, kaydet: stil müellif beyanıyla
  taşıyıcı; duvarın Öznitelikler'i "Stilinden (Evet)" ve notu, olgular taşıyıcı; duvarın kendi Hayır'ı geçer; stilin
  rolü kaldırılınca bilinmiyor; 8068'in stil sınıfı sınaması değişmedi. Ortak süzgeçlerle (modül, şerit, geçmiş,
  katalog, ayarlar, Öznitelikler, beyanlar, mevzuat, malzemeler, dayanım, şaftlar) 265 sınama yeşil.
- **Resimler:** `.run/gui-evidence/stil-tasiyici-8137/` (`performance_tests::style_bearing_screens`): Duvar stilleri ve
  duvarın Öznitelikler'i; açık ve koyu.

## §6. Kalanlar

- Tipin özellik kümeleri (`IfcWallType.HasPropertySets`): stilin sınıfı ve rolü tipte de yazılabilir; bugün örnekte.
- Komut sürümü 2: `wallStyle`'ın sınıf için "verilmezse kaldırır" anlamı ancak sürüm atlatmayla kalkar.
- Döşeme ve kolon tiplerinin (8106) taşıyıcılığı.
