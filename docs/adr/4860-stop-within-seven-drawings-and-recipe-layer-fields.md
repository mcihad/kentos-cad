# 4860 — Yedi çizimin kendi hesabının içinde durması; tarifin katmanının alanları

Tarih: 2026-11-10. plan/map-cad.md MAP-20, dokuzuncu tur. Durum: taslak. ADR 4040'ın ve 4440'ın “Sonra”sındaki
“etiket, boykesit, korkuluk, menfez, kavşak, platform ve boru hattı profili çizimlerinin kendi hesabının içinde
durması” ile “katmanın alanlarının tarife girmesi (değer listeleri, bağlı tablo ve alt türlerle birlikte)”. Bellek ve
CPU bütçesi gerçek veriyle ölçüm ister; sahibin kararıyla bekleyen işlerdendir, açık kalır (Sınırlar).

İlgili:
- [ADR 0199](0199-layer-fields-and-feature-table.md), [0422](0422-subtypes.md), [0430](0430-project-value-lists.md): katmanın alanları, alt
  türler, projenin değer listeleri;
- [ADR 2440](2440-rebuild-as-a-job-progress-and-stop.md): yeniden üretim bir iş, ilerleme ve durdurma;
- [ADR 3640](3640-recipe-layers-and-stop-within-a-drawing.md), [4040](4040-recipe-layer-groups-style-and-stop-within-drawings.md),
  [4440](4440-recipe-layer-transparency-type-and-print.md): tarifin katmanı; çizimlerin kendi hesabının içinde durması.

## Dayanak

10 Kasım 2026'da okundu; notlar ve ekran resimleri `.run/research/map-cad/uygulama/MAP-20.9.md` ve `MAP-20.9-resim/`.

- **Netcad, Proje Katmanlarının ve Özniteliklerinin Oluşturulması**
  ([sayfa](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=217384889)): katmanın öznitelikleri veritabanı
  tablosunun kolonlarıdır. **Tablo Düzenle** penceresi: Yeni Kolon, Kolon Sil; Kolon Adı, Veri Türü, Gerekli; Tablo Adı,
  Sınıf Adı, Geometri Türü, Projeksiyon. Veri türleri Metin, Uzun Metin, Tam Sayı, Ondalık Sayı, Mantıksal, Tarih …
- **Netcad, Lookup (Açılır Liste) Tanımlama İşlemleri**
  ([sayfa](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=217387674)): değer listesi ayrı bir veri tablosudur;
  **Lookup Tanımı** penceresinde Tablo, Değer Kolonu, Görüntü Kolonu ve Aktif. Liste projenin veritabanındadır: katman
  başka veritabanına giderken listesi de gerekir. Bunu kendiliğinden yapan bir katman şablonu belgesi bulunamadı.
- **ArcGIS Pro, Copy** ([belge](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/data-management/copy.html)):
  nesne sınıfının alan listeleri (domains), alt türleri ve dizinleri sınıfla birlikte kopyalanır; hedefte adı olan
  bağımlı veri alt çizgi ve sayıyla (“rivers_1”) gelir. Esri bilgi bankası 000038116 (sayfa tarayıcıya kapalı, arama
  özetinden): aynı adlı aynı türlü liste kodları farklı olsa da aynı sayılır. KentOS bunu almaz: sessiz birleştirme yok.
- **ArcGIS Pro, Use a geoprocessing tool**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/help/analysis/geoprocessing/basics/run-geoprocessing-tools.html)):
  çalışan aracın durum şeridinde yüzde ilerleme ve işlemi durduran iptal düğmesi. ArcGIS'in betik araçları iptali
  `arcpy.env.isCancelled` ile kendi seçtiği yerlerde yoklar. Netcad'in uzun hesaplarında durdurmayı anlatan bir sayfa
  bulunamadı.
- **QGIS, QgsMapLayer::StyleCategory** (ADR 4440'ın okuması): Fields (takma adlar, bileşenler, kısıtlar) stilin ayrı
  bir kategorisidir; alanların kendisi veri kaynağındadır.
- **Mevzuat** (`.run/research/map-cad/mevzuat/MAP-20.9.md`): çizimin hesabıyla birlikte tutulması için kural yok; KGM
  2019 şartnamesi çizim dosyalarıyla hesabın program dosyalarını, kübaj ve metrajı birlikte ister. Katmanın alanları
  için BÖHHBÜY Ek-1'in detay-öznitelik tablosu bir profil olabilir; alanları ona benzeyen katman “BÖHHBÜY'e uygun”
  sayılmaz.
- **KentOS bugün:** ADR 4040'tan sonra kendi hesabının içinde duranlar şevli plan ve beş çizimdi. Etiketler, boykesit,
  otokorkuluk, menfez, kavşak, platform ve boru hattı profili bitince duruyordu. Tarif katmanın görünüşünü taşıyordu,
  alanlarını taşımıyordu: Tariften üret'in eklediği katman alansız çıkıyordu.

## Karar

1. **Yedi çizim kendi hesabının içinde durur** (ilerleme işin içinde kendi payında, ADR 3640'ın `in_share`'i; doğrulama ve
   plan yalnız durdurmayı dinler, yazım ilerlemeyi de söyler; durdurulan çizim `cancelled` döner, hiçbir şey yazmaz):
   - **Etiketler** (`road.alignments.label`): etiket etiket;
   - **Boykesit** (`road.profiles.draw`): ilk yarı arazinin yürüyüşü (`kentos_road::profile::terrain_told`: aksın
     elemanları yüzeyin kenarlarıyla 4096'lık bloklarla, sonra noktaların kotları), ikinci yarı bantların sütunları.
     Arazi bir kez yürünür: kot aralığı, bölme ve çizgi aynı noktaları okur;
   - **Otokorkuluk** ve **menfez** (`road.guardrails.draw`, `road.culverts.draw`): koridorun kesitleri, sonra kesitler
     (`road_stakeout::table_told`);
   - **Kavşak** (`road.junctions.draw`): kolların kesişiminin aranması, ilk kolun parçaları teker teker
     (`kentos_road::junction::crossings_told`);
   - **Platform** (`road.platforms.draw`): köşe köşe şev çizgileri, sonra tasarım yüzeyi, sonra hacimler
     (`kentos_road::grading::grade_told`; durdurulunca `GradingError::Stopped`);
   - **Boru hattı profili** (`cad.pipes.profile`): hattın boruları teker teker.

   Hesaplar aynı işlemleri aynı sırayla yapar: sonuçlar değişmez. `ExecutionContext::in_job` bir konağın komutu kendi
   iş parçacığında bir işin ilerlemesi ve durdurmasıyla koşmasını sağlar.
2. **Tarif katmanın alanlarını taşır** (`road.project.graph`, `RoadRecipeLayer`; ek alanlar, komut sürümü değişmez):
   - `fields`: katmanın alanları sırasıyla. **Bağlı tablonun sütunu (`column`) tarife girmez:** bağ projesinindir.
     Projede olmayan listeye bağlı alanın bağı kalkar, kopyası kalır (ADR 0430);
   - `domains`: alanların bağlandığı projenin değer listeleri, projedeki tanımlarıyla, alanların ilk bağladığı sırada;
   - `subtypes`: alt türleri.
3. **Tariften üret eklediği katmana** alanları, listeleri ve alt türleri verir (`road.project.produce`):
   - alanın bağlandığı her liste: bu projede aynı adlı (büyük küçük harf ayrımsız, projenin kuralı) ve aynı değerli
     (tür, değerler ya da aralık, uçları, birim) liste varsa alan ona bağlanır;
   - aynı adlı ama başka değerli liste **değişmez**; tarifinki ArcGIS'in kuralıyla ilk boş “<ad>_1”, “<ad>_2” … adıyla,
     o addan yapılmış kimlikle eklenir. ArcGIS'in kodlar farklı olsa da aynı sayması KentOS'ta yok;
   - o adda liste yoksa tarifinki kendi adıyla, kimliği boşsa kendi kimliğiyle (değilse adından) eklenir;
   - cevapta eklenen katmanın `domainsAdded`'i (buradaki adlarıyla); `recipe_layer_created` uyarısı söyler: “Alanları
     (3) ve alt türleri tarifteki gibi. “Sınır türü” değer listesi de tarifteki tanımıyla eklendi.” ya da “Bu projenin
     “Sınır türü” listesi başka değerler taşıyor, değişmedi; tarifinki “Sınır türü_1” adıyla eklendi.”;
   - **geri alma:** hiçbir çizim gitmediği için geri alınan katmanla eklenen ve artık hiçbir alanın bağlamadığı listeler
     de gider.
4. **Adıyla bulunan ya da verilen katman kendi alanlarını korur.** Tarifin onda olmayan ve başka türlü alanları
   `recipe_layer_fields` uyarısıyla söylenir: “… tarifin “Tür” alanı onda yok; “Derinlik” onda Metin, tarifte Ondalık
   sayı. Alanları Katman özellikleri › Alanlar'da düzenleyin ya da çizimin katmanını layers ile verin.”
5. **Retler** (`invalid_project_recipe`, `path`; çizimden önce): tarifin katman tanımında bağlı tablonun sütununu taşıyan
   alan; tarifte tanımı olmayan listeye bağlı alan; değerleri listesininkiyle aynı olmayan alan; alanlara uymayan alt
   türler; alanların kendi kuralları (aynı adlı iki alan …).
6. **Masaüstü** (İş grafiği › Tariften üret…): plan ve sonuç eklenen katmanın alanlarını ve listesini söyler (“Alanları
   (3) ve alt türleri tarifteki gibi. “Sınır türü” değer listesi de eklenecek.”); adıyla bulunan katmanın eksikleri
   planın uyarısında. Katman seçicinin açıklaması: “… saydamlığı, türü, Yazdır'ı ve alanlarıyla eklenir”.
7. **Kapsam işareti:** ADR 3640 gibi **Harita + Yol**.

## Sınırlar

- **Bellek ve CPU bütçesi yok:** gerçek veriyle ölçüm ister; sahibin, gerçek verinin beklendiği işlerdendir.
- Yüzeyin üçgenlenmesi (platformun ve boykesitin arazisi), bir tasarım yüzeyinin kurulması ve hacimleri tek adımdır;
  durdurma onlardan önce ya da sonra duyulur.
- Katmanın formları, değer bileşimleri, kayıt kuralları ve dizinleri tarife girmez; alanlar, listeler ve alt türler
  girer. Bağlı tablonun sütunu girmez.
- Adıyla bulunan katmanın alanları değiştirilmez, eksikleri eklenmez: yalnız söylenir.
- Aynı adlı liste başka değerliyse tarifinki yeni adla eklenir; iki listeyi birleştirmek kullanıcınındır (Değer
  listeleri penceresi).

## Kanıt

- **Çekirdek** (`kentos-road`): `profiles::the_walk_told_is_the_walk_and_hears_a_stop` (yürüyüş düz çağrınınki, iki
  yarıda artar, dördüncüde durur), `grading::the_grading_told_is_the_grading_and_hears_a_stop` (köşe köşe altı tik),
  `junction::tests::the_crossings_told_are_the_crossings_and_hear_a_stop` (yüz parça). Eski profil ve tesviye testleri
  geçer.
- **Uygulama birim testleri** (`stop_tests`, altı çizim, ortak durum dosyalarının ilk durumuyla; `told_tests`'in iki
  ortak denetimi): ilerleme tek bir bütünün içinde artar, dördüncü yoklamada istenen durdurma `cancelled`; durdurulmuş
  işin parçası olarak hiçbir şey çizilmez. Boru hattı profili işlem araçlarını ister: tümleşik testte
  (`pipes_profile_stop`) durdurulmuş iş hiçbir şey yazmaz, koşan iş ilerlemesini son boruda (6'da 5) söyler.
- **Ortak durumlar:** yedi çizimin ve onları okuyan sorguların (otokorkuluk, menfez, kavşak, platform, aplikasyon,
  paftalar) ve proje komutlarının (rebuild, produce, graph, query, edit) durumları değişmeden geçer.
- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/road_project_graph_cases.py`: 1 yeni durum (24): Sıfır hattı katmanının dört alanı (biri listeye
    bağlı, biri bağlı tablonun sütunlu, biri projede olmayan listeye bağlı), alt türleri ve kullanılmayan bir liste.
    Eski 23 durum bayt bayt aynı;
  - `scripts/fixtures/road_project_produce_cases.py`: 11 yeni durum (75), tarifler `fixtures/recipes/v1` altında elle;
    listenin kimliği adından elle: alanlar, liste ve alt türlerle eklenir, ikinci kez adıyla bulunur; planı; aynı
    değerli listeye bağlanır; başka değerli listenin yanına “Sınır türü_1”; adıyla bulunan katmanın eksikleri; geri
    alınan katmanla liste de gider; beş ret. Eski 64 durum değişmedi. Bozma denemesi: “Sınır türü_1” “_2” yazılınca
    koşucu yakalar (75'te 1).
- **Python** (`python/tests/test_recipe_layer_fields.py`), testin kendi okumasıyla: tarif alanları sütunsuz, listeyi ve
  alt türleri taşır; B'de katman ve liste eklenir; C'nin başka değerli listesi değişmez, yanına “Sınır türü_1”; D'nin
  aynı adlı katmanı kendi alanını korur, eksikler söylenir; sütunlu tarif reddedilir. ADR 4040 ve 4440'ın testleri
  geçer.
- **MCP** (`crates/native/mcp/tests/recipe_layer_fields.rs`): eklenen katmanın alanları ve listesi; grafiğin tarifleri
  onları taşır; sütunlu tarif reddedilir.
- **Masaüstü** (`recipe_layer_fields_tests`): plan alanları ve listeyi söyler, Üret'e dek hiçbir şey yazılmaz; Üret
  katmanı tarifinki gibi ekler; adıyla bulunan katmanın eksikleri planda. Resimler
  `.run/gui-evidence/tarif-katman-alanlari-4860/`.

## Sonra

- Bellek ve CPU bütçesi, gerçek veriyle ölçümle (sahibin beklediği).
- Katmanın formları, değer bileşimleri, kayıt kuralları ve dizinlerinin tarife girmesi; adıyla bulunan katmana eksik
  alanları ekleme seçeneği: [ADR 5240](5240-recipe-layer-rules-forms-and-missing-fields.md).
- Web (TODOS §0.2).
