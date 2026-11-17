# ADR 8105: Duvar stili IFC'de tip — IfcWallType, katmanlı malzeme takımı ve duvarların tipe bağlanması

- **Durum:** uygulandı (17 Kasım 2026), kısmen. Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-04 ("Tipler, katmanlı malzeme
  ve ilişkiler"), [bim-plan.md](../../plan/bim-plan.md) B21. IFC-04'ün ilk kabulü sağlandı (§4); kolon, döşeme ve çatı
  tipleri BIM-12'nin verisiyle (ADR 8106), katman takımının kullanımı ve çok kata uzanan eleman §6'da.
- **Bağlam:** ADR [8071](8071-geometry-accuracy.md) §8 (çatının katman takımı), [8047](8047-ifc-building-export.md) (IFC
  yazımı), [8068](8068-fire-performance-material-declarations.md) (duvar stilinin yangın sınıfı), duvar stilleri ve bileşenleri
  (`CAD-ARCH-04`, `WallStyleDef`, `WallComponent`).
- **Web:** dokunulmadı (ADR 0228). **`.kcad`:** değişmedi. **Komut:** değişmedi (`cad.arch.ifc`'nin çıktısı aynı).
  **Bağımlılık:** yok.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| IFC 4.3 [`IfcRelDefinesByType`](https://raw.githubusercontent.com/buildingSMART/IFC4.3.x-development/master/docs/schemas/core/IfcKernel/Entities/IfcRelDefinesByType.md) | Bir tip birçok örneğe bağlanır (1'e N); özellik setleri tipte ve örnekte olabilir; aynı adlı özellikte örneğin değeri tipinkine üstün gelir | Her duvar stili bir tip, stilin bütün duvarları ona bağlı; duvarın kendi setleri (yangın sınıfı, dış duvar) örnekte kalır (K179) |
| IFC 4.3 [`IfcMaterialLayerSetUsage`](https://raw.githubusercontent.com/buildingSMART/IFC4.3.x-development/master/docs/schemas/resource/IfcMaterialResource/Entities/IfcMaterialLayerSetUsage.md) | Katman takımı tipe bağlanabilir; kullanım (LayerSetDirection duvarda AXIS2, yani nesnenin y ekseni; DirectionSense; eksenden ofset) her zaman örneğe bağlanır; takımın toplam kalınlığı duvar kalınlığına eşit olmalı | Takım tipe ve duvara bağlanır; kalınlığı başka olan duvarın kendi takımı olur (K180). Kullanım yazılmaz: KentOS duvarları kendi eksen çerçevesinde değil kat kökünde, gövdeleri dünya koordinatında yazar; AXIS2 eğik duvarda duvara dik olmazdı (K181) |
| IFC 4 ve 4.3, `IfcTypeObject` ile `IfcRelDeclares` | Tip nesneleri projenin bağlamında bildirilir | Tipler tek bir `IfcRelDeclares` ile projede (K179) |

## §2. Karar

- **K179 — Tip:** çizimde kullanılan her duvar stili bir `IfcWallType` (`STANDARD`, adı stilin adı, GlobalId'si stilin
  kimliğinden ve bina dosyasında binanın kimliğinden kurallı); stilin duvarları `IfcRelDefinesByType` ile ona bağlı; tipler
  projede `IfcRelDeclares` ile bildirilir.
- **K180 — Katmanlar:** her duvar kendi bileşenlerini `IfcMaterialLayerSet` olarak taşır: sol yüzünden sağa, her katman
  bileşenin adı, malzemesi (modelin malzeme adıyla: Ad_Dayanım) ve duvarın genişliğindeki kalınlığı. Tipin takımı stilin
  bileşenleri stilin kendi genişliğinde. Adı ve katmanları aynı olan takım bir kez yazılır, paylaşılır: stilin genişliğindeki
  duvarın malzemesi tipinkiyle aynı varlıktır. Takımı olan duvarın tek malzemesi yazılmaz (çatıdaki gibi, ADR 8071 §8).
  Stilsiz bileşenli duvar tipsizdir, takımı "Duvar katmanları" adıyla.
- **K181 — Kullanım yok:** `IfcMaterialLayerSetUsage` duvarlar kendi eksen çerçevelerinde yazılınca gelir (§6).

## §3. Uygulama

- `crates/shared/ifc/src/building.rs`: `TypeObject`, `Building::types`; aynı takımın paylaşılması (içerikle önbellek),
  takım başına tek `IfcRelAssociatesMaterial`; tiplerin, `IfcRelDefinesByType`'ın ve `IfcRelDeclares`'in yazımı.
- `crates/shared/ifc/src/arch.rs`: duvarların takımları ve stillerin tipleri.
- `crates/native/application/tests/all/roof_sections.rs`: çatı testi artık çatının kendi takımını sayar (duvarların
  takımları da yazıldığından toplam sayı değişti; çatının beş katmanlı tek takımı aynı).

## §4. Sınamalar

- **İlk kabul** (`crates/native/application/tests/all/ifc_types.rs`): beyanlı kesit binası; iki stil (Dış duvar 25, İç duvar
  15) iki `IfcWallType`, 8'er duvar; dosya her seferinde aynı (`fixtures/arch/v1/ifc/kesit-binasi-tipli.ifc`), stiller
  çizimdeki hâliyle yanında (`kesit-binasi-tipli.json`).
- **Bağımsız okuyucu** (`scripts/fixtures/ifc_types_check.py`, IfcOpenShell 0.9.0): her stil adıyla bir `IfcWallType`;
  stilin duvarları (GlobalId'leri kalıcı kimliklerinin sıkıştırılmışı) yalnız ona bağlı; tipin malzemesi katman takımı ve
  katmanları stilin bileşenleri sırasıyla (İç sıva 0,02 · Tuğla 0,15 · Isı yalıtımı 0,05 · Dış sıva 0,03; Sıva 0,02 · Tuğla
  0,11 · Sıva 0,02); duvarın kendi malzemesi aynı takım; tipler tek `IfcRelDeclares`'te: "uyuyor".
- **Gerilemeler:** IFC kasası 94 test; dışa aktarımın, zorunlu alanların, çatının, altlığın ve revizyonların testleri;
  `cad.arch.ifc` ortak durumları; yayının gerçek kuralla bütün yayımlanması ve gelen paketin incelemesi geçti. Dosyadan
  okunan duvar malzemesi artık katman takımının adıdır (stilin adı; çatıda olduğu gibi); EK-6'nın zorunlu alan satırı
  çizimin beyanından (çekirdek katmanın malzemesi) gelir, değişmedi.

## §5. Arayüz

Değişmedi. Tip ve katmanlar yazılan dosyadadır; bağımsız okuyucunun okuduğu §4'te.

## §6. Sıradaki adımlar

- **Kolon, döşeme, kiriş, çatı tipleri:** BIM-12'nin verisiyle (ADR 8106).
- **Katman takımının kullanımı:** duvarlar kendi eksen çerçevelerinde yazılınca `IfcMaterialLayerSetUsage` (AXIS2, yön,
  eksenden ofset, toplam kalınlık = duvar kalınlığı).
- **Çok kata uzanan elemanın başvuruları** (IFC-04'ün üçüncü maddesi).
- **Okunan malzeme:** dosyanın okuyucusu (`read::material`) katman takımında takımın adını verir; görüntüleyicinin ayrıntısı
  ve gelen paketin incelemesi için katmanların malzemeleri de (çekirdek katman önde) verilebilir.
