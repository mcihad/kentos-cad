# ADR 8102: Altlık modeli denetle — katlar, akslar, aynı yerde duran ve çakışan elemanlar; başka datum geometriden önce durdurur

- **Durum:** uygulandı (17 Kasım 2026), kısmen. Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-17, B19 ("Dış disiplin ve
  çakışma"); [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-10'un hizalama maddesi. BIM-17'nin ilk kabulü sağlandı (§4); bulgunun
  açık sorun biçimiyle (BCF) dışa verilmesi IFC-10'un işidir (§6).
- **Bağlam:** ADR [8101](8101-ifc-underlay-model.md) (altlık model, yerleştirme K157–K158), [8061](8061-ifc-viewer-and-reading-engine.md)
  §5 (gövdeler float64 köken etrafında float32 köşeler), [8076](8076-ifc-mandatory-fields-complete.md) (bölüm binasının IFC'si,
  `fixtures/arch/v1/ifc/kesit-binasi-tam.ifc`), [8070](8070-missing-building-elements.md) §28 (kolonun yüksekliği),
  [8040](8040-vertical-model.md) §1 (duvarın yüksekliği), [8047](8047-ifc-building-export.md) (EK-5 adları), kılavuz §5 (sessiz
  dönüşüm ve tahmin yok), §23.3 (gizli tolerans yok).
- **Web:** dokunulmadı (ADR 0228); TS türleri üretildi (`UnderlayCheck`, `UnderlayChecked`, `UnderlayLevelCheck`,
  `UnderlayAxisCheck`, `UnderlayClash`, `UnderlayUnmatched`); pencerenin Denetim bölümü TODOS.md §0.2'de. **`.kcad`:**
  değişmedi. **Komut:** `cad.underlay.check` v1 (okur, yazmaz). **Bağımlılık:** yok.

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| [Designing Buildings: Clash](https://www.designingbuildings.co.uk/wiki/Clash), [Clash detection in 3D BIM models](https://www.designingbuildings.co.uk/wiki/Clash_Detection_in_3D_BIM_Models) | Sert çakışma: iki öğe aynı yeri tutar. Yumuşak (boşluk) çakışma: öğenin çevresinde istenen açıklık ihlal edilir. İş akışı çakışması: zamanlama. Federasyon modelinde disiplinler arası denetlenir | Bu dilimde sert çakışma (K163); boşluk çakışması sonraki iş (§6) |
| [Spatial: clash detection](https://www.spatial.com/glossary/clash-detection), [what is clash detection](https://www.spatial.com/glossary/what-is-clash-detection) | Önce sınır kutularıyla aday çiftler, sonra gövdelerin kesişimi; değen yüzler ve tolerans altındaki örtüşme gürültüdür | Kutu ön süzgeç, kararı gövde verir (K163); tolerans açık ve sonuçta yazılı |
| [IfcOpenShell PR 4282](https://github.com/IfcOpenShell/IfcOpenShell/pull/4282) (IfcClash), [eş düzlemli yüzler düzeltmesi](https://dev.glassvision.dedyn.io/Glassvision/IfcOpenShell/src/branch/fix-4594-clash-coplanar), [IFC Nodes: clash detection](https://basselharby.github.io/IFCNodes/nodes/ifc-clash-detection) | Kesişim, çarpışma ve boşluk denetimi; "değmeye izin ver" ve tolerans: toleranstan az giren gövde çakışma değildir; eş düzlemli yüzler yanlış bulgu üretir | Değen yüzler çakışma değildir; yüzeylerin kesişmesi yalnız iki yanına kesin geçen üçgenlerle sayılır, eş düzlem sayılmaz; yüzleri üst üste ya da biri ötekinin içindeyse ortak kutudaki tanık noktalar karar verir (K163) |
| [HOOPS çakışma yöneticisi](https://www.npmjs.com/package/ts3d-hc-clashmanager), [katları kesme](https://docs.techsoft3d.com/hnp/latest/tutorials/bim_viewer/sectioning_floors.html) | Çakışma çiftleri iki nesneyle listelenir; seçilen çift iki nesneyi gösterir; kat kat kesit | Çakışma satırı çizimdeki nesneyi seçer, dosyadakini gösterir (§5) |
| [Dalux: modelde ötelenme](https://support.dalux.com/hc/en-us/articles/17453731132956-Offset-in-model-visualization) | Disiplin modelleri farklı başlangıçla gelince birbirinden kayar; önce hizalama | Yerleştirme geometriden önce: başka sistem ya da datum geometriye bakılmadan durdurur (K164) |
| IFC 4.3 [`IfcBuildingStorey`](https://raw.githubusercontent.com/buildingSMART/IFC4.3.x-development/master/docs/schemas/product/IfcProductExtension/Entities/IfcBuildingStorey.md); [buildingSMART forumu: katın kotu](https://forums.buildingsmart.org/t/definition-of-the-level-for-the-ifcbuildingstorey-opinion-poll/404?page=2) | `Elevation`: katın tabanının binanın ±0,00 iç referansına göre kotu; isteğe bağlı. Bitmiş döşeme mi taşıyıcı üst kotu mu olduğu disiplinden disipline değişir | Kotlar eşlenir ve fark değeriyle yazılır; gizli eşik yok; kotu yazılmamış kat kotsuz listelenir, tahmin edilmez (K165) |
| IFC 4.3 [`IfcGrid`](https://raw.githubusercontent.com/buildingSMART/IFC4.3.x-development/master/docs/schemas/shared/IfcProductExtension/Entities/IfcGrid.md), [forum: IfcGridPlacement](https://forums.buildingsmart.org/t/ifcgridplacement-issues/2908) | U, V, W aks listeleri; her `IfcGridAxis` etiketi (`AxisTag`) ve ızgaranın yerleşimindeki eğrisi (`AxisCurve`) | Akslar etiketle eşlenir; dosyanın aksı ızgaranın yerleşimiyle dünyaya, oradan çizime (K166) |

## §2. Karar

- **K163 — Elemanlar** (`kentos_ifc::clash`): dosyanın bir elemanı ile çizimin modelinin (IFC dışa aktarımının kurduğu
  gövdeler) aynı sınıftan bir elemanı, kutuları her kenarda toleransın içindeyse aynı elemandır (eşleşir; dosyanınkiler kendi
  sırasıyla, çizimin ilk boştaki elemanını kalıcı kimlik sırasıyla alır; her eleman bir kez). Dosyanın eşleşmeyen
  elemanları çizimin bütün elemanlarıyla denetlenir: kutular her eksende toleranstan çok örtüşmüyorsa çakışma değildir;
  örtüşüyorsa yüzeyleri kesişen (iki yanına kesin geçen üçgenler, ortak aralığı 1e-9 m'den uzun) ya da ortak kutunun 27
  tanık noktasından biri ikisinin de içinde olan çift çakışmadır. "İçinde": yüzeyler kesişmiyor ve biri ötekinin kutusunda.
  Dosyanın sınıflarından eşleşmeyen ve çakışmayan elemanlar iki yanda da "yalnız birinde"dir.
- **K164 — Önce yerleşim:** dosya çizimin andığı revizyondur (SHA-256) ve ADR 8101'in K157–K158 kuralıyla yerleştirilir;
  başka sistem, dönüşümsüz dosya, sistemsiz proje ya da iki adlandırılmış ve farklı yükseklik datumu geometriye bakılmadan
  `model_underlay_not_placed` ile durdurur. Harita dönüşümüyle yerleşen dosyada proje ya da dosya yükseklik datumunu
  söylemiyorsa kotlar aynı datumda sayılır ve `underlay_datum_unsaid` uyarısı bunu, dayandığı varsayımla söyler: sessiz değil.
- **K165 — Katlar:** projenin katları ile dosyanın kotlu katları, her çiftin farkının mutlak değeri küçükten büyüğe (eşitte
  projenin, sonra dosyanın sırası) açgözlü eşlenir. Sıra: çiftler projenin kat sırasıyla, projenin eşsiz katları, dosyanın
  eşsiz katları, kotu yazılmamış katları. Fark dosyanınki eksi projeninki; eşik yoktur, pencere toleranstan büyük farkı
  "farklı" der.
- **K166 — Akslar:** etiketle (kırpılmış) eşlenir, etikete göre sıralanır; ikisinde de olan aksın dosyadaki orta noktasının
  projenin aks doğrusuna uzaklığı ve iki doğru arasındaki dönme (0–90°). Uzunluksuz aksın ölçüsü yazılmaz.
- **K167 — Tolerans:** metre, 0,0001 ile 0,5 arası, verilmezse 0,001 (çizimin milimetresi). Alt sınır gövdelerin float32
  köşelerinin kayabileceği miktarın üstündedir. Tolerans çıktıya yazılır.
- **K168 — Sıralar ve okuma:** çakışmalar dosyanın sırasıyla, her dosya nesnesininkiler çizimin kalıcı kimliğine göre;
  "yalnız birinde" önce çizimin (kimliğe göre), sonra dosyanın (sırasıyla). Komut çizime yazmaz, geri alma adımı yoktur;
  sonucu çizimin revizyonuyla döner.

## §3. Uygulama

- **Çekirdek** (`crates/shared/ifc`): `clash.rs` (`Body`, `Clash`, `bounds`, `clashes`), `read/coordination.rs`
  (`Model::storeys`, `Model::grid_axes`: ızgaranın yerleşimi motorun `resolve_object_placement`'ıyla, birim ölçeğiyle),
  `underlay.rs` (`pair_levels`, `axis_offset`).
- **Komut:** `crates/shared/commands/src/arch/cad_underlay.rs` (`UnderlayCheck`, `UnderlayChecked`…, `MIN_`/`MAX_UNDERLAY_TOLERANCE`);
  uygulama `crates/native/application/src/arch/underlay_check.rs` (dosyayı okur, yerleştirir, çizimin modelini
  `arch_ifc::model` ile kurar); kodlar `model_underlay_changed` (ret), `underlay_datum_unsaid` (uyarı).
- **Masaüstü:** `apps/desktop/src/arch/underlay_check_view.rs` (Denetim bölümü, iş parçacığı, tablolar, çakışma seçimi);
  `underlays_window.rs` (olaylar, iz denetimleri Denetle, Tolerans, satır).
- **Python:** `kentos.cad.underlay` (`cad.underlay.check`); MCP katalogdan.

## §4. Sınamalar ve ölçümler

- **İlk kabul** (`crates/native/application/tests/all/underlay_check.rs`): bölüm binası (ADR 8076'nın beyanlı çizimi; IFC'si
  `kesit-binasi-tam.ifc`, TUREF / TM33, ±0,00 1285,40 m Antalya height) ile IfcOpenShell 0.9.0'ın yazdığı bağımsız statik
  model (`scripts/fixtures/structural_underlay.py`: mimarinin kolon, kiriş ve döşemelerinin kutuları, milimetre, Scale
  0,001, kendi katları ve aksları):
  - hizalı: iki kat farksız, yedi aks (A–D, 1–3) uzaklıksız, 27 eleman (16 kolon, 8 kiriş, 3 döşeme) eşleşir, çakışma ve
    eşsiz yok;
  - kaydırılmış (zemin katın A-1 kolonu 40 cm doğuya, "1. Kat" 3150 mm): 26 eşleşme, "1. kat"–"1. Kat" farkı +0,150 m,
    kaydırılan kolon yalnız kendi çakışmalarında (bir duvarla dahil), mimarinin A-1 kolonu "yalnız birinde";
  - datumu ITRF96 olan aynı model eklenirken `model_underlay_not_placed` ("ITRF96" ve "Antalya" adlarıyla), çizime bir şey
    eklenmez.
- **Ortak durumlar** (`fixtures/commands/v1/cad.underlay.check.json`, 29; `scripts/fixtures/underlay_check_cases.py`):
  dosyaları IfcOpenShell yazar (`fixtures/ifc/v1/statik/denetim-*.ifc`), beklenenleri betik ADR'nin kurallarından ve
  kutulardan hesaplar, KentOS kodu yok. Hizalı model ve önizleme; kaydırılan kolon, yükseltilen kat, kayan ve dönen aks,
  duvarı delen kiriş, duvarın içindeki eleman, 2 cm gömülen döşeme; tolerans 0,05 / 0,25 / 0,5 / 0,0001; elemansız model ve
  kotsuz kat; kendi yüksekliği 2,50 m olan kolonun içinde kalması; gizli altlık; ikinci altlık; başlangıç noktasında başka
  sistem; datumsuz projede uyarı; datum, sistem, sistemsiz proje, değişen, olmayan ve IFC olmayan dosya, kimlik ve tolerans
  retleri sırasıyla.
- **Çekirdek** (`clash::tests`, `underlay::tests`): duvara giren kolon çakışır, yüzüne değen çakışmaz; duvarın üstündeki
  kiriş değer, 2 cm gömülen çakışır, 5 cm toleransta düşer; boru içinde kalır; L köşesi çakışmaz; büyük koordinatlar.
- **Masaüstü** (`underlay_check_tests`): hizalı modelde 2 eşleşme, 0 çakışma, 1 "yalnız birinde"; kaydırılmış modelde beş
  çakışma ve tablo Çakışmalar'da açılır; ilk satır çizimin kolonunu seçer, dosyanın kolonunu Seçilen nesne'de gösterir;
  sayı olmayan tolerans ve ITRF96 dosyası söylenir. Kesişen masaüstü süzgeçleri: 199 geçti.
- **Resimler:** `.run/gui-evidence/altlik-denetim-8102/` (hizalı, çakışmalar, katlar, akslar, yalnız birinde, plan; açık ve
  koyu).

## §5. Arayüz

- Mimari › Proje ▾ › Altlık modeller › seçilen altlık › **Denetim**: Tolerans [0.001] m [Denetle]; özet satırı "1 eleman iki
  modelde aynı yerde · 5 çakışma · 1 eleman yalnız bir modelde · 1 kat ve 2 aks farklı"; datum uyarısı; sekmeler Katlar |
  Akslar | Çakışmalar | Yalnız birinde, bir seferde bir tablo (en çok sekiz satır, sonra kayar). Çakışma varsa tablo
  Çakışmalar'da açılır. Katlar ve Akslar'da "Durum" sütunu: aynı, farklı, eşi yok, kotu yazılmamış.
- Yeni şerit düğmesi yoktur: denetim altlığın penceresinin içindedir.

## §6. Sıradaki adımlar

- **IFC-10:** iki revizyonun GlobalId ile farkı (taşınan, eklenen, silinen, özelliği değişen) ve çakışmanın BCF ile dışa ve
  içe verilmesi; bağımsız bir görüntüleyicide aynı nesne.
- **Boşluk (yumuşak) çakışması:** istenen açıklık (ör. tesisat çevresi) ile denetim.
- **Projenin ortak alanları ve açıklık–şaft:** döşeme boşluğu ile şaft, duvar açıklığı ile kiriş; perde (IfcWall
  SHEAR) ile mimari duvar ayrı kural.
- **Büyük model:** binlerce eleman çiftinde ızgara ya da ağaç ön süzgeci; bu dilimde her dosya elemanı her çizim elemanının
  kutusuyla karşılaştırılır (bölüm binasında ölçülmedi, küçük).
- **Bulgunun kaynağı:** disiplin ve revizyonla kayıtlı sorun (BIM-15'in teslim kaydına bağlanması).
