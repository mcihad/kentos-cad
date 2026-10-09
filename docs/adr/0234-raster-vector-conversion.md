# ADR 0234: Raster ve vektör dönüşümü

- **Durum:** kabul edildi (2026-10-09). Kapsam sahibin sözleridir (9 Ekim): “yeni bir branch açarak GIS-31 ve GIS-36 aralığını
  yapacağız”; “yüksek performans ilk önceliğimiz”; “Birleştirmeyi en sona bırakacağız sana söylediğim tüm maddeleri bitirelim”;
  MADDE-TARIFI.md'nin sırası. Madde tek parçada biter; dal `gis-31-36-raster-analysis`. İki platformda.
- **Bağlam belgesi:** TODOS.md `GIS-34`, ADR 0231 (raster çözümleme altyapısı, eş yükselti eğrilerinin vektör çıktısı), ADR 0232
  (noktalardan raster: ızgara, Kapsam, katmanın `below`'u), ADR 0233 (operasyon işi, değer okuma, alanları hücrelere çevirme,
  `RunResult.above`), ADR 0204 (raster nesnesi), ADR 0142 (köşe kotu), ADR 0149 (eğrilerin 0,1 mm'lik kirişleri).

## Bağlam

Raster ile vektör arasında iki yönde iş var. Vektörden rastere: parselleri, yolları, noktaları hücrelere yakmak (maske, sınıf
rasteri, maliyet yüzeyinin girdisi). Rasterden vektöre: sınıflandırılmış bir rasterin bölgelerini alana, ince çizgi rasterini
çizgiye, DEM'i yükseklik noktalarına çevirmek. Ayrıca taranmış paftalardan yarı otomatik sayısallaştırma: bir çizgiye tıklayıp
onu yakalamak, bir alanın içine tıklayıp onu kapatmak, yakalanan eş yükselti eğrilerine sırayla kot vermek. Netcad bunları RASVEK'te
(Rasterdan Hat Yakala, Rasterdan Eğri Yakala, Eğrilere Kot Ver, Rasterdan Alan Kapat) ve Yükseklik Noktaları'nda; ArcGIS Feature
to Raster, Raster to Polygon, Raster to Polyline, Extract Scanned Lines ile; QGIS Rasterize, Polygonize, Raster pixels to points
ile verir.

## Karar

### 1. Kapsam

İşlemler'in iki yeni kategorisinde yedi araç:

| Araç | Kimlik | Kategori | Çıktı |
|---|---|---|---|
| Rasterleştir | `raster.rasterize` | Raster ve vektör | raster (32/64 bit, tam sayı ya da bayt) |
| Rasterden alan | `raster.toPolygons` | Raster ve vektör | alanlar, Değer özniteliğiyle |
| Rasterden çizgi | `raster.toLines` | Raster ve vektör | çoklu çizgiler |
| Rasterden nokta | `raster.toPoints` | Raster ve vektör | noktalar, kotlu, Değer özniteliğiyle |
| Çizgi yakala | `scan.captureLine` | Taranmış harita | çoklu çizgiler (isteğe bağlı kotlu) |
| Alan kapat | `scan.closeArea` | Taranmış harita | bir alan |
| Eğrilere kot ver | `scan.contourElevations` | Taranmış harita | eğrilerin köşe kotları |

Çizgi yakala ve Alan kapat yarı otomatiktir: kullanıcı çizimde bir nokta seçer (İşlemler'in nokta parametresi, Sahneden seç),
gerisini araç bulur. Eğrilere kot ver iki noktanın çizdiği kesen çizgiyle çalışır.

### 2. Ortak kurallar

- **Değer, yer, hücre:** ADR 0233 §2'deki gibi: rasterin değeri bandın örneği; NaN, nodata ve alfası 0 olan piksel değersiz; yer
  nesnenin afini; hücre (i, j) hücre uzayında [i, i + 1) × [j, j + 1) yarı açık karesi (u satır boyunca, v satırlar aşağı), merkezi
  (i + ½, j + ½); bir noktanın hücre uzayındaki yeri ters afinle (§2'nin formülü, float64).
- **Hücre uzayından dünyaya:** köşe ve merkez noktaları afinle: x = x₀ + a·u + b·v, y = y₀ + c·u + d·v.
- **Değerin metni** (Değer özniteliği): örnek türünün en kısa ondalık yazımı, üssüz (tam sayı türünde tam sayı; 32 bitlikte float32'nin
  geri okununca aynı sayıyı veren en kısa ondalığı; `3`, `0.1`, `-2.5`).
- **Sonuçlar:** raster sonuç (Rasterleştir) ADR 0231 §2'nin dosyası ve nesnesi; vektör sonuçlar yeni katmanda nesneler, tek adımda.
  Rasterleştir'in katmanı girdinin katmanının hemen altında (ADR 0232'nin `below`'u: nesneler yüzeyin üstünde kalır); öbürlerininki
  rasterin katmanının hemen üstünde (`above`).
- **Sınırlar:** bütün rasteri okuyan vektörleştirme (Rasterden alan, Rasterden çizgi) en çok 2²⁶ hücre (8192 × 8192; etiketler ve
  işaretler hücre başına 5 bayt, web'in işçisi de taşısın diye); en çok 1 000 000 alan ya da çizgi, 2 000 000 nokta; Rasterden alan'ın
  satır satır verdiği geçici etiketler en çok 8 000 000. Aşınca ret, nedeni ve çaresiyle (Yeniden sınıflandır, Maskeyle kırp, Adım).

### 3. Rasterleştir

- **Girdi:** nesneler (kapalı alan, daire, elips, eğri, tarama, çizgi, çoklu çizgi, yay, nokta; Katman, Seçili, Görünen, Tümü).
- **Değer:** Sabit (varsayılan, Sabit değer 1) ya da Alandan: seçilen alandaki sayı (`kentos.statistics/1`'in okuması: ondalık nokta
  ya da virgül); sayı olmayan nesne alınmaz, söylenir.
- **Izgara:** ADR 0232 §3: Hücre boyu (0: nesnelerin kutusunun kısa kenarının 250'de biri, 1, 2, 2,5, 5 × 10ᵏ'ye aşağı
  yuvarlanır), kutu bu boyun katlarına oturur; ya da Kapsam = Rasterin ızgarası (seçilen rasterin afini ve boyu). Kutu bütün
  nesnelerin (çizgilerin kirişleri, noktalar dahil) kutusudur.
- **Yakma:**
  - Kapalı şekiller (kapalı alan, daire, tam elips, kapalı eğri, tarama): merkezi içinde olan hücreler (ADR 0233 §6).
  - Açık şekiller (çizgi, açık çoklu çizgi ve parçaları, yay, elips yayı, açık eğri): hücre, şeklin bir noktası onun yarı açık
    karesindeyse yanar. Yaylar 0,1 mm'lik kirişlerle (kiriş sayısı n = ⌈|süpürme| / (2·acos(1 − 10⁻⁴ / r))⌉, noktalar eşit açılarla),
    elips ve eğri geometri çekirdeğinin kirişleriyle (ADR 0149); her kiriş hücre uzayına alınır ve uçları dahil izlenir. Kirişin hangi
    hücrelerden geçtiği kesin karar verilir: sıradaki köşe noktasının (k, m) kirişin hangi yanında kaldığı float64 uçlarla geometri
    çekirdeğinin kesin yönlendirmesiyle (orient2d); kiriş köşeden geçerse o köşeyi içeren hücre de (yarı açık karenin sahibi) yanar.
  - Noktalar (çok noktalı nesnenin her noktası): içinde olduğu hücre (⌊u⌋, ⌊v⌋).
  - Bir nesne bir hücreyi bir kez sayılır (çizgi aynı hücreden iki kez geçse de).
- **Çakışanlar:** bir hücreyi birden çok nesne yakarsa: Son çizilen (varsayılan; girdinin sırasında sonraki), İlk çizilen, En büyük,
  En küçük, Toplam (ADR 0233 §9'un çift-çift toplamı, bir kez yuvarlanır), Sayı (nesne sayısı).
- **Tür:** Ondalık 32 bit (varsayılan), Ondalık 64 bit, Tam sayı 32 bit, Bayt; tam sayıda yarımlar sıfırdan uzağa yuvarlanır, türe
  sığmayan değer ret (Tam sayıda −2 147 483 648, Bayt'ta 255 değersiz için ayrılır). Yanmayan hücre değersiz. Görünüş Viridis.

### 4. Rasterden alan

- **Girdi:** raster ve bant; Komşuluk: 4 komşu (kenar; varsayılan) ya da 8 komşu (kenar ve köşe).
- **Bölge:** değeri birbirine tam eşit ve komşu hücrelerin bağlı kümesi; değersiz hücre bölge değildir.
- **Halkalar:** bölgenin her hücre kenarı, öbür yanı başka bölge, değersiz ya da rasterin dışıysa sınırdır; yönü bölge solda kalacak
  biçimdedir (hücre uzayında: üst kenar −u, alt kenar +u, sol kenar +v, sağ kenar −v). Bir köşede çapraz iki hücre aynı bölgedeyse
  (eyer) sınır köşeden çaprazına geçer (giren kenar çaprazdaki hücrenin kenarıyla sürer); değilse her hücre kendi köşesinden döner.
  Böylece her bölgenin bir dış halkası olur; bölgeyi bir noktadan dokunarak saran boşluk dış halkaya o noktada dokunan delik olur
  (OGC'ye göre geçerli). Köşe noktaları yalnız yönün değiştiği yerlerdir.
- **Biçim:** dış halka hücre uzayında eksi işaretli alanlı halkadır, delikler artı; dünyada dış halka saat yönünün tersine, delikler
  saat yönünde yazılır (afin aynalıysa halkalar ters çevrilir). Her halka en üstteki (en küçük v), sonra en soldaki (en küçük u)
  köşesinden başlar; delikler bu başlangıç noktalarının sırasıyla. Alanlar bölgelerin ilk hücresinin (satır satır) sırasıyla.
- **Öznitelik:** Değer (bölgenin değerinin metni, §2).

### 5. Rasterden çizgi

- **Girdi:** raster ve bant; Çizgi hücreleri: “0 ve değersiz dışındakiler” (varsayılan), Değer aralığı (en küçük ≤ değer ≤ en büyük)
  ya da Renk (üç bantlı rasterde #RRGGBB'ye Renk toleransı içinde: (ΔR² + ΔG² + ΔB²) ≤ tolerans², float64); Kısa parçaları at
  (hücre, varsayılan 0); Sadeleştirme (hücre, varsayılan 1).
- **İnceltme:** Zhang–Suen (1984), Lü ve Wang'ın (1986) düzeltmesiyle: ön plan pikseli P1, komşuları P2 … P9 kuzeyden (hücre
  uzayında v − 1) saat yönünde; B komşulardaki ön plan sayısı, A P2, P3, …, P9, P2 dizisindeki 0 → 1 geçişleri. Birinci alt adımda
  3 ≤ B ≤ 6, A = 1, P2·P4·P6 = 0, P4·P6·P8 = 0 olan pikseller, ikincide aynısı P2·P4·P8 = 0 ve P2·P6·P8 = 0 ile, alt adımın
  başındaki görüntüye bakılarak hep birlikte silinir; bir tur hiçbir şey silmeyene dek. Rasterin dışı arka plandır. Zhang ve Suen'in
  2 ≤ B'si iki piksel kalınlığındaki çapraz çizgiyi uçlarından yiyip siler (taranmış paftada sık); 3 ≤ B onu kenarla bağlı tek
  piksellik merdiven olarak bırakır.
- **İskelet:** kalan piksellerin m-komşuluğu (yatay ve düşey komşular; çapraz komşu, ikisinin ortak yatay-düşey komşularından hiçbiri
  iskelette değilse). Derecesi 2 olmayan pikseller düğümdür. Düğümler satır satır, her düğümün komşuları P2 … P9 sırasıyla gezilir;
  her yol düğümden düğüme derecesi 2 olan piksellerden geçer, her kenar bir kez. Düğümsüz halkalar ardından satır satır ilk
  pikselinden, ilk komşusuna (P2 … P9 sırasıyla) doğru, başlangıcına dönerek. Tek pikseller atılır.
- **Kısa parçalar:** önce ucu boşta, öbür ucu kavşakta olan yollardan adımı (piksel sayısı − 1) L'den az olanlar kavşak pikseli
  dışında silinir ve iskelet yeniden kurulur; sonra adımı L'den az olan bütün yollar atılır (L = 0: hiçbiri).
- **Sadeleştirme:** Douglas–Peucker hücre uzayında (piksel merkezleri), tolerans ε hücre: uzaklık |(x₁ − x₀)(yᵢ − y₀) − (y₁ − y₀)(xᵢ − x₀)|
  / √((x₁ − x₀)² + (y₁ − y₀)²) float64'te bu sırayla (uçlar aynıysa |pᵢ − p₀|), en uzak nokta (eşitse ilki) ε'dan uzaksa kalır.
  Kapalı halka önce başlangıcına en uzak noktasından (eşitse ilki) ikiye bölünür. ε = 0 yalnız doğrusal noktaları atar.
- **Sonuç:** her yol bir çoklu çizgi (piksel merkezleri dünyada); halka ilk noktası sonda yinelenerek kapanır.

### 6. Rasterden nokta

- **Girdi:** raster ve bant; Biçim: Adımla (varsayılan; Adım k hücre, varsayılan 10: i mod k = ⌊k/2⌋ ve j mod k = ⌊k/2⌋ olan
  hücreler), Her hücre, ya da Tepeler ve çukurlar (Pencere yarıçapı r hücre, varsayılan 1: (2r + 1)²'lik pencerede, rasterin
  içinde kalan, değeri olan bütün komşularından kesin büyük olan tepe, kesin küçük olan çukur; en az bir komşusu değerli olmalı).
- **Sonuç:** değeri olan hücrelerin merkezinde nokta, satır satır; Kot olarak yaz açıkken (varsayılan) kotu değer. Öznitelikler: Değer
  (§2), Tepeler ve çukurlar'da Tür (Tepe, Çukur).

### 7. Çizgi yakala

- **Girdi:** raster (taranmış pafta), Çizgi üzerinde nokta, Renk toleransı (varsayılan 60), Kısa parçaları at (hücre, varsayılan
  5), Sadeleştirme (hücre, varsayılan 1), Kot (isteğe bağlı: verilirse çizgilerin bütün köşelerinin kotu; eş yükselti eğrisi).
- **Tohum:** noktanın hücresinin çevresindeki 7 × 7 pencerede (rasterin içinde, değeri olan) en koyu hücre: parlaklık
  0,299·R + 0,587·G + 0,114·B (tek bantta değer); eşitse noktanın hücresine Chebyshev uzaklığı en küçük olan, o da eşitse satır satır
  ilki. Hedef renk tohumun rengi.
- **Çizgi:** hedef renge Renk toleransı içindeki (§5'in uzaklığı; tek bantta |Δ| ≤ tolerans) hücrelerin tohumu içeren 8-bağlı kümesi.
- **Pencere:** okuma tohumu ortalayan 1024 × 1024 hücrelik pencereyle başlar; küme pencerenin rasterin kenarı olmayan bir kenarına
  değerse pencere iki katına çıkar (ortası yine tohum) ve yeniden okunur; en çok 2²⁶ hücre. Sonuç bütün rasterde bulunacakla aynıdır.
- Küme 4 194 304 hücreden büyükse ret (tıklanan yer çizgi değil gibi).
- **Sonuç:** kümeye §5'in inceltmesi, iskeleti, kısa parçaları ve sadeleştirmesi; her yol bir çoklu çizgi.

### 8. Alan kapat

- **Girdi:** raster, Alanın içinde nokta, Renk toleransı (varsayılan 60), Delikler: Doldur (varsayılan) ya da Koru, Sadeleştirme
  (hücre, varsayılan 1).
- **Alan:** noktanın hücresinin rengine toleransı içindeki hücrelerin o hücreyi içeren 4-bağlı kümesi (çizgiler 8-bağlı olduğu için
  çapraz boşluktan sızmaz); §7'nin penceresiyle. Küme rasterin kenarına değerse ret: alan kapanmıyor.
- **Halkalar:** §4'ün halkaları (bölge bu küme); Doldur'da delikler atılır. Sadeleştirme 0'dan büyükse her halka §5'in
  Douglas–Peucker'ıyla (kapalı halka kuralı) sadeleştirilir; üç köşeden aza inen halka kendi köşeleriyle kalır. Sadeleşen
  halkalardan birinin alanı sıfırsa ya da iki kenar birbirine değer ya da keserse (uçlar dahil; aynı halkanın ardışık iki kenarı
  yalnız ikincisi birincinin üstüne geri katlanırsa sayılır; hücre uzayında tam sayılarla, kesin) bütün halkalar sadeleştirilmemiş
  yazılır ve söylenir.
- **Sonuç:** bir alan.

### 9. Eğrilere kot ver

- **Girdi:** Eğriler (çizgi, çoklu çizgi, kapalı alan; kilitli katmandakiler alınmaz), Başlangıç ve Bitiş noktaları (kesen çizgi),
  İlk kot, Aralık (0 olamaz; eksi azalır).
- **Sıra:** her eğrinin kesen çizgiyle kesişimleri (geometri çekirdeğinin kenar kesişimleri) arasında başlangıca en yakını eğrinin
  yeridir; kesmeyen eğri değişmez. Eğriler yerlerine göre, eşitse girdinin sırasıyla sıralanır; k. eğrinin kotu İlk kot + k · Aralık
  (float64'te bu sırayla).
- **Yazma:** eğrinin bütün köşelerinin (deliklerin ve parçaların dahil) kotu; tek adımda.

### 10. Arayüz

- **Araçlar** iki platformda aynı adlar, parametreler ve varsayılanlarla; Rasterleştir'in Hücre boyu, Kapsam ve Izgara rasteri
  ADR 0232'ninkiler.
- **Komutlar:** `processing.run.<kimlik>`. **Şerit:** CBS'nin Raster sekmesinde Raster ve vektör ve Taranmış harita panelleri.
- **Takma adlar:** RASTERIZE, VEKTORRASTER (RASTERLESTIR masaüstünde nokta bulutunun Rasterleştir'inindir, ADR 0207); RASTERDENALAN, RASTERALAN; RASTERDENCIZGI; RASTERDENNOKTA, YUKSEKLIKNOKTALARI;
  RASTERDENHATYAKALA, HATYAKALA, RASTERDENEGRIYAKALA; RASTERDENALANKAPAT, PAFTAALANKAPAT (POLYGONIZE ve ALANKAPAT Toplu alan'ındır, ADR 0151); EGRILEREKOTVER.

### 11. Performans

- Rasterleştir ADR 0232'nin şerit işidir (256 satır; alanlar ADR 0233 §6'nın satır aralıklarıyla, çizgi ve nokta hücreleri bir kez
  sıralanıp şeritlere dağıtılarak). Vektörleştirme ADR 0233'ün operasyon işiyle şerit şerit okur: Rasterden alan bölgeleri satır
  satır birleşim-bul ile etiketler (iki satır ve etiketler bellekte), Rasterden çizgi ön plan maskesini tutar; halkalar ve iskelet
  okuma bitince. İnceltme yalnız sınır piksellerini gezer.
- **Bütçeler** (release, geliştirme makinesi):

| İş | Masaüstü (8) | Web (işçi) |
|---|---|---|
| Rasterleştir, 10 000 parsel → 4096² | ≤ 1 s | ≤ 4 s |
| Rasterden alan, 4096², ~50 000 bölge | ≤ 2 s | ≤ 8 s |
| Rasterden çizgi, 4096², ~%3 çizgi | ≤ 2 s | ≤ 8 s |
| Rasterden nokta, 4096², Adım 10 | ≤ 0,5 s | ≤ 2 s |
| Rasterden nokta, 4096², Tepeler ve çukurlar (r = 1) | ≤ 1 s | |
| Çizgi yakala, 8192² pafta, boydan boya bir eğri | ≤ 0,3 s | ≤ 1,5 s |
| Çizgi yakala, 8192² pafta, ızgaraya bağlı bütün ağ | ≤ 1 s | |
| Alan kapat, 8192² pafta, bir parsel | ≤ 0,3 s | ≤ 1,5 s |
| Eğrilere kot ver, 10 000 eğri | ≤ 0,2 s | ≤ 0,5 s |

## Uygulama

- **Raster çekirdeği** `kentos-raster`:
  - `vector`: `Features` (tür, değerler, metinler, etiketler, halka sayıları, köşe sayıları, köşeler; `push_area`, `push_line`,
    `push_point`, `value_text`), `label` (Rasterden alan'ın satır satır birleşim-bul etiketleri), `rings` (halkaların yürünmesi, eyer
    kuralı, başlangıç ve sıra), `thin` (Lü–Wang'lı Zhang–Suen; ön plan listesi iş parçacıklarında taranır, inceltme yalnız listeyi gezer;
    m-komşuluklu yollar, kısa dallar), `simplify` (açık ve kapalı Douglas–Peucker), `capture` (tohum, pencere, 8 ve 4 bağlı dolgu),
    `work` (`VectorWork`: araç, aşamalar, pencere geçişleri, blokların satırları iş parçacıklarında, `finish`; kesişme denetimi
    ızgara kovalarıyla).
  - `rasterize` (`Objects`: nesnelerin türleri ve değerleri; `Burn`: alanlar ADR 0233'ün `Areas`'ıyla, kirişler satır satır
    `row_range` ve kesin `orient2d` düzeltmesiyle, noktalar; çakışma kuralları, türün sığması), `from_points`'in
    `PointTool::Rasterize`'ı (nesneler ızgara bilinince şerit işine döner), `ops`'un vektörleştirme işi (`Work::Vector`,
    `OpsFinished::Features`), `inputs`'un `Raw`'ı (pencerenin örnekleri dosyanın türünde, bütün bandı float64'e çevirmeden).
- **Geometri çekirdeği:** `ops::contour_elevations` (işlem `contourElevations`).
- **WASM** `raster-wasm`: `OpsAnalysis`'in nesneleri (`featureKind`, `featureValues`, `featureTexts`, `featureTags`, `featureRings`,
  `featureSizes`, `featureXy`), `PointAnalysis.sample`, notlarda `outside`.
- **İşlemler (Rust)** `builtin::raster_vector` (`mod.rs`: nesnelerin girdisi, vektörleştirme ADR 0233'ün `drive`'ıyla, Rasterleştir,
  Eğrilere kot ver; `tools.rs`: yedi araç), kategoriler `rasterVector` ve `scannedMap`; `Patch.zs` (köşe kotları; çalıştırıcı
  `elevation::assign`'la yazar). Ortak durumlar `fixtures/processing/v1/raster-vector.json` ve `.kcad`, rasterleri `raster-vector/*.tif`
  (`raster_vector_processing_cases.py`; anahtarlar `rasterVectorOf`, `elevations`), oynatıcı `tests/cases/surface.rs`'in
  `the_raster_and_vector_cases_do_what_they_say`'i.
- **Masaüstü:** `catalog.rs`'in `PORTED`'ı, `ported.json`; testler `processing/raster_vector_tests.rs`, resimler `raster_vector_scenes.rs`
  (`tools_screens`'in `vek-*`); taranmış paftanın sahnesi `fixtures/interaction/v1/scanned.kcad` ve `scanned/pafta.tif`
  (`scanned_scene.py`: kâğıt ve tarayıcı gürültüsü, vadinin 5 m'lik eğrileri kahverengi, dere, yol ve altı parsel).
- **Web:** `io/rasterAnalysisProtocol.ts` (`AnalysisFeatures`, `OpsResult.features`, `PointResult.sample`), işçi ve
  `processing/surfaceTesting.ts` aynı biçimde; `model/ops/contourElevations.ts`; araçlar `processing/builtin/rasterVector/shared.ts` ve
  `tools.ts` (kotlar ADR 0142'nin `assignElevations`'ıyla); kategoriler; şeridin `RASTER_ANALYSIS`'i Raster ve vektör ve Taranmış
  harita'yla; ikonlar. Testler `processing/cases.test.ts`'in raster ve vektör bloğu, `io/raster.wasm.test.ts`'in raster ve vektör
  bloğu (başvurunun girdileri test içinde sıkıştırmasız TIFF; yazıcı 16 bitlik işaretli bandı da yazar), `model/ops/contourElevations.test.ts`;
  ölçüm `scripts/perf/raster.mjs --only vector`; resimler `shots.mjs`'in `rastervector` grubu.
- **İkonlar** (sorulmadan seçildi): `rasterize` (ızgaranın hücrelerine yakılmış bir alan; Raster ve vektör kategorisinin de),
  `toPolygons` (ızgarada basamaklı sınırlı bölge), `toLines` (piksel merdiveni ve onu izleyen çizgi), `toPoints` (ızgarada hücre
  merkezlerinde noktalar), `captureLine` (taranmış kalın çizgi, üstünde ince yakalanan çizgi ve imleç; Taranmış harita kategorisinin de),
  `closeArea` (kalın sınırlı dolu alan ve içinde imleç), `contourElevations` (üç eğri ve onları kesen kesikli çizgi, kesişimlerde
  noktalar). İmlecin iç rengi masaüstünde SVG özniteliğinde CSS değişkeni okunmadığı için `stroke="none"` ile çizilir.

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/raster_vector_cases.py --check`: altı aracın kuralları ADR'den Python'la (yakmada kesirler; halkalar, inceltme,
    yollar ve sadeleştirme ADR'nin sırasıyla): Rasterleştir 12 durum (altı çakışma kuralı, Bayt'ta sabit, Tam sayı'da toplam, rasterin
    ızgarası, dönük ızgara, sığmayan değer ve değersiz alan retleri), Rasterden alan 6 (4 ve 8 komşu, cebi saran bölge, aynalı afin, 16
    bitlik tam sayı ve nodata), Rasterden çizgi 5 (sıfır dışı, kısa parçalar, aralık, halka, renk), Rasterden nokta 4, Çizgi yakala 5 (eğik,
    dallı, kısa parça, boşluk, rasterin dışı), Alan kapat 7 (merdiven, dış halkaya değen delik, üçgen, delikli, sadeleşmeden, sadeleşince
    kesişen, kenara ulaşan); 39 durum. Eğrilere kot ver 2 durum (yukarı ve aşağı), kesişimler kesirlerle. GDAL'la çapraz denetim:
    Rasterden alan'ın 6 durumu `gdal.Polygonize`'ın bölgeleriyle (değerler, alanlar, halka sayıları; 4 ve 8 bağlılık), Rasterleştir'in
    Sayı durumlarındaki alanlar ve çizgiler `gdal.RasterizeLayer`'ın yaktığı hücrelerle (alanlar merkez kuralıyla, çizgiler ALL_TOUCHED'la;
    6 nesne, genel konumda: kirişin tam köşeden geçtiği hücrede ADR'nin yarı açık kuralı GDAL'ınkinden ayrılabilir).
  - `scripts/fixtures/raster_vector_processing_cases.py --check`: İşlemler'in 13 ortak durumu (Rasterleştir, ızgarası, sığdırma; alanlar,
    çizgiler, renkle çizgiler, noktalar; Çizgi yakala; Alan kapat, kesişen ve açık; Eğrilere kot ver ve azalan adım); rasterleri GDAL
    yazar, okunup başvurunun girdisiyle karşılaştırılır; Rasterleştir'in dosyası başvurunun örnekleriyle bit bit (`rasterVectorOf`).
  - `scripts/fixtures/scanned_scene.py --check`: taranmış paftanın çizimi ve pikselleri.
- **Kurallar:** her durum bit bit; vektörleştirme bir ve üç iş parçacığında aynı nesneleri verir.
- **Testler:** `kentos-raster` 8 raster ve vektör testi (başvurunun 39 durumu: Rasterleştir'inkiler bir kez, öbürleri bir ve üç iş
  parçacığında; retler nedenleriyle) ve 18 birim testi
  (yakma, etiketler, halkalar, inceltme: iki piksellik çapraz çizgi kalır, sadeleştirme, yakalama, kesişme); geometri çekirdeğinde 2
  (ortak durumlar dahil); `kentos-processing` ortak durumların 13'ü ve varsayılanlar; masaüstü `raster_vector_tests` 3 (Rasterleştir
  parsellerin katmanının hemen altında ve tek adımda geri alınır, Çizgi yakala kotlu kapalı halka, Eğrilere kot ver tek adım), araç
  sayısı 60; web `cases.test.ts`'in 14'ü, `raster.wasm.test.ts`'in 40'ı (39 durum WASM modülünde), `contourElevations.test.ts`,
  `processing.test.ts` (kategori ağacında iki yeni kategori), pencere formları (`dialog.json`'a yedi form eklendi, başka satır
  değişmedi).
- **Tarayıcıda uçtan uca:** `shots.mjs rastervector` her sahnede aracı İşlemler penceresinden çalıştırır (9 sahne, iki tema, iki boy);
  masaüstü aynı yerlerde aynı değerlerle (`vek-*`).
- **Görsel incelemede düzeltilenler:** Yakalanan çizgiler katmanının rengi kahverengiydi, taranmış paftanın eğrileri üstünde
  görünmüyordu (iki platformda #D6409F); Çizgi yakala ve Alan kapat ikonlarının imleci masaüstünde boş çiziliyordu (CSS değişkeni);
  Alan kapat'ın sahnesinde 250'lik tolerans tarama gürültüsüyle (kâğıt ile kahverengi arası 246, gürültü yaklaşık ±18) eğrilerin bazı
  piksellerini dışarıda bırakıp sınırda yarık açıyordu; sahne 300'le çalışır (aracın varsayılanı 60 kalır). Takma adlardan POLYGONIZE ve
  ALANKAPAT Toplu alan'ın, RASTERLESTIR nokta bulutunun Rasterleştir'inin olduğu envanterde görüldü (§10).
- **Süreler** (9 Ekim 2026; Intel Core i5-13500, 20 iş parçacığı, Linux; karolu ve Deflate'li girdiler, dosyanın blokları, iş ve
  nesnelerin ya da sonucun yazılması dahil; masaüstünde 8 iş parçacığı, web'de işçi, release WASM, tek iş parçacığı; üç koşunun ortancası;
  web aynı kurguları kendi rasterleriyle ölçer, sınıf rasterinin karma işlevi başka):

  | İş | Masaüstü (8) | Bütçe | Web (işçi, 1) | Bütçe |
  |---|---|---|---|---|
  | Rasterleştir, 10 000 parsel → 4096², Tam sayı 32 bit | 0,145 s | 1 s | 0,417 s | 4 s |
  | Rasterden alan, 4096², ~50 000 bölge | 0,222 s | 2 s | 0,572 s | 8 s |
  | Rasterden çizgi, 4096², %3 çizgi | 0,039 s | 2 s | 0,174 s | 8 s |
  | Rasterden nokta, 4096², Adım 10 | 0,098 s | 0,5 s | 0,524 s | 2 s |
  | Rasterden nokta, 4096², Tepeler ve çukurlar (r = 1) | 0,127 s | 1 s | | |
  | Çizgi yakala, 8192², boydan boya bir eğri | 0,165 s | 0,3 s | 0,842 s | 1,5 s |
  | Çizgi yakala, 8192², ızgaraya bağlı bütün ağ | 0,381 s | 1 s | | |
  | Alan kapat, 8192², bir parsel | 0,005 s | 0,3 s | 0,045 s | 1,5 s |
  | Eğrilere kot ver, 10 000 eğri | 0,011 s | 0,2 s | 0,193 s | 0,5 s |

  Ölçümle yapılan iyileştirmeler: Çizgi yakala ağda önce 1,88 s sürüyordu (pencerenin bantları float64'e çevriliyordu: 529 ms; bloklar
  tek iş parçacığındaydı: 886 ms); `Raw` (örnekler dosyanın türünde), blokların satırlarının iş parçacıklarında işlenmesi ve inceltmenin
  bütün hücreler yerine ön plan listesini gezmesiyle 0,381 s. Masaüstü:
  `cargo test --release -p kentos-raster --test all vector_timing -- --ignored --nocapture --test-threads=1` (`KENTOS_PHASES=1` aşamaların
  sürelerini de yazar); web: `node scripts/perf/raster.mjs --only vector`.
- **Ölçülmeyenler:** p99 (az koşu); web'de Tepeler ve çukurlar ve bütün ağın yakalanması; 2²⁶ hücreye varan yakalama penceresi; dönük
  rasterlerde vektörleştirme; çok sayıda açık şeklin (yol ağının) yakılması.

## Kapsam dışı

- Kenar yumuşatma (eğri uydurma), konturlardan otomatik kot (yazıları okumak), renk sınıflandırması (renklerin kümelenmesi),
  vektörleştirmede topoloji korumalı sadeleştirme (Rasterden alan sınırları hücre kenarlarıdır).
- Uzaklık ve maliyet yüzeyleri (`GIS-36`), hidrolojinin dere ağı vektörü (`GIS-35`).
