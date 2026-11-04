# 0916 — KentOS Yol: yüzeyin analizi, farkı ve ölçme kalitesi

Tarih: 2026-11-04. plan/road.md Y18 (ROAD-TIN-12, 13, 16–18; ROAD-SURVEY-04, 05, 09). Durum: kabul (Y18.1–Y18.5), açıklar aşağıda.

## Dayanak

- **Civil 3D:** yüzeyin Özellikler › Analiz sekmesinde Contours, Directions, Levels (Elevations), Slopes, Slope Arrows,
  User-Defined Contours, Watersheds; her analizin aralık sayısı ve her aralığın en az ve en çok değeri; Slopes ve
  Directions üçgenleri girdikleri aralığa göre boyar; lejant tablosu aralığın 2B ve 3B alanını verir; yüzey değişince
  aralıkların değerleri kendiliğinden yenilenmez
  ([Analysis Tab (Surface Properties Dialog Box)](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-841EAEF9-C05D-4CEE-9907-7E898902EB56.htm),
  [About Updating Surface Analysis Values](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-FAD4881C-26F8-47ED-99A9-987519BE1214.htm),
  [Noble Desktop: slope ranges and tables](https://www.nobledesktop.com/learn/civil-3d/setting-up-slope-analysis-in-civil-3d-creating-slope-ranges-and-displaying-data),
  4 Kasım 2026'da okundu). İki yüzeyin farkı TIN Volume Surface'tir: taban ve karşılaştırma yüzeyi, kazı, dolgu ve net
  hacim; fark yüzeyinde eksi kazı, artı dolgu; kazı ve dolgu bantları sıfır datumla
  ([Volumes Dashboard](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-F06475D1-BCB8-456A-AB29-E6B731DB32A8.htm),
  [Calculating Bounded Volumes](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-DevGuide/files/GUID-DA353D59-E5F2-4CE7-BFE9-1117EE9A8942.htm),
  [Elevation Banding](https://thecadmasters.com/civil-3d-2012-elevation-banding/)).
- **OpenRoads Designer:** Terrain › Analysis › Analyze Point bir noktanın kotunu, bulunduğu üçgenin eğimini (yüzde, yatay
  düzleme göre) ve bakısını (derece, kuzey sıfır, saat yönünde) verir; Analyze Trace Slope en dik ya da sabit eğimle iz
  sürer ([Analyze Point](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/353439/GUID-1517115C-5040-8A07-1B4E-1A2A2A3A1DD5.html),
  [FHWA ORD manual: terrain analysis](https://highways.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13221/ord-manual21-terrain-analysis.pdf)).
- **Netcad:** NETPRO'nun eğim haritası ve yüzey karşılaştırması için çevrimiçi resmî kılavuz bulunamadı; Mersin
  Üniversitesi'nin NETPRO kitabı kübajı enkesitlerin yarma ve dolgu alanlarıyla anlatır
  ([NETPRO kitabı](https://publish.mersin.edu.tr/index.php/geoengbooks/article/download/722/620)).
- **Ölçme kalitesi:** ASPRS'in Konum Doğruluğu Standartları'nın 2. baskısı (2023, 2024'te 2. sürüm) doğruluğu yalnız
  RMSE ile bildirir, %95 güven düzeyini bırakır; bitkisiz zeminin doğruluğu (NVA) karşılanır, bitkili zeminin (VVA)
  bildirilir; kontrol noktası en az 30
  ([LiDAR Magazine: Edition 2](https://lidarmag.com/2023/10/06/the-asprs-positional-accuracy-standards-edition-2/),
  [USGS: adopt updated accuracy standards](https://www.usgs.gov/ngp-standards-and-specifications/adopt-updated-accuracy-standards)).
  Esri'nin Locate Outliers'ı noktayı komşularıyla kot ya da eğim farkından aykırı sayar, çıplak zemin verisinde
  ([Locate Outliers](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/3d-analyst/locate-outliers.html)); Civil
  3D'de noktalar yüzeyle karşılaştırılıp fark sütunu sıralanır, sivriyi yapan nokta silinir (NRCS'in
  [LiDAR karşılaştırma yöntemi](https://www.nrcs.usda.gov/sites/default/files/2023-06/HowTo-%20LiDAR%20Comparison%20C3D.pdf)).
  KentOS sınırı kendi koymaz, standardın sayısını kural yapmaz; sınıf sınıf istatistik verir.
- **Parçalı yüzey:** Isenburg, Liu, Shewchuk ve Snoeyink'in akışlı Delaunay'ı (SIGGRAPH 2006) üçgeni çevrel çemberi
  henüz görülmemiş alana uzanmadığında kesinleştirir; 11,2 GB LIDAR'dan bir milyar üçgen 70 MB bellekle
  ([proje sayfası](https://www.cs.unc.edu/~isenburg/sd),
  [UNC: akışlı Delaunay](https://wwwx.cs.unc.edu/Research/compgeom/twiki/bin/view.cgi/TModeling/StreamingDelaunay)).
  LAStools karoları sabit bir tamponla işler ve yalnız iç kısmı rasterleştirir
  ([rapidlasso: use buffers when processing lidar in tiles](https://rapidlasso.de/use-buffers-when-processing-lidar-in-tiles/));
  sabit tampon kenar izlerini azaltır ama bütün yüzeyin Delaunay'ını garanti etmez (ROAD-TIN-12).
- **Pürüzlülük:** Sappington, Longshore ve Thomson'ın (2007) vektör pürüzlülük ölçüsü (VRM): eğim ve bakı birim
  vektörlere ayrılır, pencerede bileşkenin boyu; 1 − |Σn|/N, 0 düz, 1'e doğru pürüzlü
  ([GRASS r.vector.ruggedness](https://grass.osgeo.org/grass84/manuals/addons/r.vector.ruggedness.html)).

## Karar

1. **Çekirdek `kentos_road::surface` analizi (Y18.1).** `Surface::bands(measure, breaks, pieces)`: kot (üçgen iki
   düzlemle kırpılır, aralığın plan alanı kesin, 3B alanı plan alanı çarpı √(1 + |∇z|²)), eğim (yüzde) ve bakı (derece,
   kuzeyden saat yönünde; düz üçgen ayrı) aralıkları; aralık [bᵢ, bᵢ₊₁), sonuncusu kapalı, dışarıdakiler ayrı; aralıklar
   kullanıcının. `Surface::roughness`: köşede üçgenlerin birim normallerinin plan açılarıyla ağırlıklı VRM'i (bir
   düzlemin iki yönlü bölünüşü aynı sonucu verir), ortalaması ve en büyüğü. `Surface::grid_slope(cell)`: yüzeyden
   örneklenen ızgarada Horn'un 3 × 3 farkıyla eğim, merkezin düştüğü üçgeninkine karşı (ortalama, mutlak ortalama, en
   büyük fark ve yeri): rasterin TIN'den neden farklı eğim verdiğini söyler (ROAD-TIN-16).
2. **Komut (Y18.2).** `road.terrains.analysis`: bir yüzeyin kot, eğim, bakı aralıkları, pürüzlülüğü ve isteğe bağlı
   raster karşılaştırması.
3. **Yüzey farkı (Y18.3).** `road.terrains.compare`: taban ve karşılaştırma yüzeyi, kazı, dolgu, net hacim ve alanları
   (Y8'in `volume::between_surfaces`'i), farkın en büyük ve en küçüğü, fark aralıklarının alanları.
4. **Ölçme kalitesi (Y18.4).** Aynı yerde farklı kotlu noktalar, komşularından aykırı kot, kenarı uzun üçgenle kapsama
   boşluğu, kontrol noktalarının yüzeye farkı sınıfına göre (sert zemin, bitkili; tek RMSE değil).
5. **Masaüstü (Y18.5).** Yüzey analizi penceresi.

6. **Onarım (Y18.6).** Yüzeyin tanımında `excluded` (nokta kayıtlarının yerleri): kaynak toplanırken o yerdeki nokta
   kayıtları alınmaz, çizimin noktaları değişmez, kırık hattın köşesi bırakılmaz, nokta kaydırılmaz (ROAD-TIN-17);
   `road.terrains.repair` önce farkı söyler, tek geri alma adımıyla yazar, `restore` geri alır.

7. **Parçalı yüzey (Y18.8).** `surface::build_tiled`: noktaların kutusu kare karolara bölünür, her karo çevresindeki
   payın noktalarıyla üçgenlenir; karonun üstündeki her üçgen kesinleşmeli (payın dışındaki, çemberin eriştiği hücrelerin
   noktaları tam in-circle yüklemiyle sorulur), karonun bütün yüzeyin örtüsü kadarı üçgenlerle örtülmeli; değilse karonun
   payı ikiye katlanır. Karo üçgenleri ağırlık merkezleriyle sahiplenir; birleşim üçgen sayısı (2n − 2 − h) ve alanla
   bütüne karşı denetlenir, tutmazsa bütün paylar büyür; her karo bütün noktaları gördüyse ve yine tutmuyorsa (eş
   çemberli noktalar) yüzey bütün kurulur. Kırık hat, sınır ve boşluk karolanmaz.

Açık: kırık hatlı yüzeyin karolanması (kısıtlı Delaunay'da çember yalnız görünen noktalara bakar), dış kabuğa komşu
karoların büyük payı (kabuğa göre pay), GPU ön elemesi (ROAD-TIN-18), yumuşatma (onarım yalnız dışarıda bırakır),
ölçme kaydının tipli kaynağı (ROAD-SURVEY-01).

## Aşamalar

Y18.1 (yapıldı, çekirdek): §1. `crates/shared/road/src/surface/analysis.rs`, testleri `tests/all/analysis.rs`: düzlemin
kot aralıkları şeritleri (plan ve 3B alan kapalı biçimle), z = g(x) + h(y) kafesinde (her kare düzlem: sonuç
köşegenlerden bağımsız) eğim ve bakı aralıkları karelerle, düzlemin pürüzlülüğü sıfır, sırtın 1 − cos 45°, düzlemde
raster eğimi TIN'inkiyle aynı, sırtta küçük; retler.

Y18.2 (yapıldı, komut): §2. `road.terrains.analysis` (`crates/native/application/src/road_terrains_analysis.rs`):
yüzeyin plan ve 3B alanı, `elevations`, `slopes`, `aspects` aralıkları (her biri plan ve 3B alan, eğim ve bakıda üçgen
sayısı, aralık dışı ve düz alan, `pieces` ile parçalar), `roughness` (ortalama, en büyük, yeri), `grid` (rasterin
eğiminin üçgenlerinkinden farkı). Ret `invalid_analysis`. 29 ortak durum
(`scripts/fixtures/road_terrain_analysis_cases.py`: drenajın vadi, çukur ve düzlem kafesleri ile teras ve sırt; her
kafes z = g(x) + h(y), her kare düzlem, cevaplar karelerle; en büyüğün yeri yalnız tekse yazılır; KentOS kodu yok).
Python `kentos.cad.road.terrains.analysis`, MCP.

Y18.3 (yapıldı, yüzey farkı): §3. `kentos_road::volume`'e `Ground::overlay` (karşılaştırmanın ve tabanın üçgenlerinin
kesişen parçaları, d köşelerinde) ve `difference` (hacimler Y8'in kuralıyla, en küçük ve en büyük fark ve yeri, fark
aralıklarının alanı); `road.terrains.compare` (`road_terrains_compare.rs`): kazı, dolgu, net, alanları, örtülen ve
tabanın dışında kalan alan (uyarı `compare_uncovered`), uçlar, aralıklar. 22 ortak durum
(`scripts/fixtures/road_terrain_compare_cases.py`: iki kafesin dikdörtgenlerle üst üste konuşu, her dikdörtgende fark
doğrusal; düzlem üstünde yarısı dışarıda kazı, teras üstünde set; KentOS kodu yok). Python, MCP.

Y18.4 (yapıldı, ölçme kalitesi): §4. Çekirdekte `Surface::near_pairs`, `spikes` (köşenin kotu erim içindeki öbür
köşelerin ortalamasından; kotlar küçükten büyüğe toplanır, eşit cevaplar bit bit eşit), `gaps`;
`road.terrains.quality` (`road_terrains_quality.rs`): yakın çiftler, sivri noktalar, boşluklar (listeler en çok 1000,
sayıları bütün), kontrol noktaları (katman ya da nesneler, `classField` ile sınıf; her noktanın farkı, sınıf sınıf ve
hepsinin sayısı, yüzey dışı, ortalama, standart sapma (n − 1), RMSE, uçlar). 29 ortak durum
(`scripts/fixtures/road_terrain_quality_cases.py`: seyrek kafes, sivri kafes ve iki kez ölçülen nokta, terasta iki
sınıflı kontrol noktaları; KentOS kodu yok). Python, MCP. Yolda bulunan: Python 3.12'nin `sum()`'ı telafili toplar,
Rust'ın düz toplamından son basamakta ayrılır; başvuru düz toplar.

Y18.5 (yapıldı, masaüstü): §5. Arazi sekmesinde Analiz paneli, `terrain.analysis`
(`apps/desktop/src/surface_analysis_window.rs`): Kip (Analiz, Fark, Kalite) ve Yüzey; Analiz'de ölçü (kot, eğim, bakı),
sınırlar, pürüzlülük, raster hücresi: planda aralıklar yeşilden kırmızıya, tabloda plan ve 3B alan ve pay; Fark'ta
karşılaştırma yüzeyi ve aralıklar: kazı, dolgu, net, tabanın dışı, fark aralıkları çubuklarla; Kalite'de yakın, sivri
erimi ve sınırı, boşluk kenarı, kontrol katmanı ve sınıf alanı: kontrol noktaları farklarıyla planda, sınıf sınıf
istatistik. Hiçbir şey yazılmaz. Testler `surface_analysis_window_tests.rs`, resimler
`.run/gui-evidence/yol-y18-yuzey-analizi-*`.

Y18.6 (yapıldı, onarım): §6. `TerrainSurfaceDef.excluded` (`SurfaceExclusion` x, y; en çok 100 000, her yer bir kez;
`.kcad`'in yol ayarlarında, iki okuyucuda; `road-work.kcad`'de örnek, `broken/road-surface-excluded-twice.kcad`),
`kentos_road::sources::gather` o yerlerdeki nokta kayıtlarını almaz (`excluded`, `unmatched`). Belgede geri alınabilir
`Op::RoadSurfaces` ve `set_road_surfaces`. `road.terrains.repair` (`road_terrains_repair.rs`): `exclude` ve `restore`,
kırık hattın köşesi `repair_breakline`, kaydı olmayan yer `repair_not_a_point`; fark (sonra − önce: kazı, dolgu, net,
değişen alan, en büyük değişiklik ve yeri; köşe ve üçgen sayıları); kabul edilmiş yüzey kabul edilmiş kalır
(`tests/all/terrain_repair.rs`). 22 ortak durum (`scripts/fixtures/road_terrain_repair_cases.py`: düzlemde kare içindeki
sivri noktanın çadırı, üçgenleri boş çember sınamasıyla; KentOS kodu yok). Python, MCP.

Y18.7 (yapıldı, masaüstünde onarım): Yüzey analizi'nin Kalite kipinde Sivrileri bırak bulunan sivri noktaların
`road.terrains.repair` planını okur ve farkı (kazı, dolgu, değişen alan, en büyük değişiklik, köşeler) yazmadan söyler;
Onarımı uygula tek adımda yazar ve kaliteyi yeniden okur; Bırakılanları geri al yüzeyin bıraktıklarının hepsini geri
alır. Test `the_spikes_are_left_out_after_their_difference_and_taken_back` (fark yazmaz, uygulanınca sivri kalmaz, geri
alınır, geri al adımı), resimler `.run/gui-evidence/yol-y18-yuzey-analizi-onarim-*`.

Y18.8 (yapıldı, parçalı yüzey): §7. `crates/shared/road/src/surface/tiled.rs`, testleri `tests/all/tiled.rs`: genel
konumdaki 3000 ve 1500 noktada her karo ve pay için birleşim bütün yüzeyle üçgen üçgen aynı (sıralı köşe üçlüleri), 61 ×
41 kafeste (her yerde dört nokta bir çemberde) üçgen sayısı ve alan tutar, retler. Ölçüm (release, M5, karolar
iş parçacıklarında): 200 000 noktada bütün yüzey 98 ms; 500 m karo, 20 m pay 925 ms (48 karo, 202 kurma, en büyük pay
2560 m); 1000 m karo 1609 ms. Bu boyutta bütünü kurmak daha hızlı: karolama belleği sınırlı tutmanın ve kanıtlı
birleşimin yoludur; dış kabuğa komşu karoların payı büyüdüğü için yavaştır (açık). İlk sürüm çemberin kutusunu
noktaların kutusuyla sınırlıyor, payları bütün kutuya büyütüyordu (5 120 m); karonun yalnız merkezi kendinde olan
üçgenlerini kesinleştirmek kabuğa yakın boşluk bırakıyordu (ölçümle bulundu, örtü denetimi eklendi).
