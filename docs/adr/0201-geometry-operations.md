# ADR 0201: Geometri işlemleri

- **Durum:** kabul edildi (2026-10-07). Kapsam sahibin kararlarıdır (7 Ekim): Tampon, Kes ve Birleştir (dissolve); Kesişim, Fark,
  Simetrik fark ve birleşim, alan oranıyla değer paylaştırma; Geçerlilik ve onarım; Sadeleştir ve koordinat sistemine dönüştürme;
  hepsi İşlemler aracı, sonuç yeni ya da seçilen katmana, girdi değişmez. Simgeler sahibin seçtikleridir (7 Ekim, hepsi önerilen
  seçenek). Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-03` (ve araştırma notu), ADR 0084 (İşlem araçları), ADR 0200 (sorgular, tablo çıktısı, sayıların
  kuralı), ADR 0065 ve 0069 (alan işlemleri ve çekirdeğin örtüşmesi), ADR 0143 (çok parçalı alan), ADR 0174 (çok parçalı çizgi ve nokta),
  ADR 0140 (Sadeleştir), ADR 0149 (eğri ve elipsin 0,1 mm'lik sınırı), ADR 0167 ve 0168 (koordinat dönüşümleri).

## Bağlam

Çizimde seçili nesnelerde alan birleştirme, kesiştirme, çıkarma, Ötele ve Sadeleştir vardır (ADR 0065, 0047, 0140); bir katmanın bütün
nesnelerini öznitelikleriyle başka bir katmanla işleyen, sonucu yeni katmana yazan ve neyin kaybolduğunu söyleyen araçlar yoktur. Netcad'de
Tampon Bölge, Poligon Kesiştir ve Birleştir; ArcGIS'te Buffer, Clip, Dissolve, Intersect, Erase, Symmetrical Difference, Union, Check ve
Repair Geometry, Simplify, Project; QGIS'te bunların karşılıkları vardır.

## Karar

On bir yeni İşlemler aracı iki platformda, ADR 0084'ün çalıştırıcısıyla (tek geri alma adımı, kilitli katmana yazmama, arka planda
çalışma, modellerde kullanılma), Geometri kategorisinde. Hesap ortak çekirdekte (`ops::geoprocess`), araçların tanımı web'de
`processing/builtin/geometry/`, masaüstünde `kentos-processing`'in `builtin/geometry/`'sinde.

İki ad sahibin listesinden değişti, çünkü çizimde aynı adla başka bir komut vardır: **Kes** panonun Kes'idir, araç **Kırp** oldu (QGIS'in
Türkçesi); **Birleştir** uç uca çizgileri birleştirir, araç **Gruplayarak birleştir** oldu. Sadeleştir çizimdeki Sadeleştir'in kuralıdır,
adı aynı kaldı; takma adları ayrıdır.

### 1. Ortak kurallar

- **Girdi türleri.** Alan: kapalı alan (parçaları ve delikleriyle), daire, tam elips, kapalı eğri. Çizgi: çizgi, çoklu çizgi (parçalarıyla),
  yay, elips yayı, açık eğri. Nokta: nokta (çok noktalı da). Öbür türler (yazı, ölçü, blok, tablo, resim, kılavuz, tarama, sonsuz doğru,
  ışın) araca girmez; parametrenin açıklaması söyler. Çekirdekte bir nesne alanlar, yollar ya da noktalardır (`geoprocess::class_of`);
  yolun aynı yerdeki iki köşesi arasındaki sıfır uzunluklu kenar yoktur.
- **Eğriler.** Yaylar ve daireler kesin kalır (merkez ve yarıçap; çekirdeğin örtüşmesi yaylarla çalışır). Elips ve eğri, ADR 0149'un
  0,1 mm'lik sınırıyla doğru parçalarına çevrilerek girer; böyle bir girdi varsa (iki taraftakiler de, her nesne bir kez) söylenir:
  “n elips ya da eğri 0,1 mm içinde doğru parçalarına çevrildi.”
- **Kotlar.** Sonucun köşeleri yenidir; çizgi ve alanların köşe kotları, noktanın kotu sonuca taşınmaz ve söylenir: “n nesnenin kotları
  sonuca taşınmadı.” (Koordinat sistemine dönüştür noktanın kotunu korur.)
- **Çıktı.** Sonuç, aracın Çıktı katmanına yazılır (yeni katman varsayılandır, adı aracın: Tampon, Kırpılan, Birleştirilen, Kesişim, Fark,
  Simetrik fark, Birleşim, Onarılan, Sadeleştirilen, Dönüştürülen; rengi aracın, 0,25 mm çizgi, alanlarda rengin %15'i dolgu); girdi
  değişmez. Alan sonucu kapalı alandır (yaylar büküm olarak), birden çok parçası olan sonuç tek nesnedir, parçaları büyükten küçüğe
  (ADR 0143); çizgi sonucu çoklu çizgi (çok parçalı olabilir, ADR 0174), nokta sonucu nokta (çok noktalı olabilir). Sonuç girdinin
  özniteliklerini taşır (aracın kuralıyla); etiket, sembol, renk ve kalınlık taşınmaz, katmanın stili geçerlidir.
- **Boş sonuç.** Sonucu boş kalan girdi yazılmaz, sayılır ve söylenir: “n nesnenin sonucu boş; yazılmadı.”
- **Kayıp raporu.** Özet satırı ne yazıldığını, uyarılar neyin girmediğini ya da değiştiğini söyler; ayrıntı isteyen araçlar (Geçerlilik,
  Onar, Sadeleştir) bir tablo çıktısı verir (ADR 0200 §7), pencere onu formun altında gösterir. Tabloda sayı sütunları (her dolu hücresi
  sayı) sağa yaslı rakamlarla, öbürleri soldan okunur (iki platformda).
- **Tolerans.** Çekirdeğin örtüşmesinin toleransıdır (`arrangement::TOL`, 1 µm); başka gizli yakalama, yuvarlama ya da onarım yoktur
  (CLAUDE.md §23.3).

### 2. Tampon (`geometry.buffer`)

- **Nesneler** (alan, çizgi, nokta), **Uzaklık** (m, varsayılan 5; alanda eksi değer içe; 0,001–100 000 m), **Yan** (İki yan, Sol, Sağ;
  yalnız çizgilerde), **Halka sayısı** (1–20), **Birleştir**, **Uzaklık alanı** (Gelişmiş; seçilirse her nesnenin uzaklığı bu alandan
  okunur, Uzaklık kullanılmaz), Çıktı katmanı.
- **Geometri.** Noktanın tamponu dairedir; doğru parçasınınki kapsül (iki yan, iki yarım daire); yay kenarınınki halka dilimi (merkeze
  ulaşırsa dilim) ve uçlarında daireler; bir nesnenin tamponu parçalarının çekirdeğin örtüşmesiyle birleşimidir (tek kaynak, sıfırdan
  farklı sarım), alanda alanın kendisi de katılır. Eksi uzaklıkta alan, kenarlarının |d| kapsüllerinin birleşimi çıkarılarak küçülür
  (köşeler keskin kalır); ortasını aşan küçültme bir şey bırakmaz. Çizgi ve noktada eksi uzaklık boş sonuçtur. Tek yanlı tampon her
  kenarın o yandaki şeridi (doğruda dikdörtgen, yayda merkeze ya da dışa halka dilimi) ve yolun o yanın dışına döndüğü köşede dairenin
  iki normal arasındaki dilimidir; uçları düz. Yaylar ve dairenin yayları kesindir.
- **Halkalar.** 1. halka d'deki tampon, k. halka (k − 1)·d ile k·d arasındaki şerittir (k·d'deki tampondan (k − 1)·d'dekinin çıkarılmışı).
  Eksi uzaklıkla birden çok halka çizilmez: Uzaklık'la çalıştırılmaz (“Halkalar yalnız artı uzaklıkla çizilir; Halka sayısını 1 yapın ya
  da uzaklığı artı yazın.”), Uzaklık alanından gelen eksi değerli nesne alınmaz ve söylenir.
- **Uzaklık metni.** Uzaklık bir ondalık sayı olarak okunur (ADR 0200 §4'ün kuralı: nokta ya da virgül, üs yok); Uzaklık parametresi
  sayının en kısa yazılışıyla (JavaScript'in `String`'i, masaüstünde `js_number`). Okunamayan nesne alınmaz ve söylenir (“n nesnenin
  uzaklığı “Alan” alanından sayı olarak okunamadı; alınmadı.”).
- **Öznitelikler.** Her parça nesnenin öznitelikleri, “Uzaklık” (k·d, ondalık olarak kesin: 2,5 ve 2. halka “5.0”) ve Halka sayısı
  birden çoksa “Halka” (k). Birleştir açıkken her halka bütün nesnelerin tamponlarının birleşiminden alınır, tek nesnedir; öznitelikleri
  yalnız “Uzaklık” (bütün uzaklıklar aynıysa) ve “Halka”dır.
- Çekirdek: `geoprocess::buffer` (`buffer`, parçalar), `calls::buffer_run` (okunamayan, eksi ve boş nesneler, uzaklık metinleri).

### 3. Kırp (`geometry.clip`)

- **Nesneler** (alan, çizgi, nokta), **Kesen alanlar** (varsayılan seçili nesneler). Kesen alanlar önce birleşir. Alan birleşimle kesişir;
  çizgi birleşimin sınırını kestiği yerlerden bölünür, ortası içeride ya da sınırda olan parçalar kalır (sınır üstündeki parça içeridedir);
  nokta içinde ya da sınırında kalırsa kalır. Öznitelikler olduğu gibi. Çekirdek: `clip::within`, `calls::clip_run`.

### 4. Gruplayarak birleştir (`geometry.dissolve`)

- **Nesneler** (alan, çizgi, nokta), **Grupla** (isteğe bağlı alan; boşsa hepsi tek grup; değer metin olarak, eksik değer boş grup),
  **Toplanacak alanlar** (isteğe bağlı, virgülle; her grubun değerleri ADR 0200 §4'ün kuralıyla kesin toplanır, okunamayan atlanır ve
  söylenir), **Çok parçalı** (varsayılan evet: grup başına tek nesne; hayır: her bağlı alan parçası ayrı nesne, büyükten küçüğe, sonra
  yollar ve noktalar sırasıyla). Alanlar birleşir (ortak sınırlar kalkar); çizgiler ve noktalar grubun çok parçalı nesnesi olur (uç uca
  eklenmez). Gruplar ilk nesnelerinin sırasıyla yazılır. Sonucun öznitelikleri: grup alanı ve değeri, “Nesne sayısı” ve toplanan
  alanlar (okunan değeri olmayan alan yazılmaz).

### 5. Kesişim, Fark, Simetrik fark, Birleşim

- **Kesişim** (`geometry.intersection`): **Nesneler** (birinci taraf: alan, çizgi, nokta) ve **Kesen alanlar** (ikinci taraf). Her birinci
  taraf nesnesi, kesiştiği her ikinci taraf alanıyla ayrı bir parça verir (nesnelerin sırasıyla); parça iki tarafın özniteliklerini taşır.
- **Fark** (`geometry.difference`): **Nesneler** ve **Çıkarılacak alanlar**; ikinci taraf birleşir ve her nesneden çıkar (sınır üstündeki
  çizgi parçası da çıkar); birinci tarafın öznitelikleri.
- **Simetrik fark** (`geometry.symDifference`): **Birinci alanlar** ve **İkinci alanlar**; önce birincilerin ikincilerin dışında kalan
  parçaları birincilerin, sonra ikincilerin birincilerin dışında kalanları ikincilerin öznitelikleriyle.
- **Birleşim** (`geometry.union`): **Birinci alanlar** ve **İkinci alanlar**; önce kesişimler (iki tarafın), sonra birincilerin
  ikincilerin dışında kalanları, en son ikincilerin birincilerin dışında kalanları (kendi taraflarının öznitelikleriyle).
- **Önek.** İkinci tarafın öznitelikleri **Önek** ile yazılır (varsayılan boş); adı birinci tarafta da olan ad “ad (2)”, o da varsa
  “ad (3)” … olur.
- **Alan oranıyla paylaştır** (Kesişim ve Birleşim; Gelişmiş; isteğe bağlı alanlar, virgülle): birinci taraftaki nesnenin bu alanlarının
  değerleri parçanın payıyla çarpılır: pay parçanın alanının nesnenin alanına (çizgide uzunluğunun, noktada sayısının) oranıdır, çekirdeğin
  hesapladığı çift duyarlıklı sayı. Değer ondalık sayı olarak okunur, payın çift duyarlıklı değeriyle kesin çarpılır ve değerin kesir
  basamağından iki fazla basamağa yarım çifte yuvarlanır (`kentos.statistics/1`'in eki, `statistics::apportion`; 800,5 × 0,125 =
  100,0625 → “100.062”). Sayı okunamayan değer olduğu gibi kalır ve söylenir.
- **Boş sonuç.** Hiç parça vermeyen birinci taraf nesneleri (Simetrik fark ve Birleşim'de ikinci taraftakiler de) söylenir.
- Çekirdek: `clip::{intersection, difference, sym_difference, union}`, `calls::overlay_run`.

### 6. Geçerlilik ve onarım

- **Geçerliliği denetle** (`geometry.validity`; kapalı alan, çoklu çizgi, çizgi): her halka ve yol için, sırasıyla, her sorun bir kez ve
  ilk göründüğü yerle:
  - **Yinelenen köşe** (art arda iki köşe 1 µm içinde; kapalı halkada son ve ilk de): ilk köşenin yeri.
  - **Alanı sıfır olan halka**: düz kenarlı halkanın bütün köşeleri, ilk köşeden ve ona en uzak köşeden geçen doğrunun 1 µm içinde; yeri
    ilk köşe. Böyle halkada kesişme aranmaz.
  - **Kendini kesen halka** ve **kendini kesen yol**: sıfır uzunluklu kenarlar atıldıktan sonra iki kenar kesişir, değer ya da aynı doğru
    üstünde 1 µm'den uzun üst üste biner; komşu kenarlar yalnız ortak köşelerinde buluşabilir (kapalı halkanın son ve ilk kenarı da
    komşudur; başladığı yerde biten yol kapalı sayılır). Yeri sıradaki ilk kenar çiftinin, birinci kenar boyunca ilk buluşmasıdır.
  - **Dış halkanın dışına taşan delik**: deliğin halkanın dışında (sınırında değil) kalan ilk köşesi; yoksa deliğin kenarının halkanın
    kenarını iki halkanın köşesi dışında bir yerde kestiği ilk yer.
  - **Delikler örtüşüyor**: iki deliğin kenarlarının köşeleri dışında kesiştiği ilk yer, yoksa birinin öbürünün içindeki ilk köşesi
    (deliklerin sırasıyla, ilk çift).
  - Çıktı bir tablodur (Nesne, Katman, Sorun, Doğu, Kuzey; yer projenin uzunluk basamağıyla) ve sorunlu nesneler seçilir; çizim
    değişmez. Sorun yoksa seçim değişmez.
- **Onar** (`geometry.repair`): sorunu olmayan nesne olduğu gibi yazılır. Sorunlu alanın her parçası kendi halkasından yeniden kurulur:
  halka yazıldığı gibi (kendini kesen halkanın her ilmeği içeridedir: papyon iki üçgen olur), delikler aynı yöne çevrilip birleşerek
  çıkarılır (taşan delik halkayla kesilir, örtüşen delikler tek delik olur), sonra parçalar birleşir; yinelenen köşeler örtüşmede düşer,
  alanı kalmayan nesne yazılmaz. Çizgilerde yinelenen köşeler (sıfır uzunluklu kenarlar) düşer; kendini kesen yol onarılmaz, olduğu gibi
  yazılır ve söylenir (“n kendini kesen yol onarılmadı; olduğu gibi yazıldı.”). Sonuç Çıktı katmanına, öznitelikleriyle; rapor tablosu
  her sorunlu nesne için (Nesne, Önce, Sonra, Değişiklik): alanda “1 parça, 1 delik, 84.00 m²” (yazıldığı gibi: halkaların mutlak
  alanları, delikler çıkarılmış), çizgide “4 köşe”, hiçbir şey kalmadıysa “Boş”; Değişiklik sorunların adları, kendini kesen yol
  “(onarılmaz)” ile. Çekirdek: `geoprocess::validity` (`problems`, `repair`).

### 7. Sadeleştir (`geometry.simplify`)

- **Nesneler** (kapalı alan, çoklu çizgi), **Tolerans** (m, 0,001–1 000, varsayılan 0,1). Sadeleştir'in kuralı (ADR 0140: açık yolun
  uçları, kapalı halkanın ilk köşesi ve ona en uzak köşe, yay kenarların uçları sabit; aralarında Douglas–Peucker; alan en az üç köşe).
  Her nesne bir kez yazılır, değişmeyen olduğu gibi. Rapor tablosu değişen nesneler için: Nesne, Köşe (önce, sonra), Alan değişimi (m², alan
  basamağıyla; %, iki basamak; çizgide boş), En büyük sapma (m); özet atılan köşe toplamını ve en büyük sapmayı söyler. Ortak sınırı
  koruyan sadeleştirme `GIS-25`'tedir.

### 8. Koordinat sistemine dönüştür (`geometry.reproject`)

- **Nesneler** (alan, çizgi, nokta), **Kaynak sistem** (kayıttaki sistemler, yerel olan dışında; varsayılan WGS 84, EPSG:4326). Hedef
  projenin sistemidir (kayıttaki ya da projenin tanımı); projenin datum seçimleri uygulanır (ADR 0168, `crs::transform_in`). Projenin
  tanımları kaynak olarak sunulmaz (kaynak, verinin geldiği bilinen sistemdir).
- Köşeler tek tek dönüştürülür; yay ve daire kenarları önce, her kiriş yaydan en çok 1 mm sapacak en az sayıda eşit parçaya bölünür
  (n = ⌈|süpürme| / 2 acos(1 − 0,001/r)⌉) ve yayları böyle çevrilen nesneler söylenir. Noktanın kotu korunur. Bir köşesi dönüştürülemeyen
  nesne yazılmaz, nedeniyle sayılır: izdüşümün dışında, datumlar arasında yol yok, datum seçiminin ızgarası bu cihazda yok, ızgaranın
  dışında. Projenin sistemi yoksa araç çalışmaz (“Projenin koordinat sistemi yok; önce Proje ayarları'nda sistem seçin.”); kaynak
  projenin sistemiyle aynıysa da (“Kaynak sistem projenin sistemiyle aynı; dönüştürülecek bir şey yok.”). Özet: “n nesne dönüştürüldü:
  EPSG:2320 → EPSG:5254.”
- Web'de çalıştırma projenin sistemini işe koyar (`RunJob.crs`, `RunContext.crs`: SRID, kod, sistem ve datum seçimleri; işçiye de
  gider); masaüstünde araç çizimin ayarlarından okur (`kentos_project::systems`; işlem araçları artık proje modeline bağlanabilir).

## Uygulama

- **Çekirdek** `crates/shared/geometry-core/src/ops/geoprocess/`: `mod.rs` (nesnenin sınıfı, birleşim, çıkarma, kesişim, ölçüler,
  sonucun şekli), `buffer.rs`, `clip.rs` (Kırp, örtüşmeler, Gruplayarak birleştir), `validity.rs` (sorunlar, Onar), `reproject.rs`,
  `calls.rs` (araç başına işlev ve web'in işlemleri: `geoBuffer`, `geoClip`, `geoOverlay`, `geoDissolve`, `geoValidity`, `geoRepair`,
  `geoSimplify`, `geoReproject`, `geoMeasure`); paylaştırma `ops::statistics::apportion` (işlem `apportion`).
- **Web** `model/ops/geoprocess.ts`; araçlar `processing/builtin/geometry/` (`shared.ts`, `buffer.ts`, `clip.ts`, `dissolve.ts`,
  `overlay.ts`, `validity.ts`, `simplify.ts`, `reproject.ts`); `RunJob.crs` ve `RunContext.crs` (`processing/job.ts`, `runner.ts`,
  `types.ts`); sonuç tablosunun sütun hizası (`ui/processing/ToolDialog.ts`); simgeler `ui/icons.ts` (`geoBuffer` … `geoReproject`).
- **Masaüstü** `crates/native/processing/src/builtin/geometry/` (aynı on bir araç; yeni katmanın stili kalınlık ve dolguyla,
  `NewLayerStyle`); pencere ve komutlar mevcut İşlemler penceresidir (`apps/desktop/ported.json`'da on bir komut), tablo hizası
  `processing/window.rs`; KentOS UI'ın simge çizicisi `stroke-opacity` okur (Tampon'un şeridi).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/geoprocess_cases.py` (ADR'den, KentOS kodu olmadan; mpmath 50 basamak, düz kenarlarda kesirler,
  pyproj): `fixtures/geoprocess/v1/cases.json` — 20 tampon durumu (noktanın, doğru parçasının, dışbükey alanın, delikli alanın, dönen
  yolun, tek yanlı yolların, yayın, dairenin ve çok noktalı nesnenin kapalı biçimli alanları; içeride ve dışarıda örnek yerler uzaklıkları
  hesaplanarak; halkalar, virgüllü uzaklık, Birleştir, okunamayan ve boş), 8 Kırp, 8 örtüşme, 6 Gruplayarak birleştir, 14 geçerlilik
  (yerler kesirlerle), 8 Onar, 3 Sadeleştir (Douglas–Peucker'ın başvurusu), 4 dönüştürme (PROJ'un köşeleri, 1 mm'lik kirişler, ret) ve 10
  paylaştırma. Çekirdeğin `tests/all/geoprocess.rs`'i işlemleri adlarıyla, web'in çağırdığı gibi oynatır.
- Çalıştırmalar: `fixtures/processing/v1/geometry.json` ve `geometry.kcad` aynı betikten; 19 durum (her araç, iki ret), yeni nesneler
  ölçüleriyle (`addedShapes`: tür, katman, öznitelikler, parça, delik, alan ya da uzunluk, noktalar, köşeler) ve araçların çizimdeki
  varsayılanları. Web (`processing/cases.test.ts`) her durumu sayfada ve işçinin yolundan, masaüstü (`kentos-processing`'in
  `tests/cases.rs`'i) burada ve çizimin okuma kopyasında oynatır; ikisi de geçer.
- Masaüstünün pencere testleri (`apps/desktop/src/processing/geometry_tests.rs`: Tampon'un halkaları tek adımda yeni katmanda,
  Geçerliliği denetle'nin tablosu ve seçimi, aynı sistemden dönüştürmenin reddi) ve resimler (`islem-geometri-*`, web'de
  `shots.mjs geometry`).
- Süre (`geoprocess::timing`, release): 24 köşeli 400 parselin 2 m tamponu tek tek 0,36 s, birleştirerek 0,46 s; Gruplayarak birleştir
  0,02 s; 400 × 400 Kesişim 0,12 s.
