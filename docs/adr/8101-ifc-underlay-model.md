# ADR 8101: Altlık model — başka disiplinin IFC modeli projede salt okunur, planda katında kesilmiş, 3B'de gövdesiyle

- **Durum:** uygulandı (17 Kasım 2026), kısmen. Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-09, [bim-plan.md](../../plan/bim-plan.md)
  B18 ("Statik model planda ve 3B'de altlık"). İlk kabulün dört adımı sağlandı (§4); kalanlar §6'da.
- **Bağlam:** ADR [8061](8061-ifc-viewer-and-reading-engine.md) (okuma motoru, görüntüleyici, §4'ün "altlık olarak ekle"
  isteği), [8073](8073-ifc-viewer-first-slice.md) (görüntüleyicinin ilk dilimi), [8072](8072-buildings-shared-basement-location.md)
  §3 (projenin yerel sistemi ve IFC'nin harita dönüşümü), [8067](8067-building-3d-view-and-room-overlaps.md) (binanın 3B görünümü),
  [8012](8012-plan-cut-and-overhead.md) (planın kesme yüksekliği), [0207](0207-point-clouds-and-large-data.md) (dış dosyaya
  başvuran büyük veri), kılavuz §5 (sessiz dönüşüm yok).
- **Web:** dokunulmadı (ADR 0228); TS türleri üretildi (`ModelUnderlay`, `UnderlayPlacement`, `UnderlayChange`,
  `UnderlayEdit`, `UnderlayEdited`, `ArchSettings`); pencere, plan örtüsü ve Mimari verisi 18'in okuyucusu TODOS.md §0.2'de.
  **`.kcad`:** belge şeması değişmedi; mimari verisi sürüm 18 (`settings.arch.underlays`). **Komut:** `cad.underlay.edit`
  v1. **Bağımlılık:** yok (okuma motoru ADR 8061'in).

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| IFC 4.3 `IfcMapConversion`, `IfcMapConversionScaled` (buildingSMART; `crates/shared/ifc/src/crs.rs`'in yazdığı) | Modelin yerel koordinatı haritaya: Eastings, Northings, OrthogonalHeight öteler; XAxisAbscissa/Ordinate döndürür; Scale birimi çevirir (yükseklik dahil); ölçekli sürüm eksen başına FactorX, FactorY, FactorZ | Dosyanın yerel metresi kendi dönüşümüyle haritaya, oradan projenin dönüşümünün tersiyle çizime (K157) |
| [BIMcollab: dünya koordinatlı IFC modellerinin hizalanması](https://helpcenter.bimcollab.com/en/articles/326917-coordinating-ifc-models-with-world-coordinate-system-information) | Harita dönüşümü olan ve olmayan modeller birlikte açılınca biri uzağa düşebilir; çözüm tedarikçiden dönüşümü istemek ya da coğrafi konumlandırmayı kapatmak; IFC2x3'ün özellik setindeki konum ve IfcSite'ın enlem-boylamı hizalamada kullanılmaz | Dönüşümü olmayan dosya yerleştirilmez, nedeni söylenir; yerel metreleriyle başlangıç noktasına yerleştirme yalnız açıkça istenince (K158). IfcSite'ın enlem-boylamı kullanılmaz |
| [Revit, IFC ve koordinat sistemleri](https://bim-me-up.com/en/revit-ifc-and-coordinate-systems/) | Bir yazılım bağlanan IFC'yi her zaman başlangıç noktasından başlangıç noktasına koyar: dışa aktarmada seçilen başlangıç belirleyicidir | "Başlangıç noktasına" seçeneği vardır ama varsayılan harita dönüşümüdür: TUREF'teki projede sessiz kayma olmaz |
| [Plan görünüm aralığı (Revit belgesi)](https://help.autodesk.com/cloudhelp/2018/ENU/Revit-DocumentPresent/files/GUID-58711292-AB78-4C8F-BAA1-0855DDB518BF.htm) | Üst, kesme, alt düzlem ve görünüm derinliği; kesme düzlemini kesen öğe kesit çizgisiyle, altındaki öğe izdüşüm çizgisiyle; bağlanan modelin öğeleri de aynı aralıkla | Altlık etkin katın kesme yüksekliğinde (ADR 8012) kesilir; kesme düzlemi ile katın tabanı arasındaki kenarlar izdüşümüyle, daha soluk (K160) |
| [ArchiCAD topluluğu: bağlanan IFC kat kesme düzlemine uymuyor](https://community.graphisoft.com/t5/Collaboration-with-other/Actual-projected-view-of-hotlinked-elements-Floor-cut-plane-does/td-p/705802) | Nesneye çevrilmiş IFC öğeleri planın kesme düzlemine uymuyor | Altlık KentOS nesnesine çevrilmez; kesit gövdenin üçgenlerinden doğrudan alınır, her öğe aynı kuralla kesilir |
| ADR 8061 §5, `read::bodies` | Motor gövdeyi float64 köken etrafında float32 köşelerle verir; KentOS köşeleri float64'e çevirir; gövdesi çıkarılamayan nesne nedeniyle listelenir | Gövdeler aynen alınır; gösterilemeyenler listelenir, sessizce atlanmaz |

## §2. Karar

- **K157 — Yerleştirme** (`kentos_ifc::underlay::place`): dosyanın harita dönüşümünden (hedef sistem, yükseklik datumu,
  köken, x ekseni, eksen başına ölçek) projenin IFC dosyalarının yazıldığı sisteme (`ifc_system::of`): x′ =
  S_p⁻¹·R_p⁻¹·(O_f + R_f·S_f·x − O_p), z′ = s_z·z + (H_f − H_p). Dosyanın metreleri motorun birim çevirisiyle gelir:
  dönüşümün ölçeği birime bölünür; birimin açıklamadığı ölçek (%2'den çok sapma) dönüşümün yanlış yazıldığını söyler.
- **K158 — Ret, sessiz dönüşüm yok:** iki sistem farklıysa, dosyada harita dönüşümü yoksa, projenin sistemi ya da ±0,00 kotu
  yoksa, iki yükseklik datumu adlandırılmış ve farklıysa, ölçek birimle uyuşmuyorsa `model_underlay_not_placed` nedeniyle
  döner; yeniden yansıtma yapılmaz. "Başlangıç noktasına" (dosyanın metreleri çizimin metreleri, sıfırı ±0,00) yalnız
  kullanıcı isteyince; pencere nedeni ve bu seçeneği birlikte gösterir.
- **K159 — Kayıt** (`settings.arch.underlays`, Mimari verisi 18, en çok 50): kimlik, ad (yoksa dosyanın adı), dosyanın yolu,
  SHA-256'sı ve boyu (gösterilen revizyon), şeması, kullanıcının disiplini ve revizyonu, yerleşim, gösterilsin mi. Model
  çizime kopyalanmaz; yabancı özellik setleri ve sınıflandırma dosyada kalır, okunurken olduğu gibi gösterilir. Dosya
  diskte değişirse (SHA-256 başka) gösterilmez, "değişmiş" denir; Yeniden oku yeni revizyonu alır.
- **K160 — Plan** (`kentos_ifc::underlay::plan`): etkin katın tabanı + planın kesme yüksekliğinde her üçgenin kesme
  düzlemiyle kesişimi kesit çizgisidir; kesme düzlemi ile katın tabanı arasında, iki yüzü 20°'den çok kırılan (ya da tek
  yüzlü) kenarlar izdüşüm çizgisidir; dik kenar noktaya iner, çizilmez. Gizli çizgi kaldırılmaz. Çizgiler çizimin altında,
  ızgaranın üstünde, paletin ikincil mürekkebiyle (kesit tam, izdüşüm %45) çizilir; çizime yazılmaz. Bir resimdir, ölçü
  değildir (kılavuz §23.3).
- **K161 — 3B ve seçim:** 3B görünümde altlığın gövdeleri kendi rengiyle (mor, yarı saydam; lejantta "Altlık model");
  tıklanan gövde Altlık modeller'de gösterilir, planın seçimi değişmez. Planda seç, plandaki çizgiye 8 piksel içinde en
  yakın nesneyi bulur; nesnenin ayrıntıları (sınıf, ad, GlobalId, yer, özellik ve miktar setleri) dosyadan okunur.
- **K162 — Pencere** `arch.underlays` "Altlık modeller" (Mimari › Proje ▾, Teslim kaydı'nın altı): solda altlıklar (Ad,
  Disiplin, Şema, Durum), sağda seçilenin dosyası, okuma (gösterilen / şekilli nesne, gösterilemeyen ve nedenleri, kutusu,
  süre ve bellek), seçilen nesne, ad-disiplin-revizyon, göster. Simge: kesik çerçevede kiriş.

## §3. Uygulama

- **Çekirdek:** `crates/shared/ifc/src/underlay.rs` (`place`, `plan`, `MapFrame`, `Transform`); `read::Location::frame`.
- **Sözleşme:** `crates/shared/contracts/src/arch/underlays.rs`; kodlar `invalid_model_underlay`, `model_underlay_not_found`,
  `duplicate_model_underlay`, `model_underlay_unreadable`, `model_underlay_not_placed`, `too_many_model_underlays`.
- **Dosya:** `kentos_kcad::{ARCH_WITH_UNDERLAYS, uses_underlays}`.
- **Komut:** `crates/shared/commands/src/arch/cad_underlay.rs`; uygulama `arch/underlay_edit.rs` (dosyayı okur, yerleştirir);
  alan `Document::{underlays, set_underlays}`, geri alma `ArchOp::Underlays`.
- **Çizici:** `kentos_render_wgpu::scene::build_lines` (renkli çizgi gruplarından parça); görünümün parçaları yedi
  (ızgara, altlık, çizim…), stilli çizimin altında iki parça.
- **Masaüstü:** `apps/desktop/src/arch/underlays.rs` (okuma iş parçacığı, önbellek, plan, seçim), `underlays_window.rs`,
  `shell/scene3d_building.rs` (altlık gövdeleri).

## §4. Sınamalar ve ölçümler

- **Çekirdek** (`underlay::tests`): kutunun kesiti çevresi, tabanı izdüşümü; eğik kenar kesme düzleminde kesilir; projenin
  kendi dönüşümündeki dosya olduğu yere; haritadaki dosya yerel projeye projenin dönüşümüyle; milimetre dosyası Scale
  0,001 ile aynı yere, 1 ile ret; başka sistem, dönüşümsüz dosya, sistemsiz proje ve farklı datum ret, başlangıç noktası her
  zaman. **Sözleşme**, **`.kcad`** (mimari verisi 18 gidip gelir).
- **Komut durumları** (`fixtures/commands/v1/cad.underlay.edit.json`, 32; `scripts/fixtures/underlay_cases.py` dosyaların
  SHA-256'sını, boyunu, şemasını ve sistemini baytlarından okur, KentOS kodu yok).
- **Masaüstü** (`underlays_tests`, ilk kabul): `dated_sizes_tests`'in evi TUREF / TM36'da, ±0,00 1285,40 m, kendi IFC'si
  altlık: her şekilli nesnenin gövdesi var (16), kutu 0–10 × 0–5 × 0–7,60 m (1 mm içinde), Zemin kat'ın kesiti x = 0, 0,25,
  7,95, 8,05, 9,75, 10 üzerinde; tek geri alma adımı. buildingSMART'ın IFC 4.3 mimari örneği (EPSG:32760) bulguyla reddedilir
  (iki sistem adıyla), başlangıç noktasına istenince 14 şekilli nesneden 14'ü gösterilir; IFC2X3 örneği (dönüşümsüz) aynı,
  14/14. Planda seç batı duvarını (IfcWall) bulur; 3B'de tıklanan altlık gövdesi pencerede gösterilir; diskte değişen dosya
  "değişmiş", Yeniden oku yeni revizyonu alır.
- **Süre ve bellek** (bu makinede, M5, hata ayıklama derlemesi): evin modeli 24 KB, 16 gövde, okuma 0,014 s, yaklaşık
  39 KB; IFC 4.3 örneği 139 KB, 0,024 s, yaklaşık 197 KB; IFC2X3 örneği 90 KB, 0,016 s, yaklaşık 148 KB.
- **Resimler:** `.run/gui-evidence/altlik-8101/` (kendi modeli ve seçilen duvar, başka sistem bulgusu, örnek, planda
  kaydırılmış duvar ile altlığın duvarı, 3B, şeritteki yeri; açık ve koyu).

## §5. Arayüz

- Bulgu: "bsi-ifc4x3-building-architecture.ifc: Dosyanın koordinat sistemi EPSG:32760, projeninki EPSG:5256: yeniden yansıtma
  yapılmaz, dosya yerleştirilmedi. Dosyayı projenin sisteminde yeniden dışa aktarın." [Başlangıç noktasına yerleştir] [Vazgeç].
- Okuma: "Gösterilen 16 / 16 nesne (%100) · gösterilemeyen 0", "Kutusu 10,00 × 5,00 × 7,60 m", "0,01 s · yaklaşık 39 KB".

## §6. Sıradaki adımlar

- **Gösterilemeyen nesnenin işareti:** gövdesi çıkarılamayan nesne şimdi yalnız listede; planda ve 3B'de yerleşiminin
  noktasında bir işaret ve (varsa) sınır kutusuyla durması.
- **Bulut projesi:** dosyanın yolu yazarın makinesinindir; bulut projesinde altlığın dosyası projenin varlığı olarak
  yüklenmeli (ADR 0031'in revizyon deposu).
- **Büyük model:** kat değişince plan çizgileri pencerenin iş parçacığında yeniden hesaplanır; büyük modelde arka plana.
- **IFC-10:** GlobalId haritası ve yeni revizyonla fark; **IFC-11:** eski sürümlerin (IFC4, IFC2X3) okunuşundaki kayıp
  listesi (kaldırılan sınıflar, karakter kodlaması, birim ve gövde farkları).
- **Onayla dönüştürme:** altlık öğesini düzenlenebilir KentOS nesnesine çevirmek ayrı ADR'dir (IFC-09'un dışında).
