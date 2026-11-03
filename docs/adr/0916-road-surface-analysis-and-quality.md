# 0916 — KentOS Yol: yüzeyin analizi, farkı ve ölçme kalitesi

Tarih: 2026-11-04. plan/road.md Y18 (ROAD-TIN-12, 13, 16–18; ROAD-SURVEY-04, 05, 09). Durum: taslak, aşamalar aşağıda.

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

Açık: parçalı yüzeyin dikişleri ve sınırda boş çember (ROAD-TIN-12, 13), GPU ön elemesi (ROAD-TIN-18), onarımın geri
alınabilir farkı (ROAD-TIN-17), ölçme kaydının tipli kaynağı (ROAD-SURVEY-01).

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
