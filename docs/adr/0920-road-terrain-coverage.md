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

## Açıklar

- Yaylı sınır, çok parçalı sınır, nokta yoğunluğu ızgarası ve alan olarak (delikleriyle) yazım açık.
