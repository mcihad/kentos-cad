# 0920 — KentOS Yol: yüzeyin proje sınırını kapsaması, verisiz ve tahmin edilen yerler

Tarih: 2026-11-04. plan/road.md Y19.1 (ROAD-SURVEY-08, ROAD-UX-07). Durum: kabul, açıklar aşağıda.

## Dayanak

- **Civil 3D:** yüzeyin Use Maximum Triangle Length seçeneği, en büyük üçgen boyunu aşan üçgenleri yüzeyin sınırından
  çıkarır; Use Maximum Angle'la birlikte önce açı, sonra boy uygulanır
  ([UseMaximumTriangleLength](https://help.autodesk.com/cloudhelp/2026/ENU/Civil3D-API/files/html/d645797d-2881-24e7-00ff-8fb3af8fefef.htm),
  [Definition Tab](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-FBCE86FA-8941-4A9A-9F89-17C8B334A45E.htm),
  4 Kasım 2026'da okundu).
- **ASPRS ve USGS:** düşük güvenli alanlar (low confidence areas), çıplak zemin modelinin doğruluğu tutturamayabileceği
  yerlerdir ve 2B çokgen olarak teslim edilir. Zemin nokta yoğunluğu nominal aralığın dörtte birinin altına düşen yerler
  adaydır; boşluğun kabul edilen nedenleri su, yansıtmayan yüzey ve gölgedir
  ([GeoCue: Generating Low Confidence Polygons](https://support.geocue.com/wp-content/uploads/2015/04/Generating-Low-Confidence-Polygons.pdf),
  [USGS Lidar Base Specification](https://www.usgs.gov/node/28848)).
- **USGS Lidar Base Specification (yoğunluk ızgarası):** hücre tasarımdaki toplam nominal nokta aralığının iki katıdır
  (QL2'de 2 × 0,71 m), her hücre içindeki noktaların sayısıyla doldurulur; hücrelerin en az %90'ında en az bir nokta
  olmalıdır ([Lidar Base Specification: Collection Requirements](https://www.usgs.gov/ngp-standards-and-specifications/lidar-base-specification-collection-requirements),
  [Tables](https://www.usgs.gov/ngp-standards-and-specifications/lidar-base-specification-tables), 4 Kasım 2026'da
  okundu). Bu sayılar USGS'nindir; KentOS'ta hücre, en az nokta ve pay kullanıcınındır.
- **Civil 3D ve OpenRoads Designer:** yüzeyin istatistiklerinde nokta sayısı ve alan var, hücre hücre yoğunluk aracı
  yok; OpenRoads'un arazi analizi (Analyze Point, Analyze Elevation) yoğunluk vermiyor.
- **OpenRoads Designer:** arazi modelinin kenar yöntemi en büyük üçgen boyuyla dış kenarı keser (Y2'de ADR 0900 §5'in
  dayanağı).
- **Netcad:** NETPRO için verisiz alanı ve tahmin edilen yüzeyi raporlayan çevrimiçi resmî belge bulunamadı.

## Karar

### §1 Komut

`road.terrains.coverage` v1, yalnız okur. Girdi: `surface`; `boundary`: çizimin kapalı nesnesi (düz kenarlı, tek
parçalı kapalı çoklu çizgi ya da alan; alanın delikleri sayılmaz), yoksa yüzeyin kendisi; `longest`: planda kenarı bunu
aşan üçgen tahmin edilmiştir. Bu kullanıcınındır; KentOS bir sayı önermez.

### §2 Hesap

Her üçgen sınırın dış halkasıyla ve deliklerle Sutherland–Hodgman ile kesin kırpılır; pencere üçgendir, dışbükeydir. Dış
halka dışbükey olmasa da alan doğrudur. Örtülen alan üçgenlerin sınır içindeki alanlarının toplamıdır. Verisiz alan,
sınırın alanından örtülen alanın çıkarılmasıdır. Dışarıdaki alan, yüzeyin alanından örtülen alanın çıkarılmasıdır.
Kenarı `longest`'ı aşan ve sınıra giren üçgenin sınır içindeki alanı tahmin edilen alandır; sayısı ve örtülenin içindeki
payı verilir. İçerideki en uzun kenar da verilir.

### §3 Ret ve uyarılar

Ret sırası: `invalid_coverage` (`longest` 0'dan büyük değil), `not_a_road_project`, `unknown_terrain`,
`terrain_unbuildable`, `entity_not_found`, `invalid_coverage` (sınır olamayan nesne). Uyarı: `terrain_no_data`; yüzey
sınırın tamamını örtmüyorsa verisiz kalan yer bilinmeyen arazidir, boş arazi gibi sunulmaz.

### §4 Masaüstü

Yüzey analizi penceresinde Kapsam kipi: sınır nesnesi, en uzun kenar; alanlar ve paylar, planda yüzeyin ölçülen ve
tahmin edilen üçgenleri ve sınır.

### §5 Düşük güvenli alanlar

`road.terrains.lowconfidence` v1, tek geri alma adımı “Düşük güvenli alanları çiz”. Kenarı `longest`'ı aşan
üçgenlerin birleşiminin dış çizgileri katmana kapalı çoklu çizgi olarak yazılır: yalnız bir seçili üçgenin kenarı olan
kenarlar, üçgen solda kalacak yönde, uçtan uca zincirlenir. İki dış çizginin değdiği köşede en sağa dönüş seçilir;
düz giden köşeler atılır. Dış çizgi saat yönünün tersine, delik saat yönünde döner. Her biri en küçük köşesinden
(x, sonra y) başlar ve bu sırayla yazılır. ASPRS'in teslim ettiği 2B düşük güvenli alan çokgenleri bunlardır.
`labels` ile her dış çizginin ortasına alanı yazılır (dış halka eksi içindeki delikler). `outline` ile yüzeyin kendi
dış çizgisi de yazılır, yani verinin olduğu yer. Tahmin yoksa ve dış çizgi istenmemişse hiçbir şey yazılmaz.
Ret: `invalid_revision`, `revision_conflict`, `invalid_coverage`, `not_a_road_project`, `unknown_terrain`,
`terrain_unbuildable`, katmanın retleri; uyarı `layer_hidden`.

### §6 Nokta yoğunluğu

`road.terrains.density` v1, yalnız okur. Yüzeyin köşeleri `cell` metrelik kare hücrelerde sayılır. Hücreler dünyanın
başlangıcından `cell`'in katlarıdır, satırlar y'ye, sütunlar x'e göre dizilir. Nokta ⌊x/cell⌋, ⌊y/cell⌋ hücresinindir:
kenardaki nokta sağdaki ve üstteki hücreye düşer. Izgara sınırın dış halkasının ya da sınır yoksa yüzeyin köşelerinin
kutusunu örter. Merkezi sınırın içinde (tek-çift kuralı, deliklerde değil) ya da sınır yoksa yüzeyin üstünde (kenarı
dahil) olan hücre sayılır. Sayılmayan hücreye düşen nokta da sayılmaz. `least` ile bundan az noktalı hücreler seyrektir.
`share` (0–1) ile noktası olan hücrelerin payının ona ulaşıp ulaşmadığı söylenir. Cevap: ızgara, sayılan, noktalı ve
boş hücreler, pay, seyrek hücreler, noktalar, alan (hücre × hücre²), ortalama yoğunluk, en kalabalık hücre ve tuttu mu.
En çok 1 000 000 hücre okunur. Ret: `invalid_point_density` (hücre, least 0, pay, çok hücre), `not_a_road_project`,
`unknown_terrain`, `terrain_unbuildable`, `entity_not_found`, `invalid_coverage`. Uyarı: `terrain_density_short`.
Yüzey analizi'nde Yoğunluk kipi: hücreler sayılarıyla, boş kırmızı, seyrek turuncu.

### §7 Seyrek alanlar

`road.terrains.sparse` v1, tek geri alma adımı “Seyrek alanları çiz”. Yoğunluk ızgarasının `least`'ten az noktalı
sayılan hücreleri birleştirilir: yalnız bir seçili hücrenin kenarı olan kenarlar §5'in kuralıyla zincirlenir (iki
alanın köşeden değdiği yerde en sağa dönüş). Kenarla birleşen hücreler tek alandır. Dış çizgiler ve delikler kapalı
çoklu çizgi olarak yazılır, `labels` ile alanları §5'teki gibi. Ret: §6'nınkiler, katmanın retleri,
`invalid_revision`, `revision_conflict`; uyarı `layer_hidden`.

## Açıklar

- Yaylı sınır, çok parçalı sınır, alan olarak (delikleriyle) yazım, kaynak nokta bulutunun kendi yoğunluğu (yüzeyin
  köşeleri değil, buluttaki zemin noktaları; ADR 0928'ün kaydı) ve yalnız ilk dönüşlerle sayım açık.
