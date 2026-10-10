# ADR 0215: Yakınlık analizi

- **Durum:** kabul edildi (2026-10-10). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki
  platform (masaüstü ve web); çizim verisi değişmez biçimde (öznitelik ve yeni katman), `.kcad` şeması değişmez; ikonlar sorulmadan
  seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-19`; ADR 0200 (Konuma göre seç, Bilgi al: ilişkiler, `relate_pairs`, tablo çıktısı), ADR 0201
  (geometri işlemleri: sonuç yeni katmana, örtüşmeler), ADR 0202 (topoloji toleransı), ADR 0214 (ifade dilinin `en_yakın`'ı ve `uzaklık`'ı).
- **Araştırma:** ArcGIS Pro'nun Near (NEAR_FID, NEAR_DIST, NEAR_X, NEAR_Y, NEAR_ANGLE; arama yarıçapı), Generate Near Table (en yakın
  `k`, hepsi, sıra NEAR_RANK), Polygon Neighbors (iki yönlü satırlar, AREA_OVERLAP, LENGTH, NODE_COUNT; kenar ve köşe komşuluğu); QGIS'in
  Join attributes by nearest (`k`, en çok uzaklık, önek, `distance`, `feature_x`/`nearest_x`), Distance matrix (doğrusal, standart N × T,
  özet), Distance to nearest hub (çizgi ya da nokta çıktısı; merkezden), Shortest line between features (`k`, en çok uzaklık); Netcad'in
  Çevreleyenden Bilgi Al'ı (tamponla).

## Bağlam

Konuma göre seç ve Bilgi al (ADR 0200) bir nesnenin ötekine göre durumunu sorar: kesişiyor mu, içinde mi, uzaklıkta mı. Yakınlık
analizi uzaklığın kendisini sorar: en yakın durak hangisi ve ne kadar uzakta, her parselden okullara uzaklıklar, her mahalleyi en
yakın merkeze bağlayan çizgiler, komşu parseller ve ortak sınırlarının uzunluğu, iki katman arası en kısa bağlantılar. İfade dilinin
`en_yakın` ve `uzaklık`'ı (ADR 0214) tek değer verir; burada tablolar, k komşu, öznitelik takımları ve çizgiler gerekir.

## Karar

### 1. Kapsam

İşlemler'e iki platformda yeni kategori **Yakınlık** (`proximity`) ve beş araç:

| Araç | Kimlik | Ne yapar | Çıktı |
|---|---|---|---|
| En yakını bul | `proximity.nearest` | Her nesneye en yakın hedefin uzaklığını, istenen alanlarını ve semtini yazar (ArcGIS Near, QGIS Join attributes by nearest) | girdinin öznitelikleri, tek adım |
| Uzaklık matrisi | `proximity.matrix` | Her nesneden hedeflere uzaklıklar: en yakın `k` ya da hepsi; liste, matris ya da özet (Generate Near Table, Distance matrix) | tablo (Panoya kopyala, CSV) |
| En yakın merkeze bağla | `proximity.hub` | Her nesneyi merkezden merkeze en yakın “merkez”e bir çizgiyle bağlar (Distance to nearest hub) | yeni katmanda çizgiler |
| Komşu alanlar | `proximity.neighbors` | Alanların komşularını, ortak kenar uzunluğunu ve örtüşmesini verir (Polygon Neighbors) | tablo; isteğe bağlı komşu sayısı ve adları öznitelik |
| En kısa çizgi | `proximity.shortestLine` | Her nesneden en yakın `k` hedefe en kısa doğru parçaları (Shortest line between features) | yeni katmanda çizgiler |

- Uzaklıklar düzlemdedir (projenin koordinatlarında, metre); elipsoit ve zemin değerleri kapsam dışı (Mesafe ölç'ünkiler, ADR 0171).
- **Kapsam dışı:** ağ üzerinden uzaklık (Ağ analizi'nin En yakın tesis ve Maliyet matrisi'nde, ADR 0209), raster uzaklık yüzeyi
  (ADR 0236), üç boyutlu uzaklık, “en uzak” ve dairesel arama yarıçapının dışında kalanı bulma, Thiessen alanları (`GIS-23`).

### 2. Tanımlar

#### 2.1 Uzaklık

- **Kenardan kenara** (varsayılan): iki nesnenin en kısa uzaklığı, Konuma göre seç'in `distance`'ı (ADR 0200): değen, kesişen ya da
  biri öbürünün içinde kalan nesneler için 0; yaylar ve daireler kesin; alanın içi alandır (alanın içindeki nokta 0).
- **Merkezden merkeze:** iki nesnenin `$merkez`'leri (ADR 0100 §3: alanın ağırlık merkezi, öbür nesnede yer noktası) arası uzaklık.
- **En yakın noktalar:** kenardan kenara uzaklığı veren iki nokta, biri her nesnede; uzaklık 0 ise ikisi aynı buluşma noktasıdır.
- Bir nesne kendisinin komşusu ya da hedefi değildir (girdi ve hedef aynı katmansa). Yardımcı çizgi ve ışın katılmaz.
- **Sıra:** hedefler uzaklığa göre, eşitlikte hedef listesindeki (çizimdeki) sırayla. **En çok uzaklık** verilirse ondan uzak
  olanlar alınmaz (sınır dahil); boş ya da 0 sınırsızdır.

#### 2.2 Komşuluk

- İki alanın **ortak kenarı**: sınırlarının birbirinin üstünde kalan parçalarının uzunluğu, `tolerans` içinde. Düz iki kenar, birinin
  iki ucu öbürünün doğrusuna toleranstan yakınsa aynı doğrudadır; ortak uzunluk iki kenarın o doğru üzerindeki izdüşümlerinin örtüştüğü
  uzunluktur. İki yay aynı daireden (merkezleri ve yarıçapları tolerans içinde) ise ortak uzunluk açı aralıklarının örtüşmesi × yarıçap.
  Bütün kenar çiftlerinin toplamıdır; delikler ve çok parçalı alanın parçaları dahil.
- **Kenar komşusu:** ortak kenar toleranstan uzun. **Köşe komşusu** (isteğe bağlı): sınırlar tolerans içinde buluşuyor ama ortak kenar
  toleranstan kısa. **Örtüşme:** içleri örtüşen iki alan; örtüşen alan (m²) örtüşme işlemlerinindir (ADR 0201) ve tabloda ayrı sütun.
- **İçlerin örtüşmesi:** birinin bir köşesi öbürünün içinde ve sınırından 1 mm'den uzak; ya da sınırları uçlarının dışında kesişiyor
  (iki kenarın da parametresi 10⁻⁹ ile 1 − 10⁻⁹ arasında); ya da birinin bir kenarının ortasından 1 cm içerideki nokta öbürünün içinde
  ve sınırından 1 mm'den uzak (aynı sınırı paylaşan özdeş alanlar, delikler). Kutular her sınamayı önce daraltır: kutuları yalnız değen
  alanların içleri örtüşmez; öbürünün kutusunun dışındaki köşe ve nokta onun içinde olamaz; kutuları buluşmayan iki kenar kesişemez.
- **Tolerans:** varsayılan 0,001 m; pencerede 0 ile 1 m arası değiştirilebilir.
- Satırlar iki yönlüdür (A–B ve B–A), girdinin sırasıyla, her nesnenin komşuları kendi sıralarıyla. Alanı olmayan nesne (açık
  eğri, yay biçimli elips) katılmaz.

#### 2.3 Adlar ve yazılan değerler

- Tablolarda ve çizgilerde bir nesnenin **adı**: seçilen **Ad alanı**'nın değeri; boşsa nesnenin etiketi; o da yoksa girdideki sırası
  (`1`'den).
- Uzaklıklar ve uzunluklar projenin uzunluk basamağıyla, alanlar alan basamağıyla gösterim kuralıyla (ADR 0149) yazılır; yazılan
  nesnenin katmanında aynı adlı tipli bir alan varsa onun kuralıyla (ADR 0199). Semt projenin açı biriminde (grad ya da derece),
  kuzeyden saat yönünde, dört basamakla; uzaklık 0 ise boş.

### 3. Araçlar

#### 3.1 En yakını bul (`proximity.nearest`)

Girdi (yazılır), Hedefler, Ölçü (Kenardan kenara, Merkezden merkeze), En çok uzaklık, Alınacak alanlar (hedefin alanları, çoklu),
Önek (varsayılan “Yakın ”), Semt de yaz. Her girdiye `<önek>uzaklık`, her alınan alan için `<önek><alan>` ve istenirse `<önek>semt`
yazılır (semt kenardan kenara ölçüde en yakın noktalar arasında, merkezden merkeze ölçüde merkezler arasında). En çok uzaklık içinde
hedefi olmayan girdinin bu alanları boşaltılır ve sayısı söylenir. Tek geri alma adımı; kilitli katmandaki girdi alınmaz.

#### 3.2 Uzaklık matrisi (`proximity.matrix`)

Girdi, Hedefler, Ölçü, En yakın `k` (0: hepsi), En çok uzaklık, Ad alanı (girdi), Hedef ad alanı, Biçim:

- **Liste:** satır başına bir çift: Kaynak, Hedef, Sıra, Uzaklık.
- **Matris:** satırlar kaynaklar, sütunlar hedefler (en çok 200 hedef; fazlası ölçülmeden reddedilir: “Matris en çok 200 hedef alır;
  N hedef var. Liste biçimini seçin.”); `k` ve en çok uzaklığın dışındaki hücreler boş.
- **Özet:** kaynak başına: Hedef sayısı, En az, Ortalama, En çok; ortalama uzaklıkların sırayla (en yakından) toplamının sayısına
  bölümüdür, iki platformda aynı işlemlerle (en çok `k` toplanan, gösterim basamağına yuvarlanan değerde kesin toplam gerekmez).

Tablo en çok 1 000 000 satırdır: Liste, `k` 0 ve en çok uzaklık verilmemişken satır sayısı (kaynak × hedef) bunu aşarsa ölçmeden,
öbür hâllerde ölçtükten sonra reddedilir (“Tablo N satır olurdu (en çok 1000000); En yakın k ya da En çok uzaklık verin.”). Çizim
değişmez; tablo pencerede Panoya kopyala ve CSV olarak kaydet ile.

#### 3.3 En yakın merkeze bağla (`proximity.hub`)

Girdi, Merkezler, Merkez ad alanı, En çok uzaklık, Çıktı katmanı (yeni ya da var olan). Her girdinin `$merkez`'inden en yakın merkezin
`$merkez`'ine bir çizgi; çizgi girdinin özniteliklerini, `Merkez` (merkezin adı) ve `Uzaklık`'ı taşır. Merkezi olmayan (en çok uzaklık
dışında) girdi çizgisizdir ve sayısı söylenir; merkezi merkezle aynı yerde olan girdiye de çizgi çizilmez ve sayısı söylenir.

#### 3.4 Komşu alanlar (`proximity.neighbors`)

Alanlar (bir ya da birkaç katman), Tolerans, Köşe komşuları da, Örtüşenler de, Ad alanı, Özniteliğe de yaz (Komşu sayısı ve Komşular
alanlarının adlarıyla). Tablo: Alan, Komşu, Komşuluk (Kenar, Köşe, Örtüşme), Ortak kenar (m), Örtüşen alan (m²). Özniteliğe yazılırsa
her alana komşu sayısı ve komşularının adları (“, ” ile, tablodaki sırayla); tek geri alma adımı. Alanlar yazılabilen girdidir: kilitli
katmandakiler alınmaz. Örtüşen çiftlerin sayısı uyarıyla söylenir.

#### 3.5 En kısa çizgi (`proximity.shortestLine`)

Girdi, Hedefler, En yakın `k` (varsayılan 1), En çok uzaklık, Ad alanları, Çıktı katmanı. Her girdiden en yakın `k` hedefe en yakın
noktalar arası doğru parçası; çizgi Kaynak, Hedef, Sıra ve Uzaklık'ı taşır. Uzaklığı 0 olan (değen) çiftler çizgi vermez ve sayısı
söylenir.

### 4. Çekirdek ve depo

- `kentos_geometry_core::ops::proximity`: `nearest_points(a, b)` (en yakın iki nokta ve uzaklık; Konuma göre seç'in `distance`'ıyla aynı
  uzaklık; değenlerde buluşma noktası), `center_distance`, `edge_overlap` ve `shared_boundary(a, b, tolerance)` (ortak kenar uzunluğu,
  buluşma), `interiors_overlap`, `overlap_area` (örtüşme işlemlerinin alanı).
- **Depo** (`store::proximity`): `nearest(inputs, targets, k, max, measure)` girdi başına sıralı kayıtlar, her biri 8 sayı: girdinin ve
  hedefin yeri, uzaklık, en yakın iki nokta, birinciden ikinciye semt (`survey::bearing`; uzaklık 0'da NaN); `neighbors(ids, tolerance,
  corners, overlaps)` çiftler, her biri 5 sayı: yerler, tür (0 kenar, 1 köşe, 2 örtüşme), ortak kenar, örtüşen alan; çiftin cevabı bir kez
  hesaplanır. Hedefler kendi paketlenmiş Hilbert R-ağaçlarına (`PackedTree`, kutu ya da merkez noktası) konur ve en iyi önce aranır
  (`PackedTree::nearest`): kuyruktan önce en yakın olabilecek düğüm çıkar, yaprakta hedef ölçülür ve ölçüsüyle kuyruğa döner, ölçülmüş
  hedef ancak ondan yakın olabilecek bir kutu kalmadığında verilir; kutuların aralığından 2 mm (birbirine 1 mm'den yakın nesneler değer,
  0'dır) düşülür, eşit anahtarda kutu ölçülmüş hedeften, ölçülmüş hedefler hedef sırasıyla önce gelir; böylece sıra uzaklık, eşitlikte
  hedef sırasıdır. En çok uzaklık kuyruğu sınırlar (sınır dahil). Komşular deponun dizininden aranır. İki platform aynı kodu çalıştırır: web
  çalıştırmanın deposunda (WASM, `ObjectStore.nearest`, `.neighbors`), masaüstü `RunGeometry`'de.

### 5. Arayüz

- İşlemler'in araç kutusunda Yakınlık kategorisi (iki platform), araçların tanımdan üretilen pencereleri (ADR 0084); tablolar Özet
  istatistik'inki gibi formun altında (ADR 0200 §5).
- CBS'de Analiz sekmesinin Analiz panelinde, Özet istatistik'in yanında (beş araç; kendi paneli sekmeyi 1100 piksele sığdırmadı, web'in
  yerleşim denetimi taştığını gösterdi); CAD'de İşlemler'den.
- **İkonlar** (sorulmadan): kategori ve En yakını bul `nearestFeature` (noktadan en yakın hedefe çizgi, uzaktaki kesikli), Uzaklık matrisi
  `distanceMatrix` (çapraz başlıklı tablo), En yakın merkeze bağla `nearestHub` (üç kaynaktan ortası noktalı merkeze; ilk çizimi 16 px'te
  “X”e benzediği için yeniden çizildi), Komşu alanlar `polygonNeighbors` (kalın ortak kenarlı iki alan), En kısa çizgi `shortestLine`
  (kareden daireye kesikli en kısa çizgi).

### 6. Performans

Hesap çekirdekte; hedefler kendi ağaçlarında en iyi önce aranır, nesne başına yalnız aday hedefler kesin ölçülür; geometriler nesne başına
bir kez kurulur; komşu çiftinin cevabı bir kez. Ölçü sahnesi iki platformda aynıdır: 100 × 100'lük 20 m'lik ızgarada köşeleri 3 m'ye dek
kaydırılmış (komşular paylaşır, kutular gerçek parsellerdeki gibi örtüşür) 10 000 parsel, her 97.'sinin içinde 5 m'lik bir kare (104,
yalnız parseliyle örtüşür), R2 dizisiyle dağılmış 2 000 durak ve 2 000 yol parçası, 200 m'lik blokların ortalarında 100 merkez.
Bütçeler (release, bu makine): En yakını bul (10 000 parsel → 2 000 durak) ≤ 40 ms; Uzaklık matrisi (10 000 → 2 000, `k` 5) ≤ 60 ms; En
yakın merkeze bağla (10 000 → 100) ≤ 20 ms; Komşu alanlar (10 104 alan, köşeler ve örtüşmeler de) ≤ 100 ms; En kısa çizgi (10 000 → 2 000
yol parçası, `k` 1) ≤ 40 ms; web'de aynı işler, kayıtlar okunarak, ≤ 2,5 katı.

## Uygulama

- **Çekirdek** (`crates/shared/geometry-core`): `ops/proximity.rs` (`Nearest`, `nearest_points`, `center_distance`, `Shared`,
  `edge_overlap`, `shared_boundary`, `interiors_overlap`, `overlap_area`; kenar kutularının ön süzgeci `geom::arrangement::edge_box`'la),
  `ops/spatial_query.rs`'in yardımcıları crate içine açıldı (`boxes_meet`, `inside`, `in_or_on`, `strictly_inside`, `meets_point`, `ends`,
  `arc_toward`, `edge_gap`, `Geometry::is_empty`); `store/proximity.rs` (`Measure`, `NEAREST_STRIDE`, `NEIGHBOR_STRIDE`, `Store::nearest`,
  `Store::neighbors`; girdinin aramasını `Nearer` olarak yürüten `Search`); `store/rtree.rs`'e en iyi önce arama (`PackedTree::nearest`,
  `Nearer`, `NearestQueue`).
- **WASM ve iki çalıştırma deposu:** `crates/wasm/geometry-wasm/src/store.rs` (`nearest`, `neighbors`), web `wasm/core.ts`;
  `processing/geometry.ts` (`RunGeometry.nearest`, `.neighbors`, `NearestFound`, `NeighborFound`, `ObjectStore`); masaüstü
  `kentos_processing::geometry` (`RunGeometry::nearest`, `::neighbors`, `NearestFound`, `NeighborKind`, `NeighborFound`).
- **Araçlar:** web `processing/builtin/proximity/` (`shared.ts`, `nearest.ts`, `matrix.ts`, `hub.ts`, `neighbors.ts`, `shortestLine.ts`,
  `tools.ts`), `builtin/index.ts`, `categories.ts`'in `proximity`'si; masaüstü `crates/native/processing/src/builtin/proximity/` (`mod.rs`
  ortak yardımcılar, `nearest.rs`, `matrix.rs`, `hub.rs`, `neighbors.rs`, `shortest_line.rs`), `builtin/mod.rs`, `categories.rs`.
  Komşu alanlar'ın takma adlarından `KOMSUALAN` çıktı: Bitişik alan'ındır.
- **Arayüz:** pencereler tanımdan (ADR 0084), tablolar formun altında (ADR 0200 §5); CBS'nin Analiz sekmesinde Yakınlık kategorisi
  kendi paneli olmadan Analiz paneline katılır (web `app/ribbon.ts`'in `PROXIMITY_COMMANDS`'ı ve `omit`'i, masaüstüne envanterin
  `ribbonByMode`'uyla); web'in yerleşim denetimine Komşu alanlar ve Uzaklık matrisi pencereleri (`layout.mjs`); masaüstünün `catalog.rs`'inde `PORTED` ve
  `ported.json`; ikonlar `ui/icons.ts`'te (`nearestFeature`, `distanceMatrix`, `nearestHub`, `polygonNeighbors`, `shortestLine`; masaüstüne
  envanterle).
- **Bağımsız başvuru:** `scripts/fixtures/proximity_cases.py` (`fixtures/processing/v1/proximity.kcad` ve `proximity.json`: dokuz durum;
  yedi parsel: ikisi yaylı ortak kenarlı, ikisi köşeden değen, ikisi örtüşen; dört durak, iki okul, iki yol).
- **Sınamalar:** web `processing/cases.test.ts` (sayfada ve işçi yolunda), `processing.test.ts` (kategorilerin sırası; “KENAR” araması
  Komşu alanlar'ı da bulur); masaüstü `kentos-processing`'in `tests/cases.rs`'i (`the_proximity_cases_do_what_they_say`),
  `apps/desktop/src/processing/proximity_tests.rs` (Komşu alanlar'ın tablosu ve tek adımı, En yakını bul'un alanları); çekirdeğin birim
  sınamaları (`ops::proximity`, `store::proximity`, `store::rtree`'nin `the_nearest_come_by_distance_then_tie_breaker`'ı: rastgele 600
  kutuda kaba kuvvetin sırasıyla aynı).
- **Resimler:** masaüstü `processing::proximity_tests::screens` (`islem-yakinlik-*`), web `shots.mjs proximity` (`yakinlik-*`).
- **Süre ölçümleri:** `cargo test --release -p kentos-geometry-core --test all proximity::timing -- --ignored --nocapture` (sahne
  `tests/all/proximity.rs`'in `scene`'i), web `apps/web/scripts/perf/proximity.test.ts` (aynı sahne bit bit, `ObjectStore` ile).

## Doğrulama

10 Ekim 2026, geliştirme makinesinde (11. nesil Intel Core i5-11300H, 8 iş parçacığı, Linux; derleme 4 işle):

- **Bağımsız başvuru** (KentOS kodu olmadan, `--check` ile geçer): `proximity_cases.py` (`proximity.json`: dokuz durum; en yakın noktalar
  ve uzaklıklar kapalı biçimde, yaylı ortak kenar açı aralığıyla, örtüşen alan dikdörtgenlerin kesişimiyle).
- **Platformlar:** dokuz durum web'de sayfada ve işçi yolunda (`processing/cases.test.ts`), masaüstünde doğrudan ve çizimin okuma kopyasında
  (`the_proximity_cases_do_what_they_say`) aynı sonuçları verir; masaüstünün penceresinde Komşu alanlar'ın tablosu ortak durumun tablosuyla
  bire bir, sayıları tek adımda geri alınır, En yakını bul'un alanları ortak durumdakilerle aynı (`processing::proximity_tests`); beş aracın
  formları web'in kaydıyla (`dialog.json`) iki platformda aynı. Çekirdeğin birim sınamaları: en iyi önce arama rastgele 600 kutuda kaba
  kuvvetin sırasıyla aynı (eşit uzaklıklar hedef sırasıyla, en çok uzaklık, `k`, katılmayan öğeler), depo ve ölçülerin kuralları.
- **Resimler** iki platformda 1440×900 ve 1100×650'de, iki temada: masaüstü `islem-yakinlik-*` (Analiz sekmesi, En yakını bul, Uzaklık
  matrisi, Komşu alanlar, merkez ve en kısa çizgiler çizimde), web `shots.mjs proximity` (`yakinlik-*`). En yakın merkeze bağla'nın ilk
  ikonu 16 px'te “X”e benzediği için yeniden çizildi (§5).
- **Süreler** (release; çekirdek `proximity::timing`'in beş koşudan en kısası, web `scripts/perf/proximity.test.ts`'in gönderilen WASM'la
  10 koşusunun p50'si; ölçerken masaüstü uygulaması açıktı):

  | İş | Masaüstü | Bütçe | Web | Web bütçesi |
  |---|---|---|---|---|
  | En yakını bul, 10 000 parsel → 2 000 durak | 19,4 ms | 40 | 23,5 ms | 100 |
  | Uzaklık matrisi, `k` 5 (50 000 kayıt) | 32,7 ms | 60 | 41,1 ms | 150 |
  | En yakın merkeze bağla, → 100 merkez | 8,0 ms | 20 | 11,1 ms | 50 |
  | Komşu alanlar, 10 104 alan, köşeler ve örtüşmeler (79 012 kayıt) | 63,8 ms | 100 | 80,1 ms | 250 |
  | En kısa çizgi, → 2 000 yol parçası, `k` 1 | 23,1 ms | 40 | 28,4 ms | 100 |

  İlk ölçüm sahneyi `put` ile tek tek kurmuştu ve deponun dizini kurulmamıştı (her sorgu bütün nesneleri taradı: 900–6 600 ms);
  çalıştırmanın deposu gibi `put_many`'yle kurulunca En yakını bul 25, matris 104, merkez 129, Komşu alanlar 141 ms'ydi. Matris ve merkez,
  girdinin kutusunu iki katına çıkararak deponun bütün nesneleri arasında arıyordu; hedeflerin kendi ağacında en iyi önce aramayla (§4) 33
  ve 8 ms oldu. Komşu alanlar'ın süresinin çoğu içlerin örtüşmesi sınamasıydı (tam kare ızgarada çift başına 2,4 µs); köşe ve kesişme
  sınamalarını kutularla daraltmak (§2.2) tam kare ızgarada 141'den 33 ms'ye indirdi (kutuları yalnız değen komşular hemen elenir),
  köşeleri kaydırılmış gerçekçi parsellerde ise 96 ms'ye; yoklamaları öbür alanın kutusuna yakın kenarlarla sınırlamak 64 ms verdi. Ölçü
  sahnesi bu yüzden gerçekçi parsellerdir. Çekirdek WASM (`pnpm build`) 4 735,33 kB, gzip 1 708,44 kB (ADR 0214'ün ölçümünden +24,08 kB,
  gzip +9,18 kB).
