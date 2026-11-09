# 4040 — Tarifin katmanının grubu ve tam stili; öbür çizimlerin kendi hesabının içinde durması

Tarih: 2026-11-09. plan/map-cad.md MAP-20, yedinci tur. Durum: taslak. ADR 3640'ın “Sonra”sındaki “Kesitler, istinat
duvarı, eş yükselti eğrileri ve kazı-dolgu çizimlerinin kendi hesabının içinde durması ve ilerlemesi” ile “Tarifin
katmanının grubu ve tam stili”.

İlgili:
- [ADR 0313](0313-line-type-patterns.md): proje çizgi türleri;
- [ADR 1940](1940-project-graph.md): İş grafiği ve Tarifi kaydet…;
- [ADR 2440](2440-rebuild-as-a-job-progress-and-stop.md): yeniden üretim bir iş, ilerleme ve durdurma;
- [ADR 3640](3640-recipe-layers-and-stop-within-a-drawing.md): tarifin katmanları adıyla; şevli planda kesitler arasında
  durdurma.

## Dayanak

- **AutoCAD 2020**, [Edit/New Layer Dialog Box (Layer Translator)](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-Core/files/GUID-07E2A06E-793C-4D54-B5B7-F72B17A40437.htm)
  (9 Kasım 2026'da okundu): çeviride yeni tanımlanan katmanın alanları ad, çizgi türü, renk, çizgi kalınlığı, saydamlık
  ve çizim stilidir. Çizim stili yalnız çeviricinin baktığı bütün çizimler adlı çizim stili kullanıyorsa değişir.
- **QGIS API**, [QgsLayerDefinition](https://api.qgis.org/api/classQgsLayerDefinition.html) (9 Kasım 2026'da okundu):
  - QLR dosyası dışa aktarılan katmanların katman ağacı bilgisini de, grup bilgisi dahil, saklar;
  - seçilen katman ağacı düğümleri dışa aktarılır;
  - yüklenirken içerik verilen gruba (`rootGroup`) eklenir.
- **GDAL**, [Algorithms C API](https://gdal.org/en/stable/api/gdal_alg.html) (9 Kasım 2026'da okundu): eş yükselti
  üreticisinin (`GDALContourGenerate`) `pfnProgress`'i ilerlemeyi bildirmek ya da algoritmayı kesmek için kullanılır.
  [CPL](https://gdal.org/en/stable/api/cpl.html)'nin `GDALCreateScaledProgress`'i alt işlemin 0'ını ve 1'ini bütünün
  `dfMin`'ine ve `dfMax`'ına eşler (aynı gün okundu).
- **QGIS'in QgsFeedback'i:** ADR 3640'ın okuması (durdurma düzenli yoklanır, alt adımın ilerlemesi bütünün aralığında).
- **KentOS bugün** (ADR 3640):
  - kendi hesabının içinde duran tek çizim koridorun şevli planıydı. Enkesitler, istinat duvarı, eş yükselti eğrileri,
    kazı-dolgu ve sıfır hattı çizimleri bitene dek durdurmayı duymuyordu; ilerleme çubuğu o sürede kımıldamıyordu;
  - tarif katmanın yalnız adını, rengini, yerleşik çizgi türünü ve kalınlığını taşıyordu. Eklenen katman ağacın en üst
    düzeyine giriyordu; dolgusu, nokta ve etiket stili, proje çizgi türü tarife girmiyordu.

## Karar

1. **Çizimlerin kendi hesabının içinde durması:**
   - **Enkesitler** (`road.sections.draw`): kesit takımının km'leri tek tek (`road_sections::sections_told`);
   - **İstinat duvarı** (`road.walls.draw`): ilk yarı koridorun kesitleri (`make_ticked`), ikinci yarı duvarın kendi
     km'leri (`road_walls::read_told`);
   - **Eş yükselti eğrileri** (`road.terrains.contours`): düzey düzey (`Surface::contours_told`), GDAL'ın eş yükselti
     üreticisi gibi;
   - **Kazı-dolgu bölgeleri** (`road.terrains.cutfill`): ilk yarı karşılaştırma yüzeyinin üçgenleri (`Ground::overlay_told`),
     ikinci yarı bantların birleştirilmesi, bant bant (`volume_zones::cut_fill_told`);
   - **Sıfır hattı** (`road.terrains.zeroline`): karşılaştırmanın üçgenleri (`volume::zero_lines_told`);
   - yazım işin ilerlemesini söyler ve durdurmasını dinler (`ExecutionContext::told`). Doğrulama ve plan yalnız
     durdurmayı dinler (`stop_only`; şevli plan da artık bu ikisini kullanır). Durdurulan çizim `cancelled` döner, hiçbir
     şey yazmaz;
   - yeniden üretimde ve Tariften üret'te her çizimin ilerlemesi işin içinde kendi payındadır (ADR 3640'ın `in_share`'i);
   - hesaplar aynı işlemleri aynı sırayla yapar: sonuçlar değişmez.
2. **Tarif katmanın grubunu ve tam stilini taşır** (`road.project.graph`; ek alanlar, komut sürümü değişmez):
   - `group`: katmanın içinde olduğu grupların adları, ağacın tepesinden. Katman tepedeyse yazılmaz;
   - `style`: katmanın bütün stili (renk, yerleşik ve proje çizgi türü, kalınlık, dolgu, nokta, etiket, etiket sınıfları …).
     Rengi, çizgi türü ve kalınlığı eski alanlarla aynıdır, eski okuyucu onları okur;
   - `lineTypeDef`: stilin adladığı proje çizgi türünün projedeki tanımı (ad büyük küçük harf ayrımsız, projenin kuralı).
     Proje o adda tür tanımlamıyorsa yazılmaz.
3. **Tariften üret eklediği katmanı** (`road.project.produce`):
   - **gruplarına koyar:** her grup, üstündekinin grupları arasında katlanmış adıyla (Katmanları çevir'in katlaması) bulunan
     ilk gruptur. Yoksa üstündekinin sonuna eklenir (QGIS'in verilen gruba eklemesi gibi);
   - **tarifin stiliyle ekler.** Eski tarif: renk, çizgi türü, kalınlık, gerisi öntanımlı;
   - **proje çizgi türü:** stilin adladığı tür bu projede o adla varsa projeninki kullanılır (tanımı başka olsa da; adla
     bulunan katmanın kendi görünüşünü koruması gibi). Yoksa tarifteki tanımla projeye eklenir. Tarifte de tanımı yoksa
     stilden çıkar, katman yerleşik türüyle çizilir ve uyarıda söylenir;
   - **geri alma:** hiçbir çizim ona çizilmezse katman, onun için eklenip boş kalan gruplar ve hiçbir katmanın adlamadığı
     eklenen çizgi türü adımın içinde geri alınır;
   - **cevap:** eklenen katmanın `group`'u (grupların buradaki adlarıyla), `groupsAdded` ve `lineTypeAdded`.
     `recipe_layer_created` uyarısı bunları söyler (“…tarifteki stiliyle “Yol › Toprak işleri” grubuna eklendi. “Toprak
     işleri” grubu da eklendi. “SINIR” çizgi türü de tarifteki tanımıyla eklendi.”).
4. **Retler** (`invalid_project_recipe`, `path`; çizimden önce): tarifin katman tanımında
   - boş grup adı;
   - rengi, çizgi türü ya da kalınlığı tanımınkiyle aynı olmayan stil;
   - stilin adlamadığı ya da proje çizgi türü olarak tutmayan (adının başında boşluk …) çizgi türü tanımı.
5. **Masaüstü** (İş grafiği › Tariften üret…): plan ve sonuç eklenen katmanın grubunu, eklenecek grupları ve çizgi türünü
   söyler. Katman seçicinin açıklaması: “yoksa tarifteki stili ve gruplarıyla eklenir”.
6. **Kapsam işareti:** ADR 3640 gibi **Harita + Yol**.

## Sınırlar

- Bellek ve CPU bütçesi yoktur (ADR 3640'ın sınırı; gerçek veriyle ölçüm ister).
- Kendi hesabının içinde duranlar şevli plan ve bu beş çizimdir. Etiketler, boykesit, korkuluk, menfez, kavşak, platform ve
  boru hattı profili çizimleri bitince durur.
- Yüzeyin üçgenlenmesi ve bir bandın birleştirilmesi (alan cebiri) tek adımdır; durdurma onlardan sonra duyulur.
- Katmanın saydamlığı (Katman çizimi), yazdırılıp yazdırılmadığı, türü (nokta, çizgi, alan, açıklama) ve alanları tarife
  girmez.

## Bulunan

- **Proje üreticileri** (`road_project_cases.py` ve onu kullanan `road_project_graph_cases.py`) ADR 3920'den beri
  yüzeylerin damgalarını boş çizimden hesaplıyordu:
  - arazi üreticisinin `gather`'ı yapıştırma için çizimi `ALL_UIDS`, `ALL_ENTITIES` ve `PAD_LAYERS`'tan okumaya başlamıştı;
  - proje üreticisi ise yalnız `UIDS`, `ENTITIES` ve `LAYERS`'ı kendi çizimine çeviriyordu;
  - yazılmış durum dosyaları doğruydu (üreticiler yeniden koşulmamıştı), `--check` güncel değil diyordu.

  Proje üreticisi artık üçünü de çevirir. query, rebuild ve edit durumları bayt bayt yeniden üretilir.

## Kanıt

- **Çekirdek:**
  - `surface::contours::told_tests`: düzeyler tek bütünün içinde sırayla söylenir; çizgiler düz çağrınınki; dördüncüde
    istenen durdurma `None`;
  - `volume::told_tests`: örtüşme her üçgenden önce söylenir; sıfır hattı dördüncüde durur;
  - `volume_zones::tests::the_overlay_and_the_bands_are_told_and_a_stop_is_heard_between_them`: örtüşme ilk yarıda,
    on bant ikinci yarıda; bölgeler düz çağrınınki.
- **Uygulama birim testleri** (beş çizimin her biri, ortak durum dosyasının ilk durumuyla):
  - ilerleme tek bir bütünün içinde artar, dördüncü yoklamada istenen durdurma orada `cancelled` döner;
  - durdurulmuş işin parçası olarak hiçbir şey çizilmez, belge aynı kalır.
- **Ortak durumlar:** enkesit (31), istinat duvarı (21), eş yükselti (23), kazı-dolgu (44), sıfır hattı (23), şevli plan
  ve proje (query, rebuild, edit) durumları değişmeden Rust'ta ve Python'da geçer.
- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/road_project_graph_cases.py`: her tarif katmanında kurulumun katmanının stili. Yeni durum (22):
    Harita › Toprak işleri grubundaki, SINIR çizgi türünü adlayan katmanın tarifi grupları, stili ve SINIR'ın tanımıyla.
    Eski 21 durum yalnız `style` ile değişti;
  - `scripts/fixtures/road_project_produce_cases.py`: 55 durum (9 yeni), tarifler `fixtures/recipes/v1` altında elle.
    Katman ağacı ve çizgi türleri durumda bütün olarak denetlenir:
    - YOL › Toprak işleri: Yol katlanarak bulunur, Toprak işleri ve SINIR eklenir, ikinci kez adıyla bulunur; planı;
    - iki grup da eklenir, projenin “Sinir”i kullanılır;
    - tarifte tanımı olmayan çizgi türü yerleşik türle;
    - çizilemeyen çizimin katmanı, grupları ve çizgi türü geri alınır;
    - dört ret.
- **Python** (`python/tests/test_recipe_layer_groups.py`), testin kendi okumasıyla:
  - A'nın tarifi grupları, stili ve SINIR'ı taşır;
  - B'de katman Harita › Toprak işleri'ne A'nın stiliyle eklenir, SINIR A'nınki; çizgiler A'nınki, ikinci kez yazılmaz;
  - C'de “HARİTA” grubu ve C'nin “Sinir”i kullanılır;
  - D'de çizimler bırakılır, ağaç ve çizgi türleri aynı kalır.

  ADR 3640'ın testi tarifte stili de bekler.
- **MCP** (`crates/native/mcp/tests/recipe_layer_groups.rs`): eklenen katmanın grupları ve çizgi türü; grafiğin tarifleri
  grupları, stili ve SINIR'ı taşır; tutarsız tarif reddedilir. ADR 3640'ın MCP testi tarifte stili de bekler.
- **Masaüstü** (`project_graph_window_tests::tariften_uret_adds_the_recipe_layer_in_its_groups_with_its_style`):
  - plan Yol › Toprak işleri'ni, eklenecek grubu ve SINIR'ı söyler; Üret'e dek hiçbir şey yazılmaz;
  - katman Yol › Toprak işleri altında tarifin stiliyle, SINIR projenin çizgi türlerinde.

  Pencerenin öbür testleri, Enkesit, Proje ağacı ve yüzey analizi pencereleri ve çapraz denetimler geçer. Resimler
  `.run/gui-evidence/tarif-grubu-4040/`.

## Sonra

- Bellek ve CPU bütçesi, gerçek veriyle ölçümle birlikte.
- Etiket, boykesit, korkuluk, menfez, kavşak, platform ve boru hattı profili çizimlerinin kendi hesabının içinde durması.
- Katmanın saydamlığı, türü ve alanlarının tarife girmesi (saydamlık, karışım kipi, tür ve Yazdır:
  [ADR 4440](4440-recipe-layer-transparency-type-and-print.md); alanlar açık).
- Ölçüm arşivinin zincire bağlanması; atlas sayfalarının denetimi (ADR 3640 Sonra).
- Web (TODOS §0.2).
