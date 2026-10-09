# ADR 0236: Uzaklık ve maliyet

- **Durum:** kabul edildi (2026-10-09). Kapsam sahibin sözleridir (9 Ekim): “yeni bir branch açarak GIS-31 ve GIS-36 aralığını
  yapacağız”; “yüksek performans ilk önceliğimiz”; “Tamam 36 bitince main ile birleştir ve main push yap”; MADDE-TARIFI.md'nin sırası.
  Madde tek parçada biter; dal `gis-31-36-raster-analysis`. İki platformda.
- **Bağlam belgesi:** TODOS.md `GIS-36` (ilgili `CIVIL-12`), ADR 0231 (raster çözümleme altyapısı; coğrafi rasterde satırın metresi),
  ADR 0232 §3 (nesnelerin kutusundan ızgara, hücre boyunun seçimi), ADR 0233 (operasyon işi, rasterlerin tek ızgarada okunması, çift
  doğrusal örnek), ADR 0234 (Rasterleştir'in yakma kuralı, Douglas–Peucker), ADR 0235 (hidrolojinin bellekteki işi, taban-2 öbeği).

## Bağlam

Güzergâh ön çalışması (yol, boru hattı, enerji nakil hattı), hizmet alanları ve uygunluk çözümlemeleri iki yüzeye dayanır: bir yere düz
uzaklık ve oraya varmanın birikimli maliyeti. Netcad bunları Koridor Analizi'nde (Güzergah Bul, Maliyet Hesapla) verir. ArcGIS Pro'da
Distance Accumulation, Distance Allocation, Optimal Path As Line ve Least Cost Corridor, QGIS'te GDAL'ın Proximity'si ve GRASS'ın
r.cost, r.walk, r.path ve r.grow.distance araçları vardır.

Araştırmada bulunanlar (9 Ekim):
- **Düz uzaklık:** ızgara örneklerinde kesin Öklit uzaklık dönüşümü doğrusal sürede yapılır (Felzenszwalb ve Huttenlocher 2012,
  *Theory of Computing* 8: 415–428; Meijster, Roerdink ve Hesselink 2000). Dönüşüm ayrılabilirdir: önce sütunlar, sonra satırlar;
  dikdörtgen hücrede ağırlıklarla.
- **Birikimli maliyet:**
  - GRASS'ın r.cost'u hücre merkezlerinin ağında Dijkstra'dır (8 komşu, `-k` ile at hamleleriyle 16).
  - ArcGIS'in eski Cost Distance'ı da 8 komşulu ağdır.
  - Bu oturumda GRASS 8.4.2 ile denenerek bulundu: r.cost dik ve çapraz adımda iki hücrenin maliyetini ortalar; at hamlesinde iki uçla
    adımın geçtiği iki hücreyi, dört hücreyi ortalar. Uzunluğu doğu–batı hücre boyu biriminde ölçer, maliyeti “hücre başına” sayar.
    Değersiz hücreye adım atmaz, ama çapraz adımı iki yanındaki değersiz hücrelerin köşesinden geçirir.
- **Ağın bozulması:** sabit maliyette ağın yolu düz uzaklıktan uzundur. Bu oturumda hesaplanan en büyük fark 8 komşuda %8,24 (22,5°'de),
  16 komşuda %2,75'tir.
- **ArcGIS Pro'nun yöntemi:** Pro 2.5'ten beri Distance Accumulation eikonal denklemini çözer (Sethian 1999, Zhao 2004). Ağ yerine
  yüzeyi yeniden kurar, maliyeti hücrenin kendi eğimiyle alır; sabit maliyette düz uzaklığı verir.
- **ArcGIS'in koridoru:** Least Cost Corridor iki uçtan birikimli maliyetlerin toplamıdır. Eşiği yok, en küçük toplamın yüzdesi ya da
  birikimli maliyet değeridir.

Bağımsız çapraz denetim için GRASS GIS 8.4 ve GDAL kullanılır. Bu oturumda denendi:
- r.grow.distance'ın Öklit uzaklığı kare ve dikdörtgen hücrede kaba kuvvetle hücre hücre aynı çıktı.
- GDAL'ın Proximity'si yalnız kare pikselde doğru (dikdörtgende kendisi “Pixels not square” diye uyarır).

## Karar

### 1. Kapsam

İşlemler'in yeni Uzaklık ve maliyet kategorisinde dört araç:

| Araç | Kimlik | Çıktı |
|---|---|---|
| Uzaklık yüzeyi | `distance.euclidean` | raster: en yakın kaynağa düz uzaklık (m) ya da en yakın kaynak |
| Birikimli maliyet | `distance.cost` | raster: en ucuz kaynağa birikimli maliyet ya da en ucuz kaynak |
| En düşük maliyetli yol | `distance.path` | çoklu çizgiler, her varış nesnesinden en ucuz kaynağa |
| Maliyet koridoru | `distance.corridor` | raster: iki uç kümesinden birikimli maliyetlerin toplamı |

Düz uzaklık kesin dönüşümle, maliyet araçları ağ yöntemiyle hesaplanır (§4). Ara sonuçlar yazılmadan tek çalıştırmada biter.

### 2. Ortak kurallar

- **Değer, yer, hücre:** ADR 0233 §2'deki gibi: bandın örneği; NaN, nodata ve alfası 0 olan piksel değersizdir; hücre (i, j)'nin
  merkezi (i + ½, j + ½).
- **Komşular** bu sırayla (raster uzayında doğudan saat yönünde, j aşağı): (1, 0), (2, 1), (1, 1), (1, 2), (0, 1), (−1, 2),
  (−1, 1), (−2, 1), (−1, 0), (−2, −1), (−1, −1), (−1, −2), (0, −1), (1, −2), (1, −1), (2, −1).
  - 8 komşuda bunlardan |Δi|, |Δj| ≤ 1 olanlar aynı sırayla kullanılır.
  - |Δi| + |Δj| = 3 olanlar at hamleleridir.
- **Uzunluk:** adımın çıktığı hücrenin satırının eksenleriyle (ADR 0231 §2: projeksiyonlu rasterde afin, coğrafi rasterde satırın GRS80
  metresi; `[a, b, c, d]`). Adım (Δi, Δj) (x, y) = (a·Δi + b·Δj, c·Δi + d·Δj) olur, uzunluğu L = √(x·x + y·y), float64'te bu sırayla.
- **Nesnelerin hücreleri:** kaynaklar ve varışlar ızgaraya ADR 0234 §3'ün kuralıyla yakılır:
  - kapalı şekil merkezini içine alan hücreleri, açık şekil dokunduğu hücreleri, nokta içinde olduğu hücreyi yakar;
  - nesnenin numarası girdideki sırasıdır (1'den);
  - bir hücreyi birden çok kaynak yakarsa hücre girdide önce gelenindir;
  - hiçbir hücreye düşmeyen nesne sayılır ve söylenir.
- **Sınır:** bütün raster bellekte çalışılır (float64): en çok 2²⁵ hücre. Aşınca ret, çaresiyle (Yeniden örnekle, Maskeyle kırp).
- **Sonuçlar:** raster sonuçlar ADR 0231 §2'nin dosyası ve nesnesidir. Katmanları:
  - nesnelerden Uzaklık yüzeyi kaynakların katmanının hemen altındadır (nesneler üstte kalır, Rasterleştir gibi);
  - rasterden Uzaklık yüzeyi ve maliyet araçlarının rasterleri girdi rasterinin hemen üstündedir;
  - yollar maliyet rasterinin katmanının hemen üstündeki yeni katmandadır.

### 3. Uzaklık yüzeyi

- **Girdi:**
  - Kaynak: Nesneler (nokta, çizgi, alan) ya da Raster (bandın değerli hücreleri kaynaktır).
  - Izgara, nesnelerde ikisinden biri:
    - kaynakların kutusu, her yanında Kenar payı (m, varsayılan 100) kadar geniş; Hücre boyu verilmezse ADR 0232 §3'ün kuralıyla, payla
      genişlemiş kutunun kısa kenarının 250'de biri, güzel boya yuvarlanarak; kenarlar hücre boyunun katlarında.
    - bir rasterin ızgarası.
  - Rasterde ızgara rasterin kendisidir.
  - En büyük uzaklık (m; 0: sınırsız).
  - Sonuç: Uzaklık (varsayılan) ya da En yakın kaynak.
- **Izgara:**
  - Eksenleri dik olmalıdır: a·b + c·d, |a·b + c·d| ≤ 10⁻¹²·(a² + b² + c² + d²) değilse ret (Yeniden örnekle ile kuzeyi yukarıda
    ızgaraya alın).
  - Coğrafi ızgarada düz uzaklık metre değildir: ret (projeksiyonlu sisteme alın).
  - sx = √(a·a + c·c) sütun adımının, sy = √(b·b + d·d) satır adımının uzunluğudur.
- **Tanım:**
  - Hücrenin uzaklığı, merkezinin en yakın kaynak hücrenin merkezine düz uzaklığıdır: d = √(x·x + y·y), x = Δi·sx, y = Δj·sy, float64'te
    bu sırayla. Kaynak hücrede 0'dır.
  - En yakın kaynak, (Δi·sx)² + (Δj·sy)²'yi en küçük yapan hücredir; eşitse sütunu küçük olan, o da eşitse satırı küçük olan.
- **En yakın kaynak:** kaynak hücrenin numarasıdır. Nesnelerde yakan nesnenin numarası, rasterde hücrenin değeri.
- **En büyük uzaklık:** d bundan büyükse hücre değersizdir.
- **Hesap:** Felzenszwalb ve Huttenlocher'in ayrılabilir dönüşümü.
  - Önce her sütunda her satıra en yakın kaynak satırı bulunur (eşitse küçük satır).
  - Sonra her satırda sütunların parabollerinin alt zarfı kurulur (eşitse küçük sütun).
  - Kare hücrede (sx = sy) karşılaştırmalar tam sayılarla kesindir; dikdörtgen hücrede ağırlıklarla float64'tedir.
  - Sütunlar ve satırlar iş parçacıklarında çalışır.
- **Sonuç:** 32 bit ondalık. Görünüş Uzaklık'ta Viridis (en küçükten en büyüğe), En yakın kaynak'ta Spektral ve en yakın örnekleme.
- **Engel yoktur:** düz uzaklık engelin üstünden ölçülür. Engel çevresinden uzaklık için Birikimli maliyet 1 maliyetle kullanılır.

### 4. Birikimli maliyet

- **Girdi:**
  - Maliyet rasteri ve bant. Değer metre başına maliyettir; değersiz hücre engeldir. 0 ya da eksi değer reddedilir: geçilmeyecek yer
    değersiz yapılır, çok ucuz yer küçük bir artı değer alır.
  - Kaynaklar (nesneler).
  - Komşuluk: 16, at hamleleriyle (varsayılan) ya da 8.
  - Yükseklik modeliyle (varsayılan kapalı); açıkken Yükseklik modeli, Yüzey uzunluğu (varsayılan kapalı) ve En büyük boyuna eğim
    (%, 0: sınırsız). Yükseklik modeli maliyet rasterinin ızgarasına ADR 0233 §2'nin çift doğrusal örneğiyle okunur. Anahtar, seçimi
    alan isteğe bağlı rasterin maliyet rasterinin kendisi olmasını önler; açıkken model seçilmemişse ret.
  - En büyük maliyet (0: sınırsız).
  - Sonuç: Birikimli maliyet (varsayılan) ya da En ucuz kaynak.
  - Sonuç türü: 32 bit (varsayılan) ya da 64 bit ondalık.
- **Adımın maliyeti:**
  - Dik ve çapraz adımda iki hücrenin maliyetinin ortalaması çarpı adımın uzunluğu: ((c₀ + c₁) / 2)·L.
  - At hamlesinde iki uçla adımın geçtiği iki hücrenin, dört hücrenin ortalaması (GRASS'ın r.cost `-k`'sı):
    (((c₀ + c₁) + c₂) + c₃) / 4·L. Geçtiği hücreler: Δi = ±2'de (i + Δi/2, j) ve (i + Δi/2, j + Δj), Δj = ±2'de (i, j + Δj/2) ve
    (i + Δi, j + Δj/2).
  - Toplamlar float64'te bu sırayla yapılır.
  - Yüzey uzunluğu açıkken L yerine √(x·x + y·y + Δz·Δz) alınır (Δz varışın yüksekliği eksi çıkışınki).
- **Geçilemezlik:**
  - Değersiz hücreye (maliyette ya da yükseklik modelinde) adım atılmaz.
  - Çapraz adım, iki yanındaki dik komşunun ikisi de değersizse atılmaz. Köşeden bağlı bir engel çizgisi geçilmez; ArcGIS Pro da böyle
    yapar, GRASS geçirir.
  - At hamlesi, geçtiği iki hücreden biri değersizse atılmaz.
  - En büyük boyuna eğimde |Δz| > eğim / 100 · L ise adım atılmaz (iki yönde). Böylece yol dik yamaçta kıvrılarak çıkabilir.
- **Birikimli maliyet:**
  - Kaynak hücrelerde 0'dır. Öbür hücrelerde, f(n) + adım(n → c)'nin komşular üstünden en küçüğüdür.
  - Bu, toplamların float64'te bu sırayla yapıldığı eşitliklerin en küçük çözümüdür (Dijkstra'nın verdiği); hesabın sırası sonucu
    değiştirmez.
  - Kaynaktan erişilemeyen ya da En büyük maliyeti aşan hücre değersizdir.
- **Geldiği komşu:** f(n) + adım(n → c) = f(c) olan komşulardan f'si en küçük olan; eşitse hücre numarası (j·genişlik + i) küçük olan.
  En ucuz kaynak, geldiği komşunun en ucuz kaynağıdır.
- **Hesap:** taban-2 öbekli Dijkstra (ADR 0235 §3'ün öbeği). Geldiği komşular sonra satırlarda paralel bulunur; kaynaklar hücrelerin
  kesinleşme sırasıyla iletilir.
- **Bozulma:** sabit maliyette ağın verdiği maliyet düz uzaklığın en çok %8,24 (8 komşu) ve %2,75 (16 komşu) fazlasıdır.
  - Eikonal yöntem (ArcGIS Pro) bu farkı kapatır, ama seçilmedi:
    - yolun maliyeti adımlarının toplamı olmaz;
    - sonuç GRASS'la karşılaştırılamaz;
    - geri yönün izlenmesi hücre merkezlerinden çıkar.
  - Ağ yönteminde her çizginin maliyeti, adımlarından yeniden hesaplanabilir.
- **Sonuç:** Birikimli maliyet'te Viridis, Yüzde gerdirme. En ucuz kaynak'ta Spektral, en yakın örnekleme.

### 5. En düşük maliyetli yol

- **Girdi:** §4'ünkiler (Sonuç ve Sonuç türü hariç), Varış (nesneler) ve Sadeleştirme (hücre; varsayılan 0).
- **Varış hücresi:** varış nesnesinin hücrelerinden birikimli maliyeti en küçük olanı; eşitse hücre numarası küçük olanı.
- **Yol:** varış hücresinden geldiği komşular boyunca kaynağa. Çizgi kaynaktan varışa doğru hücre merkezlerindendir.
- **Sadeleştirme:** ADR 0234 §5'in Douglas–Peucker'ıyla, hücre uzayında; 0 yalnız doğrusal köşeleri atar.
- **Öznitelikler:**
  - Yol: varış nesnesinin girdideki sırası.
  - Kaynak: kaynak nesnesinin numarası.
  - Maliyet: varış hücresinin birikimli maliyeti.
  - Uzunluk: m, adımların plan uzunlukları kaynaktan varışa sırayla toplanır.
  - Yükseklik modeli varken ayrıca Yüzey uzunluğu (m) ve En büyük eğim (%, adımların |Δz| / L'lerinin en büyüğü).
- Erişilemeyen ya da rasterin dışındaki varış atlanır, sayısı söylenir.
- **Hesap:** Dijkstra, varışların hücreleri kesinleşip sıradaki maliyet en büyük varış maliyetini aşınca durur.

### 6. Maliyet koridoru

- **Girdi:**
  - Maliyet rasteri.
  - Birinci uçlar (A) ve İkinci uçlar (B), nesneler.
  - §4'ün komşuluk ve yükseklik ayarları.
  - Eşik: Yok (varsayılan), En küçük toplamın yüzdesi ya da Birikimli maliyet. Yüzde varsayılan 10, Değer.
  - Sonuç türü.
- **Koridor:** K(c) = f_A(c) + f_B(c) (float64). İkisinden biri tanımlı değilse hücre değersizdir. En küçük K, en ucuz A–B yolunun
  maliyetidir; koridorun ortası o yoldur. Hiçbir hücrede K yoksa ret (uçlar birbirine erişemiyor). En küçük K eşikten önce alınır ve
  özette söylenir: eşik ondan küçükse koridor boştur, söylenir.
- **Eşik:**
  - Yüzde: K ≤ K_en küçük · (1 + yüzde / 100).
  - Değer: K ≤ değer.
  - Eşiği aşan hücre değersizdir. ArcGIS'in Least Cost Corridor'ı budur.
- **Hesap:** iki Dijkstra iki iş parçacığında.
- **Sonuç:** Viridis, en küçükten en büyüğe gerdirme.

### 7. Arayüz

- **Araçlar** iki platformda aynı adlar, parametreler ve varsayılanlarla; **komutlar** `processing.run.<kimlik>`. **Şerit:** CBS'nin
  Raster sekmesinde Uzaklık ve maliyet paneli.
- **Parametreler:** kaynak nesneler `sources`, varışlar ve ikinci uçlar `targets`, raster `input` (Uzaklık yüzeyi'nde Kaynak Raster'ken
  kaynak raster, öbürlerinde maliyet rasteri); Uzaklık yüzeyi'nin Kaynak'ı (Nesneler, Raster) yalnız birini gösterir. Çıktı katmanı
  görünen girdiye göre yerleşir: nesnelerin sonucu `sources`'un katmanının hemen altına, rasterin sonucu `input`'unkinin hemen üstüne
  (iki platformun koşucusu görünmeyen girdiyi yerleşimde saymaz).
- **Takma adlar:** UZAKLIK, PROXIMITY, EUCDIST; MALIYET, COSTDIST; GUZERGAHBUL, COSTPATH; KORIDOR, CORRIDOR.

### 8. Performans

- Raster tek okumada belleğe alınır; blokları iş parçacıklarında çözülür.
- Uzaklık dönüşümü doğrusaldır: sütunlar ve satırlar iş parçacıklarında.
- Dijkstra tek iş parçacığındadır, adım adımdır (adımda en çok 2²¹ hücre kesinleşir; ilerleme ve Durdur çalışır). Komşunun maliyeti,
  uzunluğu ve geçilebilirliği satırın tablolarından okunur.
- Geldiği komşu ve sonucun yazılması paraleldir. Koridorun iki Dijkstra'sı iki iş parçacığındadır; yol varışlarda durur.
- Kesinleşmiş ya da aynı toplamla ulaşılmış komşu, adımının maliyeti hesaplanmadan atlanır (toplamı küçülemez); iç hücrelerde
  komşuların ve adımın geçtiği hücrelerin yeri tablodan okunur.
- **Bütçeler** (release, geliştirme makinesi, 4096² maliyet rasteri, 10 kaynak):

| İş | Masaüstü (8) | Web (işçi) |
|---|---|---|
| Uzaklık yüzeyi, rasterden | ≤ 1 s | ≤ 3 s |
| Uzaklık yüzeyi, en yakın kaynak | ≤ 1 s | ≤ 3 s |
| Uzaklık yüzeyi, 2000 nokta ve 100 çizgiden | ≤ 1,5 s | ≤ 4 s |
| Birikimli maliyet, 8 komşu | ≤ 4 s | ≤ 10 s |
| Birikimli maliyet, 16 komşu | ≤ 5 s | ≤ 12 s |
| Birikimli maliyet, En ucuz kaynak | ≤ 5,5 s | ≤ 13 s |
| Birikimli maliyet, yükseklik modeli ve eğimle | ≤ 6 s | ≤ 15 s |
| En düşük maliyetli yol, 5 varış | ≤ 5 s | ≤ 12 s |
| Maliyet koridoru | ≤ 6 s | ≤ 18 s |

### 9. Doğrulamanın ilkesi

Bağımsız başvuru (KentOS kodu olmadan, ADR'den Python'la) her aracın sonucunu hücre hücre verir:
- düz uzaklıkta kaba kuvvetle (her hücre için her kaynak, kesirlerle en küçük);
- maliyette (birikim, kesinleşme sırası) önceliğiyle Dijkstra'yla;
- geldiği komşu, yollar ve koridor ADR'nin kurallarıyla.

Çapraz denetimler:
- GRASS GIS 8.4'ün r.cost'u (8 ve 16 komşu; uzunluk birimi doğu–batı hücre boyu olduğundan bizimki hücre boyuna bölünür), köşeden
  bağlı engeli olmayan rasterlerde;
- r.grow.distance (kare ve dikdörtgen hücre);
- GDAL'ın Proximity'si (kare hücre).

## Uygulama

- **Raster çekirdeği** `kentos-raster`'ın `distance`'ı:
  - `edt` (`nearest`: sütunların en yakın kaynak satırları parçalar hâlinde iş parçacıklarında, satırların alt zarfı; kare hücrede
    kesirlerle i64/i128, dikdörtgende float64 ağırlıklarla; `distance`), `network` (`MOVES`; `Network`: maliyet, yükseklik, satırların
    adım tabloları, hamlelerin türleri ve iç hücre ofsetleri, `step`; `Search`: taban-2 öbekli Dijkstra adım adım, `stop`, `lost`;
    `predecessor`, `predecessors`, `sources_of`, `path_to`), `mod` (`DistanceTool`, `DistanceNotes`, `plane_steps`, `euclid`, `burnt`,
    `Near`: nesnelerden düz uzaklık; `DistanceWork`: okuma, hazırlık, arama, sonuç ve yazma aşamaları; varışların hücreleri, yollar).
  - `ops`'un dört türü (`distance`, `costDistance`, `costPath`, `costCorridor`), `Work::Distance`, görünüşler ve `Notes.distance`;
    `from_points`'in `PointTool::Distance`'ı (nesnelerin kutusu ve Kenar payı ya da bir rasterin ızgarası); `rasterize`'ın
    `Objects::numbered`'ı.
- **WASM** `raster-wasm`: notlarda `distance`.
- **İşlemler (Rust)** `builtin::distance` (`mod.rs`: raster sonucu, yollar ve nesnelerden uzaklığın koşucuları; `tools.rs`: dört araç),
  `distance` kategorisi (18. kategori). Ortak durumlar `fixtures/processing/v1/distance.json` ve `.kcad`, rasterleri `distance/*.tif`
  (`distance_processing_cases.py`; anahtar `distanceOf`), oynatıcı `tests/cases/surface.rs`'in `the_distance_cases_do_what_they_say`'i.
- **Masaüstü:** `catalog.rs`'in `PORTED`'ı, `ported.json`; testler `processing/distance_tests.rs`, resimler `distance_scenes.rs`
  (`tools_screens`'in `uzk-*`); sahne `fixtures/interaction/v1/distance.kcad` ve `distance/maliyet.tif` (`distance_scene.py`: vadinin
  yükseklik modeli, yumuşatılmış eğiminden maliyet, değersiz göl, beş köy, mevcut yol).
- **Web:** araçlar `processing/builtin/distance/shared.ts` ve `tools.ts`; kategori; koşucunun yerleşimi görünen girdiyle
  (`runner.ts`'in `layerOf`'u); şeridin `RASTER_ANALYSIS`'i Uzaklık ve maliyet'le; ikonlar. Testler `processing/cases.test.ts`'in uzaklık
  bloğu, `io/raster.wasm.test.ts`'in uzaklık bloğu; ölçüm `scripts/perf/raster.mjs --only distance`; resimler `shots.mjs`'in `distance`
  grubu.
- **İkonlar** (sorulmadan seçildi): `distanceSurface` (raster çerçevesinde kaynak ve eş uzaklık halkaları), `costDistance` (kaynaktan
  uzaklaştıkça koyulaşan hücreler), `costPath` (engelin çevresinden iki uç arasında yol; kategorinin de), `costCorridor` (iki uç
  arasında S biçimli bant ve ortasında yol). Birincisi önce köşeden yayılan çeyrek halkalardı; kablosuz ağ simgesine benzediği için
  değişti.

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/distance_cases.py --check`: dört aracın kuralları ADR'den Python'la, 39 durum. Düz uzaklık kaba kuvvetle (her
    hücre için her kaynak, karesi kesirle; kare, dikdörtgen ve dönük hücre, eşitlikler, en büyük uzaklık, en yakın kaynak; coğrafi,
    eğik ve kaynaksız ızgaranın reddi), nesnelerden (kutu ve pay, rasterin ızgarası); birikimli maliyet önceliğiyle Dijkstra'yla (8 ve
    16 komşu, en ucuz kaynak, en büyük maliyet, köşeden bağlı duvar ve engel blokları, dikdörtgen hücre, coğrafi rasterde satırın
    metresi, yükseklik modeliyle yüzey uzunluğu, eğim sınırı ve ikisi, başka ızgaradaki modelin çift doğrusal örneği; 0 maliyetin,
    engeldeki kaynağın ve eğim aralığının reddi); yollar (16 ve 8 komşu, sadeleştirme, engel, yükseklik modeliyle, rasterin dışındaki ve
    engeldeki varış; varışsızın reddi); koridor (eşiksiz, yüzde, değer, en ucuz yoldan küçük eşik; erişilemeyen uçların reddi).
  - Çapraz denetim (aynı betik): GRASS GIS 8.4'ün r.cost'u 8 ve 16 komşuyla, kare ve dikdörtgen hücrede 3219 hücre (bizim toplam hücre
    boyuna bölünür; 10⁻¹² bağıl içinde), r.grow.distance 5734 hücre (kare ve dikdörtgen), GDAL'ın Proximity'si 2867 hücre (kare; kendi
    float32'sine yuvarlanarak en çok bir son basamak birimi).
  - `scripts/fixtures/distance_processing_cases.py --check`: İşlemler'in 18 ortak durumu; rasterleri GDAL yazar, okunup başvurunun
    girdisiyle karşılaştırılır; yazılan dosyalar başvurunun örnekleriyle (`distanceOf`); yollar, öznitelikleri, özetler ve uyarılar
    başvurunun notlarından.
  - `scripts/fixtures/distance_scene.py --check`: resimlerin çizimi ve maliyet rasteri.
- **Kurallar:** her örnek bit bit; coğrafi rasterde derecenin metresi sin ve cos'tan geldiğinden (kitaplıklar son biti farklı
  yuvarlayabilir) float32'de en çok bir son basamak birimi. Çekirdek her durumu bir ve üç iş parçacığında oynatır.
- **Testler:** `kentos-raster` uzaklık testi (39 durum, iki iş parçacığı sayısıyla) ve 6 birim testi (en yakın kaynağın kaba kuvvetle
  aynılığı, eşitlikler dahil; sabit maliyetin ağ uzaklığı; köşeden bağlı engel; eğim sınırı; yol); `kentos-processing` ortak durumların
  18'i ve varsayılanlar; masaüstü `distance_tests` 3 (yollar maliyet rasterinin hemen üstünde, tek adımda geri alınır, uçları köylerin
  hücrelerinde, eğimi en çok %30; nesnelerden uzaklık nesnelerin hemen altında, rasterden rasterin hemen üstünde; yükseklik modeliyle
  maliyet ve koridorun özeti, modelsiz anahtarın reddi), araç sayısı 72; web `cases.test.ts`'in 19'u, `raster.wasm.test.ts`'in 40'ı
  (39 durum WASM modülünde), `processing.test.ts` (kategori ağacında Uzaklık ve maliyet), şeridin panelleri, pencere formları
  (`dialog.json`'a dört form eklendi, başka satır değişmedi).
- **Tarayıcıda uçtan uca:** `shots.mjs distance` her sahnede aracı İşlemler penceresinden çalıştırır (8 sahne, iki tema, iki boy);
  masaüstü aynı yerlerde aynı değerlerle (`uzk-*`).
- **Görsel incelemede düzeltilenler:** sahnenin maliyeti önce ham yükseklik modelinin eğimindendi; modelin gürültüsü maliyete geçip
  yolları titretiyordu, eğim σ = 3 hücrelik süzgeçten geçmiş modelden alındı; süzgeç modelin değersiz köşesini ortalamayla dolduruyor,
  kenarında pahalı bir şerit bırakıyordu, değersiz hücre ağırlıksız sayıldı. Koridor sahnesinde yol katmanı koridorun altında kalıyordu
  (önce yol çalışır). İsteğe bağlı yükseklik modeli seçimi alınca maliyet rasterinin kendisi oluyordu: Yükseklik modeliyle anahtarı
  eklendi. Web'in koşucusu görünmeyen girdinin ham değerini katman yerleşiminde okuyordu.
- **Süreler** (9 Ekim 2026; Intel Core i5-13500, 20 iş parçacığı, Linux; 4096² 32 bitlik, karolu ve Deflate'li maliyet rasteri, 5 m,
  tepelerle pahalanan maliyet ve değersiz göller; kaynak rasteri 839 kaynak hücreli; yükseklik modeli hidrolojininki; dosyanın blokları,
  iş ve sonucun ya da yolların yazılması dahil; masaüstünde 8 iş parçacığı, web'de işçi, release WASM, tek iş parçacığı; üç koşunun
  ortancası):

  | İş | Masaüstü (8) | Bütçe | Web (işçi, 1) | Bütçe |
  |---|---|---|---|---|
  | Uzaklık yüzeyi, rasterden | 0,367 s | 1 s | 1,692 s | 3 s |
  | Uzaklık yüzeyi, en yakın kaynak | 0,274 s | 1 s | 1,170 s | 3 s |
  | Uzaklık yüzeyi, 2000 nokta ve 100 çizgiden | 0,350 s | 1,5 s | 1,678 s | 4 s |
  | Birikimli maliyet, 8 komşu | 2,675 s | 4 s | 4,074 s | 10 s |
  | Birikimli maliyet, 16 komşu | 3,459 s | 5 s | 5,041 s | 12 s |
  | Birikimli maliyet, En ucuz kaynak | 3,678 s | 5,5 s | 6,075 s | 13 s |
  | Birikimli maliyet, yükseklik modeli ve eğimle | 4,064 s | 6 s | 8,862 s | 15 s |
  | En düşük maliyetli yol, 5 varış | 3,173 s | 5 s | 4,054 s | 12 s |
  | Maliyet koridoru | 4,358 s | 6 s | 7,907 s | 18 s |

  16 komşuda arama zamanın %90'ıdır (3,03 s; 16,7 milyon hücre, hücre başına 180 ns); masaüstünde tek iş parçacığıyla 4,0 s. Ölçümle
  yapılan iyileştirme: kesinleşmiş komşu adımın maliyeti hesaplanmadan atlanır, iç hücrelerde ofsetler tablodan (3,74 s'den 3,46 s'ye).
  Masaüstü: `cargo test --release -p kentos-raster --test all distance_timing -- --ignored --nocapture --test-threads=1`
  (`KENTOS_PHASES=1` aşamaları da yazar); web: `node scripts/perf/raster.mjs --only distance`.
- **Ölçülmeyenler:** p99 (az koşu); 2²⁵ hücreye varan raster; web'de bellek tepesi; coğrafi rasterde süreler.

## Kapsam dışı

- Eikonal (ArcGIS Pro'nun bozulmasız) yöntemi; yatay ve düşey etkenlerin başka işlevleri (Tobler, r.walk'un Naismith'i); kaynak
  özellikleri (başlangıç maliyeti, kapasite, yön).
- Engel çevresinden düz uzaklık (Birikimli maliyet 1 maliyetle ağ uzaklığını verir); coğrafi ızgarada jeodezik düz uzaklık.
- Geri yön ve kaynak yönü rasterleri; çok ölçütlü maliyet yüzeyi (`GIS-37`); güzergâhın alternatifleri ve raporu (`CIVIL-12`).
