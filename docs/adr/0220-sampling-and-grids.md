# ADR 0220: Örnekleme ve ağ üretimi

- **Durum:** kabul edildi (2026-10-11). Kapsamı ben belirledim (sahibin sözü: “Benim seçmeme gerek yok sen sıradan devam et”). İki platform
  (masaüstü ve web); `.kcad` şeması değişmez; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-24`; ADR 0219 (Geometri üretimi: Çıktı katmanı, Analiz › Geometri ▾), ADR 0201 (geometri işlemleri:
  overlay, Kırp'ın “sınırda içeridir” kuralı, eğriler 0,1 mm), ADR 0200 §1 (Konuma göre seç'in Kesişen'i, 1 mm), ADR 0142 (kotların
  taşınması), ADR 0218 (yol yürüyüşü, `toleranceFields`), ADR 0188 (yol boyunca doğrultu: köşede sonraki kenar), ADR 0140 (Böl, Ara nokta),
  ADR 0047 (Dizi).
- **Araştırma:** QGIS'in Create grid (Point, Line, Rectangle, Diamond, Hexagon; yatay ve düşey aralık, örtüşme; id, left, top, right,
  bottom, row_index, column_index; yukarıdan aşağı, soldan sağa), Regular points (aralık ya da sayı, iç pay, Randomize), Random points
  in polygons ve Random points on lines (nesne başına sayı ya da ifade, nesne içi ve genel en az uzaklık, nokta başına deneme, tohum,
  öznitelikler; yazılamayan noktaların sayısı), Points along geometry (aralık, başlangıç ve bitiş payı; distance, angle) ve Subdivide'ı
  (en çok köşe); ArcGIS'in Create Fishnet'i (başlangıç, hücre genişliği ve yüksekliği, satır ve sütun, etiket noktaları), Generate
  Tessellation'ı (SQUARE, HEXAGON, TRANSVERSE_HEXAGON, TRIANGLE, DIAMOND; alanla boy; GRID_ID), Create Random Points'i (kısıtlayan
  nesneler, sabit ya da alandan sayı, en az uzaklık), Generate Points Along Lines'ı (uzaklık ya da yüzde, uçlar) ve Subdivide Polygon'u
  (eşit alanlar: ifraz, burada değil); PostGIS'in `ST_Subdivide`'ı (`lwgeom_subdivide_recursive`: köşe sınırı, en az 5, kutunun uzun
  kenarına dik kesim, kesim yeri halkanın ortaya en yakın köşesi, delikler çoksa en büyük deliğinki, 50 kat); Netcad'in Karelaj Üret'i.

## Bağlam

Bir bölgeyi düzenli hücrelere ayırmak (balık ağı, altıgen ağ: sayım, yoğunluk, örnekleme çerçevesi), bir alanın içine düzenli ya da
rastgele örnek noktaları koymak (orman envanteri, toprak ve su örneklemesi, denetim noktaları), bir hattın üstüne aralıkla noktalar
koymak (direk, kilometre, enkesit yerleri) ve çok köşeli büyük nesneleri dizinlenebilir parçalara ayırmak CBS'nin gündelik işidir.
Çizimde Böl, Ara nokta ve Dizi vardı; katman katman çalışan araçları yoktu.

## Karar

### 1. Kapsam

İşlemler'in yeni **Örnekleme** kategorisinde beş araç, iki platformda:

1. **Ağ oluştur** (`sample.grid`): dikdörtgen, altıgen ya da üçgen hücreler, ya da dikdörtgen ağın çizgileri.
2. **Düzenli noktalar** (`sample.regular`): alanların içine aralıkla, isteğe bağlı rastgele kaydırmayla.
3. **Rastgele noktalar** (`sample.random`): alanların içine ya da çizgilerin üstüne, tohumla.
4. **Hat boyunca nokta** (`sample.alongLines`): çizgilerin ve alan sınırlarının üstüne aralıkla ya da eşit parçalara.
5. **Alt bölümlere ayır** (`sample.subdivide`): alanları ve çoklu çizgileri en çok köşe sayısına göre parçalara.

Ortak kurallar ADR 0201 ve 0219'unkiler: sonuç **Çıktı katmanı**na yeni nesnelerdir (yoksa adıyla açılır), girdi değişmez ve yalnız
okunur (kilitli katmanınki de); hesaplar çekirdekte (`ops::sampling`); en çok **1 000 000** nesne yazılır, daha çoğu çalıştırmadan önce
reddedilir. Rastgelelik tohumludur: aynı girdi ve tohum her platformda ve her çalıştırmada bit bit aynı noktaları verir (§4). Şeritte
CBS'nin Analiz sekmesinde Geometri panelinin ▾'inde, Geometri üretimi'nin araçlarından sonra (sekme 1100 piksel genişliğinde dolu).
**Kapsam dışı:** elmas ve sivri üstlü altıgen hücreler, hücrelerin alanlarla kırpılması (ardından Kırp, ADR 0201 §3), örtüşen hücreler,
alanla verilen hücre boyu, eşit alanlı alt bölümler (ifraz: GIS-06), ağırlıklı ya da tabakalı örnekleme, çizgide yüzdeyle yer, üç
boyutlu ağlar.

### 2. Ağ oluştur

- **Girdi:** Kapsam nesneleri (her tür; yardımcı çizgi ve ışın dışında); ağ onların sınır kutusunu kaplar (yaylar ve yazılar kendi
  kutularıyla, `entity_bounds`). Kapsamın kutusu yoksa reddedilir (“Kapsamda kutusu olan nesne yok.”).
- **Biçim:** Dikdörtgen (varsayılan), Altıgen (düz üstlü düzgün altıgen), Üçgen (eşkenar) ya da Çizgi (dikdörtgen ağın çizgileri).
  **Doğu aralığı** ve **Kuzey aralığı** (m, varsayılan 100) dikdörtgen ve çizgi için; **Kenar** (m, varsayılan 100) altıgen ve üçgen için.
- **Koordinatlara otur** (varsayılan açık): ağ koordinat başlangıcına bağlı genel kafese oturur; dikdörtgenler aralığın katlarında
  başlar, ayrı ayrı üretilen ağlar birbirine uyar (Netcad'in karelajı). Kapalıyken ağ kapsamın sol alt köşesinden başlar.
- **Kafes** (float64'te bu sırayla; ağın köşeleri yalnız tam sayı kafes numaralarından hesaplanır, komşu hücreler köşelerini bit bit
  paylaşır):
  - Dikdörtgen: `ox = otur ? floor(x0 / dx) · dx : x0`, `oy` aynı; sütun sayısı `nc = max(1, ceil((x1 − ox) / dx))`, satır `nr` aynı.
    Satır r (1 en üst), sütun c (1 en sol): `xa = ox + (c − 1) · dx`, `xb = ox + c · dx`, `ya = oy + (nr − r) · dy`,
    `yb = oy + (nr − r + 1) · dy`; halka (xa, ya), (xb, ya), (xb, yb), (xa, yb).
  - Çizgi: yatay çizgiler `y = oy + (nr + 1 − r) · dy` (r = 1 … nr + 1, yukarıdan) `ox`'tan `ox + nc · dx`'e, sonra düşey çizgiler
    `x = ox + (c − 1) · dx` (c = 1 … nc + 1, soldan) `oy`'dan `oy + nr · dy`'ye.
  - Altıgen (kenar a): `u = a / 2`, `h = (a · √3) / 2`; `otur` ise `ox = floor(x0 / (3a)) · 3a`, `oy = floor(y0 / (2h)) · 2h`. Kafes
    sütunu c'nin merkezi `kx = 3c + 2`, satırı r'nin `ky = 2r + 1 + (c mod 2)` (eksiye de tam sayı kalanı); köşeler `(ox + k · u, oy + m · h)`:
    (kx+1, ky−1), (kx+2, ky), (kx+1, ky+1), (kx−1, ky+1), (kx−2, ky), (kx−1, ky−1). Taranan sütunlar
    `floor((x0 − ox) / (3u)) − 1 … ceil((x1 − ox) / (3u))`, satırlar `floor((y0 − oy) / (2h)) − 1 … ceil((y1 − oy) / (2h))`.
  - Üçgen (kenar a): `otur` ise `ox = floor(x0 / a) · a`, `oy = floor(y0 / (2h)) · 2h`; satır r `[oy + r·h, oy + (r+1)·h]`, tek
    satırlar u kadar kayık: `x = 2j + (r mod 2)`; yukarı üçgen (x, r), (x+2, r), (x+1, r+1), aşağı üçgen (x+2, r), (x+3, r+1), (x+1, r+1);
    taranan j `floor((x0 − ox) / a) − 1 … ceil((x1 − ox) / a)`, r `floor((y0 − oy) / h) … max(r0, ceil((y1 − oy) / h) − 1)`.
  - Altıgen ve üçgende kutuyla içi örtüşen (ayıran eksen sınaması; değmek yetmez) hücreler alınır; dikdörtgenler kutuyu tam kaplar.
- **Hücreler:** Kutunun hepsi (varsayılan) ya da **Nesnelere değenler** (Konuma göre seç'in Kesişen'i, 1 mm: kenarı bir nesnenin
  kenarına değen ya da kesen, bir nesneyi içine alan ya da bir nesnenin alanında kalan hücre; ADR 0200 §1). Çizgi biçiminde yoktur.
- **Öznitelikler:** hücrede **Satır** ve **Sütun** (kafesin numaraları; seçilmeyen hücrelerin numaraları atlanır); yatay çizgide **Satır**
  ve **Kuzey**, düşey çizgide **Sütun** ve **Doğu** (çizginin koordinatı, projenin uzunluk basamaklarıyla). Sıra: satırlar yukarıdan,
  sütunlar soldan.
- **Sınır:** 4 000 000'dan çok hücre taranacaksa ya da 1 000 000'dan çok parça yazılacaksa reddedilir (“Ağ … çok büyük …; aralığı
  büyütün ya da kapsamı küçültün.”).

### 3. Düzenli noktalar

- **Girdi:** alanlar (alan, daire, kapalı elips ve eğri); **Doğu aralığı**, **Kuzey aralığı** (m, varsayılan 10); **Koordinatlara otur**
  (varsayılan açık); **Rastgele kaydırma** (m, varsayılan 0) ve **Tohum** (tam sayı, varsayılan 1).
- **Kafes:** `otur` ise noktalar `(i · dx, j · dy)`; değilse bütün alanların kutusunun sol alt köşesinden yarım aralık içeride,
  `(X0 + (i + 0,5) · dx, Y0 + (j + 0,5) · dy)` (hücrelerin ortaları). Her alan kendi kutusundaki (kaydırmayla genişlemiş) kafes
  noktalarını sınar.
- **Kaydırma:** kafes noktası (i, j) her eksende `(2u − 1) · kaydırma` kadar kayar; u'lar `Sıra(tohum ⊕ i · 0x9E3779B97F4A7C15 ⊕
  j · 0xC2B2AE3D27D4EB4F)`'nın ilk iki sayısıdır (§4; i ve j ikinin tümleyeniyle 64 bit, çarpım ve ⊕ 64 bitte): aynı kafes noktası her
  alanda aynı kayar, sonuç tarama sırasına bağlı değildir.
- **İçeride:** nokta alanın içinde ya da sınırında (1 µm; Kırp'ın kuralı) ise yazılır; delikler dışarıdır. Ortak sınırdaki nokta iki
  alana da yazılır. Nokta alanın özniteliklerini alır.
- **Sıra:** alanlar girdideki sırayla; bir alanda satırlar yukarıdan, sütunlar soldan.
- **Sınır:** 16 000 000'dan çok kafes noktası sınanacaksa ya da 1 000 000'dan çok nokta yazılacaksa reddedilir.

### 4. Rastgele noktalar

- **Girdi:** alanlar ve çizgiler (alan, daire, elips, eğri, çizgi, çoklu çizgi, yay); **Nokta sayısı** (ifade, nesne başına, varsayılan
  “10”; örneğin `$alan / 10000 * 5`: hektarda beş); **En az uzaklık** (m, varsayılan 0: bütün yazılan noktalar arasında); **Deneme**
  (tam sayı, varsayılan 10: en az uzaklık sağlanmazsa nokta başına aday); **Tohum** (tam sayı, varsayılan 1).
- **Sayı:** ifadenin değerinin tam kısmı; sonlu değilse ya da sayı değilse nesne atlanır ve söylenir, eksiyse sıfırdır. Toplam
  1 000 000'u aşarsa reddedilir.
- **Sıra (rastgele sayılar):** bütün çalıştırmanın tek dizisi: SplitMix64 tohumu dört 64 bitlik duruma yayar, xoshiro256** sayıları
  verir; [0, 1)'deki sayı üst 53 bitin 2⁵³'e bölümüdür. Yalnız tam sayı işlemleri ve tek bölme: bağımsız başvuru bit bit yineler.
- **Alanda:** nesnenin alanlarının kutusunda (x0, y0, x1, y1) aday `x = x0 + u · (x1 − x0)`, `y = y0 + v · (y1 − y0)` (önce u, sonra v);
  alanın içinde ya da sınırında olan ilk aday alınır, 10 000 aday dışarıda kalırsa nesnenin kalan noktaları yazılmaz (“çok ince”).
- **Çizgide:** yolların toplam uzunluğu L (yollar ve kenarlar sırayla toplanır); aday `s = u · L`, yolların uzunlukları sırayla
  düşülerek yoluna, orada düştüğü kenara (köşede sonraki kenar) yerleşir. Noktanın kotu kenarın uçlarının kotlarından doğrusal (ADR 0142), **Uzaklık** yolunun başından, birden çok
  yolu olan nesnede **Parça**.
- **En az uzaklık:** aday, yazılmış bir noktaya `dx² + dy² < d²` ise reddedilir (kareler: iki platformda ve başvuruda aynı işlem); bir
  nokta için **Deneme** kadar aday reddedilirse o nokta yazılmaz. Yazılamayan noktalar ve nesneleri söylenir.
- Nokta nesnenin özniteliklerini alır.

### 5. Hat boyunca nokta

- **Girdi:** çizgiler (çizgi, çoklu çizgi ve parçaları, yay, daire, elips, eğri) ve alanların sınırları (dış halka ve delikler, her parça,
  kapalı yol olarak); elips ve eğri 0,1 mm'lik kirişlerle (söylenir).
- **Yerleşim:** **Aralıkla** (varsayılan; **Aralık** m, varsayılan 10; **Başlangıç payı**, **Bitiş payı** m, varsayılan 0) ya da
  **Eşit parçalara** (**Parça sayısı**, varsayılan 10); **Uçları da** (varsayılan açık).
- **Uzaklıklar** (yol başına, L uzunluk, `ε = 1e-9 · max(1, L)`):
  - Aralıkla: `s_k = başlangıç + k · aralık`, `s_k ≤ L − bitiş + ε` iken (L'yi aşan L olur); kapalı yolda `L − ε`'dan büyük olan
    başlangıcın kendisidir, alınmaz. Uçları da: açık yolda ilk uzaklık ε'dan büyükse 0, sonuncusu `L − ε`'dan küçükse L eklenir;
    kapalı yolda yalnız 0.
  - Eşit parçalara (n): `s_k = (L · k) / n`, k = 1 … n − 1; Uçları da: açık yolda k = 0 ve n, kapalı yolda k = 0.
- **Nokta:** s'nin düştüğü kenarda (köşede sonraki kenar, L'de son kenarın ucu); **Semt** oradaki gidiş doğrultusu (köşede sonraki
  kenarınki; projenin açı biriminde, kuzeyden saat yönünde), **Uzaklık** s, birden çok yolu olan nesnede **Parça**. Kot: köşedeki
  noktada köşenin kotu, kenarda iki ucunun kotu varsa doğrusal (ADR 0142).
- **Sıra:** nesneler girdideki sırayla, yollar sırayla, noktalar uzaklığa göre. 1 000 000'dan çok nokta reddedilir (önce sayılır).

### 6. Alt bölümlere ayır

- **Girdi:** alanlar ve çoklu çizgiler; **En çok köşe** (tam sayı, en az 5, varsayılan 256).
- **Kural** (PostGIS'in `ST_Subdivide`'ı, KentOS'un yaylı geometrisine uyarlanmış):
  1. Çok parçalı nesnenin her parçası (delikleriyle) ayrı ele alınır.
  2. Köşe sayısı (halkaların, deliklerin de; yolun) en çok köşeyi aşmayan ya da 50 kattan derindeki parça yazılır.
  3. Değilse kutusu (yayların uçları kesin) uzun kenarına dik ikiye kesilir (genişlik yükseklikten büyükse düşey çizgiyle, değilse
     yatay). Kesim yeri alanda dış halkanın kutunun ortasına en yakın köşesinin o eksendeki koordinatı (eşitse küçüğü); köşe sayısı dış
     halkanınkinin iki katından az değilse en büyük deliğin (eşitse en alttaki, sonra en batıdaki köşesiyle). Kesim yeri kutunun bir
     kenarına 1e-12'den yakınsa ya da parça yolsa kutunun ortası.
  4. Alan kesim doğrusunun iki yanına ayrılır (alçak, sonra yüksek): overlay'in alanla kutunun iki yarısından verdiğinin aynısı, yaylar
     kesin, ama genel overlay'den geçmeden (§7): halkalar doğruyu kestikleri yerde bölünür (düz kenarın kesim noktası tam doğrunun
     üstünde; doğruya 1 µm'den yakın köşe doğrunun üstünde sayılır, düğümü köşenin kendisidir, overlay'in birleştirdiği gibi), parçalar
     ortalarının yanına, doğru boyunca giden kenar alanın kaldığı yana düşer; doğrunun alanın içinde kalan aralıkları iki yarının
     parçalarını birleştirir; parçalar alan sollarında kalacak biçimde halkalara bağlanır (birkaçının buluştuğu noktada geri dönüş
     yönünden saat yönünde ilk dönüş), saat yönünün tersindekiler dış halka, öbürleri onları tutan en küçük dış halkanın deliği. Yolda
     yol kesim çizgisinde bölünür, parçaları ortalarının yanına göre alçak (kesimde ya da altında) ya da yüksek yarıya düşer, böylece
     kesim çizgisi üstündeki parça bir kez yazılır. Her sonuç (alanın her parçası, her yol) bir kat derinde yeniden ele alınır.
- Parçalar nesnenin özniteliklerini ve **Parça** 1, 2 … alır (kutularının sol alt köşesine göre: önce batı, sonra güney; eşitse sağ üst
  köşesine, sonra ilk, en alttaki köşelerine göre: overlay'in parçaları veriş sırasına bağlı olmayan bir sıra). Kotlar ADR
  0142'nin kuralıyla: özgün köşe kendi kotunu, kesimin yeni köşesi kenarının uçlarından doğrusal kotu alır.

### 7. Performans

Hesaplar çekirdekte, nesne başına JSON'suz; web'e ağlar ve noktalar düz sayı dizileriyle döner (yüz bin hücre dört dizi). Nesnelere
değenler bir milyon hücrede de her hücreyi yalnız yakın kenarlarla sınar (kenarlar ve noktalar paketlenmiş R-ağaçlarında, alanların içi
sargı dizinlerinde). Alt bölümlere ayır'ın kesimi genel overlay'den geçmez: 10 000 köşeli bir halkanın bir kesimi overlay'de 297 ms,
doğru boyunca kesimde 2 ms; doğrunun içte kalan aralıkları az iken sargı sayısı kenarlardan, çokken bir dizinden okunur. Bütçeler
Doğrulama'da.

## Sonuçlar

- Beş araç iki platformda, ortak durumlar bağımsız başvuruyla; Analiz › Geometri ▾ on araç taşır.
- Rastgele sayıların kuralı sabittir: kural değişirse ortak durumlar ve bu ADR birlikte değişir.

## Uygulama

- **Çekirdek** `ops::sampling` (`crates/shared/geometry-core`): `random` (SplitMix64 ve xoshiro256**, kafes noktasının dizisi),
  `grid` (kafesler, ayıran eksen sınaması), `cover` (Nesnelere değenler; `spatial_query`'nin `edges_meet`'i), `inside` (alanın içi ve
  1 µm'lik sınırı), `regular`, `scatter`, `walk` (yollar uzunluğa göre, kotlarıyla), `along`, `cut` (doğru boyunca kesim), `subdivide`
  (köşelerin kotları paketlenmiş ağaçlarla), `calls` (web'in `sampleGrid`, `sampleRegular`, `sampleRandom`, `sampleAlong`,
  `sampleSubdivide`'ı). `geom::arrangement`'in `WindingIndex`'i kenarlarını da taşıyabilir (`owned`).
- **Masaüstü** `kentos-processing`'in `builtin/sampling/` (`grid.rs`, `points.rs`, `along.rs`, `subdivide.rs`), kategori `sampling`;
  pencere testleri ve resimler `apps/desktop/src/processing/sampling_tests.rs`.
- **Web** `model/ops/sampling.ts`, `processing/builtin/sampling/` (`grid.ts`, `points.ts`, `along.ts`, `subdivide.ts`, `shared.ts`),
  kategori `sampling`, ikonlar `sampling`, `fishnet`, `regularPoints`, `randomPoints`, `pointsAlongLine`, `subdivideParts`; şeritte
  `GENERATE_UNDER`'a eklendi; resimler `shots.mjs sampling`, yerleşim `layout.mjs`'in `processing-sample-*` görünümleri.

## Doğrulama

- **Bağımsız başvuru** `scripts/fixtures/sampling_cases.py` (KentOS kodu olmadan; `fixtures/processing/v1/sampling.json` ve
  `sampling.kcad`, 29 durum): ağlar ADR'nin kafes formülleriyle aynı işlem sırasında, Nesnelere değenler GEOS'un uzaklığıyla (1 mm'ye
  yakın hücre olmadığı denetlenir); alan içi kesirlerle (1 µm sınırına yakın nokta olmadığı denetlenir); rastgele sayılar Python'un
  tam sayılarıyla; yol noktaları ve semtler yayların tanımından; parçalar GEOS'un kesişimiyle ve kesim doğrusuna 1 µm kadar yakın köşe
  olmadığı denetlenerek. İki platform 29 durumu sayfada, işçide, masaüstünde doğrudan ve okuma kopyasında, pencereden kullanıcının
  doldurduğu gibi geçer; rastgele noktalar bit bit.
- **Çekirdek testleri** (`ops::sampling::tests`, 13): hücreler kutuyu boşluksuz ve örtüşmesiz kaplar, komşular köşelerini bit bit
  paylaşır; Nesnelere değenler `spatial_query::intersects` ile her hücrede aynı; alan içi `inside_area` ile aynı; düzenli noktalar
  sınırı alır; uzaklıklar kurala uyar; rastgele noktalar tohumlu ve en az uzaklıkta; parçalar alanı ve uzunluğu korur; doğru boyunca
  kesim overlay'le aynı alanları ve köşeleri verir (köşelerden ve kenarlardan geçen, delikli ve yaylı, alana yalnız köşelerinden değen);
  doğruya köşelerinden değen yaylı alan bütün kalır.
- **Süreler** (release; masaüstü `cargo test --release -p kentos-processing --test cases sampling_timing -- --ignored --nocapture`,
  web `SAMPLING_BENCH=1 … scripts/perf/sampling.test.ts` gönderilen WASM'la; p50 / p95, çalıştırıcının denetimi, araç ve tek geri
  alma adımı dahil):

  | İş | Masaüstü | Web | Bütçe |
  |---|---|---|---|
  | Ağ oluştur, 100 000 parselin kutusunda 12 m'lik altıgenler, Nesnelere değenler (107 427 hücre) | 466,7 / 473,8 ms | 948,0 / 990,3 ms | 800 / 1500 ms |
  | Düzenli noktalar, 100 000 parselde 12 m (169 662 nokta) | 406,8 / 421,7 ms | 809,3 / 859,4 ms | 700 / 1300 ms |
  | Rastgele noktalar, 100 000 parselde birer, en az 2 m | 363,2 / 368,1 ms | 816,3 / 865,5 ms | 600 / 1300 ms |
  | Hat boyunca nokta, 100 000 parselin sınırında 25 m'de bir (257 142 nokta) | 501,9 / 507,0 ms | 1090,4 / 1181,7 ms | 900 / 1800 ms |
  | Alt bölümlere ayır, 10 alanda 100 000 köşe, en çok 256 (31 450 parça) | 583,8 / 670,2 ms | 922,1 / 972,8 ms | 1200 / 1500 ms |

  Alt bölümlere ayır genel overlay'le 28,5 s sürüyordu; doğru boyunca kesimle 0,58 s.
- **Resimler:** masaüstü `.run/shots/islem-orn-*` (36), web `apps/web/scripts/e2e/out/shots/sampling` (36); yerleşim denetimi 1100 × 650
  ve 1440 × 900'de iki temada sorunsuz.
